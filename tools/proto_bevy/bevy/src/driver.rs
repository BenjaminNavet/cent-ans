//! Frame loop: loading, warm-up, camera path, metrics rows, screenshot, state dump, exit.

use std::fmt::Write as _;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
};
use std::time::Instant;

use bevy::{
    diagnostic::DiagnosticsStore,
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use serde_json::{Value, json};

use crate::Options;
use crate::scene::{self, Scene};
use crate::view::MainCamera;
use crate::walk::WalkMaterial;
use crate::world::{self, BuildParams, Counts, Pending};

/// Consecutive frames without a pending pipeline before the warm-up may end.
const QUIET_FRAMES: u32 = 15;
/// Give up waiting for loading or pipelines after this long.
const MAX_WAIT_S: f32 = 240.0;

/// Facts the render world shares with the main world.
#[derive(Resource, Clone, Default)]
pub struct RenderShared {
    /// Pipelines still compiling.
    pub pending_pipelines: Arc<AtomicUsize>,
    /// Render-world frame time in ms (`f32` bits).
    pub render_ms: Arc<AtomicU32>,
    /// Draw commands of the main view, when the render world could count them (`u32::MAX` = unknown).
    pub draw_calls: Arc<AtomicU32>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Phase {
    Loading,
    Warmup,
    Running,
}

pub struct Row {
    pub frame: u32,
    pub t_s: f32,
    pub frame_ms: f32,
    pub cpu_ms: f32,
    pub gpu_ms: Option<f32>,
    pub draw_calls: Option<u32>,
}

#[derive(Resource)]
pub struct Run {
    pub options: Options,
    pub scene: Scene,
    pub effective: Value,
    pub phase: Phase,
    pub phase_frames: u32,
    pub quiet_frames: u32,
    pub t_path: f32,
    pub frame_index: u32,
    pub warmup_ms: Vec<f32>,
    pub rows: Vec<Row>,
    pub counts: Counts,
    pub started: Instant,
    pub main_start: Instant,
    pub shot_requested: bool,
    pub shot_saved: Arc<AtomicBool>,
    pub metrics_done: bool,
    pub dump_done: bool,
    pub error: Option<String>,
}

impl Run {
    pub fn new(options: Options, scene: Scene, effective: Value) -> Self {
        Self {
            options,
            scene,
            effective,
            phase: Phase::Loading,
            phase_frames: 0,
            quiet_frames: 0,
            t_path: 0.0,
            frame_index: 0,
            warmup_ms: Vec::new(),
            rows: Vec::new(),
            counts: Counts::default(),
            started: Instant::now(),
            main_start: Instant::now(),
            shot_requested: false,
            shot_saved: Arc::new(AtomicBool::new(false)),
            metrics_done: false,
            dump_done: false,
            error: None,
        }
    }
}

/// `First`: start of the main-world frame.
pub fn begin_main_frame(mut run: ResMut<Run>) {
    run.main_start = Instant::now();
}

/// `Update` while loading: builds the world once every asset is ready.
pub fn build_when_loaded(
    mut commands: Commands,
    mut run: ResMut<Run>,
    pending: Res<Pending>,
    mut params: BuildParams,
) {
    if run.phase != Phase::Loading {
        return;
    }
    if run.started.elapsed().as_secs_f32() > MAX_WAIT_S {
        run.error = Some("timeout while loading assets".into());
        return;
    }
    if world::any_failed(&pending, &params.asset_server) {
        run.error = Some("an asset failed to load".into());
        return;
    }
    if !world::all_loaded(&pending, &params.asset_server) {
        return;
    }
    let root = scene::proto_root();
    match world::build_world(&mut commands, &mut params, &run.scene, &root, &pending) {
        Ok(counts) => {
            run.counts = counts;
            run.phase = Phase::Warmup;
            run.phase_frames = 0;
            info!("world built, warming up");
        }
        Err(message) => run.error = Some(message),
    }
}

/// `Update`: moves the camera along the path and feeds the walk clock to the shaders.
pub fn apply_path_and_walk(
    run: Res<Run>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
    mut materials: ResMut<Assets<WalkMaterial>>,
) {
    if run.phase == Phase::Loading {
        return;
    }
    let (pos, look_at) = scene::camera_at(&run.scene.camera_path, run.t_path);
    for mut transform in &mut camera {
        *transform = Transform::from_translation(Vec3::from_array(pos))
            .looking_at(Vec3::from_array(look_at), Vec3::Y);
    }
    for handle in &run.counts.walk_materials {
        if let Some(mut material) = materials.get_mut(handle) {
            material.extension.params.t = run.t_path;
        }
    }
}

fn gpu_ms(store: &DiagnosticsStore) -> Option<f32> {
    let mut total = 0.0;
    let mut found = false;
    for diagnostic in store.iter() {
        let path = diagnostic.path().as_str();
        if path.starts_with("render/")
            && path.ends_with("/elapsed_gpu")
            && path.matches('/').count() == 2
            && let Some(value) = diagnostic.value()
        {
            total += value;
            found = true;
        }
    }
    found.then_some(total as f32)
}

fn write_csv(path: &str, rows: &[Row]) -> std::io::Result<()> {
    let mut text = String::from("frame,t_s,frame_ms,cpu_ms,gpu_ms,draw_calls\n");
    for row in rows {
        let gpu = row.gpu_ms.map(|v| format!("{v:.3}")).unwrap_or_default();
        let draws = row.draw_calls.map(|v| v.to_string()).unwrap_or_default();
        let _ = writeln!(
            text,
            "{},{:.4},{:.3},{:.3},{},{}",
            row.frame, row.t_s, row.frame_ms, row.cpu_ms, gpu, draws
        );
    }
    std::fs::write(path, text)
}

fn dump_state(run: &Run, entities: usize, draw_calls: Option<u32>) -> Value {
    let tail = &run.warmup_ms[run.warmup_ms.len() / 2..];
    let average = tail.iter().sum::<f32>() / tail.len().max(1) as f32;
    json!({
        "engine": "bevy",
        "quality": run.effective["quality"],
        "counts": {
            "placements": run.counts.placements,
            "army": run.counts.army,
            "grass": run.counts.grass,
            "firs": run.counts.firs,
            "entities_or_nodes": entities,
            "draw_calls": draw_calls,
        },
        "effective": run.effective,
        "frame_ms_avg": average,
    })
}

/// `Last`: warm-up bookkeeping, metrics rows, screenshot, dump, exit.
#[allow(clippy::too_many_arguments)]
pub fn drive(
    mut commands: Commands,
    mut run: ResMut<Run>,
    time: Res<Time<Real>>,
    shared: Res<RenderShared>,
    store: Res<DiagnosticsStore>,
    entities: Query<Entity>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(message) = run.error.clone() {
        error!("{message}");
        exit.write(AppExit::error());
        return;
    }
    let frame_ms = time.delta_secs() * 1000.0;
    match run.phase {
        Phase::Loading => {}
        Phase::Warmup => {
            run.phase_frames += 1;
            run.warmup_ms.push(frame_ms);
            if shared.pending_pipelines.load(Ordering::Relaxed) == 0 {
                run.quiet_frames += 1;
            } else {
                run.quiet_frames = 0;
            }
            let waited_too_long = run.started.elapsed().as_secs_f32() > MAX_WAIT_S;
            let warmed = run.phase_frames >= run.scene.bench.warmup_frames
                && run.quiet_frames >= QUIET_FRAMES;
            if warmed || waited_too_long {
                run.phase = Phase::Running;
                run.t_path = 0.0;
                run.frame_index = 0;
                info!("warm-up done after {} frames", run.phase_frames);
            }
        }
        Phase::Running => running_frame(&mut commands, &mut run, &time, &shared, &store, &entities, &mut exit, frame_ms),
    }
}

#[allow(clippy::too_many_arguments)]
fn running_frame(
    commands: &mut Commands,
    run: &mut Run,
    time: &Time<Real>,
    shared: &RenderShared,
    store: &DiagnosticsStore,
    entities: &Query<Entity>,
    exit: &mut MessageWriter<AppExit>,
    frame_ms: f32,
) {
    let draw_calls = match shared.draw_calls.load(Ordering::Relaxed) {
        u32::MAX => None,
        value => Some(value),
    };
    if let Some(path) = run.options.dump_state.clone()
        && !run.dump_done
    {
        let text = serde_json::to_string_pretty(&dump_state(run, entities.iter().count(), draw_calls))
            .unwrap_or_default();
        if let Err(error) = std::fs::write(&path, text) {
            error!("{path}: {error}");
        }
        run.dump_done = true;
    }

    if run.options.metrics.is_some() && !run.metrics_done {
        let main_ms = run.main_start.elapsed().as_secs_f32() * 1000.0;
        let render_ms = f32::from_bits(shared.render_ms.load(Ordering::Relaxed));
        let row = Row {
            frame: run.frame_index,
            t_s: run.t_path,
            frame_ms,
            cpu_ms: main_ms.max(render_ms),
            gpu_ms: gpu_ms(store),
            draw_calls,
        };
        run.rows.push(row);
        if run.t_path >= run.scene.bench.duration_s {
            if let Some(path) = &run.options.metrics
                && let Err(error) = write_csv(path, &run.rows)
            {
                error!("{path}: {error}");
            }
            run.metrics_done = true;
        }
    }

    if let (Some(path), Some(at)) = (run.options.shot.clone(), run.options.shot_at_s.or(Some(0.0)))
        && !run.shot_requested
        && run.t_path >= at
    {
        run.shot_requested = true;
        let saved = run.shot_saved.clone();
        commands.spawn(Screenshot::primary_window()).observe(
            move |captured: On<ScreenshotCaptured>| {
                match captured.image.clone().try_into_dynamic() {
                    Ok(image) => {
                        if let Err(error) = image.to_rgb8().save(&path) {
                            error!("{path}: {error}");
                        }
                    }
                    Err(error) => error!("screenshot conversion: {error}"),
                }
                saved.store(true, Ordering::SeqCst);
            },
        );
    }

    let wants_exit = run.options.metrics.is_some()
        || run.options.shot.is_some()
        || run.options.dump_state.is_some();
    let done = (run.options.metrics.is_none() || run.metrics_done)
        && (run.options.shot.is_none() || run.shot_saved.load(Ordering::SeqCst))
        && (run.options.dump_state.is_none() || run.dump_done);
    if wants_exit && done {
        exit.write(AppExit::Success);
    }
    run.t_path += time.delta_secs();
    run.frame_index += 1;
}
