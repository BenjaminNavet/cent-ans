//! Lot PB3d (ADR 0081): the end of turn resolved on a worker thread.
//!
//! The job works on a *clone* of the campaign state, so the Godot side keeps
//! reading the state of before while the AI plays; the resolved state is
//! installed at once when the job is collected. Both the synchronous and the
//! threaded paths go through [`resolve_turn`], so they are bit-identical for
//! the same state and data (the simulation is deterministic and keeps no
//! global state; see the `threaded_turn_matches_synchronous_turn` test).
//!
//! No Godot type is touched here: the module is plain Rust, testable with
//! `cargo test`.

use std::sync::Arc;
use std::thread::JoinHandle;

use data_model::GameData;
use sim_campaign::{CampaignState, GameEvent};

/// Stack of the worker thread: the AI planner recurses on route tables, and a
/// spawned thread only gets 2 MiB by default (the main thread has 8 MiB).
const WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;

/// A resolved turn: the new state and the turn's events.
pub(crate) type TurnOutcome = (CampaignState, Vec<GameEvent>);

/// The one end-of-turn resolution of the Godot bridge (lot M9: every AI
/// faction plays with the strategic planner). Lot PB3f may parallelise the
/// planner inside; this signature stays.
pub(crate) fn resolve_turn(state: &mut CampaignState, data: &GameData) -> Vec<GameEvent> {
    state.end_turn_with(data, ai::plan_turn)
}

/// The player waits for this thread: on macOS a new thread gets the default
/// quality of service and may be scheduled on the efficiency cores (the end
/// of turn then took 3 to 8 times longer in the A/B runs); it asks for
/// `QOS_CLASS_USER_INITIATED`.
#[cfg(target_os = "macos")]
fn raise_thread_priority() {
    /// `QOS_CLASS_USER_INITIATED` from `<sys/qos.h>`.
    const QOS_CLASS_USER_INITIATED: u32 = 0x19;
    extern "C" {
        fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
    }
    // SAFETY: libSystem call on the current thread, no pointer involved; a
    // failure only leaves the default quality of service.
    unsafe {
        pthread_set_qos_class_self_np(QOS_CLASS_USER_INITIATED, 0);
    }
}

#[cfg(not(target_os = "macos"))]
fn raise_thread_priority() {}

/// An end of turn running on its own thread.
pub(crate) struct TurnJob {
    handle: Option<JoinHandle<TurnOutcome>>,
}

impl TurnJob {
    /// Starts resolving the turn of `state` (moved into the worker).
    pub(crate) fn spawn(mut state: CampaignState, data: Arc<GameData>) -> std::io::Result<Self> {
        let handle = std::thread::Builder::new()
            .name("cent-ans-end-turn".to_owned())
            .stack_size(WORKER_STACK_BYTES)
            .spawn(move || {
                raise_thread_priority();
                let events = resolve_turn(&mut state, &data);
                (state, events)
            })?;
        Ok(TurnJob {
            handle: Some(handle),
        })
    }

    /// True once the worker has returned (or panicked).
    pub(crate) fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// Waits for the worker and returns its outcome, or the panic message.
    pub(crate) fn join(mut self) -> Result<TurnOutcome, String> {
        let Some(handle) = self.handle.take() else {
            return Err("fin de tour déjà relevée".to_owned());
        };
        handle.join().map_err(|panic| {
            panic
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panique sans message".to_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use data_model::FactionId;

    use super::*;

    fn load_data() -> Option<Arc<GameData>> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
        if !dir.join("factions").exists() {
            return None;
        }
        Some(Arc::new(GameData::load(&dir).expect("game data loads").0))
    }

    /// The threaded end of turn gives exactly the state and events of the
    /// synchronous one, turn after turn (same seed, same data).
    #[test]
    fn threaded_turn_matches_synchronous_turn() {
        let Some(data) = load_data() else {
            return;
        };
        let player = FactionId::new("fac_france").unwrap();
        let start = CampaignState::new_1337(&data, player, 1337).expect("1337 start");
        let mut sync_state = start.clone();
        let mut async_state = start;
        for turn in 0..3 {
            let sync_events = resolve_turn(&mut sync_state, &data);
            let job = TurnJob::spawn(async_state, Arc::clone(&data)).expect("thread spawns");
            let (state, events) = job.join().expect("worker does not panic");
            async_state = state;
            assert_eq!(events, sync_events, "events differ at turn {turn}");
            assert_eq!(
                async_state.save_json(),
                sync_state.save_json(),
                "state differs at turn {turn}"
            );
        }
    }

    #[test]
    fn job_reports_finished_after_join_ready() {
        let Some(data) = load_data() else {
            return;
        };
        let player = FactionId::new("fac_england").unwrap();
        let state = CampaignState::new_1337(&data, player, 7).expect("1337 start");
        let turn = state.turn();
        let job = TurnJob::spawn(state, data).expect("thread spawns");
        while !job.is_finished() {
            std::thread::yield_now();
        }
        let (state, events) = job.join().expect("worker does not panic");
        assert_eq!(state.turn(), turn + 1);
        assert!(!events.is_empty());
    }
}
