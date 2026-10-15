//! Building kit of the Blender models (`game/assets/models/buildings/manifest.json`): picks the
//! model whose proportions fit a footprint. Port of `building_kit.gd`.
//!
//! Model frame: origin at the footprint centre, ground at y = 0 (foundations down to -2 m),
//! ridge along +X, facade towards +Z; manifest sizes are in metres.

use std::collections::BTreeMap;
use std::path::Path;

use bevy::math::Vec3;
use serde_json::Value;

use super::rng::{Rng, hash01};

/// Directory of the kit, relative to the asset root.
pub const DIR: &str = "models/buildings/";

#[derive(Debug, Clone, PartialEq)]
pub struct KitEntry {
    pub name: String,
    pub kind: String,
    pub ruined: bool,
    pub framed: bool,
    pub southern: bool,
    /// Walls footprint (m).
    pub length: f32,
    pub depth: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Kit {
    entries: BTreeMap<String, KitEntry>,
}

impl Kit {
    pub fn load(asset_root: &Path) -> Kit {
        std::fs::read_to_string(asset_root.join(DIR).join("manifest.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|v| Kit::from_json(&v))
            .unwrap_or_default()
    }

    pub fn from_json(doc: &Value) -> Kit {
        let mut entries = BTreeMap::new();
        if let Some(map) = doc.as_object() {
            for (name, e) in map {
                let (Some(kind), Some(length), Some(depth)) = (
                    e["kind"].as_str(),
                    e["length"].as_f64(),
                    e["depth"].as_f64(),
                ) else {
                    continue;
                };
                entries.insert(
                    name.clone(),
                    KitEntry {
                        name: name.clone(),
                        kind: kind.to_string(),
                        ruined: e["ruined"].as_bool().unwrap_or(false),
                        framed: e["framed"].as_bool().unwrap_or(false),
                        southern: e["southern"].as_bool().unwrap_or(false),
                        length: length as f32,
                        depth: depth as f32,
                        height: e["height"].as_f64().unwrap_or(6.0) as f32,
                    },
                );
            }
        }
        Kit { entries }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&self, name: &str) -> Option<&KitEntry> {
        self.entries.get(name)
    }

    /// Whole-model asset path of `name` (relative to the asset root).
    pub fn asset_path(name: &str) -> String {
        format!("{DIR}{name}.glb")
    }

    /// Intact models of a kind, sorted by name.
    pub fn models_of(&self, kind: &str) -> Vec<&KitEntry> {
        self.entries
            .values()
            .filter(|e| e.kind == kind && !e.ruined && !e.southern)
            .collect()
    }

    /// The model of `kind` whose proportions best fit `length` x `width` (one of the best two,
    /// drawn, to vary); `None` when the kind has no model.
    pub fn pick(&self, kind: &str, length: f32, width: f32, rng: &mut Rng) -> Option<&KitEntry> {
        let mut candidates = self.models_of(kind);
        if candidates.is_empty() {
            return None;
        }
        let target = length / width.max(0.1);
        candidates.sort_by(|a, b| {
            let da = ((a.length / a.depth) / target).ln().abs();
            let db = ((b.length / b.depth) / target).ln().abs();
            da.total_cmp(&db)
        });
        let n = candidates.len().min(2) as i32;
        Some(candidates[rng.int(0, n - 1) as usize])
    }

    /// Scale taking the model's walls footprint to `length` x `width`; the height follows the
    /// geometric mean (bounded) so roofs are not flattened.
    pub fn fit_scale(&self, name: &str, length: f32, width: f32) -> Vec3 {
        let Some(e) = self.entries.get(name) else {
            return Vec3::ONE;
        };
        let sx = length / e.length;
        let sz = width / e.depth;
        Vec3::new(sx, (sx * sz).sqrt().clamp(0.75, 1.3), sz)
    }

    /// Furniture piece (`stall`, `cart`, `barrels`, `woodpile`, `well`) chosen by index.
    pub fn prop_model(&self, kind: &str, index: u32) -> Option<&KitEntry> {
        let models = self.models_of(kind);
        if models.is_empty() {
            return None;
        }
        Some(models[(hash01(index, 29) * models.len() as f32) as usize % models.len()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn kit() -> Kit {
        Kit::from_json(&json!({
            "timber_0": {"kind": "timber", "length": 10.0, "depth": 5.0, "height": 8.0, "ruined": false},
            "timber_1": {"kind": "timber", "length": 6.0, "depth": 6.0, "height": 8.0, "ruined": false},
            "timber_ruin_0": {"kind": "timber", "length": 6.0, "depth": 6.0, "ruined": true},
        }))
    }

    #[test]
    fn picks_by_proportion_and_skips_ruins() {
        let k = kit();
        assert_eq!(k.models_of("timber").len(), 2);
        let mut rng = Rng::new(1);
        for _ in 0..20 {
            let e = k.pick("timber", 10.0, 5.0, &mut rng).unwrap();
            assert!(!e.ruined);
        }
        assert!(k.pick("church", 1.0, 1.0, &mut rng).is_none());
    }

    #[test]
    fn fit_scale_clamps_height() {
        let s = kit().fit_scale("timber_0", 20.0, 5.0);
        assert!((s.x - 2.0).abs() < 1e-5 && (s.z - 1.0).abs() < 1e-5);
        assert!((s.y - 1.3).abs() < 1e-5);
    }

    #[test]
    fn real_manifest_loads() {
        let root = crate::paths::find_root().unwrap();
        let kit = Kit::load(&crate::paths::asset_root(&root));
        assert!(kit.len() > 90, "{}", kit.len());
        assert!(!kit.models_of("townhouse").is_empty());
    }
}
