//! Generated texture packs and per-species tree textures (`tree_textures.gd`, TX T3, ADR 0241;
//! quality choice of `texture_quality.gd`, ADR 0236).
//!
//! A pack manifest (`data/art/tx_*_pack.json`) lists layers (`id`, `layer`, `mean_linear`) of a
//! layered texture (`albedo`, optional `normal`, paths `res://assets/...`). Each species of
//! `data/art/tree_species.json` may carry `textures: {bark, leaves}` (layer ids of the bark and
//! leaves packs). Species without an entry, a missing pack or the legacy flag give layer -1 and
//! the previous look. Purely visual; the battle trees consume it (`BattleTrees`).

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

pub const SPECIES_FILE: &str = "art/tree_species.json";
pub const BARK_PACK: &str = "art/tx_bark_pack.json";
pub const LEAVES_PACK: &str = "art/tx_leaves_pack.json";
/// Bark micro-detail, faded in close up (asset path).
pub const MICRO_BARK: &str = "textures/vegetation/tx_micro_bark.jpg";
/// Micro bark: strength, full distance (m), zero distance (m).
pub const MICRO_FADE: [f32; 3] = [0.7, 3.0, 14.0];
pub const HI_SIZE: u32 = 2048;
pub const LEGACY_FLAG: &str = "--legacy-textures";

/// "Texture quality" setting (`video/texture_quality`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Quality {
    #[default]
    High,
    Medium,
}

impl Quality {
    pub fn from_setting(value: &str) -> Quality {
        if value == "medium" {
            Quality::Medium
        } else {
            Quality::High
        }
    }
}

/// `TextureQuality.use_tx`: false with `--legacy-textures`.
pub fn use_tx(args: &[String]) -> bool {
    !args.iter().any(|a| a == LEGACY_FLAG)
}

/// `res://assets/a/b.jpg` -> `a/b.jpg` (path in the Bevy asset root); other paths unchanged.
pub fn asset_path(res_path: &str) -> String {
    res_path
        .strip_prefix("res://assets/")
        .or_else(|| res_path.strip_prefix("res://"))
        .unwrap_or(res_path)
        .to_string()
}

/// `TextureQuality.texture_path`: `a/b.jpg` -> `a/hi/b.jpg` in high quality when `exists`.
pub fn texture_path(path: &str, quality: Quality, exists: impl Fn(&str) -> bool) -> String {
    if quality != Quality::High {
        return path.to_string();
    }
    let hi = match path.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/hi/{file}"),
        None => format!("hi/{path}"),
    };
    if exists(&hi) { hi } else { path.to_string() }
}

/// `TextureQuality.manifest_path`: `art/x.json` -> `art/x_2048.json` in high quality when present.
pub fn manifest_path(file: &str, quality: Quality, data_dir: &Path) -> String {
    if quality != Quality::High {
        return file.to_string();
    }
    let hi = match file.rsplit_once('.') {
        Some((base, ext)) => format!("{base}_{HI_SIZE}.{ext}"),
        None => format!("{file}_{HI_SIZE}"),
    };
    if data_dir.join(&hi).is_file() {
        hi
    } else {
        file.to_string()
    }
}

/// One parsed texture pack.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pack {
    /// Layer id -> layer index.
    pub layers: HashMap<String, i32>,
    /// Mean linear albedo per layer index (ground materials).
    pub means: Vec<[f32; 3]>,
    /// Asset paths of the layered textures.
    pub albedo: Option<String>,
    pub normal: Option<String>,
    pub layer_size: u32,
    pub grid: (u32, u32),
}

impl Pack {
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Parses a manifest; `None` when it is not an object.
    pub fn parse(value: &Value) -> Option<Pack> {
        let object = value.as_object()?;
        let mut pack = Pack::default();
        let mut max_layer = -1i32;
        for entry in object.get("layers").and_then(Value::as_array)? {
            let (Some(id), Some(layer)) = (
                entry.get("id").and_then(Value::as_str),
                entry.get("layer").and_then(Value::as_i64),
            ) else {
                continue;
            };
            pack.layers.insert(id.to_string(), layer as i32);
            max_layer = max_layer.max(layer as i32);
        }
        pack.means = vec![[0.18; 3]; (max_layer + 1).max(0) as usize];
        for entry in object["layers"].as_array()? {
            let (Some(layer), Some(mean)) = (
                entry.get("layer").and_then(Value::as_i64),
                entry.get("mean_linear").and_then(Value::as_array),
            ) else {
                continue;
            };
            let channel = |i: usize| mean.get(i).and_then(Value::as_f64).unwrap_or(0.18) as f32;
            if let Some(slot) = pack.means.get_mut(layer as usize) {
                *slot = [channel(0), channel(1), channel(2)];
            }
        }
        pack.albedo = object.get("albedo").and_then(Value::as_str).map(asset_path);
        pack.normal = object.get("normal").and_then(Value::as_str).map(asset_path);
        pack.layer_size = object
            .get("layer_size")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32;
        if let Some(grid) = object.get("grid").and_then(Value::as_array)
            && let (Some(a), Some(b)) = (
                grid.first().and_then(Value::as_u64),
                grid.get(1).and_then(Value::as_u64),
            )
        {
            pack.grid = (a as u32, b as u32);
        }
        Some(pack)
    }

    /// Reads a manifest of the data dir at `file` (after the quality substitution); `None`
    /// when missing or invalid. With `quality` high, the `hi/` variants of the textures are used.
    pub fn read(data_dir: &Path, asset_root: &Path, file: &str, quality: Quality) -> Option<Pack> {
        let chosen = manifest_path(file, quality, data_dir);
        let text = std::fs::read_to_string(data_dir.join(chosen)).ok()?;
        let mut pack = Pack::parse(&serde_json::from_str(&text).ok()?)?;
        let exists = |p: &str| asset_root.join(p).is_file();
        for path in [&mut pack.albedo, &mut pack.normal].into_iter().flatten() {
            *path = texture_path(path, quality, exists);
        }
        Some(pack)
    }
}

/// Species -> layer ids of `tree_species.json`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpeciesTextures {
    /// Species id -> (bark layer id, leaves layer id).
    pub species: HashMap<String, (String, String)>,
}

impl SpeciesTextures {
    pub fn parse(value: &Value) -> SpeciesTextures {
        let mut out = SpeciesTextures::default();
        for entry in value
            .get("species")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(id), Some(textures)) = (
                entry.get("id").and_then(Value::as_str),
                entry.get("textures"),
            ) else {
                continue;
            };
            let text = |key: &str| {
                textures
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string()
            };
            out.species
                .insert(id.to_string(), (text("bark"), text("leaves")));
        }
        out
    }
}

/// `TreeTextures`: bark and leaves layers per species.
#[derive(Clone, Debug, Default)]
pub struct TreeTextures {
    pub species: SpeciesTextures,
    pub bark: Pack,
    pub leaves: Pack,
}

impl TreeTextures {
    /// Loads the species table and the two packs; empty when `use_tx` is false.
    pub fn load(
        data_dir: &Path,
        asset_root: &Path,
        quality: Quality,
        use_tx: bool,
    ) -> TreeTextures {
        if !use_tx {
            return TreeTextures::default();
        }
        let species = std::fs::read_to_string(data_dir.join(SPECIES_FILE))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|v| SpeciesTextures::parse(&v))
            .unwrap_or_default();
        TreeTextures {
            species,
            bark: Pack::read(data_dir, asset_root, BARK_PACK, quality).unwrap_or_default(),
            leaves: Pack::read(data_dir, asset_root, LEAVES_PACK, quality).unwrap_or_default(),
        }
    }

    /// Bark layer of `species`, -1 without a texture.
    pub fn bark_layer(&self, species: &str) -> i32 {
        self.species
            .species
            .get(species)
            .and_then(|(bark, _)| self.bark.layers.get(bark))
            .copied()
            .unwrap_or(-1)
    }

    /// Leaves sheet layer of `species`, -1 without a texture.
    pub fn leaves_layer(&self, species: &str) -> i32 {
        self.species
            .species
            .get(species)
            .and_then(|(_, leaves)| self.leaves.layers.get(leaves))
            .copied()
            .unwrap_or(-1)
    }

    /// Whether the regional bark applies to `species` (`apply_bark` returns true).
    pub fn has_bark(&self, species: &str) -> bool {
        self.bark_layer(species) >= 0 && self.bark.albedo.is_some()
    }

    /// Whether the leaves sheet applies to `species` (`apply_leaves` returns true).
    pub fn has_leaves(&self, species: &str) -> bool {
        self.leaves_layer(species) >= 0 && self.leaves.albedo.is_some()
    }

    /// Species that carry both textures (dump).
    pub fn textured_species(&self) -> usize {
        self.species
            .species
            .keys()
            .filter(|s| self.has_bark(s) || self.has_leaves(s))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pack_json() -> Value {
        json!({"layer_size": 1024, "grid": [3, 4],
            "albedo": "res://assets/textures/vegetation/a.jpg",
            "normal": "res://assets/textures/vegetation/n.jpg",
            "layers": [
                {"id": "oak_bark", "layer": 0, "mean_linear": [0.1, 0.2, 0.3]},
                {"id": "beech_bark", "layer": 2}]})
    }

    #[test]
    fn pack_parses_layers_means_and_paths() {
        let pack = Pack::parse(&pack_json()).unwrap();
        assert_eq!(pack.layers["beech_bark"], 2);
        assert_eq!(pack.means.len(), 3);
        assert_eq!(pack.means[0], [0.1, 0.2, 0.3]);
        assert_eq!(pack.albedo.as_deref(), Some("textures/vegetation/a.jpg"));
        assert_eq!(pack.grid, (3, 4));
    }

    #[test]
    fn texture_path_prefers_hi_variant_only_in_high_quality() {
        let exists = |p: &str| p == "textures/hi/a.jpg";
        assert_eq!(
            texture_path("textures/a.jpg", Quality::High, exists),
            "textures/hi/a.jpg"
        );
        assert_eq!(
            texture_path("textures/a.jpg", Quality::Medium, exists),
            "textures/a.jpg"
        );
        assert_eq!(
            texture_path("textures/b.jpg", Quality::High, exists),
            "textures/b.jpg"
        );
    }

    #[test]
    fn species_layers_resolve_or_give_minus_one() {
        let textures = TreeTextures {
            species: SpeciesTextures::parse(&json!({"species": [
                {"id": "oak", "textures": {"bark": "oak_bark", "leaves": "oak_leaves"}},
                {"id": "fir"}]})),
            bark: Pack::parse(&pack_json()).unwrap(),
            leaves: Pack::default(),
        };
        assert_eq!(textures.bark_layer("oak"), 0);
        assert_eq!(textures.leaves_layer("oak"), -1);
        assert_eq!(textures.bark_layer("fir"), -1);
        assert!(textures.has_bark("oak"));
        assert!(!textures.has_leaves("oak"));
    }

    #[test]
    fn legacy_flag_disables_tx() {
        assert!(use_tx(&[]));
        assert!(!use_tx(&["--legacy-textures".into()]));
    }

    #[test]
    fn real_packs_load_when_the_repo_is_present() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let textures = TreeTextures::load(
            &crate::paths::data_dir(&root),
            &crate::paths::asset_root(&root),
            Quality::High,
            true,
        );
        assert!(textures.bark.layer_count() > 0);
        assert!(textures.textured_species() > 0);
    }
}
