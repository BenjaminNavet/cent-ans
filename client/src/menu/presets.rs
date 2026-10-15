//! Named armies of the custom battle (game/scripts/ui/custom_battle_presets.gd): a faction, a
//! budget, granted technologies and units kept under a name in `custom_battle_presets.json`
//! (map name -> army), next to the saves folder. No rule here: the core judges a reloaded army.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// Longest preset name (characters).
pub const MAX_NAME: usize = 40;

/// One kept army.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PresetArmy {
    pub faction: String,
    pub budget: u32,
    pub technologies: Vec<String>,
    pub units: Vec<String>,
}

/// `custom_battle_presets.json` in `dir` (the per-user `Cent Ans` folder).
pub fn presets_path(dir: &Path) -> PathBuf {
    dir.join("custom_battle_presets.json")
}

/// Name trimmed and bounded.
pub fn clean_name(raw: &str) -> String {
    raw.trim().chars().take(MAX_NAME).collect()
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Normalised army, `None` when `value` is not one (no faction).
pub fn sanitize(value: &Value) -> Option<PresetArmy> {
    let faction = value.get("faction")?.as_str().filter(|f| !f.is_empty())?;
    Some(PresetArmy {
        faction: faction.to_owned(),
        budget: value
            .get("budget")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
        technologies: strings(value.get("technologies")),
        units: strings(value.get("units")),
    })
}

/// Every kept army by name (empty when the file is absent or unreadable).
pub fn load_all(path: &Path) -> BTreeMap<String, PresetArmy> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&text) else {
        return BTreeMap::new();
    };
    map.iter()
        .filter(|(name, _)| !name.is_empty())
        .filter_map(|(name, army)| sanitize(army).map(|a| (name.clone(), a)))
        .collect()
}

fn write(path: &Path, all: &BTreeMap<String, PresetArmy>) -> bool {
    let map: serde_json::Map<String, Value> = all
        .iter()
        .map(|(name, a)| {
            (
                name.clone(),
                json!({
                    "faction": a.faction,
                    "budget": a.budget,
                    "technologies": a.technologies,
                    "units": a.units,
                }),
            )
        })
        .collect();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    serde_json::to_string_pretty(&Value::Object(map))
        .ok()
        .is_some_and(|text| std::fs::write(path, text).is_ok())
}

/// Keeps (or replaces) `name`; false when the name is empty or the write fails.
pub fn save(path: &Path, name: &str, army: &PresetArmy) -> bool {
    let name = clean_name(name);
    if name.is_empty() || army.faction.is_empty() {
        return false;
    }
    let mut all = load_all(path);
    all.insert(name, army.clone());
    write(path, &all)
}

/// Forgets `name`; false when it did not exist.
pub fn remove(path: &Path, name: &str) -> bool {
    let mut all = load_all(path);
    all.remove(name).is_some() && write(path, &all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_remove_round_trip() {
        let dir = std::env::temp_dir().join(format!("cent-ans-presets-{}", std::process::id()));
        let path = presets_path(&dir);
        let army = PresetArmy {
            faction: "fac_france".into(),
            budget: 9000,
            technologies: vec!["tech_x".into()],
            units: vec!["unit_a".into(), "unit_b".into()],
        };
        assert!(!save(&path, "  ", &army));
        assert!(save(&path, "  Ost de France ", &army));
        let all = load_all(&path);
        assert_eq!(all.get("Ost de France"), Some(&army));
        assert!(remove(&path, "Ost de France"));
        assert!(!remove(&path, "Ost de France"));
        assert!(load_all(&path).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn sanitize_rejects_armies_without_faction() {
        assert!(sanitize(&json!({"budget": 5})).is_none());
        assert!(sanitize(&json!("x")).is_none());
        let army = sanitize(&json!({"faction": "f", "units": ["u", 3]})).unwrap();
        assert_eq!(army.units, vec!["u", "3"]);
    }
}
