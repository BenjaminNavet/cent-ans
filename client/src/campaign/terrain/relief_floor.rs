//! Valley floor of the exaggerated relief: port of `relief_floor.gd` (`ReliefFloor.compute`).
//!
//! Low-resolution grid computed once from the world heightmap (cells of `floor_cell_px` pixels):
//! minimum, min filter, box blurs, cap at `tops - local_relief_cap_m`, then the regional
//! amplitude drives the squash / lift factor `k`, the smooth massif lift `D` and the plain
//! flattening. The true-scale zones around 1:1 emblematic towns (`_add_true_scale_zones`) are not
//! ported (they need the landmark library, owned by another lot).

use std::sync::Arc;

use super::profile::ReliefProfile;

/// World heightmap: row-major raw 16-bit samples (1 pixel = 1 map unit).
#[derive(Debug, Clone, Default)]
pub struct HeightGrid {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u16>,
    pub h_min_m: f32,
    pub h_range_m: f32,
}

impl HeightGrid {
    fn meters(&self, raw: u16) -> f32 {
        self.h_min_m + raw as f32 / 65535.0 * self.h_range_m
    }

    /// Altitude (m) at pixel coordinates (`fx`, `fy`), bilinear, clamped to the edge.
    pub fn bilinear_m(&self, fx: f32, fy: f32) -> f32 {
        if self.width < 2 || self.height < 2 {
            return 0.0;
        }
        let fx = fx.clamp(0.0, self.width as f32 - 1.0);
        let fy = fy.clamp(0.0, self.height as f32 - 1.0);
        let i = (fx as usize).min(self.width - 2);
        let j = (fy as usize).min(self.height - 2);
        let (tx, ty) = (fx - i as f32, fy - j as f32);
        let at = |x: usize, y: usize| self.data[y * self.width + x] as f32;
        let top = at(i, j) + (at(i + 1, j) - at(i, j)) * tx;
        let bottom = at(i, j + 1) + (at(i + 1, j + 1) - at(i, j + 1)) * tx;
        self.h_min_m + (top + (bottom - top) * ty) / 65535.0 * self.h_range_m
    }

    /// Altitude (m) at the map point (x, z): pixel i is centred on x = i (ADR 0086).
    pub fn height_m_at(&self, x: f32, z: f32) -> f32 {
        self.bilinear_m(x, z)
    }

    /// (min, max) altitude in metres per `chunk` x `chunk` block, row-major, with the margins of
    /// `TerrainBuilder._chunk_bounds_m` so that bounding boxes stay conservative.
    pub fn chunk_bounds(&self, chunk: usize, cols: usize, rows: usize) -> Vec<(f32, f32)> {
        let mut out = Vec::with_capacity(cols * rows);
        for row in 0..rows {
            for col in 0..cols {
                let (mut lo, mut hi) = (u16::MAX, 0u16);
                let (y0, y1) = (row * chunk, ((row + 1) * chunk).min(self.height));
                let (x0, x1) = (col * chunk, ((col + 1) * chunk).min(self.width));
                for y in y0..y1 {
                    for &raw in &self.data[y * self.width + x0..y * self.width + x1.max(x0)] {
                        lo = lo.min(raw);
                        hi = hi.max(raw);
                    }
                }
                let (lo_m, hi_m) = if hi == 0 && lo == u16::MAX {
                    (0.0, 0.0)
                } else {
                    (self.meters(lo), self.meters(hi))
                };
                out.push((
                    lo_m.min(0.0) - 150.0,
                    hi_m + (0.5 * (hi_m - lo_m)).max(200.0),
                ));
            }
        }
        out
    }
}

/// Floor grids (metres; cell-centre convention: cell c centre at x = c*cell + (cell-1)/2).
#[derive(Debug, Clone, Default)]
pub struct FloorGrid {
    pub side: (usize, usize),
    pub cell: f32,
    pub floor: Arc<Vec<f32>>,
    pub base: Arc<Vec<f32>>,
    pub squash: Arc<Vec<f32>>,
    pub lift: Arc<Vec<f32>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Min,
    Mean,
    Max,
}

fn filter(src: &[f32], side: (usize, usize), radius: i32, mode: Mode) -> Vec<f32> {
    if radius <= 0 {
        return src.to_vec();
    }
    let (w, h) = (side.0 as i32, side.1 as i32);
    let inv = 1.0 / (2 * radius + 1) as f32;
    let pass = |input: &[f32], horizontal: bool| -> Vec<f32> {
        let mut out = vec![0.0f32; input.len()];
        for y in 0..h {
            for x in 0..w {
                let sample = |k: i32| {
                    let (sx, sy) = if horizontal {
                        ((x + k).clamp(0, w - 1), y)
                    } else {
                        (x, (y + k).clamp(0, h - 1))
                    };
                    input[(sy * w + sx) as usize]
                };
                let mut acc = sample(-radius);
                for k in -radius + 1..=radius {
                    let v = sample(k);
                    acc = match mode {
                        Mode::Mean => acc + v,
                        Mode::Min => acc.min(v),
                        Mode::Max => acc.max(v),
                    };
                }
                out[(y * w + x) as usize] = if mode == Mode::Mean { acc * inv } else { acc };
            }
        }
        out
    };
    pass(&pass(src, true), false)
}

/// `ReliefFloor.compute`.
pub fn compute(heights: &HeightGrid, profile: &ReliefProfile) -> FloorGrid {
    let cell = profile.floor_cell_px.max(1) as usize;
    let side = (heights.width.div_ceil(cell), heights.height.div_ceil(cell));
    let step = (profile.floor_sample_step.clamp(1, cell as i32)) as usize;
    let mut lows = vec![0.0f32; side.0 * side.1];
    let mut highs = vec![0.0f32; side.0 * side.1];
    for r in 0..side.1 {
        for c in 0..side.0 {
            let (mut low, mut high) = (65536i32, -1i32);
            let y_end = ((r + 1) * cell).min(heights.height);
            let x_end = ((c + 1) * cell).min(heights.width);
            let mut y = r * cell;
            while y < y_end {
                let mut x = c * cell;
                while x < x_end {
                    let raw = heights.data[y * heights.width + x] as i32;
                    low = low.min(raw);
                    high = high.max(raw);
                    x += step;
                }
                y += step;
            }
            let i = r * side.0 + c;
            if high >= 0 {
                lows[i] = heights.meters(low as u16).max(0.0);
                highs[i] = heights.meters(high as u16).max(0.0);
            }
        }
    }
    let passes = profile.floor_blur_passes.max(0);
    let blur = profile.floor_blur_radius.max(0);
    let mut grid = filter(&lows, side, profile.floor_min_radius.max(0), Mode::Min);
    for _ in 0..passes {
        grid = filter(&grid, side, blur, Mode::Mean);
    }
    let base = grid.clone();
    let n = grid.len();
    let mut squash = vec![0.0f32; n];
    let mut lift = vec![0.0f32; n];
    let mut amplitude = vec![0.0f32; n];
    if profile.local_relief_cap_m > 0.0 || profile.shapes_relief() {
        let reach = profile.floor_min_radius.max(0) + blur * passes;
        let mut tops = filter(&highs, side, reach, Mode::Max);
        for _ in 0..passes {
            tops = filter(&tops, side, blur, Mode::Mean);
        }
        for i in 0..n {
            if profile.local_relief_cap_m > 0.0 {
                grid[i] = grid[i].max(tops[i] - profile.local_relief_cap_m);
            }
            amplitude[i] = tops[i] - base[i];
            squash[i] = profile.mountain_squash_of(amplitude[i]);
        }
        if profile.massif_lift > 0.0 || profile.plain_flatten > 0.0 {
            let fade = profile.massif_coast_fade_cells.max(0);
            let coast: Vec<f32> = lows
                .iter()
                .map(|&l| if l <= 0.0 { 0.0 } else { 1.0 })
                .collect();
            let coast = filter(
                &filter(&coast, side, fade, Mode::Min),
                side,
                fade,
                Mode::Mean,
            );
            let mids: Vec<f32> = (0..n).map(|i| 0.5 * (lows[i] + highs[i])).collect();
            let mids = filter(&mids, side, profile.massif_smooth_radius.max(0), Mode::Mean);
            for i in 0..n {
                lift[i] =
                    profile.massif_lift_of(amplitude[i]) * (mids[i] - base[i]).max(0.0) * coast[i];
                squash[i] -= profile.plain_flatten_of(amplitude[i]) * (1.0 - coast[i]);
            }
        }
    }
    FloorGrid {
        side,
        cell: cell as f32,
        floor: Arc::new(grid),
        base: Arc::new(base),
        squash: Arc::new(squash),
        lift: Arc::new(lift),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::terrain::tres::Tres;

    fn synthetic(width: usize, height: usize, f: impl Fn(usize, usize) -> u16) -> HeightGrid {
        let mut data = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                data.push(f(x, y));
            }
        }
        HeightGrid {
            width,
            height,
            data,
            h_min_m: -200.0,
            h_range_m: 5000.0,
        }
    }

    #[test]
    fn bilinear_matches_corners_and_midpoints() {
        let grid = synthetic(4, 4, |x, _| (x as u16) * 6553);
        let a = grid.bilinear_m(1.0, 1.0);
        let b = grid.bilinear_m(2.0, 1.0);
        let mid = grid.bilinear_m(1.5, 1.0);
        assert!((mid - 0.5 * (a + b)).abs() < 1e-3);
        assert!((grid.bilinear_m(-5.0, -5.0) - grid.bilinear_m(0.0, 0.0)).abs() < 1e-6);
    }

    #[test]
    fn flat_sea_has_zero_floor_and_lift() {
        // Raw value of 0 m: (0 + 200) / 5000 * 65535.
        let sea = (200.0f32 / 5000.0 * 65535.0) as u16;
        let grid = synthetic(64, 64, |_, _| sea);
        let profile = ReliefProfile::from_tres(&Tres::parse(
            "[resource]\nmassif_lift = 5.5\nmountain_lift = 0.2\nplain_flatten = 0.6\nmountain_knee_m = 0\n",
        ));
        let floor = compute(&grid, &profile);
        assert_eq!(floor.side, (8, 8));
        assert!(floor.floor.iter().all(|v| v.abs() < 1.0));
        assert!(floor.lift.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn mountain_raises_regional_amplitude_and_lift() {
        // A 2000 m plateau in the middle of a plain: lift > 0 inside, 0 on the plain.
        let high = (2200.0f32 / 5000.0 * 65535.0) as u16;
        let low = (400.0f32 / 5000.0 * 65535.0) as u16;
        let grid = synthetic(160, 160, |x, y| {
            if (60..100).contains(&x) && (60..100).contains(&y) {
                high
            } else {
                low
            }
        });
        let profile = ReliefProfile::from_tres(&Tres::parse(
            "[resource]\nmassif_lift = 5.5\nmountain_lift = 0.2\nmountain_lift_start_m = 350\nmountain_lift_full_m = 1400\nmassif_smooth_radius = 2\nmountain_knee_m = 0\n",
        ));
        let floor = compute(&grid, &profile);
        let centre = floor.lift[10 * floor.side.0 + 10];
        let corner = floor.lift[0];
        assert!(centre > 0.0, "centre lift {centre}");
        assert!(corner < centre);
    }

    #[test]
    fn chunk_bounds_contain_the_data() {
        let grid = synthetic(512, 256, |x, _| (x as u16) * 100);
        let bounds = grid.chunk_bounds(256, 2, 1);
        assert_eq!(bounds.len(), 2);
        assert!(bounds[1].1 > bounds[0].1);
        assert!(bounds[0].0 <= -150.0);
    }
}
