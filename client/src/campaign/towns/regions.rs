//! Regional style of the kit houses (`data/art/building_regions.json`): share of half-timbered
//! houses (`framed`) and southern variants (`southern`: plaster or stone, canal tiles). Port of
//! `building_regions.gd` and of `BuildingKit.regional_candidates`. Province exceptions win over the
//! region (e.g. Lorraine in stone); the region of a province is `Province::region`.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use super::kit::{Kit, KitEntry};
use super::rng::Rng;

/// Kinds whose kit exports framed and southern variants.
pub const REGIONAL_KINDS: [&str; 3] = ["cottage", "timber", "townhouse"];

#[derive(Debug, Clone, PartialEq)]
pub struct RegionStyle {
    pub region: String,
    pub framed: f32,
    pub southern: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BuildingRegions {
    default: Option<(f32, bool)>,
    regions: HashMap<String, (f32, bool)>,
    provinces: HashMap<String, (f32, bool)>,
}

fn style_of(v: &Value) -> Option<(f32, bool)> {
    Some((
        v["framed"].as_f64()? as f32,
        v["southern"].as_bool().unwrap_or(false),
    ))
}

impl BuildingRegions {
    pub fn load(data_dir: &Path) -> Self {
        std::fs::read_to_string(data_dir.join("art/building_regions.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|v| Self::from_json(&v))
            .unwrap_or_default()
    }

    pub fn from_json(doc: &Value) -> Self {
        let table = |key: &str| -> HashMap<String, (f32, bool)> {
            doc[key]
                .as_object()
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| style_of(v).map(|s| (k.clone(), s)))
                        .collect()
                })
                .unwrap_or_default()
        };
        Self {
            default: style_of(&doc["default"]),
            regions: table("regions"),
            provinces: table("provinces"),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.default.is_none() && self.regions.is_empty()
    }

    /// Style of a province of region `region`; `None` without data (choice unchanged).
    pub fn style_for(&self, province: &str, region: &str) -> Option<RegionStyle> {
        let (framed, southern) = self
            .provinces
            .get(province)
            .or_else(|| self.regions.get(region))
            .or(self.default.as_ref())
            .copied()?;
        Some(RegionStyle {
            region: region.to_string(),
            framed,
            southern,
        })
    }
}

impl Kit {
    /// Intact models of `kind` with the given `southern` flag, sorted by name.
    fn variants(&self, kind: &str, southern: bool) -> Vec<&KitEntry> {
        self.all_of(kind)
            .into_iter()
            .filter(|e| !e.ruined && e.southern == southern)
            .collect()
    }

    /// Candidates of a regional kind: draws framed or not (`style.framed`), keeps the models of
    /// that sort (southern variants when the region asks and the draw is not framed). Falls back
    /// to stone houses for `timber`/`townhouse`, else every plain model.
    pub fn regional_candidates(
        &self,
        kind: &str,
        rng: &mut Rng,
        style: &RegionStyle,
    ) -> Vec<&KitEntry> {
        let want_framed = rng.randf() < style.framed;
        let pool = self.variants(kind, style.southern && !want_framed);
        let mut out: Vec<&KitEntry> = pool
            .into_iter()
            .filter(|e| e.framed == want_framed)
            .collect();
        if out.is_empty() && !want_framed && kind != "cottage" {
            out = self.variants("stonehouse", false);
        }
        if out.is_empty() {
            out = self.variants(kind, false);
        }
        out
    }

    /// `pick` with a regional style: regional kinds draw from `regional_candidates`.
    pub fn pick_styled(
        &self,
        kind: &str,
        length: f32,
        width: f32,
        rng: &mut Rng,
        style: Option<&RegionStyle>,
    ) -> Option<&KitEntry> {
        match style {
            Some(style) if REGIONAL_KINDS.contains(&kind) => {
                let mut candidates = self.regional_candidates(kind, rng, style);
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
            _ => self.pick(kind, length, width, rng),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_styles_and_variants() {
        let root = crate::paths::find_root().unwrap();
        let regions = BuildingRegions::load(&crate::paths::data_dir(&root));
        assert!(!regions.is_empty());
        let north = regions.style_for("prov_x", "france_nord").unwrap();
        assert!(north.framed > 0.8 && !north.southern);
        let south = regions.style_for("prov_x", "languedoc").unwrap();
        assert!(south.southern);
        let lorraine = regions.style_for("prov_lorraine", "france_est").unwrap();
        assert!((lorraine.framed - 0.3).abs() < 1e-6);
        let kit = Kit::load(&crate::paths::asset_root(&root));
        let mut rng = Rng::new(7);
        // Southern region: unframed draws give southern models only.
        let mut southern_seen = 0;
        for _ in 0..200 {
            if let Some(e) = kit.pick_styled("townhouse", 8.0, 6.0, &mut rng, Some(&south)) {
                assert!(e.framed || e.southern, "{}", e.name);
                southern_seen += usize::from(e.southern);
            }
        }
        assert!(southern_seen > 150, "{southern_seen}");
        // Northern region: mostly framed.
        let framed = (0..200)
            .filter_map(|_| kit.pick_styled("townhouse", 8.0, 6.0, &mut rng, Some(&north)))
            .filter(|e| e.framed)
            .count();
        assert!(framed > 140, "{framed}");
    }
}
