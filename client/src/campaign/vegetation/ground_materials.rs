//! Campaign ground materials (`ground_materials.gd`): fal.ai / TX top-down tiling textures packed
//! in two layered textures (albedo with equalised mean luminance, the colour map gives the hue;
//! OpenGL normal (R, G) + roughness (B)). The id -> layer mapping comes from the generated
//! manifests `data/art/ground_materials_pack.json` (HB) and `tx_campaign_parcels_pack.json` (TX,
//! ADR 0243). Not wired into the terrain shader yet in Godot either (HB3); this is the loader.

use std::path::Path;

use bevy::math::Vec3;
use serde_json::Value;

use super::textures::{Pack, Quality};

pub const MANIFEST_FILE: &str = "art/ground_materials_pack.json";
pub const TX_MANIFEST_FILE: &str = "art/tx_campaign_parcels_pack.json";

/// Source of the parcels: HB (original fal.ai materials) or TX (regional, default).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Source {
    Hb,
    #[default]
    Tx,
}

/// Loaded arrays: manifest layers plus the texture paths and the mean linear albedo.
#[derive(Clone, Debug)]
pub struct GroundMaterials {
    pub source: Source,
    pub pack: Pack,
}

impl GroundMaterials {
    pub fn manifest_file(source: Source) -> &'static str {
        match source {
            Source::Hb => MANIFEST_FILE,
            Source::Tx => TX_MANIFEST_FILE,
        }
    }

    /// `load_arrays`: `None` when the manifest, a texture or the layer count is unavailable.
    pub fn load(
        data_dir: &Path,
        asset_root: &Path,
        source: Source,
        quality: Quality,
    ) -> Option<GroundMaterials> {
        let pack = Pack::read(data_dir, asset_root, Self::manifest_file(source), quality)?;
        let albedo = pack.albedo.as_ref()?;
        let normal = pack.normal.as_ref()?;
        if !asset_root.join(albedo).is_file() || !asset_root.join(normal).is_file() {
            return None;
        }
        Some(GroundMaterials { source, pack })
    }

    /// Identifier -> layer (-1 for an unknown id).
    pub fn layer(&self, id: &str) -> i32 {
        self.pack.layers.get(id).copied().unwrap_or(-1)
    }

    /// Mean linear albedo of a layer (`means`).
    pub fn mean(&self, layer: i32) -> Option<Vec3> {
        self.pack
            .means
            .get(usize::try_from(layer).ok()?)
            .map(|m| Vec3::from_array(*m))
    }

    pub fn from_manifest(source: Source, manifest: &Value) -> Option<GroundMaterials> {
        Some(GroundMaterials {
            source,
            pack: Pack::parse(manifest)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn layers_and_means_resolve() {
        let materials = GroundMaterials::from_manifest(
            Source::Hb,
            &json!({"layers": [
                {"id": "wheat_ripe", "layer": 0, "mean_linear": [0.2671, 0.1651, 0.0713]},
                {"id": "barley_green", "layer": 1}]}),
        )
        .unwrap();
        assert_eq!(materials.layer("barley_green"), 1);
        assert_eq!(materials.layer("nothing"), -1);
        assert_eq!(materials.mean(0).unwrap().x, 0.2671);
        assert_eq!(materials.mean(1).unwrap().x, 0.18);
        assert!(materials.mean(-1).is_none());
    }

    #[test]
    fn both_real_manifests_load_with_textures() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        for source in [Source::Hb, Source::Tx] {
            let materials = GroundMaterials::load(
                &crate::paths::data_dir(&root),
                &crate::paths::asset_root(&root),
                source,
                Quality::Medium,
            )
            .unwrap_or_else(|| panic!("{source:?} ground materials"));
            assert!(materials.pack.layer_count() >= 20, "{source:?}");
            assert!(materials.layer("wheat_ripe") >= 0);
        }
    }
}
