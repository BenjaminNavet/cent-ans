//! Campaign map bench (`--bench-map`): scripted camera path, real frame times, JSON report.
//!
//! Port of `game/scripts/dev/map_bench.gd`: a warm-up, a pan over Normandy / Île-de-France at
//! `--bench-distance` (30) for `--bench-seconds` (20 s), then the « descent » over Rouen, the
//! Grande Chartreuse and Paris (strategic 150 -> site floor -> back, logarithmic in the distance,
//! 6 s each way, 1.5 s hold). Frame times are the real time between two frames (`Time<Real>`),
//! vsync is switched off at the start (`AutoNoVsync`) so the GPU is what is measured.
//!
//! Differences from Godot: no separate « zoom » phase (the descent covers it), no relief quadtree
//! statistics, and no draw-call counter exists in Bevy: `visible_meshes` (entities with a `Mesh3d`
//! whose `ViewVisibility` is set, sampled every 10 frames) stands in; instanced batches count once.
//!
//! The report is written to `--bench-out` and logged as `CampaignMap: bench_map {...}`; the run
//! then exits (code 0), or fails with `AGENT_FATAL` if the campaign never became ready.
//! Run it through `tools/bevy_bench/bench.sh` (release build).

use std::path::PathBuf;
use std::time::Instant;

use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
use serde_json::{Value, json};

use super::cli::{BenchConfig, Cli};
use crate::app::AppState;
use crate::campaign::camera::CameraSet;
use crate::campaign::camera::rig::{CampaignRig, Ground};
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::Surface;

/// Pan stops (map x, z), as in `map_bench.gd`: Caen, Rouen, Paris, Chartres, Évreux.
pub const PAN_PATH: [Vec2; 5] = [
    Vec2::new(1880.0, 3125.0),
    Vec2::new(2097.0, 3099.0),
    Vec2::new(2213.0, 3204.0),
    Vec2::new(2150.0, 3285.0),
    Vec2::new(2080.0, 3170.0),
];
/// Descent sites: Rouen, Grande Chartreuse, Paris.
pub const DESCENT_SITES: [Vec2; 3] = [
    Vec2::new(2096.5, 3099.7),
    Vec2::new(2537.6, 3772.0),
    Vec2::new(2212.9, 3204.5),
];
const WARMUP_FRAMES: u32 = 90;
const DESCENT_SECONDS: f32 = 6.0;
const DESCENT_HOLD: f32 = 1.5;
const DESCENT_FAR: f32 = 150.0;
/// Frames waited for the campaign to be ready before giving up.
const READY_TIMEOUT_FRAMES: u32 = 6000;
const SPIKE_MS: f32 = 50.0;
/// Frames slower than this are attributed to systems in the `systems` section (feature `profile`).
const PROFILE_SPIKE_MS: f32 = 33.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    WaitCampaign,
    Warmup,
    Pan,
    /// Site index, leg: 0 down, 1 hold, 2 up.
    Descent(usize, u8),
    Done,
}

#[derive(Resource)]
struct BenchRun {
    config: BenchConfig,
    started: Instant,
    startup_ms: f64,
    phase: Phase,
    phase_time: f32,
    frames: u32,
    frame_ms: Vec<f32>,
    visible: Vec<u32>,
    pan: Option<Stats>,
    descent: Vec<f32>,
    descent_visible: Vec<u32>,
    entities_max: usize,
    /// `(end_ms, frame_ms)` of the frames over `PROFILE_SPIKE_MS` (profiler clock).
    spikes: Vec<(f64, f32)>,
}

/// Percentiles of a series (sorted in place); `q` in 0..=1, index `min(n-1, floor(n*q))`.
pub fn percentile(sorted: &[f32], q: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() as f32 * q) as usize).min(sorted.len() - 1)]
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Stats {
    frames: usize,
    mean_fps: f32,
    p50: f32,
    p95: f32,
    p99: f32,
    max: f32,
    over_50: usize,
    worst_1pct_fps: f32,
    visible_p50: u32,
    visible_max: u32,
}

impl Stats {
    pub(super) fn of(frame_ms: &[f32], visible: &[u32]) -> Stats {
        let mut sorted = frame_ms.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let n = sorted.len();
        let total: f32 = sorted.iter().sum();
        let worst = &sorted[n - n.div_ceil(100).min(n)..];
        let worst_mean = worst.iter().sum::<f32>() / worst.len().max(1) as f32;
        let mut seen = visible.to_vec();
        seen.sort_unstable();
        Stats {
            frames: n,
            mean_fps: if total > 0.0 {
                n as f32 * 1000.0 / total
            } else {
                0.0
            },
            p50: percentile(&sorted, 0.5),
            p95: percentile(&sorted, 0.95),
            p99: percentile(&sorted, 0.99),
            max: sorted.last().copied().unwrap_or(0.0),
            over_50: sorted.iter().filter(|ms| **ms > SPIKE_MS).count(),
            worst_1pct_fps: if worst_mean > 0.0 {
                1000.0 / worst_mean
            } else {
                0.0
            },
            visible_p50: seen.get(seen.len() / 2).copied().unwrap_or(0),
            visible_max: seen.last().copied().unwrap_or(0),
        }
    }

    pub(super) fn json(&self) -> Value {
        let r = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "frames": self.frames,
            "fps_mean": r(self.mean_fps),
            "frame_ms_p50": r(self.p50),
            "frame_ms_p95": r(self.p95),
            "frame_ms_p99": r(self.p99),
            "frame_ms_max": r(self.max),
            "worst_1pct_fps": r(self.worst_1pct_fps),
            "frames_over_50ms": self.over_50,
            "visible_meshes_p50": self.visible_p50,
            "visible_meshes_max": self.visible_max,
        })
    }
}

/// Position on the pan at `t` in 0..=1: equal time per segment, linear.
pub fn pan_point(t: f32) -> Vec2 {
    let segments = (PAN_PATH.len() - 1) as f32;
    let scaled = t.clamp(0.0, 1.0) * segments;
    let index = (scaled.floor() as usize).min(PAN_PATH.len() - 2);
    PAN_PATH[index].lerp(PAN_PATH[index + 1], scaled - index as f32)
}

/// Distance of the descent at fraction `t` (0 = far, 1 = site floor), logarithmic.
pub fn descent_distance(t: f32, floor: f32) -> f32 {
    let (far, near) = (DESCENT_FAR.ln(), floor.max(0.1).ln());
    (far + (near - far) * t.clamp(0.0, 1.0)).exp()
}

pub fn add(app: &mut App, cli: &Cli) {
    let Some(config) = cli.bench.clone() else {
        return;
    };
    if config.battle {
        super::bench_battle::add(app, config);
        return;
    }
    app.insert_resource(BenchRun {
        config,
        started: Instant::now(),
        startup_ms: 0.0,
        phase: Phase::WaitCampaign,
        phase_time: 0.0,
        frames: 0,
        frame_ms: Vec::new(),
        visible: Vec::new(),
        pan: None,
        descent: Vec::new(),
        descent_visible: Vec::new(),
        entities_max: 0,
        spikes: Vec::new(),
    })
    .add_systems(
        Update,
        bench_step
            .before(CameraSet)
            .run_if(resource_exists::<BenchRun>),
    );
}

#[allow(clippy::too_many_arguments)]
fn bench_step(
    mut run: ResMut<BenchRun>,
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    rig: Option<ResMut<CampaignRig>>,
    surface: Option<Res<Surface>>,
    relief: Option<Res<ReliefState>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    meshes: Query<&ViewVisibility, With<Mesh3d>>,
    all: Query<Entity>,
    mut exit: MessageWriter<AppExit>,
) {
    run.frames += 1;
    let dt = time.delta_secs();
    let frame_ms = dt * 1000.0;
    let Some(mut rig) = rig.filter(|_| *state.get() == AppState::Campaign) else {
        if run.phase == Phase::WaitCampaign && run.frames > READY_TIMEOUT_FRAMES {
            error!("AGENT_FATAL: bench-map: the campaign never became ready");
            exit.write(AppExit::error());
            run.phase = Phase::Done;
        }
        return;
    };
    let height = |x: f32, z: f32| match (&surface, &relief) {
        (Some(surface), Some(relief)) => surface.display_height_at(relief, x, z),
        _ => 0.0,
    };
    let ground = Ground {
        height: &height,
        pyramid: surface.as_ref().map(|s| s.pyramid.as_ref()),
    };
    let place = |rig: &mut CampaignRig, point: Vec2, distance: f32| {
        rig.target_focus = Vec3::new(point.x, 0.0, point.y);
        rig.target_distance = distance.clamp(rig.floor_distance.max(0.1), rig.max_distance);
    };

    // Measured frame: every frame after the warm-up.
    let measuring = !matches!(run.phase, Phase::WaitCampaign | Phase::Warmup | Phase::Done);
    if measuring && dt > 0.0 {
        if frame_ms > PROFILE_SPIKE_MS {
            run.spikes.push((super::profile::now_ms(), frame_ms));
        }
        let sample_visible = run.frames.is_multiple_of(10);
        let visible = if sample_visible {
            meshes.iter().filter(|v| v.get()).count() as u32
        } else {
            0
        };
        match run.phase {
            Phase::Pan => {
                run.frame_ms.push(frame_ms);
                if sample_visible {
                    run.visible.push(visible);
                }
            }
            _ => {
                run.descent.push(frame_ms);
                if sample_visible {
                    run.descent_visible.push(visible);
                }
            }
        }
    }
    run.phase_time += dt;
    if run.frames.is_multiple_of(30) {
        let count = all.iter().count();
        run.entities_max = run.entities_max.max(count);
    }

    match run.phase {
        Phase::WaitCampaign => {
            for mut window in &mut windows {
                window.present_mode = PresentMode::AutoNoVsync;
            }
            rig.edge_pan_enabled = false;
            let start = if run.config.descent_only {
                DESCENT_SITES[0]
            } else {
                PAN_PATH[0]
            };
            place(&mut rig, start, run.config.distance);
            rig.snap(&ground);
            run.startup_ms = run.started.elapsed().as_secs_f64() * 1000.0;
            run.phase = Phase::Warmup;
            run.phase_time = 0.0;
            run.frames = 0;
        }
        Phase::Warmup => {
            if run.frames >= WARMUP_FRAMES {
                super::profile::reset();
                run.phase = if run.config.descent_only {
                    Phase::Descent(0, 0)
                } else {
                    Phase::Pan
                };
                run.phase_time = 0.0;
            }
        }
        Phase::Pan => {
            let t = run.phase_time / run.config.seconds.max(0.1);
            place(&mut rig, pan_point(t), run.config.distance);
            if t >= 1.0 {
                run.pan =
                    (!run.frame_ms.is_empty()).then(|| Stats::of(&run.frame_ms, &run.visible));
                if run.config.pan_only {
                    finish(&mut run, &mut exit);
                } else {
                    run.phase = Phase::Descent(0, 0);
                    run.phase_time = 0.0;
                    place(&mut rig, DESCENT_SITES[0], DESCENT_FAR);
                    rig.snap(&ground);
                }
            }
        }
        Phase::Descent(site, leg) => {
            let point = DESCENT_SITES[site];
            let floor = rig.min_distance_at(Vec3::new(point.x, 0.0, point.y), &ground);
            let floor = floor.max(rig.floor_distance);
            let (length, next_leg) = match leg {
                0 => (DESCENT_SECONDS, 1),
                1 => (DESCENT_HOLD, 2),
                _ => (DESCENT_SECONDS, 0),
            };
            let progress = (run.phase_time / length).min(1.0);
            let distance = match leg {
                0 => descent_distance(progress, floor),
                1 => floor,
                _ => descent_distance(1.0 - progress, floor),
            };
            place(&mut rig, point, distance);
            if run.phase_time >= length {
                run.phase_time = 0.0;
                if leg == 2 {
                    if site + 1 < DESCENT_SITES.len() {
                        run.phase = Phase::Descent(site + 1, 0);
                        place(&mut rig, DESCENT_SITES[site + 1], DESCENT_FAR);
                        rig.snap(&ground);
                    } else {
                        finish(&mut run, &mut exit);
                    }
                } else {
                    run.phase = Phase::Descent(site, next_leg);
                }
            }
        }
        Phase::Done => {}
    }
}

/// Writes `report` (pretty JSON) to `out`, logs it, and exits: shared with the battle bench.
pub(super) fn write_report(
    report: &Value,
    out: Option<&PathBuf>,
    exit: &mut MessageWriter<AppExit>,
) {
    let text = serde_json::to_string_pretty(report).unwrap_or_default();
    info!(
        "CampaignMap: bench_{} {}",
        report["bench"].as_str().unwrap_or("map"),
        report
    );
    if let Some(path) = out {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) = std::fs::write::<&PathBuf, _>(path, text) {
            error!("AGENT_FATAL: {}: {error}", path.display());
            exit.write(AppExit::error());
            return;
        }
    }
    info!("AGENT_EXIT bench");
    exit.write(AppExit::Success);
}

fn finish(run: &mut BenchRun, exit: &mut MessageWriter<AppExit>) {
    run.phase = Phase::Done;
    let descent = (!run.descent.is_empty()).then(|| Stats::of(&run.descent, &run.descent_visible));
    let report = json!({
        "bench": "map",
        "engine": "bevy",
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "distance": run.config.distance,
        "pan_seconds": run.config.seconds,
        "startup_total_ms": (run.startup_ms * 10.0).round() / 10.0,
        "entities_max": run.entities_max,
        "pan": run.pan.as_ref().map(Stats::json),
        "descent": descent.as_ref().map(Stats::json),
        "systems": super::profile::report(&run.spikes),
        "note": "frame_ms = real time between frames, vsync off; visible_meshes replaces draw calls",
    });
    write_report(&report, run.config.out.as_ref(), exit);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles() {
        let sorted: Vec<f32> = (1..=100).map(|n| n as f32).collect();
        assert_eq!(percentile(&sorted, 0.5), 51.0);
        assert_eq!(percentile(&sorted, 0.95), 96.0);
        assert_eq!(percentile(&sorted, 0.99), 100.0);
        assert_eq!(percentile(&[], 0.5), 0.0);
    }

    #[test]
    fn stats_counts_spikes() {
        let frames = [10.0, 10.0, 10.0, 60.0];
        let stats = Stats::of(&frames, &[5, 9]);
        assert_eq!(stats.over_50, 1);
        assert_eq!(stats.max, 60.0);
        assert_eq!(stats.visible_max, 9);
        assert!((stats.worst_1pct_fps - 1000.0 / 60.0).abs() < 0.01);
    }

    #[test]
    fn pan_and_descent_paths() {
        assert_eq!(pan_point(0.0), PAN_PATH[0]);
        assert_eq!(pan_point(1.0), PAN_PATH[4]);
        assert_eq!(pan_point(0.25), PAN_PATH[1]);
        assert!((descent_distance(0.0, 5.0) - 150.0).abs() < 0.01);
        assert!((descent_distance(1.0, 5.0) - 5.0).abs() < 0.01);
    }
}
