//! Understory and ground tufts of the campaign map (lot TS-T7, Total War look): the pure part, i.e.
//! the settings (`data/fx/understory.json`), the sowing of a cell and the merged vertex data.
//! The streaming around the camera is `understory_layer.rs`.
//!
//! Two levels of square cells sown on worker threads, deterministic per cell:
//! - `far`: the belt of undergrowth along the forest edges (a band of `belt.width_px` on the
//!   meadow side: tall grass, ferns, bushes, deadwood; the density falls towards the meadow) and
//!   the pale tufts (straw / frost) dotted over the snow and the tundra;
//! - `near`: tall russet tufts and small flowers on the open ground, close to the camera.
//!
//! Nothing on water, roads, settlement clearings or cultivated fields (`clutter::Cover::open`
//! and the crops mask). Purely visual, no game rule.

use bevy::prelude::*;
use serde_json::Value;

use super::clutter::MeshData;
use super::placement::Rng;

/// The categories (one material and one merged mesh per cell each), in the order of the arrays.
pub const CATEGORIES: [&str; CATEGORY_COUNT] = [
    "belt_tall",
    "belt_fern",
    "belt_bush",
    "belt_deadwood",
    "pale",
    "near_tall",
    "near_flowers",
];
pub const CATEGORY_COUNT: usize = 7;
pub const BELT_TALL: u8 = 0;
pub const BELT_FERN: u8 = 1;
pub const BELT_BUSH: u8 = 2;
pub const BELT_DEADWOOD: u8 = 3;
pub const PALE: u8 = 4;
pub const NEAR_TALL: u8 = 5;
pub const NEAR_FLOWERS: u8 = 6;
/// Categories of each level.
pub const FAR_CATEGORIES: std::ops::Range<usize> = 0..5;
pub const NEAR_CATEGORIES: std::ops::Range<usize> = 5..7;

/// Settings of a level of cells.
#[derive(Clone, Debug)]
pub struct Level {
    pub cell_px: f32,
    pub max_distance: f32,
    /// Fraction of `max_distance` where the fade-out starts.
    pub fade_start: f32,
    pub radius_factor: f32,
    pub min_radius: f32,
    pub max_radius: f32,
    pub max_cells: usize,
}

impl Level {
    /// Opacity factor (1 full, 0 hidden) at the camera distance `d`.
    pub fn fade(&self, d: f32) -> f32 {
        let start = self.max_distance * self.fade_start;
        1.0 - ((d - start) / (self.max_distance - start).max(1e-3)).clamp(0.0, 1.0)
    }

    /// Radius (map px) of the disc of cells around the focus.
    pub fn radius(&self, d: f32) -> f32 {
        (d * self.radius_factor).clamp(self.min_radius, self.max_radius)
    }
}

#[derive(Clone, Debug)]
pub struct Near {
    pub spacing_px: f32,
    pub forest_max: f32,
    pub crops_max: f32,
    pub snow_max: f32,
    pub flower_patch_frequency: f32,
    pub flower_patch_threshold: f32,
    pub flower_share_in_patch: f32,
    pub flower_share_outside: f32,
}

#[derive(Clone, Debug)]
pub struct Belt {
    pub width_px: f32,
    pub grid_px: f32,
    pub spacing_px: f32,
    pub forest_min: f32,
    pub inside_max: f32,
    pub crops_max: f32,
    pub falloff: f32,
    /// Share of each belt category (`belt_tall` .. `belt_deadwood`), cumulative.
    pub cumulative: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct Pale {
    pub spacing_px: f32,
    pub snow_min: f32,
    pub crops_max: f32,
    pub forest_max: f32,
    pub tundra_start_m: f32,
    pub tundra_end_m: f32,
    pub tundra_share: f32,
    pub patch_frequency: f32,
}

/// Look of a category.
#[derive(Clone, Debug)]
pub struct Kind {
    /// Asset path, or `@bush` / `@deadwood` for the baked cards.
    pub texture: String,
    pub height_px: [f32; 2],
    pub width_ratio: f32,
    pub cards: u32,
    pub flat: bool,
    /// Linear colour multiplier per season (spring, summer, autumn, winter).
    pub season_tint: [[f32; 3]; 4],
}

/// Snow cover model of the CPU (the same inputs as `season_snow` of `terrain.wesl`, without the
/// patch noise of the ground, which the pale tufts replace by their own).
#[derive(Clone, Debug, Default)]
pub struct SnowModel {
    pub snow_lines: Vec4,
    pub snow_plain: Vec4,
    pub line_jitter_m: f32,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub seed: u64,
    pub far: Level,
    pub near_level: Level,
    pub near: Near,
    pub belt: Belt,
    pub pale: Pale,
    /// Steepest ground (m of height per m, real relief) that carries any piece, and the steepest
    /// that carries a pale tuft (grassy flats only).
    pub slope_max: f32,
    pub pale_slope_max: f32,
    /// Per level (far, near), per quality level (low, medium, high, ultra): cap of pieces shown.
    pub budget: [[usize; 4]; 2],
    pub kinds: Vec<Kind>,
    pub snow: SnowModel,
}

fn num(v: &Value, default: f32) -> f32 {
    v.as_f64().map_or(default, |x| x as f32)
}

fn level_of(v: &Value, default: &Level) -> Level {
    Level {
        cell_px: num(&v["cell_px"], default.cell_px).max(0.5),
        max_distance: num(&v["max_distance"], default.max_distance),
        fade_start: num(&v["fade_start"], default.fade_start).clamp(0.0, 0.99),
        radius_factor: num(&v["radius_factor"], default.radius_factor),
        min_radius: num(&v["min_radius"], default.min_radius),
        max_radius: num(&v["max_radius"], default.max_radius),
        max_cells: v["max_cells"]
            .as_u64()
            .map_or(default.max_cells, |n| n as usize),
    }
}

fn budget_of(v: &Value) -> [usize; 4] {
    ["low", "medium", "high", "ultra"].map(|k| v[k].as_u64().unwrap_or(0) as usize)
}

fn color_of(v: &Value) -> [f32; 3] {
    match v.as_array() {
        Some(a) if a.len() >= 3 => [num(&a[0], 1.0), num(&a[1], 1.0), num(&a[2], 1.0)],
        _ => [1.0; 3],
    }
}

impl Config {
    /// Settings from `understory.json` and the snow lines of `campaign_seasons.json`.
    pub fn from_json(doc: &Value, snow: SnowModel) -> Config {
        let far_default = Level {
            cell_px: 16.0,
            max_distance: 260.0,
            fade_start: 0.8,
            radius_factor: 1.3,
            min_radius: 40.0,
            max_radius: 300.0,
            max_cells: 700,
        };
        let near_default = Level {
            cell_px: 10.0,
            max_distance: 120.0,
            fade_start: 0.75,
            radius_factor: 1.0,
            min_radius: 20.0,
            max_radius: 70.0,
            max_cells: 160,
        };
        let n = &doc["near"];
        let b = &doc["belt"];
        let p = &doc["pale"];
        let mix = |k: &str| num(&b["mix"][k], 0.25).max(0.0);
        let shares = [
            mix("belt_tall"),
            mix("belt_fern"),
            mix("belt_bush"),
            mix("belt_deadwood"),
        ];
        let total = shares.iter().sum::<f32>().max(1e-4);
        let mut cumulative = [0.0f32; 4];
        let mut run = 0.0;
        for (i, s) in shares.iter().enumerate() {
            run += s / total;
            cumulative[i] = run;
        }
        cumulative[3] = 1.0;
        let kinds = CATEGORIES
            .iter()
            .map(|name| {
                let k = &doc["kinds"][name];
                let h = k["height_px"].as_array();
                let height_px = match h {
                    Some(a) if a.len() >= 2 => [num(&a[0], 0.3), num(&a[1], 0.5)],
                    _ => [0.3, 0.5],
                };
                let tints = k["season_tint"].as_array();
                let season_tint = std::array::from_fn(|i| {
                    tints.map_or([1.0; 3], |t| color_of(&t[i.min(t.len() - 1)]))
                });
                Kind {
                    texture: k["texture"].as_str().unwrap_or("@bush").to_string(),
                    height_px,
                    width_ratio: num(&k["width_ratio"], 0.8),
                    cards: k["cards"].as_u64().unwrap_or(3).clamp(1, 4) as u32,
                    flat: k["flat"].as_bool().unwrap_or(false),
                    season_tint,
                }
            })
            .collect();
        Config {
            seed: doc["seed"].as_u64().unwrap_or(7007),
            far: level_of(&doc["far"], &far_default),
            near_level: level_of(&doc["near"], &near_default),
            near: Near {
                spacing_px: num(&n["spacing_px"], 0.42).max(0.05),
                forest_max: num(&n["forest_max"], 0.4),
                crops_max: num(&n["crops_max"], 0.2),
                snow_max: num(&n["snow_max"], 0.6),
                flower_patch_frequency: num(&n["flower_patch_frequency"], 0.06),
                flower_patch_threshold: num(&n["flower_patch_threshold"], 0.3),
                flower_share_in_patch: num(&n["flower_share_in_patch"], 0.55),
                flower_share_outside: num(&n["flower_share_outside"], 0.04),
            },
            belt: Belt {
                width_px: num(&b["width_px"], 1.8).max(0.1),
                grid_px: num(&b["grid_px"], 0.4).max(0.05),
                spacing_px: num(&b["spacing_px"], 0.26).max(0.05),
                forest_min: num(&b["forest_min"], 0.45),
                inside_max: num(&b["inside_max"], 0.35),
                crops_max: num(&b["crops_max"], 0.2),
                falloff: num(&b["falloff"], 1.3),
                cumulative,
            },
            pale: Pale {
                spacing_px: num(&p["spacing_px"], 0.85).max(0.05),
                snow_min: num(&p["snow_min"], 0.2),
                crops_max: num(&p["crops_max"], 0.25),
                forest_max: num(&p["forest_max"], 0.3),
                tundra_start_m: num(&p["tundra_start_m"], 500.0),
                tundra_end_m: num(&p["tundra_end_m"], 150.0),
                tundra_share: num(&p["tundra_share"], 0.8),
                patch_frequency: num(&p["patch_frequency"], 0.05),
            },
            slope_max: num(&doc["slope"]["max"], 0.45),
            pale_slope_max: num(&doc["slope"]["pale_max"], 0.3),
            budget: [
                budget_of(&doc["budget"]["far"]),
                budget_of(&doc["budget"]["near"]),
            ],
            kinds,
            snow,
        }
    }

    /// Cap of pieces for a quality level (`low` .. `ultra`; anything else is `high`).
    pub fn budget_for(&self, level_index: usize, quality: &str) -> usize {
        let q = match quality {
            "low" => 0,
            "medium" => 1,
            "ultra" => 3,
            _ => 2,
        };
        self.budget[level_index][q]
    }
}

/// What the sowing reads of the world (a trait so that the placement is tested on a flat world).
pub trait Env {
    /// Forest cover [0, 1].
    fn forest(&self, p: Vec2) -> f32;
    /// Cultivated fields cover [0, 1].
    fn crops(&self, p: Vec2) -> f32;
    /// Land that is neither water, nor a road, nor a clearing.
    fn open(&self, p: Vec2) -> bool;
    /// Snow cover [0, 1] in the season of the sowing.
    fn snow(&self, p: Vec2) -> f32;
    /// Weight [0, 1] of the pale ground (patchy snow, tundra).
    fn pale(&self, p: Vec2) -> f32;
    /// Slope of the real relief (m of height per m).
    fn slope(&self, p: Vec2) -> f32;
}

/// One sown tuft, bush or branch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub pos: Vec2,
    pub yaw: f32,
    /// Map px.
    pub height: f32,
    pub category: u8,
    /// [0, 1), variation of the shade.
    pub tint: f32,
}

fn cell_rng(cfg: &Config, key: IVec2, salt: u64) -> Rng {
    Rng::new(
        (key.x as i64 as u64).wrapping_mul(0x9E37_79B1)
            ^ (key.y as i64 as u64).wrapping_mul(0x85EB_CA77)
            ^ cfg.seed.wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ salt,
    )
}

fn size_of(cfg: &Config, category: u8, rng: &mut Rng) -> f32 {
    let [lo, hi] = cfg.kinds[category as usize].height_px;
    rng.range(lo, hi)
}

/// Weights of the belt on a grid of `n x n` points of step `step` (the forest cover of each
/// point in `forest`): 0 inside the forest and beyond the band, otherwise `(1 - d / width)^falloff`
/// with `d` the distance to the nearest forest point.
pub fn belt_weights(forest: &[f32], n: usize, step: f32, belt: &Belt) -> Vec<f32> {
    let reach = (belt.width_px / step).ceil() as i32;
    let mut out = vec![0.0f32; n * n];
    for j in 0..n as i32 {
        for i in 0..n as i32 {
            if forest[j as usize * n + i as usize] > belt.inside_max {
                continue;
            }
            let mut nearest = f32::MAX;
            for dj in -reach..=reach {
                for di in -reach..=reach {
                    let (x, y) = (i + di, j + dj);
                    if x < 0 || y < 0 || x >= n as i32 || y >= n as i32 {
                        continue;
                    }
                    if forest[y as usize * n + x as usize] >= belt.forest_min {
                        nearest = nearest.min(((di * di + dj * dj) as f32).sqrt() * step);
                    }
                }
            }
            if nearest < belt.width_px {
                out[j as usize * n + i as usize] =
                    (1.0 - nearest / belt.width_px).powf(belt.falloff);
            }
        }
    }
    out
}

/// Sowing of a cell of the far level (`far.cell_px`): belt of undergrowth, then pale tufts.
pub fn seed_far(cfg: &Config, env: &dyn Env, key: IVec2) -> Vec<Piece> {
    let side = cfg.far.cell_px;
    let low = key.as_vec2() * side;
    let mut out = Vec::new();
    seed_belt(cfg, env, key, low, side, &mut out);
    seed_pale(cfg, env, key, low, side, &mut out);
    out
}

fn seed_belt(cfg: &Config, env: &dyn Env, key: IVec2, low: Vec2, side: f32, out: &mut Vec<Piece>) {
    let belt = &cfg.belt;
    let step = belt.grid_px;
    let margin = belt.width_px;
    let origin = low - Vec2::splat(margin);
    let n = ((side + 2.0 * margin) / step).ceil() as usize + 1;
    let mut forest = vec![0.0f32; n * n];
    let mut any_forest = false;
    for j in 0..n {
        for i in 0..n {
            let f = env.forest(origin + Vec2::new(i as f32, j as f32) * step);
            any_forest |= f >= belt.forest_min;
            forest[j * n + i] = f;
        }
    }
    if !any_forest {
        return;
    }
    let weights = belt_weights(&forest, n, step, belt);
    if weights.iter().all(|w| *w <= 0.0) {
        return;
    }
    let mut rng = cell_rng(cfg, key, 0xBE17);
    let candidates = (side * side / (belt.spacing_px * belt.spacing_px)).round() as usize;
    for _ in 0..candidates {
        let p = low + Vec2::new(rng.f32(), rng.f32()) * side;
        let roll = rng.f32();
        let kind_roll = rng.f32();
        let tint = rng.f32();
        let yaw = rng.f32() * std::f32::consts::TAU;
        let g = ((p - origin) / step).round().as_ivec2();
        let (gi, gj) = (
            g.x.clamp(0, n as i32 - 1) as usize,
            g.y.clamp(0, n as i32 - 1) as usize,
        );
        if roll >= weights[gj * n + gi]
            || env.crops(p) > belt.crops_max
            || !env.open(p)
            || env.slope(p) > cfg.slope_max
        {
            continue;
        }
        let category = belt
            .cumulative
            .iter()
            .position(|c| kind_roll < *c)
            .unwrap_or(3) as u8;
        let height = size_of(cfg, category, &mut rng);
        out.push(Piece {
            pos: p,
            yaw,
            height,
            category,
            tint,
        });
    }
}

fn seed_pale(cfg: &Config, env: &dyn Env, key: IVec2, low: Vec2, side: f32, out: &mut Vec<Piece>) {
    let pale = &cfg.pale;
    // Quick refusal: a 5 x 5 probe of the cell finds no pale ground.
    let probe = (0..25).any(|k| {
        let q = low + Vec2::new((k % 5) as f32, (k / 5) as f32) * (side / 4.0);
        env.pale(q) > 0.02
    });
    if !probe {
        return;
    }
    let mut rng = cell_rng(cfg, key, 0xFA1E);
    let candidates = (side * side / (pale.spacing_px * pale.spacing_px)).round() as usize;
    for _ in 0..candidates {
        let p = low + Vec2::new(rng.f32(), rng.f32()) * side;
        let roll = rng.f32();
        let tint = rng.f32();
        let yaw = rng.f32() * std::f32::consts::TAU;
        if roll >= env.pale(p)
            || env.forest(p) > pale.forest_max
            || env.crops(p) > pale.crops_max
            || !env.open(p)
            || env.slope(p) > cfg.pale_slope_max
        {
            continue;
        }
        let height = size_of(cfg, PALE, &mut rng);
        out.push(Piece {
            pos: p,
            yaw,
            height,
            category: PALE,
            tint,
        });
    }
}

/// Sowing of a cell of the near level: tall tufts and flowers on the open ground (`flower`
/// the low-frequency noise of the flower patches, in [-1, 1]).
pub fn seed_near(
    cfg: &Config,
    env: &dyn Env,
    flower: &dyn Fn(Vec2) -> f32,
    key: IVec2,
) -> Vec<Piece> {
    let near = &cfg.near;
    let side = cfg.near_level.cell_px;
    let low = key.as_vec2() * side;
    let mut rng = cell_rng(cfg, key, 0x0EA2);
    let candidates = (side * side / (near.spacing_px * near.spacing_px)).round() as usize;
    let mut out = Vec::with_capacity(candidates / 2);
    for _ in 0..candidates {
        let p = low + Vec2::new(rng.f32(), rng.f32()) * side;
        let kind_roll = rng.f32();
        let tint = rng.f32();
        let yaw = rng.f32() * std::f32::consts::TAU;
        if env.forest(p) > near.forest_max
            || env.crops(p) > near.crops_max
            || env.snow(p) > near.snow_max
            || !env.open(p)
            || env.slope(p) > cfg.slope_max
        {
            continue;
        }
        let share = if flower(p) > near.flower_patch_threshold {
            near.flower_share_in_patch
        } else {
            near.flower_share_outside
        };
        let category = if kind_roll < share {
            NEAR_FLOWERS
        } else {
            NEAR_TALL
        };
        let height = size_of(cfg, category, &mut rng);
        out.push(Piece {
            pos: p,
            yaw,
            height,
            category,
            tint,
        });
    }
    out
}

/// Number of cells kept, nearest first, so that their pieces stay under `budget` (`counts` in the
/// order of the nearest-first list; a cell still being sown counts 0).
pub fn cells_within_budget(counts: &[usize], budget: usize) -> usize {
    let mut total = 0usize;
    for (i, c) in counts.iter().enumerate() {
        if total + c > budget {
            return i;
        }
        total += c;
    }
    counts.len()
}

/// Vertex data of the pieces of `category` of a cell (positions relative to `origin`, `heights`
/// the displayed ground under each piece of `pieces`). Cards cross at equal angles; the flat kind
/// (branches) is one quad lying on the ground.
pub fn build_mesh_data(
    kind: &Kind,
    category: u8,
    pieces: &[Piece],
    heights: &[f32],
    origin: Vec2,
) -> (MeshData, usize) {
    let mut data = MeshData::default();
    let mut count = 0;
    for (piece, y) in pieces.iter().zip(heights) {
        if piece.category != category {
            continue;
        }
        count += 1;
        let shade = 0.85 + 0.3 * piece.tint;
        let color = [shade, shade, shade, 1.0];
        let (sin_yaw, cos_yaw) = piece.yaw.sin_cos();
        let rotate = |v: Vec3| {
            Vec3::new(
                v.x * cos_yaw - v.z * sin_yaw,
                v.y,
                v.x * sin_yaw + v.z * cos_yaw,
            )
        };
        let height = piece.height;
        let width = height * kind.width_ratio;
        let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        let mut quad = |corners: [Vec3; 4], normal: Vec3, foot: Vec3| {
            let base = data.positions.len() as u32;
            for (corner, uv) in corners.iter().zip(uvs) {
                data.positions.push((foot + rotate(*corner)).to_array());
                data.normals.push(rotate(normal).to_array());
                data.uvs.push(uv);
                data.colors.push(color);
            }
            data.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        };
        let local = Vec3::new(piece.pos.x - origin.x, 0.0, piece.pos.y - origin.y);
        if kind.flat {
            // Lying on the ground, a hair above it.
            let foot = local + Vec3::Y * (*y + height * 0.03);
            let half = Vec3::new(0.5 * height, 0.0, 0.0);
            let depth = Vec3::new(0.0, 0.0, 0.5 * width);
            quad(
                [-half - depth, half - depth, half + depth, -half + depth],
                Vec3::Y,
                foot,
            );
        } else {
            let foot = local + Vec3::Y * (*y - height * 0.08);
            for k in 0..kind.cards {
                let angle = std::f32::consts::PI * k as f32 / kind.cards as f32;
                let side = Vec3::new(angle.cos(), 0.0, angle.sin()) * 0.5 * width;
                let normal = Vec3::new(-angle.sin(), 0.0, angle.cos())
                    .lerp(Vec3::Y, 0.6)
                    .normalize();
                let up = Vec3::Y * height;
                quad([-side, side, side + up, -side + up], normal, foot);
            }
        }
    }
    (data, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat test world: forest for `x < 0`, everything else configurable.
    struct Flat {
        crops: f32,
        open: bool,
        snow: f32,
        pale: f32,
        slope: f32,
        forest_all: bool,
    }

    impl Default for Flat {
        fn default() -> Self {
            Flat {
                crops: 0.0,
                open: true,
                snow: 0.0,
                pale: 0.0,
                slope: 0.0,
                forest_all: false,
            }
        }
    }

    impl Env for Flat {
        fn forest(&self, p: Vec2) -> f32 {
            if self.forest_all || p.x < 0.0 {
                1.0
            } else {
                0.0
            }
        }
        fn crops(&self, _: Vec2) -> f32 {
            self.crops
        }
        fn open(&self, _: Vec2) -> bool {
            self.open
        }
        fn snow(&self, _: Vec2) -> f32 {
            self.snow
        }
        fn pale(&self, _: Vec2) -> f32 {
            self.pale
        }
        fn slope(&self, _: Vec2) -> f32 {
            self.slope
        }
    }

    fn config() -> Config {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/fx/understory.json"),
        )
        .expect("understory.json");
        Config::from_json(&serde_json::from_str(&text).unwrap(), SnowModel::default())
    }

    /// Cells along the forest edge `x = 0` (the cell column `x in [0, 16)`).
    fn edge_pieces(env: &Flat) -> Vec<Piece> {
        let cfg = config();
        (-3..3)
            .flat_map(|cy| seed_far(&cfg, env, IVec2::new(0, cy)))
            .collect()
    }

    #[test]
    fn belt_hugs_the_forest_edge_on_the_meadow_side() {
        let cfg = config();
        let pieces = edge_pieces(&Flat::default());
        assert!(pieces.len() > 200, "{} pieces", pieces.len());
        for p in &pieces {
            assert!(p.pos.x >= 0.0, "inside the forest: {:?}", p.pos);
            assert!(
                p.pos.x <= cfg.belt.width_px + 0.5,
                "beyond the band: {:?}",
                p.pos
            );
            assert!(p.category <= BELT_DEADWOOD);
        }
    }

    #[test]
    fn belt_density_falls_towards_the_meadow() {
        let cfg = config();
        let pieces = edge_pieces(&Flat::default());
        let half = cfg.belt.width_px * 0.5;
        let near = pieces.iter().filter(|p| p.pos.x < half).count();
        let far = pieces.iter().filter(|p| p.pos.x >= half).count();
        assert!(near > far * 2, "near {near} far {far}");
    }

    #[test]
    fn belt_mixes_the_four_categories() {
        let pieces = edge_pieces(&Flat::default());
        for c in [BELT_TALL, BELT_FERN, BELT_BUSH, BELT_DEADWOOD] {
            assert!(pieces.iter().any(|p| p.category == c), "category {c}");
        }
    }

    #[test]
    fn no_belt_on_closed_ground_or_fields() {
        let closed = Flat {
            open: false,
            ..Flat::default()
        };
        assert!(edge_pieces(&closed).is_empty());
        let fields = Flat {
            crops: 0.9,
            ..Flat::default()
        };
        assert!(edge_pieces(&fields).is_empty());
    }

    #[test]
    fn no_belt_inside_a_forest_nor_without_one() {
        let cfg = config();
        let inside = Flat {
            forest_all: true,
            ..Flat::default()
        };
        assert!(seed_far(&cfg, &inside, IVec2::new(0, 0)).is_empty());
        // Far from the edge (x in [160, 176)): meadow only.
        assert!(seed_far(&cfg, &Flat::default(), IVec2::new(10, 0)).is_empty());
    }

    #[test]
    fn pale_tufts_follow_the_pale_ground() {
        let cfg = config();
        let none = seed_far(&cfg, &Flat::default(), IVec2::new(10, 0));
        assert!(none.is_empty());
        let snowy = Flat {
            pale: 0.8,
            ..Flat::default()
        };
        let pieces = seed_far(&cfg, &snowy, IVec2::new(10, 0));
        assert!(pieces.len() > 100, "{}", pieces.len());
        assert!(pieces.iter().all(|p| p.category == PALE));
        let closed = Flat {
            pale: 0.8,
            open: false,
            ..Flat::default()
        };
        assert!(seed_far(&cfg, &closed, IVec2::new(10, 0)).is_empty());
        let fields = Flat {
            pale: 0.8,
            crops: 0.9,
            ..Flat::default()
        };
        assert!(seed_far(&cfg, &fields, IVec2::new(10, 0)).is_empty());
    }

    #[test]
    fn near_tufts_keep_off_forest_fields_water_and_deep_snow() {
        let cfg = config();
        let flat = |_: Vec2| 0.0;
        let key = IVec2::new(20, 20);
        let ok = seed_near(&cfg, &Flat::default(), &flat, key);
        assert!(ok.len() > 200, "{}", ok.len());
        assert!(ok.iter().all(|p| p.category >= NEAR_TALL));
        let deep = Flat {
            snow: 0.9,
            ..Flat::default()
        };
        assert!(seed_near(&cfg, &deep, &flat, key).is_empty());
        let wet = Flat {
            open: false,
            ..Flat::default()
        };
        assert!(seed_near(&cfg, &wet, &flat, key).is_empty());
        let crops = Flat {
            crops: 1.0,
            ..Flat::default()
        };
        assert!(seed_near(&cfg, &crops, &flat, key).is_empty());
        let wood = Flat {
            forest_all: true,
            ..Flat::default()
        };
        assert!(seed_near(&cfg, &wood, &flat, key).is_empty());
    }

    #[test]
    fn flowers_cluster_in_their_patches() {
        let cfg = config();
        let key = IVec2::new(20, 20);
        let count = |patch: f32| {
            seed_near(&cfg, &Flat::default(), &move |_| patch, key)
                .iter()
                .filter(|p| p.category == NEAR_FLOWERS)
                .count()
        };
        assert!(count(1.0) > count(-1.0) * 4);
    }

    #[test]
    fn nothing_on_steep_ground_and_pale_only_on_flats() {
        let cfg = config();
        let flat = |_: Vec2| 0.0;
        let steep = Flat {
            slope: cfg.slope_max + 0.1,
            pale: 0.8,
            ..Flat::default()
        };
        assert!(edge_pieces(&steep).is_empty());
        assert!(seed_far(&cfg, &steep, IVec2::new(10, 0)).is_empty());
        assert!(seed_near(&cfg, &steep, &flat, IVec2::new(20, 20)).is_empty());
        // Between the two limits: the belt and the near tufts stay, the pale tufts go.
        let sloped = Flat {
            slope: (cfg.slope_max + cfg.pale_slope_max) * 0.5,
            pale: 0.8,
            ..Flat::default()
        };
        assert!(!edge_pieces(&sloped).is_empty());
        assert!(seed_far(&cfg, &sloped, IVec2::new(10, 0)).is_empty());
        assert!(!seed_near(&cfg, &sloped, &flat, IVec2::new(20, 20)).is_empty());
    }

    #[test]
    fn sowing_is_deterministic() {
        let cfg = config();
        let env = Flat::default();
        assert_eq!(
            seed_far(&cfg, &env, IVec2::new(0, 1)),
            seed_far(&cfg, &env, IVec2::new(0, 1))
        );
        assert_ne!(
            seed_far(&cfg, &env, IVec2::new(0, 1)),
            seed_far(&cfg, &env, IVec2::new(0, 2))
        );
    }

    #[test]
    fn budget_keeps_the_nearest_cells() {
        assert_eq!(cells_within_budget(&[10, 10, 10], 25), 2);
        assert_eq!(cells_within_budget(&[10, 10, 10], 100), 3);
        assert_eq!(cells_within_budget(&[50, 1], 10), 0);
        assert_eq!(cells_within_budget(&[], 10), 0);
    }

    #[test]
    fn real_settings_are_complete() {
        let cfg = config();
        assert_eq!(cfg.kinds.len(), CATEGORY_COUNT);
        assert!(cfg.far.max_distance > cfg.near_level.max_distance);
        assert_eq!(cfg.budget_for(0, "low"), 0);
        assert!(cfg.budget_for(1, "ultra") > cfg.budget_for(1, "medium"));
        assert!((cfg.belt.cumulative[3] - 1.0).abs() < 1e-6);
        assert!(cfg.kinds.iter().all(|k| k.season_tint.len() == 4));
        // Fade: full under the start, nothing at the end.
        assert_eq!(cfg.near_level.fade(10.0), 1.0);
        assert_eq!(cfg.near_level.fade(cfg.near_level.max_distance), 0.0);
    }

    #[test]
    fn mesh_has_four_vertices_per_card() {
        let cfg = config();
        let piece = |category| Piece {
            pos: Vec2::new(3.0, 4.0),
            yaw: 0.5,
            height: 0.3,
            category,
            tint: 0.5,
        };
        let pieces = [piece(BELT_TALL), piece(BELT_DEADWOOD)];
        let heights = [2.0, 2.5];
        let (tall, n) = build_mesh_data(
            &cfg.kinds[BELT_TALL as usize],
            BELT_TALL,
            &pieces,
            &heights,
            Vec2::new(1.0, 1.0),
        );
        assert_eq!(n, 1);
        assert_eq!(tall.positions.len(), 12);
        let (wood, n) = build_mesh_data(
            &cfg.kinds[BELT_DEADWOOD as usize],
            BELT_DEADWOOD,
            &pieces,
            &heights,
            Vec2::ZERO,
        );
        assert_eq!((n, wood.positions.len()), (1, 4));
        assert!(wood.positions.iter().all(|p| p[1] > 2.5));
    }
}
