//! Bevy 0.20 developer tools (lot devtools): diagnostics overlay, render debug overlay, system
//! stepping. Exists only with the cargo feature `dev`; release builds contain none of it.
//!
//! Enabled at startup by `CENT_ANS_DEVTOOLS=<list>` (comma separated, `all` = everything):
//! - `diagnostics` (or `fps`): draggable window with FPS, frame time and entity count (UI overlay,
//!   never on by default so that captures stay clean);
//! - `render`: render debug overlay (`F9` cycles the mode, `F10` the opacity; `F2` is taken by the
//!   battle camera bookmarks);
//! - `stepping`: system stepping of `Update` starts enabled (see the console command `step`).
//!
//! Console commands (always registered): `overlay` toggles the diagnostics window, `step
//! on|off|frame|continue|status` drives the stepping.

use bevy::dev_tools::diagnostics_overlay::{DiagnosticsOverlay, DiagnosticsOverlayPlugin};
use bevy::dev_tools::render_debug::{RenderDebugOverlayKeybindings, RenderDebugOverlayPlugin};
use bevy::diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::ecs::schedule::Stepping;
use bevy::prelude::*;
use bevy_console::{AddConsoleCommand, ConsoleCommand, reply};
use clap::Parser;

/// The tools requested by `CENT_ANS_DEVTOOLS`.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Requested {
    pub diagnostics: bool,
    pub render: bool,
    pub stepping: bool,
}

/// Parses a `CENT_ANS_DEVTOOLS` value (unknown names are ignored).
pub fn parse_requested(text: &str) -> Requested {
    let mut requested = Requested::default();
    for name in text.split(',').map(|n| n.trim().to_ascii_lowercase()) {
        match name.as_str() {
            "all" => {
                requested = Requested {
                    diagnostics: true,
                    render: true,
                    stepping: true,
                }
            }
            "diagnostics" | "fps" => requested.diagnostics = true,
            "render" => requested.render = true,
            "stepping" => requested.stepping = true,
            _ => {}
        }
    }
    requested
}

pub struct BevyToolsPlugin;

impl Plugin for BevyToolsPlugin {
    fn build(&self, app: &mut App) {
        let requested = std::env::var("CENT_ANS_DEVTOOLS")
            .map(|text| parse_requested(&text))
            .unwrap_or_default();
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        if !app.is_plugin_added::<EntityCountDiagnosticsPlugin>() {
            app.add_plugins(EntityCountDiagnosticsPlugin::default());
        }
        app.add_plugins((DiagnosticsOverlayPlugin, RenderDebugOverlayPlugin));
        app.insert_resource(RenderDebugOverlayKeybindings {
            enable_keybindings: requested.render,
            cycle_mode: KeyCode::F9,
            cycle_opacity: KeyCode::F10,
        });
        let mut stepping = Stepping::new();
        stepping.add_schedule(Update);
        if requested.stepping {
            stepping.enable();
        }
        app.insert_resource(stepping);
        if requested.diagnostics {
            app.add_systems(Startup, spawn_overlay);
        }
        app.add_console_command::<OverlayCommand, _>(overlay_command)
            .add_console_command::<StepCommand, _>(step_command);
        if requested != Requested::default() {
            info!("AGENT_DEVTOOLS {requested:?}");
        }
    }
}

/// Window title, also used to find the window again.
const OVERLAY_TITLE: &str = "Diagnostics Cent Ans";

fn overlay_bundle() -> DiagnosticsOverlay {
    DiagnosticsOverlay::new(
        OVERLAY_TITLE,
        vec![
            FrameTimeDiagnosticsPlugin::FPS.into(),
            FrameTimeDiagnosticsPlugin::FRAME_TIME.into(),
            EntityCountDiagnosticsPlugin::ENTITY_COUNT.into(),
        ],
    )
}

fn spawn_overlay(mut commands: Commands) {
    commands.spawn(overlay_bundle());
}

/// Toggles the diagnostics window (FPS, frame time, entity count)
#[derive(Parser, ConsoleCommand)]
#[command(name = "overlay")]
struct OverlayCommand;

fn overlay_command(
    mut cmd: ConsoleCommand<OverlayCommand>,
    mut commands: Commands,
    existing: Query<(Entity, &DiagnosticsOverlay)>,
) {
    if cmd.take().is_none() {
        return;
    }
    let found: Vec<Entity> = existing
        .iter()
        .filter(|(_, overlay)| overlay.title == OVERLAY_TITLE)
        .map(|(entity, _)| entity)
        .collect();
    if found.is_empty() {
        commands.spawn(overlay_bundle());
        reply!(cmd, "diagnostics overlay on");
    } else {
        for entity in found {
            commands.entity(entity).despawn();
        }
        reply!(cmd, "diagnostics overlay off");
    }
    cmd.ok();
}

/// System stepping of `Update`: on, off, frame (one frame), continue (until breakpoint), status
#[derive(Parser, ConsoleCommand)]
#[command(name = "step")]
struct StepCommand {
    /// on | off | frame | continue | status
    action: String,
}

fn step_command(mut cmd: ConsoleCommand<StepCommand>, mut stepping: ResMut<Stepping>) {
    let Some(Ok(StepCommand { action })) = cmd.take() else {
        return;
    };
    match action.as_str() {
        "on" => {
            stepping.enable();
        }
        "off" => {
            stepping.disable();
        }
        "frame" => {
            stepping.step_frame();
        }
        "continue" => {
            stepping.continue_frame();
        }
        "status" => {}
        other => {
            reply!(
                cmd,
                "unknown action {other} (on, off, frame, continue, status)"
            );
            cmd.failed();
            return;
        }
    }
    reply!(
        cmd,
        "stepping {}",
        if stepping.is_enabled() { "on" } else { "off" }
    );
    cmd.ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lists() {
        assert_eq!(parse_requested(""), Requested::default());
        let some = parse_requested("fps, render");
        assert!(some.diagnostics && some.render && !some.stepping);
        let all = parse_requested("all");
        assert!(all.diagnostics && all.render && all.stepping);
        assert_eq!(parse_requested("nonsense"), Requested::default());
    }
}
