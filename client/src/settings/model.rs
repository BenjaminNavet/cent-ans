//! The `Settings` resource: every player setting of the Godot `settings.gd` (`DEFAULTS`),
//! grouped by settings tab, and its JSON persistence in the user config directory.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Environment variable overriding the settings file (tests, background runs).
pub const PATH_ENV: &str = "CENT_ANS_SETTINGS_PATH";

/// Window and graphics (`video/*`, `interface/ui_size`, `interface/text_size`).
#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    // ---- Affichage ----
    /// `video/fullscreen` (windowed by default).
    pub fullscreen: bool,
    /// `video/resolution` (windowed size, logical pixels).
    pub resolution: (u32, u32),
    /// `video/vsync`.
    pub vsync: bool,
    /// `video/quality`: `auto`, `low`, `medium`, `high`, `ultra`.
    pub quality: String,
    /// `video/msaa`: `auto` (the quality preset), `off`, `2x`, `4x`.
    pub msaa: String,
    /// `video/shadows`: sun shadows on or off (the preset sets their size and distance).
    pub shadows: bool,
    /// `video/view_distance`: multiplier of the camera far planes.
    pub view_distance: f32,
    /// `video/upscale`: `auto`, `off`, `quality`, `performance`.
    pub upscale: String,
    /// `video/texture_quality`: `high` or `medium`.
    pub texture_quality: String,
    /// `interface/ui_size`: multiplies the automatic UI scale (`UiScale`).
    pub ui_size: f32,
    /// `interface/text_size`: multiplier of text sizes (exposed; the kit does not read it yet).
    pub text_size: f32,
    // ---- Audio (`audio/bus_*`, `voice/*`), linear 0..1 ----
    pub audio_master: f32,
    pub audio_music: f32,
    /// `audio/bus_BatailleMusique`.
    pub audio_battle_music: f32,
    pub audio_ambience: f32,
    /// `audio/bus_Interface` -> `AudioSettings::sfx`.
    pub audio_sfx: f32,
    /// `audio/bus_Bataille` (battle effects; exposed for the battle lots).
    pub audio_battle_sfx: f32,
    pub audio_voice: f32,
    /// `voice/advisor`: spoken advisor.
    pub voice_advisor: bool,
    /// `voice/barks`: unit replies.
    pub voice_barks: bool,
    // ---- Interface ----
    pub skip_loading: bool,
    pub campaign_briefing: bool,
    /// `interface/season_report`: `always`, `auto`, `off`.
    pub season_report: String,
    /// `interface/confirm_end_turn`: `off`, `warnings`, `always`.
    pub confirm_end_turn: String,
    /// `interface/news_filter`: `interest` mode name (see `NewsInterest.MODES`).
    pub news_filter: String,
    pub next_hint: bool,
    /// `interface/decor_hover_delay` in seconds, 0 = off.
    pub decor_hover_delay: f32,
    /// `access/colorblind`.
    pub colorblind: bool,
    /// `access/reduce_motion` -> `ReduceMotion`.
    pub reduce_motion: bool,
    /// `access/high_contrast`.
    pub high_contrast: bool,
    // ---- Jeu / campagne / bataille ----
    pub edge_pan: bool,
    /// `camera/speed`.
    pub camera_speed: f32,
    pub fog_of_war: bool,
    pub stance_fill: bool,
    pub show_zoc: bool,
    /// `map/ai_moves_speed`.
    pub ai_moves_speed: f32,
    /// `game/autosave_interval` in turns, 0 = off.
    pub autosave_interval: u32,
    pub interactive_battles: bool,
    pub tutorial_enabled: bool,
    /// `tutorial/step`: step the guide is at (resumes there).
    pub tutorial_step: u32,
    /// `tutorial/done`: the guide was finished or skipped.
    pub tutorial_done: bool,
    /// `tutorial/postponed`: put aside by « Plus tard ».
    pub tutorial_postponed: bool,
    /// `feudal_tutorial/done`: the three-step feudal guide was finished or skipped.
    pub feudal_tutorial_done: bool,
    /// `interface/event_feed_open`: the campaign event feed (top right) is unfolded.
    pub event_feed_open: bool,
    /// `interface/event_feed_folded`: ids of the feed rubrics the player folded; `None` until
    /// the player folds or unfolds one (the data's `default_folded` applies).
    pub event_feed_folded: Option<Vec<String>>,
    /// `battle/blood`: 0 off, 1 moderate, 2 full.
    pub blood: u32,
    /// `battle/unit_size`: figures per simulated man.
    pub unit_size: f32,
    pub max_figures: u32,
    pub cinematic: bool,
    pub cinematic_slowmo: bool,
    pub auto_pause_on_alert: bool,
    /// `dev/mode`.
    pub dev_mode: bool,
    // ---- Commandes ----
    /// `input/layout`: `auto`, `azerty`, `qwerty`.
    pub keyboard_layout: String,
    /// `input/bindings`: action -> keys (campaign).
    pub bindings: BTreeMap<String, Vec<String>>,
    /// `input/battle_bindings`: action -> key chord (battle).
    pub battle_bindings: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            fullscreen: false,
            resolution: (1440, 900),
            vsync: true,
            quality: "auto".into(),
            msaa: "auto".into(),
            shadows: true,
            view_distance: 1.0,
            upscale: "auto".into(),
            texture_quality: "high".into(),
            ui_size: 1.0,
            text_size: 1.0,
            audio_master: 1.0,
            audio_music: 0.6,
            audio_battle_music: 1.0,
            audio_ambience: 0.8,
            audio_sfx: 0.8,
            audio_battle_sfx: 0.9,
            audio_voice: 0.9,
            voice_advisor: true,
            voice_barks: true,
            skip_loading: false,
            campaign_briefing: true,
            season_report: "auto".into(),
            confirm_end_turn: "warnings".into(),
            news_filter: "interest".into(),
            next_hint: true,
            decor_hover_delay: 1.5,
            colorblind: false,
            reduce_motion: false,
            high_contrast: false,
            edge_pan: true,
            camera_speed: 1.0,
            fog_of_war: true,
            stance_fill: true,
            show_zoc: true,
            ai_moves_speed: 1.0,
            autosave_interval: 1,
            interactive_battles: true,
            tutorial_enabled: true,
            tutorial_step: 0,
            tutorial_done: false,
            tutorial_postponed: false,
            feudal_tutorial_done: false,
            event_feed_open: false,
            event_feed_folded: None,
            blood: 1,
            unit_size: 1.0,
            max_figures: 15000,
            cinematic: true,
            cinematic_slowmo: true,
            auto_pause_on_alert: false,
            dev_mode: false,
            keyboard_layout: "auto".into(),
            bindings: BTreeMap::new(),
            battle_bindings: BTreeMap::new(),
        }
    }
}

/// UI scale bounds (`settings.gd`): automatic scale from the window height, then the final clamp.
const AUTO_REFERENCE_HEIGHT: f32 = 900.0;
const AUTO_MIN: f32 = 0.9;
const AUTO_MAX: f32 = 1.6;
const SCALE_MIN: f32 = 0.7;
const SCALE_MAX: f32 = 2.0;

/// Automatic scale for a window `height` logical pixels high (1.0 at 900, 0.9..1.6).
pub fn auto_ui_scale(height: f32) -> f32 {
    if height < 300.0 {
        return 1.0;
    }
    (height / AUTO_REFERENCE_HEIGHT).clamp(AUTO_MIN, AUTO_MAX)
}

/// Smallest logical window the HUD is laid out for: the UI scale never grows so far that the
/// window, once divided by it, gets smaller (top bar, minimap and panels no longer fit below).
pub const MIN_LOGICAL_WIDTH: f32 = 1150.0;
pub const MIN_LOGICAL_HEIGHT: f32 = 640.0;

impl Settings {
    /// [`Self::effective_ui_scale`] capped so the logical window stays at least
    /// `MIN_LOGICAL_WIDTH` x `MIN_LOGICAL_HEIGHT` (« Très grande » on a 1280x800 window).
    pub fn fitted_ui_scale(&self, window_width: f32, window_height: f32) -> f32 {
        let wanted = self.effective_ui_scale(window_height);
        if window_width < 1.0 || window_height < 1.0 {
            return wanted;
        }
        let cap = (window_width / MIN_LOGICAL_WIDTH).min(window_height / MIN_LOGICAL_HEIGHT);
        wanted.min(cap.max(SCALE_MIN))
    }

    /// Final UI scale: automatic scale x « Taille de l'interface », clamped 0.7..2.0.
    pub fn effective_ui_scale(&self, window_height: f32) -> f32 {
        (auto_ui_scale(window_height) * self.ui_size).clamp(SCALE_MIN, SCALE_MAX)
    }

    /// Settings file: `CENT_ANS_SETTINGS_PATH`, else the per-user config directory
    /// (`~/Library/Application Support/Cent Ans/settings.json` on macOS).
    pub fn file_path() -> PathBuf {
        if let Some(path) = std::env::var_os(PATH_ENV) {
            return PathBuf::from(path);
        }
        let home = std::env::var_os("HOME").map(PathBuf::from);
        if cfg!(target_os = "macos") {
            home.unwrap_or_default()
                .join("Library/Application Support/Cent Ans/settings.json")
        } else if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .or(home)
                .unwrap_or_default()
                .join("Cent Ans/settings.json")
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| home.map(|home| home.join(".config")))
                .unwrap_or_default()
                .join("cent-ans/settings.json")
        }
    }

    /// Reads `path`; a missing or unreadable file gives the defaults (missing keys too).
    pub fn load_from(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|error| {
                warn!("settings {}: {error}; using defaults", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Writes the settings as pretty JSON, creating the directory.
    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_partial_file() {
        let dir = std::env::temp_dir().join(format!("cent_ans_settings_{}", std::process::id()));
        let path = dir.join("nested/settings.json");
        let mut settings = Settings {
            audio_music: 0.25,
            ..Settings::default()
        };
        settings
            .bindings
            .insert("end_turn".into(), vec!["Enter".into()]);
        settings.save_to(&path).unwrap();
        assert_eq!(Settings::load_from(&path), settings);
        std::fs::write(&path, r#"{"audio_music": 0.1, "unknown": 3}"#).unwrap();
        let partial = Settings::load_from(&path);
        assert_eq!(partial.audio_music, 0.1);
        assert_eq!(partial.audio_master, 1.0);
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(Settings::load_from(&path), Settings::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ui_scale_follows_godot() {
        let settings = Settings::default();
        assert_eq!(settings.effective_ui_scale(900.0), 1.0);
        assert_eq!(settings.effective_ui_scale(64.0), 1.0);
        assert_eq!(settings.effective_ui_scale(2000.0), 1.6);
        // A big interface size on a small window is capped to keep the HUD laid out.
        let big = Settings {
            ui_size: 1.6,
            ..Settings::default()
        };
        let fitted = big.fitted_ui_scale(1280.0, 800.0);
        assert!(1280.0 / fitted >= MIN_LOGICAL_WIDTH - 0.5, "{fitted}");
        assert_eq!(settings.fitted_ui_scale(2560.0, 1440.0), 1.6);
        let big = Settings {
            ui_size: 1.25,
            ..Settings::default()
        };
        assert!((big.effective_ui_scale(1800.0) - 2.0).abs() < 1e-6);
    }
}
