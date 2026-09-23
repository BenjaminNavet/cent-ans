//! Serializable game data types and loading helpers.
//!
//! All game data lives in `data/` as JSON files. This crate only defines the
//! shapes of that data (via `serde`) and knows how to read it from disk.
//! No game rules live here.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Identifier of a playable or non-playable faction (e.g. `"france"`).
pub type FactionId = String;
/// Identifier of a province on the campaign map.
pub type ProvinceId = String;
/// Identifier of a unit type (e.g. `"longbowmen"`).
pub type UnitTypeId = String;

/// A political entity on the campaign map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faction {
    pub id: FactionId,
    /// Display name in French (e.g. "Royaume de France").
    pub name: String,
    /// Whether the player may pick this faction at campaign start.
    #[serde(default)]
    pub playable: bool,
}

/// A region of the campaign map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Province {
    pub id: ProvinceId,
    /// Period toponym in French (e.g. "Normandie").
    pub name: String,
    /// Faction owning the province at campaign start.
    pub owner: FactionId,
}

/// A recruitable unit type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitTypeId,
    pub name: String,
    /// Number of soldiers in a full-strength unit.
    pub soldiers: u32,
}

/// Errors raised while loading data files.
#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// Reads every `*.json` file in `dir` and deserializes each one into `T`.
///
/// The result is keyed by file stem (e.g. `data/factions/france.json` gives the
/// key `"france"`), sorted for deterministic iteration. Sub-directories and
/// non-JSON files are ignored. A missing directory is an error.
pub fn load_dir<T: DeserializeOwned>(dir: &Path) -> Result<BTreeMap<String, T>, DataError> {
    let entries = fs::read_dir(dir).map_err(|source| DataError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    let mut loaded = BTreeMap::new();
    for entry in entries {
        let path = entry
            .map_err(|source| DataError::Io {
                path: dir.to_path_buf(),
                source,
            })?
            .path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|source| DataError::Io {
            path: path.clone(),
            source,
        })?;
        let value: T = serde_json::from_str(&text).map_err(|source| DataError::Json {
            path: path.clone(),
            source,
        })?;
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned();
        loaded.insert(stem, value);
    }
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_dir_reads_json_files_keyed_by_stem() {
        let dir = std::env::temp_dir().join(format!("cent-ans-data-model-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("france.json"),
            r#"{"id":"france","name":"Royaume de France","playable":true}"#,
        )
        .unwrap();
        fs::write(dir.join("notes.txt"), "ignored").unwrap();

        let factions: BTreeMap<String, Faction> = load_dir(&dir).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(factions.len(), 1);
        assert!(factions["france"].playable);
    }

    #[test]
    fn load_dir_reports_missing_directory() {
        let result: Result<BTreeMap<String, Faction>, _> =
            load_dir(Path::new("/nonexistent/cent-ans"));
        assert!(matches!(result, Err(DataError::Io { .. })));
    }
}
