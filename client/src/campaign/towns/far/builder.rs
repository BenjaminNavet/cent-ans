//! Far mesh of the 1:1 towns without a plan (port of `town_far_builder.gd`, ADR 0138).
//!
//! - F1: polar roof sheet on the bearings of `radii` (centre + 2 rings), skirt, suburbs, enclosure
//!   (outer wall face, wall walk, towers, gates), simplified monuments, parish steeples;
//! - F2: 16-sided polygon, skirt, one spire at most (cathedral, else keep).
//!
//! Pure functions (no ECS, no file access): callable from a worker thread.
//!
//! Vertex contract (shared with `town_far.wesl`):
//! - `position.x / z`: world position in map units (anchor `px` + local metres / metres per unit;
//!   local frame of the town plans: x towards +X world (east), y towards +Z world);
//! - `position.y`: ABSOLUTE altitude in metres (ground + height of the building; skirt foot =
//!   ground - 30);
//! - `uv_1.x`: town index (mask texel), `uv_1.y`: height (m) above the local ground (0 at the
//!   foot, -30 at the bottom of the skirt), so the vertex ground is `position.y - uv_1.y`;
//! - `color`: tint (linear rgb), alpha 1; `normal`: computed in metres (not exaggerated).
//!
//! Ground: [`Ground`] holds `1 + 2n` samples (centre, `0.5 * radii[k]`, `radii[k]`), taken from the
//! resident relief on the main thread (Godot read the `ground_m` baked in `towns_1340.json`, which
//! predates the composed map). Front faces are counter-clockwise seen from outside (Bevy).

use std::f32::consts::TAU;

use bevy::math::{Vec2, Vec3};

use super::super::data::{Town, WallSpec};
use super::super::plan::radius_at;
use super::super::rng::Rng;

/// Depth of the skirt under the ground (m): hides the gap with the relief.
pub const SKIRT_M: f32 = 30.0;
const CROWN_M: f32 = 3.0;
const EAVE_JITTER_M: f32 = 1.0;
/// Fraction of the radius of the inner ring (convention of [`Ground`]).
pub const INNER_RING: f32 = 0.5;
const WALLED_EDGE: f32 = 0.97;
const TOWER_EVERY: usize = 4;
const GATE_SIZE_M: f32 = 12.0;
const GATE_EXTRA_M: f32 = 5.0;
const FAUBOURG_WIDTH_M: f32 = 56.0;
const FAUBOURG_EAVE_M: f32 = 7.0;
const FAUBOURG_RIDGE_M: f32 = 10.0;
const FAUBOURG_SPLIT_M: f32 = 150.0;
const CATHEDRAL_WIDTH: f32 = 0.28;
const CATHEDRAL_EAVE_M: f32 = 22.0;
const CATHEDRAL_RIDGE_M: f32 = 34.0;
const CATHEDRAL_TOWER_M: (f32, f32) = (60.0, 90.0);
const CATHEDRAL_SPIRE_M: f32 = 25.0;
const ABBEY_CHURCH: f32 = 0.5;
const ABBEY_EAVE_M: f32 = 16.0;
const ABBEY_RIDGE_M: f32 = 25.0;
const ABBEY_TOWER_M: f32 = 38.0;
const ABBEY_SPIRE_M: f32 = 14.0;
const KEEP_FRACTION: f32 = 0.2;
const KEEP_HEIGHT_M: f32 = 28.0;
const HALL_EAVE_M: f32 = 9.0;
const HALL_RIDGE_M: f32 = 17.0;
const WINDMILL_HEIGHT_M: f32 = 10.0;
const WINDMILL_CAP_M: f32 = 4.0;
const PARISH_MAX: i32 = 4;
const PARISH_TOWER_M: f32 = 30.0;
const PARISH_SPIRE_M: f32 = 16.0;
const PARISH_HALF_M: f32 = 3.5;
pub const F2_SIDES: usize = 16;

/// sRGB tints of `town_far_builder.gd`.
const ROOF_TILE: [f32; 3] = [0.60, 0.33, 0.24];
const ROOF_THATCH: [f32; 3] = [0.55, 0.47, 0.31];
const FACADE: [f32; 3] = [0.68, 0.62, 0.53];
const STONE: [f32; 3] = [0.72, 0.68, 0.60];
const CHURCH_STONE: [f32; 3] = [0.78, 0.74, 0.66];
const LEAD: [f32; 3] = [0.38, 0.40, 0.44];
const TIMBER: [f32; 3] = [0.45, 0.36, 0.26];

/// Eave of the roof sheet (m above the ground) by settlement kind.
fn eave_m(kind: &str) -> f32 {
    match kind {
        "city" => 11.0,
        "town" => 9.5,
        "village" => 6.5,
        _ => 9.0,
    }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn lin(c: [f32; 3]) -> [f32; 4] {
    [
        srgb_to_linear(c[0]),
        srgb_to_linear(c[1]),
        srgb_to_linear(c[2]),
        1.0,
    ]
}

/// Merged vertex arrays of a far mesh (see the module contract).
#[derive(Debug, Clone, Default)]
pub struct FarMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub uv1: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    /// Altitude range (m) of the vertices.
    pub y_min: f32,
    pub y_max: f32,
}

impl FarMesh {
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    /// Appends `part` (indices offset).
    pub fn append(&mut self, part: &FarMesh) {
        if part.triangles() == 0 {
            return;
        }
        if self.positions.is_empty() {
            self.y_min = f32::INFINITY;
            self.y_max = f32::NEG_INFINITY;
        }
        let offset = self.positions.len() as u32;
        self.positions.extend_from_slice(&part.positions);
        self.normals.extend_from_slice(&part.normals);
        self.colors.extend_from_slice(&part.colors);
        self.uv1.extend_from_slice(&part.uv1);
        self.indices.extend(part.indices.iter().map(|i| i + offset));
        self.y_min = self.y_min.min(part.y_min);
        self.y_max = self.y_max.max(part.y_max);
    }
}

/// Vertex accumulator: positions in local metres kept apart for the normals.
struct Acc {
    anchor: Vec2,
    inv_mpu: f32,
    town_index: f32,
    pm: Vec<Vec3>,
    out: FarMesh,
}

impl Acc {
    fn new(anchor: Vec2, meters_per_unit: f32, index: u32) -> Self {
        Acc {
            anchor,
            inv_mpu: 1.0 / meters_per_unit.max(1e-6),
            town_index: index as f32,
            pm: Vec::new(),
            out: FarMesh {
                y_min: f32::INFINITY,
                y_max: f32::NEG_INFINITY,
                ..Default::default()
            },
        }
    }

    /// Vertex at the local point `p` (m), ground `ground` (absolute m), height `rel` above it.
    fn vert(&mut self, p: Vec2, ground: f32, rel: f32, normal: Vec3, color: [f32; 4]) -> u32 {
        let y = ground + rel;
        self.pm.push(Vec3::new(p.x, y, p.y));
        self.out.positions.push([
            self.anchor.x + p.x * self.inv_mpu,
            y,
            self.anchor.y + p.y * self.inv_mpu,
        ]);
        self.out.normals.push(normal.to_array());
        self.out.colors.push(color);
        self.out.uv1.push([self.town_index, rel]);
        self.out.y_min = self.out.y_min.min(y);
        self.out.y_max = self.out.y_max.max(y);
        (self.pm.len() - 1) as u32
    }

    /// Triangle facing `hint` (approximate outward direction, m).
    fn tri(&mut self, a: u32, b: u32, c: u32, hint: Vec3) {
        let (pa, pb, pc) = (
            self.pm[a as usize],
            self.pm[b as usize],
            self.pm[c as usize],
        );
        let n = (pb - pa).cross(pc - pa);
        if n.dot(hint) < 0.0 {
            self.out.indices.extend([a, c, b]);
        } else {
            self.out.indices.extend([a, b, c]);
        }
    }

    /// Flat face (3 or 4 points `(local, ground, rel)`), flat normal towards `hint`.
    fn face(&mut self, pts: &[(Vec2, f32, f32)], hint: Vec3, color: [f32; 3]) {
        let p: Vec<Vec3> = pts
            .iter()
            .map(|(xy, g, r)| Vec3::new(xy.x, g + r, xy.y))
            .collect();
        let mut n = (p[1] - p[0]).cross(p[2] - p[0]);
        if pts.len() == 4 && n.length_squared() < 1e-8 {
            n = (p[2] - p[0]).cross(p[3] - p[0]);
        }
        if n.length_squared() < 1e-8 {
            return;
        }
        if n.dot(hint) < 0.0 {
            n = -n;
        }
        let n = n.normalize();
        let color = lin(color);
        let ids: Vec<u32> = pts
            .iter()
            .map(|(xy, g, r)| self.vert(*xy, *g, *r, n, color))
            .collect();
        self.tri(ids[0], ids[1], ids[2], n);
        if ids.len() == 4 {
            self.tri(ids[0], ids[2], ids[3], n);
        }
    }

    /// Smoothed normals of the vertices `[from, to)` from the triangles added since `i_from`.
    fn smooth(&mut self, from: usize, to: usize, i_from: usize) {
        let mut acc = vec![Vec3::ZERO; to - from];
        for t in self.out.indices[i_from..].as_chunks::<3>().0 {
            let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let n = (self.pm[b] - self.pm[a]).cross(self.pm[c] - self.pm[a]);
            for k in [a, b, c] {
                if (from..to).contains(&k) {
                    acc[k - from] += n;
                }
            }
        }
        for (k, n) in acc.into_iter().enumerate() {
            self.out.normals[from + k] = if n.length_squared() > 1e-12 {
                n.normalize().to_array()
            } else {
                [0.0, 1.0, 0.0]
            };
        }
    }

    fn finish(self) -> FarMesh {
        self.out
    }
}

/// Ground of a town: polar grid `g` (`[0]` centre, `[1..=n]` at `0.5 * radii[k]`, `[n+1..]` at
/// `radii[k]`), else `z` everywhere.
#[derive(Debug, Clone, Default)]
pub struct Ground {
    pub radii: Vec<f32>,
    pub g: Vec<f32>,
    pub z: f32,
}

impl Ground {
    /// Samples `height_m` (absolute metres at a local point) on the polar grid of `radii`.
    pub fn sample(radii: &[f32], height_m: impl Fn(Vec2) -> f32) -> Ground {
        let n = radii.len();
        let mut g = Vec::with_capacity(1 + 2 * n);
        g.push(height_m(Vec2::ZERO));
        for ring in [INNER_RING, 1.0] {
            for (k, r) in radii.iter().enumerate() {
                let a = k as f32 / n as f32 * TAU;
                g.push(height_m(Vec2::new(a.cos(), a.sin()) * r * ring));
            }
        }
        Ground {
            radii: radii.to_vec(),
            g,
            z: 0.0,
        }
    }

    pub fn at(&self, p: Vec2) -> f32 {
        let n = self.radii.len();
        if n < 3 || self.g.len() != 1 + 2 * n {
            return self.z;
        }
        let d = p.length();
        if d < 1e-3 {
            return self.g[0];
        }
        let f = p.y.atan2(p.x).rem_euclid(TAU) / TAU * n as f32;
        let i = f as usize % n;
        let j = (i + 1) % n;
        let t = f - f.floor();
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let r = lerp(self.radii[i], self.radii[j], t);
        let ga = lerp(self.g[1 + i], self.g[1 + j], t);
        let gb = lerp(self.g[1 + n + i], self.g[1 + n + j], t);
        let s = d / r;
        if s <= INNER_RING {
            return lerp(self.g[0], ga, s / INNER_RING);
        }
        lerp(
            ga,
            gb,
            ((s - INNER_RING) / (1.0 - INNER_RING)).clamp(0.0, 1.0),
        )
    }
}

/// Radii of a town (at least 3 bearings, 1 m minimum).
pub fn radii_of(town: &Town) -> Vec<f32> {
    let radii: Vec<f32> = town.radii.iter().map(|r| r.max(1.0)).collect();
    if radii.len() < 3 {
        vec![60.0; 4]
    } else {
        radii
    }
}

fn roof_color(kind: &str, rng: &mut Rng) -> [f32; 3] {
    let base = if kind == "village" {
        ROOF_THATCH
    } else {
        ROOF_TILE
    };
    let k = rng.range(-0.05, 0.05);
    [
        (base[0] + k).clamp(0.0, 1.0),
        (base[1] + k * 0.7).clamp(0.0, 1.0),
        (base[2] + k * 0.5).clamp(0.0, 1.0),
    ]
}

fn cathedral_tower_m(size: f32) -> f32 {
    let t = ((size - 60.0) / 70.0).clamp(0.0, 1.0);
    CATHEDRAL_TOWER_M.0 + (CATHEDRAL_TOWER_M.1 - CATHEDRAL_TOWER_M.0) * t
}

/// Wall parameters of the far enclosure: `towns_1340.json` → `walls.<kind>` over the defaults.
pub fn wall_params(kind: &str, spec: Option<&WallSpec>) -> WallSpec {
    let mut w = if kind == "palisade" {
        WallSpec {
            height_m: 4.5,
            thickness_m: 0.6,
            ..Default::default()
        }
    } else {
        WallSpec {
            height_m: 9.0,
            thickness_m: 2.2,
            tower_height_m: 14.0,
            tower_radius_m: 4.5,
            ..Default::default()
        }
    };
    if let Some(s) = spec {
        if s.height_m > 0.0 {
            w.height_m = s.height_m;
        }
        if s.thickness_m > 0.0 {
            w.thickness_m = s.thickness_m;
        }
        if s.tower_height_m > 0.0 {
            w.tower_height_m = s.tower_height_m;
        }
        if s.tower_radius_m > 0.0 {
            w.tower_radius_m = s.tower_radius_m;
        }
    }
    w
}

// --- API -------------------------------------------------------------------------------------

/// Complete far mesh (F1) of a town of `towns_1340.json`.
pub fn build_f1(
    town: &Town,
    town_index: u32,
    meters_per_unit: f32,
    ground: &Ground,
    walls: &WallSpec,
) -> FarMesh {
    let mut acc = Acc::new(town.px, meters_per_unit, town_index);
    let radii = radii_of(town);
    let mut rng = Rng::new(town.seed.max(1));
    let kind = town.kind.as_str();
    let walled = town.walls == "stone" || town.walls == "palisade";
    let roof = roof_color(kind, &mut rng);
    nappe(
        &mut acc,
        ground,
        &radii,
        2,
        eave_m(kind),
        if walled { WALLED_EDGE } else { 1.0 },
        roof,
        Some(&mut rng),
    );
    for f in &town.faubourgs {
        faubourg(&mut acc, ground, f.bearing, f.start_m, f.length_m, roof);
    }
    if walled {
        let gates: Vec<f32> = town.gates.iter().map(|g| g.bearing).collect();
        enclosure(
            &mut acc,
            ground,
            &radii,
            &gates,
            walls,
            town.walls == "palisade",
        );
    }
    let mut has_cathedral = false;
    for m in &town.monuments {
        monument(&mut acc, ground, &m.kind, m.at, m.size_m, m.yaw);
        has_cathedral |= m.kind == "cathedral";
    }
    if kind == "city" || kind == "town" {
        let count = (town.parishes as i32 - i32::from(has_cathedral)).min(PARISH_MAX);
        for _ in 0..count.max(0) {
            let a = rng.randf() * TAU;
            let p = Vec2::new(a.cos(), a.sin()) * radius_at(&radii, a) * rng.range(0.3, 0.75);
            spire_tower(
                &mut acc,
                ground,
                p,
                PARISH_HALF_M,
                PARISH_TOWER_M,
                PARISH_SPIRE_M,
                CHURCH_STONE,
                LEAD,
            );
        }
    }
    acc.finish()
}

/// 16-sided polygon + skirt + one spire at most (cathedral, else keep).
pub fn build_f2(town: &Town, town_index: u32, meters_per_unit: f32, ground: &Ground) -> FarMesh {
    let mut acc = Acc::new(town.px, meters_per_unit, town_index);
    let radii = radii_of(town);
    let mut rng = Rng::new(town.seed.max(1));
    let kind = town.kind.as_str();
    let roof = roof_color(kind, &mut rng);
    let sub: Vec<f32> = (0..F2_SIDES)
        .map(|k| radius_at(&radii, k as f32 / F2_SIDES as f32 * TAU))
        .collect();
    nappe(&mut acc, ground, &sub, 1, eave_m(kind), 1.0, roof, None);
    let best = town
        .monuments
        .iter()
        .find(|m| m.kind == "cathedral")
        .or_else(|| town.monuments.iter().find(|m| m.kind == "castle"));
    if let Some(m) = best {
        if m.kind == "cathedral" {
            pyramid(
                &mut acc,
                ground,
                m.at,
                m.size_m * 0.08,
                0.0,
                cathedral_tower_m(m.size_m) + CATHEDRAL_SPIRE_M,
                LEAD,
            );
        } else {
            let half = m.size_m * KEEP_FRACTION * 0.5;
            cuboid(
                &mut acc,
                ground,
                m.at,
                Vec2::X,
                half,
                half,
                KEEP_HEIGHT_M,
                STONE,
                false,
            );
        }
    }
    acc.finish()
}

// --- Sheet and skirt -------------------------------------------------------------------------

/// Polar sheet: centre + `rings` rings (1: edge only; 2: 0.5 r and edge) + skirt at the edge.
fn nappe(
    acc: &mut Acc,
    ground: &Ground,
    radii: &[f32],
    rings: usize,
    eave: f32,
    edge: f32,
    roof: [f32; 3],
    mut rng: Option<&mut Rng>,
) {
    let n = radii.len();
    let v0 = acc.pm.len();
    let i0 = acc.out.indices.len();
    let roof_l = lin(roof);
    let center = acc.vert(
        Vec2::ZERO,
        ground.at(Vec2::ZERO),
        eave + CROWN_M,
        Vec3::Y,
        roof_l,
    );
    let mut ring_ids: Vec<Vec<u32>> = Vec::new();
    let mut edge_pts = Vec::new();
    let mut edge_rel = Vec::new();
    for ring in 0..rings {
        let last = ring == rings - 1;
        let f = if last { edge } else { INNER_RING };
        let crown = if last { 0.0 } else { CROWN_M * 0.5 };
        let mut ids = Vec::with_capacity(n);
        for (k, r) in radii.iter().enumerate() {
            let a = k as f32 / n as f32 * TAU;
            let p = Vec2::new(a.cos(), a.sin()) * r * f;
            let jitter = rng
                .as_deref_mut()
                .map_or(0.0, |r| r.range(-EAVE_JITTER_M, EAVE_JITTER_M));
            let rel = eave + crown + jitter;
            ids.push(acc.vert(p, ground.at(p), rel, Vec3::Y, roof_l));
            if last {
                edge_pts.push(p);
                edge_rel.push(rel);
            }
        }
        ring_ids.push(ids);
    }
    for k in 0..n {
        let k1 = (k + 1) % n;
        acc.tri(center, ring_ids[0][k], ring_ids[0][k1], Vec3::Y);
        for ring in 1..rings {
            let (a, b) = (ring_ids[ring - 1].clone(), ring_ids[ring].clone());
            acc.tri(a[k], b[k], b[k1], Vec3::Y);
            acc.tri(a[k], b[k1], a[k1], Vec3::Y);
        }
    }
    acc.smooth(v0, acc.pm.len(), i0);
    skirt(acc, ground, &edge_pts, &edge_rel, FACADE);
}

/// Skirt of a closed outline from the edge (`top_rel`) down to ground - SKIRT_M, radial normals.
fn skirt(acc: &mut Acc, ground: &Ground, pts: &[Vec2], top_rel: &[f32], color: [f32; 3]) {
    let n = pts.len();
    for k in 0..n {
        let k1 = (k + 1) % n;
        let (a, b) = (pts[k], pts[k1]);
        let out = (a + b).normalize_or_zero();
        let (ga, gb) = (ground.at(a), ground.at(b));
        acc.face(
            &[
                (a, ga, -SKIRT_M),
                (b, gb, -SKIRT_M),
                (b, gb, top_rel[k1]),
                (a, ga, top_rel[k]),
            ],
            Vec3::new(out.x, 0.0, out.y),
            color,
        );
    }
}

// --- Suburbs, enclosure ----------------------------------------------------------------------

fn faubourg(
    acc: &mut Acc,
    ground: &Ground,
    bearing_deg: f32,
    start: f32,
    length: f32,
    roof: [f32; 3],
) {
    if length < 5.0 {
        return;
    }
    let a = bearing_deg.to_radians();
    let u = Vec2::new(a.cos(), a.sin());
    let parts = if length > FAUBOURG_SPLIT_M { 2 } else { 1 };
    for i in 0..parts {
        let p0 = u * (start + length * i as f32 / parts as f32);
        let p1 = u * (start + length * (i + 1) as f32 / parts as f32);
        gable_block(
            acc,
            ground,
            p0,
            p1,
            FAUBOURG_WIDTH_M * 0.5,
            FAUBOURG_EAVE_M,
            FAUBOURG_RIDGE_M,
            roof,
            FACADE,
        );
    }
}

/// Enclosure at the radius: outer face (down to the skirt foot) and wall walk; towers every
/// `TOWER_EVERY` bearings, gates at `gates` (degrees).
fn enclosure(
    acc: &mut Acc,
    ground: &Ground,
    radii: &[f32],
    gates: &[f32],
    params: &WallSpec,
    palisade: bool,
) {
    let n = radii.len();
    let height = params.height_m;
    let half = params.thickness_m * 0.5;
    let color = if palisade { TIMBER } else { STONE };
    for k in 0..n {
        let k1 = (k + 1) % n;
        let a0 = k as f32 / n as f32 * TAU;
        let a1 = (k + 1) as f32 / n as f32 * TAU;
        let d0 = Vec2::new(a0.cos(), a0.sin());
        let d1 = Vec2::new(a1.cos(), a1.sin());
        let o0 = d0 * (radii[k] + half);
        let o1 = d1 * (radii[k1] + half);
        let i0 = d0 * (radii[k] - half);
        let i1 = d1 * (radii[k1] - half);
        let (go0, go1, gi0, gi1) = (ground.at(o0), ground.at(o1), ground.at(i0), ground.at(i1));
        let mid = (d0 + d1).normalize_or_zero();
        acc.face(
            &[
                (o0, go0, -SKIRT_M),
                (o1, go1, -SKIRT_M),
                (o1, go1, height),
                (o0, go0, height),
            ],
            Vec3::new(mid.x, 0.0, mid.y),
            color,
        );
        acc.face(
            &[
                (o0, go0, height),
                (o1, go1, height),
                (i1, gi1, go1 + height - gi1),
                (i0, gi0, go0 + height - gi0),
            ],
            Vec3::Y,
            color,
        );
    }
    if !palisade && params.tower_height_m > height && params.tower_radius_m > 0.0 {
        for k in (0..n).step_by(TOWER_EVERY) {
            let a = k as f32 / n as f32 * TAU;
            let d = Vec2::new(a.cos(), a.sin());
            cuboid(
                acc,
                ground,
                d * (radii[k] + half),
                d,
                params.tower_radius_m,
                params.tower_radius_m,
                params.tower_height_m,
                STONE,
                true,
            );
        }
    }
    for bearing in gates {
        let ag = bearing.to_radians();
        let dg = Vec2::new(ag.cos(), ag.sin());
        cuboid(
            acc,
            ground,
            dg * radius_at(radii, ag),
            dg,
            GATE_SIZE_M * 0.5,
            GATE_SIZE_M * 0.5,
            height + if palisade { 0.0 } else { GATE_EXTRA_M },
            color,
            true,
        );
    }
}

// --- F1 monuments ----------------------------------------------------------------------------

fn monument(acc: &mut Acc, ground: &Ground, kind: &str, at: Vec2, size: f32, yaw: f32) {
    let u = Vec2::new(yaw.cos(), yaw.sin());
    match kind {
        "cathedral" => {
            let half_len = size * 0.5;
            gable_block(
                acc,
                ground,
                at - u * half_len,
                at + u * half_len,
                size * CATHEDRAL_WIDTH * 0.5,
                CATHEDRAL_EAVE_M,
                CATHEDRAL_RIDGE_M,
                LEAD,
                CHURCH_STONE,
            );
            let half_t = size * 0.07;
            spire_tower(
                acc,
                ground,
                at - u * (half_len - half_t),
                half_t,
                cathedral_tower_m(size),
                CATHEDRAL_SPIRE_M,
                CHURCH_STONE,
                LEAD,
            );
        }
        "abbey" => {
            let church_len = size * ABBEY_CHURCH;
            let width = church_len * 0.25;
            gable_block(
                acc,
                ground,
                at - u * church_len * 0.5,
                at + u * church_len * 0.5,
                width * 0.5,
                ABBEY_EAVE_M,
                ABBEY_RIDGE_M,
                LEAD,
                CHURCH_STONE,
            );
            spire_tower(
                acc,
                ground,
                at,
                width * 0.35,
                ABBEY_TOWER_M,
                ABBEY_SPIRE_M,
                CHURCH_STONE,
                LEAD,
            );
        }
        "castle" => {
            let half = size * KEEP_FRACTION * 0.5;
            cuboid(acc, ground, at, u, half, half, KEEP_HEIGHT_M, STONE, true);
        }
        "hall" => gable_block(
            acc,
            ground,
            at - u * size * 0.5,
            at + u * size * 0.5,
            size * 0.25,
            HALL_EAVE_M,
            HALL_RIDGE_M,
            ROOF_TILE,
            STONE,
        ),
        "windmill" => spire_tower(
            acc,
            ground,
            at,
            size * 0.5,
            WINDMILL_HEIGHT_M,
            WINDMILL_CAP_M,
            TIMBER,
            TIMBER,
        ),
        _ => cuboid(
            acc,
            ground,
            at,
            u,
            size * 0.3,
            size * 0.3,
            HALL_EAVE_M,
            STONE,
            true,
        ),
    }
}

// --- Primitives ------------------------------------------------------------------------------

/// Block of axis `u`, from the skirt foot to `top_rel` above the ground under its centre (flat
/// top); lid if `cap`.
fn cuboid(
    acc: &mut Acc,
    ground: &Ground,
    center: Vec2,
    u: Vec2,
    half_len: f32,
    half_w: f32,
    top_rel: f32,
    color: [f32; 3],
    cap: bool,
) {
    let v = Vec2::new(-u.y, u.x);
    let top = ground.at(center) + top_rel;
    let c = [
        center - u * half_len - v * half_w,
        center + u * half_len - v * half_w,
        center + u * half_len + v * half_w,
        center - u * half_len + v * half_w,
    ];
    let g: Vec<f32> = c.iter().map(|p| ground.at(*p)).collect();
    for i in 0..4 {
        let j = (i + 1) % 4;
        let out = ((c[i] + c[j]) * 0.5 - center).normalize_or_zero();
        acc.face(
            &[
                (c[i], g[i], -SKIRT_M),
                (c[j], g[j], -SKIRT_M),
                (c[j], g[j], top - g[j]),
                (c[i], g[i], top - g[i]),
            ],
            Vec3::new(out.x, 0.0, out.y),
            color,
        );
    }
    if cap {
        acc.face(
            &[
                (c[0], g[0], top - g[0]),
                (c[1], g[1], top - g[1]),
                (c[2], g[2], top - g[2]),
                (c[3], g[3], top - g[3]),
            ],
            Vec3::Y,
            color,
        );
    }
}

/// Square pyramid (side `2 * half`) from `base_rel` to `apex_rel` above the central ground.
fn pyramid(
    acc: &mut Acc,
    ground: &Ground,
    center: Vec2,
    half: f32,
    base_rel: f32,
    apex_rel: f32,
    color: [f32; 3],
) {
    let gc = ground.at(center);
    let c = [
        center + Vec2::new(-half, -half),
        center + Vec2::new(half, -half),
        center + Vec2::new(half, half),
        center + Vec2::new(-half, half),
    ];
    for i in 0..4 {
        let (a, b) = (c[i], c[(i + 1) % 4]);
        let out = ((a + b) * 0.5 - center).normalize_or_zero();
        let (ga, gb) = (ground.at(a), ground.at(b));
        acc.face(
            &[
                (a, ga, gc + base_rel - ga),
                (b, gb, gc + base_rel - gb),
                (center, gc, apex_rel),
            ],
            Vec3::new(out.x, 0.3, out.y),
            color,
        );
    }
}

fn spire_tower(
    acc: &mut Acc,
    ground: &Ground,
    center: Vec2,
    half: f32,
    tower_rel: f32,
    spire_m: f32,
    wall: [f32; 3],
    spire: [f32; 3],
) {
    cuboid(
        acc,
        ground,
        center,
        Vec2::X,
        half,
        half,
        tower_rel,
        wall,
        false,
    );
    pyramid(
        acc,
        ground,
        center,
        half,
        tower_rel,
        tower_rel + spire_m,
        spire,
    );
}

/// Gable block from `p0` to `p1` (ridge along the axis), half width `half_w`; walls from the skirt
/// foot to the eave; heights relative to the ground of each end. 14 triangles.
fn gable_block(
    acc: &mut Acc,
    ground: &Ground,
    p0: Vec2,
    p1: Vec2,
    half_w: f32,
    eave: f32,
    ridge: f32,
    roof: [f32; 3],
    wall: [f32; 3],
) {
    let axis = p1 - p0;
    if axis.length_squared() < 1e-4 {
        return;
    }
    let u = axis.normalize();
    let v = Vec2::new(-u.y, u.x);
    let (g0, g1) = (ground.at(p0), ground.at(p1));
    let (a0, b0, a1, b1) = (
        p0 - v * half_w,
        p0 + v * half_w,
        p1 - v * half_w,
        p1 + v * half_w,
    );
    let hu = Vec3::new(u.x, 0.0, u.y);
    let hv = Vec3::new(v.x, 0.0, v.y);
    let s = -SKIRT_M;
    acc.face(
        &[(a0, g0, s), (a1, g1, s), (a1, g1, eave), (a0, g0, eave)],
        -hv,
        wall,
    );
    acc.face(
        &[(b0, g0, s), (b1, g1, s), (b1, g1, eave), (b0, g0, eave)],
        hv,
        wall,
    );
    acc.face(
        &[(a0, g0, s), (b0, g0, s), (b0, g0, eave), (a0, g0, eave)],
        -hu,
        wall,
    );
    acc.face(
        &[(a0, g0, eave), (b0, g0, eave), (p0, g0, ridge)],
        -hu,
        wall,
    );
    acc.face(
        &[(a1, g1, s), (b1, g1, s), (b1, g1, eave), (a1, g1, eave)],
        hu,
        wall,
    );
    acc.face(&[(a1, g1, eave), (b1, g1, eave), (p1, g1, ridge)], hu, wall);
    acc.face(
        &[
            (a0, g0, eave),
            (a1, g1, eave),
            (p1, g1, ridge),
            (p0, g0, ridge),
        ],
        Vec3::new(-v.x, 1.0, -v.y),
        roof,
    );
    acc.face(
        &[
            (b0, g0, eave),
            (b1, g1, eave),
            (p1, g1, ridge),
            (p0, g0, ridge),
        ],
        Vec3::new(v.x, 1.0, v.y),
        roof,
    );
}

#[cfg(test)]
mod tests {
    use super::super::super::data::{Faubourg, Gate, MonumentSpec};
    use super::*;

    fn town(kind: &str, walls: &str) -> Town {
        Town {
            id: "set_test".into(),
            kind: kind.into(),
            seed: 7,
            population: 8000,
            households: 1600,
            core_population: 6000,
            walls: walls.into(),
            radii: vec![300.0; 32],
            gates: vec![Gate {
                bearing: 90.0,
                kind: "main".into(),
            }],
            faubourgs: vec![Faubourg {
                bearing: 0.0,
                start_m: 300.0,
                length_m: 200.0,
            }],
            monuments: vec![MonumentSpec {
                kind: "cathedral".into(),
                at: Vec2::new(20.0, 10.0),
                size_m: 100.0,
                yaw: 0.3,
                keep: false,
            }],
            parishes: 3,
            river: None,
            bridge: None,
            px: Vec2::new(1000.0, 2000.0),
        }
    }

    /// Every triangle faces its outward hint: the up faces of the sheet have an upward normal.
    fn upward_share(m: &FarMesh) -> f32 {
        let mut up = 0;
        for t in m.indices.as_chunks::<3>().0 {
            let p = |i: u32| {
                let v = m.positions[i as usize];
                Vec3::new((v[0] - 1000.0) * 719.0, v[1], (v[2] - 2000.0) * 719.0)
            };
            let n = (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0]));
            if n.y > 0.0 {
                up += 1;
            }
        }
        up as f32 / m.triangles() as f32
    }

    #[test]
    fn f1_is_richer_than_f2_and_well_formed() {
        let t = town("city", "stone");
        let radii = radii_of(&t);
        let ground = Ground::sample(&radii, |p| 50.0 + p.x * 0.01);
        let walls = wall_params("stone", None);
        let f1 = build_f1(&t, 3, 719.0, &ground, &walls);
        let f2 = build_f2(&t, 3, 719.0, &ground);
        assert!(
            f1.triangles() > 4 * f2.triangles(),
            "{} vs {}",
            f1.triangles(),
            f2.triangles()
        );
        for m in [&f1, &f2] {
            assert_eq!(m.positions.len(), m.uv1.len());
            assert!(m.indices.iter().all(|i| (*i as usize) < m.positions.len()));
            assert!(m.uv1.iter().all(|uv| uv[0] == 3.0));
            assert!(m.uv1.iter().any(|uv| uv[1] == -SKIRT_M));
            // Vertex ground = y - rel: on the sampled relief (40..60 m here).
            for (p, uv) in m.positions.iter().zip(&m.uv1) {
                let g = p[1] - uv[1];
                assert!((40.0..=60.0).contains(&g), "ground {g}");
            }
        }
        // The cathedral spire is the top (60-90 m tower + 25 m).
        assert!(f1.y_max - 50.0 > 80.0);
        // A good share of the triangles face up (sheet, roofs, wall walk).
        assert!(upward_share(&f2) > 0.3, "{}", upward_share(&f2));
        assert!(upward_share(&f1) > 0.3);
    }

    #[test]
    fn walls_add_triangles_and_palisade_has_no_towers() {
        let ground = Ground {
            z: 10.0,
            ..Default::default()
        };
        let open = build_f1(
            &town("town", "none"),
            0,
            719.0,
            &ground,
            &wall_params("stone", None),
        );
        let stone = build_f1(
            &town("town", "stone"),
            0,
            719.0,
            &ground,
            &wall_params("stone", None),
        );
        let pal = build_f1(
            &town("town", "palisade"),
            0,
            719.0,
            &ground,
            &wall_params("palisade", None),
        );
        assert!(stone.triangles() > pal.triangles());
        assert!(pal.triangles() > open.triangles());
    }

    #[test]
    fn ground_interpolates_rings() {
        let radii = vec![100.0; 8];
        let g = Ground::sample(&radii, |p| p.length());
        assert!((g.at(Vec2::ZERO) - 0.0).abs() < 1e-3);
        assert!((g.at(Vec2::new(50.0, 0.0)) - 50.0).abs() < 1e-2);
        assert!((g.at(Vec2::new(100.0, 0.0)) - 100.0).abs() < 1e-2);
        // Beyond the radius: clamped to the edge ring.
        assert!((g.at(Vec2::new(300.0, 0.0)) - 100.0).abs() < 1e-2);
    }

    #[test]
    fn append_offsets_indices() {
        let ground = Ground::default();
        let t = town("village", "none");
        let a = build_f2(&t, 0, 719.0, &ground);
        let mut all = FarMesh::default();
        all.append(&a);
        all.append(&a);
        assert_eq!(all.triangles(), 2 * a.triangles());
        assert_eq!(
            all.indices[a.indices.len()],
            a.indices[0] + a.positions.len() as u32
        );
    }
}
