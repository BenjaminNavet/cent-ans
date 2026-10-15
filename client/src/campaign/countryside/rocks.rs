//! Landscape-scale rock outcrops (port of `rock_outcrops.gd`, ADR 0143): limestone cliffs,
//! granite tors, stratified bars, alpine spires, sandstone, erratic blocks, megaliths, volcano.
//!
//! Catalogue `data/art/rock_outcrops.yaml` (flow-style YAML = JSON once the `#` lines are
//! dropped). Tiles of `tile_units` px are sown on worker threads (`seed_tile`, pure, tested on a
//! synthetic map): candidates on a jittered grid, thinned by a clustering noise, scored per model
//! (biome, altitude window, slope, ridge curvature, heath, above the tree line), each accepted
//! outcrop pulling a group (a band along the contour for cliffs and bars, a tight heap otherwise).
//! Never in water, in a town clearing or on a road.
//!
//! Run time: the rocks of the ready tiles around the focus are staged, nearest
//! first, at most [`MAX_LIVE`], as primitives of the model (`proto`, drawn as GPU instances by
//! Bevy's batching; Godot: one `MultiMesh` per model and tile). Size is the real
//! one, grown with the camera distance up to `far_scale` (`scale_for`), faded out between
//! `fade_start` and `fade_end`. Model files: `models/<model>_lod{0,1,2}.glb`, native long side
//! `model_length_m` metres.
//!
//! Not ported: coast rocks (`coast` entries need `coast_dist.png` and the coast geology), the
//! shader albedo gain and pixel fade, the vertical-follow shear, the visible-triangle cap, the
//! pre-examination of flat tiles (the seeding is cheap enough on a thread), simplex noise (a
//! smooth value noise stands in).

use std::collections::{HashMap, HashSet};
use std::f32::consts::{PI, TAU};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use serde_json::{Value, json};

use super::LoadedCountryside;
use super::placement::{Rng, Terrain};
use super::proto::{Lookup, Prototypes, spawn_prims};
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::terrain::surface::GroundHeight;

/// Live rock roots cap (320 while every rock was a glb scene).
pub const MAX_LIVE: usize = 4000;
const MAX_JOBS: usize = 4;

#[derive(Debug, Clone)]
pub struct Outcrop {
    pub id: String,
    pub model: String,
    pub length_m: f32,
    pub size_m: [f32; 2],
    pub biomes: u32,
    pub min_slope: f32,
    pub min_alt: f32,
    pub max_alt: f32,
    pub ridge: f32,
    pub heath: f32,
    pub density: f32,
    pub group: [i32; 2],
    pub along_contour: bool,
}

#[derive(Debug, Clone)]
pub struct RockRender {
    pub tile_units: f32,
    pub grid_step: f32,
    pub max_probability: f32,
    pub cluster_scale: f32,
    pub cluster_bias: f32,
    pub cluster_threshold: [f32; 2],
    pub forest_shading: f32,
    pub road_clearance: f32,
    pub river_clearance: f32,
    pub near_distance: f32,
    pub far_distance: f32,
    pub far_scale: f32,
    pub lod_distances: [f32; 2],
    pub fade_start: f32,
    pub fade_end: f32,
    pub view_radius_factor: f32,
    pub min_view_radius: f32,
    pub max_view_radius: f32,
    pub max_cached_tiles: usize,
}

#[derive(Debug, Clone)]
pub struct RockConfig {
    pub render: RockRender,
    pub outcrops: Vec<Outcrop>,
    pub meters_per_px: f32,
    pub town_clearance_px: f32,
    pub road_cell_px: f32,
}

fn num(v: &Value, key: &str, default: f32) -> f32 {
    v[key].as_f64().map_or(default, |x| x as f32)
}

fn pair(v: &Value, default: [f32; 2]) -> [f32; 2] {
    match (
        v.get(0).and_then(Value::as_f64),
        v.get(1).and_then(Value::as_f64),
    ) {
        (Some(a), Some(b)) => [a as f32, b as f32],
        _ => default,
    }
}

/// Parses the catalogue text (flow YAML: lines starting with `#` dropped, then JSON).
pub fn parse_text(text: &str) -> Option<Value> {
    let body: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect();
    serde_json::from_str(&body.join("\n")).ok()
}

pub fn parse(
    doc: &Value,
    meters_per_px: f32,
    town_clearance_px: f32,
    road_cell_px: f32,
) -> RockConfig {
    let r = &doc["render"];
    let render = RockRender {
        tile_units: num(r, "tile_units", 128.0),
        grid_step: num(r, "grid_step", 2.0),
        max_probability: num(r, "max_probability", 0.4),
        cluster_scale: num(r, "cluster_scale", 0.035),
        cluster_bias: num(r, "cluster_bias", 0.15),
        cluster_threshold: pair(&r["cluster_threshold"], [0.15, 0.45]),
        forest_shading: num(r, "forest_shading", 0.7),
        road_clearance: num(r, "road_clearance", 1.2),
        river_clearance: num(r, "river_clearance", 1.5),
        near_distance: num(r, "near_distance", 15.0),
        far_distance: num(r, "far_distance", 110.0),
        far_scale: num(r, "far_scale", 2.6),
        lod_distances: pair(&r["lod_distances"], [45.0, 160.0]),
        fade_start: num(r, "fade_start", 950.0),
        fade_end: num(r, "fade_end", 1200.0),
        view_radius_factor: num(r, "view_radius_factor", 2.2),
        min_view_radius: num(r, "min_view_radius", 96.0),
        max_view_radius: num(r, "max_view_radius", 1000.0),
        max_cached_tiles: num(r, "max_cached_tiles", 48.0) as usize,
    };
    let outcrops = doc["outcrops"]
        .as_array()
        .into_iter()
        .flatten()
        // Coast rocks need the coast distance raster and the coast geology: not ported.
        .filter(|e| e.get("coast").is_none() && e["model"].is_string())
        .map(|e| Outcrop {
            id: e["id"].as_str().unwrap_or("").to_string(),
            model: e["model"].as_str().unwrap_or("").to_string(),
            length_m: num(e, "model_length_m", 1.0).max(1e-3),
            size_m: pair(&e["size_m"], [300.0, 600.0]),
            biomes: e["biomes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_u64)
                .fold(0u32, |m, b| m | (1u32 << b.min(31))),
            min_slope: num(e, "min_slope", 0.05),
            min_alt: num(e, "min_altitude_m", 0.0),
            max_alt: num(e, "max_altitude_m", 5000.0),
            ridge: num(e, "ridge", 0.0),
            heath: num(e, "heath", 0.0),
            density: num(e, "density", 1.0),
            group: {
                let g = pair(&e["group"], [0.0, 0.0]);
                [g[0] as i32, g[1] as i32]
            },
            along_contour: e["along_contour"].as_bool().unwrap_or(false),
        })
        .collect();
    RockConfig {
        render,
        outcrops,
        meters_per_px,
        town_clearance_px,
        road_cell_px,
    }
}

// --- Math ---------------------------------------------------------------------------------

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    if (b - a).abs() < 1e-9 {
        return if x < a { 0.0 } else { 1.0 };
    }
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lattice(ix: i32, iy: i32) -> f32 {
    let mut h =
        (ix as u32).wrapping_mul(0x85eb_ca6b) ^ (iy as u32).wrapping_mul(0xc2b2_ae35) ^ 0x4B5;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65535.0 * 2.0 - 1.0
}

/// Smooth value noise in [-1, 1] (two octaves, stands in for the Godot simplex).
fn noise(x: f32, y: f32, frequency: f32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut norm = 0.0;
    let mut f = frequency;
    for _ in 0..2 {
        let (px, py) = (x * f, y * f);
        let (ix, iy) = (px.floor() as i32, py.floor() as i32);
        let (fx, fy) = (px - ix as f32, py - iy as f32);
        let (u, v) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
        let a = lattice(ix, iy) * (1.0 - u) + lattice(ix + 1, iy) * u;
        let b = lattice(ix, iy + 1) * (1.0 - u) + lattice(ix + 1, iy + 1) * u;
        total += (a * (1.0 - v) + b * v) * amplitude;
        norm += amplitude;
        amplitude *= 0.5;
        f *= 2.0;
    }
    total / norm
}

// --- Seeding ------------------------------------------------------------------------------

/// Relief at a point: height, slope (rise over run), ridge curvature (m) and gradient.
#[derive(Debug, Clone, Copy)]
pub struct Probe {
    pub h: f32,
    pub slope: f32,
    pub ridge: f32,
    pub grad: Vec2,
}

pub fn terrain_probe(t: &dyn Terrain, mpp: f32, p: Vec2) -> Probe {
    let h = t.height_m(p);
    let e = t.height_m(p + Vec2::new(2.0, 0.0));
    let w = t.height_m(p - Vec2::new(2.0, 0.0));
    let s = t.height_m(p + Vec2::new(0.0, 2.0));
    let n = t.height_m(p - Vec2::new(0.0, 2.0));
    let grad = Vec2::new(e - w, s - n) / (4.0 * mpp);
    Probe {
        h,
        slope: grad.length(),
        ridge: h - (e + w + s + n) * 0.25,
        grad,
    }
}

fn treeline_m(size_y: f32, y: f32) -> f32 {
    let north = 1.0 - (y / size_y.max(1.0)).clamp(0.0, 1.0);
    let k = smooth(0.55, 0.95, north);
    2100.0 + (900.0 - 2100.0) * k
}

/// Suitability [0, 1] of an outcrop at a probed point.
pub fn suitability(
    t: &dyn Terrain,
    o: &Outcrop,
    probe: Probe,
    biome: i32,
    heath: f32,
    y: f32,
) -> f32 {
    if !t.biome_allows(o.biomes, biome) {
        return 0.0;
    }
    let h = probe.h;
    let window = smooth(o.min_alt - 80.0, o.min_alt + 80.0, h)
        * (1.0 - smooth(o.max_alt - 150.0, o.max_alt + 150.0, h));
    if window <= 0.0 {
        return 0.0;
    }
    let slope_term = smooth(o.min_slope, o.min_slope * 2.5 + 0.05, probe.slope);
    let ridge_term = o.ridge
        * smooth(15.0, 110.0, probe.ridge)
        * smooth(o.min_slope * 0.5, o.min_slope * 1.2, probe.slope);
    let heath_term = o.heath * heath * smooth(0.012, 0.05, probe.slope);
    let line = treeline_m(t.size().y, y);
    let alpine = 0.8
        * smooth(line - 200.0, line + 300.0, h)
        * smooth(o.min_slope * 0.5, o.min_slope * 1.5, probe.slope);
    slope_term
        .max(ridge_term)
        .max(heath_term)
        .max(alpine)
        .clamp(0.0, 1.0)
        * window
        * o.density
}

/// Heath (x) and shading under dense forest (y) at a point.
fn heath_at(t: &dyn Terrain, cfg: &RockConfig, p: Vec2, probe: Probe) -> Vec2 {
    let s = t.splat(p);
    if s == [0.0; 4] {
        return Vec2::new(0.0, 1.0);
    }
    let steep = smooth(0.15, 0.4, probe.slope);
    let shade = 1.0 - cfg.render.forest_shading * smooth(0.3, 0.8, s[2]) * (1.0 - steep);
    Vec2::new(s[3] * (1.0 - s[2] * 0.6), shade)
}

/// One sown outcrop.
#[derive(Debug, Clone, PartialEq)]
pub struct Rock {
    pub outcrop: usize,
    pub pos: Vec2,
    pub yaw: f32,
    /// Size in px (map units).
    pub size: f32,
    pub stretch: f32,
}

fn clear_of_water(t: &dyn Terrain, p: Vec2, clearance: f32) -> bool {
    t.is_land(p) && !t.in_lake(p) && t.river_sd(p) >= clearance
}

fn near_roads(t: &dyn Terrain, cfg: &RockConfig, p: Vec2, reach: f32) -> bool {
    let cell = (p / cfg.road_cell_px).floor().as_ivec2();
    for dy in -1..=1 {
        for dx in -1..=1 {
            for seg in t.roads_in(cell + IVec2::new(dx, dy)) {
                let ab = seg.b - seg.a;
                let k = ((p - seg.a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
                if (seg.a + ab * k).distance(p) < reach {
                    return true;
                }
            }
        }
    }
    false
}

fn place(
    t: &dyn Terrain,
    cfg: &RockConfig,
    out: &mut Vec<Rock>,
    chosen: usize,
    p: Vec2,
    probe: Probe,
    rng: &mut Rng,
) -> bool {
    let o = &cfg.outcrops[chosen];
    let size = (o.size_m[0] + (o.size_m[1] - o.size_m[0]) * rng.f32()) / cfg.meters_per_px;
    let radius = size * 0.5;
    let mut yaw = rng.f32() * TAU;
    let stretch = rng.range(0.85, 1.15);
    if !clear_of_water(t, p, cfg.render.river_clearance) {
        return false;
    }
    if t.near_town(p, cfg.town_clearance_px + radius) {
        return false;
    }
    if near_roads(t, cfg, p, radius + cfg.render.road_clearance) {
        return false;
    }
    if o.along_contour && probe.grad.length() > 1e-4 {
        // Long side (model X) along the contour line, a little play.
        yaw = -probe.grad.x.atan2(probe.grad.y) + PI * 0.5 + (yaw - PI) * 0.08;
    }
    out.push(Rock {
        outcrop: chosen,
        pos: p,
        yaw,
        size,
        stretch,
    });
    true
}

fn place_group(
    t: &dyn Terrain,
    cfg: &RockConfig,
    out: &mut Vec<Rock>,
    rect: Rect,
    chosen: usize,
    p: Vec2,
    parent_score: f32,
    rng: &mut Rng,
) {
    let o = &cfg.outcrops[chosen];
    let count = if o.group[1] > o.group[0] {
        o.group[0] + (rng.f32() * (o.group[1] - o.group[0] + 1) as f32) as i32
    } else {
        o.group[0]
    };
    if count <= 0 {
        return;
    }
    let mean_units = (o.size_m[0] + o.size_m[1]) * 0.5 / cfg.meters_per_px;
    let mut cursor = [p, p];
    for n in 0..count {
        let q = if o.along_contour {
            let side = (n % 2) as usize;
            let from = cursor[side];
            let grad = terrain_probe(t, cfg.meters_per_px, from).grad;
            if grad.length() < 1e-4 {
                break;
            }
            let along = Vec2::new(-grad.y, grad.x).normalize() * if side == 0 { 1.0 } else { -1.0 };
            let q = from
                + along * mean_units * rng.range(0.7, 0.95)
                + grad.normalize() * mean_units * rng.range(-0.15, 0.15);
            cursor[side] = q;
            q
        } else {
            let a = rng.f32() * TAU;
            p + Vec2::new(a.cos(), a.sin()) * mean_units * rng.range(0.6, 1.3)
        };
        if !rect.contains(q) || !t.is_land(q) {
            continue;
        }
        let probe = terrain_probe(t, cfg.meters_per_px, q);
        let heath = heath_at(t, cfg, q, probe).x;
        let score = suitability(t, o, probe, t.biome(q), heath, q.y);
        if score < parent_score * 0.4 {
            continue;
        }
        place(t, cfg, out, chosen, q, probe, rng);
    }
}

/// Sows one tile, deterministic (seed = tile).
pub fn seed_tile(t: &dyn Terrain, cfg: &RockConfig, tile: IVec2) -> Vec<Rock> {
    let mut out = Vec::new();
    if cfg.outcrops.is_empty() {
        return out;
    }
    let side = cfg.render.tile_units;
    let rect = Rect::from_corners(tile.as_vec2() * side, (tile.as_vec2() + Vec2::ONE) * side);
    let mut rng =
        Rng::new(0x4B5 ^ ((tile.x as i64 as u64) << 32) ^ (tile.y as u32 as u64) ^ 0x9E37);
    let step = cfg.render.grid_step.max(0.5);
    let p_max = cfg.render.max_probability;
    let [t0, t1] = cfg.render.cluster_threshold;
    let cells = (side / step).ceil() as i32;
    let mut scores = vec![0.0f32; cfg.outcrops.len()];
    for j in 0..cells {
        for i in 0..cells {
            let p =
                rect.min + (Vec2::new(i as f32, j as f32) + Vec2::new(rng.f32(), rng.f32())) * step;
            let roll = rng.f32();
            let pick = rng.f32();
            let seed_value = rng.next_u64();
            if roll >= p_max || !t.is_land(p) {
                continue;
            }
            let cluster = smooth(
                t0,
                t1,
                noise(p.x, p.y, cfg.render.cluster_scale) + 0.5 * cfg.render.cluster_bias,
            );
            if roll >= p_max * cluster {
                continue;
            }
            let probe = terrain_probe(t, cfg.meters_per_px, p);
            let biome = t.biome(p);
            let heath = heath_at(t, cfg, p, probe);
            let mut best = 0.0f32;
            let mut total = 0.0f32;
            for (m, o) in cfg.outcrops.iter().enumerate() {
                scores[m] = suitability(t, o, probe, biome, heath.x, p.y) * heath.y;
                total += scores[m];
                best = best.max(scores[m]);
            }
            if best <= 0.0 || roll >= p_max * cluster * best {
                continue;
            }
            let mut chosen = 0;
            let mut acc = 0.0;
            for (m, s) in scores.iter().enumerate() {
                acc += s;
                if pick * total <= acc && *s > 0.0 {
                    chosen = m;
                    break;
                }
            }
            let mut local = Rng::new(seed_value);
            if !place(t, cfg, &mut out, chosen, p, probe, &mut local) {
                continue;
            }
            place_group(
                t,
                cfg,
                &mut out,
                rect,
                chosen,
                p,
                scores[chosen],
                &mut local,
            );
        }
    }
    // Shuffled (the budget under the live cap then drops a uniform share, not a corner).
    for k in (1..out.len()).rev() {
        let j = (rng.f32() * (k + 1) as f32) as usize;
        out.swap(k, j.min(k));
    }
    out
}

// --- View ---------------------------------------------------------------------------------

/// Growth of the outcrops with the camera distance (`scale_for`): 1 near, `far_scale` far.
pub fn scale_for(r: &RockRender, d: f32) -> f32 {
    let near = r.near_distance.max(1e-3);
    let far = r.far_distance.max(near * 1.01);
    let t = ((d.max(1e-3) / near).ln() / (far / near).ln()).clamp(0.0, 1.0);
    let s = t * t * (3.0 - 2.0 * t);
    1.0 + (r.far_scale - 1.0) * s
}

pub fn lod_for(r: &RockRender, d: f32) -> usize {
    if d < r.lod_distances[0] {
        0
    } else if d < r.lod_distances[1] {
        1
    } else {
        2
    }
}

pub fn fade_for(r: &RockRender, d: f32) -> f32 {
    1.0 - smooth(r.fade_start, r.fade_end, d)
}

pub enum TileState {
    Pending(Arc<Mutex<Option<Vec<Rock>>>>),
    Ready(Vec<Rock>),
}

#[derive(Resource)]
pub struct RockPlan {
    pub config: Arc<RockConfig>,
    /// Model file per outcrop and level of detail.
    pub files: Arc<Vec<[Option<String>; 3]>>,
}

/// A mesh primitive of a rock model.
#[derive(Component, Clone)]
pub struct RockModel;

#[derive(Component)]
pub struct RockInst {
    pub key: (IVec2, u32),
    pub grounded: bool,
    pub last_relief: u32,
}

#[derive(Resource, Default)]
pub struct RockView {
    pub tiles: HashMap<IVec2, TileState>,
    pub last_seen: HashMap<IVec2, u64>,
    pub live: HashMap<(IVec2, u32), Entity>,
    pub lod: usize,
    pub frame: u64,
    pub relief_version: u32,
    pub opacity: f32,
    pub scale: f32,
    pub sown_tiles: usize,
    pub sown_rocks: usize,
    pub candidates: usize,
    pub spawned: usize,
    pub despawned: usize,
    pub visible: usize,
    pub by_outcrop: HashMap<String, usize>,
}

pub fn build_plan(
    loaded: Option<Res<LoadedCountryside>>,
    existing: Option<Res<RockPlan>>,
    mut commands: Commands,
) {
    if existing.is_some() {
        return;
    }
    let Some(loaded) = loaded else {
        return;
    };
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    let Some(doc) = std::fs::read_to_string(root.join("data/art/rock_outcrops.yaml"))
        .ok()
        .and_then(|t| parse_text(&t))
    else {
        warn!("rocks: art/rock_outcrops.yaml missing or invalid");
        return;
    };
    let cfg = parse(
        &doc,
        loaded.config.meters_per_px,
        loaded.config.render.town_clearance_px,
        loaded.config.render.cell_px,
    );
    let assets = crate::paths::asset_root(&root);
    let files: Vec<[Option<String>; 3]> = cfg
        .outcrops
        .iter()
        .map(|o| {
            let found: Vec<Option<String>> = (0..3)
                .map(|l| {
                    let p = format!("models/{}_lod{l}.glb", o.model);
                    assets.join(&p).exists().then_some(p)
                })
                .collect();
            std::array::from_fn(|l| {
                found[l]
                    .clone()
                    .or_else(|| (0..l).rev().find_map(|k| found[k].clone()))
                    .or_else(|| (l + 1..3).find_map(|k| found[k].clone()))
            })
        })
        .collect();
    info!("rocks: {} outcrops", cfg.outcrops.len());
    commands.insert_resource(RockPlan {
        config: Arc::new(cfg),
        files: Arc::new(files),
    });
}

pub fn stream_rocks(
    mut commands: Commands,
    plan: Res<RockPlan>,
    loaded: Res<LoadedCountryside>,
    rig: Option<Res<CampaignRig>>,
    server: Res<AssetServer>,
    mut protos: ResMut<Prototypes>,
    mut view: ResMut<RockView>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    let Some(rig) = rig else {
        return;
    };
    let cfg = &plan.config;
    let r = &cfg.render;
    let d = rig.distance;
    view.opacity = fade_for(r, d);
    view.scale = scale_for(r, d);
    // Finished jobs.
    let mut sown = 0;
    let mut finished = 0;
    for state in view.tiles.values_mut() {
        let done = match state {
            TileState::Pending(slot) => slot.lock().expect("rock tile slot").take(),
            TileState::Ready(_) => None,
        };
        if let Some(rocks) = done {
            sown += rocks.len();
            finished += 1;
            *state = TileState::Ready(rocks);
        }
    }
    view.sown_tiles += finished;
    view.sown_rocks += sown;
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.25;
    view.frame += 1;
    if view.opacity <= 0.01 {
        let gone: Vec<Entity> = view.live.drain().map(|(_, e)| e).collect();
        view.despawned += gone.len();
        for e in gone {
            commands.entity(e).despawn();
        }
        view.candidates = 0;
        return;
    }
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    let radius = (d * r.view_radius_factor).clamp(r.min_view_radius, r.max_view_radius);
    let side = r.tile_units;
    let lo = ((focus - Vec2::splat(radius)) / side).floor().as_ivec2();
    let hi = ((focus + Vec2::splat(radius)) / side).floor().as_ivec2();
    let frame = view.frame;
    let mut wanted_tiles: Vec<(f32, IVec2)> = Vec::new();
    for ty in lo.y.max(0)..=hi.y {
        for tx in lo.x.max(0)..=hi.x {
            let tile = IVec2::new(tx, ty);
            if tile.x as f32 * side >= loaded.terrain.size.x
                || tile.y as f32 * side >= loaded.terrain.size.y
            {
                continue;
            }
            let rect =
                Rect::from_corners(tile.as_vec2() * side, (tile.as_vec2() + Vec2::ONE) * side);
            let nearest = focus.clamp(rect.min, rect.max);
            let dist = nearest.distance(focus);
            if dist <= radius {
                wanted_tiles.push((dist, tile));
            }
        }
    }
    wanted_tiles.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut jobs = view
        .tiles
        .values()
        .filter(|s| matches!(s, TileState::Pending(_)))
        .count();
    for (_, tile) in &wanted_tiles {
        view.last_seen.insert(*tile, frame);
        if view.tiles.contains_key(tile) || jobs >= MAX_JOBS {
            continue;
        }
        let slot = Arc::new(Mutex::new(None));
        let out = slot.clone();
        let terrain = loaded.terrain.clone();
        let config = plan.config.clone();
        let tile = *tile;
        std::thread::spawn(move || {
            let rocks = seed_tile(&*terrain, &config, tile);
            *out.lock().expect("rock tile slot") = Some(rocks);
        });
        view.tiles.insert(tile, TileState::Pending(slot));
        jobs += 1;
    }
    // Eviction of the oldest tiles beyond the cache.
    if view.tiles.len() > r.max_cached_tiles {
        let mut by_age: Vec<(u64, IVec2)> = view
            .tiles
            .keys()
            .map(|k| (view.last_seen.get(k).copied().unwrap_or(0), *k))
            .collect();
        by_age.sort_by_key(|(age, _)| *age);
        let excess = view.tiles.len() - r.max_cached_tiles;
        for (age, key) in by_age.into_iter().take(excess) {
            if age < frame {
                view.tiles.remove(&key);
                view.last_seen.remove(&key);
            }
        }
    }
    // Candidates of the ready tiles in range, nearest first.
    let lod = lod_for(r, d);
    if lod != view.lod {
        view.lod = lod;
        let gone: Vec<Entity> = view.live.drain().map(|(_, e)| e).collect();
        view.despawned += gone.len();
        for e in gone {
            commands.entity(e).despawn();
        }
    }
    let mut found: Vec<(f32, (IVec2, u32))> = Vec::new();
    for (_, tile) in &wanted_tiles {
        if let Some(TileState::Ready(rocks)) = view.tiles.get(tile) {
            for (i, rock) in rocks.iter().enumerate() {
                let dist = rock.pos.distance(focus);
                if dist <= radius {
                    found.push((dist, (*tile, i as u32)));
                }
            }
        }
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    view.candidates = found.len();
    found.truncate(MAX_LIVE);
    let wanted: HashSet<(IVec2, u32)> = found.iter().map(|f| f.1).collect();
    let stale: Vec<(IVec2, u32)> = view
        .live
        .keys()
        .filter(|k| !wanted.contains(k))
        .copied()
        .collect();
    for k in stale {
        if let Some(e) = view.live.remove(&k) {
            commands.entity(e).despawn();
            view.despawned += 1;
        }
    }
    for (_, key) in found {
        if view.live.contains_key(&key) {
            continue;
        }
        let Some(TileState::Ready(rocks)) = view.tiles.get(&key.0) else {
            continue;
        };
        let rock = rocks[key.1 as usize].clone();
        let Some(path) = plan.files[rock.outcrop][lod].clone() else {
            continue;
        };
        let o = &cfg.outcrops[rock.outcrop];
        let unit = rock.size / o.length_m;
        let Lookup::Ready(proto) = protos.request(&path, &mut commands, &server) else {
            continue;
        };
        let prims = spawn_prims(&mut commands, &proto, Vec3::ZERO, RockModel);
        let root = commands
            .spawn((
                Name::new(format!("rock:{}", o.id)),
                Transform::from_xyz(rock.pos.x, 0.0, rock.pos.y)
                    .with_rotation(Quat::from_rotation_y(rock.yaw))
                    .with_scale(Vec3::new(unit * rock.stretch, unit, unit / rock.stretch)),
                Visibility::Hidden,
                RockInst {
                    key,
                    grounded: false,
                    last_relief: u32::MAX,
                },
                DespawnOnExit(AppState::Campaign),
            ))
            .add_children(&prims)
            .id();
        view.live.insert(key, root);
        view.spawned += 1;
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for key in view.live.keys() {
        if let Some(TileState::Ready(rocks)) = view.tiles.get(&key.0) {
            *counts
                .entry(cfg.outcrops[rocks[key.1 as usize].outcrop].id.clone())
                .or_default() += 1;
        }
    }
    view.by_outcrop = counts;
}

/// Height on the displayed surface, growth and visibility of the live rocks.
pub fn apply_rocks(
    plan: Res<RockPlan>,
    ground: GroundHeight,
    relief: Option<Res<super::ReliefVersion>>,
    mut view: ResMut<RockView>,
    mut query: Query<(&mut RockInst, &mut Transform, &mut Visibility)>,
) {
    if relief.is_some_and(|r| r.is_changed()) {
        view.relief_version = view.relief_version.wrapping_add(1);
    }
    let factor = view.scale * view.opacity;
    let mut visible = 0;
    for (mut inst, mut tr, mut vis) in &mut query {
        let Some(TileState::Ready(rocks)) = view.tiles.get(&inst.key.0) else {
            continue;
        };
        let rock = &rocks[inst.key.1 as usize];
        let o = &plan.config.outcrops[rock.outcrop];
        if (!inst.grounded || inst.last_relief != view.relief_version)
            && let Some(y) = ground.at(rock.pos.x, rock.pos.y)
        {
            tr.translation.y = y;
            inst.grounded = true;
            inst.last_relief = view.relief_version;
        }
        let unit = rock.size / o.length_m * factor.max(1e-5);
        tr.scale = Vec3::new(unit * rock.stretch, unit, unit / rock.stretch);
        let show = inst.grounded && view.opacity > 0.01;
        if show {
            visible += 1;
        }
        let wanted = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != wanted {
            *vis = wanted;
        }
    }
    view.visible = visible;
}

pub fn reset(mut view: ResMut<RockView>) {
    *view = RockView::default();
}

pub fn dump(world: &World) -> (String, Value) {
    let view = world.resource::<RockView>();
    let plan = world.get_resource::<RockPlan>();
    (
        "rocks".into(),
        json!({
            "outcrops": plan.map_or(0, |p| p.config.outcrops.len()),
            "models_resolved": plan.map_or(0, |p| p.files.iter().filter(|f| f.iter().any(Option::is_some)).count()),
            "tiles_cached": view.tiles.len(),
            "tiles_sown": view.sown_tiles,
            "rocks_sown": view.sown_rocks,
            "candidates": view.candidates,
            "live": view.live.len(),
            "spawned": view.spawned,
            "despawned": view.despawned,
            "visible": view.visible,
            "lod": view.lod,
            "scale": view.scale,
            "opacity": view.opacity,
            "by_outcrop": view.by_outcrop,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::countryside::placement::{RoadSeg, Village};

    /// A land with a ridge (a slope band x in 100..130 climbing 1500 m) and flat plains.
    struct Hills;

    impl Terrain for Hills {
        fn size(&self) -> Vec2 {
            Vec2::new(2000.0, 2000.0)
        }
        fn is_land(&self, _: Vec2) -> bool {
            true
        }
        fn river_sd(&self, _: Vec2) -> f32 {
            8.0
        }
        fn in_lake(&self, _: Vec2) -> bool {
            false
        }
        fn height_m(&self, p: Vec2) -> f32 {
            300.0 + ((p.x - 100.0) / 30.0).clamp(0.0, 1.0) * 1500.0
        }
        fn splat(&self, _: Vec2) -> [f32; 4] {
            [0.0, 0.0, 0.0, 0.8]
        }
        fn wet(&self, _: Vec2) -> [f32; 2] {
            [0.0; 2]
        }
        fn biome(&self, _: Vec2) -> i32 {
            2
        }
        fn biome_allows(&self, mask: u32, biome: i32) -> bool {
            (mask >> biome) & 1 == 1
        }
        fn landscape_row(&self, _: Vec2) -> i32 {
            -1
        }
        fn landscape_id_row(&self, _: &str) -> i32 {
            -2
        }
        fn near_town(&self, _: Vec2, _: f32) -> bool {
            false
        }
        fn villages_in(&self, _: IVec2) -> &[Village] {
            &[]
        }
        fn roads_in(&self, _: IVec2) -> &[RoadSeg] {
            &[]
        }
    }

    fn config() -> RockConfig {
        let root = crate::paths::find_root().expect("root");
        let doc =
            parse_text(&std::fs::read_to_string(root.join("data/art/rock_outcrops.yaml")).unwrap())
                .expect("catalogue");
        parse(&doc, 719.0, 0.8, 96.0)
    }

    #[test]
    fn catalogue_parses_without_coast_entries() {
        let c = config();
        assert!(c.outcrops.len() >= 20, "{}", c.outcrops.len());
        assert!(c.outcrops.iter().all(|o| !o.id.starts_with("coast_")));
        assert!(
            c.outcrops
                .iter()
                .any(|o| o.id == "cliff_limestone" && o.along_contour)
        );
        assert_eq!(c.render.tile_units, 256.0);
    }

    #[test]
    fn seeding_is_deterministic_and_stays_on_the_slope() {
        let c = config();
        let a = seed_tile(&Hills, &c, IVec2::new(0, 3));
        let b = seed_tile(&Hills, &c, IVec2::new(0, 3));
        assert_eq!(a, b);
        assert!(!a.is_empty(), "the ridge carries rocks");
        let inside = a
            .iter()
            .filter(|r| r.pos.x > 80.0 && r.pos.x < 160.0)
            .count();
        assert!(inside * 10 >= a.len() * 8, "{inside}/{}", a.len());
        // A plain tile far from the ridge but flat and high carries (almost) nothing.
        let flat = seed_tile(&Hills, &c, IVec2::new(5, 3));
        assert!(
            flat.len() * 20 < a.len().max(20),
            "{} vs {}",
            flat.len(),
            a.len()
        );
    }

    #[test]
    fn towns_keep_rocks_away() {
        struct Town(Hills);
        impl Terrain for Town {
            fn size(&self) -> Vec2 {
                self.0.size()
            }
            fn is_land(&self, p: Vec2) -> bool {
                self.0.is_land(p)
            }
            fn river_sd(&self, p: Vec2) -> f32 {
                self.0.river_sd(p)
            }
            fn in_lake(&self, p: Vec2) -> bool {
                self.0.in_lake(p)
            }
            fn height_m(&self, p: Vec2) -> f32 {
                self.0.height_m(p)
            }
            fn splat(&self, p: Vec2) -> [f32; 4] {
                self.0.splat(p)
            }
            fn wet(&self, p: Vec2) -> [f32; 2] {
                self.0.wet(p)
            }
            fn biome(&self, p: Vec2) -> i32 {
                self.0.biome(p)
            }
            fn biome_allows(&self, m: u32, b: i32) -> bool {
                self.0.biome_allows(m, b)
            }
            fn landscape_row(&self, p: Vec2) -> i32 {
                self.0.landscape_row(p)
            }
            fn landscape_id_row(&self, i: &str) -> i32 {
                self.0.landscape_id_row(i)
            }
            fn near_town(&self, _: Vec2, _: f32) -> bool {
                true
            }
            fn villages_in(&self, c: IVec2) -> &[Village] {
                self.0.villages_in(c)
            }
            fn roads_in(&self, c: IVec2) -> &[RoadSeg] {
                self.0.roads_in(c)
            }
        }
        assert!(seed_tile(&Town(Hills), &config(), IVec2::new(0, 3)).is_empty());
    }

    #[test]
    fn scale_grows_with_distance_and_fades_out() {
        let c = config();
        let r = &c.render;
        assert!((scale_for(r, 1.0) - 1.0).abs() < 1e-4);
        assert!((scale_for(r, 1000.0) - r.far_scale).abs() < 1e-4);
        assert!(scale_for(r, 30.0) > 1.0 && scale_for(r, 30.0) < r.far_scale);
        assert_eq!(fade_for(r, 100.0), 1.0);
        assert_eq!(fade_for(r, 5000.0), 0.0);
        assert_eq!(lod_for(r, 10.0), 0);
        assert_eq!(lod_for(r, 60.0), 1);
        assert_eq!(lod_for(r, 500.0), 2);
    }
}
