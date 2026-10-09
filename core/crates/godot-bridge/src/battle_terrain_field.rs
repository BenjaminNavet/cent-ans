//! Lot SC PF-08 / BT7 / GB6 (terrain part): the height field, the river and
//! the meshes of the battle ground (`battle_terrain.gd`), computed in plain
//! Rust instead of GDScript loops. Rendering only, no game rule: the
//! simulation still owns `heights`, rivers and fords; this module blends them
//! with the decorative hills of the horizon and builds the ground meshes.
//!
//! The arithmetic follows the GDScript it replaces (`float` in double,
//! `Vector2`/`Vector3` and packed arrays in single precision), so heights,
//! meshes and relief bytes are the same as before the port.

/// Decorative noise of the hills, ridges and rolls beyond the field.
pub trait Noise {
    fn hills(&self, x: f32, z: f32) -> f32;
    fn ridges(&self, x: f32, z: f32) -> f32;
    fn rolls(&self, x: f32, z: f32) -> f32;
}

/// Godot `Math::is_equal_approx`.
fn approx_eq(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    let tolerance = (1e-5 * a.abs()).max(1e-5);
    (a - b).abs() < tolerance
}

/// Godot `smoothstep(from, to, x)`.
pub fn smoothstep(from: f64, to: f64, x: f64) -> f64 {
    if approx_eq(from, to) {
        return if x <= from { 0.0 } else { 1.0 };
    }
    let s = ((x - from) / (to - from)).clamp(0.0, 1.0);
    s * s * (3.0 - 2.0 * s)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// `Vector2(dx, dz).length()` in single precision.
fn len32(dx: f64, dz: f64) -> f64 {
    let (x, z) = (dx as f32, dz as f32);
    f64::from((x * x + z * z).sqrt())
}

/// `Geometry2D.get_closest_point_to_segment(p, a, b).distance_to(p)`.
pub fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (px, pz) = (p.0 - a.0, p.1 - a.1);
    let (nx, nz) = (b.0 - a.0, b.1 - a.1);
    let l2 = nx * nx + nz * nz;
    let closest = if l2 < 1e-20 {
        a
    } else {
        let d = (nx * px + nz * pz) / l2;
        if d <= 0.0 {
            a
        } else if d >= 1.0 {
            b
        } else {
            (a.0 + nx * d, a.1 + nz * d)
        }
    };
    let (dx, dz) = (p.0 - closest.0, p.1 - closest.1);
    (dx * dx + dz * dz).sqrt()
}

/// Real relief of the place (EP2), blended over the generated hills.
#[derive(Debug, Clone, Default)]
pub struct Horizon {
    pub heights: Vec<f32>,
    pub n: usize,
    pub step: f64,
    pub half: f64,
    pub centre: (f32, f32),
    pub field_size: (f32, f32),
    pub cos: f64,
    pub sin: f64,
    pub offset_y: f64,
    pub blend_start: f64,
    pub blend_end: f64,
    pub river_keep: f64,
}

impl Horizon {
    fn real_height(&self, x: f64, z: f64) -> f64 {
        let e = x - f64::from(self.centre.0);
        let nn = f64::from(self.centre.1) - z;
        let rx = e * self.cos - nn * self.sin;
        let ry = nn * self.cos + e * self.sin;
        let cx = (rx + self.half) / self.step;
        let cz = (self.half - ry) / self.step;
        let last = (self.n - 1) as f64;
        let fx = cx.clamp(0.0, last);
        let fz = cz.clamp(0.0, last);
        let ix = (fx as usize).min(self.n - 2);
        let iz = (fz as usize).min(self.n - 2);
        let (tx, tz) = (fx - ix as f64, fz - iz as f64);
        let h = |i: usize| f64::from(self.heights[i]);
        let i = iz * self.n + ix;
        let top = lerp(h(i), h(i + 1), tx);
        let bottom = lerp(h(i + self.n), h(i + self.n + 1), tx);
        lerp(top, bottom, tz) + self.offset_y
    }

    fn weight(&self, x: f64, z: f64, river_distance: f64) -> f64 {
        let dx = x - x.clamp(0.0, f64::from(self.field_size.0));
        let dz = z - z.clamp(0.0, f64::from(self.field_size.1));
        let mut w = smoothstep(self.blend_start, self.blend_end, len32(dx, dz));
        if river_distance < f64::INFINITY {
            w *= smoothstep(40.0, self.river_keep, river_distance);
        }
        w
    }

    fn blend(&self, x: f64, z: f64, generated: f64, river_distance: f64) -> f64 {
        let w = self.weight(x, z, river_distance);
        if w <= 0.0 {
            return generated;
        }
        lerp(generated, self.real_height(x, z), w)
    }
}

/// Sea on one flank of the field (B5).
#[derive(Debug, Clone, Copy)]
pub struct Coast {
    pub west: bool,
    pub shore_x: f64,
    pub sea_level: f64,
}

/// Biome settings of the hills beyond the field.
#[derive(Debug, Clone, Copy)]
pub struct Biome {
    pub relief: f64,
    pub ridges: f64,
    pub rolls: f64,
}

/// A pool dug visually into the ground (x, z, radius).
pub type Pool = (f32, f32, f64);

const RIVER_CARVE: f64 = 1.6;
const RIVER_SPAN: f64 = 27.0;
const POOL_CARVE: f64 = 0.7;

#[derive(Debug, Clone)]
pub struct Terrain {
    pub heights: Vec<f32>,
    pub nx: usize,
    pub nz: usize,
    pub resolution: f64,
    pub width: f64,
    pub depth: f64,
    pub mean_height: f64,
    pub river_points: Vec<(f32, f32)>,
    pub river_widths: Vec<f32>,
    pub river_default_width: f64,
    pub biome: Biome,
    pub coast: Option<Coast>,
    pub horizon: Option<Horizon>,
    pub pools: Vec<Pool>,
}

impl Default for Terrain {
    fn default() -> Self {
        Terrain {
            heights: Vec::new(),
            nx: 0,
            nz: 0,
            resolution: 10.0,
            width: 1200.0,
            depth: 800.0,
            mean_height: 0.0,
            river_points: Vec::new(),
            river_widths: Vec::new(),
            river_default_width: 0.0,
            biome: Biome {
                relief: 0.45,
                ridges: 0.0,
                rolls: 3.0,
            },
            coast: None,
            horizon: None,
            pools: Vec::new(),
        }
    }
}

/// A triangle mesh ready for `ArrayMesh.add_surface_from_arrays`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshData {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<i32>,
}

fn normal32(x: f64, y: f64, z: f64) -> [f32; 3] {
    let (x, y, z) = (x as f32, y as f32, z as f32);
    let l = x * x + y * y + z * z;
    if l == 0.0 {
        return [x, y, z];
    }
    let l = l.sqrt();
    [x / l, y / l, z / l]
}

/// Godot `Rect2::encloses`, single precision.
fn encloses(hole: (f32, f32, f32, f32), cell: (f32, f32, f32, f32)) -> bool {
    cell.0 >= hole.0
        && cell.1 >= hole.1
        && cell.0 + cell.2 <= hole.0 + hole.2
        && cell.1 + cell.3 <= hole.1 + hole.3
}

/// Godot `Rect2::has_point`.
fn has_point(rect: (f32, f32, f32, f32), p: (f32, f32)) -> bool {
    p.0 >= rect.0 && p.1 >= rect.1 && p.0 < rect.0 + rect.2 && p.1 < rect.1 + rect.3
}

impl Terrain {
    /// Bilinear height of the simulation grid, clamped to the field.
    pub fn height_at(&self, x: f64, z: f64) -> f64 {
        if self.nx < 2 {
            return 0.0;
        }
        let fx = (x / self.resolution).clamp(0.0, (self.nx - 1) as f64);
        let fz = (z / self.resolution).clamp(0.0, (self.nz - 1) as f64);
        let ix = fx.floor() as usize;
        let iz = fz.floor() as usize;
        let ix1 = (ix + 1).min(self.nx - 1);
        let iz1 = (iz + 1).min(self.nz - 1);
        let (tx, tz) = (fx - ix as f64, fz - iz as f64);
        let h = |iz: usize, ix: usize| f64::from(self.heights[iz * self.nx + ix]);
        let top = h(iz, ix) * (1.0 - tx) + h(iz, ix1) * tx;
        let bottom = h(iz1, ix) * (1.0 - tx) + h(iz1, ix1) * tx;
        top * (1.0 - tz) + bottom * tz
    }

    /// Distance to the river bed (extended beyond the field), infinite without river.
    pub fn river_distance(&self, x: f64, z: f64) -> f64 {
        let pts = &self.river_points;
        if pts.len() < 2 {
            return f64::INFINITY;
        }
        let centre = ((x - f64::from(pts[0].0)) / 10.0) as i64;
        let lo = (centre - 4).max(0);
        let hi = (centre + 4).min(pts.len() as i64 - 1);
        let p = (x as f32, z as f32);
        let mut best = f64::INFINITY;
        for i in lo..hi {
            let d = segment_distance(p, pts[i as usize], pts[i as usize + 1]);
            best = best.min(f64::from(d));
        }
        best
    }

    fn river_index(&self, x: f64) -> f64 {
        if self.river_points.len() < 2 {
            return -1.0;
        }
        ((x - f64::from(self.river_points[0].0)) / 10.0)
            .clamp(0.0, (self.river_points.len() - 1) as f64)
    }

    /// Width of the water at `x` (extended like the course).
    pub fn river_width_at(&self, x: f64) -> f64 {
        let f = self.river_index(x);
        if f < 0.0 || self.river_widths.is_empty() {
            return self.river_default_width;
        }
        let i = (f as usize).min(self.river_widths.len().saturating_sub(2));
        lerp(
            f64::from(self.river_widths[i]),
            f64::from(self.river_widths[i + 1]),
            f - i as f64,
        )
    }

    /// Half width of the dug bed (1.5 times the width, as the simulation).
    pub fn river_span_at(&self, x: f64) -> f64 {
        let w = self.river_width_at(x);
        if w > 0.0 {
            w * 1.5
        } else {
            RIVER_SPAN
        }
    }

    /// Decorative height everywhere: the field inside, hills and river valley outside.
    pub fn world_height(&self, noise: &impl Noise, x: f64, z: f64) -> f64 {
        let cx = x.clamp(0.0, self.width);
        let cz = z.clamp(0.0, self.depth);
        let base = self.height_at(cx, cz);
        let d = len32(x - cx, z - cz);
        if d <= 0.0 {
            return base;
        }
        let (xf, zf) = (x as f32, z as f32);
        let t = smoothstep(0.0, 450.0, d);
        let mut n = f64::from(noise.hills(xf, zf)) * 0.5 + 0.5;
        let ridge_share = self.biome.ridges;
        if ridge_share > 0.0 {
            let ridge = (f64::from(noise.ridges(xf, zf)) * 0.6 + 0.35).clamp(0.0, 1.0);
            n = lerp(n, ridge, ridge_share);
        }
        let mut hills =
            n.powf(1.6) * lerp(22.0, 160.0, smoothstep(300.0, 4500.0, d)) * self.biome.relief;
        hills += f64::from(noise.rolls(xf, zf)) * self.biome.rolls * smoothstep(0.0, 250.0, d);
        let rd = self.river_distance(x, z);
        if rd < f64::INFINITY {
            hills *= smoothstep(25.0, 320.0, rd);
        }
        let mut h = lerp(base, self.mean_height + hills, t);
        let span = self.river_span_at(x);
        if rd < span {
            h -= RIVER_CARVE * (1.0 - rd / span) * t;
        }
        if let Some(horizon) = &self.horizon {
            h = horizon.blend(x, z, h, rd);
        }
        if let Some(coast) = &self.coast {
            let beyond = if coast.west { -x } else { x - self.width };
            if beyond > 0.0 {
                let edge = if coast.west { 0.0 } else { self.width };
                let offset = (coast.shore_x - edge).abs();
                h = lerp(
                    base,
                    coast.sea_level - 4.0,
                    smoothstep(0.0, offset * 2.5 + 40.0, beyond),
                );
            }
        }
        h
    }

    /// `world_height` over an `nx × nz` grid from `origin` (row major, rows along z).
    pub fn height_grid(
        &self,
        noise: &impl Noise,
        origin: (f64, f64),
        step: f64,
        nx: usize,
        nz: usize,
    ) -> Vec<f32> {
        let mut out = Vec::with_capacity(nx * nz);
        for iz in 0..nz {
            let z = origin.1 + iz as f64 * step;
            for ix in 0..nx {
                let x = origin.0 + ix as f64 * step;
                out.push(self.world_height(noise, x, z) as f32);
            }
        }
        out
    }

    /// Visual dig of the pools at (x, z) (0 outside).
    pub fn pool_carve(&self, x: f64, z: f64) -> f64 {
        let mut carve = 0.0f64;
        for &(px, pz, r) in &self.pools {
            let d = len32(x - f64::from(px), z - f64::from(pz));
            if d < r * 1.1 {
                carve = carve.max(POOL_CARVE * (1.0 - smoothstep(r * 0.55, r * 1.1, d)));
            }
        }
        carve
    }

    /// The simulation grid as a mesh with a 3 m skirt around it.
    pub fn field_mesh(&self) -> MeshData {
        let (nx, nz) = (self.nx, self.nz);
        let res = self.resolution;
        let mut m = MeshData::default();
        for iz in 0..nz {
            for ix in 0..nx {
                let i = iz * nx + ix;
                let carve = if self.pools.is_empty() {
                    0.0
                } else {
                    self.pool_carve(ix as f64 * res, iz as f64 * res)
                };
                m.vertices.push([
                    (ix as f64 * res) as f32,
                    (f64::from(self.heights[i]) - carve) as f32,
                    (iz as f64 * res) as f32,
                ]);
                let h = |iz: usize, ix: usize| f64::from(self.heights[iz * nx + ix]);
                let hl = h(iz, ix.saturating_sub(1));
                let hr = h(iz, (ix + 1).min(nx - 1));
                let hd = h(iz.saturating_sub(1), ix);
                let hu = h((iz + 1).min(nz - 1), ix);
                m.normals.push(normal32(hl - hr, 2.0 * res, hd - hu));
            }
        }
        for iz in 0..nz - 1 {
            for ix in 0..nx - 1 {
                let a = (iz * nx + ix) as i32;
                let (b, c) = (a + 1, a + nx as i32);
                m.indices.extend_from_slice(&[a, b, c, b, c + 1, c]);
            }
        }
        let mut border: Vec<i32> = (0..nx as i32).collect();
        border.extend((1..nz).map(|iz| (iz * nx + nx - 1) as i32));
        border.extend((0..nx - 1).rev().map(|ix| ((nz - 1) * nx + ix) as i32));
        border.extend((0..nz - 1).rev().map(|iz| (iz * nx) as i32));
        let base = m.vertices.len() as i32;
        for &k in &border {
            let v = m.vertices[k as usize];
            m.vertices.push([v[0], v[1] - 3.0, v[2]]);
            m.normals.push(m.normals[k as usize]);
        }
        for k in 0..border.len() {
            let k1 = (k + 1) % border.len();
            let (top0, top1) = (border[k], border[k1]);
            let (lo0, lo1) = (base + k as i32, base + k1 as i32);
            m.indices
                .extend_from_slice(&[top0, lo0, top1, top1, lo0, lo1]);
        }
        m
    }

    /// Terrain ring of grid `step` around `hole` (quads wholly inside are left out).
    pub fn ring_mesh(
        &self,
        noise: &impl Noise,
        rect: (f32, f32, f32, f32),
        step: f64,
        hole: (f32, f32, f32, f32),
        sink: f64,
    ) -> MeshData {
        let nx = (f64::from(rect.2) / step) as usize + 1;
        let nz = (f64::from(rect.3) / step) as usize + 1;
        let mut heights = vec![0.0f32; nx * nz];
        let mut m = MeshData::default();
        for iz in 0..nz {
            for ix in 0..nx {
                let x = f64::from(rect.0) + ix as f64 * step;
                let z = f64::from(rect.1) + iz as f64 * step;
                let mut h = self.world_height(noise, x, z);
                if has_point(hole, (x as f32, z as f32)) {
                    h -= sink;
                }
                heights[iz * nx + ix] = h as f32;
                m.vertices.push([x as f32, h as f32, z as f32]);
            }
        }
        let at = |iz: usize, ix: usize| f64::from(heights[iz * nx + ix]);
        for iz in 0..nz {
            for ix in 0..nx {
                let hl = at(iz, ix.saturating_sub(1));
                let hr = at(iz, (ix + 1).min(nx - 1));
                let hd = at(iz.saturating_sub(1), ix);
                let hu = at((iz + 1).min(nz - 1), ix);
                m.normals.push(normal32(hl - hr, 2.0 * step, hd - hu));
            }
        }
        for iz in 0..nz - 1 {
            for ix in 0..nx - 1 {
                let x0 = (f64::from(rect.0) + ix as f64 * step) as f32;
                let z0 = (f64::from(rect.1) + iz as f64 * step) as f32;
                if encloses(hole, (x0, z0, step as f32, step as f32)) {
                    continue;
                }
                let a = (iz * nx + ix) as i32;
                let (b, c) = (a + 1, a + nx as i32);
                m.indices.extend_from_slice(&[a, b, c, b, c + 1, c]);
            }
        }
        m
    }
}

/// The simulation river (x from the first to the last point, a point every
/// 10 m) extended `reach` metres each side by successive mirror images
/// (triangle wave in x), so the horizon rings have a valley too.
/// Returns the points and the width at each point.
pub fn extend_river(
    points: &[(f32, f32)],
    widths: &[f32],
    default_width: f64,
    reach: f64,
) -> (Vec<(f32, f32)>, Vec<f32>) {
    let (mut out_points, mut out_widths) = (Vec::new(), Vec::new());
    if points.len() < 2 {
        return (out_points, out_widths);
    }
    let first = f64::from(points[0].0);
    let span = f64::from(points[points.len() - 1].0) - first;
    let last = f64::from(points[points.len() - 1].0);
    let mut x = first - reach;
    while x <= last + reach {
        // `fposmod`: result with the sign of the divisor.
        let period = 2.0 * span;
        let mut m = (x - first) % period;
        if m < 0.0 {
            m += period;
        }
        if m > span {
            m = period - m;
        }
        let f = m / span * (points.len() - 1) as f64;
        let i = (f as usize).min(points.len() - 2);
        let t = f - i as f64;
        out_points.push((
            x as f32,
            lerp(f64::from(points[i].1), f64::from(points[i + 1].1), t) as f32,
        ));
        out_widths.push(if widths.len() == points.len() {
            lerp(f64::from(widths[i]), f64::from(widths[i + 1]), t) as f32
        } else {
            default_width as f32
        });
        x += 10.0;
    }
    (out_points, out_widths)
}

/// Ground relief map baked on the height grid (R2): r = slope (0.5 = 50 %),
/// g = hollow (> 0.5) or bump (< 0.5) at 30 m, b = same at 90 m; RGBA8 bytes.
pub fn relief_from_heights(hdata: &[f32], hw: usize, hh: usize, texel: f64) -> Vec<u8> {
    let mut bytes = vec![0u8; hw * hh * 4];
    if hw == 0 || hh == 0 {
        return bytes;
    }
    let at = |ix: i64, iz: i64| -> f64 {
        let z = iz.clamp(0, hh as i64 - 1) as usize;
        let x = ix.clamp(0, hw as i64 - 1) as usize;
        f64::from(hdata[z * hw + x])
    };
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0) as u8;
    for iz in 0..hh as i64 {
        for ix in 0..hw as i64 {
            let h = at(ix, iz);
            let gx = (at(ix + 1, iz) - at(ix - 1, iz)) / (2.0 * texel);
            let gz = (at(ix, iz + 1) - at(ix, iz - 1)) / (2.0 * texel);
            let near =
                (at(ix + 3, iz) + at(ix - 3, iz) + at(ix, iz + 3) + at(ix, iz - 3)) * 0.25 - h;
            let far =
                (at(ix + 9, iz) + at(ix - 9, iz) + at(ix, iz + 9) + at(ix, iz - 9)) * 0.25 - h;
            let i = (iz as usize * hw + ix as usize) * 4;
            bytes[i] = byte((gx * gx + gz * gz).sqrt() / 0.5);
            bytes[i + 1] = byte(0.5 + near / 4.0);
            bytes[i + 2] = byte(0.5 + far / 10.0);
            bytes[i + 3] = 255;
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Flat;
    impl Noise for Flat {
        fn hills(&self, _: f32, _: f32) -> f32 {
            0.0
        }
        fn ridges(&self, _: f32, _: f32) -> f32 {
            0.0
        }
        fn rolls(&self, _: f32, _: f32) -> f32 {
            0.0
        }
    }

    /// Same synthetic scene as `game/tests/sc_terrain_golden_test.gd`.
    fn scene() -> Terrain {
        let (nx, nz) = (121usize, 81usize);
        let mut heights = vec![0.0f32; nx * nz];
        for iz in 0..nz {
            for ix in 0..nx {
                heights[iz * nx + ix] = (6.0 * (ix as f64 * 0.11).sin() * (iz as f64 * 0.17).cos()
                    + 0.02 * ix as f64) as f32;
            }
        }
        let mut points = Vec::new();
        let mut widths = Vec::new();
        for i in 0..nx {
            points.push((
                i as f32 * 10.0,
                (380.0 + 40.0 * (i as f64 * 0.09).sin()) as f32,
            ));
            widths.push((14.0 + 4.0 * (i as f64 * 0.2).sin()) as f32);
        }
        let (points, widths) = extend_river(&points, &widths, 16.0, 2600.0);
        Terrain {
            mean_height: heights.iter().map(|h| f64::from(*h)).sum::<f64>() / heights.len() as f64,
            heights,
            nx,
            nz,
            river_points: points,
            river_widths: widths,
            river_default_width: 16.0,
            pools: vec![(700.0, 120.0, 14.0)],
            ..Terrain::default()
        }
    }

    #[test]
    fn bilinear_matches_the_gdscript_reference() {
        let t = scene();
        // Reference values taken from the GDScript run before the port.
        for (x, z, expected) in REFERENCE_HEIGHTS {
            let got = t.height_at(*x, *z);
            assert!(
                (got - expected).abs() < 1e-4,
                "({x}, {z}): {got} vs {expected}"
            );
        }
    }

    #[test]
    fn river_matches_the_gdscript_reference() {
        let t = scene();
        for (x, z, distance, width) in REFERENCE_RIVER {
            assert!(
                (t.river_distance(*x, *z) - distance).abs() < 1e-3,
                "distance ({x}, {z})"
            );
            assert!((t.river_width_at(*x) - width).abs() < 1e-4, "width {x}");
        }
        // Outside the extended course there is nothing to measure against.
        assert!(Terrain::default().river_distance(0.0, 0.0).is_infinite());
    }

    #[test]
    fn inside_the_field_world_height_is_the_grid() {
        let t = scene();
        for (x, z) in [(5.0, 5.0), (600.0, 400.0), (1190.0, 790.0)] {
            assert_eq!(t.world_height(&Flat, x, z), t.height_at(x, z));
        }
        // Far away the generated hills take over from the field.
        let far = t.world_height(&Flat, -3000.0, 4000.0);
        assert!(far.is_finite() && (far - t.height_at(0.0, 800.0)).abs() > 1.0);
    }

    #[test]
    fn meshes_have_the_expected_shape() {
        let t = scene();
        let field = t.field_mesh();
        // The skirt walks the border like the GDScript did: the start corner comes back once.
        let border = 2 * (t.nx - 1) + 2 * (t.nz - 1) + 1;
        assert_eq!(field.vertices.len(), t.nx * t.nz + border);
        assert_eq!(
            field.indices.len(),
            (t.nx - 1) * (t.nz - 1) * 6 + border * 6
        );
        let hole = (0.0, 0.0, 1200.0, 800.0);
        let ring = t.ring_mesh(&Flat, (-900.0, -900.0, 3000.0, 2600.0), 20.0, hole, 0.0);
        assert_eq!(ring.vertices.len(), 151 * 131);
        // 60 × 40 cells are wholly inside the hole.
        assert_eq!(ring.indices.len(), (150 * 130 - 60 * 40) * 6);
    }

    #[test]
    fn relief_of_a_ramp() {
        // 50 % slope along x: r = 255, flat curvature: g = b = 127.
        let (hw, hh) = (32, 32);
        let hdata: Vec<f32> = (0..hw * hh).map(|i| (i % hw) as f32 * 5.0).collect();
        let bytes = relief_from_heights(&hdata, hw, hh, 10.0);
        let i = (16 * hw + 16) * 4;
        assert_eq!(&bytes[i..i + 4], &[255, 127, 127, 255]);
    }

    const REFERENCE_HEIGHTS: &[(f64, f64, f64)] = &[
        (5.0, 5.0, 0.33696118),
        (600.0, 400.0, 2.8251197),
        (1190.0, 790.0, 4.329151),
        (0.0, 200.0, 0.0),
        (1200.0, 800.0, 4.217798),
        (300.0, 0.0, -0.34647417),
        (0.0, 800.0, 0.0),
        (1200.0, 0.0, 5.952441),
        (612.5, 391.3, 3.6740974669206476),
        (0.0, 400.0, 0.0),
    ];
    /// (x, z, distance to the bed, width of the water).
    const REFERENCE_RIVER: &[(f64, f64, f64, f64)] = &[
        (5.0, 5.0, 375.03333, 14.397339),
        (600.0, 400.0, 49.450066, 11.853708),
        (1190.0, 790.0, 443.75662, 10.112806),
        (-300.0, 200.0, 187.57469, 12.882338),
        (1700.0, 900.0, 506.8624, 17.96243),
        (300.0, -1200.0, 1583.7654, 12.882338),
        (612.5, 391.3, 37.799213, 12.759707689285278),
        (-20.0, 400.0, 12.114975, 15.557673),
    ];
}
