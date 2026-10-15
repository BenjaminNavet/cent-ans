//! Shared PBR material of the Blender kit buildings (port of `building_materials.gd` atlas part,
//! `building_atlas.gdshader`, `town_building.gdshader` and `building_aging.gdshaderinc`).
//!
//! The kit glb files (`models/buildings`, `town_kit`, `outbuildings`, the settlement maquettes)
//! carry one material named `Building`, UVs in metres and vertex colours (rgb: building tint x
//! baked occlusion x patina, alpha: layer of the matter, `(index + 0.5) / 16`). Without this
//! module Bevy renders them with the glb's plain white material, i.e. white/grey volumes. Every
//! glTF mesh whose material is named `Building` gets [`BuildingAtlasMaterial`] instead, in any
//! state (campaign towns, battle site, siege town), so other lots need no change.
//!
//! - Layers, tints, tile sizes, roof and plain ranges: `data/art/building_materials.json`.
//! - Textures: `textures/buildings/building_{albedo,normal}_array.jpg`, 16 square layers stacked
//!   vertically, turned into mip-mapped 2D arrays on load (`battle::decor::ground::strip_to_array`).
//! - Variants of the material are cached by [`AtlasStyle`] (forced layer and box UVs for the town
//!   blocks, walls and towers, block tint, soot, snow).
//!
//! Not ported: regional matters (`tx_building_pack.json`, TX T4), masonry micro-grain, roofscape
//! colour of the far roofs (the far layer `towns/far` owns its own look).

use std::collections::HashMap;
use std::path::Path;

use bevy::asset::embedded_asset;
use bevy::gltf::GltfMaterialName;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::battle::decor::ground::{PendingStrip, strip_to_array};

/// Layers of the atlas (`atlas_layers` of the data file; the glb files bake indices 0..15).
pub const LAYER_COUNT: usize = 16;
/// Layer order baked in the kit glb files (`kit_export.ATLAS_LAYERS`); the data file's
/// `atlas_layers` must list the same (tested).
pub const BAKED_LAYERS: [&str; LAYER_COUNT] = [
    "Plaster",
    "Rubble",
    "Ashlar",
    "Masonry",
    "Timber",
    "Planks",
    "Door",
    "RoofTile",
    "RoofFlat",
    "RoofSlate",
    "Thatch",
    "Window",
    "Iron",
    "Canvas",
    "TimberFrame",
    "TimberFrameFar",
];

/// Vertex alpha coding the layer `name` (Plaster when unknown).
pub fn layer_alpha(name: &str) -> f32 {
    let i = BAKED_LAYERS.iter().position(|n| *n == name).unwrap_or(0);
    (i as f32 + 0.5) / LAYER_COUNT as f32
}

/// Name of the kit material replaced by the atlas.
pub const KIT_MATERIAL: &str = "Building";
const ALBEDO: &str = "textures/buildings/building_albedo_array.jpg";
const NORMAL: &str = "textures/buildings/building_normal_array.jpg";
/// Default aging (`BuildingMaterials.SR5_AGING`).
pub const AGING: f32 = 0.5;
/// Moss on the walls (`SR5_WALL_MOSS`).
const WALL_MOSS: f32 = 0.25;

/// Layers of `data/art/building_materials.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct AtlasSpec {
    pub names: Vec<String>,
    /// rgb tint (linear), a roughness.
    pub tint: [Vec4; LAYER_COUNT],
    /// Repeats per metre.
    pub tile: [f32; LAYER_COUNT],
    pub first_plain: i32,
    pub plain_last: i32,
    pub roof_first: i32,
    pub roof_last: i32,
}

impl Default for AtlasSpec {
    fn default() -> Self {
        AtlasSpec {
            names: Vec::new(),
            tint: [Vec4::new(1.0, 1.0, 1.0, 0.9); LAYER_COUNT],
            tile: [1.0; LAYER_COUNT],
            first_plain: LAYER_COUNT as i32,
            plain_last: -2,
            roof_first: 7,
            roof_last: 10,
        }
    }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn rgb_of(v: &Value) -> Option<[f32; 3]> {
    let a = v.as_array()?;
    Some([
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
        a.get(2)?.as_f64()? as f32,
    ])
}

impl AtlasSpec {
    pub fn load(data_dir: &Path) -> AtlasSpec {
        std::fs::read_to_string(data_dir.join("art/building_materials.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|v| AtlasSpec::from_json(&v))
            .unwrap_or_default()
    }

    /// Same tables as `BuildingMaterials._atlas`: textured layers keep their tint as is (it
    /// multiplies an sRGB texture already decoded), plain layers are sRGB colours.
    pub fn from_json(doc: &Value) -> AtlasSpec {
        let mut spec = AtlasSpec {
            names: doc["atlas_layers"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            ..default()
        };
        let mut textured: HashMap<&str, (Vec4, f32)> = HashMap::new();
        for e in doc["textured"].as_array().into_iter().flatten() {
            let (Some(name), Some(t), Some(size)) = (
                e["name"].as_str(),
                rgb_of(&e["tint"]),
                e["tile_size_m"].as_f64(),
            ) else {
                continue;
            };
            let rough = e["roughness"].as_f64().unwrap_or(0.9) as f32;
            textured.insert(
                name,
                (
                    Vec4::new(t[0], t[1], t[2], rough),
                    1.0 / (size as f32).max(0.1),
                ),
            );
        }
        let mut plain: HashMap<&str, Vec4> = HashMap::new();
        for e in doc["plain"].as_array().into_iter().flatten() {
            let (Some(name), Some(c)) = (e["name"].as_str(), rgb_of(&e["color"])) else {
                continue;
            };
            let rough = e["roughness"].as_f64().unwrap_or(0.9) as f32;
            plain.insert(
                name,
                Vec4::new(
                    srgb_to_linear(c[0]),
                    srgb_to_linear(c[1]),
                    srgb_to_linear(c[2]),
                    rough,
                ),
            );
        }
        let (mut first_plain, mut plain_last) = (-1, -2);
        for (i, name) in spec.names.iter().enumerate().take(LAYER_COUNT) {
            if let Some((tint, tile)) = textured.get(name.as_str()) {
                spec.tint[i] = *tint;
                spec.tile[i] = *tile;
            } else if let Some(tint) = plain.get(name.as_str()) {
                spec.tint[i] = *tint;
                if first_plain < 0 {
                    first_plain = i as i32;
                }
                plain_last = i as i32;
            }
        }
        spec.first_plain = if first_plain >= 0 {
            first_plain
        } else {
            LAYER_COUNT as i32
        };
        spec.plain_last = plain_last;
        if let Some(i) = spec.layer("RoofTile") {
            spec.roof_first = i as i32;
        }
        if let Some(i) = spec.layer("Thatch") {
            spec.roof_last = i as i32;
        }
        spec
    }

    /// Index of the layer `name`.
    pub fn layer(&self, name: &str) -> Option<u32> {
        self.names.iter().position(|n| n == name).map(|i| i as u32)
    }
}

/// One variant of the material.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtlasStyle {
    /// Layer of meshes without vertex colours.
    pub layer: u32,
    /// Box UVs from the world position (meshes without metre UVs).
    pub box_uv: bool,
    /// Colour multiplier (block tint variation).
    pub color: Vec3,
    /// Soot of a sacked town (0-1).
    pub soot: f32,
    /// Snow on the roofs (0-1).
    pub snow: f32,
    /// Metres per world unit (1 in battle, the map scale under a 1:1 town root).
    pub metres_per_unit: f32,
    /// Normal-map strength (0: none, the campaign far views).
    pub normals: f32,
    /// Distance (world units) where the textures fade to their mean (0: never).
    pub mean_beyond: f32,
}

impl Default for AtlasStyle {
    fn default() -> Self {
        AtlasStyle {
            layer: 0,
            box_uv: false,
            color: Vec3::ONE,
            soot: 0.0,
            snow: 0.0,
            metres_per_unit: 1.0,
            normals: 1.0,
            mean_beyond: 0.0,
        }
    }
}

impl AtlasStyle {
    fn key(&self) -> [u32; 11] {
        [
            self.layer,
            u32::from(self.box_uv),
            self.color.x.to_bits(),
            self.color.y.to_bits(),
            self.color.z.to_bits(),
            self.soot.to_bits(),
            self.snow.to_bits(),
            self.metres_per_unit.to_bits(),
            self.normals.to_bits(),
            self.mean_beyond.to_bits(),
            0,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct AtlasParams {
    pub tint: [Vec4; LAYER_COUNT],
    pub tile: [Vec4; 4],
    pub ranges: Vec4,
    pub opts: Vec4,
    pub style: Vec4,
    pub color: Vec4,
}

impl AtlasParams {
    pub fn new(spec: &AtlasSpec, style: &AtlasStyle) -> AtlasParams {
        let mut tile = [Vec4::ONE; 4];
        for (i, t) in spec.tile.iter().enumerate() {
            tile[i / 4][i % 4] = *t;
        }
        AtlasParams {
            tint: spec.tint,
            tile,
            ranges: Vec4::new(
                spec.first_plain as f32,
                spec.plain_last as f32,
                spec.roof_first as f32,
                spec.roof_last as f32,
            ),
            opts: Vec4::new(
                AGING,
                style.snow,
                style.normals,
                if style.box_uv { 1.0 } else { 0.0 },
            ),
            style: Vec4::new(
                style.layer as f32,
                style.soot,
                style.metres_per_unit,
                style.mean_beyond,
            ),
            color: style.color.extend(WALL_MOSS),
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct BuildingAtlasExt {
    #[uniform(100)]
    pub params: AtlasParams,
    #[texture(101, dimension = "2d_array")]
    #[sampler(102)]
    pub albedo: Handle<Image>,
    #[texture(103, dimension = "2d_array")]
    #[sampler(104)]
    pub normal: Handle<Image>,
}

impl MaterialExtension for BuildingAtlasExt {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/towns/atlas/building_atlas.wesl".into()
    }
}

pub type BuildingAtlasMaterial = ExtendedMaterial<StandardMaterial, BuildingAtlasExt>;

/// The atlas textures and the cached variants of the material.
#[derive(Resource)]
pub struct BuildingAtlas {
    pub spec: AtlasSpec,
    albedo: Handle<Image>,
    normal: Handle<Image>,
    variants: HashMap<[u32; 11], Handle<BuildingAtlasMaterial>>,
    /// Style of the glb pieces remapped from now on (battle snow, for instance).
    pub kit_style: AtlasStyle,
}

impl BuildingAtlas {
    /// Material of `style` (cached).
    pub fn material(
        &mut self,
        materials: &mut Assets<BuildingAtlasMaterial>,
        style: &AtlasStyle,
    ) -> Handle<BuildingAtlasMaterial> {
        let key = style.key();
        if let Some(h) = self.variants.get(&key) {
            return h.clone();
        }
        let handle = materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.9,
                reflectance: 0.3,
                // The kit is closed but the town blocks and some props are seen from inside.
                double_sided: true,
                cull_mode: None,
                ..default()
            },
            extension: BuildingAtlasExt {
                params: AtlasParams::new(&self.spec, style),
                albedo: self.albedo.clone(),
                normal: self.normal.clone(),
            },
        });
        self.variants.insert(key, handle.clone());
        handle
    }

    /// Layer index of `name`, 0 when unknown.
    pub fn layer(&self, name: &str) -> u32 {
        self.spec.layer(name).unwrap_or(0)
    }
}

/// Counters for the dump.
#[derive(Resource, Default, Debug)]
pub struct AtlasStats {
    pub remapped: usize,
    pub arrays_converted: usize,
    pub layers: usize,
}

#[derive(Resource, Default)]
struct AtlasStrips(Vec<PendingStrip>);

/// glTF meshes already handled (kept off the query once remapped or skipped).
#[derive(Component)]
pub struct AtlasDone;

pub fn add(app: &mut App) {
    embedded_asset!(app, "building_atlas.wesl");
    app.add_plugins(MaterialPlugin::<BuildingAtlasMaterial>::default())
        .init_resource::<AtlasStats>()
        .register_dump(dump_atlas)
        .add_systems(Startup, init_atlas)
        .add_systems(
            Update,
            (
                convert_atlas.run_if(resource_exists::<AtlasStrips>),
                remap_kit.run_if(resource_exists::<BuildingAtlas>),
            ),
        );
}

fn init_atlas(mut commands: Commands, assets: Res<AssetServer>, mut stats: ResMut<AtlasStats>) {
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    let spec = AtlasSpec::load(&crate::paths::data_dir(&root));
    stats.layers = spec.names.len();
    let albedo: Handle<Image> = assets.load(ALBEDO);
    let normal: Handle<Image> = assets
        .load_builder()
        .with_settings(|s: &mut bevy::image::ImageLoaderSettings| s.is_srgb = false)
        .load(NORMAL);
    commands.insert_resource(AtlasStrips(vec![
        PendingStrip {
            handle: albedo.clone(),
            layers: LAYER_COUNT as u32,
            anisotropy: 8,
        },
        PendingStrip {
            handle: normal.clone(),
            layers: LAYER_COUNT as u32,
            anisotropy: 8,
        },
    ]));
    commands.insert_resource(BuildingAtlas {
        spec,
        albedo,
        normal,
        variants: HashMap::new(),
        kit_style: AtlasStyle::default(),
    });
}

fn convert_atlas(
    mut commands: Commands,
    mut pending: ResMut<AtlasStrips>,
    mut images: ResMut<Assets<Image>>,
    mut stats: ResMut<AtlasStats>,
) {
    pending.0.retain(|strip| {
        let Some(mut image) = images.get_mut(&strip.handle) else {
            return true;
        };
        if strip_to_array(&mut image, strip.layers, strip.anisotropy) {
            stats.arrays_converted += 1;
        } else {
            warn!("building atlas: texture strip has an unexpected shape, not converted");
        }
        false
    });
    if pending.0.is_empty() {
        commands.remove_resource::<AtlasStrips>();
    }
}

/// Every glTF mesh whose material is the kit's `Building` gets the atlas material.
fn remap_kit(
    mut commands: Commands,
    mut atlas: ResMut<BuildingAtlas>,
    mut materials: ResMut<Assets<BuildingAtlasMaterial>>,
    mut stats: ResMut<AtlasStats>,
    new: Query<
        (Entity, &GltfMaterialName),
        (With<MeshMaterial3d<StandardMaterial>>, Without<AtlasDone>),
    >,
) {
    let mut handle = None;
    for (entity, name) in &new {
        let base = name.0.split('.').next().unwrap_or("");
        if base != KIT_MATERIAL {
            commands.entity(entity).try_insert(AtlasDone);
            continue;
        }
        let style = atlas.kit_style;
        let h = handle
            .get_or_insert_with(|| atlas.material(&mut materials, &style))
            .clone();
        commands
            .entity(entity)
            .try_remove::<MeshMaterial3d<StandardMaterial>>()
            .try_insert((MeshMaterial3d(h), AtlasDone));
        stats.remapped += 1;
    }
}

fn dump_atlas(world: &World) -> (String, Value) {
    let stats = world.resource::<AtlasStats>();
    let variants = world
        .get_resource::<BuildingAtlas>()
        .map_or(0, |a| a.variants.len());
    (
        "building_atlas".into(),
        json!({
            "layers": stats.layers,
            "arrays_converted": stats.arrays_converted,
            "remapped_meshes": stats.remapped,
            "variants": variants,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_data_matches_the_baked_layers() {
        let root = crate::paths::find_root().unwrap();
        let spec = AtlasSpec::load(&crate::paths::data_dir(&root));
        assert_eq!(spec.names, BAKED_LAYERS.map(String::from).to_vec());
        assert_eq!(spec.layer("Plaster"), Some(0));
        assert_eq!(spec.roof_first, 7);
        assert_eq!(spec.roof_last, 10);
        assert_eq!((spec.first_plain, spec.plain_last), (11, 13));
        // Plaster tiles every 2.2 m, glass is dark and glossy.
        assert!((spec.tile[0] - 1.0 / 2.2).abs() < 1e-5);
        assert!(spec.tint[11].x < 0.01 && spec.tint[11].w < 0.5);
        // Timber frame (after the plain block) keeps a texture tile.
        assert!((spec.tile[14] - 1.0 / 2.2).abs() < 1e-5);
    }

    #[test]
    fn layer_alpha_decodes_like_the_shader() {
        for (i, name) in BAKED_LAYERS.iter().enumerate() {
            assert_eq!((layer_alpha(name) * 16.0) as usize, i);
        }
    }

    #[test]
    fn params_pack_tiles_by_four() {
        let mut spec = AtlasSpec::default();
        spec.tile[5] = 0.25;
        let p = AtlasParams::new(
            &spec,
            &AtlasStyle {
                layer: 3,
                box_uv: true,
                ..default()
            },
        );
        assert_eq!(p.tile[1].y, 0.25);
        assert_eq!(p.style.x, 3.0);
        assert_eq!(p.opts.w, 1.0);
    }
}
