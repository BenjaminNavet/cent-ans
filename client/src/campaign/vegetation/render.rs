//! Tree rendering: GPU-instanced batches (one draw per species and level of detail, reusing the
//! `battle::units::instancing` pipeline) built from the scattered trees.
//!
//! Levels of detail (`Vegetation._update_model_zone`, `generalised_*` of `MapPropScale`):
//! - near the focus point, the decimated glTF model of the species (`dn/vegetation/<id>_lod2`,
//!   normalised to height 1 and the species' mean crown ratio);
//! - everywhere else, an impostor drawn by `billboard.rs` / `billboard.wesl`: a camera-facing card
//!   sampling the generated impostor atlas (`ga3_impostors_albedo` / `_normal`, 8 azimuths x one
//!   row per species), with azimuth blend, normals, seasonal tint and wind.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::gltf::GltfAssetLabel;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

use crate::battle::units::instancing::MaterialParams;

use super::billboard::{Mode, Params as VegParams, push_record};

use super::data::VegetationWorld;
use super::tiles::TreeInst;

pub const ATLAS_ALBEDO: &str = "textures/vegetation/ga3/ga3_impostors_albedo.png";
pub const ATLAS_NORMAL: &str = "textures/vegetation/ga3/ga3_impostors_normal.png";
/// Azimuth columns of the atlas.
pub const ATLAS_VIEWS: u32 = 8;
/// Baking frame (`ortho`, `foot` of `campaign_tree_impostor.gdshaderinc`).
const ORTHO: f32 = 1.7;
const FOOT: f32 = 0.2588;
/// Tree models closer than this to the focus are real meshes (`generalised_model_*`).
pub const MODEL_RADIUS_FACTOR: f32 = 0.6;
pub const MODEL_RADIUS_MIN: f32 = 8.0;
pub const MODEL_RADIUS_MAX: f32 = 40.0;
pub const MODEL_MAX_DISTANCE: f32 = 150.0;
/// Bytes of a model record: x, y, z, yaw (deg), scale, model index.
pub const RECORD_BYTES: usize = 24;

/// Radius of the model zone for a camera at `distance` of its focus, 0 beyond the range.
pub fn model_radius(distance: f32) -> f32 {
    if distance > MODEL_MAX_DISTANCE {
        return 0.0;
    }
    (MODEL_RADIUS_FACTOR * distance).clamp(MODEL_RADIUS_MIN, MODEL_RADIUS_MAX)
}

/// A glTF model of a species while it loads, then once normalised.
pub struct ModelSlot {
    pub mesh_src: Handle<Mesh>,
    pub material_src: Handle<bevy::gltf::GltfMaterial>,
    pub ready: Option<(Handle<Mesh>, Handle<Image>)>,
    pub failed: bool,
}

/// Models and impostor cards of the species, built lazily.
#[derive(Default)]
pub struct TreeAssets {
    pub models: HashMap<u16, ModelSlot>,
    pub quad: Option<Handle<Mesh>>,
    pub normal: Option<Handle<Image>>,
    pub atlas: Option<Handle<Image>>,
    pub atlas_rows: u32,
}

impl TreeAssets {
    pub fn ready_models(&self) -> usize {
        self.models.values().filter(|m| m.ready.is_some()).count()
    }
}

/// Starts loading the glb of species `row` (no-op without a `dn_id` or when already started).
pub fn request_model(
    assets: &mut TreeAssets,
    world: &VegetationWorld,
    server: &AssetServer,
    row: u16,
) {
    if assets.models.contains_key(&row) {
        return;
    }
    let dn_id = world.dn_ids.get(row as usize).map_or("", String::as_str);
    if dn_id.is_empty() {
        return;
    }
    let path = format!("models/dn/vegetation/{dn_id}_lod2.glb");
    let present = crate::paths::find_root()
        .map(|root| crate::paths::asset_root(&root).join(&path).exists())
        .unwrap_or(false);
    if !present {
        return;
    }
    assets.models.insert(
        row,
        ModelSlot {
            mesh_src: server.load(
                GltfAssetLabel::Primitive {
                    mesh: 0,
                    primitive: 0,
                }
                .from_asset(path.clone()),
            ),
            material_src: server.load(
                GltfAssetLabel::Material {
                    index: 0,
                    is_scale_inverted: false,
                }
                .from_asset(path),
            ),
            ready: None,
            failed: false,
        },
    );
}

/// Normalises the loaded models: height 1, foot at 0, crown centred, mean crown ratio.
/// Returns true when a model became ready.
pub fn finish_models(
    assets: &mut TreeAssets,
    world: &VegetationWorld,
    server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &Assets<bevy::gltf::GltfMaterial>,
) -> bool {
    let mut changed = false;
    let rows: Vec<u16> = assets.models.keys().copied().collect();
    for row in rows {
        let slot = assets.models.get_mut(&row).expect("slot");
        if slot.ready.is_some() || slot.failed {
            continue;
        }
        if server.load_state(&slot.mesh_src).is_failed()
            || server.load_state(&slot.material_src).is_failed()
        {
            slot.failed = true;
            continue;
        }
        let (Some(source), Some(material)) = (
            meshes.get(&slot.mesh_src),
            materials.get(&slot.material_src),
        ) else {
            continue;
        };
        let Some(texture) = material.base_color_texture.clone() else {
            slot.failed = true;
            continue;
        };
        let ratio = world.width_ratio.get(row as usize).copied().unwrap_or(1.0);
        match normalised(source, ratio) {
            Some(mesh) => {
                slot.ready = Some((meshes.add(mesh), texture));
                changed = true;
            }
            None => slot.failed = true,
        }
    }
    changed
}

fn normalised(source: &Mesh, ratio: f32) -> Option<Mesh> {
    let mut mesh = source.clone();
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    else {
        return None;
    };
    mesh_bounds_and_scale(positions, ratio)?;
    // Foliage shading (`dn_tree_models.gd`): normals rounded away from the crown centre, here
    // lifted towards the sky as well so the side away from the sun is not black (the instanced
    // pipeline has neither the Godot backlight nor ambient occlusion).
    let crown_centre = Vec3::new(0.0, 0.62, 0.0);
    let normals: Vec<[f32; 3]> = positions
        .iter()
        .map(|p| {
            let radial = (Vec3::from(*p) - crown_centre).normalize_or_zero();
            (radial * 0.2 + Vec3::Y).normalize().to_array()
        })
        .collect();
    mesh.attribute(Mesh::ATTRIBUTE_UV_0)?;
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    Some(mesh)
}

/// Rescales `positions` in place; `None` when degenerate.
fn mesh_bounds_and_scale(positions: &mut [[f32; 3]], ratio: f32) -> Option<()> {
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for p in positions.iter() {
        low = low.min(Vec3::from(*p));
        high = high.max(Vec3::from(*p));
    }
    let extent = high - low;
    if extent.y <= 1e-6 {
        return None;
    }
    let crown = extent.x.max(extent.z).max(1e-6);
    let centre = (low + high) * 0.5;
    for p in positions.iter_mut() {
        p[0] = (p[0] - centre.x) / crown * ratio;
        p[2] = (p[2] - centre.z) / crown * ratio;
        p[1] = (p[1] - low.y) / extent.y;
    }
    Some(())
}

/// Unit quad of the impostor batches: the vertex shader only reads the corner UV (0, 0 top left)
/// and builds the camera-facing card itself (`billboard.wesl`).
pub fn quad_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0, 0.0, 0.0]; 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

/// Impostor batch parameters of species `row` (`campaign_tree_impostor.gdshaderinc` uniforms).
pub fn impostor_params(row: u16, rows: u32, ratio: f32, season_class: i32) -> VegParams {
    let mut p = VegParams::new(Mode::Impostor);
    p.tint.w = season_class as f32;
    p.atlas = Vec4::new(ATLAS_VIEWS as f32, rows as f32, row as f32, ratio);
    p.frame.x = ORTHO;
    p.frame.y = FOOT;
    p
}

/// Material of the model batches / of the impostor cards.
pub fn params() -> MaterialParams {
    MaterialParams {
        base_color: [1.15, 1.15, 1.15, 1.0],
        // Both are alpha-tested: the model textures carry cut-out foliage, black where empty.
        alpha_cutoff: 0.5,
        roughness: 0.9,
        metallic: 0.0,
        double_sided: true,
    }
}

/// Records per batch key `(row, model)` of one assembly, with its counters.
#[derive(Default)]
pub struct Assembled {
    pub batches: HashMap<(u16, bool), Vec<u8>>,
    pub near: usize,
    pub far: usize,
    pub thinned: usize,
    pub ms: f32,
    /// Species seen inside the model zone (models to load).
    pub wanted_rows: std::collections::HashSet<u16>,
}

/// Thinning density at camera `distance` (`generalised_thin_start`, `generalised_far_density`).
pub fn density_at(distance: f32) -> f32 {
    (150.0 / distance.max(1.0)).clamp(0.35, 1.0)
}

/// Builds the records of all tiles (worker thread). `model_rows`: species with a ready model.
pub fn assemble(
    tiles: &[Arc<Vec<TreeInst>>],
    focus: Vec2,
    model_radius: f32,
    density: f32,
    model_rows: &[bool],
) -> Assembled {
    let mut out = Assembled::default();
    let threshold = 1.0 - density;
    let grow = 1.0 / density.sqrt();
    let radius2 = model_radius * model_radius;
    for tile in tiles {
        for tree in tile.iter() {
            if tree.seed < threshold {
                out.thinned += 1;
                continue;
            }
            let dx = tree.x - focus.x;
            let dz = tree.z - focus.y;
            let inside = model_radius > 0.0 && dx * dx + dz * dz <= radius2;
            if inside {
                out.wanted_rows.insert(tree.row);
            }
            let near = inside && model_rows.get(tree.row as usize).copied().unwrap_or(false);
            // Thinning grows the remaining far trees; near models keep their size.
            let scale = if near {
                tree.height
            } else {
                tree.height * grow
            };
            let key = (tree.row, near);
            let buffer = out.batches.entry(key).or_default();
            if near {
                for value in [tree.x, tree.y, tree.z, tree.yaw_deg, scale, 0.0] {
                    buffer.extend_from_slice(&value.to_le_bytes());
                }
            } else {
                push_record(
                    buffer,
                    Vec3::new(tree.x, tree.y, tree.z),
                    tree.yaw_deg,
                    scale,
                    tree.seed,
                    [1.0; 3],
                );
            }
            if near {
                out.near += 1;
            } else {
                out.far += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_zone_shrinks_to_nothing_far_away() {
        assert_eq!(model_radius(10.0), MODEL_RADIUS_MIN);
        assert_eq!(model_radius(100.0), MODEL_RADIUS_MAX);
        assert_eq!(model_radius(151.0), 0.0);
    }

    #[test]
    fn density_is_bounded() {
        assert_eq!(density_at(10.0), 1.0);
        assert_eq!(density_at(10_000.0), 0.35);
    }

    #[test]
    fn quad_has_four_corners() {
        let mesh = quad_mesh();
        assert_eq!(mesh.count_vertices(), 4);
        assert_eq!(mesh.indices().unwrap().len(), 6);
        let p = impostor_params(3, 41, 0.8, 2);
        assert_eq!((p.atlas.z, p.atlas.y, p.tint.w), (3.0, 41.0, 2.0));
    }

    #[test]
    fn assembly_splits_models_impostors_and_thins() {
        let tree = |x: f32, seed: f32| TreeInst {
            x,
            z: 0.0,
            y: 0.0,
            yaw_deg: 0.0,
            height: 1.0,
            row: 0,
            seed,
        };
        let tiles = vec![Arc::new(vec![
            tree(1.0, 0.9),
            tree(100.0, 0.9),
            tree(2.0, 0.1),
        ])];
        let out = assemble(&tiles, Vec2::ZERO, 10.0, 0.5, &[true]);
        assert_eq!((out.near, out.far, out.thinned), (1, 1, 1));
        assert_eq!(out.batches[&(0, true)].len(), RECORD_BYTES);
        assert_eq!(
            out.batches[&(0, false)].len(),
            super::super::billboard::RECORD_BYTES
        );
        // Without a ready model the near tree stays an impostor, but its species is wanted.
        let out = assemble(&tiles, Vec2::ZERO, 10.0, 1.0, &[false]);
        assert_eq!((out.near, out.far), (0, 3));
        assert!(out.wanted_rows.contains(&0));
    }
}
