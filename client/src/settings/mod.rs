//! Player settings (window mode, volumes, accessibility), their persistence and the Réglages screen.
//!
//! Ports `game/scripts/ui/settings.gd` (the `Settings` autoload: keys, defaults, persistence,
//! live application) and `settings_menu.gd` (the tabbed screen). The kit replaces the Godot
//! widgets; the table of rows is in `rows.rs`, the screen in `screen.rs`, the data in `model.rs`.
//!
//! Open the screen by sending [`OpenSettings`] (the menu lot does) or with F10.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

mod model;
mod rows;
mod screen;

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::ui::UiScale;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode};
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::audio::AudioSettings;
use crate::ui::kit::modal::ReduceMotion;

pub use model::{PATH_ENV, Settings, auto_ui_scale};
pub use rows::{ROWS, Tab};
pub use screen::SettingsScreen;

/// Request to open the Réglages screen (sent by the menu / pause lots). Opening twice is a no-op.
#[derive(Message, Debug, Clone, Copy, Default)]
pub struct OpenSettings;

/// Where the settings are stored and how often they were written.
#[derive(Resource, Debug)]
pub struct SettingsFile {
    pub path: PathBuf,
    pub saves: u32,
    pub last_error: Option<String>,
}

impl SettingsFile {
    pub fn save(&mut self, settings: &Settings) {
        match settings.save_to(&self.path) {
            Ok(()) => {
                self.saves += 1;
                self.last_error = None;
            }
            Err(error) => {
                warn!("settings: cannot write {}: {error}", self.path.display());
                self.last_error = Some(error.to_string());
            }
        }
    }
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let path = Settings::file_path();
        let loaded = Settings::load_from(&path);
        info!("settings: {} ({})", path.display(), if path.exists() { "loaded" } else { "defaults" });
        app.insert_resource(loaded)
            .insert_resource(SettingsFile {
                path,
                saves: 0,
                last_error: None,
            })
            .init_resource::<SettingsScreen>()
            .init_resource::<AudioSettings>()
            .init_resource::<ReduceMotion>()
            .add_message::<OpenSettings>()
            .add_systems(
                Update,
                (
                    screen::open_and_close,
                    screen::handle_widgets,
                    screen::refresh_value_labels,
                    apply_settings,
                    apply_ui_scale,
                )
                    .chain(),
            )
            .register_dump(dump_settings);
    }
}

/// What the window looked like the last time settings were applied to it.
#[derive(Default)]
struct AppliedWindow {
    fullscreen: Option<bool>,
    resolution: Option<(u32, u32)>,
    vsync: Option<bool>,
}

/// Pushes `Settings` into the live resources: audio volumes, reduced motion, window mode, vsync.
/// The saved resolution is not forced at start-up (a window given by the command line or the
/// agent wins until the player picks another one in the screen).
fn apply_settings(
    settings: Res<Settings>,
    mut audio: ResMut<AudioSettings>,
    mut reduce_motion: ResMut<ReduceMotion>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut applied: Local<AppliedWindow>,
) {
    if !settings.is_changed() {
        return;
    }
    let wanted = AudioSettings {
        master: settings.audio_master,
        music: settings.audio_music,
        sfx: settings.audio_sfx,
        voice: settings.audio_voice,
        ambience: settings.audio_ambience,
    };
    if *audio != wanted {
        *audio = wanted;
    }
    if reduce_motion.0 != settings.reduce_motion {
        reduce_motion.0 = settings.reduce_motion;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let first = applied.fullscreen.is_none();
    if applied.vsync != Some(settings.vsync) {
        let present = if settings.vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
        if window.present_mode != present {
            window.present_mode = present;
        }
        applied.vsync = Some(settings.vsync);
    }
    if applied.fullscreen != Some(settings.fullscreen) {
        let is_full = !matches!(window.mode, WindowMode::Windowed);
        if is_full != settings.fullscreen {
            window.mode = if settings.fullscreen {
                WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            } else {
                WindowMode::Windowed
            };
        }
        applied.fullscreen = Some(settings.fullscreen);
    }
    if first {
        applied.resolution = Some(settings.resolution);
    } else if applied.resolution != Some(settings.resolution) {
        if !settings.fullscreen {
            let (width, height) = settings.resolution;
            window.resolution.set(width as f32, height as f32);
        }
        applied.resolution = Some(settings.resolution);
    }
}

/// `UiScale` = automatic scale from the window height x « Taille de l'interface ».
fn apply_ui_scale(
    settings: Res<Settings>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut ui_scale: ResMut<UiScale>,
) {
    let height = windows.single().map_or(900.0, Window::height);
    let wanted = settings.effective_ui_scale(height);
    if (ui_scale.0 - wanted).abs() > 1e-4 {
        ui_scale.0 = wanted;
    }
}

fn dump_settings(world: &World) -> (String, Value) {
    let settings = world.resource::<Settings>();
    let file = world.resource::<SettingsFile>();
    let screen = world.resource::<SettingsScreen>();
    let audio = world.resource::<AudioSettings>();
    let on_disk = std::fs::read_to_string(&file.path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    (
        "settings".into(),
        json!({
            "open": screen.root.is_some(),
            "tab": screen.tab.map(Tab::id),
            "path": file.path.display().to_string(),
            "saves": file.saves,
            "last_error": file.last_error,
            "values": settings,
            "audio": {
                "master": audio.master,
                "music": audio.music,
                "sfx": audio.sfx,
                "voice": audio.voice,
                "ambience": audio.ambience,
            },
            "reduce_motion": world.get_resource::<ReduceMotion>().map(|r| r.0),
            "ui_scale": world.get_resource::<UiScale>().map(|s| s.0),
            "file_exists": on_disk.is_some(),
            "file_audio_music": on_disk.as_ref().and_then(|v| v.get("audio_music")).cloned(),
        }),
    )
}
