//! First-launch migration of the Godot player settings (`user://settings.cfg`, a Godot
//! `ConfigFile`) to the Bevy `settings.json`.
//!
//! Godot stores `[section]` / `key=value` lines whose values use the Variant text syntax
//! (`true`, `1.5`, `"text"`, `Vector2i(1440, 900)`, multi-line `{...}` dictionaries, arrays).
//! [`parse_config`] reads that text; [`settings_from_godot`] maps the keys of `settings.gd`
//! (`DEFAULTS`) onto [`Settings`], and the re-bound keys of `input/bindings` (Godot key codes
//! with modifier masks) and `input/battle_bindings` (`{key, mods}`) onto chord labels.
//! Keys with no Bevy counterpart (tutorial flags, `custom_battle/last`, voice memory...) are
//! ignored. The migration runs once: only when `settings.json` does not exist yet.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy::prelude::KeyCode;

use super::keymap::{Chord, definition, key_from_label};
use super::model::{PATH_ENV, Settings};

/// Environment variable naming the Godot `settings.cfg` to import (tests, odd installs).
pub const GODOT_FILE_ENV: &str = "CENT_ANS_GODOT_SETTINGS";

/// A Godot `Variant` as far as settings need it.
#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    /// `Vector2` / `Vector2i`.
    Vec2(f64, f64),
    /// `[...]`, `Array[T]([...])`, `PackedXArray(...)`.
    Array(Vec<Variant>),
    /// `{...}` in file order.
    Dict(Vec<(Variant, Variant)>),
}

impl Variant {
    fn as_f64(&self) -> Option<f64> {
        match self {
            Variant::Int(i) => Some(*i as f64),
            Variant::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Variant::Int(i) => Some(*i),
            Variant::Float(f) if f.is_finite() => Some(*f as i64),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Variant::Bool(b) => Some(*b),
            Variant::Int(i) => Some(*i != 0),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Variant::Str(s) => Some(s),
            _ => None,
        }
    }

    fn get(&self, key: &str) -> Option<&Variant> {
        match self {
            Variant::Dict(entries) => entries
                .iter()
                .find(|(k, _)| k.as_str() == Some(key))
                .map(|(_, v)| v),
            _ => None,
        }
    }
}

/// A parsed `ConfigFile`: `"section/key"` -> value, in file order.
pub type Config = Vec<(String, Variant)>;

struct Cursor<'a> {
    text: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.pos).copied()
    }

    fn skip_blank(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn skip_blank_and_comments(&mut self) {
        loop {
            self.skip_blank();
            if self.peek() == Some(b';') {
                while self.peek().is_some_and(|c| c != b'\n') {
                    self.pos += 1;
                }
            } else {
                return;
            }
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn identifier(&mut self) -> &str {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.')
        {
            self.pos += 1;
        }
        std::str::from_utf8(&self.text[start..self.pos]).unwrap_or("")
    }

    fn string(&mut self) -> Option<String> {
        if !self.eat(b'"') {
            return None;
        }
        let mut bytes = Vec::new();
        loop {
            match self.peek()? {
                b'"' => {
                    self.pos += 1;
                    break;
                }
                b'\\' => {
                    self.pos += 1;
                    let escaped = self.peek()?;
                    self.pos += 1;
                    match escaped {
                        b'n' => bytes.push(b'\n'),
                        b't' => bytes.push(b'\t'),
                        b'r' => bytes.push(b'\r'),
                        b'u' => {
                            let hex = self.text.get(self.pos..self.pos + 4)?;
                            let code = u32::from_str_radix(std::str::from_utf8(hex).ok()?, 16)
                                .ok()
                                .and_then(char::from_u32)?;
                            self.pos += 4;
                            let mut buffer = [0u8; 4];
                            bytes.extend_from_slice(code.encode_utf8(&mut buffer).as_bytes());
                        }
                        other => bytes.push(other),
                    }
                }
                other => {
                    bytes.push(other);
                    self.pos += 1;
                }
            }
        }
        String::from_utf8(bytes).ok()
    }

    fn number(&mut self) -> Option<Variant> {
        let start = self.pos;
        if matches!(self.peek(), Some(b'-' | b'+')) {
            self.pos += 1;
        }
        let mut is_float = false;
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' => {}
                b'.' | b'e' | b'E' => is_float = true,
                b'-' | b'+' if matches!(self.text[self.pos - 1], b'e' | b'E') => {}
                _ => break,
            }
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.text[start..self.pos]).ok()?;
        if is_float {
            text.parse().ok().map(Variant::Float)
        } else {
            text.parse().ok().map(Variant::Int)
        }
    }

    /// Comma separated values up to `close` (a trailing comma is allowed).
    fn list(&mut self, close: u8) -> Option<Vec<Variant>> {
        let mut items = Vec::new();
        loop {
            self.skip_blank_and_comments();
            if self.eat(close) {
                return Some(items);
            }
            items.push(self.value()?);
            self.skip_blank_and_comments();
            if !self.eat(b',') {
                self.skip_blank_and_comments();
                return self.eat(close).then_some(items);
            }
        }
    }

    fn value(&mut self) -> Option<Variant> {
        self.skip_blank_and_comments();
        match self.peek()? {
            b'"' => self.string().map(Variant::Str),
            b'&' | b'^' => {
                self.pos += 1;
                self.string().map(Variant::Str)
            }
            b'[' => {
                self.pos += 1;
                self.list(b']').map(Variant::Array)
            }
            b'{' => {
                self.pos += 1;
                let mut entries = Vec::new();
                loop {
                    self.skip_blank_and_comments();
                    if self.eat(b'}') {
                        return Some(Variant::Dict(entries));
                    }
                    let key = self.value()?;
                    self.skip_blank_and_comments();
                    if !self.eat(b':') {
                        return None;
                    }
                    let value = self.value()?;
                    entries.push((key, value));
                    self.skip_blank_and_comments();
                    if !self.eat(b',') {
                        self.skip_blank_and_comments();
                        return self.eat(b'}').then_some(Variant::Dict(entries));
                    }
                }
            }
            b'-' | b'+' | b'0'..=b'9' | b'.' => self.number(),
            _ => {
                let name = self.identifier().to_owned();
                match name.as_str() {
                    "true" => return Some(Variant::Bool(true)),
                    "false" => return Some(Variant::Bool(false)),
                    "null" => return Some(Variant::Null),
                    "inf" => return Some(Variant::Float(f64::INFINITY)),
                    "inf_neg" => return Some(Variant::Float(f64::NEG_INFINITY)),
                    "nan" => return Some(Variant::Float(f64::NAN)),
                    "" => return None,
                    _ => {}
                }
                // Constructor: `Vector2i(1, 2)`, `Array[int]([1])`, `PackedStringArray("a")`...
                if self.eat(b'[') {
                    while self.peek().is_some_and(|c| c != b']') {
                        self.pos += 1;
                    }
                    self.pos += 1;
                }
                if !self.eat(b'(') {
                    return None;
                }
                let args = self.list(b')')?;
                Some(match name.as_str() {
                    "Vector2" | "Vector2i" => match args.as_slice() {
                        [x, y] => Variant::Vec2(x.as_f64()?, y.as_f64()?),
                        _ => return None,
                    },
                    // Typed arrays wrap their elements in one array argument.
                    "Array" => match args.into_iter().next() {
                        Some(Variant::Array(items)) => Variant::Array(items),
                        _ => Variant::Array(Vec::new()),
                    },
                    _ => Variant::Array(args),
                })
            }
        }
    }
}

/// Parses the text of a Godot `ConfigFile`. Unreadable entries are skipped (the parser
/// resumes at the next line), like Godot, which keeps the sections it could read.
pub fn parse_config(text: &str) -> Config {
    let mut cursor = Cursor {
        text: text.as_bytes(),
        pos: 0,
    };
    let mut section = String::new();
    let mut config = Config::new();
    loop {
        cursor.skip_blank_and_comments();
        let Some(first) = cursor.peek() else {
            return config;
        };
        if first == b'[' {
            cursor.pos += 1;
            let start = cursor.pos;
            while cursor.peek().is_some_and(|c| c != b']' && c != b'\n') {
                cursor.pos += 1;
            }
            section = String::from_utf8_lossy(&cursor.text[start..cursor.pos]).into_owned();
            cursor.eat(b']');
            continue;
        }
        let key = if first == b'"' {
            cursor.string().unwrap_or_default()
        } else {
            let start = cursor.pos;
            while cursor.peek().is_some_and(|c| c != b'=' && c != b'\n') {
                cursor.pos += 1;
            }
            String::from_utf8_lossy(&cursor.text[start..cursor.pos])
                .trim()
                .to_owned()
        };
        cursor.skip_blank();
        let line_start = cursor.pos;
        if cursor.eat(b'=')
            && let Some(value) = cursor.value()
        {
            let name = if section.is_empty() {
                key
            } else {
                format!("{section}/{key}")
            };
            config.push((name, value));
            continue;
        }
        // Skip the broken line.
        cursor.pos = line_start;
        while cursor.peek().is_some_and(|c| c != b'\n') {
            cursor.pos += 1;
        }
    }
}

// ---- Godot key codes ----------------------------------------------------------------------

const KEY_SPECIAL: i64 = 1 << 22;
const MASK_CMD_OR_CTRL: i64 = 1 << 24;
const MASK_SHIFT: i64 = 1 << 25;
const MASK_ALT: i64 = 1 << 26;
const MASK_CTRL: i64 = 1 << 28;
const CODE_MASK: i64 = (1 << 23) - 1;

/// Bevy key of a Godot key code (modifier bits already removed); `None` when unbindable.
fn godot_key(code: i64) -> Option<KeyCode> {
    let special = |offset: i64| code == KEY_SPECIAL | offset;
    if (65..=90).contains(&code) || (48..=57).contains(&code) {
        return key_from_label(&(code as u8 as char).to_string());
    }
    if (0x42..=0x4D).contains(&(code - KEY_SPECIAL)) {
        return key_from_label(&format!("F{}", code - KEY_SPECIAL - 0x41));
    }
    Some(match code {
        32 => KeyCode::Space,
        39 => KeyCode::Quote,
        44 => KeyCode::Comma,
        45 => KeyCode::Minus,
        46 => KeyCode::Period,
        47 => KeyCode::Slash,
        59 => KeyCode::Semicolon,
        61 => KeyCode::Equal,
        91 => KeyCode::BracketLeft,
        92 => KeyCode::Backslash,
        93 => KeyCode::BracketRight,
        96 => KeyCode::Backquote,
        _ if special(0x01) => KeyCode::Escape,
        _ if special(0x02) => KeyCode::Tab,
        _ if special(0x04) => KeyCode::Backspace,
        _ if special(0x05) => KeyCode::Enter,
        _ if special(0x06) => KeyCode::NumpadEnter,
        _ if special(0x07) => KeyCode::Insert,
        _ if special(0x08) => KeyCode::Delete,
        _ if special(0x10) => KeyCode::Home,
        _ if special(0x11) => KeyCode::End,
        _ if special(0x12) => KeyCode::ArrowLeft,
        _ if special(0x13) => KeyCode::ArrowUp,
        _ if special(0x14) => KeyCode::ArrowRight,
        _ if special(0x15) => KeyCode::ArrowDown,
        _ if special(0x16) => KeyCode::PageUp,
        _ if special(0x17) => KeyCode::PageDown,
        _ if special(0x26) => KeyCode::ScrollLock,
        _ if special(0x83) => KeyCode::NumpadSubtract,
        _ if special(0x85) => KeyCode::NumpadAdd,
        _ => return None,
    })
}

/// Chord of an encoded Godot key (`KeyBindings.encode`: key code | modifier masks).
fn chord_from_code(code: i64) -> Option<Chord> {
    let mut chord = Chord::key(godot_key(code & CODE_MASK)?);
    chord.shift = code & MASK_SHIFT != 0;
    chord.ctrl = code & (MASK_CTRL | MASK_CMD_OR_CTRL) != 0;
    chord.alt = code & MASK_ALT != 0;
    Some(chord)
}

/// Chord of a battle binding `{key, mods}` (`mods`: `ctrl`, `shift`, `alt`, joined by `_`).
fn chord_from_battle(entry: &Variant) -> Option<Chord> {
    let mut chord = Chord::key(godot_key(entry.get("key")?.as_i64()? & CODE_MASK)?);
    for modifier in entry.get("mods").and_then(Variant::as_str)?.split('_') {
        match modifier {
            "ctrl" | "cmd" => chord.ctrl = true,
            "shift" => chord.shift = true,
            "alt" => chord.alt = true,
            _ => {}
        }
    }
    Some(chord)
}

// ---- Mapping --------------------------------------------------------------------------------

fn pick_string(value: &Variant, allowed: &[&str]) -> Option<String> {
    value
        .as_str()
        .filter(|s| allowed.contains(s))
        .map(str::to_owned)
}

/// Ladder of `season_report` / `confirm_end_turn` older booleans, as `_coerce` in `settings.gd`.
fn legacy_bool_or(
    value: &Variant,
    allowed: &[&str],
    when_true: &str,
    when_false: &str,
) -> Option<String> {
    match value {
        Variant::Bool(true) => Some(when_true.to_owned()),
        Variant::Bool(false) => Some(when_false.to_owned()),
        other => pick_string(other, allowed),
    }
}

/// Maps a parsed Godot config onto the defaults. Returns the settings and the number of Godot
/// keys that were carried over.
pub fn settings_from_godot(config: &Config) -> (Settings, usize) {
    let mut settings = Settings::default();
    let mut carried = 0;
    for (key, value) in config {
        if apply_key(&mut settings, key, value) {
            carried += 1;
        }
    }
    (settings, carried)
}

fn apply_key(s: &mut Settings, key: &str, v: &Variant) -> bool {
    let unit = |v: &Variant| v.as_f64().map(|f| (f as f32).clamp(0.0, 1.0));
    let positive = |v: &Variant| v.as_f64().map(|f| f as f32).filter(|f| *f > 0.0);
    let count = |v: &Variant| v.as_i64().and_then(|i| u32::try_from(i).ok());
    macro_rules! set {
        ($field:expr, $value:expr) => {
            match $value {
                Some(value) => {
                    $field = value;
                    true
                }
                None => false,
            }
        };
    }
    match key {
        "video/fullscreen" => set!(s.fullscreen, v.as_bool()),
        "video/vsync" => set!(s.vsync, v.as_bool()),
        "video/resolution" => match v {
            Variant::Vec2(x, y) if *x >= 640.0 && *y >= 360.0 => {
                s.resolution = (*x as u32, *y as u32);
                true
            }
            _ => false,
        },
        "video/quality" => set!(
            s.quality,
            pick_string(v, &["auto", "low", "medium", "high", "ultra"])
        ),
        "video/upscale" => set!(
            s.upscale,
            pick_string(v, &["auto", "off", "quality", "performance"])
        ),
        "video/texture_quality" => set!(s.texture_quality, pick_string(v, &["high", "medium"])),
        "interface/ui_size" => set!(s.ui_size, positive(v)),
        "interface/text_size" => set!(s.text_size, positive(v)),
        "interface/skip_loading" => set!(s.skip_loading, v.as_bool()),
        "interface/campaign_briefing" => set!(s.campaign_briefing, v.as_bool()),
        "interface/season_report" => set!(
            s.season_report,
            legacy_bool_or(v, &["always", "auto", "off"], "auto", "off")
        ),
        "interface/confirm_end_turn" => set!(
            s.confirm_end_turn,
            legacy_bool_or(v, &["off", "warnings", "always"], "always", "warnings")
        ),
        "interface/news_filter" => set!(s.news_filter, v.as_str().map(str::to_owned)),
        "interface/next_hint" => set!(s.next_hint, v.as_bool()),
        "interface/decor_hover_delay" => {
            set!(s.decor_hover_delay, v.as_f64().map(|f| (f as f32).max(0.0)))
        }
        "input/layout" => set!(
            s.keyboard_layout,
            pick_string(v, &["auto", "azerty", "qwerty"])
        ),
        "audio/bus_Master" => set!(s.audio_master, unit(v)),
        "audio/bus_Musique" => set!(s.audio_music, unit(v)),
        "audio/bus_BatailleMusique" => set!(s.audio_battle_music, unit(v)),
        "audio/bus_Ambiance" => set!(s.audio_ambience, unit(v)),
        "audio/bus_Bataille" => set!(s.audio_battle_sfx, unit(v)),
        "audio/bus_Interface" => set!(s.audio_sfx, unit(v)),
        "audio/bus_Voix" => set!(s.audio_voice, unit(v)),
        "access/colorblind" => set!(s.colorblind, v.as_bool()),
        "access/reduce_motion" => set!(s.reduce_motion, v.as_bool()),
        "access/high_contrast" => set!(s.high_contrast, v.as_bool()),
        "camera/edge_pan" => set!(s.edge_pan, v.as_bool()),
        "camera/speed" => set!(s.camera_speed, positive(v)),
        "map/fog_of_war" => set!(s.fog_of_war, v.as_bool()),
        "map/stance_fill" => set!(s.stance_fill, v.as_bool()),
        "map/show_zoc" => set!(s.show_zoc, v.as_bool()),
        "map/ai_moves_speed" => set!(s.ai_moves_speed, positive(v)),
        "game/autosave_interval" => set!(s.autosave_interval, count(v)),
        "game/interactive_battles" => set!(s.interactive_battles, v.as_bool()),
        "tutorial/enabled" => set!(s.tutorial_enabled, v.as_bool()),
        "tutorial/step" => set!(s.tutorial_step, count(v)),
        "tutorial/done" => set!(s.tutorial_done, v.as_bool()),
        "tutorial/postponed" => set!(s.tutorial_postponed, v.as_bool()),
        "battle/blood" => set!(s.blood, count(v).map(|n| n.min(2))),
        "battle/unit_size" => set!(s.unit_size, positive(v)),
        "battle/max_figures" => set!(s.max_figures, count(v)),
        "battle/cinematic" => set!(s.cinematic, v.as_bool()),
        "battle/cinematic_slowmo" => set!(s.cinematic_slowmo, v.as_bool()),
        "battle/auto_pause_on_alert" => set!(s.auto_pause_on_alert, v.as_bool()),
        "voice/advisor" => set!(s.voice_advisor, v.as_bool()),
        "voice/barks" => set!(s.voice_barks, v.as_bool()),
        "dev/mode" => set!(s.dev_mode, v.as_bool()),
        "input/bindings" => carry_bindings(&mut s.bindings, v),
        "input/battle_bindings" => carry_battle_bindings(&mut s.bindings, v),
        _ => false,
    }
}

fn carry_bindings(bindings: &mut BTreeMap<String, Vec<String>>, value: &Variant) -> bool {
    let Variant::Dict(entries) = value else {
        return false;
    };
    let mut any = false;
    for (action, codes) in entries {
        let (Some(action), Variant::Array(codes)) = (action.as_str(), codes) else {
            continue;
        };
        let Some(def) = definition(action) else {
            continue;
        };
        let labels: Option<Vec<String>> = codes
            .iter()
            .map(|code| code.as_i64().and_then(chord_from_code).map(|c| c.label()))
            .collect();
        // One unreadable key drops the action back to its defaults rather than half of it.
        if let Some(labels) = labels.filter(|l| !l.is_empty())
            && labels != def.defaults
        {
            bindings.insert(def.id.to_owned(), labels);
            any = true;
        }
    }
    any
}

fn carry_battle_bindings(bindings: &mut BTreeMap<String, Vec<String>>, value: &Variant) -> bool {
    let Variant::Dict(entries) = value else {
        return false;
    };
    let mut any = false;
    for (action, entry) in entries {
        let Some(action) = action.as_str() else {
            continue;
        };
        let id = format!("battle_{action}");
        let (Some(def), Some(chord)) = (definition(&id), chord_from_battle(entry)) else {
            continue;
        };
        let label = chord.label();
        if def.defaults != [label.as_str()] {
            bindings.insert(def.id.to_owned(), vec![label]);
            any = true;
        }
    }
    any
}

// ---- Locating and running -----------------------------------------------------------------

/// Where Godot keeps `settings.cfg` for this project, best candidates first: the exported
/// game's own folder (`config/custom_user_dir_name`) then the shared `Godot/app_userdata`
/// folder of the editor (`user_dir_migration.gd`).
pub fn godot_candidates() -> Vec<PathBuf> {
    if let Some(path) = std::env::var_os(GODOT_FILE_ENV) {
        return vec![PathBuf::from(path)];
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let (base, shared) = if cfg!(target_os = "macos") {
        (
            home.map(|h| h.join("Library/Application Support")),
            "Godot/app_userdata",
        )
    } else if cfg!(target_os = "windows") {
        (
            std::env::var_os("APPDATA").map(PathBuf::from).or(home),
            "Godot/app_userdata",
        )
    } else {
        (
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| home.map(|h| h.join(".local/share"))),
            "godot/app_userdata",
        )
    };
    let Some(base) = base else {
        return Vec::new();
    };
    vec![
        base.join("Cent Ans/settings.cfg"),
        base.join(shared).join("Cent Ans/settings.cfg"),
    ]
}

/// The most recently modified existing file of `candidates`.
pub fn newest_existing(candidates: &[PathBuf]) -> Option<&Path> {
    candidates
        .iter()
        .filter_map(|path| {
            let modified = std::fs::metadata(path).ok()?.modified().ok()?;
            Some((modified, path.as_path()))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

/// Result of a migration, for the log and the dump.
#[derive(Debug, Clone, PartialEq)]
pub struct Migration {
    pub source: PathBuf,
    pub carried_keys: usize,
    pub bindings: usize,
}

/// Imports the Godot file at `source` into `target` (a missing `target` only; an existing
/// one is never overwritten). Returns `None` when there is nothing to import or to write.
pub fn migrate_file(source: &Path, target: &Path) -> Option<Migration> {
    if target.exists() {
        return None;
    }
    let text = std::fs::read_to_string(source).ok()?;
    let (settings, carried_keys) = settings_from_godot(&parse_config(&text));
    if carried_keys == 0 {
        return None;
    }
    settings.save_to(target).ok()?;
    Some(Migration {
        source: source.to_owned(),
        carried_keys,
        bindings: settings.bindings.len(),
    })
}

/// Migration of the real files at startup. Skipped when the settings file is redirected by
/// `CENT_ANS_SETTINGS_PATH` (tests, background runs) unless a Godot file is named explicitly.
pub fn migrate_at_startup(target: &Path) -> Option<Migration> {
    if target.exists() {
        return None;
    }
    if std::env::var_os(PATH_ENV).is_some() && std::env::var_os(GODOT_FILE_ENV).is_none() {
        return None;
    }
    let candidates = godot_candidates();
    migrate_file(newest_existing(&candidates)?, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"; Engine configuration file.
[video]

fullscreen=true
resolution=Vector2i(1920, 1080)
quality="high"
upscale="bogus"

[interface]

ui_size=1.25
confirm_end_turn=true
season_report=false
decor_hover_delay=0.0

[audio]

bus_Master=0.5
bus_Musique=1
bus_Bataille=0.35
bus_Interface=0.2

[battle]

blood=2
max_figures=8000

[input]

layout="azerty"
bindings={
"map_pan_up": [73, 4194323],
"map_pan_down": [83, 4194355],
"end_turn": [4194309],
"codex_open": [74],
"map_rotate_left": [69, 369098840],
"unknown_action": [65]
}
battle_bindings={
"fire_at_will": {
"key": 74,
"mods": "ctrl_shift"
},
"halt": {
"key": 72,
"mods": ""
}
}

[tutorial]

step=3
done=true
"#;

    #[test]
    fn parses_godot_variants() {
        let config = parse_config(SAMPLE);
        let get = |key: &str| {
            config
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(get("video/fullscreen"), Some(Variant::Bool(true)));
        assert_eq!(get("video/resolution"), Some(Variant::Vec2(1920.0, 1080.0)));
        assert_eq!(get("audio/bus_Musique"), Some(Variant::Int(1)));
        assert_eq!(get("interface/ui_size"), Some(Variant::Float(1.25)));
        let Some(Variant::Dict(bindings)) = get("input/bindings") else {
            panic!("bindings");
        };
        assert_eq!(bindings.len(), 6);
        assert_eq!(
            parse_config(
                "[a]\nlist=Array[int]([1, 2])\nbad=\nok=\"x\\ty\"\nvec=PackedStringArray(\"p\", \"q\")\n"
            ),
            vec![
                (
                    "a/list".to_owned(),
                    Variant::Array(vec![Variant::Int(1), Variant::Int(2)])
                ),
                ("a/ok".to_owned(), Variant::Str("x\ty".into())),
                (
                    "a/vec".to_owned(),
                    Variant::Array(vec![Variant::Str("p".into()), Variant::Str("q".into())])
                ),
            ]
        );
    }

    #[test]
    fn maps_godot_keys_to_settings() {
        let (s, carried) = settings_from_godot(&parse_config(SAMPLE));
        assert!(carried >= 15, "{carried}");
        assert!(s.fullscreen);
        assert_eq!(s.resolution, (1920, 1080));
        assert_eq!(s.quality, "high");
        assert_eq!(s.upscale, "auto", "unknown choice keeps the default");
        assert_eq!(s.ui_size, 1.25);
        assert_eq!(s.confirm_end_turn, "always", "old boolean");
        assert_eq!(s.season_report, "off", "old boolean");
        assert_eq!(s.decor_hover_delay, 0.0);
        assert_eq!(s.audio_master, 0.5);
        assert_eq!(s.audio_music, 1.0);
        assert_eq!(s.audio_battle_sfx, 0.35, "bus Bataille");
        assert_eq!(s.audio_sfx, 0.2, "bus Interface");
        assert_eq!(s.audio_ambience, 0.8, "absent key keeps the default");
        assert_eq!((s.blood, s.max_figures), (2, 8000));
        assert_eq!(s.keyboard_layout, "azerty");
        assert_eq!((s.tutorial_step, s.tutorial_done), (3, true));
    }

    #[test]
    fn maps_bindings() {
        let (s, _) = settings_from_godot(&parse_config(SAMPLE));
        assert_eq!(
            s.bindings["map_pan_up"],
            vec!["I".to_owned(), "↑".to_owned()]
        );
        assert_eq!(
            s.bindings.get("map_pan_down"),
            None,
            "unreadable key: defaults"
        );
        assert!(
            !s.bindings.contains_key("end_turn"),
            "Entrée is the default"
        );
        assert_eq!(s.bindings["codex_open"], vec!["J".to_owned()]);
        assert_eq!(
            s.bindings["map_rotate_left"],
            vec!["E".to_owned(), "Ctrl+Alt+Maj+X".to_owned()]
        );
        assert!(!s.bindings.contains_key("unknown_action"));
        assert_eq!(
            s.bindings["battle_fire_at_will"],
            vec!["Ctrl+Maj+J".to_owned()]
        );
        assert!(!s.bindings.contains_key("battle_halt"), "H is the default");
        // The result is understood by the key map.
        let map = super::super::KeyMap::from_bindings(&s.bindings);
        assert_eq!(map.label("battle_fire_at_will", "qwerty"), "Ctrl+Maj+J");
    }

    #[test]
    fn migrates_once_and_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("cent_ans_migrate_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("settings.cfg");
        std::fs::write(&source, SAMPLE).unwrap();
        let target = dir.join("out/settings.json");
        let migration = migrate_file(&source, &target).expect("migrated");
        assert_eq!(migration.source, source);
        assert!(migration.bindings >= 3);
        let loaded = Settings::load_from(&target);
        assert_eq!(loaded.resolution, (1920, 1080));
        assert_eq!(loaded.audio_battle_sfx, 0.35);
        // Second run: the JSON exists, it is left alone.
        std::fs::write(&target, r#"{"audio_music": 0.1}"#).unwrap();
        assert_eq!(migrate_file(&source, &target), None);
        assert_eq!(Settings::load_from(&target).audio_music, 0.1);
        // Empty / foreign file: nothing written.
        let empty = dir.join("empty.cfg");
        std::fs::write(&empty, "[foo]\nbar=1\n").unwrap();
        assert_eq!(migrate_file(&empty, &dir.join("nothing.json")), None);
        assert!(!dir.join("nothing.json").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
