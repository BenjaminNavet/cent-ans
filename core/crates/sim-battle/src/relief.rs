//! Realistic relief of the battlefields (lot R2, ADR 0020).
//!
//! The pre-R2 field (tilt, gaussian hills, river bed) is kept as the base and
//! its draws from the battle stream are unchanged; everything here comes from
//! a stream derived from the seed ([`BattleRng::derive`]), so the units, the
//! weather and every later draw of a battle stay where they were.
//!
//! Added on top of the base, with amplitudes and styles per province terrain
//! ([`ReliefStyle`]):
//! - large landforms: fractal gradient noise (fBm) with domain warping, mixed
//!   with ridged noise (crests and spurs) in hills and mountains;
//! - one or two valleys (talwegs) that drain towards the river when there is
//!   one, deepening downstream;
//! - breaks of slope: escarpments in hills and mountains, flights of banks
//!   (the lynchets of Picardy) in plains, heath and bocage;
//! - micro-undulations (1-2 m in the plains);
//! - a floodplain along the river, where the detail fades out;
//! - in hills and mountains, the defender's line is kept above the
//!   attacker's; slopes on the centre of both deployment lines are bounded.
//!
//! Woods and mud ([`shape_cover`]) become irregular clusters of discs: a
//! massif of overlapping lobes plus small copses, moved and grown along the
//! relief (mud and pools in the hollows, woods on steep slopes and crests).
//! The anchor disc of each zone keeps its index in `forests` / `mud`, the
//! extra lobes go to `forest_parts` / `mud_parts`.

use data_model::Terrain;

use crate::field::{River, Zone};
use crate::rng::BattleRng;
use crate::scale::{FieldSize, GRID_RESOLUTION};

/// Salt of the derived stream of the relief (lot R2).
pub(crate) const RELIEF_STREAM: u64 = 0x5232;

/// Style of the relief of a province terrain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReliefStyle {
    /// Amplitude (m, ±) of the large landforms.
    pub macro_amp: f64,
    /// Wavelength (m) of the large landforms.
    pub macro_wavelength: f64,
    /// Share of ridged noise (crests, spurs), 0-1.
    pub ridged: f64,
    /// Domain warp, in wavelengths.
    pub warp: f64,
    /// Amplitude (m, ±) of the micro-undulations.
    pub micro_amp: f64,
    /// Wavelength (m) of the micro-undulations.
    pub micro_wavelength: f64,
    /// Number of valleys (min, max).
    pub valleys: (u32, u32),
    /// Depth (m) of a valley at its mouth.
    pub valley_depth: f64,
    /// Half-width (m) of a valley.
    pub valley_width: f64,
    /// Number of breaks of slope (min, max).
    pub scarps: (u32, u32),
    /// Height (m) of a break of slope.
    pub scarp_height: f64,
    /// Half-width (m) of the steep part of a break of slope.
    pub scarp_width: f64,
    /// Breaks of slope come in flights of parallel banks (lynchets).
    pub flights: bool,
    /// Maximum slope (rise over run) on the centre of the deployment lines.
    pub line_slope: f64,
    /// Minimum rise (m) of the defender's line above the attacker's.
    pub defender_rise: f64,
}

impl ReliefStyle {
    pub fn of(terrain: Terrain) -> ReliefStyle {
        let base = ReliefStyle {
            macro_amp: 3.0,
            macro_wavelength: 700.0,
            ridged: 0.0,
            warp: 0.35,
            micro_amp: 1.2,
            micro_wavelength: 110.0,
            valleys: (1, 1),
            valley_depth: 4.0,
            valley_width: 90.0,
            scarps: (0, 2),
            scarp_height: 1.8,
            scarp_width: 8.0,
            flights: true,
            line_slope: 0.07,
            defender_rise: 0.0,
        };
        match terrain {
            // OM3 (ADR 0116): steppe and desert reuse the plains relief.
            Terrain::Plains | Terrain::Steppe | Terrain::Desert => base,
            Terrain::Heath => ReliefStyle {
                macro_amp: 4.5,
                macro_wavelength: 600.0,
                ridged: 0.25,
                warp: 0.45,
                micro_amp: 1.1,
                micro_wavelength: 90.0,
                valleys: (1, 2),
                valley_depth: 5.0,
                valley_width: 80.0,
                scarps: (0, 1),
                scarp_height: 2.5,
                line_slope: 0.08,
                ..base
            },
            Terrain::Bocage => ReliefStyle {
                macro_amp: 5.0,
                macro_wavelength: 520.0,
                ridged: 0.1,
                warp: 0.4,
                micro_wavelength: 100.0,
                valleys: (1, 2),
                valley_depth: 7.0,
                valley_width: 100.0,
                scarps: (1, 3),
                scarp_height: 1.6,
                scarp_width: 6.0,
                line_slope: 0.08,
                ..base
            },
            Terrain::Forest => ReliefStyle {
                macro_amp: 5.5,
                macro_wavelength: 560.0,
                ridged: 0.2,
                warp: 0.4,
                micro_amp: 1.2,
                micro_wavelength: 90.0,
                valleys: (1, 2),
                valley_depth: 6.0,
                scarps: (0, 1),
                scarp_height: 2.5,
                scarp_width: 10.0,
                flights: false,
                line_slope: 0.09,
                ..base
            },
            Terrain::Hills => ReliefStyle {
                macro_amp: 12.0,
                macro_wavelength: 650.0,
                ridged: 0.55,
                warp: 0.5,
                micro_amp: 1.6,
                micro_wavelength: 90.0,
                valleys: (1, 2),
                valley_depth: 12.0,
                valley_width: 120.0,
                scarps: (1, 2),
                scarp_height: 5.0,
                scarp_width: 12.0,
                flights: false,
                line_slope: 0.14,
                defender_rise: 8.0,
            },
            Terrain::Mountains => ReliefStyle {
                macro_amp: 26.0,
                macro_wavelength: 700.0,
                ridged: 0.8,
                warp: 0.55,
                micro_amp: 2.4,
                micro_wavelength: 90.0,
                valleys: (2, 2),
                valley_depth: 22.0,
                valley_width: 150.0,
                scarps: (1, 3),
                scarp_height: 9.0,
                scarp_width: 14.0,
                flights: false,
                line_slope: 0.2,
                defender_rise: 16.0,
            },
            Terrain::Marsh => ReliefStyle {
                macro_amp: 1.0,
                macro_wavelength: 600.0,
                warp: 0.3,
                micro_amp: 0.35,
                micro_wavelength: 70.0,
                valleys: (1, 2),
                valley_depth: 1.4,
                valley_width: 45.0,
                scarps: (0, 0),
                line_slope: 0.05,
                ..base
            },
        }
    }
}

// --- Noise --------------------------------------------------------------------------------

/// Sixteen unit gradients (every 22.5°), without trigonometry at run time.
const D: f64 = std::f64::consts::FRAC_1_SQRT_2;

const GRADIENTS: [(f64, f64); 16] = [
    (1.0, 0.0),
    (0.923_879_532_5, 0.382_683_432_4),
    (D, D),
    (0.382_683_432_4, 0.923_879_532_5),
    (0.0, 1.0),
    (-0.382_683_432_4, 0.923_879_532_5),
    (-D, D),
    (-0.923_879_532_5, 0.382_683_432_4),
    (-1.0, 0.0),
    (-0.923_879_532_5, -0.382_683_432_4),
    (-D, -D),
    (-0.382_683_432_4, -0.923_879_532_5),
    (0.0, -1.0),
    (0.382_683_432_4, -0.923_879_532_5),
    (D, -D),
    (0.923_879_532_5, -0.382_683_432_4),
];

fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn lattice_hash(seed: u64, ix: i64, iz: i64) -> u64 {
    let h = mix64(seed ^ (ix as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    mix64(h ^ (iz as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Gradient noise at (x, z) in lattice units, roughly in `[-1, 1]`.
pub fn gradient_noise(seed: u64, x: f64, z: f64) -> f64 {
    let (x0, z0) = (x.floor(), z.floor());
    let (fx, fz) = (x - x0, z - z0);
    let (ix, iz) = (x0 as i64, z0 as i64);
    let corner = |cx: i64, cz: i64, dx: f64, dz: f64| {
        let g = GRADIENTS[(lattice_hash(seed, ix + cx, iz + cz) & 15) as usize];
        g.0 * dx + g.1 * dz
    };
    let n00 = corner(0, 0, fx, fz);
    let n10 = corner(1, 0, fx - 1.0, fz);
    let n01 = corner(0, 1, fx, fz - 1.0);
    let n11 = corner(1, 1, fx - 1.0, fz - 1.0);
    let (u, v) = (fade(fx), fade(fz));
    (lerp(lerp(n00, n10, u), lerp(n01, n11, u), v) * 1.41).clamp(-1.0, 1.0)
}

/// Fractal sum of `octaves` octaves of gradient noise, roughly in `[-1, 1]`.
pub fn fbm(seed: u64, x: f64, z: f64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for octave in 0..octaves {
        let s = seed.wrapping_add(u64::from(octave).wrapping_mul(0x632B_E59B_D9B4_E019));
        // Each octave is rotated a little to hide the lattice.
        let (rx, rz) = (x * 0.8 - z * 0.6, x * 0.6 + z * 0.8);
        sum += amp * gradient_noise(s, rx * freq + 17.3, rz * freq - 9.1);
        norm += amp;
        amp *= 0.5;
        freq *= 2.03;
    }
    sum / norm
}

/// Ridged multifractal noise in `[0, 1]`: sharp crests where the noise
/// crosses zero, with detail concentrated on the crests.
pub fn ridged(seed: u64, x: f64, z: f64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm, mut weight) = (0.0, 1.0, 1.0, 0.0, 1.0);
    for octave in 0..octaves {
        let s = seed.wrapping_add(u64::from(octave).wrapping_mul(0x632B_E59B_D9B4_E019));
        let n = 1.0 - gradient_noise(s, x * freq + 3.7, z * freq + 11.9).abs();
        let n = n * n * weight;
        weight = (n * 1.6).clamp(0.0, 1.0);
        sum += n * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.1;
    }
    sum / norm
}

// --- Relief -------------------------------------------------------------------------------

/// A valley: a meandering polyline whose depth grows downstream.
struct Valley {
    points: Vec<(f64, f64)>,
    depth: f64,
    /// Share of the depth at the head (the mouth has the full depth).
    head: f64,
    half_width: f64,
}

impl Valley {
    /// (distance to the talweg, position along it 0-1).
    fn nearest(&self, x: f64, z: f64) -> (f64, f64) {
        let segments = (self.points.len() - 1) as f64;
        let mut best = (f64::MAX, 0.0);
        for (i, pair) in self.points.windows(2).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            let (dx, dz) = (b.0 - a.0, b.1 - a.1);
            let len2 = (dx * dx + dz * dz).max(1e-9);
            let t = (((x - a.0) * dx + (z - a.1) * dz) / len2).clamp(0.0, 1.0);
            let (px, pz) = (a.0 + dx * t, a.1 + dz * t);
            let d2 = (x - px).powi(2) + (z - pz).powi(2);
            if d2 < best.0 {
                best = (d2, (i as f64 + t) / segments);
            }
        }
        (best.0.sqrt(), best.1)
    }

    fn carve(&self, x: f64, z: f64) -> f64 {
        let (d, t) = self.nearest(x, z);
        if d >= self.half_width {
            return 0.0;
        }
        let profile = 1.0 - smoothstep(self.half_width * 0.18, self.half_width, d);
        self.depth * lerp(self.head, 1.0, t) * profile
    }
}

/// A break of slope: a step of `height` along a wobbly line, fading out at
/// both ends. The side the normal points to is raised.
struct Scarp {
    cx: f64,
    cz: f64,
    tangent: (f64, f64),
    length: f64,
    height: f64,
    half_width: f64,
    wobble: f64,
    seed: u64,
}

impl Scarp {
    fn step(&self, x: f64, z: f64) -> f64 {
        let (dx, dz) = (x - self.cx, z - self.cz);
        let along = dx * self.tangent.0 + dz * self.tangent.1;
        let half = self.length * 0.5;
        if along.abs() >= half {
            return 0.0;
        }
        let normal = (-self.tangent.1, self.tangent.0);
        let across = dx * normal.0
            + dz * normal.1
            + self.wobble * gradient_noise(self.seed, along / 140.0, 0.37);
        let fade = 1.0 - smoothstep(half * 0.6, half, along.abs());
        self.height * (smoothstep(-self.half_width, self.half_width, across) - 0.5) * fade
    }
}

fn draw_count(range: (u32, u32), stream: &mut BattleRng) -> u32 {
    range.0 + stream.below(range.1 - range.0 + 1)
}

/// `count` scaled by `factor` (EP1: more landforms on a larger field; the
/// same count on the standard field).
fn scaled(count: u32, factor: f64) -> u32 {
    (f64::from(count) * factor).round() as u32
}

/// x of a valley head, most often on a flank of the field.
fn flank_x(size: &FieldSize, stream: &mut BattleRng) -> f64 {
    let width = size.width;
    if stream.unit() < 0.7 {
        if stream.unit() < 0.5 {
            stream.range(60.0, 320.0)
        } else {
            stream.range(width - 320.0, width - 60.0)
        }
    } else {
        stream.range(60.0, width - 60.0)
    }
}

fn draw_valley(
    style: &ReliefStyle,
    size: &FieldSize,
    river: Option<&River>,
    stream: &mut BattleRng,
) -> Valley {
    let (width, depth) = (size.width, size.depth);
    let seed = stream.next_u64();
    let (start, end, head) = match river {
        Some(river) => {
            let x0 = flank_x(size, stream);
            let z0 = if stream.unit() < 0.5 {
                -60.0
            } else {
                depth + 60.0
            };
            let x1 = (x0 + stream.range(-180.0, 180.0)).clamp(40.0, width - 40.0);
            ((x0, z0), (x1, river.center_z(x1)), 0.35)
        }
        None if stream.unit() < 0.5 => {
            // Across a flank, from one edge to the other.
            let x0 = flank_x(size, stream);
            let x1 = (x0 + stream.range(-220.0, 220.0)).clamp(40.0, width - 40.0);
            let (z0, z1) = if stream.unit() < 0.5 {
                (-60.0, depth + 60.0)
            } else {
                (depth + 60.0, -60.0)
            };
            ((x0, z0), (x1, z1), 0.5)
        }
        None => {
            // Along the field, in the no man's land or behind a line.
            let (mid, back, front) = (
                size.center_z(),
                size.attacker_line_z(),
                size.defender_line_z(),
            );
            let z0 = match stream.below(3) {
                0 => stream.range(mid - 70.0, mid + 70.0),
                1 => stream.range(back - 190.0, back - 80.0),
                _ => stream.range(front + 80.0, front + 190.0),
            };
            let z1 = (z0 + stream.range(-120.0, 120.0)).clamp(40.0, depth - 40.0);
            let (x0, x1) = if stream.unit() < 0.5 {
                (-60.0, width + 60.0)
            } else {
                (width + 60.0, -60.0)
            };
            ((x0, z0), (x1, z1), 0.5)
        }
    };
    let meander = stream.range(20.0, 60.0);
    let (dx, dz) = (end.0 - start.0, end.1 - start.1);
    let len = (dx * dx + dz * dz).sqrt().max(1.0);
    let normal = (-dz / len, dx / len);
    let count = 24;
    let points = (0..=count)
        .map(|i| {
            let t = f64::from(i) / f64::from(count);
            // No meander at the ends: the valley reaches the river where drawn.
            let off =
                meander * gradient_noise(seed, t * 3.2, 0.61) * (t * (1.0 - t) * 4.0).min(1.0);
            (
                start.0 + dx * t + normal.0 * off,
                start.1 + dz * t + normal.1 * off,
            )
        })
        .collect();
    Valley {
        points,
        depth: style.valley_depth * stream.range(0.7, 1.15),
        head: if river.is_some() {
            head
        } else {
            stream.range(0.6, 1.0)
        },
        half_width: style.valley_width * stream.range(0.8, 1.2),
    }
}

fn draw_scarps(
    style: &ReliefStyle,
    terrain: Terrain,
    size: &FieldSize,
    stream: &mut BattleRng,
) -> Vec<Scarp> {
    let mut scarps = Vec::new();
    for _ in 0..scaled(draw_count(style.scarps, stream), size.area_ratio()) {
        let angle = stream.range(-0.6, 0.6);
        let tangent = (angle.cos(), angle.sin());
        // Hills and mountains: the defender's side (north) is most often the high side.
        let upward = if matches!(terrain, Terrain::Hills | Terrain::Mountains) {
            stream.unit() < 0.75
        } else {
            stream.unit() < 0.5
        };
        let sign = if upward { 1.0 } else { -1.0 };
        let cx = stream.range(200.0, size.width - 200.0);
        let cz = stream.range(120.0, size.depth - 120.0);
        let length = stream.range(300.0, 700.0);
        let flight = if style.flights {
            1 + stream.below(3)
        } else {
            1
        };
        let spacing = stream.range(40.0, 75.0);
        for k in 0..flight {
            let shift = f64::from(k) * spacing;
            scarps.push(Scarp {
                cx: cx - tangent.1 * shift,
                cz: cz + tangent.0 * shift,
                tangent: (tangent.0 * sign, tangent.1 * sign),
                length: length * stream.range(0.7, 1.0),
                height: style.scarp_height * stream.range(0.6, 1.2),
                half_width: style.scarp_width * stream.range(0.8, 1.3),
                wobble: stream.range(10.0, 35.0),
                seed: stream.next_u64(),
            });
        }
    }
    scarps
}

fn grid_dims(heights: &[f64], size: &FieldSize) -> (usize, usize) {
    let nx = size.nx();
    (nx, heights.len() / nx)
}

/// Mean height along the centre of a deployment line.
fn line_mean(heights: &[f64], size: &FieldSize, line: f64) -> f64 {
    let (nx, _) = grid_dims(heights, size);
    let iz = (line / GRID_RESOLUTION).round() as usize;
    let (x0, x1) = (
        ((size.center_x() - size.line_half()) / GRID_RESOLUTION) as usize,
        ((size.center_x() + size.line_half()) / GRID_RESOLUTION) as usize,
    );
    let row = &heights[iz * nx + x0..=iz * nx + x1];
    row.iter().sum::<f64>() / row.len() as f64
}

/// Adds the relief detail of `terrain` to the pre-R2 heights (row-major grid
/// of the field at [`GRID_RESOLUTION`]).
pub(crate) fn shape_relief(
    heights: &mut [f64],
    size: &FieldSize,
    terrain: Terrain,
    river: Option<&River>,
    stream: &mut BattleRng,
) {
    let style = ReliefStyle::of(terrain);
    let (nx, nz) = grid_dims(heights, size);
    let seeds: [u64; 5] = std::array::from_fn(|_| stream.next_u64());
    let valleys: Vec<Valley> = (0..scaled(draw_count(style.valleys, stream), size.sx()))
        .map(|_| draw_valley(&style, size, river, stream))
        .collect();
    let scarps = draw_scarps(&style, terrain, size, stream);
    let floodplain = style.macro_amp * 0.35 + 0.8;
    for iz in 0..nz {
        for ix in 0..nx {
            let (x, z) = (ix as f64 * GRID_RESOLUTION, iz as f64 * GRID_RESOLUTION);
            // Large landforms: warped fBm and ridged noise.
            let (px, pz) = (x / style.macro_wavelength, z / style.macro_wavelength);
            let wx = fbm(seeds[0], px * 0.7, pz * 0.7, 3) * style.warp;
            let wz = fbm(seeds[1], px * 0.7 + 5.2, pz * 0.7 + 1.3, 3) * style.warp;
            let (qx, qz) = (px + wx, pz + wz);
            let smooth = fbm(seeds[2], qx, qz, 5);
            let crest = ridged(seeds[3], qx * 0.9, qz * 0.9, 5) * 2.0 - 0.8;
            let landform = lerp(smooth, crest, style.ridged) * style.macro_amp;
            let micro = fbm(
                seeds[4],
                x / style.micro_wavelength,
                z / style.micro_wavelength,
                3,
            ) * style.micro_amp;
            let valley: f64 = valleys.iter().map(|v| v.carve(x, z)).sum();
            let step: f64 = scarps.iter().map(|s| s.step(x, z)).sum();
            let mut detail = landform + micro - valley + step;
            if let Some(river) = river {
                // The river runs in a floodplain: the detail fades out on its banks.
                let d = (z - river.center_z(x)).abs();
                detail = detail * smoothstep(15.0, 170.0, d)
                    - floodplain * (1.0 - smoothstep(20.0, 230.0, d));
            }
            heights[iz * nx + ix] += detail;
        }
    }
    soften_lines(heights, size, style.line_slope);
    if style.defender_rise > 0.0 {
        let (attacker_line, defender_line) = (size.attacker_line_z(), size.defender_line_z());
        let rise =
            line_mean(heights, size, defender_line) - line_mean(heights, size, attacker_line);
        if rise < style.defender_rise {
            let (low, high) = (attacker_line - 60.0, defender_line + 60.0);
            let span = smoothstep(low, high, defender_line) - smoothstep(low, high, attacker_line);
            let lift = (style.defender_rise - rise) / span;
            for iz in 0..nz {
                let ramp = lift * smoothstep(low, high, iz as f64 * GRID_RESOLUTION);
                for h in &mut heights[iz * nx..(iz + 1) * nx] {
                    *h += ramp;
                }
            }
        }
    }
}

/// Half-depth (m) of the shelf of a deployment line (full weight), and of
/// its blend into the surrounding relief.
const SHELF_HALF: f64 = 55.0;
const SHELF_BLEND: f64 = 135.0;

/// Lays each deployment line on a gentle shelf: across the line, the ground
/// keeps its mean height and a bounded grade; along the line, the profile is
/// smoothed until its slope is bounded. The shelf blends into the relief
/// beyond (a terrace on a hillside), so the high ground is kept.
fn soften_lines(heights: &mut [f64], size: &FieldSize, max_slope: f64) {
    let (nx, nz) = grid_dims(heights, size);
    let (center, half) = (size.center_x(), size.line_half());
    for line in [size.attacker_line_z(), size.defender_line_z()] {
        let rows: Vec<usize> = (0..nz)
            .filter(|&iz| (iz as f64 * GRID_RESOLUTION - line).abs() <= SHELF_HALF)
            .collect();
        // Per column: mean height and grade across the line (least squares).
        let mut level = vec![0.0; nx];
        let mut grade = vec![0.0; nx];
        for ix in 0..nx {
            let n = rows.len() as f64;
            let mean_h = rows.iter().map(|&iz| heights[iz * nx + ix]).sum::<f64>() / n;
            let (mut cov, mut var) = (0.0, 0.0);
            for &iz in &rows {
                let dz = iz as f64 * GRID_RESOLUTION - line;
                cov += dz * (heights[iz * nx + ix] - mean_h);
                var += dz * dz;
            }
            level[ix] = mean_h;
            grade[ix] = (cov / var).clamp(-0.5 * max_slope, 0.5 * max_slope);
        }
        let steepest = |p: &[f64]| {
            p.windows(2)
                .map(|w| (w[1] - w[0]).abs() / GRID_RESOLUTION)
                .fold(0.0, f64::max)
        };
        let relax = |p: &mut Vec<f64>| {
            let before = p.clone();
            for i in 0..before.len() {
                let left = before[i.saturating_sub(1)];
                let right = before[(i + 1).min(before.len() - 1)];
                p[i] = before[i] + ((left + right) * 0.5 - before[i]) * 0.5;
            }
        };
        for _ in 0..40 {
            relax(&mut grade);
        }
        for _ in 0..4000 {
            if steepest(&level) <= 0.6 * max_slope {
                break;
            }
            relax(&mut level);
        }
        for iz in 0..nz {
            let dz = iz as f64 * GRID_RESOLUTION - line;
            let wz = 1.0 - smoothstep(SHELF_HALF, SHELF_BLEND, dz.abs());
            if wz <= 0.0 {
                continue;
            }
            for ix in 0..nx {
                let dx = (ix as f64 * GRID_RESOLUTION - center).abs();
                let w = wz * (1.0 - smoothstep(half + 20.0, half + 110.0, dx));
                let shelf = level[ix] + grade[ix] * dz;
                let h = &mut heights[iz * nx + ix];
                *h = lerp(*h, shelf, w);
            }
        }
    }
}

// --- Woods and mud ------------------------------------------------------------------------

/// Read-only view of the height grid for the placement of woods and mud.
struct Ground<'a> {
    heights: &'a [f64],
    nx: usize,
    nz: usize,
    size: FieldSize,
}

impl Ground<'_> {
    fn height(&self, x: f64, z: f64) -> f64 {
        let fx = (x / GRID_RESOLUTION).clamp(0.0, (self.nx - 1) as f64);
        let fz = (z / GRID_RESOLUTION).clamp(0.0, (self.nz - 1) as f64);
        let (ix, iz) = (fx.floor() as usize, fz.floor() as usize);
        let (ix1, iz1) = ((ix + 1).min(self.nx - 1), (iz + 1).min(self.nz - 1));
        let (tx, tz) = (fx - ix as f64, fz - iz as f64);
        let at = |i: usize, j: usize| self.heights[j * self.nx + i];
        let top = lerp(at(ix, iz), at(ix1, iz), tx);
        let bottom = lerp(at(ix, iz1), at(ix1, iz1), tx);
        lerp(top, bottom, tz)
    }

    /// Slope (rise over run) over 20 m.
    fn slope(&self, x: f64, z: f64) -> f64 {
        let gx = (self.height(x + 10.0, z) - self.height(x - 10.0, z)) / 20.0;
        let gz = (self.height(x, z + 10.0) - self.height(x, z - 10.0)) / 20.0;
        (gx * gx + gz * gz).sqrt()
    }

    /// Hollowness (m): mean of the ring at 40 m minus the centre (> 0 in a
    /// hollow, < 0 on a crest).
    fn hollow(&self, x: f64, z: f64) -> f64 {
        let r = 40.0;
        let ring = (self.height(x + r, z)
            + self.height(x - r, z)
            + self.height(x, z + r)
            + self.height(x, z - r))
            * 0.25;
        ring - self.height(x, z)
    }
}

/// Same rule as the pre-R2 placement: a disc stays off the centre of both
/// deployment lines.
fn near_line(size: &FieldSize, x: f64, z: f64, radius: f64) -> bool {
    [size.attacker_line_z(), size.defender_line_z()]
        .iter()
        .any(|line| {
            (z - line).abs() < radius + 40.0
                && (x - size.center_x()).abs() < size.line_half() + radius
        })
}

fn in_river(river: Option<&River>, x: f64, z: f64, radius: f64) -> bool {
    river.is_some_and(|r| (z - r.center_z(x)).abs() < radius + r.width_at(x) * 0.5 + 4.0)
}

fn fits(zone: &Zone, ground: &Ground, river: Option<&River>) -> bool {
    let size = &ground.size;
    !near_line(size, zone.x, zone.z, zone.radius)
        && !in_river(river, zone.x, zone.z, zone.radius)
        && (0.0..=size.width).contains(&zone.x)
        && (0.0..=size.depth).contains(&zone.z)
}

#[derive(Clone, Copy, PartialEq)]
enum Cover {
    Wood,
    Mud,
}

impl Cover {
    /// Preference of the cover for (x, z): woods on steep slopes and
    /// crests, mud in hollows and low ground.
    fn score(self, ground: &Ground, x: f64, z: f64) -> f64 {
        match self {
            Cover::Wood => ground.slope(x, z) * 25.0 - ground.hollow(x, z) * 0.6,
            Cover::Mud => ground.hollow(x, z) * 1.2 - ground.height(x, z) * 0.15,
        }
    }
}

/// Moves `anchor` within `reach` metres to the best place for its cover
/// (never onto the deployment lines or into the river).
fn settle(
    anchor: Zone,
    cover: Cover,
    ground: &Ground,
    river: Option<&River>,
    reach: f64,
    stream: &mut BattleRng,
) -> Zone {
    let mut best = (cover.score(ground, anchor.x, anchor.z), anchor);
    for _ in 0..14 {
        let angle = stream.range(0.0, std::f64::consts::TAU);
        let dist = reach * stream.unit().sqrt();
        let candidate = Zone {
            x: anchor.x + angle.cos() * dist,
            z: anchor.z + angle.sin() * dist,
            radius: anchor.radius,
        };
        if !fits(&candidate, ground, river) {
            continue;
        }
        let score = cover.score(ground, candidate.x, candidate.z) - dist * 0.004;
        if score > best.0 {
            best = (score, candidate);
        }
    }
    best.1
}

/// Grows the lobes and copses of one cluster around `anchor`.
fn grow(
    anchor: Zone,
    full_radius: f64,
    cover: Cover,
    ground: &Ground,
    river: Option<&River>,
    stream: &mut BattleRng,
) -> Vec<Zone> {
    let mut cluster = vec![anchor];
    let lobes = match cover {
        Cover::Wood => 2 + stream.below(3),
        Cover::Mud => 1 + stream.below(3),
    };
    for _ in 0..lobes {
        let base = cluster[stream.below(cluster.len() as u32) as usize];
        let mut best: Option<(f64, Zone)> = None;
        for _ in 0..3 {
            let radius = full_radius * stream.range(0.32, 0.6);
            let angle = stream.range(0.0, std::f64::consts::TAU);
            let dist = (base.radius + radius) * stream.range(0.45, 0.8);
            let lobe = Zone {
                x: base.x + angle.cos() * dist,
                z: base.z + angle.sin() * dist,
                radius,
            };
            if !fits(&lobe, ground, river) {
                continue;
            }
            let score = cover.score(ground, lobe.x, lobe.z) + stream.range(0.0, 0.5);
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, lobe));
            }
        }
        if let Some((_, lobe)) = best {
            cluster.push(lobe);
        }
    }
    // Copses (woods) or puddles (mud) a little apart from the massif.
    let satellites = match cover {
        Cover::Wood => stream.below(4),
        Cover::Mud => stream.below(3),
    };
    for _ in 0..satellites {
        let (low, high) = match cover {
            Cover::Wood => (7.0, 16.0),
            Cover::Mud => (5.0, 11.0),
        };
        let radius = stream.range(low, high);
        let angle = stream.range(0.0, std::f64::consts::TAU);
        let dist = full_radius * stream.range(1.15, 2.1);
        let copse = Zone {
            x: anchor.x + angle.cos() * dist,
            z: anchor.z + angle.sin() * dist,
            radius,
        };
        if fits(&copse, ground, river) {
            cluster.push(copse);
        }
    }
    cluster.remove(0);
    cluster
}

/// Turns the round woods and mud of the pre-R2 field into irregular clusters
/// that follow the relief: each anchor keeps its index (shrunk, moved to its
/// best ground nearby), the lobes and copses are returned as parts.
pub(crate) fn shape_cover(
    forests: &mut [Zone],
    mud: &mut [Zone],
    heights: &[f64],
    size: &FieldSize,
    river: Option<&River>,
    stream: &mut BattleRng,
) -> (Vec<Zone>, Vec<Zone>) {
    let (nx, nz) = grid_dims(heights, size);
    let ground = Ground {
        heights,
        nx,
        nz,
        size: *size,
    };
    let mut shape = |zones: &mut [Zone], cover: Cover, reach: f64| {
        let mut parts = Vec::new();
        for zone in zones.iter_mut() {
            let full = zone.radius;
            let mut anchor = settle(*zone, cover, &ground, river, reach, stream);
            anchor.radius = full * 0.72;
            parts.extend(grow(anchor, full, cover, &ground, river, stream));
            *zone = anchor;
        }
        parts
    };
    let forest_parts = shape(forests, Cover::Wood, 90.0);
    let mud_parts = shape(mud, Cover::Mud, 110.0);
    (forest_parts, mud_parts)
}
