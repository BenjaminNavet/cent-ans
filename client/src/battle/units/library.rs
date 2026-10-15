//! Figure library: resolves a regiment to its figure (family and variant from the unit type's
//! `figure` field), loads and caches meshes, bone textures and albedos on demand.
//! Ports `BattleSkinned.figure/rig/mesh/bone_texture`, `BattleMeshes.variant_of` and the
//! settings readers of game/scripts/battle/battle_soldiers.gd.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use data_model::{GameData, UnitCategory};
use serde::Deserialize;
use sim_battle::Unit;

use super::anim::{JitterSettings, RigClips, style_of};
use super::figures::{self, Manifest};
use super::gaits::GaitSettings;
use super::looks::UnitLooks;

/// One resolved figure: meshes (three levels of detail), textures and animation tables.
pub struct FigureSpec {
    /// Manifest key, `<kind>_<variant>`.
    pub key: String,
    pub kind: &'static str,
    pub variant: u32,
    pub style: String,
    pub rig: String,
    pub clips: RigClips,
    pub clip_table: Vec<(u32, u32, bool)>,
    pub fps: f32,
    pub lods: [Handle<Mesh>; 3],
    pub albedo: Option<Handle<Image>>,
    pub bones: Handle<Image>,
    pub variants: u32,
    pub noble: bool,
    pub ga3_lum: [f32; 2],
    pub head_y: f32,
    pub mounted: bool,
}

/// `battle_animation.json` and `battle_gore.json` settings the units need.
#[derive(Debug, Clone, Default)]
pub struct AnimSettings {
    pub jitter: JitterSettings,
    pub gaits: GaitSettings,
    /// Nominal ground speed (m/s) of locomotion clips at clip speed 1.
    pub cadence: HashMap<String, f32>,
    /// Stride factor per unit type.
    pub cadence_unit: HashMap<String, f32>,
}

#[derive(Deserialize, Default)]
struct AnimationFile {
    #[serde(default)]
    loco_rate_jitter: Option<f32>,
    #[serde(default)]
    idle_rate_jitter: Option<f32>,
    #[serde(default)]
    cavalry_gaits: Option<GaitSettings>,
}

#[derive(Deserialize, Default)]
struct GoreFile {
    #[serde(default)]
    cadence: HashMap<String, f32>,
    #[serde(default)]
    cadence_unit: HashMap<String, serde_json::Value>,
}

impl AnimSettings {
    pub fn load(data_dir: &std::path::Path) -> Self {
        fn read<T: for<'de> Deserialize<'de> + Default>(path: PathBuf) -> T {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default()
        }
        let animation: AnimationFile = read(data_dir.join("fx/battle_animation.json"));
        let gore: GoreFile = read(data_dir.join("fx/battle_gore.json"));
        let defaults = JitterSettings::default();
        Self {
            jitter: JitterSettings {
                loco: animation.loco_rate_jitter.unwrap_or(defaults.loco),
                idle: animation.idle_rate_jitter.unwrap_or(defaults.idle),
            },
            gaits: animation.cavalry_gaits.unwrap_or_default(),
            cadence: gore.cadence,
            cadence_unit: gore
                .cadence_unit
                .into_iter()
                .filter_map(|(k, v)| v.as_f64().map(|f| (k, f as f32)))
                .collect(),
        }
    }

    /// Nominal speed of a locomotion clip for a unit type (`nominal_cadence`).
    pub fn nominal_cadence(&self, clip: &str, unit_type: &str) -> Option<f32> {
        let base = *self.cadence.get(clip)?;
        Some(base * self.cadence_unit.get(unit_type).copied().unwrap_or(1.0))
    }
}

#[derive(Resource)]
pub struct FigureLibrary {
    pub manifest: Manifest,
    pub asset_root: PathBuf,
    pub looks: UnitLooks,
    pub settings: AnimSettings,
    meshes: HashMap<String, Handle<Mesh>>,
    bones: HashMap<String, Handle<Image>>,
    specs: HashMap<String, Option<Arc<FigureSpec>>>,
    /// Figures that failed to load (reported once).
    pub failures: Vec<String>,
}

/// Figure family of a regiment (`render_key` of the Godot bridge, minus siege engines).
pub fn render_kind(unit: &Unit) -> Option<&'static str> {
    if unit.category == UnitCategory::Siege {
        return None;
    }
    Some(if unit.mounted {
        "cavalry"
    } else {
        match unit.category {
            UnitCategory::Ranged => "archer",
            UnitCategory::Cavalry => "cavalry",
            _ => "infantry",
        }
    })
}

/// Variant of a `figure` field value (`infantry_3` gives 3).
pub fn variant_of_figure(figure: Option<&str>) -> u32 {
    figure
        .and_then(|f| f.rsplit_once('_'))
        .and_then(|(_, n)| n.parse().ok())
        .unwrap_or(0)
}

impl FigureLibrary {
    pub fn load(asset_root: PathBuf, data_dir: &std::path::Path) -> Result<Self, String> {
        Ok(Self {
            manifest: Manifest::load(&asset_root)?,
            looks: UnitLooks::load(data_dir),
            settings: AnimSettings::load(data_dir),
            asset_root,
            meshes: HashMap::new(),
            bones: HashMap::new(),
            specs: HashMap::new(),
            failures: Vec::new(),
        })
    }

    /// Variant of `unit_type` from its `figure` field.
    pub fn variant_for(game: &GameData, unit_type: &str) -> u32 {
        crate::battle::scene::setup::unit_type(game, unit_type)
            .map(|t| variant_of_figure(t.figure.as_deref()))
            .unwrap_or(0)
    }

    fn mesh(&mut self, path: &str, meshes: &mut Assets<Mesh>) -> Option<Handle<Mesh>> {
        if let Some(handle) = self.meshes.get(path) {
            return Some(handle.clone());
        }
        let full = self.asset_root.join(figures::relative_asset_path(path));
        match figures::load_mesh(&full) {
            Ok(mesh) => {
                let handle = meshes.add(mesh);
                self.meshes.insert(path.to_owned(), handle.clone());
                Some(handle)
            }
            Err(error) => {
                self.failures.push(error);
                None
            }
        }
    }

    fn bones(&mut self, rig: &str, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        if let Some(handle) = self.bones.get(rig) {
            return Some(handle.clone());
        }
        let entry = self.manifest.rig(rig)?;
        match figures::load_bone_texture(&self.asset_root, entry) {
            Ok(image) => {
                let handle = images.add(image);
                self.bones.insert(rig.to_owned(), handle.clone());
                Some(handle)
            }
            Err(error) => {
                self.failures.push(error);
                None
            }
        }
    }

    /// Figure for a family and variant (variant 0 of the family when the variant is absent).
    pub fn spec(
        &mut self,
        kind: &'static str,
        variant: u32,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        assets: &AssetServer,
    ) -> Option<Arc<FigureSpec>> {
        let wanted = format!("{kind}_{variant}");
        if let Some(cached) = self.specs.get(&wanted) {
            return cached.clone();
        }
        let built = self.build_spec(kind, variant, meshes, images, assets);
        self.specs.insert(wanted, built.clone());
        built
    }

    fn build_spec(
        &mut self,
        kind: &'static str,
        variant: u32,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
        assets: &AssetServer,
    ) -> Option<Arc<FigureSpec>> {
        let (variant, entry) = {
            let (v, e) = self.manifest.figure_or_default(kind, variant)?;
            (v, e.clone())
        };
        let rig = self.manifest.rig(&entry.rig)?.clone();
        let lods: Vec<Handle<Mesh>> = (0..3)
            .map(|i| {
                let path = entry.lods.get(i).or_else(|| entry.lods.last())?;
                self.mesh(path, meshes)
            })
            .collect::<Option<_>>()?;
        let bones = self.bones(&entry.rig, images)?;
        let clips = rig.clips();
        let clip_table = clips
            .names
            .iter()
            .filter_map(|n| clips.get(n))
            .map(|c| (c.start, c.frames, c.looped))
            .collect();
        let albedo = entry
            .ga3_albedo
            .as_deref()
            .map(|p| assets.load::<Image>(figures::relative_asset_path(p)));
        Some(Arc::new(FigureSpec {
            key: format!("{kind}_{variant}"),
            kind,
            variant,
            style: style_of(entry.style.as_deref(), kind, variant),
            rig: entry.rig.clone(),
            fps: rig.fps,
            clips,
            clip_table,
            lods: [lods[0].clone(), lods[1].clone(), lods[2].clone()],
            albedo,
            bones,
            variants: entry.variants,
            noble: self.manifest.is_noble(kind, variant),
            ga3_lum: entry.ga3_lum.unwrap_or([0.2, 0.2]),
            head_y: entry.ga3_head_y.unwrap_or(99.0),
            mounted: kind == "cavalry",
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_comes_from_the_figure_suffix() {
        assert_eq!(variant_of_figure(Some("infantry_3")), 3);
        assert_eq!(variant_of_figure(Some("archer_12")), 12);
        assert_eq!(variant_of_figure(None), 0);
        assert_eq!(variant_of_figure(Some("weird")), 0);
    }

    #[test]
    fn settings_read_the_bundled_data() {
        let root = crate::paths::find_root().expect("root");
        let settings = AnimSettings::load(&crate::paths::data_dir(&root));
        assert!((settings.jitter.loco - 0.03).abs() < 1e-6);
        assert!((settings.gaits.trot_min_speed - 3.4).abs() < 1e-6);
        let walk = settings.nominal_cadence("walk", "unit_knights").unwrap();
        assert!((walk - 1.35 * 0.96).abs() < 1e-5, "{walk}");
        assert!(settings.nominal_cadence("idle", "x").is_none());
    }
}
