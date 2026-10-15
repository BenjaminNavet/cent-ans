//! Trees and bushes of the battlefield (port of game/scripts/battle/battle_terrain_scatter.gd
//! `_build_trees` and the species of battle_trees.gd).
//!
//! Placement is the Godot one: trees drawn in each wood disc of the simulation (`forests` and
//! `forest_parts`), uniform density from `woodland`, roads and water kept clear, smaller and
//! sparser at the edge, a fringe of bushes, a few bushes scattered over the field. Each species is
//! one trunk mesh and one crown mesh (crossed leaf-spray cards of the species' texture) drawn by
//! Bevy's automatic instancing. Not ported: branching skeleton, three LODs, impostor atlas, the
//! near / far decorative ring of woods, the leaf-sprays winter variant, wind.

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::VisibilityRange;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use data_model::Terrain;
use sim_battle::{BattleSeason, Battlefield, Weather};

use super::hash01;
use super::look::DecorLook;
use super::wind::{WindExtension, WindMaterial, WindParams};
use crate::app::AppState;
use crate::battle::scene::BattleSession;

/// Species parameters (battle_trees.gd `SPECIES`): height, crown width, crown base, trunk height,
/// trunk radius, card size, cards per crown, leaf tint.
struct Species {
    name: &'static str,
    height: f32,
    width: f32,
    crown_base: f32,
    trunk_radius: f32,
    card: f32,
    cards: usize,
    leaf: [f32; 3],
}

const SPECIES: [Species; 7] = [
    Species {
        name: "oak",
        height: 15.0,
        width: 14.5,
        crown_base: 3.6,
        trunk_radius: 0.46,
        card: 3.3,
        cards: 46,
        leaf: [0.9, 0.96, 0.82],
    },
    Species {
        name: "beech",
        height: 19.0,
        width: 12.5,
        crown_base: 4.8,
        trunk_radius: 0.38,
        card: 3.1,
        cards: 44,
        leaf: [1.0, 1.0, 0.8],
    },
    Species {
        name: "ash",
        height: 18.0,
        width: 10.5,
        crown_base: 6.0,
        trunk_radius: 0.32,
        card: 2.6,
        cards: 38,
        leaf: [1.0, 1.06, 0.88],
    },
    Species {
        name: "poplar",
        height: 22.0,
        width: 9.0,
        crown_base: 5.0,
        trunk_radius: 0.42,
        card: 2.8,
        cards: 40,
        leaf: [0.94, 1.0, 0.82],
    },
    Species {
        name: "willow",
        height: 8.5,
        width: 7.5,
        crown_base: 2.6,
        trunk_radius: 0.52,
        card: 2.1,
        cards: 26,
        leaf: [0.95, 1.0, 0.96],
    },
    Species {
        name: "fruit",
        height: 5.6,
        width: 6.2,
        crown_base: 1.8,
        trunk_radius: 0.17,
        card: 2.0,
        cards: 22,
        leaf: [0.95, 1.0, 0.84],
    },
    Species {
        name: "bush",
        height: 2.4,
        width: 3.0,
        crown_base: 0.2,
        trunk_radius: 0.06,
        card: 1.8,
        cards: 14,
        leaf: [0.9, 0.96, 0.82],
    },
];

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

/// Inside the rectangle of the field extended by `margin`, outside the field itself.
fn outside_field(field: &Battlefield, x: f32, z: f32) -> bool {
    x < 0.0 || z < 0.0 || x > field.width as f32 || z > field.depth as f32
}

/// Height of the ground at `(x, z)` beyond the field edge: the edge extended flat (the horizon
/// ring is the same up to 700 m from the field).
pub fn ground_height(field: &Battlefield, x: f32, z: f32) -> f32 {
    field.height(
        f64::from(x.clamp(0.0, field.width as f32)),
        f64::from(z.clamp(0.0, field.depth as f32)),
    ) as f32
}

/// Decorative woods round the field (port of the near ring of battle_terrain_scatter.gd): clumps
/// of trees drawn outside the field up to `reach`, denser than the generated woods inside.
pub fn plan_ring_woods(field: &Battlefield, look: &DecorLook, seed: u64) -> Vec<Planting> {
    let cfg = &look.forest_ring;
    let mut rng = Lcg(seed ^ 0x005e_ed0f_u64);
    let (w, d) = (field.width as f32, field.depth as f32);
    let reach = cfg.reach_m;
    let band_area = (w + 2.0 * reach) * (d + 2.0 * reach) - w * d;
    let clumps = (cfg.clumps_per_km2 * band_area / 1.0e6).round() as usize;
    let mut out = Vec::new();
    for _ in 0..clumps {
        let (cx, cz) = (rng.range(-reach, w + reach), rng.range(-reach, d + reach));
        if !outside_field(field, cx, cz) {
            continue;
        }
        let radius = rng.range(cfg.clump_radius_m[0], cfg.clump_radius_m[1]);
        let count = ((PI * radius * radius / cfg.area_per_tree_m2) as usize).clamp(6, 600);
        for _ in 0..count {
            if out.len() >= cfg.max_trees {
                return out;
            }
            let angle = rng.next() * TAU;
            let dist = rng.next().sqrt() * radius;
            let (x, z) = (cx + angle.cos() * dist, cz + angle.sin() * dist);
            if !outside_field(field, x, z)
                || x < -reach
                || z < -reach
                || x > w + reach
                || z > d + reach
            {
                continue;
            }
            let edge = ((radius - dist) / 14.0).clamp(0.0, 1.0);
            let name = if rng.next() < 0.12 {
                "poplar"
            } else {
                broadleaf_species(field, x, z)
            };
            out.push(Planting {
                species: species_index(name),
                x,
                z,
                yaw: rng.next() * TAU,
                scale: rng.range(0.8, 1.35) * (0.8 + 0.2 * edge),
                tint: (rng.next() * 3.0) as u8 % 3,
                ring: true,
            });
        }
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

fn push_quad(
    mesh: &mut MeshBuffers,
    centre: Vec3,
    right: Vec3,
    up: Vec3,
    normal: Vec3,
    shade: f32,
) {
    let base = mesh.positions.len() as u32;
    for (corner, uv) in [
        (-right - up, [0.0, 1.0]),
        (right - up, [1.0, 1.0]),
        (right + up, [1.0, 0.0]),
        (-right + up, [0.0, 0.0]),
    ] {
        mesh.positions.push((centre + corner).to_array());
        mesh.normals.push(normal.to_array());
        mesh.uvs.push(uv);
        mesh.colors.push([shade, shade, shade, 1.0]);
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[derive(Default)]
struct MeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuffers {
    fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Crown of a species: `cards` leaf-spray quads scattered in the crown ellipsoid, normals rounded
/// outward from its centre, darker towards the core and the bottom (baked shade).
fn crown_mesh(species: &Species) -> Mesh {
    let mut mesh = MeshBuffers::default();
    let half_w = species.width * 0.5;
    let half_h = (species.height - species.crown_base) * 0.5;
    let centre = Vec3::new(0.0, species.crown_base + half_h, 0.0);
    let mut rng = Lcg(species.name.bytes().map(u64::from).sum::<u64>() * 7919 + 1);
    for _ in 0..species.cards {
        // Point in the ellipsoid, biased to the surface.
        let dir = Vec3::new(
            rng.range(-1.0, 1.0),
            rng.range(-1.0, 1.0),
            rng.range(-1.0, 1.0),
        )
        .normalize_or(Vec3::Y);
        let reach = 0.35 + 0.65 * rng.next().sqrt();
        let offset = Vec3::new(dir.x * half_w, dir.y * half_h, dir.z * half_w) * reach;
        let position = centre + offset;
        let outward = dir;
        let spin = rng.next() * TAU;
        let tangent = outward.cross(Vec3::Y).normalize_or(Vec3::X);
        let bitangent = outward.cross(tangent);
        let right = (tangent * spin.cos() + bitangent * spin.sin()) * species.card * 0.5;
        let up = outward.cross(right).normalize_or(Vec3::Y) * species.card * 0.5;
        let shade = (0.55 + 0.45 * reach * (0.6 + 0.4 * (dir.y * 0.5 + 0.5))).clamp(0.0, 1.0);
        let normal = (outward * 0.8 + Vec3::Y * 0.4).normalize();
        push_quad(&mut mesh, position, right, up, normal, shade);
    }
    mesh.into_mesh()
}

/// Trunk: tapered 7-sided prism up into the crown, with a flared foot.
fn trunk_mesh(species: &Species) -> Mesh {
    let mut mesh = MeshBuffers::default();
    let top = species.crown_base + (species.height - species.crown_base) * 0.55;
    let sides = 7u32;
    let rings = [
        (-0.1, species.trunk_radius * 1.25),
        (0.0, species.trunk_radius * 1.1),
        (top * 0.5, species.trunk_radius * 0.85),
        (top, species.trunk_radius * 0.35),
    ];
    for (ring, (y, radius)) in rings.iter().enumerate() {
        for side in 0..=sides {
            let a = side as f32 / sides as f32 * TAU;
            let (s, c) = a.sin_cos();
            mesh.positions.push([c * radius, *y, s * radius]);
            mesh.normals.push([c, 0.15, s]);
            mesh.uvs
                .push([side as f32 / sides as f32 * 2.0, *y / top * 3.0]);
            mesh.colors.push([1.0; 4]);
            if ring > 0 && side < sides {
                let ring_len = sides + 1;
                let here = (ring as u32) * ring_len + side;
                let below = here - ring_len;
                mesh.indices.extend_from_slice(&[
                    below,
                    below + 1,
                    here,
                    below + 1,
                    here + 1,
                    here,
                ]);
            }
        }
    }
    mesh.into_mesh()
}

/// Statistics for the dump.
#[derive(Resource, Debug, Clone, Default)]
pub struct TreeStats {
    pub trees: usize,
    pub bushes: usize,
    pub species: Vec<(String, usize)>,
    pub zones: usize,
    /// Trees of the decorative wood ring (counted in `trees`).
    pub ring_trees: usize,
}

pub struct TreesPlugin;

impl Plugin for TreesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, plant.run_if(resource_added::<BattleSession>))
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<TreeStats>();
            });
    }
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

fn plant(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut wind_materials: ResMut<Assets<WindMaterial>>,
    assets: Res<AssetServer>,
    look: Res<DecorLook>,
) {
    let seed = session.seed;
    let sim = session.sim_mut();
    let field = sim.field();
    let weather = sim.weather();
    let mut plan = plan_plantings(field);
    plan.extend(plan_orchards(field, &look));
    let ring_trees = plan_ring_woods(field, &look, seed);
    let ring_count = ring_trees.len();
    plan.extend(ring_trees);
    let tint = foliage_tint(field.season, weather);
    let bark: Handle<Image> = assets.load("textures/battle/bark_brown_02_diff.jpg");
    let scope = DespawnOnExit(AppState::Battle);
    // Per species: trunk mesh, crown mesh, three crown materials.
    struct Kit {
        trunk: Handle<Mesh>,
        crown: Handle<Mesh>,
        trunk_material: Handle<StandardMaterial>,
        leaves: [Handle<WindMaterial>; 3],
    }
    let bare = matches!(field.season, BattleSeason::Winter);
    let kits: Vec<Kit> = SPECIES
        .iter()
        .map(|species| {
            let leaf_texture: Handle<Image> =
                assets.load(format!("textures/battle/leaf_spray_{}.png", species.name));
            let make = |factor: f32, materials: &mut Assets<WindMaterial>| {
                materials.add(WindMaterial {
                    base: StandardMaterial {
                        base_color: Color::srgb(
                            species.leaf[0] * tint[0] * factor,
                            species.leaf[1] * tint[1] * factor,
                            species.leaf[2] * tint[2] * factor,
                        ),
                        base_color_texture: Some(leaf_texture.clone()),
                        alpha_mode: AlphaMode::Mask(0.5),
                        double_sided: true,
                        cull_mode: None,
                        perceptual_roughness: 0.9,
                        reflectance: 0.05,
                        ..default()
                    },
                    extension: WindExtension {
                        params: WindParams::new(&look.tree_wind, 1.0, species.height),
                        ..default()
                    },
                })
            };
            Kit {
                trunk: meshes.add(trunk_mesh(species)),
                crown: meshes.add(crown_mesh(species)),
                trunk_material: materials.add(StandardMaterial {
                    base_color: Color::srgb(0.7, 0.66, 0.6),
                    base_color_texture: Some(bark.clone()),
                    perceptual_roughness: 1.0,
                    reflectance: 0.03,
                    ..default()
                }),
                leaves: [
                    make(0.92, &mut wind_materials),
                    make(1.0, &mut wind_materials),
                    make(1.08, &mut wind_materials),
                ],
            }
        })
        .collect();
    let ring_range = VisibilityRange {
        start_margin: 0.0..0.0,
        end_margin: look.forest_ring.fade_start_m..look.forest_ring.fade_end_m,
        use_aabb: false,
    };
    let mut counts = vec![0usize; SPECIES.len()];
    for tree in &plan {
        let kit = &kits[tree.species];
        counts[tree.species] += 1;
        let y = ground_height(field, tree.x, tree.z);
        let transform = Transform::from_xyz(tree.x, y - 0.05, tree.z)
            .with_rotation(Quat::from_rotation_y(tree.yaw))
            .with_scale(Vec3::splat(tree.scale));
        // LOD: trunks vanish beyond `trunk_range_m` (the crown stays); ring woods fade out.
        let trunk_range = if tree.ring {
            ring_range.clone()
        } else {
            VisibilityRange {
                start_margin: 0.0..0.0,
                end_margin: (look.forest_ring.trunk_range_m - 30.0)..look.forest_ring.trunk_range_m,
                use_aabb: false,
            }
        };
        commands.spawn((
            Name::new("Tree"),
            Mesh3d(kit.trunk.clone()),
            MeshMaterial3d(kit.trunk_material.clone()),
            transform,
            trunk_range,
            scope.clone(),
        ));
        // Bare crowns in winter: half the cards would be left, drawn smaller and without cast.
        let crown_transform = if bare {
            transform.with_scale(Vec3::splat(tree.scale * 0.8))
        } else {
            transform
        };
        commands.spawn((
            Name::new("TreeCrown"),
            Mesh3d(kit.crown.clone()),
            MeshMaterial3d(kit.leaves[usize::from(tree.tint)].clone()),
            crown_transform,
            NotShadowCaster,
            if tree.ring {
                ring_range.clone()
            } else {
                VisibilityRange::abrupt(0.0, f32::MAX)
            },
            scope.clone(),
        ));
    }
    let bush = species_index("bush");
    commands.insert_resource(TreeStats {
        trees: plan.len() - counts[bush],
        bushes: counts[bush],
        species: SPECIES
            .iter()
            .zip(&counts)
            .map(|(s, n)| (s.name.to_owned(), *n))
            .collect(),
        zones: field.forests.len() + field.forest_parts.len(),
        ring_trees: ring_count,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crown_and_trunk_meshes_are_well_formed() {
        for species in &SPECIES {
            let crown = crown_mesh(species);
            assert_eq!(crown.count_vertices(), species.cards * 4);
            let trunk = trunk_mesh(species);
            assert!(trunk.count_vertices() > 0);
        }
    }

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
