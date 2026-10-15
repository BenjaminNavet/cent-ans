//! House arms (port of `game/scripts/ui/house_arms.gd`): `data/heraldry/houses.json` maps a
//! house name (« Plantagenêt ») or id to its shield `assets/heraldry/houses/<id>.png`, with the
//! blazon and its certainty for the tooltip.

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct House {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub blazon: String,
    #[serde(default)]
    pub certainty: String,
}

#[derive(Debug, Clone, Default)]
pub struct HouseArms {
    houses: Vec<House>,
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    houses: Vec<House>,
}

impl HouseArms {
    pub fn from_json(text: &str) -> Self {
        let mut houses =
            serde_json::from_str::<File>(text).map_or_else(|_| Vec::new(), |f| f.houses);
        houses.sort_by(|a, b| a.id.cmp(&b.id));
        Self { houses }
    }

    pub fn load(data_dir: &Path) -> Self {
        std::fs::read_to_string(data_dir.join("heraldry/houses.json"))
            .map_or_else(|_| Self::default(), |t| Self::from_json(&t))
    }

    pub fn len(&self) -> usize {
        self.houses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.houses.is_empty()
    }

    /// Entry of the house `key` (display name or id).
    pub fn entry(&self, key: &str) -> Option<&House> {
        self.houses.iter().find(|h| h.name == key || h.id == key)
    }

    /// Asset path (under `game/assets`) of the shield of `house`.
    pub fn asset_path(&self, house: &str) -> Option<String> {
        self.entry(house)
            .map(|h| format!("heraldry/houses/{}.png", h.id))
    }

    /// « Armes de la maison Valois : D'azur… (armes de substitution) ».
    pub fn tooltip(&self, house: &str) -> String {
        let Some(h) = self.entry(house) else {
            return String::new();
        };
        let mut text = format!("Armes de la maison {} : {}", h.name, h.blazon);
        if h.certainty == "substitution" {
            text.push_str(" (armes de substitution)");
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"badges":{},"houses":[
        {"id":"valois","name":"Valois","blazon":"D'azur semé de fleurs de lis d'or.","certainty":"attesté"},
        {"id":"abel","name":"Abel","blazon":"Plain.","certainty":"substitution"}]}"#;

    #[test]
    fn looks_up_by_name_and_id() {
        let arms = HouseArms::from_json(SAMPLE);
        assert_eq!(arms.len(), 2);
        assert_eq!(arms.entry("Valois").unwrap().id, "valois");
        assert_eq!(arms.entry("abel").unwrap().name, "Abel");
        assert_eq!(
            arms.asset_path("Valois").as_deref(),
            Some("heraldry/houses/valois.png")
        );
        assert!(arms.asset_path("Inconnue").is_none());
        assert!(arms.tooltip("Abel").ends_with("(armes de substitution)"));
        assert_eq!(arms.tooltip("Inconnue"), "");
    }

    #[test]
    fn real_data_file_parses_when_present() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let arms = HouseArms::load(&crate::paths::data_dir(&root));
        assert!(arms.len() > 10);
        assert!(arms.entry("Valois").is_some());
    }
}
