//! Coarse vegetation masks of a tile (port of `VegetationMask.sample` and
//! `VegetationTileJob._sample_coarse`): forest, crops, conifer share, beech share, grove noise,
//! biome per coarse cell. Hedges and the parcel region are zero: the generalised tree style
//! draws no aligned hedges (`drop_hedges`), the scatter takes isolated trees instead.
//!
//! Differences with Godot: the low-frequency noise is a gradient-noise fBm (not FastNoiseLite
//! simplex), and the wetlands raster (marsh, ponds) removes forest and crops.

use std::sync::atomic::{AtomicU64, Ordering};

use super::data::{VegetationWorld, style};

/// Coarse cells cleared by the wetlands raster, since the start.
pub static WET_CLEARED: AtomicU64 = AtomicU64::new(0);

/// Coarse grids of a tile, `side x side`.
pub struct Coarse {
    pub side: usize,
    /// Forest, crops, conifer, beech, hedge, grove, region.
    pub grids: [Vec<f32>; 7],
    pub biome: Vec<f32>,
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash(ix: i32, iy: i32, seed: u32) -> u32 {
    let mut h = (ix as u32).wrapping_mul(0x9E37_79B1)
        ^ (iy as u32).wrapping_mul(0x85EB_CA77)
        ^ seed.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ (h >> 15)
}

fn gradient(h: u32, dx: f32, dy: f32) -> f32 {
    match h & 7 {
        0 => dx + dy,
        1 => dx - dy,
        2 => -dx + dy,
        3 => -dx - dy,
        4 => dx,
        5 => -dx,
        6 => dy,
        _ => -dy,
    }
}

/// Gradient noise in about [-1, 1].
fn perlin(x: f32, y: f32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (ix, iy) = (x0 as i32, y0 as i32);
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let (u, v) = (fade(fx), fade(fy));
    let g =
        |dx: i32, dy: i32| gradient(hash(ix + dx, iy + dy, seed), fx - dx as f32, fy - dy as f32);
    let top = g(0, 0) + (g(1, 0) - g(0, 0)) * u;
    let bottom = g(0, 1) + (g(1, 1) - g(0, 1)) * u;
    ((top + (bottom - top) * v) * 1.41).clamp(-1.0, 1.0)
}

/// Fractal noise (`make_noise`): frequency `freq`, `octaves`, gain; normalised to [-1, 1].
pub fn fbm(x: f32, y: f32, seed: u32, freq: f32, octaves: u32, gain: f32) -> f32 {
    let (mut amp, mut sum, mut norm, mut fr) = (1.0f32, 0.0f32, 0.0f32, freq);
    for o in 0..octaves {
        sum += amp * perlin(x * fr, y * fr, seed.wrapping_add(o * 101));
        norm += amp;
        amp *= gain;
        fr *= 2.0;
    }
    sum / norm
}

const NOISE_SEED: u32 = 1337;
const GROVE_SEED: u32 = 4242;

/// One sample of the masks.
pub struct Sample {
    pub forest: f32,
    pub crops: f32,
    pub conifer: f32,
    pub beech: f32,
}

impl VegetationWorld {
    fn height_m(&self, x: f64, y: f64) -> f32 {
        self.heights.height_m_at(x as f32, y as f32)
    }

    pub(crate) fn treeline_m(&self, y: f64) -> f32 {
        let north = 1.0 - (y / self.size.1.max(1.0)).clamp(0.0, 1.0) as f32;
        let t = smoothstep(0.55, 0.95, north);
        2100.0 + (900.0 - 2100.0) * t
    }

    fn slope(&self, x: f64, y: f64) -> f32 {
        let dx = self.height_m(x + 2.0, y) - self.height_m(x - 2.0, y);
        let dy = self.height_m(x, y + 2.0) - self.height_m(x, y - 2.0);
        (dx * dx + dy * dy).sqrt() / (4.0 * self.meters_per_px)
    }

    /// `VegetationMask.sample`.
    pub fn sample(&self, x: f64, y: f64) -> Sample {
        let mut out = Sample {
            forest: 0.0,
            crops: 0.0,
            conifer: 0.0,
            beech: 0.0,
        };
        let height_m = self.height_m(x, y);
        let land = if self.land.is_empty() {
            true
        } else {
            self.land.texel(self.size, x, y, 0) > 0.5
        };
        if height_m <= 1.0 || !land {
            return out;
        }
        let treeline = self.treeline_m(y);
        let altitude_factor = 1.0 - smoothstep(treeline - 350.0, treeline, height_m);
        let slope_factor = 1.0 - smoothstep(0.35, 0.8, self.slope(x, y));
        let (xf, yf) = (x as f32, y as f32);
        let n = fbm(xf, yf, NOISE_SEED, 1.0 / 48.0, 4, 0.55);
        let raw_forest = self.splat_forest.texel(self.size, x, y, 0);
        let mut forest = smoothstep(self.cover.0, self.cover.1, raw_forest);
        let mut crops = self.splat_crops.texel(self.size, x, y, 0);
        // Wetlands: marsh and ponds are not planted.
        if !self.wet.is_empty() {
            let wet = self
                .wet
                .texel(self.size, x, y, 0)
                .max(self.wet.texel(self.size, x, y, 1));
            let keep = 1.0 - smoothstep(0.3, 0.7, wet);
            if keep < 1.0 && forest + crops > 0.0 {
                WET_CLEARED.fetch_add(1, Ordering::Relaxed);
            }
            forest *= keep;
            crops *= keep;
        }
        out.forest = forest * altitude_factor * slope_factor;
        out.crops = crops * slope_factor;
        let north = 1.0 - (y / self.size.1.max(1.0)).clamp(0.0, 1.0) as f32;
        let conifer =
            smoothstep(treeline * 0.35, treeline * 0.7, height_m) + smoothstep(0.62, 0.85, north);
        out.conifer = (conifer + n * 0.25).clamp(0.0, 1.0);
        if !self.conifer.is_empty() {
            out.conifer = self.conifer.texel(self.size, x, y, 0);
        }
        let b = &self.beech;
        let patch = 48.0 / b.patch_px;
        let n2 = fbm(
            xf * patch + 5171.0,
            yf * patch - 3307.0,
            NOISE_SEED,
            1.0 / 48.0,
            4,
            0.55,
        );
        let south = smoothstep(
            0.55,
            0.78,
            (y / self.size.1.max(1.0)).clamp(0.0, 1.0) as f32,
        );
        let beech =
            0.2 + 0.7 * smoothstep(b.altitude_low_m, b.altitude_high_m, height_m) + 0.9 * n2
                - b.south_oak * south;
        out.beech = smoothstep(0.35, 0.65, beech);
        out
    }

    /// Biome of the map point (nearest; sea or unknown: the default biome).
    pub fn biome_at(&self, x: f64, y: f64) -> usize {
        if self.biomes.is_empty() {
            return self.default_biome;
        }
        let b = (self.biomes.texel(self.size, x, y, 0) * 255.0).round() as usize;
        if b > 0 && b < vegetation::species::BIOME_COUNT {
            b
        } else {
            self.default_biome
        }
    }

    /// `VegetationTileJob._sample_coarse` for the tile at `origin`.
    pub fn coarse(&self, origin: (f64, f64), size_px: f64, step: f64) -> Coarse {
        let side = (size_px / step) as usize + 2;
        let n = side * side;
        let mut grids: [Vec<f32>; 7] = std::array::from_fn(|_| vec![0.0; n]);
        let mut biome = vec![0.0f32; n];
        let mut k = 0;
        for j in 0..side {
            let y = origin.1 + j as f64 * step;
            for i in 0..side {
                let x = origin.0 + i as f64 * step;
                let s = self.sample(x, y);
                grids[0][k] = s.forest;
                grids[1][k] = s.crops;
                grids[2][k] = s.conifer;
                grids[3][k] = s.beech;
                let g = fbm(x as f32, y as f32, GROVE_SEED, 1.0 / 9.0, 2, 0.5);
                grids[5][k] = smoothstep(style::GROVE_LOW, style::GROVE_HIGH, g);
                biome[k] = self.biome_at(x, y) as f32;
                k += 1;
            }
        }
        Coarse { side, grids, biome }
    }
}
