//! River crossings of the campaign map (chantier RC, ADR 0117): the rules
//! `data/rules/river_crossings.json` (schema
//! `data/schemas/river_crossings_rules.schema.json`), the crossings of
//! `data/map/crossings_px.json` and the French river names of
//! `data/map/river_names.json`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The rules file as bundled at build time: the fallback when
/// `data/rules/river_crossings.json` is absent (test fixtures).
const BUNDLED: &str = include_str!("../../../../../data/rules/river_crossings.json");

/// Contents of `data/rules/river_crossings.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiverCrossingRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A crossing farther than this from the battle is ignored (km).
    pub search_radius_km: f64,
    /// Bed width (map px, `width` of `crossings_px.json`) at or below which
    /// a crossing has no effect.
    #[serde(default = "default_min_width_px")]
    pub min_width_px: f64,
    /// Bed width from which a crossing has its full effect (linear between).
    #[serde(default = "default_full_width_px")]
    pub full_width_px: f64,
    /// Multiplier of the attacker's damage, by crossing structure.
    pub attacker_factor: CrossingFactors,
    /// Multiplier of the defender's ranged damage, by crossing structure.
    pub defender_ranged_factor: CrossingFactors,
}

impl Default for RiverCrossingRules {
    fn default() -> Self {
        serde_json::from_str(BUNDLED).expect("bundled data/rules/river_crossings.json is valid")
    }
}

fn default_min_width_px() -> f64 {
    0.2
}

fn default_full_width_px() -> f64 {
    0.45
}

/// One coefficient per crossing structure.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossingFactors {
    pub stone_bridge: f64,
    pub wood_bridge: f64,
    pub boat_bridge: f64,
    pub ferry: f64,
    pub ford: f64,
}

/// One crossing of `data/map/crossings_px.json` (other fields ignored).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapCrossing {
    pub id: String,
    /// Empty for an anonymous road bridge.
    #[serde(default)]
    pub name: String,
    /// `bridge`, `ford` or `road`.
    #[serde(rename = "type", default)]
    pub kind: String,
    /// `stone`, `wood`, `boats`, `ferry` or `ford`.
    #[serde(default)]
    pub structure: String,
    /// Source name of the river (empty when unknown).
    #[serde(default)]
    pub river: String,
    /// Map-pixel position.
    pub px: [f32; 2],
    /// Direction of the river at the crossing (map pixels, unit length).
    pub dir: [f32; 2],
    /// Width of the river bed at the crossing (map pixels); 0 when unknown.
    #[serde(default)]
    pub width: f32,
}

#[derive(Deserialize)]
struct CrossingFile {
    #[serde(default)]
    crossings: Vec<MapCrossing>,
}

#[derive(Deserialize)]
struct RiverNameFile {
    #[serde(default)]
    names: BTreeMap<String, String>,
}

/// Reads `crossings_px.json`; empty when absent or malformed.
pub(crate) fn read_crossings(path: &Path) -> Vec<MapCrossing> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<CrossingFile>(&text).ok())
        .map(|file| file.crossings)
        .unwrap_or_default()
}

/// Reads `river_names.json`; empty when absent or malformed.
pub(crate) fn read_river_names(path: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<RiverNameFile>(&text).ok())
        .map(|file| file.names)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_parse() {
        let rules = RiverCrossingRules::default();
        assert!(rules.search_radius_km > 0.0);
        assert!(rules.attacker_factor.stone_bridge < 1.0);
        assert!(rules.defender_ranged_factor.ford > 1.0);
        assert!(rules.full_width_px > rules.min_width_px);
    }

    #[test]
    fn crossing_file_ignores_extra_fields() {
        let text = r#"{"crossings":[{"id":"a","name":"Pont","type":"bridge","structure":"stone",
            "river":"Loire","px":[1.0,2.0],"dir":[1.0,0.0],"width":1.2,"snapped":true}]}"#;
        let file: CrossingFile = serde_json::from_str(text).unwrap();
        assert_eq!(file.crossings[0].kind, "bridge");
        assert_eq!(file.crossings[0].px, [1.0, 2.0]);
        assert_eq!(file.crossings[0].width, 1.2);
    }
}
