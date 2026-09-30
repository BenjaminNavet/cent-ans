//! Scattering of campaign-map vegetation tiles (lot PB2), pure Rust.
//!
//! Port of the scatter, hedge and pack stages of `VegetationTileJob`
//! (`game/scripts/map/vegetation_tile_job.gd`) and of the parcel lattice of
//! `VegetationFields`. The coarse mask grid (noise, slopes, splat) is still
//! sampled in GDScript off the main thread; the main thread then hands plain
//! arrays to `VegetationScatter` (godot-bridge), whose native threads call
//! `scatter_tile` without touching the Godot API. The random
//! stream differs from Godot's `RandomNumberGenerator`, so the result is
//! statistically equivalent, not identical. Rendering support only, no game
//! rule.

use std::f64::consts::TAU;
use std::sync::Arc;

pub mod species;
pub use species::{Distribution, SpeciesTable};
use species::{
    B_FOREST, B_GROVE, B_ISOLATED, B_ORCHARD, B_ORCHARD_OPEN, B_ORCHARD_RING, B_RIPARIAN,
    B_RIPARIAN_PX, B_SCRUB, CUSTOM_STRIDE, ROLE_EDGE, ROLE_ISOLATED, ROLE_MASSIF, ROLE_ORCHARD,
    ROLE_RIPARIAN, ROLE_SCRUB,
};
const KIND_OAK: usize = 0;
const KIND_BEECH: usize = 1;
const KIND_CONIFER: usize = 2;
pub const KIND_HEDGE: usize = 3;
pub const KIND_COUNT: usize = 4;
const CANOPY_SPREAD: f64 = 0.4;
const PARTS_SIDE: usize = 2;
#[cfg(test)]
const PARTS: usize = PARTS_SIDE * PARTS_SIDE;
/// Detail cells (lot SZ4b): at most this many parts per side.
const MAX_DETAIL_PARTS_SIDE: usize = 8;
pub const FLOATS_PER_INSTANCE: usize = 16;
const GROUND_SINK: f64 = 0.08;
const RIVER_CLEARANCE: f64 = 0.3;
const HEDGE_STEP: f64 = 0.62;
const HEDGE_GAP: f64 = 0.1;
const HEDGE_TREE: f64 = 0.07;
/// `VegetationFields.LAYOUTS`: [k (shear), fu, fv (parcel size, px), warp phase].
const LAYOUTS: [[f64; 4]; 2] = [[0.35, 5.0, 4.2, 0.0], [-0.8, 4.6, 3.8, 2.1]];

// --- Relief pyramid (`ReliefPyramid`, `ReliefQuadtree`) ---
pub const PAGE_PX: usize = 512;
const ROOT_TILE_UNITS: f64 = 256.0;
/// Tile grid offset from map coordinates: 0 since SZ2b (ADR 0086, map pixel i centred at
/// x = i + 0.5 like every tool-side vector).
const GRID_OFFSET: f64 = 0.0;

/// Map-wide rasters, shared by every job.
#[derive(Default)]
pub struct MapRasters {
    /// Heightmap samples (`MapData.height_bytes`).
    pub height: Vec<u8>,
    pub width: usize,
    pub height_px: usize,
    pub bpp: usize,
    pub little_endian: bool,
    pub height_min_m: f64,
    pub height_max_m: f64,
    /// River bed signed distance, one byte per pixel (`MapData.river_bed_image`, R channel).
    pub river: Vec<u8>,
    pub river_width: usize,
    pub river_height: usize,
}

impl MapRasters {
    fn height01_px(&self, px: i64, py: i64) -> f64 {
        let px = px.clamp(0, self.width as i64 - 1) as usize;
        let py = py.clamp(0, self.height_px as i64 - 1) as usize;
        let offset = py * self.width + px;
        if self.bpp == 2 {
            let o = offset * 2;
            let (a, b) = (self.height[o] as u32, self.height[o + 1] as u32);
            let value = if self.little_endian {
                a | (b << 8)
            } else {
                (a << 8) | b
            };
            return value as f64 / 65535.0;
        }
        self.height[offset] as f64 / 255.0
    }

    /// `MapData.height_m_at` (bilinear, clamped; pixel i centred at x = i + 0.5, ADR 0086).
    fn height_m_at(&self, x: f64, y: f64) -> f64 {
        if self.width == 0 {
            return 0.0;
        }
        let fx = (x - 0.5).clamp(0.0, self.width as f64 - 1.0);
        let fy = (y - 0.5).clamp(0.0, self.height_px as f64 - 1.0);
        let x0 = fx as i64;
        let y0 = fy as i64;
        let tx = fx - x0 as f64;
        let ty = fy - y0 as f64;
        let top = lerp(self.height01_px(x0, y0), self.height01_px(x0 + 1, y0), tx);
        let bottom = lerp(
            self.height01_px(x0, y0 + 1),
            self.height01_px(x0 + 1, y0 + 1),
            tx,
        );
        let h = lerp(top, bottom, ty);
        self.height_min_m + h * (self.height_max_m - self.height_min_m)
    }

    /// `MapData.river_sd_at` (nearest pixel; 8 without river raster).
    fn river_sd_at(&self, x: f64, y: f64) -> f64 {
        if self.river.is_empty() {
            return 8.0;
        }
        let px = (x as i64).clamp(0, self.river_width as i64 - 1) as usize;
        let py = (y as i64).clamp(0, self.river_height as i64 - 1) as usize;
        (self.river[py * self.river_width + px] as f64 - 128.0) / 16.0
    }
}

/// Relief fields of the displayed height (`MapData.relief_fields_at`), bilinear between cell
/// centres, edges replicated: valley floor of the local relief exaggeration (lot ZG8, metres),
/// uncapped base and mountain squash factor k (lot SZ1). Empty `base` / `squash`: base = floor,
/// k = 0.
#[derive(Default)]
pub struct ReliefFloor {
    pub data: Vec<f32>,
    pub base: Vec<f32>,
    pub squash: Vec<f32>,
    pub side: (usize, usize),
    pub cell: f64,
}

impl ReliefFloor {
    /// (floor, base, k) at the map point (x, z).
    fn fields_at(&self, x: f64, z: f64) -> (f64, f64, f64) {
        let (sx, sz) = self.side;
        if self.data.is_empty() || sx < 2 || sz < 2 {
            return (0.0, 0.0, 0.0);
        }
        let floor = Self::bilinear(&self.data, self.side, self.cell, x, z);
        let base = if self.base.len() == self.data.len() {
            Self::bilinear(&self.base, self.side, self.cell, x, z)
        } else {
            floor
        };
        let squash = if self.squash.len() == self.data.len() {
            Self::bilinear(&self.squash, self.side, self.cell, x, z)
        } else {
            0.0
        };
        (floor, base, squash)
    }

    fn bilinear(d: &[f32], side: (usize, usize), cell: f64, x: f64, z: f64) -> f64 {
        let (sx, sz) = side;
        let (fx, fz) = {
            let half = 0.5 * (cell - 1.0);
            (
                ((x - half) / cell).clamp(0.0, sx as f64 - 1.0),
                ((z - half) / cell).clamp(0.0, sz as f64 - 1.0),
            )
        };
        let i = (fx as usize).min(sx - 2);
        let j = (fz as usize).min(sz - 2);
        let tx = fx - i as f64;
        let tz = fz - j as f64;
        let o = j * sx + i;
        let top = lerp(d[o] as f64, d[o + 1] as f64, tx);
        let bottom = lerp(d[o + sx] as f64, d[o + sx + 1] as f64, tx);
        lerp(top, bottom, tz)
    }
}

/// Bytes of a quadtree page, shared with the page store of `ReliefLod` (lot PB3g: no copy per
/// request).
pub type PageBytes = std::sync::Arc<Vec<u8>>;

/// Displayed ground under the trees (`TerrainBuilder.surface_grid`).
pub enum Ground {
    /// Heightmap only.
    None,
    /// Regular mesh grid, triangulated along the a → d diagonal.
    Grid {
        heights: Vec<f32>,
        side: usize,
        unit: f64,
    },
    /// Quadtree snapshot: loaded pages (key → little-endian 16-bit samples).
    Pages {
        pages: std::collections::HashMap<i64, PageBytes>,
        max_level: i64,
        h_min: f64,
        h_range: f64,
    },
}

/// One tile to scatter (fields of `VegetationTileJob`).
pub struct TileRequest {
    pub tile_index: i64,
    pub origin: (f64, f64),
    pub size_px: f64,
    pub spacing: f64,
    pub coarse_step: f64,
    pub tree_scale: f64,
    pub vertical_scale: f64,
    /// Local relief gain (`MapData.relief_gain`, lot ZG8), mountain squash weight
    /// (`MapData.relief_squash`, lot SZ1) and their relief fields.
    pub relief_gain: f64,
    pub relief_squash: f64,
    pub floor: std::sync::Arc<ReliefFloor>,
    /// Coarse grids, `side × side`: forest, crops, conifer, beech, hedge, grove, region.
    pub coarse: [Vec<f32>; 7],
    pub side: usize,
    pub exclusions: Vec<(f64, f64, f64)>,
    pub ground: Ground,
    /// Lot SZ4b: dense forest cell scattered inside the tile whose coarse grids are given.
    pub detail: Option<DetailArea>,
    /// Lot HB4: biome index per coarse cell (`side × side`, nearest; empty = default biome).
    pub biome: Vec<f32>,
    /// Lot HB4: species table; `None` keeps the V4 scatter (oak, beech, conifer).
    pub species: Option<Arc<SpeciesTable>>,
}

/// Lot SZ4b (dense forest near the camera at the valley tier): a sub-rectangle of the tile
/// (`TileRequest::origin`, `size_px` still describe the coarse grids), scattered without hedges.
/// Only instances whose random seed is at least `1 − keep` are kept; the random stream does not
/// depend on `keep`, so a cell scattered with a smaller `keep` is an exact subset of the same
/// cell scattered with a larger one. Parts: `parts_side²` slots per kind over the rectangle.
/// `corridors`: segments `[x0, y0, x1, y1, half_width]` (world units) kept free of trees (fine
/// rivers and draped roads of lot ZG5b, which the map-wide river raster does not know).
#[derive(Clone, Debug, Default)]
pub struct DetailArea {
    pub rect: (f64, f64, f64, f64),
    pub keep: f64,
    pub parts_side: usize,
    pub corridors: Vec<[f64; 5]>,
}

/// Corridor segments binned on a regular grid over the scattered rectangle.
struct CorridorBins {
    origin: (f64, f64),
    cell: f64,
    side_x: usize,
    side_y: usize,
    bins: Vec<Vec<u32>>,
}

impl CorridorBins {
    const CELL: f64 = 1.0;

    fn new(rect: (f64, f64, f64, f64), corridors: &[[f64; 5]]) -> Self {
        let cell = Self::CELL;
        let side_x = (((rect.2 - rect.0) / cell).ceil() as usize).max(1);
        let side_y = (((rect.3 - rect.1) / cell).ceil() as usize).max(1);
        let mut bins = vec![Vec::new(); side_x * side_y];
        for (index, c) in corridors.iter().enumerate() {
            let (x0, x1) = (c[0].min(c[2]) - c[4], c[0].max(c[2]) + c[4]);
            let (y0, y1) = (c[1].min(c[3]) - c[4], c[1].max(c[3]) + c[4]);
            let i0 = ((x0 - rect.0) / cell).floor().max(0.0) as usize;
            let j0 = ((y0 - rect.1) / cell).floor().max(0.0) as usize;
            let i1 = (((x1 - rect.0) / cell).floor().max(-1.0) as i64).min(side_x as i64 - 1);
            let j1 = (((y1 - rect.1) / cell).floor().max(-1.0) as i64).min(side_y as i64 - 1);
            for j in j0 as i64..=j1 {
                for i in i0 as i64..=i1 {
                    bins[j as usize * side_x + i as usize].push(index as u32);
                }
            }
        }
        CorridorBins {
            origin: (rect.0, rect.1),
            cell,
            side_x,
            side_y,
            bins,
        }
    }

    fn blocks(&self, corridors: &[[f64; 5]], x: f64, y: f64) -> bool {
        let i = ((x - self.origin.0) / self.cell).floor();
        let j = ((y - self.origin.1) / self.cell).floor();
        if i < 0.0 || j < 0.0 || i as usize >= self.side_x || j as usize >= self.side_y {
            return false;
        }
        self.bins[j as usize * self.side_x + i as usize]
            .iter()
            .any(|&k| segment_distance(&corridors[k as usize], x, y) < corridors[k as usize][4])
    }
}

/// Distance from (x, y) to the segment `[x0, y0, x1, y1, _]`.
fn segment_distance(c: &[f64; 5], x: f64, y: f64) -> f64 {
    let (dx, dy) = (c[2] - c[0], c[3] - c[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((x - c[0]) * dx + (y - c[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (px, py) = (c[0] + t * dx - x, c[1] + t * dy - y);
    (px * px + py * py).sqrt()
}

/// Per-slot MultiMesh buffers (`part * KIND_COUNT + kind`).
pub struct TileResult {
    pub buffers: Vec<Vec<f32>>,
    pub counts: Vec<i32>,
}

/// PCG32 (XSH-RR), seeded per tile: deterministic, fast, uniform.
struct Rng {
    state: u64,
}

impl Rng {
    const MUL: u64 = 6364136223846793005;
    const INC: u64 = 1442695040888963407;

    fn new(seed: u64) -> Self {
        let mut rng = Rng { state: 0 };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(Self::MUL).wrapping_add(Self::INC);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in [0, 1).
    fn randf(&mut self) -> f64 {
        self.next_u32() as f64 / 4294967296.0
    }

    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.randf()
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// --- Parcel lattice (`VegetationFields`, mirrored in `terrain.gdshader`) ---

/// `VegetationFields.hash01` (same integer hash as the shader's `hash01i`).
fn hash01(n: i64) -> f64 {
    let mut h = n.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
    h = (h ^ (h >> 13)).wrapping_mul(1274126177) & 0x7fffffff;
    (h % 100000) as f64 / 100000.0
}

fn warp(x0: f64, y0: f64, phase: f64) -> (f64, f64) {
    let ox = 1.7 * (0.071 * y0 + 1.9 * (0.027 * x0).sin() + phase).sin()
        + 0.55 * (0.23 * y0 + 1.3 * (0.061 * x0).sin()).sin();
    let oy = 1.7 * (0.063 * x0 + 1.7 * (0.023 * y0).sin() + phase * 1.3).sin()
        + 0.55 * (0.19 * x0 + 1.1 * (0.047 * y0).sin()).sin();
    (ox, oy)
}

fn to_map(layout: usize, u: f64, v: f64) -> (f64, f64) {
    let [k, fu, fv, phase] = LAYOUTS[layout];
    let a = u * fu;
    let b = v * fv;
    let det = 1.0 + k * k;
    let x0 = (a - k * b) / det;
    let y0 = (b + k * a) / det;
    let (ox, oy) = warp(x0, y0, phase);
    (x0 + ox, y0 + oy)
}

fn to_uv(layout: usize, x0: f64, y0: f64) -> (f64, f64) {
    let [k, fu, fv, _] = LAYOUTS[layout];
    ((x0 + k * y0) / fu, (y0 - k * x0) / fv)
}

fn row_offset(layout: usize, column: i64) -> f64 {
    hash01(column * 5023 + 17 + layout as i64 * 101)
}

fn roll_u_edge(layout: usize, line: i64, segment: i64) -> f64 {
    hash01(line * 7919 + segment * 104729 + layout as i64 * 7 + 1)
}

fn roll_v_edge(layout: usize, row: i64, column: i64) -> f64 {
    hash01(row * 7919 + column * 104729 + layout as i64 * 7 + 31)
}

fn hedge_probability(open_land: f64, bocage: f64) -> f64 {
    open_land * lerp(0.1, 0.75, bocage)
}

/// Bounds of `t` keeping `offset + coeff * t` in `[lo, hi]`, crossed with `[cur_lo, cur_hi]`.
fn clip_linear(coeff: f64, offset: f64, lo: f64, hi: f64, cur: (f64, f64)) -> (f64, f64) {
    if coeff == 0.0 {
        return if offset >= lo && offset <= hi {
            cur
        } else {
            (1.0, -1.0)
        };
    }
    let t_a = (lo - offset) / coeff;
    let t_b = (hi - offset) / coeff;
    if coeff > 0.0 {
        (cur.0.max(t_a), cur.1.min(t_b))
    } else {
        (cur.0.max(t_b), cur.1.min(t_a))
    }
}

// --- Ground sampling ---

/// Bilinear altitude (m) in a relief page (`ReliefQuadtree._bilinear`).
fn page_bilinear(bytes: &[u8], fx: f64, fy: f64, h_min: f64, h_range: f64) -> f64 {
    let last = (PAGE_PX - 1) as f64;
    let fx = fx.clamp(0.0, last);
    let fy = fy.clamp(0.0, last);
    let i = (fx as usize).min(PAGE_PX - 2);
    let j = (fy as usize).min(PAGE_PX - 2);
    let tx = fx - i as f64;
    let ty = fy - j as f64;
    let o = (j * PAGE_PX + i) * 2;
    let sample = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as f64;
    let a = sample(o);
    let b = sample(o + 2);
    let c = sample(o + PAGE_PX * 2);
    let d = sample(o + PAGE_PX * 2 + 2);
    let top = a + (b - a) * tx;
    let v = (top + (c + (d - c) * tx - top) * ty) / 65535.0;
    h_min + v * h_range
}

impl TileRequest {
    /// `MapData.display_height` (lots ZG8, SZ1):
    /// s·(h − K·max(h − base, 0) + g·(1 − K)·max(h − floor, 0)), K = c·k.
    fn display_height(&self, h_m: f64, x: f64, z: f64) -> f64 {
        if self.relief_gain == 0.0 && self.relief_squash == 0.0 {
            return h_m * self.vertical_scale;
        }
        let (floor, base, k) = self.floor.fields_at(x, z);
        let squash = self.relief_squash * k;
        self.vertical_scale
            * (h_m - squash * (h_m - base).max(0.0)
                + self.relief_gain * (1.0 - squash) * (h_m - floor).max(0.0))
    }

    /// `MapData.height_world_at` (displayed height of the map heightmap).
    fn height_world_at(&self, map: &MapRasters, x: f64, y: f64) -> f64 {
        self.display_height(map.height_m_at(x, y), x, y)
    }

    /// `VegetationTileJob._display_ground` (`TerrainBuilder.grid_height`, never below sea level).
    fn display_ground(&self, map: &MapRasters, x: f64, y: f64, fallback: f64) -> f64 {
        let lx = x - self.origin.0;
        let ly = y - self.origin.1;
        let h = match &self.ground {
            Ground::None => return fallback,
            Ground::Grid {
                heights,
                side,
                unit,
            } => {
                let limit = *side as f64 - 1.001;
                let gx = (lx / unit).clamp(0.0, limit);
                let gy = (ly / unit).clamp(0.0, limit);
                let i = gx as usize;
                let j = gy as usize;
                let tx = gx - i as f64;
                let ty = gy - j as f64;
                let a = j * side + i;
                let ha = heights[a] as f64;
                let hb = heights[a + 1] as f64;
                let hc = heights[a + side] as f64;
                let hd = heights[a + side + 1] as f64;
                if tx >= ty {
                    ha + (hb - ha) * tx + (hd - hb) * ty
                } else {
                    ha + (hd - hc) * tx + (hc - ha) * ty
                }
            }
            Ground::Pages {
                pages,
                max_level,
                h_min,
                h_range,
            } => self
                .sample_pages(pages, *max_level, *h_min, *h_range, x, y)
                .unwrap_or_else(|| self.height_world_at(map, x, y)),
        };
        h.max(0.0)
    }

    /// `ReliefQuadtree.sample_pages`: finest loaded page covering (x, y).
    fn sample_pages(
        &self,
        pages: &std::collections::HashMap<i64, PageBytes>,
        top_level: i64,
        h_min: f64,
        h_range: f64,
        x: f64,
        y: f64,
    ) -> Option<f64> {
        if pages.is_empty() {
            return None;
        }
        for level in (0..=top_level).rev() {
            let units = ROOT_TILE_UNITS / (1i64 << level) as f64;
            let col = ((x - GRID_OFFSET) / units).floor() as i64;
            let row = ((y - GRID_OFFSET) / units).floor() as i64;
            if col < 0 || row < 0 {
                return None;
            }
            let key = (level << 24) | (row << 12) | col;
            let Some(bytes) = pages.get(&key) else {
                continue;
            };
            let px_units = units / PAGE_PX as f64;
            let ox = col as f64 * units + GRID_OFFSET;
            let oy = row as f64 * units + GRID_OFFSET;
            let h_m = page_bilinear(
                bytes,
                (x - ox) / px_units - 0.5,
                (y - oy) / px_units - 0.5,
                h_min,
                h_range,
            );
            return Some(self.display_height(h_m, x, y));
        }
        None
    }
}

// --- Scatter ---

struct Instance {
    seed: f64,
    /// Row-major 3×4 transform (basis columns interleaved with the origin).
    transform: [f32; 12],
    tint: [f32; 3],
}

struct Scatter<'a> {
    req: &'a TileRequest,
    map: &'a MapRasters,
    rng: Rng,
    raw: Vec<Vec<Instance>>,
    rect: (f64, f64, f64, f64),
    parts_side: usize,
    keep: f64,
    corridors: Option<CorridorBins>,
}

const FOREST: usize = 0;
const CROPS: usize = 1;
const CONIFER: usize = 2;
const BEECH: usize = 3;
const HEDGE: usize = 4;
const GROVE: usize = 5;
const REGION: usize = 6;

impl<'a> Scatter<'a> {
    fn has_point(&self, x: f64, y: f64) -> bool {
        let (x0, y0, x1, y1) = self.rect;
        x >= x0 && y >= y0 && x < x1 && y < y1
    }

    fn lerp_grid(&self, grid: usize, gx: f64, gy: f64) -> f64 {
        let side = self.req.side;
        let values = &self.req.coarse[grid];
        let i0 = (gx as usize).min(side - 2);
        let j0 = (gy as usize).min(side - 2);
        let tx = gx - i0 as f64;
        let ty = gy - j0 as f64;
        let k = j0 * side + i0;
        let top = lerp(values[k] as f64, values[k + 1] as f64, tx);
        let bottom = lerp(values[k + side] as f64, values[k + side + 1] as f64, tx);
        lerp(top, bottom, ty)
    }

    fn in_corridor(&self, x: f64, y: f64) -> bool {
        match (&self.corridors, &self.req.detail) {
            (Some(bins), Some(area)) => bins.blocks(&area.corridors, x, y),
            _ => false,
        }
    }

    fn excluded(&self, x: f64, y: f64) -> bool {
        self.req.exclusions.iter().any(|&(ex, ey, r)| {
            let dx = x - ex;
            let dy = y - ey;
            dx * dx + dy * dy < r * r
        })
    }

    fn slot(&self, kind: usize, x: f64, y: f64) -> usize {
        let (x0, y0, x1, y1) = self.rect;
        let side = self.parts_side;
        let last = side as i64 - 1;
        let px = (((x - x0) / (x1 - x0) * side as f64) as i64).clamp(0, last) as usize;
        let py = (((y - y0) / (y1 - y0) * side as f64) as i64).clamp(0, last) as usize;
        (py * side + px) * KIND_COUNT + kind
    }

    fn run(&mut self) {
        let req = self.req;
        let (ox, oy) = req.origin;
        let (rx0, ry0, rx1, ry1) = self.rect;
        let cells_x = ((rx1 - rx0) / req.spacing).ceil() as usize;
        let cells_y = ((ry1 - ry0) / req.spacing).ceil() as usize;
        for cj in 0..cells_y {
            for ci in 0..cells_x {
                let x = rx0 + (ci as f64 + self.rng.randf()) * req.spacing;
                let y = ry0 + (cj as f64 + self.rng.randf()) * req.spacing;
                let roll = self.rng.randf();
                let roll_kind = self.rng.randf();
                let gx = (x - ox) / req.coarse_step;
                let gy = (y - oy) / req.coarse_step;
                let forest = self.lerp_grid(FOREST, gx, gy);
                let yaw = self.rng.randf() * TAU;
                if let Some(table) = req.species.as_deref() {
                    self.species_candidate(table, x, y, roll, roll_kind, forest, yaw);
                    continue;
                }
                let mut kind = None;
                let mut scale_factor = 1.0;
                if roll < forest * 0.9 {
                    kind = Some(if roll_kind < self.lerp_grid(CONIFER, gx, gy) {
                        KIND_CONIFER
                    } else if self.rng.randf() < self.lerp_grid(BEECH, gx, gy) {
                        KIND_BEECH
                    } else {
                        KIND_OAK
                    });
                    scale_factor = 1.0 + CANOPY_SPREAD * smoothstep(0.45, 0.9, forest);
                } else {
                    let crops = self.lerp_grid(CROPS, gx, gy);
                    if crops > 0.15 {
                        let grove = self.lerp_grid(GROVE, gx, gy);
                        if roll < crops * (grove * 0.55 + 0.012) {
                            kind = Some(if self.rng.randf() < 0.2 {
                                KIND_BEECH
                            } else {
                                KIND_OAK
                            });
                            scale_factor = 0.9;
                        }
                    }
                }
                let Some(kind) = kind else {
                    continue;
                };
                if self.excluded(x, y) || !self.has_point(x, y) || self.in_corridor(x, y) {
                    continue;
                }
                if self.map.river_sd_at(x, y) < RIVER_CLEARANCE {
                    continue;
                }
                let ground = req.height_world_at(self.map, x, y);
                if ground <= 0.0 {
                    continue;
                }
                let ground = req.display_ground(self.map, x, y, ground);
                self.push(kind, x, ground, y, yaw, scale_factor);
            }
        }
        if req.detail.is_none() {
            self.hedges();
        }
    }

    fn hedges(&mut self) {
        let req = self.req;
        let crops_max = req.coarse[CROPS].iter().fold(0.0f32, |m, &v| m.max(v)) as f64;
        let hedge_max = req.coarse[HEDGE].iter().fold(0.0f32, |m, &v| m.max(v)) as f64;
        if crops_max < 0.2 {
            return;
        }
        let p_max = hedge_probability(crops_max, hedge_max);
        let (rx0, ry0, rx1, ry1) = self.rect;
        let (bx0, by0, bx1, by1) = (rx0 - 3.0, ry0 - 3.0, rx1 + 3.0, ry1 + 3.0);
        for (layout, &[k, fu, fv, _]) in LAYOUTS.iter().enumerate() {
            let (mut u_min, mut u_max) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut v_min, mut v_max) = (f64::INFINITY, f64::NEG_INFINITY);
            for (cx, cy) in [(bx0, by0), (bx1, by0), (bx0, by1), (bx1, by1)] {
                let (u, v) = to_uv(layout, cx, cy);
                u_min = u_min.min(u);
                u_max = u_max.max(u);
                v_min = v_min.min(v);
                v_max = v_max.max(v);
            }
            let det = 1.0 + k * k;
            let coeff_y = fv / det;
            let coeff_x = -k * fv / det;
            let dv = HEDGE_STEP / fv;
            let (line_lo, line_hi) = (u_min.floor() as i64, u_max.ceil() as i64);
            for line in line_lo..=line_hi {
                let a0 = line as f64 * fu;
                let range = clip_linear(coeff_y, k * a0 / det, by0, by1, (v_min, v_max));
                let range = clip_linear(coeff_x, a0 / det, bx0, bx1, range);
                if range.0 >= range.1 {
                    continue;
                }
                let mut v = v_min;
                while v < range.0 {
                    v += dv;
                }
                let mut cur = None;
                while v < range.1 {
                    let next_v = v + dv;
                    let roll = roll_u_edge(layout, line, v.floor() as i64);
                    if roll < p_max {
                        let from = cur.unwrap_or_else(|| to_map(layout, line as f64, v));
                        let next = to_map(layout, line as f64, next_v);
                        self.hedge_point(layout, roll, from, next);
                        cur = Some(next);
                    } else {
                        cur = None;
                    }
                    v = next_v;
                }
            }
            let du = HEDGE_STEP / fu;
            let cx = fu / det;
            let cy = k * fu / det;
            let margin_x = 0.5 * cx.abs();
            let margin_y = 0.5 * cy.abs();
            for column in line_lo..=line_hi {
                let offset = row_offset(layout, column);
                let u_mid = column as f64 + 0.5;
                let range = clip_linear(
                    coeff_y,
                    cy * u_mid,
                    by0 - margin_y,
                    by1 + margin_y,
                    (v_min, v_max),
                );
                let range = clip_linear(coeff_x, cx * u_mid, bx0 - margin_x, bx1 + margin_x, range);
                if range.0 >= range.1 {
                    continue;
                }
                let row_lo = (range.0 + offset).floor() as i64 - 1;
                let row_hi = (range.1 + offset).ceil() as i64 + 2;
                for row in row_lo..row_hi {
                    let roll = roll_v_edge(layout, row, column);
                    if roll >= p_max {
                        continue;
                    }
                    let v_row = row as f64 - offset;
                    let mut u = column as f64 + du * 0.5;
                    let mut cur = to_map(layout, u, v_row);
                    while u < column as f64 + 1.0 {
                        let next_u = u + du;
                        let next = to_map(layout, next_u, v_row);
                        self.hedge_point(layout, roll, cur, next);
                        u = next_u;
                        cur = next;
                    }
                }
            }
        }
    }

    fn hedge_point(&mut self, layout: usize, roll: f64, pos: (f64, f64), next: (f64, f64)) {
        if !self.has_point(pos.0, pos.1) {
            return;
        }
        let req = self.req;
        let gx = (pos.0 - req.origin.0) / req.coarse_step;
        let gy = (pos.1 - req.origin.1) / req.coarse_step;
        let region = self.lerp_grid(REGION, gx, gy);
        if (region > 0.0) != (layout == 1) || region.abs() < 0.02 {
            return;
        }
        let crops = self.lerp_grid(CROPS, gx, gy);
        if crops < 0.2 || roll >= hedge_probability(crops, self.lerp_grid(HEDGE, gx, gy)) {
            return;
        }
        let gap = self.rng.randf();
        let tree = self.rng.randf();
        let jitter_x = self.rng.range(-0.07, 0.07);
        let jitter_y = self.rng.range(-0.07, 0.07);
        if gap < HEDGE_GAP || self.excluded(pos.0, pos.1) {
            return;
        }
        if self.map.river_sd_at(pos.0, pos.1) < RIVER_CLEARANCE {
            return;
        }
        let (rx0, ry0, rx1, ry1) = self.rect;
        let px = (pos.0 + jitter_x).clamp(rx0, rx1 - 0.001);
        let py = (pos.1 + jitter_y).clamp(ry0, ry1 - 0.001);
        let ground = req.height_world_at(self.map, px, py);
        if ground <= 0.0 {
            return;
        }
        let ground = req.display_ground(self.map, px, py, ground);
        let yaw = (-(next.1 - pos.1)).atan2(next.0 - pos.0) + self.rng.range(-0.15, 0.15);
        let hedge_tree = req
            .species
            .as_deref()
            .map_or(HEDGE_TREE, |t| t.dist.hedge_tree);
        if tree < hedge_tree {
            let tree_yaw = self.rng.randf() * TAU;
            if let Some(table) = req.species.as_deref() {
                // Lot HB4: hedge trees of the local biome (role "isolated").
                let roll = self.rng.randf();
                let (b, conifer) = self.biome_and_conifer(table, gx, gy);
                let altitude = self.map.height_m_at(px, py);
                let sd = self.map.river_sd_at(px, py);
                if let Some(sp) = table.pick(ROLE_ISOLATED, b, altitude, sd, conifer, roll) {
                    self.push_species(table, sp, px, ground, py, tree_yaw, 0.78);
                }
                return;
            }
            self.push(KIND_OAK, px, ground, py, tree_yaw, 0.78);
        } else {
            self.push(KIND_HEDGE, px, ground, py, yaw, 1.0);
        }
    }

    /// `VegetationTileJob._make_instance`.
    fn push(&mut self, kind: usize, x: f64, ground: f64, y: f64, yaw: f64, scale_factor: f64) {
        let rng = &mut self.rng;
        let (height, width, tint): (f64, f64, [f64; 3]);
        match kind {
            KIND_CONIFER => {
                height = rng.range(1.3, 2.1);
                width = height * rng.range(0.85, 1.1);
                let b = rng.range(0.8, 1.15);
                tint = [b * rng.range(0.9, 1.05), b, b * rng.range(0.95, 1.1)];
            }
            KIND_HEDGE => {
                height = rng.range(0.3, 0.46);
                width = rng.range(0.72, 0.95);
                let b = rng.range(1.0, 1.3);
                let warm = rng.randf() > 0.7;
                tint = [
                    b * if warm { 1.12 } else { 1.0 },
                    b,
                    b * if warm { 0.78 } else { 0.9 },
                ];
            }
            KIND_BEECH => {
                height = rng.range(1.35, 1.95);
                width = height * rng.range(0.78, 0.98);
                let b = rng.range(0.88, 1.12);
                tint = [b * rng.range(0.95, 1.05), b, b * rng.range(0.9, 1.0)];
            }
            _ => {
                height = rng.range(1.1, 1.7);
                width = height * rng.range(0.95, 1.3);
                let b = rng.range(0.82, 1.18);
                let warm = rng.randf();
                tint = if warm > 0.9 {
                    [b * 1.35, b * 1.15, b * 0.7]
                } else if warm > 0.7 {
                    [b * 1.12, b * 1.08, b * 0.85]
                } else {
                    [b * rng.range(0.9, 1.02), b, b * rng.range(0.9, 1.05)]
                };
            }
        }
        self.emit(kind, x, ground, y, yaw, scale_factor, height, width, tint);
    }

    /// Lot HB4: biome (nearest coarse cell, default on sea / missing grid) and conifer share.
    fn biome_and_conifer(&self, table: &SpeciesTable, gx: f64, gy: f64) -> (usize, f64) {
        let side = self.req.side;
        let default = table.dist.default_biome;
        let b = if self.req.biome.len() >= side * side {
            let i = (gx.round().max(0.0) as usize).min(side - 1);
            let j = (gy.round().max(0.0) as usize).min(side - 1);
            match self.req.biome[j * side + i] as usize {
                0 => default,
                b => b.min(species::BIOME_COUNT - 1),
            }
        } else {
            default
        };
        (b, self.lerp_grid(CONIFER, gx, gy).clamp(0.0, 1.0))
    }

    /// Lot HB4: 1 inside the orchard ring around a settlement clearing (`ring` px wide beyond
    /// its exclusion radius), fading to 0 at its outer edge.
    fn village_ring(&self, x: f64, y: f64, ring: f64) -> f64 {
        if ring <= 0.0 {
            return 0.0;
        }
        let mut best = f64::INFINITY;
        for &(ex, ey, r) in &self.req.exclusions {
            let d = ((x - ex).powi(2) + (y - ey).powi(2)).sqrt() - r;
            best = best.min(d);
        }
        if best <= 0.0 {
            return 0.0;
        }
        1.0 - smoothstep(0.5 * ring, ring, best)
    }

    /// Lot HB4: role of a candidate (forest core or edge, riparian, orchard, isolated, scrub),
    /// then its species; same rejection tests as the V4 scatter.
    #[allow(clippy::too_many_arguments)]
    fn species_candidate(
        &mut self,
        table: &SpeciesTable,
        x: f64,
        y: f64,
        roll: f64,
        roll_kind: f64,
        forest_raw: f64,
        yaw: f64,
    ) {
        let req = self.req;
        let dist = &table.dist;
        let gx = (x - req.origin.0) / req.coarse_step;
        let gy = (y - req.origin.1) / req.coarse_step;
        let tree_roll = self.rng.randf();
        let (b, conifer) = self.biome_and_conifer(table, gx, gy);
        let forest = (forest_raw * table.biome(b, B_FOREST)).clamp(0.0, 1.0);
        let sd = self.map.river_sd_at(x, y);
        if sd < RIVER_CLEARANCE {
            return;
        }
        let core = forest >= dist.massif_core;
        let fill = if core {
            dist.massif_fill
        } else {
            dist.edge_fill
        };
        let mut role = None;
        let mut scale_factor = 1.0;
        if roll < forest * fill {
            role = Some(if core { ROLE_MASSIF } else { ROLE_EDGE });
            scale_factor = 1.0 + CANOPY_SPREAD * smoothstep(0.45, 0.9, forest);
        } else if sd < RIVER_CLEARANCE + table.biome(b, B_RIPARIAN_PX)
            && roll < table.biome(b, B_RIPARIAN) * (1.0 - forest)
        {
            role = Some(ROLE_RIPARIAN);
            scale_factor = 0.95;
        } else {
            let crops = self.lerp_grid(CROPS, gx, gy);
            let ring = self.village_ring(x, y, table.biome(b, B_ORCHARD_RING));
            let p_orchard =
                table.biome(b, B_ORCHARD) * ring + table.biome(b, B_ORCHARD_OPEN) * crops;
            if crops > 0.3 && p_orchard > 0.0 && table.parcel_roll(x, y) < p_orchard {
                if roll < dist.orchard_fill {
                    role = Some(ROLE_ORCHARD);
                    scale_factor = 0.9;
                }
            } else if crops > 0.15 {
                // Copses: only the core of the grove noise patches, densely filled.
                let grove = smoothstep(dist.grove_core, 1.0, self.lerp_grid(GROVE, gx, gy));
                let hedge = self.lerp_grid(HEDGE, gx, gy);
                let p = crops
                    * (grove * table.biome(b, B_GROVE)
                        + table.biome(b, B_ISOLATED)
                            * (1.0 + dist.hedge_boost * hedge + dist.village_boost * ring));
                if roll < p {
                    role = Some(ROLE_ISOLATED);
                    scale_factor = 0.9;
                }
            }
            if role.is_none() {
                let scrub = table.biome(b, B_SCRUB);
                if scrub > 0.0 && roll < scrub * (1.0 - forest_raw) * (1.2 - crops).clamp(0.0, 1.0)
                {
                    role = Some(ROLE_SCRUB);
                }
            }
        }
        let Some(role) = role else {
            return;
        };
        if self.excluded(x, y) || !self.has_point(x, y) || self.in_corridor(x, y) {
            return;
        }
        let ground = req.height_world_at(self.map, x, y);
        if ground <= 0.0 {
            return;
        }
        let species_roll = if roll_kind < dist.stand_share {
            table.stand_roll(x, y)
        } else {
            tree_roll
        };
        let altitude = self.map.height_m_at(x, y);
        let pick = table
            .pick(role, b, altitude, sd, conifer, species_roll)
            .or_else(|| {
                (role == ROLE_MASSIF)
                    .then(|| table.pick(ROLE_EDGE, b, altitude, sd, conifer, species_roll))
                    .flatten()
            });
        let Some(sp) = pick else {
            return;
        };
        let ground = req.display_ground(self.map, x, y, ground);
        self.push_species(table, sp, x, ground, y, yaw, scale_factor);
    }

    /// Lot HB4: one tree of species `sp` (size range of the catalogue, generic tint); the atlas
    /// row and season class are encoded in the tint (`CUSTOM_STRIDE`).
    #[allow(clippy::too_many_arguments)]
    fn push_species(
        &mut self,
        table: &SpeciesTable,
        sp: usize,
        x: f64,
        ground: f64,
        y: f64,
        yaw: f64,
        scale_factor: f64,
    ) {
        let rng = &mut self.rng;
        let height = rng.range(table.height[2 * sp] as f64, table.height[2 * sp + 1] as f64);
        let width = height * rng.range(table.width[2 * sp] as f64, table.width[2 * sp + 1] as f64);
        let b = rng.range(0.85, 1.15);
        let mut tint = [b * rng.range(0.93, 1.05), b, b * rng.range(0.92, 1.06)];
        tint[0] += (CUSTOM_STRIDE * (sp as f32 + 1.0)) as f64;
        tint[1] += (CUSTOM_STRIDE * (table.season[sp] as f32 + 1.0)) as f64;
        let kind = (table.kind[sp].clamp(0, KIND_CONIFER as i32)) as usize;
        self.emit(kind, x, ground, y, yaw, scale_factor, height, width, tint);
    }

    /// Transform, seed and slot of one instance (`VegetationTileJob._make_instance`).
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        kind: usize,
        x: f64,
        ground: f64,
        y: f64,
        yaw: f64,
        scale_factor: f64,
        mut height: f64,
        mut width: f64,
        tint: [f64; 3],
    ) {
        let rng = &mut self.rng;
        let tree_scale = self.req.tree_scale;
        height *= tree_scale
            * if scale_factor <= 1.0 {
                scale_factor
            } else {
                1.0 + (scale_factor - 1.0) * 0.35
            };
        width *= tree_scale * scale_factor;
        let depth = if kind == KIND_HEDGE {
            width * 0.62
        } else {
            width
        };
        // Basis(UP, yaw) * Basis(X, tilt), then scaled_local(width, height, depth).
        let tilt = rng.range(-0.06, 0.06);
        let (sy, cy) = yaw.sin_cos();
        let (st, ct) = tilt.sin_cos();
        let col_x = [cy * width, 0.0, -sy * width];
        let col_y = [sy * st * height, ct * height, cy * st * height];
        let col_z = [sy * ct * depth, -st * depth, cy * ct * depth];
        let origin = [x, ground - GROUND_SINK * height, y];
        let mut transform = [0.0f32; 12];
        for row in 0..3 {
            transform[row * 4] = col_x[row] as f32;
            transform[row * 4 + 1] = col_y[row] as f32;
            transform[row * 4 + 2] = col_z[row] as f32;
            transform[row * 4 + 3] = origin[row] as f32;
        }
        let seed = rng.randf();
        if seed < 1.0 - self.keep {
            return; // SZ4b: thinned detail cell (stream consumed as for `keep` = 1)
        }
        let slot = self.slot(kind, x, y);
        self.raw[slot].push(Instance {
            seed,
            transform,
            tint: tint.map(|c| c as f32),
        });
    }
}

/// `VegetationTileJob._pack`: sorted by decreasing seed, seed replaced by the normalised rank.
fn pack(mut items: Vec<Instance>) -> Vec<f32> {
    items.sort_by(|a, b| b.seed.total_cmp(&a.seed));
    let count = items.len();
    let mut buffer = Vec::with_capacity(count * FLOATS_PER_INSTANCE);
    for (i, item) in items.iter().enumerate() {
        buffer.extend_from_slice(&item.transform);
        buffer.extend_from_slice(&item.tint);
        buffer.push((1.0 - (i as f64 + 0.5) / count as f64) as f32);
    }
    buffer
}

/// `VegetationTileJob.reground`: re-seats packed instances on `req.ground` (same sink as
/// `push`, height = length of the basis Y column). Buffers are returned unchanged without a
/// ground grid.
pub fn reground(buffers: &mut [Vec<f32>], req: &TileRequest, map: &MapRasters) {
    if matches!(req.ground, Ground::None) {
        return;
    }
    for buffer in buffers.iter_mut() {
        for item in buffer.as_chunks_mut::<FLOATS_PER_INSTANCE>().0 {
            let height =
                ((item[1] as f64).powi(2) + (item[5] as f64).powi(2) + (item[9] as f64).powi(2))
                    .sqrt();
            let ground = req.display_ground(map, item[3] as f64, item[11] as f64, 0.0);
            item[7] = (ground - GROUND_SINK * height) as f32;
        }
    }
}

/// Scatters one tile (scatter, hedges, pack).
pub fn scatter_tile(req: &TileRequest, map: &MapRasters) -> TileResult {
    let seed = (req.tile_index as u64)
        .wrapping_mul(0x9e3779b97f4a7c15)
        .wrapping_add(91711);
    let (rect, parts_side, keep) = match &req.detail {
        Some(area) => (
            area.rect,
            area.parts_side.clamp(1, MAX_DETAIL_PARTS_SIDE),
            area.keep.clamp(0.0, 1.0),
        ),
        None => (
            (
                req.origin.0,
                req.origin.1,
                req.origin.0 + req.size_px,
                req.origin.1 + req.size_px,
            ),
            PARTS_SIDE,
            1.0,
        ),
    };
    let slots = parts_side * parts_side * KIND_COUNT;
    let mut scatter = Scatter {
        req,
        map,
        rng: Rng::new(seed),
        raw: (0..slots).map(|_| Vec::new()).collect(),
        rect,
        parts_side,
        keep,
        corridors: req
            .detail
            .as_ref()
            .filter(|area| !area.corridors.is_empty())
            .map(|area| CorridorBins::new(rect, &area.corridors)),
    };
    if req.side >= 2 {
        scatter.run();
    }
    let counts = scatter.raw.iter().map(|items| items.len() as i32).collect();
    let buffers = scatter.raw.into_iter().map(pack).collect();
    TileResult { buffers, counts }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_map(level: u16) -> MapRasters {
        let (w, h) = (64usize, 64usize);
        let mut height = Vec::with_capacity(w * h * 2);
        for _ in 0..w * h {
            height.extend_from_slice(&level.to_le_bytes());
        }
        MapRasters {
            height,
            width: w,
            height_px: h,
            bpp: 2,
            little_endian: true,
            height_min_m: -200.0,
            height_max_m: 1000.0,
            ..Default::default()
        }
    }

    fn request(forest: f32, crops: f32, hedge: f32, region: f32) -> TileRequest {
        let side = 64 / 4 + 2;
        let n = side * side;
        TileRequest {
            tile_index: 3,
            origin: (0.0, 0.0),
            size_px: 64.0,
            spacing: 1.35,
            coarse_step: 4.0,
            tree_scale: 1.0,
            vertical_scale: 0.01,
            relief_gain: 0.0,
            relief_squash: 0.0,
            floor: Default::default(),
            coarse: [
                vec![forest; n],
                vec![crops; n],
                vec![0.0; n],
                vec![0.3; n],
                vec![hedge; n],
                vec![0.0; n],
                vec![region; n],
            ],
            side,
            exclusions: Vec::new(),
            ground: Ground::None,
            detail: None,
            biome: Vec::new(),
            species: None,
        }
    }

    /// Species rows decoded from the packed buffers (`CUSTOM_STRIDE` encoding), per row.
    fn species_rows(result: &TileResult) -> std::collections::BTreeMap<i32, usize> {
        let mut rows = std::collections::BTreeMap::new();
        for buffer in &result.buffers {
            for item in buffer.as_chunks::<FLOATS_PER_INSTANCE>().0 {
                let row = (item[12] / CUSTOM_STRIDE).floor() as i32 - 1;
                *rows.entry(row).or_insert(0) += 1;
            }
        }
        rows
    }

    /// Trees of a result (hedge bushes excluded).
    fn trees(result: &TileResult) -> i32 {
        let slots = result.counts.iter().enumerate();
        slots
            .filter(|(slot, _)| slot % KIND_COUNT != KIND_HEDGE)
            .map(|(_, c)| c)
            .sum()
    }

    fn species_request(biome: f32, forest: f32, crops: f32) -> TileRequest {
        let mut req = request(forest, crops, 0.0, 1.0);
        req.biome = vec![biome; req.side * req.side];
        req.species = Some(Arc::new(species::tests::table()));
        req
    }

    #[test]
    fn species_forest_is_dense_encoded_and_mixed() {
        let map = flat_map(30000); // ~ 260 m
        let result = scatter_tile(&species_request(2.0, 1.0, 0.0), &map);
        let rows = species_rows(&result);
        let total: usize = rows.values().sum();
        assert!(total > 1500, "dense massif: {total}");
        assert!(!rows.contains_key(&-1), "every tree carries a species row");
        // low altitude: oak only among massif species (fir is above 500 m)
        assert_eq!(rows.keys().copied().collect::<Vec<_>>(), vec![0]);
        let high = flat_map(60000); // ~ 900 m: oak and fir
        let rows = species_rows(&scatter_tile(&species_request(6.0, 1.0, 0.0), &high));
        assert!(rows.len() >= 2, "{rows:?}");
    }

    #[test]
    fn species_steppe_and_fields_stay_sparse() {
        let map = flat_map(30000);
        let forest = scatter_tile(&species_request(2.0, 1.0, 0.0), &map);
        let steppe = scatter_tile(&species_request(4.0, 1.0, 0.0), &map);
        let fields = scatter_tile(&species_request(2.0, 0.0, 1.0), &map);
        let count = trees;
        assert!(
            count(&steppe) * 5 < count(&forest),
            "steppe {}",
            count(&steppe)
        );
        // open fields: far fewer trees than the V4 scatter (hedge trees and a few isolated ones)
        let mut legacy = species_request(2.0, 0.0, 1.0);
        legacy.species = None;
        let legacy = count(&scatter_tile(&legacy, &map));
        assert!(
            count(&fields) * 4 < legacy * 3,
            "fields {} vs {legacy}",
            count(&fields)
        );
    }

    #[test]
    fn species_orchards_ring_the_villages() {
        let map = flat_map(30000);
        let mut req = species_request(2.0, 0.0, 1.0);
        let bare = trees(&scatter_tile(&req, &map));
        req.exclusions = vec![(32.0, 32.0, 6.0)];
        let result = scatter_tile(&req, &map);
        let with_village = trees(&result);
        assert!(
            with_village > bare + 15,
            "orchards {with_village} vs {bare}"
        );
        for buffer in &result.buffers {
            for item in buffer.as_chunks::<FLOATS_PER_INSTANCE>().0 {
                let d = ((item[3] - 32.0).powi(2) + (item[11] - 32.0).powi(2)).sqrt();
                assert!(d >= 6.0, "no tree in the clearing");
            }
        }
    }

    #[test]
    fn detail_cell_is_dense_thinned_and_nested() {
        let map = flat_map(30000);
        let mut req = request(1.0, 0.0, 1.0, 1.0);
        req.spacing = 0.05;
        req.tile_index = 77;
        let area = |keep: f64| DetailArea {
            rect: (16.0, 20.0, 20.0, 24.0),
            keep,
            parts_side: 4,
            corridors: Vec::new(),
        };
        req.detail = Some(area(1.0));
        let full = scatter_tile(&req, &map);
        assert_eq!(full.counts.len(), 16 * KIND_COUNT);
        let total: i32 = full.counts.iter().sum();
        // 80 × 80 candidates, roll < 0.9 in a full forest, no hedges.
        assert!(total > 5200 && total < 6300, "{total}");
        assert_eq!(full.counts[KIND_HEDGE], 0);
        for buffer in &full.buffers {
            for item in buffer.chunks(FLOATS_PER_INSTANCE) {
                assert!((16.0..20.0).contains(&item[3]) && (20.0..24.0).contains(&item[11]));
            }
        }
        req.detail = Some(area(0.25));
        let thin = scatter_tile(&req, &map);
        let kept: i32 = thin.counts.iter().sum();
        assert!(
            (kept as f64 - total as f64 * 0.25).abs() < total as f64 * 0.04,
            "{kept}"
        );
        // Same stream: every thinned instance is one of the full cell (same origin).
        let origins: std::collections::HashSet<(u32, u32)> = full
            .buffers
            .iter()
            .flat_map(|b| b.chunks(FLOATS_PER_INSTANCE))
            .map(|i| (i[3].to_bits(), i[11].to_bits()))
            .collect();
        for buffer in &thin.buffers {
            for item in buffer.chunks(FLOATS_PER_INSTANCE) {
                assert!(origins.contains(&(item[3].to_bits(), item[11].to_bits())));
            }
        }
    }

    #[test]
    fn detail_corridors_stay_clear() {
        let map = flat_map(30000);
        let mut req = request(1.0, 0.0, 1.0, 1.0);
        req.spacing = 0.05;
        // A river across the cell (y = 22, half width 0.2) and a road along x = 17.
        req.detail = Some(DetailArea {
            rect: (16.0, 20.0, 20.0, 24.0),
            keep: 1.0,
            parts_side: 2,
            corridors: vec![
                [15.0, 22.0, 21.0, 22.0, 0.2],
                [17.0, 19.0, 17.0, 25.0, 0.05],
            ],
        });
        let result = scatter_tile(&req, &map);
        let mut total = 0;
        for buffer in &result.buffers {
            for item in buffer.chunks(FLOATS_PER_INSTANCE) {
                total += 1;
                assert!(
                    (item[11] - 22.0).abs() >= 0.2,
                    "tree in the river at {}",
                    item[11]
                );
                assert!(
                    (item[3] - 17.0).abs() >= 0.05,
                    "tree on the road at {}",
                    item[3]
                );
            }
        }
        assert!(total > 4000, "{total}");
    }

    #[test]
    fn hash_matches_gdscript_reference_values() {
        // GDScript: VegetationFields.hash01(0), hash01(12345), hash01(-77).
        let h = |n: i64| {
            let mut h = (n * 1103515245 + 12345) & 0x7fffffff;
            h = ((h ^ (h >> 13)) * 1274126177) & 0x7fffffff;
            (h % 100000) as f64 / 100000.0
        };
        for n in [0, 1, 12345, -77, 104729 * 3 + 31] {
            assert_eq!(hash01(n), h(n));
            assert!((0.0..1.0).contains(&hash01(n)));
        }
    }

    #[test]
    fn dense_forest_fills_tile_and_is_deterministic() {
        let map = flat_map(40000);
        let req = request(1.0, 0.0, 0.0, 1.0);
        let a = scatter_tile(&req, &map);
        let b = scatter_tile(&req, &map);
        assert_eq!(a.counts, b.counts);
        assert_eq!(a.buffers, b.buffers);
        let trees: i32 = a.counts.iter().sum();
        let cells = (64.0f64 / 1.35).ceil().powi(2);
        // roll < 0.9 on in-rect cells (the last column overflows by up to one spacing).
        assert!(trees as f64 > cells * 0.8 && (trees as f64) < cells * 0.92);
        for (slot, buffer) in a.buffers.iter().enumerate() {
            assert_eq!(buffer.len(), a.counts[slot] as usize * FLOATS_PER_INSTANCE);
            if slot % KIND_COUNT == KIND_HEDGE || slot % KIND_COUNT == KIND_CONIFER {
                assert!(buffer.is_empty());
            }
            let n = a.counts[slot] as usize;
            for i in 0..n {
                let rank = buffer[i * FLOATS_PER_INSTANCE + 15];
                assert!((rank - (1.0 - (i as f32 + 0.5) / n as f32)).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn sea_rivers_and_exclusions_stay_empty() {
        let sea = flat_map(0);
        assert_eq!(
            scatter_tile(&request(1.0, 1.0, 1.0, 1.0), &sea)
                .counts
                .iter()
                .sum::<i32>(),
            0
        );
        let mut river = flat_map(40000);
        river.river = vec![128; 64 * 64];
        river.river_width = 64;
        river.river_height = 64;
        assert_eq!(
            scatter_tile(&request(1.0, 1.0, 1.0, 1.0), &river)
                .counts
                .iter()
                .sum::<i32>(),
            0
        );
        let map = flat_map(40000);
        let mut req = request(1.0, 1.0, 1.0, 1.0);
        req.exclusions.push((32.0, 32.0, 100.0));
        assert_eq!(scatter_tile(&req, &map).counts.iter().sum::<i32>(), 0);
    }

    #[test]
    fn bocage_plants_hedges_along_parcel_edges() {
        let map = flat_map(40000);
        let open = scatter_tile(&request(0.0, 1.0, 1.0, 1.0), &map);
        let hedges: i32 = (0..PARTS)
            .map(|part| open.counts[part * KIND_COUNT + KIND_HEDGE])
            .sum();
        assert!(hedges > 500, "hedges {hedges}");
        // Region near the boundary (|region| < 0.02): the lane stays clear.
        let lane = scatter_tile(&request(0.0, 1.0, 1.0, 0.01), &map);
        assert_eq!(lane.counts[KIND_HEDGE], 0);
        // Hedge basis: height column ~0.3-0.46 × tree_scale, origin sunk below ground.
        let buffer = &open.buffers[KIND_HEDGE];
        let height = (buffer[1].powi(2) + buffer[5].powi(2) + buffer[9].powi(2)).sqrt();
        assert!((0.29..0.47).contains(&height));
        let ground = map.height_m_at(buffer[3] as f64, buffer[11] as f64) * 0.01;
        assert!((buffer[7] as f64 - (ground - GROUND_SINK * height as f64)).abs() < 1e-4);
    }

    #[test]
    fn relief_gain_lifts_ground_above_the_valley_floor() {
        let map = flat_map(40000);
        let h_m = map.height_m_at(10.0, 10.0);
        let mut req = request(1.0, 0.0, 0.0, 1.0);
        req.relief_gain = 2.0;
        req.floor = std::sync::Arc::new(ReliefFloor {
            data: vec![(h_m - 100.0) as f32; 4],
            side: (2, 2),
            cell: 64.0,
            ..Default::default()
        });
        let lifted = 0.01 * (h_m + 2.0 * 100.0);
        assert!((req.height_world_at(&map, 10.0, 10.0) - lifted).abs() < 1e-6);
        let result = scatter_tile(&req, &map);
        let buffer = result.buffers.iter().find(|b| !b.is_empty()).unwrap();
        let height = (buffer[1].powi(2) + buffer[5].powi(2) + buffer[9].powi(2)).sqrt() as f64;
        assert!((buffer[7] as f64 - (lifted - 0.08 * height)).abs() < 1e-4);
    }

    #[test]
    fn mountain_squash_lowers_ground_above_the_base() {
        let map = flat_map(40000);
        let h_m = map.height_m_at(10.0, 10.0);
        let mut req = request(1.0, 0.0, 0.0, 1.0);
        req.relief_gain = 0.5;
        req.relief_squash = 0.8;
        req.floor = std::sync::Arc::new(ReliefFloor {
            data: vec![(h_m - 100.0) as f32; 4],
            base: vec![(h_m - 1000.0) as f32; 4],
            squash: vec![0.5; 4],
            side: (2, 2),
            cell: 64.0,
        });
        // s·(h − K·(h − base) + g·(1 − K)·(h − floor)), K = c·k, as `MapData.display_height_fields`.
        let expected = 0.01 * (h_m - 0.4 * 1000.0 + 0.5 * 0.6 * 100.0);
        assert!((req.height_world_at(&map, 10.0, 10.0) - expected).abs() < 1e-6);
        // Below the base: unchanged.
        assert!(
            (req.display_height(h_m - 2000.0, 10.0, 10.0) - 0.01 * (h_m - 2000.0)).abs() < 1e-9
        );
    }

    #[test]
    fn reground_moves_only_the_origin_height() {
        let map = flat_map(40000);
        let req = request(1.0, 0.0, 0.0, 1.0);
        let mut buffers = scatter_tile(&req, &map).buffers;
        let before = buffers.clone();
        let mut lifted = request(1.0, 0.0, 0.0, 1.0);
        lifted.ground = Ground::Grid {
            heights: vec![2.0; 4 * 4],
            side: 4,
            unit: 32.0,
        };
        reground(&mut buffers, &lifted, &map);
        for (a, b) in before.iter().zip(&buffers) {
            for (x, y) in a.chunks(16).zip(b.chunks(16)) {
                let height = (x[1].powi(2) + x[5].powi(2) + x[9].powi(2)).sqrt();
                assert!((y[7] - (2.0 - 0.08 * height)).abs() < 1e-4);
                assert_eq!(x[..7], y[..7]);
                assert_eq!(x[8..], y[8..]);
            }
        }
    }

    #[test]
    fn quadtree_pages_drive_display_ground() {
        let map = flat_map(40000);
        let mut req = request(1.0, 0.0, 0.0, 1.0);
        let mut page = Vec::with_capacity(PAGE_PX * PAGE_PX * 2);
        for _ in 0..PAGE_PX * PAGE_PX {
            page.extend_from_slice(&65535u16.to_le_bytes());
        }
        let mut pages = std::collections::HashMap::new();
        pages.insert(0i64, std::sync::Arc::new(page));
        req.ground = Ground::Pages {
            pages,
            max_level: 3,
            h_min: 0.0,
            h_range: 500.0,
        };
        let result = scatter_tile(&req, &map);
        let buffer = result.buffers.iter().find(|b| !b.is_empty()).unwrap();
        let height = (buffer[1].powi(2) + buffer[5].powi(2) + buffer[9].powi(2)).sqrt();
        assert!((buffer[7] - (5.0 - 0.08 * height)).abs() < 1e-3);
    }
}
