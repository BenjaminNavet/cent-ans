//! Bevy side of the engine measurement prototype (P0): renders `scene.json`, writes metrics,
//! screenshots and a text dump of the world so agents can inspect it without captures.

mod driver;
mod instancing;
mod mips;
mod scene;
mod ui;
mod view;
mod walk;
mod world;

use std::process::ExitCode;
use std::sync::atomic::Ordering;
use std::time::Instant;

use bevy::{
    asset::AssetPlugin,
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
    light::atmosphere::ScatteringMedium,
    pbr::MaterialPlugin,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        diagnostic::RenderDiagnosticsPlugin,
        render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases},
        render_resource::PipelineCache,
    },
    window::{PresentMode, WindowResolution},
};

use driver::{RenderShared, Run};
use walk::WalkMaterial;

/// Command-line options shared with the Godot client.
#[derive(Debug, Default, Clone)]
pub struct Options {
    pub scene: Option<String>,
    pub metrics: Option<String>,
    pub shot: Option<String>,
    pub shot_at_s: Option<f32>,
    pub dump_state: Option<String>,
    /// Bevy-only: enable `RenderDiagnosticsPlugin` (Metal timestamp counters; can fail when the
    /// GPU is shared, so off by default and `gpu_ms` stays empty).
    pub gpu_timing: bool,
    pub overrides: Vec<(String, String)>,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let mut value = || {
            index += 1;
            args.get(index)
                .cloned()
                .ok_or(format!("{flag} expects a value"))
        };
        match flag {
            "--scene" => options.scene = Some(value()?),
            "--metrics" => options.metrics = Some(value()?),
            "--shot" => options.shot = Some(value()?),
            "--at" => options.shot_at_s = Some(value()?.parse().map_err(|e| format!("--at: {e}"))?),
            "--gpu-timing" => options.gpu_timing = true,
            "--dump-state" => options.dump_state = Some(value()?),
            "--set" => {
                let pair = value()?;
                let (key, val) = pair.split_once('=').ok_or("--set expects key=value")?;
                options.overrides.push((key.to_owned(), val.to_owned()));
            }
            other => return Err(format!("unknown option {other}")),
        }
        index += 1;
    }
    Ok(options)
}

/// Render-world timing and pipeline status, published through `RenderShared`.
#[derive(Resource)]
struct RenderFrameStart(Instant);

fn render_frame_begin(mut start: ResMut<RenderFrameStart>) {
    start.0 = Instant::now();
}

fn render_frame_end(
    start: Res<RenderFrameStart>,
    shared: Res<RenderShared>,
    cache: Res<PipelineCache>,
) {
    let ms = start.0.elapsed().as_secs_f32() * 1000.0;
    shared.render_ms.store(ms.to_bits(), Ordering::Relaxed);
    shared
        .pending_pipelines
        .store(cache.waiting_pipelines().count(), Ordering::Relaxed);
    shared.batches_drawn.store(
        instancing::DRAWN_BATCHES.swap(0, Ordering::Relaxed),
        Ordering::Relaxed,
    );
}

/// Draw submissions of the camera's opaque and alpha-masked phases: one per multi-draw batch
/// set, one per batchable bin, one per unbatchable entity (shadow and prepass phases excluded).
fn count_draws(
    opaque: Res<ViewBinnedRenderPhases<Opaque3d>>,
    alpha: Res<ViewBinnedRenderPhases<AlphaMask3d>>,
    transparent: Res<ViewSortedRenderPhases<Transparent3d>>,
    shared: Res<RenderShared>,
) {
    fn count<T: bevy::render::render_phase::BinnedPhaseItem>(
        phases: &ViewBinnedRenderPhases<T>,
    ) -> usize {
        phases
            .values()
            .map(|phase| {
                phase.multidrawable_meshes.len()
                    + phase.batchable_meshes.len()
                    + phase
                        .unbatchable_meshes
                        .values()
                        .map(|group| group.entities.len())
                        .sum::<usize>()
            })
            .sum()
    }
    let sorted: usize = transparent.values().map(|phase| phase.items.len()).sum();
    let total = (count(&opaque) + count(&alpha) + sorted) as u32;
    shared.draw_calls.store(total, Ordering::Relaxed);
}

fn start_loading(mut commands: Commands, run: Res<Run>, asset_server: Res<AssetServer>) {
    match world::start_loading(&run.scene, &asset_server) {
        Ok(pending) => commands.insert_resource(pending),
        Err(message) => {
            eprintln!("{message}");
            commands.write_message(AppExit::error());
        }
    }
}

fn spawn_scene_view(
    mut commands: Commands,
    run: Res<Run>,
    mut media: ResMut<Assets<ScatteringMedium>>,
    mut fonts: ResMut<Assets<Font>>,
    mut images: ResMut<Assets<Image>>,
) {
    let camera = view::spawn_view(&mut commands, &run.scene, &mut media, &mut images);
    let font = ui::load_font(&mut fonts);
    ui::spawn_panel(&mut commands, &run.scene.ui_panel, &font, camera);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse_options(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };
    let root = scene::proto_root();
    let scene_path = options
        .scene
        .as_ref()
        .map_or_else(|| root.join("scene.json"), std::path::PathBuf::from);
    let (scene, effective) = match scene::load_scene(&scene_path, &options.overrides) {
        Ok(loaded) => loaded,
        Err(message) => {
            eprintln!("scene: {message}");
            return ExitCode::FAILURE;
        }
    };
    let [width, height] = scene.bench.resolution;
    let shared = RenderShared::default();
    shared.draw_calls.store(u32::MAX, Ordering::Relaxed);

    let mut app = App::new();
    if options.gpu_timing {
        app.add_plugins(RenderDiagnosticsPlugin);
    }
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "proto-bevy".into(),
                    resolution: WindowResolution::new(width, height)
                        .with_scale_factor_override(1.0),
                    present_mode: PresentMode::AutoNoVsync,
                    resizable: false,
                    // Never take the player's focus (CONTRACT.md "Focus du joueur").
                    focused: false,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_plugins(MaterialPlugin::<WalkMaterial>::default())
    .add_plugins(instancing::InstancingPlugin)
    .insert_resource(Run::new(options, scene, effective))
    .insert_resource(shared.clone())
    .add_systems(Startup, (start_loading, spawn_scene_view))
    .add_systems(First, driver::begin_main_frame)
    .add_systems(
        Update,
        (
            driver::build_when_loaded,
            driver::apply_path_and_walk,
            ui::update_tooltip,
        ),
    )
    .add_systems(Last, driver::drive);

    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app
            .insert_resource(shared)
            .insert_resource(RenderFrameStart(Instant::now()))
            .add_systems(
                Render,
                (
                    render_frame_begin.before(RenderSystems::ExtractCommands),
                    render_frame_end.in_set(RenderSystems::PostCleanup),
                    count_draws.in_set(RenderSystems::PhaseSort),
                ),
            );
    }
    app.run();
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_set_overrides() {
        let args: Vec<String> = ["--set", "vegetation.grass=0.5"].map(String::from).to_vec();
        let options = parse_options(&args).unwrap();
        assert_eq!(
            options.overrides,
            vec![("vegetation.grass".into(), "0.5".into())]
        );
    }
}
