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

/// Version of the wrapper (`SAVE_VERSION` of `sim_facade.gd`).
pub const SAVE_VERSION: u32 = 1;
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

/// Godot's `String.validate_filename`: forbidden characters become `_`.
pub fn file_stem(save_name: &str) -> String {
    save_name
        .trim()
        .chars()
        .map(|c| match c {
            ':' | '/' | '\\' | '?' | '*' | '"' | '|' | '%' | '<' | '>' => '_',
            other => other,
        })
        .collect()
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
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let wrapper = SaveWrapper {
        version: SAVE_VERSION,
        faction: state.player_faction().as_str().to_owned(),
        date: state.date_label(),
        turn: state.turn(),
        difficulty: state.difficulty().key().to_owned(),
        timestamp: timestamp.to_owned(),
        state: state.save_json(),
    };
    let path = save_path(dir, save_name);
    let text = serde_json::to_string(&wrapper).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
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
    let _ = std::fs::write(meta_path(dir, save_name), meta_text);
    Ok(path)
}

pub fn read_save(path: &Path) -> Result<SaveWrapper, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("sauvegarde illisible : {e}"))
}

/// Restores the campaign of a save file, with the same post-load fixes as `load_from_string`.
pub fn load_save(data: &GameData, path: &Path) -> Result<CampaignState, String> {
    let wrapper = read_save(path)?;
    let mut state = CampaignState::load_json(&wrapper.state).map_err(|e| e.to_string())?;
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
    fn timestamp_has_iso_shape() {
        let stamp = timestamp_now();
        assert_eq!(stamp.len(), 19);
        assert_eq!(&stamp[10..11], "T");
    }
}
