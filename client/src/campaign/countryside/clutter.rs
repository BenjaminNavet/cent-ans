//! Ground clutter: grass tufts and bushes close to the camera on the campaign map (port of
//! `ground_clutter.gd` and `ground_clutter.gdshader`, lot FC3/FC5).
//!
//! Cells of [`CELL_SIZE`] map px around the focus (a whole terrain tile would make hundreds of
//! thousands of tufts) are sown on a worker thread (`seed_cell`: pure and deterministic, the seed
//! is the cell coordinates) over the land cover of the vegetation world (forest and crops masks,
//! land, lakes, sea, rivers, main roads, settlement clearings). A cell is drawn as ONE merged
//! mesh per kind (tufts, bushes): the crossed cards of every tuft are baked into the cell's
//! vertex buffers (positions local to the cell origin, the colour in the vertex colour), so tens
//! of thousands of tufts cost two draw calls per cell. Godot used a `MultiMesh` per cell; the
//! merge is the equivalent without a custom instancing pipeline. The heights are those of the
//! displayed surface (`GroundHeight`), re-seated when the relief exaggeration changes.
//!
//! Real size (grass ~0.6 m, bushes ~1.8 m), drawn only under [`MAX_CAMERA_DISTANCE`].
//!
//! Not ported: the TX T3 plant cards of the biome (needs the ground cards texture array: a
//! tuft stands in), the rank fade by distance and at the disc edge and the wind (the vertex
//! shader of Godot: here the cells simply show or hide), the per-frame `visible_instance_count`
//! (the share of the sown tufts follows the quality level at build time), the TRELLIS rocks
//! between the tufts (the rock outcrops layer places rocks), heath and grass share of the splat
//! (not loaded by the vegetation world: a constant grass share stands in).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde_json::{Value, json};

use super::LoadedCountryside;
use super::placement::Rng;
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::terrain::surface::GroundHeight;
use crate::campaign::vegetation::VegWorld;
use crate::campaign::vegetation::data::VegetationWorld;
use crate::campaign::vegetation::mask::{fbm, smoothstep};
use crate::campaign::vegetation::mips;
use crate::settings::RenderQuality;

pub const GRASS_TEXTURE: &str = "textures/vegetation/campaign_grass_tuft.png";
/// Side (map px) of a sowing cell.
pub const CELL_SIZE: f32 = 2.0;
/// Camera distance beyond which no tuft is drawn.
pub const MAX_CAMERA_DISTANCE: f32 = 2.6;
pub const RADIUS_FACTOR: f32 = 1.3;
pub const MIN_RADIUS: f32 = 1.5;
pub const MAX_RADIUS: f32 = 3.2;
/// Candidates per cell at density 1 over a full cover.
pub const BASE_PER_CELL: usize = 10_000;
/// Density of the Ultra preset: number of candidates sown (x `BASE_PER_CELL`).
pub const MAX_DENSITY: f32 = 1.5;
pub const GRASS_HEIGHT_M: f32 = 0.6;
pub const BUSH_HEIGHT_M: f32 = 1.8;
pub const MAX_CACHED_CELLS: usize = 96;
const MAX_JOBS: usize = 4;
/// Cells meshed (or re-seated) per frame.
const BUILDS_PER_FRAME: usize = 1;
/// Grass share of the splat when the raster is missing (`grass = 0.5` in `_weight`).
const GRASS_SHARE: f32 = 0.55;

/// Density of a quality level (`clutter_density` of `data/fx/render_quality.json`).
pub fn density_of(level: &str) -> f32 {
    match level {
        "low" => 0.0,
        "medium" => 0.5,
        "ultra" => 1.5,
        _ => 1.0,
    }
}

/// One sown tuft or bush.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuft {
    pub pos: Vec2,
    pub yaw: f32,
    /// Height in map px.
    pub height: f32,
    pub bush: bool,
    /// [0, 1), variation of the colour.
    pub tint: f32,
    /// Part of the cover that is dry grass, [0, 1].
    pub dry: f32,
}

/// Seeding context of one cell (all the inputs are read-only).
pub(super) struct Cover<'a> {
    world: &'a VegetationWorld,
    circles: Vec<(Vec2, f32)>,
    lakes: Vec<usize>,
    roads: Vec<(Vec2, Vec2)>,
}

fn segment_distance_sq(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    let t = if len_sq > 1e-12 {
        ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t - p).length_squared()
}

impl<'a> Cover<'a> {
    pub(super) fn new(world: &'a VegetationWorld, low: Vec2, high: Vec2) -> Cover<'a> {
        let pad = Vec2::splat(1.0);
        let (lo, hi) = (low - pad, high + pad);
        let circles = world
            .exclusions
            .iter()
            .filter(|(x, y, r)| {
                let (x, y, r) = (*x as f32, *y as f32, *r as f32);
                x + r >= lo.x && x - r <= hi.x && y + r >= lo.y && y - r <= hi.y
            })
            .map(|(x, y, r)| (Vec2::new(*x as f32, *y as f32), *r as f32))
            .collect();
        let lakes = world
            .clearance
            .lakes
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                l.low.x <= hi.x && l.high.x >= lo.x && l.low.y <= hi.y && l.high.y >= lo.y
            })
            .map(|(i, _)| i)
            .collect();
        let roads = world
            .clearance
            .road_segments
            .iter()
            .filter(|(a, b)| {
                a.min(*b).x <= hi.x
                    && a.max(*b).x >= lo.x
                    && a.min(*b).y <= hi.y
                    && a.max(*b).y >= lo.y
            })
            .copied()
            .collect();
        Cover {
            world,
            circles,
            lakes,
            roads,
        }
    }

    /// Land that is neither water nor a clearing nor a main road.
    pub(super) fn open(&self, p: Vec2) -> bool {
        let w = self.world;
        let (x, y) = (p.x as f64, p.y as f64);
        if w.heights.height_m_at(p.x, p.y) <= 1.0 {
            return false;
        }
        if !w.land.is_empty() && w.land.texel(w.size, x, y, 0) <= 0.5 {
            return false;
        }
        if w.clearance.coast_distance(p.x, p.y) < 0.0 || w.clearance.river_distance(p.x, p.y) < 0.0
        {
            return false;
        }
        if self
            .lakes
            .iter()
            .any(|i| w.clearance.lakes[*i].blocks(p, 0.0))
        {
            return false;
        }
        let road_sq = crate::campaign::vegetation::clearance::ROAD_CLEARANCE.powi(2);
        if self
            .roads
            .iter()
            .any(|(a, b)| segment_distance_sq(p, *a, *b) < road_sq)
        {
            return false;
        }
        !self
            .circles
            .iter()
            .any(|(c, r)| c.distance_squared(p) < r * r)
    }
}

/// Probability of a tuft and bush share at `p` (`_weight`), `forest` the coarse forest cover of
/// the cell.
pub fn weight(forest: f32, crops: f32, heath: f32) -> (f32, f32) {
    let edge = (forest * (1.0 - forest) * 4.0).clamp(0.0, 1.0);
    let grass = GRASS_SHARE;
    let p = ((grass + heath * 0.6) * (1.0 - forest) + crops * 0.12 + edge * 0.7).clamp(0.0, 1.0);
    let bush = (0.04 + edge * 0.4 + heath * 0.25).clamp(0.0, 0.7);
    (p, bush)
}

/// Sowing of one cell, in a random order (the quality level keeps a prefix).
pub fn seed_cell(world: &VegetationWorld, key: IVec2, meters_per_px: f32) -> Vec<Tuft> {
    let low = key.as_vec2() * CELL_SIZE;
    let high = low + Vec2::splat(CELL_SIZE);
    let cover = Cover::new(world, low, high);
    let mut rng = Rng::new(
        (key.x as i64 as u64).wrapping_mul(0x9E37_79B1)
            ^ (key.y as i64 as u64).wrapping_mul(0x85EB_CA77)
            ^ 0xFC3,
    );
    // Coarse forest cover of the cell: 3 x 3 samples.
    let mut forest = [0.0f32; 9];
    let mut crops = [0.0f32; 9];
    for j in 0..3 {
        for i in 0..3 {
            let q = low + (Vec2::new(i as f32, j as f32) + 0.5) * CELL_SIZE / 3.0;
            let s = world.sample(q.x as f64, q.y as f64);
            forest[j * 3 + i] = s.forest;
            crops[j * 3 + i] = s.crops;
        }
    }
    let candidates = (BASE_PER_CELL as f32 * MAX_DENSITY).round() as usize;
    let mut out = Vec::with_capacity(candidates / 3);
    for _ in 0..candidates {
        let p = low + Vec2::new(rng.f32(), rng.f32()) * CELL_SIZE;
        let roll = rng.f32();
        let kind_roll = rng.f32();
        let tint = rng.f32();
        let yaw = rng.f32() * std::f32::consts::TAU;
        let size = rng.range(0.7, 1.3);
        let local = ((p - low) / CELL_SIZE * 3.0)
            .as_ivec2()
            .clamp(IVec2::ZERO, IVec2::splat(2));
        let n = (local.y * 3 + local.x) as usize;
        let (probability, bush_share) = weight(forest[n], crops[n], 0.0);
        if roll >= probability || !cover.open(p) {
            continue;
        }
        let bush = kind_roll < bush_share;
        let metres = size * if bush { BUSH_HEIGHT_M } else { GRASS_HEIGHT_M };
        // Low-frequency patches of drier grass, like the terrain.
        let patch = fbm(p.x, p.y, 91, 1.0 / 7.0, 2, 0.5) * 0.5 + 0.5;
        let dry = (smoothstep(0.45, 0.8, patch) * 0.7 + (tint - 0.5) * 0.25).clamp(0.0, 1.0);
        out.push(Tuft {
            pos: p,
            yaw,
            height: metres / meters_per_px,
            bush,
            tint,
            dry,
        });
    }
    out
}

/// Merged vertex data of the tufts of one kind of a cell (positions relative to `origin`).
#[derive(Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn tufts(&self) -> usize {
        self.positions.len() / 12
    }

    pub fn to_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

const GRASS_COLOR: [f32; 3] = [0.42, 0.52, 0.26];
const GRASS_DRY: [f32; 3] = [0.56, 0.54, 0.32];
const BUSH_COLOR: [f32; 3] = [0.28, 0.37, 0.18];

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Vertex colour (linear) of a tuft: the base colour of its kind, drier in patches, a little
/// lighter or darker per tuft (`fragment()` of the Godot shader without the season).
pub fn tuft_color(t: &Tuft) -> [f32; 4] {
    let base: [f32; 3] = if t.bush {
        BUSH_COLOR
    } else {
        std::array::from_fn(|i| GRASS_COLOR[i] + (GRASS_DRY[i] - GRASS_COLOR[i]) * t.dry)
    };
    let k = 0.9 + (1.12 - 0.9) * t.tint;
    [
        srgb_to_linear(base[0]) * k,
        srgb_to_linear(base[1]) * k,
        srgb_to_linear(base[2]) * k,
        1.0,
    ]
}

/// Bakes the crossed cards (three vertical quads at 60 degrees, `crossed_cards_mesh`) of the
/// tufts of `kind` (`bush`) at the heights `y` (displayed ground) of their foot.
pub fn build_mesh_data(tufts: &[Tuft], heights: &[f32], origin: Vec2, bush: bool) -> MeshData {
    let mut data = MeshData::default();
    for (tuft, y) in tufts.iter().zip(heights) {
        if tuft.bush != bush {
            continue;
        }
        let color = tuft_color(tuft);
        let height = tuft.height;
        // Narrower grass tufts (blades readable from above).
        let width = if bush { height } else { height * 0.7 };
        let (sin_yaw, cos_yaw) = tuft.yaw.sin_cos();
        let foot = Vec3::new(
            tuft.pos.x - origin.x,
            *y - height * 0.08,
            tuft.pos.y - origin.y,
        );
        for k in 0..3 {
            let angle = std::f32::consts::PI * k as f32 / 3.0;
            let side = Vec3::new(angle.cos(), 0.0, angle.sin()) * 0.5 * width;
            let normal = Vec3::new(-angle.sin(), 0.0, angle.cos())
                .lerp(Vec3::Y, 0.6)
                .normalize();
            let rotate = |v: Vec3| {
                Vec3::new(
                    v.x * cos_yaw - v.z * sin_yaw,
                    v.y,
                    v.x * sin_yaw + v.z * cos_yaw,
                )
            };
            let up = Vec3::Y * height;
            let corners = [-side, side, side + up, -side + up];
            let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
            let base = data.positions.len() as u32;
            for (corner, uv) in corners.iter().zip(uvs) {
                let p = foot + rotate(*corner);
                data.positions.push(p.to_array());
                data.normals.push(rotate(normal).to_array());
                data.uvs.push(uv);
                data.colors.push(color);
            }
            data.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    data
}

/// Season multiplier of the material colour (spring tender, summer dry, autumn russet, winter
/// grey-brown; `fragment()` of the Godot shader folded into a factor of the base colour).
pub fn season_tint(season: usize, bush: bool) -> Color {
    let k = if bush { 0.8 } else { 1.0 };
    match season {
        0 => Color::linear_rgb(1.03, 1.12, 0.92),
        1 => Color::linear_rgb(1.05, 1.0, 0.85),
        2 => Color::linear_rgb(1.04 * k, 0.75 * k, 0.68 * k),
        _ => Color::linear_rgb(0.89, 0.71, 0.94),
    }
}

/// A bush card: round mass with a ragged edge (`bush()` of the Godot shader), baked once.
pub fn bush_image() -> Image {
    const N: usize = 128;
    let mut data = vec![0u8; N * N * 4];
    let hash = |x: i32, y: i32| -> f32 {
        let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2C1B_3C6D);
        h ^= h >> 13;
        (h & 0xffff) as f32 / 65535.0
    };
    let noise = |x: f32, y: f32| -> f32 {
        let (ix, iy) = (x.floor(), y.floor());
        let (fx, fy) = (x - ix, y - iy);
        let (u, v) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
        let (ix, iy) = (ix as i32, iy as i32);
        let a = hash(ix, iy) + (hash(ix + 1, iy) - hash(ix, iy)) * u;
        let b = hash(ix, iy + 1) + (hash(ix + 1, iy + 1) - hash(ix, iy + 1)) * u;
        a + (b - a) * v
    };
    for j in 0..N {
        for i in 0..N {
            let (u, v) = (i as f32 / N as f32, j as f32 / N as f32);
            let (px, py) = ((u - 0.5) * 2.0, (v - 0.55) * 2.3);
            let body = 1.0 - (px * px + py * py).sqrt();
            let leaves = noise(u * 11.0, v * 11.0) * 0.55 + noise(u * 23.0, v * 23.0) * 0.3;
            let margin = body + leaves * 0.6 - 0.35;
            let alpha = (0.5 + margin * 6.0).clamp(0.0, 1.0);
            let shade = 0.78 + 0.34 * (1.0 - v) + (noise(u * 40.0, v * 40.0) - 0.5) * 0.25;
            let o = (j * N + i) * 4;
            let g = (shade.clamp(0.0, 1.2) * 215.0) as u8;
            data[o] = g;
            data[o + 1] = g;
            data[o + 2] = g;
            data[o + 3] = (alpha * 255.0) as u8;
        }
    }
    let levels = 6;
    let chain = mips::chain(&data, N, N, levels, true);
    let mut image = Image::new(
        Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(chain);
    image.texture_descriptor.mip_level_count = levels;
    image.sampler = mipmapped_sampler();
    image
}

pub(super) fn mipmapped_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 4,
        ..default()
    })
}

type Slot = Arc<Mutex<Option<(Vec<Tuft>, f32)>>>;

enum Seeding {
    Pending(Slot),
    Sown(Arc<Vec<Tuft>>),
}

struct ClutterCell {
    seeding: Seeding,
    /// One entity (and mesh) per kind once meshed: grass, bushes.
    entities: [Option<(Entity, Handle<Mesh>)>; 2],
    /// Relief version the heights of the meshes were taken at.
    seated: Option<u32>,
    tufts: usize,
    last_seen: u64,
}

#[derive(Resource, Default)]
pub struct Clutter {
    cells: HashMap<IVec2, ClutterCell>,
    materials: Option<[Handle<StandardMaterial>; 2]>,
    bush_texture: Option<Handle<Image>>,
    grass_mips_done: bool,
    pub active: bool,
    pub focus: Vec2,
    pub radius: f32,
    pub density: f32,
    pub season: usize,
    pub relief_version: u32,
    frame: u64,
    pub shown: usize,
    pub sown_cells: usize,
    pub built_cells: usize,
    pub reseated: usize,
    pub evicted: usize,
    pub build_ms_max: f32,
    pub seed_ms_max: f32,
    pub tufts_total: usize,
    pub tufts_shown: usize,
}

/// Camera, quality and season.
pub fn update_view(
    rig: Option<Res<CampaignRig>>,
    quality: Option<Res<RenderQuality>>,
    sim: Res<crate::sim::Sim>,
    relief: Option<Res<super::ReliefVersion>>,
    mut clutter: ResMut<Clutter>,
) {
    if relief.is_some_and(|r| r.is_changed()) {
        clutter.relief_version = clutter.relief_version.wrapping_add(1);
    }
    if let Some(state) = sim.state.as_ref() {
        clutter.season = match state.season {
            sim_campaign::Season::Spring => 0,
            sim_campaign::Season::Summer => 1,
            sim_campaign::Season::Autumn => 2,
            sim_campaign::Season::Winter => 3,
        };
    }
    let density = quality.map_or(1.0, |q| density_of(&q.level));
    if (density - clutter.density).abs() > 1e-4 {
        // The share of the sown tufts is fixed at meshing time: mesh again.
        clutter.density = density;
        clutter.relief_version = clutter.relief_version.wrapping_add(1);
    }
    let Some(rig) = rig else {
        clutter.active = false;
        return;
    };
    // `CENT_ANS_CLUTTER_DEMO=1` (proof hook): drawn at any camera distance, the camera floor of the
    // campaign map (20 map px) keeps the real clutter range (2.6) out of reach.
    let demo = std::env::var_os("CENT_ANS_CLUTTER_DEMO").is_some();
    clutter.active = density > 0.001 && (demo || rig.distance < MAX_CAMERA_DISTANCE);
    clutter.focus = Vec2::new(rig.focus.x, rig.focus.z);
    clutter.radius = (rig.distance * RADIUS_FACTOR).clamp(MIN_RADIUS, MAX_RADIUS);
}

fn cell_rect(key: IVec2) -> Rect {
    let min = key.as_vec2() * CELL_SIZE;
    Rect::from_corners(min, min + Vec2::splat(CELL_SIZE))
}

/// Sows the cells of the disc around the focus (threads), meshes the sown ones (one per frame),
/// shows the cells in the disc and evicts the oldest.
pub fn stream_clutter(
    mut commands: Commands,
    cfg: Option<Res<LoadedCountryside>>,
    world: Option<Res<VegWorld>>,
    ground: GroundHeight,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut clutter: ResMut<Clutter>,
    mut visibilities: Query<&mut Visibility>,
) {
    let (Some(cfg), Some(world)) = (cfg, world) else {
        return;
    };
    clutter.frame += 1;
    ensure_materials(&server, &mut materials, &mut images, &mut clutter);
    if !clutter.active {
        clutter.shown = 0;
        clutter.tufts_shown = 0;
        for cell in clutter.cells.values() {
            for (e, _) in cell.entities.iter().flatten() {
                if let Ok(mut v) = visibilities.get_mut(*e) {
                    v.set_if_neq(Visibility::Hidden);
                }
            }
        }
        return;
    }
    let focus = clutter.focus;
    let radius = clutter.radius;
    let lo = ((focus - Vec2::splat(radius)) / CELL_SIZE)
        .floor()
        .as_ivec2();
    let hi = ((focus + Vec2::splat(radius)) / CELL_SIZE)
        .floor()
        .as_ivec2();
    let mut wanted: Vec<(f32, IVec2)> = Vec::new();
    for cy in lo.y..=hi.y {
        for cx in lo.x..=hi.x {
            let key = IVec2::new(cx, cy);
            let rect = cell_rect(key);
            let nearest = focus.clamp(rect.min, rect.max);
            if nearest.distance_squared(focus) > radius * radius
                || key.x < 0
                || key.y < 0
                || rect.min.x >= cfg.terrain.size.x
                || rect.min.y >= cfg.terrain.size.y
            {
                continue;
            }
            wanted.push((rect.center().distance_squared(focus), key));
        }
    }
    wanted.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Start the sowing of the missing cells.
    let mut jobs = clutter
        .cells
        .values()
        .filter(|c| matches!(c.seeding, Seeding::Pending(_)))
        .count();
    let frame = clutter.frame;
    for (_, key) in &wanted {
        if clutter.cells.contains_key(key) {
            continue;
        }
        if jobs >= MAX_JOBS {
            break;
        }
        let slot: Slot = Arc::default();
        let out = slot.clone();
        let world = world.0.clone();
        let key = *key;
        let mpp = cfg.config.meters_per_px;
        std::thread::spawn(move || {
            let started = std::time::Instant::now();
            let tufts = seed_cell(&world, key, mpp);
            *out.lock().expect("clutter slot") =
                Some((tufts, started.elapsed().as_secs_f32() * 1000.0));
        });
        clutter.cells.insert(
            key,
            ClutterCell {
                seeding: Seeding::Pending(slot),
                entities: [None, None],
                seated: None,
                tufts: 0,
                last_seen: frame,
            },
        );
        jobs += 1;
    }
    // Collect the finished sowings.
    let mut seed_ms = 0.0f32;
    for cell in clutter.cells.values_mut() {
        let done = match &cell.seeding {
            Seeding::Pending(slot) => slot.lock().expect("clutter slot").take(),
            Seeding::Sown(_) => None,
        };
        if let Some((tufts, ms)) = done {
            cell.seeding = Seeding::Sown(Arc::new(tufts));
            seed_ms = seed_ms.max(ms);
        }
    }
    clutter.seed_ms_max = clutter.seed_ms_max.max(seed_ms);
    // Mesh (or re-seat) the nearest cells that need it, a few per frame.
    let relief_version = clutter.relief_version;
    let density = clutter.density;
    let mut budget = BUILDS_PER_FRAME;
    if ground.is_ready() {
        for (_, key) in &wanted {
            if budget == 0 {
                break;
            }
            let Some(cell) = clutter.cells.get(key) else {
                continue;
            };
            let Seeding::Sown(tufts) = &cell.seeding else {
                continue;
            };
            if cell.seated == Some(relief_version) {
                continue;
            }
            let tufts = tufts.clone();
            let started = std::time::Instant::now();
            let share = (density / MAX_DENSITY).clamp(0.0, 1.0);
            let kept = ((tufts.len() as f32) * share) as usize;
            let kept = &tufts[..kept.min(tufts.len())];
            let origin = cell_rect(*key).min;
            let heights: Vec<f32> = kept
                .iter()
                .map(|t| ground.at(t.pos.x, t.pos.y).unwrap_or(0.0))
                .collect();
            let handles = clutter.materials.clone();
            let Some(handles) = handles else {
                continue;
            };
            let cell = clutter.cells.get_mut(key).expect("cell");
            let first = cell.seated.is_none();
            let mut built = 0usize;
            for (kind, bush) in [false, true].into_iter().enumerate() {
                let data = build_mesh_data(kept, &heights, origin, bush);
                let count = data.tufts();
                built += count;
                if count == 0 {
                    // No empty mesh (the renderer warns about it): drop the entity, if any.
                    if let Some((entity, _)) = cell.entities[kind].take() {
                        commands.entity(entity).despawn();
                    }
                    continue;
                }
                let mesh = data.to_mesh();
                if let Some((entity, handle)) = &cell.entities[kind] {
                    meshes.insert(handle, mesh).ok();
                    // The bounds follow the new heights.
                    commands
                        .entity(*entity)
                        .remove::<bevy::camera::primitives::Aabb>();
                } else {
                    let handle = meshes.add(mesh);
                    let entity = commands
                        .spawn((
                            Name::new(if bush {
                                "clutter:bushes"
                            } else {
                                "clutter:tufts"
                            }),
                            Mesh3d(handle.clone()),
                            MeshMaterial3d(handles[kind].clone()),
                            Transform::from_xyz(origin.x, 0.0, origin.y),
                            Visibility::Hidden,
                            bevy::light::NotShadowCaster,
                            DespawnOnExit(AppState::Campaign),
                        ))
                        .id();
                    cell.entities[kind] = Some((entity, handle));
                }
            }
            cell.tufts = built;
            cell.seated = Some(relief_version);
            let ms = started.elapsed().as_secs_f32() * 1000.0;
            clutter.build_ms_max = clutter.build_ms_max.max(ms);
            if first {
                clutter.built_cells += 1;
            } else {
                clutter.reseated += 1;
            }
            budget -= 1;
        }
    }
    // Show the cells of the disc, touch them for the eviction.
    let wanted_set: std::collections::HashSet<IVec2> = wanted.iter().map(|w| w.1).collect();
    let (mut shown, mut tufts_shown) = (0, 0);
    for (key, cell) in clutter.cells.iter_mut() {
        let on = wanted_set.contains(key);
        if on {
            cell.last_seen = frame;
        }
        let visible = on && cell.seated.is_some();
        if visible {
            shown += 1;
            tufts_shown += cell.tufts;
        }
        for (e, _) in cell.entities.iter().flatten() {
            if let Ok(mut v) = visibilities.get_mut(*e) {
                v.set_if_neq(if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
    }
    clutter.shown = shown;
    clutter.tufts_shown = tufts_shown;
    // Evict the oldest cells beyond the cache size.
    while clutter.cells.len() > MAX_CACHED_CELLS {
        let Some((&oldest, _)) = clutter
            .cells
            .iter()
            .filter(|(_, c)| c.last_seen != frame)
            .min_by_key(|(_, c)| c.last_seen)
        else {
            break;
        };
        if let Some(cell) = clutter.cells.remove(&oldest) {
            for (e, _) in cell.entities.into_iter().flatten() {
                commands.entity(e).despawn();
            }
            clutter.evicted += 1;
        }
    }
    clutter.sown_cells = clutter
        .cells
        .values()
        .filter(|c| matches!(c.seeding, Seeding::Sown(_)))
        .count();
    clutter.tufts_total = clutter
        .cells
        .values()
        .map(|c| match &c.seeding {
            Seeding::Sown(t) => t.len(),
            Seeding::Pending(_) => 0,
        })
        .sum();
}

fn ensure_materials(
    server: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    clutter: &mut Clutter,
) {
    if clutter.materials.is_none() {
        let grass = server.load::<Image>(GRASS_TEXTURE);
        let bush = images.add(bush_image());
        clutter.bush_texture = Some(bush.clone());
        let make = |texture: Handle<Image>, bush: bool| StandardMaterial {
            base_color: season_tint(1, bush),
            base_color_texture: Some(texture),
            alpha_mode: AlphaMode::Mask(0.45),
            cull_mode: None,
            double_sided: true,
            perceptual_roughness: 0.95,
            reflectance: 0.05,
            ..default()
        };
        clutter.materials = Some([
            materials.add(make(grass, false)),
            materials.add(make(bush, true)),
        ]);
    }
    // The tuft texture loads without mipmaps: build the chain once it is there.
    if !clutter.grass_mips_done
        && let Some(handles) = &clutter.materials
        && let Some(material) = materials.get(&handles[0])
        && let Some(texture) = material.base_color_texture.clone()
        && let Some(mut image) = images.get_mut(&texture)
    {
        let (w, h) = (image.width() as usize, image.height() as usize);
        let usable = image.texture_descriptor.mip_level_count == 1
            && image.data.as_ref().is_some_and(|d| d.len() == w * h * 4)
            && w % 2 == 0
            && h % 2 == 0;
        if usable {
            let levels = mips::level_count(w.min(h) as u32).max(1) + 2;
            let levels = levels.min((w.min(h) as f32).log2() as u32);
            let base = image.data.take().expect("image data");
            image.data = Some(mips::chain(&base, w, h, levels, true));
            image.texture_descriptor.mip_level_count = levels;
            image.sampler = mipmapped_sampler();
        }
        clutter.grass_mips_done = true;
    }
}

/// Season tint of the two materials.
pub fn apply_season(clutter: Res<Clutter>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let Some(handles) = &clutter.materials else {
        return;
    };
    for (kind, handle) in handles.iter().enumerate() {
        let want = season_tint(clutter.season, kind == 1);
        if let Some(material) = materials.get(handle)
            && material.base_color != want
            && let Some(mut material) = materials.get_mut(handle)
        {
            material.base_color = want;
        }
    }
}

pub fn reset(mut clutter: ResMut<Clutter>) {
    // Meshes and entities go with the campaign state; the materials are kept.
    let materials = clutter.materials.take();
    let bush = clutter.bush_texture.take();
    let mips_done = clutter.grass_mips_done;
    *clutter = Clutter {
        materials,
        bush_texture: bush,
        grass_mips_done: mips_done,
        ..default()
    };
}

pub fn dump(world: &World) -> (String, Value) {
    let c = world.resource::<Clutter>();
    (
        "clutter".into(),
        json!({
            "active": c.active,
            "density": c.density,
            "season": c.season,
            "focus": [c.focus.x, c.focus.y],
            "radius": c.radius,
            "cells_cached": c.cells.len(),
            "cells_sown": c.sown_cells,
            "cells_built": c.built_cells,
            "cells_reseated": c.reseated,
            "cells_shown": c.shown,
            "cells_evicted": c.evicted,
            "tufts_sown": c.tufts_total,
            "tufts_shown": c.tufts_shown,
            "build_ms_max": c.build_ms_max,
            "seed_ms_max": c.seed_ms_max,
            "grass_mips": c.grass_mips_done,
            "understory": super::understory_layer::summary(world),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuft(bush: bool) -> Tuft {
        Tuft {
            pos: Vec2::new(5.0, 7.0),
            yaw: 0.7,
            height: 0.001,
            bush,
            tint: 0.5,
            dry: 0.2,
        }
    }

    #[test]
    fn quality_levels_map_to_the_godot_densities() {
        assert_eq!(density_of("low"), 0.0);
        assert_eq!(density_of("medium"), 0.5);
        assert_eq!(density_of("high"), 1.0);
        assert_eq!(density_of("ultra"), 1.5);
    }

    #[test]
    fn cover_weight_is_highest_on_open_grass_and_edges() {
        let (open, _) = weight(0.0, 0.0, 0.0);
        let (forest, _) = weight(1.0, 0.0, 0.0);
        let (edge, bush_edge) = weight(0.5, 0.0, 0.0);
        let (_, bush_open) = weight(0.0, 0.0, 0.0);
        assert!(open > forest);
        assert!(edge > open * 0.5 && bush_edge > bush_open);
    }

    #[test]
    fn a_tuft_bakes_three_crossed_cards() {
        let data = build_mesh_data(
            &[tuft(false), tuft(true)],
            &[2.0, 2.0],
            Vec2::new(4.0, 6.0),
            false,
        );
        assert_eq!(data.tufts(), 1);
        assert_eq!(data.positions.len(), 12);
        assert_eq!(data.indices.len(), 18);
        assert_eq!(data.colors.len(), 12);
        // Positions are local to the cell origin and follow the ground height.
        let lowest = data.positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        assert!((lowest - (2.0 - 0.001 * 0.08)).abs() < 1e-6);
        assert!(
            data.positions
                .iter()
                .all(|p| p[0].abs() < 2.0 && p[2].abs() < 2.0)
        );
        let bushes = build_mesh_data(&[tuft(false), tuft(true)], &[2.0, 2.0], Vec2::ZERO, true);
        assert_eq!(bushes.tufts(), 1);
    }

    #[test]
    fn indices_stay_inside_the_vertex_buffer() {
        let tufts: Vec<Tuft> = (0..50).map(|i| tuft(i % 7 == 0)).collect();
        let heights = vec![0.0; 50];
        for bush in [false, true] {
            let data = build_mesh_data(&tufts, &heights, Vec2::ZERO, bush);
            let max = data.indices.iter().max().copied().unwrap_or(0) as usize;
            assert!(max < data.positions.len());
        }
    }

    #[test]
    fn bush_card_has_a_full_mip_chain_and_a_ragged_alpha() {
        let image = bush_image();
        assert_eq!(image.texture_descriptor.mip_level_count, 6);
        let data = image.data.as_ref().expect("data");
        let alphas: Vec<u8> = data
            .iter()
            .skip(3)
            .step_by(4)
            .take(128 * 128)
            .copied()
            .collect();
        assert!(alphas.contains(&0));
        assert!(alphas.contains(&255));
    }

    #[test]
    fn season_tint_darkens_the_bush_in_autumn() {
        let (Color::LinearRgba(grass), Color::LinearRgba(bush)) =
            (season_tint(2, false), season_tint(2, true))
        else {
            panic!("linear colours");
        };
        assert!(bush.red < grass.red);
    }
}
