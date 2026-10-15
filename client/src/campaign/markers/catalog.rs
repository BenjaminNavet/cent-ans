//! Static render catalogs: `data/map/settlement_markers.json` (ranks, label visibility, declutter)
//! and `data/art/town_maquettes.json` (maquette sizes, families, collision). Port of
//! `settlement_markers.gd` and `town_maquette_data.gd`. Read only, no game rule.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

/// Place kinds, in the order of `TownMaquetteData.KINDS`.
pub const KINDS: [&str; 5] = ["city", "town", "castle", "abbey", "village"];

/// Label priority (smaller = first): city > town > others (`SettlementData.LABEL_PRIORITY`).
pub fn label_priority(kind: &str) -> u8 {
    match kind {
        "city" => 0,
        "town" => 1,
        "castle" => 2,
        "abbey" => 3,
        "village" => 4,
        _ => 9,
    }
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

// --- settlement_markers.json -------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RankRule {
    pub ids: Option<Vec<String>>,
    pub kind: Option<String>,
    pub min_fortification: Option<u32>,
    pub min_weight: Option<u32>,
    #[serde(default = "one")]
    pub rank: u8,
}

fn one() -> u8 {
    1
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct VisibilityTier {
    #[serde(default)]
    pub max_distance: f32,
    #[serde(default)]
    pub min_rank: HashMap<String, u8>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MarkerCatalog {
    #[serde(default)]
    pub rank_rules: Vec<RankRule>,
    #[serde(default = "one")]
    pub default_rank: u8,
    #[serde(default)]
    pub size_px: HashMap<String, f32>,
    #[serde(default)]
    pub kind_size_factor: HashMap<String, f32>,
    #[serde(default)]
    pub visibility: Vec<VisibilityTier>,
    #[serde(default)]
    pub declutter: HashMap<String, Value>,
    /// `shield`: `size_factor`, `gap_px`, `max_distance_by_rank`.
    #[serde(default)]
    pub shield: HashMap<String, Value>,
}

/// What `rank_of` needs of a place.
pub struct RankInput<'a> {
    pub id: &'a str,
    pub kind: &'a str,
    pub fortification: u32,
    pub weight: u32,
}

impl MarkerCatalog {
    pub fn load(data_dir: &Path) -> Self {
        read_json(&data_dir.join("map/settlement_markers.json"))
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    pub fn is_valid(&self) -> bool {
        !self.rank_rules.is_empty()
    }

    /// Rank (1 to 4): first rule that applies.
    pub fn rank_of(&self, p: &RankInput) -> u8 {
        for rule in &self.rank_rules {
            if rule
                .ids
                .as_ref()
                .is_some_and(|ids| !ids.iter().any(|i| i == p.id))
            {
                continue;
            }
            if rule.kind.as_deref().is_some_and(|k| k != p.kind) {
                continue;
            }
            if rule.min_fortification.is_some_and(|m| p.fortification < m) {
                continue;
            }
            if rule.min_weight.is_some_and(|m| p.weight < m) {
                continue;
            }
            return rule.rank;
        }
        self.default_rank
    }

    /// Screen size (px) of a marker by rank and kind.
    pub fn size_px(&self, kind: &str, rank: u8) -> f32 {
        let size = self
            .size_px
            .get(&rank.to_string())
            .or_else(|| self.size_px.get("1"))
            .copied()
            .unwrap_or(24.0);
        size * self.kind_size_factor.get(kind).copied().unwrap_or(1.0)
    }

    /// Camera distance up to which a label stays shown: the largest `max_distance` of the tiers
    /// where the rank reaches the minimum of its kind; 0 = never.
    pub fn visible_until(&self, kind: &str, rank: u8) -> f32 {
        self.visibility
            .iter()
            .filter(|t| t.min_rank.get(kind).is_some_and(|m| rank >= *m))
            .map(|t| t.max_distance)
            .fold(0.0, f32::max)
    }

    /// Side (px) of the holder's shield above a name: marker size x `shield.size_factor`.
    pub fn shield_side(&self, kind: &str, rank: u8) -> f32 {
        let factor = self
            .shield
            .get("size_factor")
            .and_then(Value::as_f64)
            .unwrap_or(0.45) as f32;
        self.size_px(kind, rank) * factor
    }

    /// Gap (px) between the shield and the name.
    pub fn shield_gap(&self) -> f32 {
        self.shield
            .get("gap_px")
            .and_then(Value::as_f64)
            .unwrap_or(1.0) as f32
    }

    /// Camera distance up to which a place of `rank` carries its shield (absent rank = always).
    pub fn shield_until(&self, rank: u8) -> f32 {
        self.shield
            .get("max_distance_by_rank")
            .and_then(|m| m.get(rank.to_string()))
            .and_then(Value::as_f64)
            .map_or(f32::INFINITY, |v| v as f32)
    }

    pub fn declutter_f32(&self, key: &str, default: f32) -> f32 {
        self.declutter
            .get(key)
            .and_then(Value::as_f64)
            .map_or(default, |v| v as f32)
    }
}

// --- town_maquettes.json -----------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Demotion {
    pub below_weight: f32,
    #[serde(rename = "as")]
    pub as_kind: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FamilyLists {
    #[serde(default)]
    pub cultures: Vec<String>,
    #[serde(default)]
    pub regions: Vec<String>,
    #[serde(default)]
    pub religions: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Collision {
    #[serde(default = "one_f")]
    pub min_scale: f32,
    #[serde(default)]
    pub margin: f32,
    #[serde(default)]
    pub absorb_ratio: f32,
}

fn one_f() -> f32 {
    1.0
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WeightGain {
    #[serde(default = "one_f")]
    pub min: f32,
    #[serde(default = "one_f")]
    pub max: f32,
    #[serde(flatten)]
    pub bounds: HashMap<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Maquettes {
    #[serde(default)]
    pub sizes: HashMap<String, f32>,
    #[serde(default)]
    pub native_width: HashMap<String, f32>,
    #[serde(default = "one_f")]
    pub landmark_scale: f32,
    #[serde(default)]
    pub visibility: HashMap<String, f32>,
    #[serde(default)]
    pub collision: Collision,
    #[serde(default)]
    pub weight_gain: WeightGain,
    #[serde(default)]
    pub display_demotion: HashMap<String, Value>,
    #[serde(default)]
    pub variants: Vec<String>,
    #[serde(default)]
    pub default_family: String,
    #[serde(default)]
    pub families: HashMap<String, FamilyLists>,
}

impl Maquettes {
    pub fn load(data_dir: &Path) -> Self {
        let mut m: Self = read_json(&data_dir.join("art/town_maquettes.json"))
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        if m.default_family.is_empty() {
            m.default_family = "west".into();
        }
        m
    }

    pub fn is_empty(&self) -> bool {
        self.sizes.is_empty()
    }

    /// Display kind: a small place takes the maquette of a more modest rank.
    pub fn kind_of(&self, kind: &str, weight: f32) -> &'static str {
        let kind = KINDS
            .iter()
            .find(|k| **k == kind)
            .copied()
            .unwrap_or("village");
        if let Some(d) = self
            .display_demotion
            .get(kind)
            .and_then(|v| serde_json::from_value::<Demotion>(v.clone()).ok())
            && weight <= d.below_weight
            && let Some(shown) = KINDS.iter().find(|k| **k == d.as_kind)
        {
            return shown;
        }
        kind
    }

    /// World width (units) of the maquette of a kind.
    pub fn width(&self, kind: &str) -> f32 {
        self.sizes.get(kind).copied().unwrap_or(0.0)
    }

    /// Scale to apply to the kit model to reach `width(kind)`.
    pub fn model_scale(&self, kind: &str) -> f32 {
        match self.native_width.get(kind) {
            Some(native) if *native > 0.0 => self.width(kind) / native,
            _ => 0.0,
        }
    }

    /// Size gain from the weight (`min` at the low bound to `max` at the high, square root).
    pub fn weight_gain(&self, kind: &str, weight: f32) -> f32 {
        let bounds = self
            .weight_gain
            .bounds
            .get(kind)
            .and_then(Value::as_array)
            .filter(|b| b.len() >= 2);
        let Some(bounds) = bounds else { return 1.0 };
        let (lo, hi) = (
            bounds[0].as_f64().unwrap_or(0.0) as f32,
            bounds[1].as_f64().unwrap_or(0.0) as f32,
        );
        if hi <= lo {
            return 1.0;
        }
        let t = ((weight - lo) / (hi - lo)).clamp(0.0, 1.0);
        self.weight_gain.min + (self.weight_gain.max - self.weight_gain.min) * t.sqrt()
    }

    /// Range (rig distance) beyond which a kind is no longer drawn.
    pub fn range(&self, kind: &str) -> f32 {
        self.visibility.get(kind).copied().unwrap_or(0.0)
    }

    pub fn fade_margin(&self) -> f32 {
        self.visibility.get("fade_margin").copied().unwrap_or(0.0)
    }

    /// Architecture family: culture, then region, then religion, else the default family.
    /// Families are scanned in sorted name order so the result is deterministic.
    pub fn family_for(&self, culture: &str, region: &str, religion: &str) -> &str {
        let mut names: Vec<&String> = self.families.keys().collect();
        names.sort();
        for (which, value) in [(0, culture), (1, region), (2, religion)] {
            for name in &names {
                let f = &self.families[*name];
                let list = match which {
                    0 => &f.cultures,
                    1 => &f.regions,
                    _ => &f.religions,
                };
                if list.iter().any(|v| v == value) {
                    return name;
                }
            }
        }
        &self.default_family
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> MarkerCatalog {
        serde_json::from_value(serde_json::json!({
            "rank_rules": [
                {"ids": ["set_paris"], "rank": 4},
                {"kind": "city", "rank": 2},
                {"kind": "town", "min_weight": 30, "rank": 2}
            ],
            "default_rank": 1,
            "size_px": {"1": 32, "2": 40},
            "kind_size_factor": {"village": 0.5},
            "visibility": [
                {"max_distance": 380, "min_rank": {"village": 1, "city": 1}},
                {"max_distance": 700, "min_rank": {"city": 1, "village": 3}}
            ]
        }))
        .unwrap()
    }

    #[test]
    fn rank_rules_first_match_wins() {
        let c = catalog();
        let r = |id, kind, w| {
            c.rank_of(&RankInput {
                id,
                kind,
                fortification: 0,
                weight: w,
            })
        };
        assert_eq!(r("set_paris", "city", 1), 4);
        assert_eq!(r("set_x", "city", 1), 2);
        assert_eq!(r("set_x", "town", 29), 1);
        assert_eq!(r("set_x", "town", 30), 2);
    }

    #[test]
    fn sizes_and_visibility() {
        let c = catalog();
        assert_eq!(c.size_px("village", 2), 20.0);
        assert_eq!(c.size_px("city", 9), 32.0);
        assert_eq!(c.visible_until("city", 1), 700.0);
        assert_eq!(c.visible_until("village", 1), 380.0);
        assert_eq!(c.visible_until("village", 3), 700.0);
        assert_eq!(c.visible_until("castle", 4), 0.0);
    }

    fn maquettes() -> Maquettes {
        serde_json::from_value(serde_json::json!({
            "sizes": {"city": 10.0}, "native_width": {"city": 2.0},
            "weight_gain": {"min": 0.8, "max": 1.2, "city": [0, 100]},
            "display_demotion": {"city": {"below_weight": 12, "as": "town"}},
            "default_family": "west",
            "families": {"med": {"cultures": ["cul_x"]}, "west": {"regions": ["r"]}}
        }))
        .unwrap()
    }

    #[test]
    fn maquette_math() {
        let m = maquettes();
        assert_eq!(m.model_scale("city"), 5.0);
        assert_eq!(m.model_scale("town"), 0.0);
        assert!((m.weight_gain("city", 100.0) - 1.2).abs() < 1e-6);
        assert!((m.weight_gain("city", 0.0) - 0.8).abs() < 1e-6);
        assert_eq!(m.weight_gain("castle", 5.0), 1.0);
        assert_eq!(m.kind_of("city", 5.0), "town");
        assert_eq!(m.kind_of("city", 50.0), "city");
        assert_eq!(m.kind_of("bogus", 50.0), "village");
        assert_eq!(m.family_for("cul_x", "r", ""), "med");
        assert_eq!(m.family_for("cul_y", "r", ""), "west");
        assert_eq!(m.family_for("", "", ""), "west");
    }
}

#[cfg(test)]
mod shield_tests {
    use super::*;

    #[test]
    fn shield_rules_from_real_data() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let c = MarkerCatalog::load(&crate::paths::data_dir(&root));
        assert!((c.shield_side("city", 4) - 58.0 * 0.45).abs() < 1e-3);
        assert_eq!(c.shield_until(1), 160.0);
        assert!(c.shield_until(4).is_infinite());
    }
}
