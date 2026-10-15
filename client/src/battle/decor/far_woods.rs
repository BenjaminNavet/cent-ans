//! Far woods of a battle: the tree line on the horizon (port of the far ring of
//! game/scripts/battle/battle_terrain_scatter.gd `_build_trees`, "Bois lointains", drawn by Godot
//! with the impostors of battle_trees.gd).
//!
//! Godot plants a tree every 42 m (jittered ±15 m) from the near ring out to 7 km where the woods
//! noise (`_woods`, 1/420 m, sampled at 0.6 x) passes the biome threshold `far_woods`, lowered
//! where the real relief of the horizon is forest (`_far_woods_at`). Trees are 1.3 to 2 times an
//! oak. Here each tree is two crossed vertical cards of the campaign impostor atlas
//! (`campaign_impostors_albedo.png`, views 0 and 2), merged into one mesh per 2 km tile (recentred
//! on the tile, so frustum culling works), hazed by the distance fog, without shadows.
//! Improvement over Godot: none; the impostor normal map and the per-view choice are not ported.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde::Deserialize;
use sim_battle::Battlefield;

use super::horizon::{RealRelief, relief_weight, ring_height, value_noise};
use super::look::DecorLook;
use crate::app::AppState;

const ATLAS: &str = "textures/vegetation/campaign_impostors_albedo.png";
/// Atlas layout: 8 views per row, rows oak, beech, fir.
const VIEWS: f32 = 8.0;
const ROWS: f32 = 3.0;
/// Bounds of the painted tree inside a cell (u0, v0, u1, v1), measured on the atlas.
const CELL_TREE: [f32; 4] = [0.14, 0.17, 0.86, 0.80];

/// `far_woods` of `battle_decor_look.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct FarWoodsLook {
    pub enabled: bool,
    pub step_m: f32,
    pub jitter_m: f32,
    /// Trees start this far outside the field (Godot: `NEAR_RECT.grow(-60)`).
    pub inner_m: f32,
    /// ... and stop this far (Godot: `FAR_RECT`).
    pub reach_m: f32,
    pub noise_period_m: f32,
    /// Spread of the value noise mapped onto the Godot simplex range.
    pub noise_gain: f32,
    pub scale: [f32; 2],
    /// Oak height (m) the scale multiplies.
    pub tree_height_m: f32,
    pub tile_m: f32,
    pub max_trees: usize,
    /// Trees of the field and of the near ring swap to these cards between these distances
    /// (Godot `IMPOSTOR_DISTANCE` 300 m), tiled finer so the fade follows the camera.
    pub impostor_from_m: [f32; 2],
    pub impostor_tile_m: f32,
    /// Albedo multiplier of the cards (three tints).
    pub tints: [[f32; 3]; 3],
    /// Noise threshold per terrain (Godot `BIOMES[...].far_woods`).
    pub thresholds: HashMap<String, f32>,
}

impl Default for FarWoodsLook {
    fn default() -> Self {
        FarWoodsLook {
            enabled: false,
            step_m: 42.0,
            jitter_m: 15.0,
            inner_m: 840.0,
            reach_m: 7000.0,
            noise_period_m: 700.0,
            noise_gain: 2.4,
            scale: [1.3, 2.0],
            tree_height_m: 14.0,
            tile_m: 2000.0,
            max_trees: 60000,
            impostor_from_m: [300.0, 400.0],
            impostor_tile_m: 160.0,
            tints: [[1.0; 3]; 3],
            thresholds: HashMap::new(),
        }
    }
}

/// One far tree.
#[derive(Debug, Clone, Copy)]
pub struct FarTree {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub height: f32,
    pub yaw: f32,
    /// Atlas row (0 oak, 1 beech, 2 fir).
    pub row: u8,
    pub tint: u8,
    /// Sink of the foot under `y` (the far ring is coarser than the sampled height).
    pub sink: f32,
}

/// Statistics for the dump.
#[derive(Resource, Debug, Clone, Default)]
pub struct FarWoodsStats {
    pub trees: usize,
    pub tiles: usize,
}

fn fbm(x: f32, z: f32) -> f32 {
    (value_noise(x, z)
        + 0.5 * value_noise(x * 2.03, z * 2.03)
        + 0.25 * value_noise(x * 4.1, z * 4.1))
        / 1.75
}

/// Small deterministic generator (same family as the other decor planners).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

fn outside_by(field: &Battlefield, x: f32, z: f32) -> f32 {
    let dx = (-x).max(x - field.width as f32);
    let dz = (-z).max(z - field.depth as f32);
    dx.max(dz)
}

/// Far trees of a field (deterministic for a seed).
pub fn plan(
    field: &Battlefield,
    look: &DecorLook,
    relief: Option<&RealRelief>,
    seed: u64,
) -> Vec<FarTree> {
    let cfg = &look.far_woods;
    if !cfg.enabled {
        return Vec::new();
    }
    let base = cfg
        .thresholds
        .get(field.terrain.key())
        .copied()
        .unwrap_or(0.26);
    let mut rng = Lcg(seed ^ 0x00fa_2000_u64);
    let (w, d) = (field.width as f32, field.depth as f32);
    let mut out = Vec::new();
    let step = cfg.step_m.max(8.0);
    let mut z = -cfg.reach_m;
    while z < d + cfg.reach_m {
        let mut x = -cfg.reach_m;
        while x < w + cfg.reach_m {
            let px = x + rng.range(-cfg.jitter_m, cfg.jitter_m);
            let pz = z + rng.range(-cfg.jitter_m, cfg.jitter_m);
            let (yaw, scale, kind, tint) = (rng.next(), rng.next(), rng.next(), rng.next());
            x += step;
            if outside_by(field, px, pz) < cfg.inner_m {
                continue;
            }
            // Godot `_far_woods_at`: the threshold drops where the real relief is forest.
            let mut threshold = base;
            if let Some(r) = relief {
                let class = r.class_at(px, pz);
                if class == 255 {
                    continue;
                }
                let forest = f32::from(class.min(200)) / 200.0;
                let weight = relief_weight(field, px, pz);
                threshold += (0.55 + (-0.9 - 0.55) * forest - threshold) * weight;
            }
            let p = cfg.noise_period_m.max(1.0);
            let n = (fbm(px / p + 17.3, pz / p - 4.1) - 0.5) * cfg.noise_gain;
            if n < threshold {
                continue;
            }
            let height = cfg.tree_height_m * (cfg.scale[0] + (cfg.scale[1] - cfg.scale[0]) * scale);
            out.push(FarTree {
                x: px,
                y: ring_height(field, look, relief, px, pz),
                z: pz,
                height,
                yaw: yaw * std::f32::consts::TAU,
                row: u8::from(kind > 0.55),
                tint: (tint * 3.0) as u8 % 3,
                sink: 1.5,
            });
        }
        z += step;
    }
    // Over the cap, thin evenly (a truncation would empty the far rows).
    if out.len() > cfg.max_trees && cfg.max_trees > 0 {
        let keep = cfg.max_trees as f64 / out.len() as f64;
        let mut acc = 0.0;
        out.retain(|_| {
            acc += keep;
            if acc >= 1.0 {
                acc -= 1.0;
                true
            } else {
                false
            }
        });
    }
    out
}

#[derive(Default)]
struct Buffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Buffers {
    fn tree(&mut self, tree: &FarTree, tint: [f32; 3]) {
        let width = tree.height * (CELL_TREE[2] - CELL_TREE[0]) / (CELL_TREE[3] - CELL_TREE[1]);
        // Sunk a little: the ring mesh is coarser than the height sampled here.
        let foot = Vec3::new(tree.x, tree.y - tree.sink, tree.z);
        let row = f32::from(tree.row);
        for (card, view) in [(0.0f32, 0.0f32), (1.0, 2.0)] {
            let a = tree.yaw + card * std::f32::consts::FRAC_PI_2;
            let along = Vec3::new(a.cos(), 0.0, a.sin()) * width * 0.5;
            let facing = Vec3::new(-a.sin(), 0.0, a.cos());
            let normal = (Vec3::Y * 0.5 + facing * 0.5).normalize().to_array();
            let (u0, u1) = ((view + CELL_TREE[0]) / VIEWS, (view + CELL_TREE[2]) / VIEWS);
            let (v0, v1) = ((row + CELL_TREE[1]) / ROWS, (row + CELL_TREE[3]) / ROWS);
            let base = self.positions.len() as u32;
            for (corner, uv, shade) in [
                (-along, [u0, v1], 0.7),
                (along, [u1, v1], 0.7),
                (along + Vec3::Y * tree.height, [u1, v0], 1.0),
                (-along + Vec3::Y * tree.height, [u0, v0], 1.0),
            ] {
                self.positions.push((foot + corner).to_array());
                self.normals.push(normal);
                self.uvs.push(uv);
                self.colors
                    .push([tint[0] * shade, tint[1] * shade, tint[2] * shade, 1.0]);
            }
            self.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    fn into_mesh(self, centre: Vec3) -> Mesh {
        let positions: Vec<[f32; 3]> = self
            .positions
            .iter()
            .map(|p| (Vec3::from(*p) - centre).to_array())
            .collect();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Spawns the far woods (one mesh per tile) and returns the statistics.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    look: &DecorLook,
    trees: &[FarTree],
) -> FarWoodsStats {
    spawn_tiles(
        commands,
        meshes,
        materials,
        assets,
        look,
        trees,
        look.far_woods.tile_m,
        None,
    )
}

/// Impostor cards of near trees, shown from `impostor_from_m` on (the full trees fade out over
/// the same band).
pub fn spawn_impostors(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    look: &DecorLook,
    trees: &[FarTree],
) -> FarWoodsStats {
    let [near, far] = look.far_woods.impostor_from_m;
    spawn_tiles(
        commands,
        meshes,
        materials,
        assets,
        look,
        trees,
        look.far_woods.impostor_tile_m,
        Some(bevy::camera::visibility::VisibilityRange {
            start_margin: near..far,
            end_margin: 1.0e6..1.1e6,
            use_aabb: false,
        }),
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_tiles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    look: &DecorLook,
    trees: &[FarTree],
    tile: f32,
    range: Option<bevy::camera::visibility::VisibilityRange>,
) -> FarWoodsStats {
    let cfg = &look.far_woods;
    if trees.is_empty() {
        return FarWoodsStats::default();
    }
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(assets.load(ATLAS)),
        alpha_mode: AlphaMode::Mask(0.5),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 1.0,
        reflectance: 0.02,
        ..default()
    });
    let tile = tile.max(50.0);
    let mut tiles: HashMap<(i32, i32), Buffers> = HashMap::new();
    for tree in trees {
        let key = (
            (tree.x / tile).floor() as i32,
            (tree.z / tile).floor() as i32,
        );
        tiles
            .entry(key)
            .or_default()
            .tree(tree, cfg.tints[usize::from(tree.tint) % 3]);
    }
    let count = tiles.len();
    for (_, buffers) in tiles {
        let (mut low, mut high) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in &buffers.positions {
            low = low.min(Vec3::from(*p));
            high = high.max(Vec3::from(*p));
        }
        let centre = (low + high) * 0.5;
        let mut entity = commands.spawn((
            Name::new("FarWoods"),
            Mesh3d(meshes.add(buffers.into_mesh(centre))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(centre),
            NotShadowCaster,
            NotShadowReceiver,
            DespawnOnExit(AppState::Battle),
        ));
        if let Some(range) = &range {
            entity.insert(range.clone());
        }
    }
    FarWoodsStats {
        trees: trees.len(),
        tiles: count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::scene::setup;
    use data_model::GameData;

    #[test]
    fn far_woods_ring_the_field_outside_the_near_band() {
        let root = crate::paths::find_root().unwrap();
        let dir = crate::paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).unwrap();
        setup::install_battle_rules(&dir);
        let custom = setup::default_custom_battle(&game);
        let sim = setup::build_sim(&game, &dir, &custom, 7).unwrap();
        let look = DecorLook::load();
        let trees = plan(sim.field(), &look, None, 7);
        assert!(trees.len() > 1000, "{} far trees", trees.len());
        assert_eq!(trees.len(), plan(sim.field(), &look, None, 7).len());
        let f = sim.field();
        assert!(
            trees
                .iter()
                .all(|t| outside_by(f, t.x, t.z) >= look.far_woods.inner_m)
        );
        // A tree line, not a forest: well under half of the candidate points.
        let cfg = &look.far_woods;
        let side = |s: f64| s as f32 + 2.0 * cfg.reach_m;
        let candidates = side(f.width) * side(f.depth) / (cfg.step_m * cfg.step_m);
        assert!((trees.len() as f32) < candidates * 0.5);
    }
}
