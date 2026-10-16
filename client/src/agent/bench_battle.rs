//! Battle bench (`--bench-battle`): real frame times over a scripted camera in a battle.
//!
//! The battle itself is opened by a `--script` (`tools/bevy_bench/battle_open.txt`: Batailles ->
//! Batailles historiques -> Crécy, defender, no prologue) with `CENT_ANS_NO_DEPLOY=1`; see
//! `tools/bevy_bench/bench.sh --bench-battle`. Two measured phases, same fields as the map bench:
//!
//! - `calm`: the opening pose (pivot, yaw and distance as the battle camera spawned them), held
//!   for `--bench-calm-seconds` (15 s), the armies still deployed and approaching;
//! - `melee`: the simulation is fast-forwarded (x16) until a regiment is in melee, then runs at
//!   x1 with a fixed oblique camera (distance 70 m, yaw of the opening plus 35 deg) pivoting on
//!   the centroid of the regiments in melee at that moment, for `--bench-melee-seconds` (20 s).
//!
//! Vsync is switched off. The camera is rewritten every frame before the camera controller, so the
//! path does not depend on input. The report is `out/bench/battle_<label>.json`.

use std::path::PathBuf;
use std::time::Instant;

use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
use serde_json::json;
use sim_battle::UnitState;

use super::bench::{Stats, write_report};
use super::cli::BenchConfig;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCameraState, control_camera};

const WARMUP_FRAMES: u32 = 90;
const READY_TIMEOUT_FRAMES: u32 = 12_000;
/// Real frames the fast-forward may last before giving up on contact.
const CONTACT_TIMEOUT_FRAMES: u32 = 3_000;
const FAST_FORWARD: f64 = 16.0;
const MELEE_DISTANCE: f32 = 70.0;
const MELEE_YAW_OFFSET_DEG: f32 = 35.0;
const PROFILE_SPIKE_MS: f32 = 33.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    WaitBattle,
    Warmup,
    Calm,
    FastForward,
    Melee,
    Done,
}

#[derive(Resource)]
struct BattleBench {
    config: BenchConfig,
    started: Instant,
    startup_ms: f64,
    phase: Phase,
    phase_time: f32,
    frames: u32,
    /// Opening pose: pivot, yaw, distance.
    opening: Option<(Vec3, f32, f32)>,
    /// Melee pose: pivot, yaw, distance.
    melee: Option<(Vec3, f32, f32)>,
    contact: bool,
    sim_seconds_at_contact: f64,
    calm: Vec<f32>,
    calm_visible: Vec<u32>,
    melee_ms: Vec<f32>,
    melee_visible: Vec<u32>,
    entities_max: usize,
    regiments: [usize; 2],
    spikes: Vec<(f64, f32)>,
}

/// Centroid (x, z) of the points, `None` when empty.
pub fn centroid(points: &[(f64, f64)]) -> Option<Vec2> {
    if points.is_empty() {
        return None;
    }
    let n = points.len() as f64;
    let (sx, sz) = points
        .iter()
        .fold((0.0, 0.0), |(sx, sz), (x, z)| (sx + x, sz + z));
    Some(Vec2::new((sx / n) as f32, (sz / n) as f32))
}

pub fn add(app: &mut App, config: BenchConfig) {
    app.insert_resource(BattleBench {
        config,
        started: Instant::now(),
        startup_ms: 0.0,
        phase: Phase::WaitBattle,
        phase_time: 0.0,
        frames: 0,
        opening: None,
        melee: None,
        contact: false,
        sim_seconds_at_contact: 0.0,
        calm: Vec::new(),
        calm_visible: Vec::new(),
        melee_ms: Vec::new(),
        melee_visible: Vec::new(),
        entities_max: 0,
        regiments: [0, 0],
        spikes: Vec::new(),
    })
    .add_systems(
        Update,
        battle_bench_step
            .before(control_camera)
            .run_if(resource_exists::<BattleBench>),
    );
}

#[allow(clippy::too_many_arguments)]
fn battle_bench_step(
    mut run: ResMut<BattleBench>,
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    session: Option<ResMut<BattleSession>>,
    camera: Option<ResMut<BattleCameraState>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    meshes: Query<&ViewVisibility, With<Mesh3d>>,
    all: Query<Entity>,
    mut exit: MessageWriter<AppExit>,
) {
    run.frames += 1;
    let dt = time.delta_secs();
    let frame_ms = dt * 1000.0;
    let (Some(mut session), Some(mut camera)) =
        (session.filter(|_| *state.get() == AppState::Battle), camera)
    else {
        if run.phase == Phase::WaitBattle && run.frames > READY_TIMEOUT_FRAMES {
            error!("AGENT_FATAL: bench-battle: the battle never became ready");
            exit.write(AppExit::error());
            run.phase = Phase::Done;
        }
        return;
    };

    let measuring = matches!(run.phase, Phase::Calm | Phase::Melee);
    if measuring && dt > 0.0 {
        if frame_ms > PROFILE_SPIKE_MS {
            run.spikes.push((super::profile::now_ms(), frame_ms));
        }
        let sampled = run.frames.is_multiple_of(10);
        let visible = if sampled {
            meshes.iter().filter(|v| v.get()).count() as u32
        } else {
            0
        };
        if run.phase == Phase::Calm {
            run.calm.push(frame_ms);
            if sampled {
                run.calm_visible.push(visible);
            }
        } else {
            run.melee_ms.push(frame_ms);
            if sampled {
                run.melee_visible.push(visible);
            }
        }
    }
    run.phase_time += dt;
    if run.frames.is_multiple_of(30) {
        let count = all.iter().count();
        run.entities_max = run.entities_max.max(count);
    }

    match run.phase {
        Phase::WaitBattle => {
            for mut window in &mut windows {
                window.present_mode = PresentMode::AutoNoVsync;
            }
            run.opening = Some((camera.target, camera.yaw, camera.target_distance));
            run.startup_ms = run.started.elapsed().as_secs_f64() * 1000.0;
            run.phase = Phase::Warmup;
            run.phase_time = 0.0;
            run.frames = 0;
        }
        Phase::Warmup => {
            hold_opening(&run, &mut camera);
            if run.frames >= WARMUP_FRAMES {
                super::profile::reset();
                run.phase = Phase::Calm;
                run.phase_time = 0.0;
            }
        }
        Phase::Calm => {
            hold_opening(&run, &mut camera);
            if run.phase_time >= run.config.calm_seconds {
                session.speed = FAST_FORWARD;
                run.phase = Phase::FastForward;
                run.phase_time = 0.0;
                run.frames = 0;
            }
        }
        Phase::FastForward => {
            hold_opening(&run, &mut camera);
            let sim = session.sim_mut();
            let fighting: Vec<(f64, f64)> = sim
                .units()
                .iter()
                .filter(|u| u.state == UnitState::Melee)
                .map(|u| (u.x, u.z))
                .collect();
            let timed_out = run.frames > CONTACT_TIMEOUT_FRAMES;
            if !fighting.is_empty() || timed_out {
                run.contact = !fighting.is_empty();
                let all_units: Vec<(f64, f64)> = sim.units().iter().map(|u| (u.x, u.z)).collect();
                let at = centroid(&fighting)
                    .or_else(|| centroid(&all_units))
                    .unwrap_or(Vec2::ZERO);
                let (_, yaw, _) = run.opening.unwrap_or((Vec3::ZERO, 0.0, 0.0));
                run.melee = Some((
                    Vec3::new(at.x, 0.0, at.y),
                    yaw + MELEE_YAW_OFFSET_DEG.to_radians(),
                    MELEE_DISTANCE,
                ));
                run.sim_seconds_at_contact = sim.elapsed();
                session.speed = 1.0;
                run.phase = Phase::Melee;
                run.phase_time = 0.0;
                // A few frames to let the scene settle on the new pose before measuring.
                run.frames = 0;
            }
        }
        Phase::Melee => {
            if let Some((target, yaw, distance)) = run.melee {
                pose(&mut camera, target, yaw, distance);
            }
            run.regiments = session.snapshot.regiments;
            if run.phase_time >= run.config.melee_seconds {
                finish(&mut run, &mut exit);
            }
        }
        Phase::Done => {}
    }
}

fn pose(camera: &mut BattleCameraState, target: Vec3, yaw: f32, distance: f32) {
    camera.target = target;
    camera.yaw = yaw;
    camera.distance = distance;
    camera.target_distance = distance;
    camera.manual_pitch_deg = None;
}

fn hold_opening(run: &BattleBench, camera: &mut BattleCameraState) {
    if let Some((target, yaw, distance)) = run.opening {
        pose(camera, target, yaw, distance);
    }
}

fn finish(run: &mut BattleBench, exit: &mut MessageWriter<AppExit>) {
    run.phase = Phase::Done;
    let calm = (!run.calm.is_empty()).then(|| Stats::of(&run.calm, &run.calm_visible));
    let melee = (!run.melee_ms.is_empty()).then(|| Stats::of(&run.melee_ms, &run.melee_visible));
    let report = json!({
        "bench": "battle",
        "engine": "bevy",
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "calm_seconds": run.config.calm_seconds,
        "melee_seconds": run.config.melee_seconds,
        "startup_total_ms": (run.startup_ms * 10.0).round() / 10.0,
        "contact": run.contact,
        "sim_seconds_at_contact": (run.sim_seconds_at_contact * 10.0).round() / 10.0,
        "regiments_at_end": run.regiments,
        "entities_max": run.entities_max,
        "calm": calm.as_ref().map(Stats::json),
        "melee": melee.as_ref().map(Stats::json),
        "systems": super::profile::report(&run.spikes),
        "note": "frame_ms = real time between frames, vsync off; calm = opening camera, melee = fixed oblique camera on the first contact",
    });
    let out: Option<PathBuf> = run.config.out.clone();
    write_report(&report, out.as_ref(), exit);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centroid_of_points() {
        assert_eq!(centroid(&[]), None);
        assert_eq!(
            centroid(&[(0.0, 0.0), (10.0, 20.0)]),
            Some(Vec2::new(5.0, 10.0))
        );
    }
}
