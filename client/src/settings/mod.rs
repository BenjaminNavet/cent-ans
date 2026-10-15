//! Player settings (window mode, volumes, accessibility), their persistence and the Réglages screen.
//!
//! Ports `game/scripts/ui/settings.gd` (the `Settings` autoload: keys, defaults, persistence,
//! live application) and `settings_menu.gd` (the tabbed screen). The kit replaces the Godot
//! widgets; the table of rows is in `rows.rs`, the screen in `screen.rs`, the data in `model.rs`.
//!
//! Open the screen by sending [`OpenSettings`] (the menu lot does) or with F10.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

pub mod keymap;
mod model;
pub mod palette;
pub mod quality;
mod rows;
mod screen;

use std::path::PathBuf;

use bevy::light::DirectionalLightShadowMap;
use bevy::prelude::*;
use bevy::ui::UiScale;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode};
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::audio::AudioSettings;
use crate::ui::kit::modal::ReduceMotion;

pub use keymap::{ACTIONS, ActionDef, Chord, KeyMap};
pub use model::{PATH_ENV, Settings, auto_ui_scale};
pub use quality::{QualityPresets, RenderQuality};
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
        let bindings = loaded.bindings.clone();
        let quality_presets = QualityPresets::load();
        let initial_quality = RenderQuality::resolve(
            &loaded,
            &quality_presets,
            if cfg!(target_os = "macos") {
                "high"
            } else {
                "medium"
            },
        );
        info!(
            "settings: {} ({})",
            path.display(),
            if path.exists() { "loaded" } else { "defaults" }
        );
        app.insert_resource(loaded)
            .insert_resource(SettingsFile {
                path,
                saves: 0,
                last_error: None,
            })
            .insert_resource(KeyMap::from_bindings(&bindings))
            .init_resource::<AppliedReport>()
            .insert_resource(quality_presets)
            .insert_resource(initial_quality)
            .init_resource::<DirectionalLightShadowMap>()
            .init_resource::<SettingsScreen>()
            .init_resource::<AudioSettings>()
            .init_resource::<ReduceMotion>()
            .init_resource::<palette::Palette>()
            .add_message::<OpenSettings>()
            .add_systems(
                Update,
                (
                    keymap::sync_keymap,
                    screen::open_and_close,
                    screen::capture_key,
                    screen::handle_widgets,
                    screen::refresh_value_labels,
                    screen::refresh_key_buttons,
                    toggle_fullscreen,
                    apply_settings,
                    apply_ui_scale,
                    palette::sync_palette,
                )
                    .chain()
                    .before(crate::menu::pause::pause_escape),
            )
            .add_systems(
                Update,
                (
                    quality::resolve_quality,
                    (quality::capture_suns, quality::capture_far),
                    (
                        quality::apply_msaa,
                        quality::apply_shadows,
                        quality::apply_view_distance,
                    ),
                    report_applied,
                )
                    .chain(),
            )
            .register_dump(dump_settings);
    }
}

/// What the render settings did to the world, for the dump (`settings.applied`).
#[derive(Resource, Default)]
struct AppliedReport(Value);

#[allow(clippy::type_complexity)]
fn report_applied(
    mut report: ResMut<AppliedReport>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Msaa, &Projection, &Camera), With<Camera3d>>,
    suns: Query<(&DirectionalLight, Option<&bevy::light::CascadeShadowConfig>)>,
    shadow_map: Res<DirectionalLightShadowMap>,
    keymap: Res<KeyMap>,
) {
    let cams: Vec<Value> = cameras
        .iter()
        .map(|(msaa, projection, camera)| {
            let far = match projection {
                Projection::Perspective(p) => p.far,
                _ => 0.0,
            };
            json!({"msaa": msaa.samples(), "far": far, "active": camera.is_active})
        })
        .collect();
    let suns: Vec<Value> = suns
        .iter()
        .map(|(light, cascades)| {
            json!({
                "shadows": light.shadow_maps_enabled,
                "cascade_far": cascades.and_then(|c| c.bounds.last().copied()),
            })
        })
        .collect();
    report.0 = json!({
        "window_mode": windows.single().ok().map(|w| format!("{:?}", w.mode)),
        "cameras": cams,
        "suns": suns,
        "shadow_map": shadow_map.size,
        "keymap_custom": keymap.custom_count(),
    });
}

/// The « Plein écran / fenêtré » key (`toggle_fullscreen`, F11) flips the setting and saves it.
fn toggle_fullscreen(
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<KeyMap>,
    screen: Res<SettingsScreen>,
    mut settings: ResMut<Settings>,
    mut file: ResMut<SettingsFile>,
) {
    if screen.capturing() || !keymap.just_pressed("toggle_fullscreen", &keys) {
        return;
    }
    settings.fullscreen = !settings.fullscreen;
    if screen.root.is_none() {
        file.save(&settings);
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
        battle_music: settings.audio_battle_music,
        battle_sfx: settings.audio_battle_sfx,
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
    let keymap = world.resource::<KeyMap>();
    let quality = world.resource::<RenderQuality>();
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
            "palette": world.resource::<palette::Palette>(),
            "audio": {
                "master": audio.master,
                "music": audio.music,
                "sfx": audio.sfx,
                "voice": audio.voice,
                "ambience": audio.ambience,
            },
            "reduce_motion": world.get_resource::<ReduceMotion>().map(|r| r.0),
            "ui_scale": world.get_resource::<UiScale>().map(|s| s.0),
            "keymap": {
                "actions": ACTIONS.len(),
                "custom": keymap.custom_count(),
                "layout": settings.keyboard_layout,
                "resolved": ACTIONS
                    .iter()
                    .map(|def| (def.id.to_owned(), json!(keymap.label(def.id, "qwerty"))))
                    .collect::<serde_json::Map<String, Value>>(),
                "capturing": screen.capture.as_ref().map(|(action, slot)| format!("{action}:{slot}")),
            },
            "quality": {
                "level": quality.level,
                "msaa_battle": format!("{:?}", quality.msaa_battle),
                "ssao_battle": quality.ssao_battle,
                "msaa_campaign": format!("{:?}", quality.msaa_campaign),
                "shadows": quality.shadows,
                "shadow_map_size": quality.shadow_map_size,
                "shadow_distance": quality.shadow_distance,
                "map_shadow_range": quality.map_shadow_range,
                "view_distance": quality.view_distance,
                "veg_max_distance": quality.veg_max_distance,
                "model_shadow_distance": quality.model_shadow_distance,
            },
            "applied": world.resource::<AppliedReport>().0,
            "file_bindings": on_disk.as_ref().and_then(|v| v.get("bindings")).cloned(),
            "file_exists": on_disk.is_some(),
            "file_audio_music": on_disk.as_ref().and_then(|v| v.get("audio_music")).cloned(),
        }),
    )
}
