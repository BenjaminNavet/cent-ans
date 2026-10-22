//! Save slots: same files as the Godot client (`SimFacade.save_game` + `SaveSlots`).
//!
//! `<dir>/<name>.json` holds a wrapper `{version, faction, date, turn, difficulty, timestamp,
//! state}` where `state` is `CampaignState::save_json`; `<dir>/<name>.meta.json` is the light
//! sheet the list reads (`{name, label, faction, date, turn, difficulty, timestamp, path}`).
//! Replaces `SimFacade.save_game/read_save/load_game/list_saves`, `SaveSlots.list/latest/delete`
//! (thumbnails are not handled here).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use data_model::GameData;
use serde::{Deserialize, Serialize};
use sim_campaign::CampaignState;

/// Version of the wrapper (not of the campaign state, which has its own `STATE_VERSION`).
/// Raise it together with a new entry in [`WRAPPER_MIGRATIONS`].
pub const SAVE_VERSION: u32 = 1;

/// Migration chain of the wrapper: entry `i` upgrades a wrapper of version `i + 1` to `i + 2`
/// (operates on the raw JSON object). Empty while [`SAVE_VERSION`] is 1.
const WRAPPER_MIGRATIONS: &[fn(&mut serde_json::Value)] = &[];
/// Prefix of the rotating autosaves (`auto_1..5`, `auto_battle`).
pub const AUTOSAVE_PREFIX: &str = "auto_";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveWrapper {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub faction: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub turn: u32,
    #[serde(default = "default_difficulty")]
    pub difficulty: String,
    #[serde(default)]
    pub timestamp: String,
    pub state: String,
    /// Opaque history kept by the client (journal, chronicle, feudal events…): the core's state
    /// does not hold it. Absent in older saves (ADR 0456).
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub client: serde_json::Value,
}

fn default_difficulty() -> String {
    "normal".to_owned()
}

/// One row of the save list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveMeta {
    pub name: String,
    pub label: String,
    pub faction: String,
    pub date: String,
    /// Zero-based turn counter.
    pub turn: u32,
    pub difficulty: String,
    /// Local ISO date (`2026-09-24T23:19:05`), newest first sort key.
    pub timestamp: String,
    pub path: String,
}

/// Longest file stem kept (bytes), leaving room for `.meta.json` under the 255-byte limit.
const MAX_STEM_BYTES: usize = 120;
/// Stem used when the name is empty once cleaned.
const FALLBACK_STEM: &str = "sans_nom";

/// File stem of a slot name: forbidden characters become `_`, control characters are dropped,
/// and the result is never empty, `.`/`..`, a Windows reserved device name, longer than
/// [`MAX_STEM_BYTES`] or ending with a dot or a space.
///
/// Distinct names may still share a stem (`a:b` and `a/b`); see [`stem_is_lossy`].
pub fn file_stem(save_name: &str) -> String {
    let mut stem: String = save_name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            ':' | '/' | '\\' | '?' | '*' | '"' | '|' | '%' | '<' | '>' => '_',
            other => other,
        })
        .collect();
    while stem.len() > MAX_STEM_BYTES {
        stem.pop();
    }
    let kept = stem.trim_end_matches(['.', ' ']).len();
    stem.truncate(kept);
    let device = stem.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(device.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (device.len() == 4
            && (device.starts_with("COM") || device.starts_with("LPT"))
            && device.as_bytes()[3].is_ascii_digit());
    if stem.is_empty() {
        FALLBACK_STEM.to_owned()
    } else if reserved {
        format!("_{stem}")
    } else {
        stem
    }
}

/// Whether the name is altered by [`file_stem`] (another name could then land on the same file),
/// so that a caller can warn before overwriting.
pub fn stem_is_lossy(save_name: &str) -> bool {
    file_stem(save_name) != save_name
}

/// Writes `text` to `path` atomically: a sibling temporary file is written and synced, then
/// renamed over the target, so a crash leaves either the old or the new file, never a torn one.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    let result = (|| {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

pub fn save_path(dir: &Path, save_name: &str) -> PathBuf {
    dir.join(format!("{}.json", file_stem(save_name)))
}

fn meta_path(dir: &Path, save_name: &str) -> PathBuf {
    dir.join(format!("{}.meta.json", file_stem(save_name)))
}

/// French label of a slot (`auto_2` -> « Sauvegarde automatique 2 »).
pub fn display_name(save_name: &str) -> String {
    if save_name == "auto_battle" {
        "Avant la bataille".to_owned()
    } else if let Some(rank) = save_name.strip_prefix(AUTOSAVE_PREFIX) {
        format!("Sauvegarde automatique {rank}")
    } else {
        save_name.to_owned()
    }
}

/// Current UTC time as `YYYY-MM-DDTHH:MM:SS` (no timezone database needed).
pub fn timestamp_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil-from-days (H. Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

/// Writes slot `save_name` (wrapper + meta sheet). Returns the wrapper path.
pub fn write_save(
    dir: &Path,
    save_name: &str,
    state: &CampaignState,
    timestamp: &str,
) -> Result<PathBuf, String> {
    write_save_with_client(dir, save_name, state, timestamp, serde_json::Value::Null)
}

/// [`write_save`] plus the client's own history (`client`, `Null` for none).
pub fn write_save_with_client(
    dir: &Path,
    save_name: &str,
    state: &CampaignState,
    timestamp: &str,
    client: serde_json::Value,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let wrapper = SaveWrapper {
        version: SAVE_VERSION,
        faction: state.player_faction().as_str().to_owned(),
        date: state.date_label(),
        turn: state.turn(),
        difficulty: state.difficulty().key().to_owned(),
        timestamp: timestamp.to_owned(),
        state: state.save_json(),
        client,
    };
    let path = save_path(dir, save_name);
    let text = serde_json::to_string(&wrapper).map_err(|e| e.to_string())?;
    write_atomic(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
    let meta = SaveMeta {
        name: file_stem(save_name),
        label: display_name(save_name),
        faction: wrapper.faction,
        date: wrapper.date,
        turn: wrapper.turn,
        difficulty: wrapper.difficulty,
        timestamp: wrapper.timestamp,
        path: path.to_string_lossy().into_owned(),
    };
    let meta_text = serde_json::to_string(&meta).map_err(|e| e.to_string())?;
    // The sheet is an optimisation: a failed write only makes the list read the wrapper.
    let _ = write_atomic(&meta_path(dir, save_name), &meta_text);
    Ok(path)
}

pub fn read_save(path: &Path) -> Result<SaveWrapper, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_wrapper(&text)
}

/// Parses a wrapper, refusing a newer version and running the migration chain on older ones.
pub fn parse_wrapper(text: &str) -> Result<SaveWrapper, String> {
    let damaged = |e: serde_json::Error| format!("sauvegarde abîmée ou illisible ({e})");
    let mut value: serde_json::Value = serde_json::from_str(text).map_err(damaged)?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .map_or(0, |v| u32::try_from(v).unwrap_or(u32::MAX));
    if version > SAVE_VERSION {
        return Err(format!(
            "cette sauvegarde vient d'une version plus récente du jeu (format {version}, \
             ce jeu lit jusqu'au format {SAVE_VERSION}) : mettez le jeu à jour pour la charger"
        ));
    }
    // Version 0 is a wrapper written before the field existed: same layout as version 1.
    for migrate in WRAPPER_MIGRATIONS
        .iter()
        .skip(version.saturating_sub(1) as usize)
    {
        migrate(&mut value);
    }
    serde_json::from_value(value).map_err(damaged)
}

/// Restores the campaign of a save file, with the same post-load fixes as `load_from_string`.
pub fn load_save(data: &GameData, path: &Path) -> Result<CampaignState, String> {
    load_state(data, &read_save(path)?)
}

/// The campaign of an already read wrapper, with the post-load fixes.
pub fn load_state(data: &GameData, wrapper: &SaveWrapper) -> Result<CampaignState, String> {
    let mut state = CampaignState::load_json(&wrapper.state).map_err(|e| match e {
        sim_campaign::CampaignError::Deserialize(detail) => {
            format!("sauvegarde abîmée ou illisible ({detail})")
        }
        other => other.to_string(),
    })?;
    sim_campaign::council::ensure_councils(&mut state, data);
    sim_campaign::march::unstick_field_armies(&mut state, data);
    Ok(state)
}

/// Slots of `dir`, newest first (`timestamp` descending); unreadable files are skipped.
pub fn list_saves(dir: &Path) -> Vec<SaveMeta> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut saves = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file) = path.file_name().and_then(|f| f.to_str()) else {
            continue;
        };
        if !file.ends_with(".json") || file.ends_with(".meta.json") {
            continue;
        }
        let name = file.trim_end_matches(".json").to_owned();
        let meta = std::fs::read_to_string(meta_path(dir, &name))
            .ok()
            .and_then(|text| serde_json::from_str::<SaveMeta>(&text).ok())
            .or_else(|| {
                read_save(&path).ok().map(|w| SaveMeta {
                    name: name.clone(),
                    label: display_name(&name),
                    faction: w.faction,
                    date: w.date,
                    turn: w.turn,
                    difficulty: w.difficulty,
                    timestamp: w.timestamp,
                    path: String::new(),
                })
            });
        if let Some(mut meta) = meta {
            meta.name = name.clone();
            meta.label = display_name(&name);
            meta.path = path.to_string_lossy().into_owned();
            saves.push(meta);
        }
    }
    saves.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then(a.name.cmp(&b.name)));
    saves
}

pub fn delete_save(dir: &Path, save_name: &str) {
    for path in [
        save_path(dir, save_name),
        meta_path(dir, save_name),
        dir.join(format!("{}.png", file_stem(save_name))),
        dir.join(format!("{}.played", file_stem(save_name))),
    ] {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_on_a_real_campaign() {
        let data = data_model::test_support::game_data();
        let mut state = sim_campaign::test_support::start_quiet(data, "fac_england", 1337);
        state.set_difficulty(sim_campaign::difficulty::Difficulty::Hard);
        let dir = std::env::temp_dir().join(format!("cent-ans-saves-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = write_save(&dir, "ma partie:1", &state, "2026-10-15T10:00:00").unwrap();
        assert!(path.ends_with("ma partie_1.json"));
        write_save(&dir, "auto_2", &state, "2026-10-15T11:00:00").unwrap();
        let list = list_saves(&dir);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].label, "Sauvegarde automatique 2");
        assert_eq!(list[1].faction, "fac_england");
        assert_eq!(list[1].difficulty, "hard");
        let loaded = load_save(data, &path).unwrap();
        assert_eq!(loaded.save_json(), state.save_json());
        delete_save(&dir, "ma partie:1");
        assert_eq!(list_saves(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn client_history_round_trips_and_old_saves_load_without_it() {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_england", 1337);
        let dir =
            std::env::temp_dir().join(format!("cent-ans-saves-client-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let history = serde_json::json!({
            "chronicle": {"lines": [["war", "Bataille"]], "world": [], "letters": [], "turns": 3},
            "feudal_recent": [{"date": "Printemps 1337", "text": "Hommage"}],
        });
        let path =
            write_save_with_client(&dir, "a", &state, "2026-10-15T10:00:00", history.clone())
                .unwrap();
        assert_eq!(read_save(&path).unwrap().client, history);
        // A save without the field (older, or written by `write_save`) reads back as Null.
        let plain = write_save(&dir, "b", &state, "2026-10-15T10:00:01").unwrap();
        assert!(read_save(&plain).unwrap().client.is_null());
        let text = std::fs::read_to_string(&plain).unwrap();
        assert!(!text.contains("\"client\""));
        assert_eq!(
            load_save(data, &path).unwrap().save_json(),
            state.save_json()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_stem_is_never_empty_or_unsafe() {
        for (raw, expected) in [
            ("", "sans_nom"),
            ("   ", "sans_nom"),
            (".", "sans_nom"),
            ("..", "sans_nom"),
            ("fin.", "fin"),
            ("a:b", "a_b"),
            ("con", "_con"),
            ("NUL.txt", "_NUL.txt"),
            ("com1", "_com1"),
            ("console", "console"),
            ("partie 1.5", "partie 1.5"),
            ("tab\tname", "tabname"),
        ] {
            assert_eq!(file_stem(raw), expected, "{raw:?}");
        }
        assert!(file_stem(&"é".repeat(300)).len() <= MAX_STEM_BYTES);
        assert!(stem_is_lossy("a:b") && !stem_is_lossy("auto_1"));
    }

    #[test]
    fn newer_wrapper_version_is_refused_in_french() {
        let text = format!(r#"{{"version": {}, "state": "{{}}"}}"#, SAVE_VERSION + 1);
        let error = parse_wrapper(&text).unwrap_err();
        assert!(error.contains("version plus récente"), "{error}");
        // Missing version (older wrapper) is accepted.
        assert_eq!(parse_wrapper(r#"{"state": "{}"}"#).unwrap().version, 0);
    }

    #[test]
    fn damaged_files_give_a_clear_message() {
        let data = data_model::test_support::game_data();
        let dir = std::env::temp_dir().join(format!("cent-ans-saves-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, text) in [
            ("empty", ""),
            ("cut", r#"{"version":1,"state":"{\"sta"#),
            ("blank_state", r#"{"version":1,"state":""}"#),
        ] {
            let path = dir.join(format!("{name}.json"));
            std::fs::write(&path, text).unwrap();
            let error = load_save(data, &path).unwrap_err();
            assert!(error.contains("abîmée"), "{name}: {error}");
        }
        assert!(load_save(data, &dir.join("missing.json")).is_err());
        assert!(list_saves(&dir.join("nope")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_replaces_without_leaving_a_temp_file() {
        let dir =
            std::env::temp_dir().join(format!("cent-ans-saves-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("x.json");
        write_atomic(&path, "old").unwrap();
        write_atomic(&path, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert!(!dir.join("x.json.tmp").exists());
        // A failure (target is a directory) keeps the old content and cleans the temp file.
        let blocked = dir.join("d.json");
        std::fs::create_dir(&blocked).unwrap();
        assert!(write_atomic(&blocked, "x").is_err());
        assert!(!dir.join("d.json.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn timestamp_has_iso_shape() {
        let stamp = timestamp_now();
        assert_eq!(stamp.len(), 19);
        assert_eq!(&stamp[10..11], "T");
    }
}
