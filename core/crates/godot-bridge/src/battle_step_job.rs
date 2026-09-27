//! Lot PB3e (ADR 0090): the next battle step computed on a worker thread.
//!
//! The fixed step (0.1 s, the AI every 2 s) used to fall whole into one
//! frame out of six: a spike. [`StepPipeline::tick`] advances the battle
//! like `sim_battle::BattleSim::tick`, but the step that falls due is the
//! one a worker computed ahead on a fork of the displayed state
//! (`BattleSim::fork_for_step`); as soon as it is adopted, the next fork is
//! sent to the worker, while the frames keep showing the adopted state.
//!
//! Same result as the synchronous path: a fork is only used for the state it
//! was taken from, checked by `(ticks, epoch)`, the epoch being bumped by
//! every change of the battle outside a step (orders, deployment, setup,
//! replay jump: `BattleSim::touch_poses` in the bridge). A stale fork is
//! dropped (its thread ends on its own) and the step computed in place. See
//! the `pipelined_battle_matches_synchronous_battle` test.
//!
//! No Godot type here: plain Rust, testable with `cargo test`.

use std::thread::JoinHandle;

use sim_battle::BattleSim;

/// Stack of the worker thread (the battle AI recurses little; the default
/// 2 MiB of a spawned thread is kept with a margin).
const WORKER_STACK_BYTES: usize = 16 * 1024 * 1024;

/// One step computed ahead: the fork after its step, and the `(ticks,
/// epoch)` of the state it was taken from.
struct StepJob {
    key: (u64, u64),
    handle: JoinHandle<BattleSim>,
}

impl StepJob {
    /// Runs one step of `fork` on a new thread; `garbage` (a replaced
    /// state) is dropped there too, off the frame.
    fn spawn(fork: BattleSim, garbage: Option<BattleSim>, key: (u64, u64)) -> Option<Self> {
        std::thread::Builder::new()
            .name("cent-ans-battle-step".to_owned())
            .stack_size(WORKER_STACK_BYTES)
            .spawn(move || {
                crate::turn_job::raise_thread_priority();
                drop(garbage);
                let mut fork = fork;
                fork.step();
                fork
            })
            .ok()
            .map(|handle| StepJob { key, handle })
    }
}

/// The step computed ahead for one battle (empty in synchronous mode).
#[derive(Default)]
pub(crate) struct StepPipeline {
    job: Option<StepJob>,
    /// PB3e measures: steps adopted from the worker, steps computed in place.
    pub(crate) adopted: u64,
    pub(crate) in_place: u64,
}

impl StepPipeline {
    /// Forgets the step computed ahead (synchronous mode, replay, new battle).
    pub(crate) fn clear(&mut self) {
        self.job = None;
    }

    /// Advances `sim` by `dt` seconds exactly like `sim.tick(dt)`; `epoch`
    /// changes whenever the battle was changed outside a step. Returns the
    /// steps run.
    pub(crate) fn tick(&mut self, sim: &mut BattleSim, dt: f64, epoch: u64) -> u32 {
        let key = (sim.ticks(), epoch);
        let mut job = self.job.take().filter(|job| job.key == key);
        let mut garbage = None;
        let (mut adopted, mut in_place) = (0, 0);
        let steps = sim.tick_with(dt, |state| {
            let computed = job.take().and_then(|job| job.handle.join().ok());
            match computed {
                Some(next) => {
                    garbage = Some(state.adopt_step(next));
                    adopted += 1;
                }
                None => {
                    state.step();
                    in_place += 1;
                }
            }
        });
        self.adopted += adopted;
        self.in_place += in_place;
        if job.is_some() {
            // No step was due: the fork still matches the state.
            self.job = job;
        } else if sim.can_step() {
            self.job = StepJob::spawn(sim.fork_for_step(), garbage, (sim.ticks(), epoch));
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use sim_battle::{BattleSetup, Command, SideId};

    use super::*;

    fn demo() -> BattleSim {
        let setup: BattleSetup = serde_json::from_str(include_str!(
            "../../sim-battle/tests/fixtures/demo_battle_1337.json"
        ))
        .expect("demo setup parses");
        BattleSim::new(setup, 1337).expect("demo battle builds")
    }

    /// What the renderer takes from the battle in a frame.
    fn drain(sim: &mut BattleSim) -> String {
        format!(
            "{:?}|{:?}|{:?}|{:?}",
            sim.take_shots(),
            sim.take_impacts(),
            sim.take_new_events(),
            sim.take_new_siege_fx()
        )
    }

    /// Orders given at some frames (the same for both runs).
    fn orders(sim: &BattleSim, frame: usize) -> Vec<Command> {
        let ours: Vec<u32> = sim
            .units()
            .iter()
            .filter(|u| u.side == SideId::Attacker)
            .map(|u| u.id)
            .collect();
        let theirs: Vec<u32> = sim
            .units()
            .iter()
            .filter(|u| u.side == SideId::Defender)
            .map(|u| u.id)
            .collect();
        match frame {
            30 => vec![Command::Move {
                units: ours[..2].to_vec(),
                x: 0.0,
                z: 20.0,
                run: false,
                facing: None,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
            }],
            250 => vec![Command::Attack {
                units: ours.clone(),
                target: theirs[0],
                run: true,
                queue: false,
            }],
            251 => vec![Command::Halt {
                units: ours[..1].to_vec(),
            }],
            _ => Vec::new(),
        }
    }

    /// Plays the same battle synchronously and pipelined, frame after frame,
    /// with orders and uneven frame times; both must stay bit-identical and
    /// the renderer must read the same volleys, impacts, journal and siege
    /// effects. Returns the steps adopted from the worker.
    fn compare(mut sync_sim: BattleSim, mut piped_sim: BattleSim, frames: usize) -> u64 {
        let mut pipeline = StepPipeline::default();
        let mut epoch = 0;
        let frame_times = [1.0 / 60.0, 1.0 / 30.0, 0.013, 0.25, 1.0 / 144.0, 0.1];
        for frame in 0..frames {
            for command in orders(&sync_sim, frame) {
                let a = sync_sim.issue_command(command.clone()).is_ok();
                let b = piped_sim.issue_command(command).is_ok();
                assert_eq!(a, b);
                epoch += 1;
            }
            let dt = frame_times[frame % frame_times.len()];
            sync_sim.tick(dt);
            pipeline.tick(&mut piped_sim, dt, epoch);
            if frame % 3 == 0 {
                assert_eq!(drain(&mut piped_sim), drain(&mut sync_sim), "frame {frame}");
            }
            assert_eq!(piped_sim.ticks(), sync_sim.ticks(), "frame {frame}");
            if frame % 97 == 0 || sync_sim.is_finished() {
                assert_eq!(
                    format!("{piped_sim:?}"),
                    format!("{sync_sim:?}"),
                    "frame {frame}"
                );
            }
            if sync_sim.is_finished() {
                break;
            }
        }
        assert_eq!(format!("{piped_sim:?}"), format!("{sync_sim:?}"));
        pipeline.adopted
    }

    #[test]
    fn pipelined_battle_matches_synchronous_battle() {
        let adopted = compare(demo(), demo(), 3000);
        assert!(adopted > 100, "adopted {adopted}");
    }

    /// Same on a siege (walls, ladders, siege effects read by cursor).
    #[test]
    fn pipelined_siege_matches_synchronous_siege() {
        let mut setup: serde_json::Value = serde_json::from_str(include_str!(
            "../../sim-battle/tests/fixtures/demo_battle_1337.json"
        ))
        .unwrap();
        setup["siege"] = serde_json::json!({"fortification": 2, "breach": 0});
        let setup: BattleSetup = serde_json::from_value(setup).unwrap();
        let build = || BattleSim::new(setup.clone(), 7).expect("siege builds");
        let sim = build();
        assert!(sim.siege().is_some());
        let adopted = compare(sim, build(), 2500);
        assert!(adopted > 100, "adopted {adopted}");
    }

    /// A fork taken before an order is not used after it.
    #[test]
    fn stale_fork_is_dropped() {
        let mut sim = demo();
        let mut pipeline = StepPipeline::default();
        pipeline.tick(&mut sim, 0.05, 0);
        assert!(pipeline.job.is_some());
        pipeline.tick(&mut sim, 0.06, 1);
        assert_eq!(pipeline.adopted, 0);
        assert_eq!(pipeline.in_place, 1);
    }
}
