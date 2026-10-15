//! Static description of the places of the map: settlements (from `GameData` + the pixel positions
//! of `settlements_px.json`), their rank, maquette model, size and collision reduction, and the
//! landmark cities (`data/landmarks/*.json`, Meshy maquettes of ADR 0356). Port of
//! `settlement_data.gd`, the setup of `town_maquette_layer.gd`, `landmark_library.gd`.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use data_model::GameData;
use serde_json::Value;

use super::catalog::{KINDS, Maquettes, MarkerCatalog, RankInput, label_priority};
use super::placement::{model_candidates, solve_collisions, variant_of, yaw_of};

/// A landmark city plan: only what the render needs.
#[derive(Debug, Clone, PartialEq)]
pub struct LandmarkPlan {
    pub id: String,
    pub settlement: String,
    /// `scale.core_radius_px`: radius of the core in map units (1 unit = 1 px).
    pub core_radius: f32,
    /// Generated glb (`dn_campaign_models.json` `landmarks.<id>`), asset path relative to
    /// `game/assets`, with its `width_factor` and `yaw_deg`.
    pub generated: Option<GeneratedLandmark>,
    /// Fallback flat Blender maquette `models/landmarks/<id>.glb`.
    pub fallback_model: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedLandmark {
    pub model: String,
    pub width_factor: f32,
    pub yaw_deg: f32,
}

/// Reads `data/landmarks/*.json` and the generated-model table.
pub fn load_landmarks(data_dir: &Path, assets_dir: &Path) -> Vec<LandmarkPlan> {
    let table: Value = std::fs::read_to_string(data_dir.join("art/dn_campaign_models.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null);
    let default_lod = table["defaults"]["lod"].as_i64().unwrap_or(1);
    let Ok(dir) = std::fs::read_dir(data_dir.join("landmarks")) else {
        return Vec::new();
    };
    let mut files: Vec<_> = dir
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let mut plans = Vec::new();
    for file in files {
        let Some(doc) = std::fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        else {
            continue;
        };
        let (Some(id), Some(settlement)) = (doc["id"].as_str(), doc["settlement"].as_str()) else {
            continue;
        };
        let entry = &table["landmarks"][id];
        let generated = entry["path"].as_str().and_then(|path| {
            let model = if path.contains("_lod") {
                format!("models/{path}.glb")
            } else {
                let lod = entry["lod"].as_i64().unwrap_or(default_lod);
                format!("models/{path}_lod{lod}.glb")
            };
            assets_dir
                .join(&model)
                .is_file()
                .then(|| GeneratedLandmark {
                    model,
                    width_factor: entry["width_factor"].as_f64().unwrap_or(1.0) as f32,
                    yaw_deg: entry["yaw_deg"].as_f64().unwrap_or(0.0) as f32,
                })
        });
        plans.push(LandmarkPlan {
            id: id.to_string(),
            settlement: settlement.to_string(),
            core_radius: doc["scale"]["core_radius_px"].as_f64().unwrap_or(5.0) as f32,
            generated,
            fallback_model: format!("models/landmarks/{id}.glb"),
        });
    }
    plans
}

/// One settlement as drawn on the campaign map.
#[derive(Debug, Clone)]
pub struct Place {
    pub id: String,
    pub province: String,
    /// Rule kind (`city`, `town`, `castle`, `abbey`, `village`).
    pub kind: String,
    /// Kind of the maquette shown (small places are demoted).
    pub shown_kind: &'static str,
    pub name: String,
    /// Position in map pixels (= world units, see `placement::px_to_world`).
    pub px: Vec2,
    pub weight: u32,
    pub fortification: u32,
    /// Current holder; refreshed from the simulation.
    pub controller: String,
    pub rank: u8,
    pub family: String,
    /// Asset path of the maquette model (`models/settlements/<name>.glb`), `None` if none exists
    /// or the place is absorbed by a landmark city.
    pub model: Option<String>,
    /// Uniform scale of the kit model (gain and collision included).
    pub model_scale: f32,
    pub yaw: f32,
    /// Half width in world units after collision reduction.
    pub radius: f32,
    /// Index into `Places::landmarks` when this place is a landmark city.
    pub landmark: Option<usize>,
    /// Swallowed by a landmark maquette (name stays, no own maquette).
    pub absorbed: bool,
    /// Candidate model name that had to fall back to another family.
    pub family_fallback: bool,
}

#[derive(Resource, Default)]
pub struct Places {
    /// Sorted by label priority, then weight (descending), then id (`SettlementData` order).
    pub places: Vec<Place>,
    pub landmarks: Vec<LandmarkPlan>,
    pub index_by_id: HashMap<String, usize>,
    pub catalog: MarkerCatalog,
    pub maquettes: Maquettes,
}

impl Places {
    pub fn get(&self, id: &str) -> Option<&Place> {
        self.index_by_id.get(id).map(|i| &self.places[*i])
    }
}

/// Builds the places. `model_exists(name)` tells whether `models/settlements/<name>.glb` exists.
pub fn build_places(
    data: &GameData,
    catalog: MarkerCatalog,
    maquettes: Maquettes,
    landmarks: Vec<LandmarkPlan>,
    model_exists: &dyn Fn(&str) -> bool,
) -> Places {
    let mut places: Vec<Place> = Vec::new();
    for (id, s) in &data.settlements {
        let Some(px) = data.settlement_px.get(id) else {
            continue;
        };
        let kind = s.kind.key().to_string();
        let weight = u32::from(s.weight);
        let fort = u32::from(s.fortification_level);
        let province = data.provinces.get(&s.province);
        let owner = s
            .owner
            .as_ref()
            .or(province.map(|p| &p.owner))
            .map(|f| f.as_str().to_string())
            .unwrap_or_default();
        let family = province
            .map(|p| {
                maquettes
                    .family_for(p.culture.as_str(), &p.region, p.religion.as_str())
                    .to_string()
            })
            .unwrap_or_else(|| maquettes.default_family.clone());
        places.push(Place {
            id: id.as_str().to_string(),
            province: s.province.as_str().to_string(),
            shown_kind: maquettes.kind_of(&kind, weight as f32),
            rank: catalog.rank_of(&RankInput {
                id: id.as_str(),
                kind: &kind,
                fortification: fort,
                weight,
            }),
            kind,
            name: s.name.display.clone(),
            px: Vec2::new(px[0], px[1]),
            weight,
            fortification: fort,
            controller: owner,
            family,
            model: None,
            model_scale: 0.0,
            yaw: yaw_of(id.as_str()),
            radius: 0.0,
            landmark: None,
            absorbed: false,
            family_fallback: false,
        });
    }
    places.sort_by(|a, b| {
        label_priority(&a.kind)
            .cmp(&label_priority(&b.kind))
            .then(b.weight.cmp(&a.weight))
            .then(a.id.cmp(&b.id))
    });
    let index_by_id: HashMap<String, usize> = places
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.clone(), i))
        .collect();

    // Base sizes, landmark cities, collision reduction (setup of town_maquette_layer.gd).
    let count = places.len();
    let mut gain = vec![1.0_f32; count];
    let mut radius = vec![0.0_f32; count];
    let mut fixed = vec![false; count];
    for (i, p) in places.iter_mut().enumerate() {
        gain[i] = maquettes.weight_gain(p.shown_kind, p.weight as f32);
        radius[i] = maquettes.width(p.shown_kind) * 0.5 * gain[i];
        if let Some(l) = landmarks.iter().position(|l| l.settlement == p.id) {
            p.landmark = Some(l);
            fixed[i] = true;
            radius[i] = landmarks[l].core_radius * maquettes.landmark_scale;
        }
    }
    let centers: Vec<Vec2> = places.iter().map(|p| p.px).collect();
    let factor = solve_collisions(
        &centers,
        &radius,
        &fixed,
        maquettes.collision.min_scale,
        maquettes.collision.margin,
    );
    let landmark_idx: Vec<usize> = (0..count)
        .filter(|i| places[*i].landmark.is_some())
        .collect();
    for i in 0..count {
        if places[i].landmark.is_some() {
            places[i].radius = radius[i];
            continue;
        }
        let swallowed = landmark_idx.iter().any(|&j| {
            centers[i].distance(centers[j]) < radius[j] * maquettes.collision.absorb_ratio
        });
        places[i].radius = radius[i] * factor[i];
        if swallowed {
            places[i].absorbed = true;
            continue;
        }
        let variant = variant_of(&places[i].id, &maquettes.variants).unwrap_or("a");
        let cands = model_candidates(
            places[i].shown_kind,
            &places[i].family,
            variant,
            &maquettes.default_family,
        );
        if let Some((n, name)) = cands.iter().enumerate().find(|(_, c)| model_exists(c)) {
            places[i].family_fallback = n > 0;
            places[i].model = Some(format!("models/settlements/{name}.glb"));
            places[i].model_scale =
                maquettes.model_scale(places[i].shown_kind) * gain[i] * factor[i];
        }
    }
    Places {
        places,
        landmarks,
        index_by_id,
        catalog,
        maquettes,
    }
}

/// Counts of maquettes by kind (dump).
pub fn count_by_kind(places: &Places) -> HashMap<&'static str, usize> {
    let mut out: HashMap<&'static str, usize> = KINDS.iter().map(|k| (*k, 0)).collect();
    for p in places.places.iter().filter(|p| p.model.is_some()) {
        *out.entry(p.shown_kind).or_default() += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths;

    /// Real data: loads `data/` and builds the places (skipped without the repository).
    #[test]
    fn real_data_builds_places() {
        let Ok(root) = paths::find_root() else { return };
        let data_dir = paths::data_dir(&root);
        let Ok((data, _)) = GameData::load(&data_dir) else {
            return;
        };
        let assets = paths::asset_root(&root);
        let places = build_places(
            &data,
            MarkerCatalog::load(&data_dir),
            Maquettes::load(&data_dir),
            load_landmarks(&data_dir, &assets),
            &|n| assets.join(format!("models/settlements/{n}.glb")).is_file(),
        );
        assert!(places.places.len() > 300, "{}", places.places.len());
        let paris = places.get("set_paris").expect("paris");
        assert!(paris.landmark.is_some());
        assert_eq!(paris.rank, 4);
        // Sorted: cities first.
        assert_eq!(places.places[0].kind, "city");
        assert!(places.places.iter().filter(|p| p.model.is_some()).count() > 100);
        assert!(places.places.iter().all(|p| p.radius >= 0.0));
    }
}
