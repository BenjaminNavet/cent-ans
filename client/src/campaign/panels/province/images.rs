//! Illustrations of the Buildings tab (port of the image part of
//! game/scripts/ui/settlement_buildings_view.gd): the tier plate of the colony in the header
//! (`illustrations/settlement_tiers/<kind>_<tier>.jpg`, nearest tier below) and one `bld_*`
//! illustration per building card. All paths are relative to `game/assets`.

use std::collections::HashSet;

use bevy::prelude::*;
use serde_json::Value;

use super::view::Snapshot;
use crate::paths;

pub const MAX_TIER: u8 = 6;
pub const HEADER_IMAGE_HEIGHT: f32 = 96.0;
pub const CARD_IMAGE_HEIGHT: f32 = 58.0;
const DEFAULT_TIER_DIR: &str = "illustrations/settlement_tiers";

/// Which illustration files exist, plus `data/ui/settlement_tiers.json` (French tier names).
#[derive(Resource, Debug, Clone, Default)]
pub struct BuildingImages {
    tier_dir: String,
    names: std::collections::HashMap<String, Vec<String>>,
    files: HashSet<String>,
}

impl BuildingImages {
    pub fn parse(tiers: &Value, files: HashSet<String>) -> Self {
        let tier_dir = tiers["illustration_dir"]
            .as_str()
            .map_or(DEFAULT_TIER_DIR, |d| d)
            .trim_start_matches("res://assets/")
            .to_owned();
        let names = tiers["names"]
            .as_object()
            .map(|map| {
                map.iter()
                    .map(|(kind, list)| {
                        let list = list.as_array().map_or_else(Vec::new, |l| {
                            l.iter()
                                .filter_map(|n| n.as_str().map(str::to_owned))
                                .collect()
                        });
                        (kind.clone(), list)
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            tier_dir,
            names,
            files,
        }
    }

    /// Reads the tier JSON and lists the image folders on disk.
    pub fn load() -> Self {
        let Ok(root) = paths::find_root() else {
            return Self::default();
        };
        let tiers =
            std::fs::read_to_string(paths::data_dir(&root).join("ui/settlement_tiers.json"))
                .ok()
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                .unwrap_or(Value::Null);
        let assets = paths::asset_root(&root);
        let mut files = HashSet::new();
        for dir in ["illustrations", "illustrations/settlement_tiers"] {
            if let Ok(entries) = std::fs::read_dir(assets.join(dir)) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.ends_with(".jpg") {
                        files.insert(format!("{dir}/{name}"));
                    }
                }
            }
        }
        Self::parse(&tiers, files)
    }

    /// `building_image`: `illustrations/<id>.jpg` when the file exists.
    pub fn building_image(&self, building_id: &str) -> Option<String> {
        let path = format!("illustrations/{building_id}.jpg");
        self.files.contains(&path).then_some(path)
    }

    /// `tier_image_path`: the nearest existing plate at or below `tier`.
    pub fn tier_image(&self, kind: &str, tier: u8) -> Option<String> {
        (1..=tier.clamp(1, MAX_TIER)).rev().find_map(|candidate| {
            let path = format!("{}/{kind}_{candidate}.jpg", self.tier_dir);
            self.files.contains(&path).then_some(path)
        })
    }

    /// `tier_name`: French name of a tier ("" when unknown).
    pub fn tier_name(&self, kind: &str, tier: u8) -> &str {
        usize::from(tier)
            .checked_sub(1)
            .and_then(|i| self.names.get(kind)?.get(i))
            .map_or("", String::as_str)
    }

    /// `tier_caption`: "Palier 3/6 — Ville marchande".
    pub fn tier_caption(&self, kind: &str, tier: u8) -> String {
        format!("Palier {tier}/{MAX_TIER} — {}", self.tier_name(kind, tier))
    }

    pub fn image_count(&self) -> usize {
        self.files.len()
    }
}

/// Header plate: the tier plate, else the highest-level standing building that has an image.
pub fn header_image(images: &BuildingImages, snap: &Snapshot) -> Option<String> {
    let tree = snap.tree.as_ref()?;
    images.tier_image(&tree.kind, snap.tier).or_else(|| {
        let mut best: Option<(u8, String)> = None;
        for node in tree.nodes.iter().filter(|n| n.state == "built") {
            let level = node.tier;
            if best.as_ref().is_none_or(|(l, _)| level > *l)
                && let Some(path) = images.building_image(&node.building)
            {
                best = Some((level, path));
            }
        }
        best.map(|(_, path)| path)
    })
}

/// Every image bound in the Buildings tab: `(widget name, asset path)`. Used by the view and by
/// the dump so both agree.
pub fn bound_images(images: &BuildingImages, snap: &Snapshot) -> Vec<(String, String)> {
    let mut bound = Vec::new();
    if let Some(path) = header_image(images, snap) {
        bound.push(("prov:tier_image".to_owned(), path));
    }
    if let Some(tree) = &snap.tree {
        for node in tree.nodes.iter().filter(|n| n.allowed_here) {
            if let Some(path) = images.building_image(&node.building) {
                bound.push((format!("prov:node_image:{}", node.building), path));
            }
        }
    }
    bound
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> BuildingImages {
        let files = [
            "illustrations/bld_abbey.jpg",
            "illustrations/settlement_tiers/town_1.jpg",
            "illustrations/settlement_tiers/town_2.jpg",
            "illustrations/settlement_tiers/castle_1.jpg",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        BuildingImages::parse(
            &json!({"illustration_dir": "res://assets/illustrations/settlement_tiers",
                    "names": {"town": ["Bourgade", "Petite ville"]}}),
            files,
        )
    }

    #[test]
    fn building_id_maps_to_its_illustration() {
        let images = sample();
        assert_eq!(
            images.building_image("bld_abbey").as_deref(),
            Some("illustrations/bld_abbey.jpg")
        );
        assert_eq!(images.building_image("bld_unknown"), None);
    }

    #[test]
    fn tier_falls_back_to_the_nearest_lower_plate() {
        let images = sample();
        assert_eq!(
            images.tier_image("town", 2).as_deref(),
            Some("illustrations/settlement_tiers/town_2.jpg")
        );
        assert_eq!(
            images.tier_image("town", 5).as_deref(),
            Some("illustrations/settlement_tiers/town_2.jpg")
        );
        assert_eq!(
            images.tier_image("castle", 4).as_deref(),
            Some("illustrations/settlement_tiers/castle_1.jpg")
        );
        assert_eq!(images.tier_image("abbey", 3), None);
    }

    #[test]
    fn tier_names_and_caption() {
        let images = sample();
        assert_eq!(images.tier_name("town", 2), "Petite ville");
        assert_eq!(images.tier_name("town", 9), "");
        assert_eq!(images.tier_name("town", 0), "");
        assert_eq!(images.tier_caption("town", 1), "Palier 1/6 — Bourgade");
    }
}
