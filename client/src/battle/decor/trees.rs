//! Trees and bushes of the battlefield (port of game/scripts/battle/battle_terrain_scatter.gd
//! `_build_trees` and the species of battle_trees.gd).
//!
//! Placement is the Godot one: trees drawn in each wood disc of the simulation (`forests` and
//! `forest_parts`), uniform density from `woodland`, roads and water kept clear, smaller and
//! sparser at the edge, a fringe of bushes, a few bushes scattered over the field, the decorative
//! wood ring and the orchards. Each species is a branched skeleton of battle_trees.gd
//! ([`super::tree_kit`]) at two levels (full, light) drawn by Bevy's automatic instancing, then
//! the atlas impostors of `far_woods.rs`; regional barks and leaves (TX T3), bare twigs in winter,
//! wind sway (`wind.wesl`).

use std::f32::consts::TAU;

use bevy::camera::visibility::VisibilityRange;
use bevy::prelude::*;
use data_model::Terrain;
use sim_battle::{BattleSeason, Battlefield, Weather};

use super::hash01;
use super::look::DecorLook;
use super::wind::{WindExtension, WindMaterial, WindParams};
use crate::app::AppState;
use crate::battle::scene::BattleSession;

use super::tree_kit::TREE_SPECIES as SPECIES;

fn species_index(name: &str) -> usize {
    SPECIES.iter().position(|s| s.name == name).unwrap_or(0)
}

/// Species of a broadleaf tree at `(x, z)` (`_broadleaf_species`).
pub fn broadleaf_species(field: &Battlefield, x: f32, z: f32) -> &'static str {
    let h = hash01(x, z, 0.0);
    let marsh = field.terrain == Terrain::Marsh;
    if marsh {
        return if h < 0.55 {
            "willow"
        } else if h < 0.75 {
            "poplar"
        } else {
            "ash"
        };
    }
    let beech = if matches!(
        field.terrain,
        Terrain::Forest | Terrain::Hills | Terrain::Mountains
    ) {
        0.35
    } else {
        0.2
    };
    let ash = if field.terrain == Terrain::Bocage {
        0.3
    } else {
        0.2
    };
    if h < beech {
        "beech"
    } else if h < beech + ash {
        "ash"
    } else {
        "oak"
    }
}

/// One planted tree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Planting {
    pub species: usize,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub scale: f32,
    /// Tint bucket 0..3.
    pub tint: u8,
    /// Planted in the decorative wood ring outside the field (fades out with the distance).
    pub ring: bool,
}

/// Small deterministic generator (the scatter must not depend on the platform).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / (1u64 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }
}

fn polyline_distance(points: &[(f64, f64)], x: f64, z: f64) -> f64 {
    let mut best = f64::MAX;
    for pair in points.windows(2) {
        let (ax, az) = pair[0];
        let (bx, bz) = pair[1];
        let (dx, dz) = (bx - ax, bz - az);
        let len2 = dx * dx + dz * dz;
        let t = if len2 > 0.0 {
            (((x - ax) * dx + (z - az) * dz) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        best = best.min((x - (ax + dx * t)).hypot(z - (az + dz * t)));
    }
    best
}

fn near_road(field: &Battlefield, x: f32, z: f32, margin: f64) -> bool {
    field.roads.iter().any(|road| {
        polyline_distance(&road.points, f64::from(x), f64::from(z)) < road.width * 0.5 + margin
    })
}

fn in_decor_area(field: &Battlefield, x: f32, z: f32) -> bool {
    field
        .decor
        .areas
        .iter()
        .any(|a| a.contains(f64::from(x), f64::from(z)))
}

/// Distance to the edge of the union of `zones` (negative inside); `f32::MAX` without zones.
fn edge_distance(zones: &[sim_battle::Zone], x: f32, z: f32) -> f32 {
    let mut inside_depth = f32::MIN;
    let mut outside = f32::MAX;
    for zone in zones {
        let d = (x - zone.x as f32).hypot(z - zone.z as f32) - zone.radius as f32;
        if d <= 0.0 {
            inside_depth = inside_depth.max(-d);
        } else {
            outside = outside.min(d);
        }
    }
    if inside_depth > f32::MIN {
        inside_depth
    } else {
        -outside
    }
}

/// Trees and bushes of the field (`woods` = anchors then lobes).
pub fn plan_plantings(field: &Battlefield) -> Vec<Planting> {
    let mut rng = Lcg(1337);
    let mut out = Vec::new();
    let woodland = field.woodland.clamp(0.0, 1.0) as f32;
    let area_per_tree = 85.0 + (36.0 - 85.0) * woodland;
    let zones: Vec<sim_battle::Zone> = field
        .forests
        .iter()
        .chain(field.forest_parts.iter())
        .copied()
        .collect();
    let bush = species_index("bush");
    for (k, zone) in zones.iter().enumerate() {
        let r = zone.radius as f32;
        let count = ((std::f32::consts::PI * r * r / area_per_tree) as usize).clamp(4, 1000);
        for _ in 0..count {
            let angle = rng.next() * TAU;
            let dist = rng.next().sqrt() * r;
            let x = zone.x as f32 + angle.cos() * dist;
            let z = zone.z as f32 + angle.sin() * dist;
            // A disc already covered by a previous one keeps its own density.
            if zones[..k]
                .iter()
                .any(|p| (x - p.x as f32).hypot(z - p.z as f32) <= p.radius as f32)
            {
                continue;
            }
            if near_road(field, x, z, 4.0)
                || field.water_at(f64::from(x), f64::from(z)).is_some()
                || in_decor_area(field, x, z)
            {
                continue;
            }
            let edge = edge_distance(&zones, x, z);
            if edge < 6.0 && rng.next() < 0.35 {
                continue;
            }
            let grow = 0.75 + 0.25 * (edge / 14.0).clamp(0.0, 1.0);
            let name = if rng.next() < 0.15 {
                "poplar"
            } else {
                broadleaf_species(field, x, z)
            };
            out.push(Planting {
                species: species_index(name),
                x,
                z,
                yaw: rng.next() * TAU,
                scale: rng.range(0.75 * grow, 1.3 * grow),
                tint: (rng.next() * 3.0) as u8 % 3,
                ring: false,
            });
        }
        // Fringe of bushes round the union.
        for _ in 0..(TAU * r / 6.0) as usize {
            let angle = rng.next() * TAU;
            let radius = r + rng.range(-2.0, 5.0);
            let x = zone.x as f32 + angle.cos() * radius;
            let z = zone.z as f32 + angle.sin() * radius;
            if edge_distance(&zones, x, z).abs() > 1.0
                || field.water_at(f64::from(x), f64::from(z)).is_some()
                || near_road(field, x, z, 3.0)
            {
                continue;
            }
            out.push(Planting {
                species: bush,
                x,
                z,
                yaw: rng.next() * TAU,
                scale: rng.range(0.6, 1.4),
                tint: (rng.next() * 3.0) as u8 % 3,
                ring: false,
            });
        }
    }
    // A few scattered bushes outside the centre of the field.
    let count = (140.0 * field.width * field.depth / 960_000.0) as usize;
    for _ in 0..count {
        let x = rng.range(0.0, field.width as f32);
        let z = rng.range(0.0, field.depth as f32);
        let centre_x = (x - field.width as f32 * 0.5).abs() < 420.0 * field.width as f32 / 1200.0;
        if centre_x && (z - field.depth as f32 * 0.5).abs() < field.depth as f32 * 0.5 - 120.0 {
            continue;
        }
        if near_road(field, x, z, 6.0)
            || field.water_at(f64::from(x), f64::from(z)).is_some()
            || in_decor_area(field, x, z)
        {
            continue;
        }
        out.push(Planting {
            species: bush,
            x,
            z,
            yaw: rng.next() * TAU,
            scale: rng.range(0.6, 1.3),
            tint: (rng.next() * 3.0) as u8 % 3,
            ring: false,
        });
    }
    out
}

/// Height of the ground at `(x, z)` beyond the field edge: the edge extended flat (the horizon
/// ring is the same up to 700 m from the field).
pub fn ground_height(field: &Battlefield, x: f32, z: f32) -> f32 {
    field.height(
        f64::from(x.clamp(0.0, field.width as f32)),
        f64::from(z.clamp(0.0, field.depth as f32)),
    ) as f32
}

/// Ground under a planting: the field inside it, the horizon ring (real or noise relief) outside
/// when the horizon is shown, so that the ring woods do not sink into the rising ring.
fn planted_height(
    field: &Battlefield,
    look: &DecorLook,
    relief: Option<&super::horizon::RealRelief>,
    x: f32,
    z: f32,
) -> f32 {
    let inside = x >= 0.0 && z >= 0.0 && x <= field.width as f32 && z <= field.depth as f32;
    if inside || !look.horizon.enabled {
        ground_height(field, x, z)
    } else {
        super::horizon::ring_height(field, look, relief, x, z)
    }
}

/// Decorative woods round the field (port of the near ring of battle_terrain_scatter.gd
/// `_build_trees`): a tree every `step_m` (jittered) from `margin_m` outside the field out to
/// `reach_m`, where the woods noise (Godot `_woods`, 1/420 m) passes the terrain's `near_woods`
/// threshold; bushes on the noise edge, a few isolated trees (poplars mostly), roads kept clear.
pub fn plan_ring_woods(field: &Battlefield, look: &DecorLook, seed: u64) -> Vec<Planting> {
    let cfg = &look.forest_ring;
    let mut rng = Lcg(seed ^ 0x005e_ed0f_u64);
    let (w, d) = (field.width as f32, field.depth as f32);
    let reach = cfg.reach_m;
    let threshold = cfg
        .thresholds
        .get(field.terrain.key())
        .copied()
        .unwrap_or(0.3);
    let period = cfg.noise_period_m.max(1.0);
    let noise = |x: f32, z: f32| {
        let v = |x: f32, z: f32| super::horizon::value_noise(x, z);
        let f = (v(x, z) + 0.5 * v(x * 2.03, z * 2.03) + 0.25 * v(x * 4.1, z * 4.1)) / 1.75;
        (f - 0.5) * cfg.noise_gain
    };
    let mut out = Vec::new();
    let step = cfg.step_m.max(4.0);
    let mut z = -reach;
    while z < d + reach {
        let mut x = -reach;
        while x < w + reach {
            let px = x + rng.range(-cfg.jitter_m, cfg.jitter_m);
            let pz = z + rng.range(-cfg.jitter_m, cfg.jitter_m);
            let (isolated, bush, kind, yaw, scale, tint) = (
                rng.next() < cfg.isolated_share,
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
                rng.next(),
            );
            x += step;
            let margin = cfg.margin_m;
            if px > -margin && pz > -margin && px < w + margin && pz < d + margin {
                continue;
            }
            let n = noise(px / period + 3.7, pz / period + 11.2);
            if n < threshold && !isolated {
                continue;
            }
            if near_road(field, px, pz, 9.0)
                || field.water_at(f64::from(px), f64::from(pz)).is_some()
            {
                continue;
            }
            if out.len() >= cfg.max_trees {
                return out;
            }
            let edge = n >= threshold && n < threshold + 0.06 && bush < 0.6;
            let (name, scale) = if edge {
                ("bush", 0.7 + 0.8 * scale)
            } else if kind < if isolated { 0.35 } else { 0.1 } {
                ("poplar", 0.8 + 0.55 * scale)
            } else {
                (broadleaf_species(field, px, pz), 0.8 + 0.55 * scale)
            };
            out.push(Planting {
                species: species_index(name),
                x: px,
                z: pz,
                yaw: yaw * TAU,
                scale,
                tint: (tint * 3.0) as u8 % 3,
                ring: true,
            });
        }
        z += step;
    }
    out
}

/// Fruit trees of the orchard areas: rows 6.5 m apart, odd rows shifted, a few dead trees missing
/// (port of `_plant_orchards` of battle_terrain_scatter.gd).
pub fn plan_orchards(field: &Battlefield, look: &DecorLook) -> Vec<Planting> {
    let cfg = &look.orchard;
    let mut rng = Lcg(6606);
    let mut out = Vec::new();
    for area in field
        .decor
        .areas
        .iter()
        .filter(|a| a.kind == sim_battle::AreaKind::Orchard)
    {
        let hl = area.length as f32 * 0.5 - cfg.margin_m;
        let hw = area.width as f32 * 0.5 - cfg.margin_m;
        let (cos, sin) = (area.yaw.cos() as f32, area.yaw.sin() as f32);
        let mut v = -hw;
        let mut row = 0;
        while v <= hw {
            let mut u = -hl + if row % 2 == 1 { cfg.step_m * 0.5 } else { 0.0 };
            while u <= hl {
                let (ju, jv) = (u + rng.range(-0.6, 0.6), v + rng.range(-0.6, 0.6));
                u += cfg.step_m;
                let x = area.x as f32 + ju * cos - jv * sin;
                let z = area.z as f32 + ju * sin + jv * cos;
                if rng.next() < cfg.dead_share
                    || near_road(field, x, z, 3.0)
                    || field.water_at(f64::from(x), f64::from(z)).is_some()
                {
                    continue;
                }
                out.push(Planting {
                    species: species_index("fruit"),
                    x,
                    z,
                    yaw: rng.next() * TAU,
                    scale: rng.range(0.85, 1.1),
                    tint: (rng.next() * 3.0) as u8 % 3,
                    ring: false,
                });
            }
            v += cfg.step_m;
            row += 1;
        }
    }
    out
}

/// A planted tree as the Codex decor pick sees it (battle trees keep no collider).
#[derive(Debug, Clone, PartialEq)]
pub struct PlantedTree {
    /// Codex species key (`tree:<species>`).
    pub species: &'static str,
    pub foot: Vec3,
    pub height: f32,
    pub radius: f32,
}

/// Every tree of the battle, in world space, for the Codex bubble over the decor.
#[derive(Resource, Debug, Clone, Default)]
pub struct PlantedTrees(pub Vec<PlantedTree>);

/// Statistics for the dump.
#[derive(Resource, Debug, Clone, Default)]
pub struct TreeStats {
    pub trees: usize,
    pub bushes: usize,
    pub species: Vec<(String, usize)>,
    pub zones: usize,
    /// Trees of the decorative wood ring (counted in `trees`).
    pub ring_trees: usize,
    /// Impostor cards of the trees beyond `impostor_from_m`, and their tiles.
    pub impostors: usize,
    pub impostor_tiles: usize,
    /// Bare winter crowns (twigs, or the dead leaves of the oak).
    pub winter: bool,
    /// Entities of each level: full bark, full foliage, light bark, light foliage.
    pub lod_entities: [usize; 4],
    /// Per species: sprays and bark quads of the full mesh, vertices of the full and light
    /// foliage.
    pub meshes: Vec<SpeciesMeshStats>,
    /// Species drawn with the regional bark / leaves packs (TX T3).
    pub tx_bark: Vec<String>,
    pub tx_leaves: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SpeciesMeshStats {
    pub species: String,
    pub clusters: usize,
    pub bark_quads: usize,
    pub leaf_vertices: [usize; 2],
    pub bark_vertices: [usize; 2],
}

pub struct TreesPlugin;

impl Plugin for TreesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<super::tree_kit::TreeTextureStats>()
            .add_systems(
                Update,
                (
                    plant
                        .after(super::horizon::build_horizon)
                        .run_if(resource_added::<BattleSession>),
                    super::tree_kit::prepare_tree_textures
                        .run_if(resource_exists::<super::tree_kit::PendingTreeTextures>),
                ),
            )
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<TreeStats>();
                commands.remove_resource::<PlantedTrees>();
                commands.remove_resource::<super::tree_kit::PendingTreeTextures>();
                commands.remove_resource::<super::tree_kit::TreeMaterials>();
                commands.insert_resource(super::tree_kit::TreeTextureStats::default());
            });
        crate::agent::dump::DumpAppExt::register_dump(app, dump_trees);
    }
}

fn dump_trees(world: &World) -> (String, serde_json::Value) {
    let textures = world.get_resource::<super::tree_kit::TreeTextureStats>();
    (
        "battle_trees".to_owned(),
        serde_json::json!({
            "stats": world.get_resource::<TreeStats>().map(|s| serde_json::json!({
                "trees": s.trees, "bushes": s.bushes, "winter": s.winter,
                "lod_entities": {"full_bark": s.lod_entities[0], "full_leaves": s.lod_entities[1],
                    "light_bark": s.lod_entities[2], "light_leaves": s.lod_entities[3],
                    "impostors": s.impostors, "impostor_tiles": s.impostor_tiles},
                "meshes": s.meshes, "tx_bark": s.tx_bark, "tx_leaves": s.tx_leaves,
            })),
            "textures_prepared": textures.map(|t| t.prepared),
            "textures_failed": textures.map(|t| t.failed),
            "textures_pending": world
                .get_resource::<super::tree_kit::PendingTreeTextures>()
                .map_or(0, |p| p.0.len()),
        }),
    )
}

/// Foliage tint of a season and weather (multiplies the leaf texture).
fn foliage_tint(season: BattleSeason, weather: Weather) -> [f32; 3] {
    let base = match season {
        BattleSeason::Spring => [0.95, 1.05, 0.85],
        BattleSeason::Summer => [0.85, 0.95, 0.75],
        BattleSeason::Autumn => [1.35, 0.95, 0.5],
        BattleSeason::Winter => [0.9, 0.8, 0.6],
    };
    if matches!(weather, Weather::Rain | Weather::Fog) {
        [base[0] * 0.92, base[1] * 0.95, base[2] * 0.95]
    } else {
        base
    }
}

/// Meshes and materials of one species: per level (full, light) a bark and a foliage.
struct Kit {
    bark: [Option<Handle<Mesh>>; 2],
    leaves: [Handle<Mesh>; 2],
    bark_material: Handle<StandardMaterial>,
    /// Three tint buckets of the foliage.
    leaf_materials: [Handle<WindMaterial>; 3],
}

/// Desaturates a tint like the foliage shader does its colour.
fn muted(c: [f32; 3]) -> [f32; 3] {
    let l = c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11;
    c.map(|v| l + (v - l) * super::tree_kit::DECOR_SATURATION)
}

#[allow(clippy::too_many_arguments)]
fn make_kits(
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    wind_materials: &mut Assets<WindMaterial>,
    assets: &AssetServer,
    look: &DecorLook,
    tint: [f32; 3],
    winter: bool,
    stats: &mut TreeStats,
) -> (Vec<Kit>, Vec<super::tree_kit::PendingTreeTexture>) {
    use super::tree_kit::{PendingTreeTexture, TexturePrep, build_tree, placeholder};
    use crate::campaign::vegetation::textures::{Quality, TreeTextures};
    let textures = crate::paths::find_root()
        .ok()
        .map(|root| {
            TreeTextures::load(
                &crate::paths::data_dir(&root),
                &crate::paths::asset_root(&root),
                Quality::Medium,
                true,
            )
        })
        .unwrap_or_default();
    let grid = |pack: &crate::campaign::vegetation::textures::Pack| {
        (pack.grid.0.max(1), pack.grid.1.max(1))
    };
    let mut jobs = Vec::new();
    let micro_bark: Option<Handle<Image>> = crate::paths::find_root().ok().and_then(|root| {
        let path = "textures/vegetation/tx_micro_bark.jpg";
        crate::paths::asset_root(&root)
            .join(path)
            .exists()
            .then(|| assets.load(path))
    });
    let mut job = |source: Handle<Image>, grid: (u32, u32), layer: u32, prep: TexturePrep| {
        let srgb = !matches!(prep, TexturePrep::BarkNormal { .. });
        let target = images.add(placeholder(srgb));
        jobs.push(PendingTreeTexture {
            source,
            target: target.clone(),
            grid,
            layer,
            prep,
            // Only the regional bark packs get the micro grain (`TreeTextures.apply_bark`).
            micro: (matches!(prep, TexturePrep::Bark { .. }) && grid != (1, 1))
                .then(|| micro_bark.clone())
                .flatten(),
        });
        target
    };
    let linear = |path: String| -> Handle<Image> {
        assets
            .load_builder()
            .with_settings(|s: &mut bevy::image::ImageLoaderSettings| s.is_srgb = false)
            .load(path)
    };
    let legacy_bark: Handle<Image> = assets.load("textures/battle/bark_brown_02_diff.jpg");
    let legacy_normal = linear("textures/battle/bark_brown_02_nor.jpg".to_owned());
    let bark_pack: Option<Handle<Image>> = textures.bark.albedo.clone().map(|p| assets.load(p));
    let bark_normal_pack = textures.bark.normal.clone().map(linear);
    let leaves_pack: Option<Handle<Image>> = textures.leaves.albedo.clone().map(|p| assets.load(p));
    let kits = SPECIES
        .iter()
        .map(|species| {
            let smooth = species.smooth;
            let bark_layer = textures.bark_layer(species.name);
            let (bark_albedo, bark_normal, bark_tint) = match (&bark_pack, bark_layer) {
                (Some(pack), layer) if layer >= 0 => {
                    stats.tx_bark.push(species.name.to_owned());
                    let g = grid(&textures.bark);
                    let normal = bark_normal_pack
                        .clone()
                        .map(|n| job(n, g, layer as u32, TexturePrep::BarkNormal { smooth }));
                    // TX T3: the generated bark has its own colour; the species tint only shades it.
                    (
                        job(pack.clone(), g, layer as u32, TexturePrep::Bark { smooth }),
                        normal,
                        species.bark.map(|c| c + (1.0 - c) * 0.5),
                    )
                }
                _ => (
                    job(legacy_bark.clone(), (1, 1), 0, TexturePrep::Bark { smooth }),
                    Some(job(
                        legacy_normal.clone(),
                        (1, 1),
                        0,
                        TexturePrep::BarkNormal { smooth },
                    )),
                    species.bark,
                ),
            };
            let leaves_layer = textures.leaves_layer(species.name);
            let mut leaf_tint = species.leaf;
            let leaf_texture = if winter {
                leaf_tint = [1.0; 3];
                let path = if species.marcescent {
                    "textures/battle/dead_leaves_oak.png"
                } else {
                    "textures/battle/twig_spray.png"
                };
                job(assets.load(path), (1, 1), 0, TexturePrep::Leaves)
            } else if let (Some(pack), true) = (&leaves_pack, leaves_layer >= 0) {
                stats.tx_leaves.push(species.name.to_owned());
                // TX T3: generated leaves already carry their colour.
                leaf_tint = leaf_tint.map(|c| c + (1.0 - c) * 0.6);
                job(
                    pack.clone(),
                    grid(&textures.leaves),
                    leaves_layer as u32,
                    TexturePrep::Leaves,
                )
            } else {
                let path = format!("textures/battle/leaf_spray_{}.png", species.name);
                job(assets.load(path), (1, 1), 0, TexturePrep::Leaves)
            };
            let full = build_tree(species, 0);
            let light = build_tree(species, 1);
            let vertices = |m: &Option<Mesh>| m.as_ref().map_or(0, Mesh::count_vertices);
            stats.meshes.push(SpeciesMeshStats {
                species: species.name.to_owned(),
                clusters: full.clusters,
                bark_quads: full.bark_quads,
                leaf_vertices: [full.leaves.count_vertices(), light.leaves.count_vertices()],
                bark_vertices: [vertices(&full.bark), vertices(&light.bark)],
            });
            let bark_material = materials.add(StandardMaterial {
                base_color: Color::srgb(bark_tint[0], bark_tint[1], bark_tint[2]),
                base_color_texture: Some(bark_albedo),
                normal_map_texture: bark_normal,
                perceptual_roughness: 0.92,
                reflectance: 0.3,
                ..default()
            });
            let mut wind = WindParams::new(&look.tree_wind, 1.0, (species.height * 0.8).max(2.0));
            if winter {
                wind.sway.x *= 0.5;
                wind.sway.z *= 0.5;
            }
            let make = |factor: f32, wind_materials: &mut Assets<WindMaterial>| {
                let c = muted([
                    leaf_tint[0] * tint[0] * factor,
                    leaf_tint[1] * tint[1] * factor,
                    leaf_tint[2] * tint[2] * factor,
                ]);
                wind_materials.add(WindMaterial {
                    base: StandardMaterial {
                        base_color: Color::srgb(c[0], c[1], c[2]),
                        base_color_texture: Some(leaf_texture.clone()),
                        alpha_mode: AlphaMode::Mask(0.5),
                        double_sided: true,
                        cull_mode: None,
                        perceptual_roughness: 0.82,
                        reflectance: 0.22,
                        diffuse_transmission: if winter { 0.06 } else { 0.25 },
                        ..default()
                    },
                    extension: WindExtension {
                        params: wind,
                        ..default()
                    },
                })
            };
            Kit {
                bark: [
                    full.bark.map(|m| meshes.add(m)),
                    light.bark.map(|m| meshes.add(m)),
                ],
                leaves: [meshes.add(full.leaves), meshes.add(light.leaves)],
                bark_material,
                leaf_materials: [
                    make(0.92, wind_materials),
                    make(1.0, wind_materials),
                    make(1.08, wind_materials),
                ],
            }
        })
        .collect();
    (kits, jobs)
}
#[allow(clippy::too_many_arguments)]
fn plant(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut wind_materials: ResMut<Assets<WindMaterial>>,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    look: Res<DecorLook>,
    relief: Option<Res<super::horizon::RealRelief>>,
) {
    use super::tree_kit::{BUSH_LOD1_DISTANCE, LOD_BAND, LOD1_DISTANCE};
    let seed = session.seed;
    let sim = session.sim_mut();
    let field = sim.field();
    let weather = sim.weather();
    let relief = relief.as_deref();
    let mut plan = plan_plantings(field);
    plan.extend(plan_orchards(field, &look));
    let ring_trees = plan_ring_woods(field, &look, seed);
    let ring_count = ring_trees.len();
    plan.extend(ring_trees);
    let tint = foliage_tint(field.season, weather);
    let winter = matches!(field.season, BattleSeason::Winter);
    let scope = DespawnOnExit(AppState::Battle);
    let mut stats = TreeStats {
        winter,
        ..default()
    };
    let (kits, jobs) = make_kits(
        &mut images,
        &mut meshes,
        &mut materials,
        &mut wind_materials,
        &assets,
        &look,
        tint,
        winter,
        &mut stats,
    );
    commands.insert_resource(super::tree_kit::PendingTreeTextures(jobs));
    commands.insert_resource(super::tree_kit::TreeMaterials {
        wind: kits
            .iter()
            .flat_map(|kit| kit.leaf_materials.iter().cloned())
            .collect(),
        standard: kits.iter().map(|kit| kit.bark_material.clone()).collect(),
    });
    let mut counts = vec![0usize; SPECIES.len()];
    // Three levels: full mesh, light mesh, then the atlas impostors of `far_woods` from
    // `impostor_from_m` on (dithered cross-fades of `LOD_BAND` metres).
    let [imp_near, imp_far] = look.far_woods.impostor_from_m;
    let mut impostors = Vec::new();
    let mut planted = Vec::with_capacity(plan.len());
    for tree in &plan {
        let kit = &kits[tree.species];
        counts[tree.species] += 1;
        let species = &SPECIES[tree.species];
        let bush = species.trunk <= 0.0;
        let y = planted_height(field, &look, relief, tree.x, tree.z);
        let transform = Transform::from_xyz(tree.x, y - 0.05, tree.z)
            .with_rotation(Quat::from_rotation_y(tree.yaw))
            .with_scale(Vec3::splat(tree.scale));
        planted.push(PlantedTree {
            species: species.name,
            foot: transform.translation,
            height: species.height * tree.scale,
            radius: species.width * 0.5 * tree.scale,
        });
        let switch = if bush {
            BUSH_LOD1_DISTANCE
        } else {
            LOD1_DISTANCE
        };
        let band = (switch - LOD_BAND * 0.5)..(switch + LOD_BAND * 0.5);
        let ranges = [
            VisibilityRange {
                start_margin: 0.0..0.0,
                end_margin: band.clone(),
                use_aabb: false,
            },
            VisibilityRange {
                start_margin: band,
                end_margin: imp_near..imp_far,
                use_aabb: false,
            },
        ];
        for (level, range) in ranges.into_iter().enumerate() {
            if let Some(bark) = &kit.bark[level] {
                commands.spawn((
                    Name::new("TreeBark"),
                    Mesh3d(bark.clone()),
                    MeshMaterial3d(kit.bark_material.clone()),
                    transform,
                    range.clone(),
                    scope.clone(),
                ));
                stats.lod_entities[level * 2] += 1;
            }
            commands.spawn((
                Name::new("TreeCrown"),
                Mesh3d(kit.leaves[level].clone()),
                MeshMaterial3d(kit.leaf_materials[usize::from(tree.tint)].clone()),
                transform,
                range,
                scope.clone(),
            ));
            stats.lod_entities[level * 2 + 1] += 1;
        }
        if !bush {
            impostors.push(super::far_woods::FarTree {
                x: tree.x,
                y: y - 0.05,
                z: tree.z,
                height: species.height * tree.scale,
                yaw: tree.yaw,
                row: u8::from(species.name == "beech" || species.name == "ash"),
                tint: tree.tint,
                sink: 0.0,
            });
        }
    }
    let impostor_stats = super::far_woods::spawn_impostors(
        &mut commands,
        &mut meshes,
        &mut materials,
        &assets,
        &look,
        &impostors,
    );
    let bush = species_index("bush");
    stats.trees = plan.len() - counts[bush];
    stats.bushes = counts[bush];
    stats.species = SPECIES
        .iter()
        .zip(&counts)
        .map(|(s, n)| (s.name.to_owned(), *n))
        .collect();
    stats.zones = field.forests.len() + field.forest_parts.len();
    stats.ring_trees = ring_count;
    stats.impostors = impostor_stats.trees;
    stats.impostor_tiles = impostor_stats.tiles;
    commands.insert_resource(stats);
    commands.insert_resource(PlantedTrees(planted));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcg_is_deterministic_and_in_unit_range() {
        let (mut a, mut b) = (Lcg(1337), Lcg(1337));
        for _ in 0..100 {
            let v = a.next();
            assert_eq!(v, b.next());
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn plan_stays_in_the_field_and_off_the_water() {
        use crate::battle::scene::setup;
        use crate::paths;
        use data_model::GameData;
        let root = paths::find_root().unwrap();
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).unwrap();
        setup::install_battle_rules(&dir);
        let custom = setup::default_custom_battle(&game);
        let sim = setup::build_sim(&game, &dir, &custom, 7).unwrap();
        let field = sim.field();
        let plan = plan_plantings(field);
        for tree in &plan {
            assert!(
                field
                    .water_at(f64::from(tree.x), f64::from(tree.z))
                    .is_none()
            );
        }
        assert_eq!(plan, plan_plantings(field), "deterministic");
    }
}
