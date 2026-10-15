//! Parametric templates of the monuments of the emblematic 1:1 cities (ADR 0078): cathedrals,
//! churches, abbeys, castles, belfries, halls, palaces, enclosures, earthworks, towers. Port of
//! `landmark_monuments.gd`. Pure geometry (any thread): [`build`] returns triangles with outward
//! normals and per-vertex colours (the layer colour times a tint) in the monument frame:
//! +X along the axis (churches: towards the choir), +Z to the right of the axis, +Y up, origin at
//! the centre of the footprint, y = 0 at the base (walls sink to `-FOUNDATION_M` for slopes).

use bevy::math::{Mat4, Quat, Vec2, Vec3};
use serde_json::Value;

use super::landmark_v2::num;

pub const FOUNDATION_M: f32 = 8.0;
const WALL: &str = "Ashlar";
const MASONRY: &str = "Masonry";
const ROOF: &str = "RoofSlate";
const ROOF_TILE: &str = "RoofTile";
const RUBBLE: &str = "Rubble";
const TIMBER: &str = "Timber";
const STONE_TINT: [f32; 3] = [0.97, 0.94, 0.86];
const RUBBLE_TINT: [f32; 3] = [0.9, 0.86, 0.78];
const EARTH_TINT: [f32; 3] = [0.62, 0.52, 0.38];
const WOOD_TINT: [f32; 3] = [0.72, 0.6, 0.46];
const GLASS_TINT: [f32; 3] = [0.16, 0.19, 0.26];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const TAU: f32 = std::f32::consts::TAU;
const PI: f32 = std::f32::consts::PI;

/// Base colour of a material layer (the atlas layers of the Godot version).
pub fn layer_rgb(layer: &str) -> [f32; 3] {
    match layer {
        "Ashlar" => [0.74, 0.70, 0.63],
        "Masonry" => [0.66, 0.63, 0.58],
        "RoofSlate" => [0.30, 0.33, 0.38],
        "RoofTile" => [0.60, 0.34, 0.26],
        "Rubble" => [0.55, 0.50, 0.42],
        "Timber" => [0.45, 0.34, 0.24],
        "Plaster" => [0.70, 0.65, 0.58],
        _ => [0.6, 0.6, 0.6],
    }
}

/// Vertex colour of a face: rgb tint over the atlas texture, alpha the atlas layer
/// (`(index + 0.5) / 16`, as baked in the kit glb files; `towns::atlas`).
fn color(layer: &str, tint: [f32; 3]) -> [f32; 4] {
    [tint[0], tint[1], tint[2], super::atlas::layer_alpha(layer)]
}

/// Triangle soup with flat normals and vertex colours.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
}

impl MeshData {
    pub fn triangles(&self) -> usize {
        self.positions.len() / 3
    }

    /// Lowest and highest vertex y.
    pub fn y_range(&self) -> (f32, f32) {
        self.positions
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p[1]), hi.max(p[1]))
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Built {
    pub mesh: MeshData,
    pub top: f32,
}

/// Rigid transform (rotation and translation) of the monument frame.
#[derive(Clone, Copy)]
struct Xf(Mat4);

impl Xf {
    const ID: Xf = Xf(Mat4::IDENTITY);

    fn new(yaw_about_y: f32, at: Vec3) -> Xf {
        Xf(Mat4::from_rotation_translation(
            Quat::from_rotation_y(yaw_about_y),
            at,
        ))
    }

    fn at(at: Vec3) -> Xf {
        Xf::new(0.0, at)
    }

    fn apply(&self, v: Vec3) -> Vec3 {
        self.0.transform_point3(v)
    }

    fn then(&self, other: Xf) -> Xf {
        Xf(self.0 * other.0)
    }
}

fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn f(p: &Value, key: &str, default: f32) -> f32 {
    num(&p[key], default)
}

fn s<'a>(p: &'a Value, key: &str, default: &'a str) -> &'a str {
    p[key].as_str().unwrap_or(default)
}

/// Footprint (length along the axis, width) in metres, to reserve the ground.
pub fn footprint(m: &Value) -> Vec2 {
    let p = &m["params"];
    let ring_extent = |key: &str, default: [[f32; 2]; 4]| -> Vec2 {
        let mut lo = Vec2::splat(f32::INFINITY);
        let mut hi = Vec2::splat(f32::NEG_INFINITY);
        match p[key].as_array() {
            Some(a) if !a.is_empty() => {
                for q in a {
                    let q = Vec2::new(num(&q[0], 0.0), num(&q[1], 0.0));
                    lo = lo.min(q);
                    hi = hi.max(q);
                }
            }
            _ => {
                for q in default {
                    lo = lo.min(Vec2::from(q));
                    hi = hi.max(Vec2::from(q));
                }
            }
        }
        hi - lo
    };
    match s(m, "model", "") {
        "gothic_cathedral" => Vec2::new(
            f(p, "length_m", 100.0),
            f(p, "width_m", 30.0).max(f(p, "transept_m", 0.0)),
        ),
        "church" => {
            let extra = if s(&p["tower"], "at", "") == "west" {
                f(&p["tower"], "size", 0.0)
            } else {
                0.0
            };
            Vec2::new(f(p, "length_m", 30.0) + extra, f(p, "width_m", 14.0))
        }
        "abbey" => Vec2::new(
            f(p, "length_m", 40.0),
            f(p, "width_m", 16.0) + f(&p["cloister"], "size", 30.0) + 4.0,
        ),
        "castle" => {
            let r = f(p, "tower_radius_m", 5.0);
            ring_extent(
                "ring",
                [[-40.0, -40.0], [40.0, -40.0], [40.0, 40.0], [-40.0, 40.0]],
            ) + Vec2::splat(r * 2.0)
        }
        "earthwork" => {
            let b = f(p, "base_m", 9.0);
            ring_extent(
                "ring",
                [[-20.0, -20.0], [20.0, -20.0], [20.0, 20.0], [-20.0, 20.0]],
            ) + Vec2::splat(b)
        }
        "belfry" | "keep" | "tower" | "gate_tower" => {
            let sz = f(p, "size", f(p, "radius_m", 5.0) * 2.0);
            Vec2::splat(sz)
        }
        _ => Vec2::new(f(p, "length_m", 30.0), f(p, "width_m", 15.0)),
    }
}

/// Mesh of a monument and its top height (m).
pub fn build(m: &Value) -> Built {
    let mut st = St::default();
    let p = &m["params"];
    let top = match s(m, "model", "") {
        "gothic_cathedral" => cathedral(&mut st, p),
        "church" => church(&mut st, p, Xf::ID),
        "abbey" => abbey(&mut st, p),
        "castle" => castle(&mut st, p),
        "belfry" => belfry(&mut st, p),
        "hall" => hall(&mut st, p),
        "royal_palace" => palace(&mut st, p),
        "enclosure" => enclosure(&mut st, p),
        "earthwork" => earthwork(&mut st, p),
        "keep" | "tower" | "gate_tower" => {
            let r = f(p, "radius_m", f(p, "size", 10.0) * 0.5);
            let h = f(p, "height", f(p, "height_m", 20.0));
            cylinder(
                &mut st,
                Xf::ID,
                r,
                -FOUNDATION_M,
                h,
                14,
                MASONRY,
                STONE_TINT,
            );
            cone(&mut st, Xf::ID, r * 1.1, h, h + r * 1.4, 14, ROOF);
            h + r * 1.4
        }
        _ => 0.0,
    };
    Built { mesh: st.0, top }
}

// --- Primitives -----------------------------------------------------------------------------

#[derive(Default)]
struct St(MeshData);

/// Triangle oriented away from `reference` (flat normal).
fn tri_oriented(st: &mut St, a: Vec3, mut b: Vec3, mut c: Vec3, col: [f32; 4], reference: Vec3) {
    let mut n = (b - a).cross(c - a);
    if n.length_squared() < 1e-10 {
        return;
    }
    if n.dot((a + b + c) / 3.0 - reference) < 0.0 {
        std::mem::swap(&mut b, &mut c);
        n = -n;
    }
    let n = n.normalize();
    for q in [a, b, c] {
        st.0.positions.push(q.to_array());
        st.0.normals.push(n.to_array());
        st.0.colors.push(col);
    }
}

#[allow(clippy::too_many_arguments)]
fn quad_oriented(
    st: &mut St,
    t: Xf,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    d: Vec3,
    layer: &str,
    tint: [f32; 3],
    reference: Vec3,
) {
    let col = color(layer, tint);
    let (ra, rb, rc, rd, rr) = (
        t.apply(a),
        t.apply(b),
        t.apply(c),
        t.apply(d),
        t.apply(reference),
    );
    tri_oriented(st, ra, rb, rc, col, rr);
    tri_oriented(st, ra, rc, rd, col, rr);
}

/// Box: foot at `center` (x, z; y = start), height `size.y` above `center.y`, sides
/// `size.x` x `size.z`; the walls go down to `bottom`.
#[allow(clippy::too_many_arguments)]
fn box_(st: &mut St, t: Xf, center: Vec3, size: Vec3, layer: &str, tint: [f32; 3], bottom: f32) {
    let (hx, hz) = (size.x * 0.5, size.z * 0.5);
    let (y0, y1) = (bottom, center.y + size.y);
    let c = [
        v3(-hx, 0.0, -hz),
        v3(hx, 0.0, -hz),
        v3(hx, 0.0, hz),
        v3(-hx, 0.0, hz),
    ];
    let mid = v3(center.x, (y0 + y1) * 0.5, center.z);
    for i in 0..4 {
        let a = center + c[i];
        let b = center + c[(i + 1) % 4];
        quad_oriented(
            st,
            t,
            v3(a.x, y0, a.z),
            v3(b.x, y0, b.z),
            v3(b.x, y1, b.z),
            v3(a.x, y1, a.z),
            layer,
            tint,
            mid,
        );
    }
    let p = |i: usize| {
        let q = center + c[i];
        v3(q.x, y1, q.z)
    };
    quad_oriented(st, t, p(0), p(1), p(2), p(3), layer, tint, mid);
}

fn bx(st: &mut St, t: Xf, center: Vec3, size: Vec3, layer: &str, tint: [f32; 3]) {
    box_(st, t, center, size, layer, tint, -FOUNDATION_M);
}

/// Gable roof along X from `x0` to `x1`, half width `half_w`, eaves `eave`, ridge `ridge`;
/// masonry gables at the requested ends.
#[allow(clippy::too_many_arguments)]
fn gable_roof_x(
    st: &mut St,
    t: Xf,
    x0: f32,
    x1: f32,
    half_w: f32,
    eave: f32,
    ridge: f32,
    layer: &str,
    gable0: bool,
    gable1: bool,
) {
    let r = v3((x0 + x1) * 0.5, eave, 0.0);
    quad_oriented(
        st,
        t,
        v3(x0, eave, -half_w),
        v3(x1, eave, -half_w),
        v3(x1, ridge, 0.0),
        v3(x0, ridge, 0.0),
        layer,
        WHITE,
        r,
    );
    quad_oriented(
        st,
        t,
        v3(x0, eave, half_w),
        v3(x1, eave, half_w),
        v3(x1, ridge, 0.0),
        v3(x0, ridge, 0.0),
        layer,
        WHITE,
        r,
    );
    let wall = color(WALL, STONE_TINT);
    if gable0 {
        tri_oriented(
            st,
            t.apply(v3(x0, eave, -half_w)),
            t.apply(v3(x0, eave, half_w)),
            t.apply(v3(x0, ridge, 0.0)),
            wall,
            t.apply(r),
        );
    }
    if gable1 {
        tri_oriented(
            st,
            t.apply(v3(x1, eave, -half_w)),
            t.apply(v3(x1, eave, half_w)),
            t.apply(v3(x1, ridge, 0.0)),
            wall,
            t.apply(r),
        );
    }
}

fn pyramid(st: &mut St, t: Xf, half: f32, y0: f32, y1: f32, layer: &str) {
    let col = color(layer, WHITE);
    let c = [
        v3(-half, y0, -half),
        v3(half, y0, -half),
        v3(half, y0, half),
        v3(-half, y0, half),
    ];
    let apex = t.apply(v3(0.0, y1, 0.0));
    let r = t.apply(v3(0.0, y0, 0.0));
    for i in 0..4 {
        tri_oriented(st, t.apply(c[i]), t.apply(c[(i + 1) % 4]), apex, col, r);
    }
}

#[allow(clippy::too_many_arguments)]
fn arc_wall(
    st: &mut St,
    t: Xf,
    r: f32,
    y0: f32,
    y1: f32,
    sides: u32,
    a0: f32,
    a1: f32,
    layer: &str,
    tint: [f32; 3],
) {
    let col = color(layer, tint);
    let reference = t.apply(v3(0.0, (y0 + y1) * 0.5, 0.0));
    for i in 0..sides {
        let u0 = a0 + (a1 - a0) * i as f32 / sides as f32;
        let u1 = a0 + (a1 - a0) * (i + 1) as f32 / sides as f32;
        let p0 = v3(u0.cos() * r, 0.0, u0.sin() * r);
        let p1 = v3(u1.cos() * r, 0.0, u1.sin() * r);
        let q0 = t.apply(p0 + v3(0.0, y0, 0.0));
        let q1 = t.apply(p1 + v3(0.0, y0, 0.0));
        let q2 = t.apply(p1 + v3(0.0, y1, 0.0));
        let q3 = t.apply(p0 + v3(0.0, y1, 0.0));
        tri_oriented(st, q0, q1, q2, col, reference);
        tri_oriented(st, q0, q2, q3, col, reference);
    }
}

#[allow(clippy::too_many_arguments)]
fn cylinder(st: &mut St, t: Xf, r: f32, y0: f32, y1: f32, sides: u32, layer: &str, tint: [f32; 3]) {
    arc_wall(st, t, r, y0, y1, sides, 0.0, TAU, layer, tint);
}

#[allow(clippy::too_many_arguments)]
fn half_cylinder(
    st: &mut St,
    t: Xf,
    r: f32,
    y0: f32,
    y1: f32,
    sides: u32,
    layer: &str,
    tint: [f32; 3],
) {
    arc_wall(st, t, r, y0, y1, sides, -PI * 0.5, PI * 0.5, layer, tint);
}

#[allow(clippy::too_many_arguments)]
fn frustum(
    st: &mut St,
    t: Xf,
    r0: f32,
    y0: f32,
    r1: f32,
    y1: f32,
    sides: u32,
    a0: f32,
    a1: f32,
    layer: &str,
) {
    let col = color(layer, WHITE);
    let reference = t.apply(v3(0.0, y0, 0.0));
    for i in 0..sides {
        let u0 = a0 + (a1 - a0) * i as f32 / sides as f32;
        let u1 = a0 + (a1 - a0) * (i + 1) as f32 / sides as f32;
        let b0 = t.apply(v3(u0.cos() * r0, y0, u0.sin() * r0));
        let b1 = t.apply(v3(u1.cos() * r0, y0, u1.sin() * r0));
        let c0 = t.apply(v3(u0.cos() * r1, y1, u0.sin() * r1));
        let c1 = t.apply(v3(u1.cos() * r1, y1, u1.sin() * r1));
        tri_oriented(st, b0, b1, c1, col, reference);
        if r1 > 0.01 {
            tri_oriented(st, b0, c1, c0, col, reference);
        }
    }
}

fn cone(st: &mut St, t: Xf, r: f32, y0: f32, y1: f32, sides: u32, layer: &str) {
    frustum(st, t, r, y0, 0.0, y1, sides, 0.0, TAU, layer);
}

#[allow(clippy::too_many_arguments)]
fn half_cone(st: &mut St, t: Xf, r0: f32, y0: f32, r1: f32, y1: f32, sides: u32, layer: &str) {
    frustum(st, t, r0, y0, r1, y1, sides, -PI * 0.5, PI * 0.5, layer);
}

/// Pointed-arch bay on a wall of normal +-Z (`side`).
#[allow(clippy::too_many_arguments)]
fn lancet(st: &mut St, t: Xf, x: f32, z: f32, y0: f32, y1: f32, w: f32, side: f32) {
    let spring = y1 - w * 0.75;
    let reference = v3(x, (y0 + y1) * 0.5, z - side);
    quad_oriented(
        st,
        t,
        v3(x - w * 0.5, y0, z),
        v3(x + w * 0.5, y0, z),
        v3(x + w * 0.5, spring, z),
        v3(x - w * 0.5, spring, z),
        MASONRY,
        GLASS_TINT,
        reference,
    );
    let col = color(MASONRY, GLASS_TINT);
    tri_oriented(
        st,
        t.apply(v3(x - w * 0.5, spring, z)),
        t.apply(v3(x + w * 0.5, spring, z)),
        t.apply(v3(x, y1, z)),
        col,
        t.apply(reference),
    );
}

/// Same bay on a wall of normal +-X (west or east facade), centred on `z`.
#[allow(clippy::too_many_arguments)]
fn lancet_x(st: &mut St, t: Xf, x: f32, z: f32, y0: f32, y1: f32, w: f32, side: f32) {
    let spring = y1 - w * 0.75;
    let reference = v3(x - side, (y0 + y1) * 0.5, z);
    quad_oriented(
        st,
        t,
        v3(x, y0, z - w * 0.5),
        v3(x, y0, z + w * 0.5),
        v3(x, spring, z + w * 0.5),
        v3(x, spring, z - w * 0.5),
        MASONRY,
        GLASS_TINT,
        reference,
    );
    let col = color(MASONRY, GLASS_TINT);
    tri_oriented(
        st,
        t.apply(v3(x, spring, z - w * 0.5)),
        t.apply(v3(x, spring, z + w * 0.5)),
        t.apply(v3(x, y1, z)),
        col,
        t.apply(reference),
    );
}

/// Rose window (12-sided disc) on the west facade (normal -X).
fn rose(st: &mut St, t: Xf, c: Vec3, r: f32) {
    let col = color(MASONRY, GLASS_TINT);
    let reference = t.apply(c + v3(1.0, 0.0, 0.0));
    for k in 0..12 {
        let a0 = TAU * k as f32 / 12.0;
        let a1 = TAU * (k + 1) as f32 / 12.0;
        tri_oriented(
            st,
            t.apply(c),
            t.apply(c + v3(0.0, a0.sin(), a0.cos()) * r),
            t.apply(c + v3(0.0, a1.sin(), a1.cos()) * r),
            col,
            reference,
        );
    }
}

// --- Templates ------------------------------------------------------------------------------

fn cathedral(st: &mut St, p: &Value) -> f32 {
    let length = f(p, "length_m", 100.0);
    let width = f(p, "width_m", 30.0);
    let nave_w = f(p, "nave_width_m", 11.0);
    let vault = f(p, "vault_m", 25.0);
    let ridge = f(p, "ridge_m", vault + 10.0);
    let aisle = f(p, "aisle_m", 12.0);
    let round_apse = s(p, "apse", "round") == "round";
    let x0 = -length * 0.5;
    let r_apse = if round_apse { width * 0.5 } else { 0.0 };
    let x1 = length * 0.5 - r_apse;
    let id = Xf::ID;
    let aisle_top = aisle + (vault - aisle) * 0.45;
    for side in [-1.0_f32, 1.0] {
        let zc = side * (nave_w + (width - nave_w) * 0.5) * 0.5;
        let zc2 = side * (width * 0.5 + nave_w * 0.5) * 0.5;
        bx(
            st,
            id,
            v3((x0 + x1) * 0.5, 0.0, zc2),
            v3(x1 - x0, aisle, (width - nave_w) * 0.5),
            WALL,
            STONE_TINT,
        );
        let z_out = side * width * 0.5;
        let z_in = side * nave_w * 0.5;
        quad_oriented(
            st,
            id,
            v3(x0, aisle, z_out + side * 0.6),
            v3(x1, aisle, z_out + side * 0.6),
            v3(x1, aisle_top, z_in),
            v3(x0, aisle_top, z_in),
            ROOF,
            WHITE,
            v3((x0 + x1) * 0.5, 0.0, zc),
        );
        let bays = p["bays"]
            .as_f64()
            .map_or(4.max(((x1 - x0) / 9.0) as i32), |b| b as i32)
            .max(1);
        for b in 0..=bays {
            let bxp = (x0 + 2.0) + ((x1 - 2.0) - (x0 + 2.0)) * b as f32 / bays as f32;
            bx(
                st,
                id,
                v3(bxp, 0.0, side * (width * 0.5 + 1.2)),
                v3(1.8, aisle + 4.0, 2.4),
                WALL,
                STONE_TINT,
            );
            quad_oriented(
                st,
                id,
                v3(bxp - 0.5, aisle + 3.0, side * (width * 0.5 + 0.5)),
                v3(bxp + 0.5, aisle + 3.0, side * (width * 0.5 + 0.5)),
                v3(bxp + 0.5, vault - 3.0, side * nave_w * 0.5),
                v3(bxp - 0.5, vault - 3.0, side * nave_w * 0.5),
                WALL,
                STONE_TINT,
                v3(bxp, 0.0, 0.0),
            );
            if b < bays {
                let wx = (x0 + 2.0) + ((x1 - 2.0) - (x0 + 2.0)) * (b as f32 + 0.5) / bays as f32;
                let bay_w = (x1 - x0 - 4.0) / bays as f32;
                lancet(
                    st,
                    id,
                    wx,
                    side * (width * 0.5 + 0.05),
                    aisle * 0.25,
                    aisle * 0.82,
                    (bay_w * 0.4).min(2.8),
                    side,
                );
                lancet(
                    st,
                    id,
                    wx,
                    side * (nave_w * 0.5 + 0.05),
                    aisle_top + 1.0,
                    vault - 1.5,
                    (bay_w * 0.5).min(3.4),
                    side,
                );
            }
        }
    }
    bx(
        st,
        id,
        v3((x0 + x1) * 0.5, 0.0, 0.0),
        v3(x1 - x0, vault, nave_w),
        WALL,
        STONE_TINT,
    );
    let open_west = p["open_west"].as_bool().unwrap_or(false);
    gable_roof_x(
        st,
        id,
        x0,
        x1,
        nave_w * 0.5 + 0.6,
        vault,
        ridge,
        ROOF,
        !open_west,
        !round_apse,
    );
    lancet_x(
        st,
        id,
        x0 - 0.05,
        0.0,
        0.0,
        (vault * 0.36).min(11.0),
        (nave_w * 0.42).min(5.0),
        -1.0,
    );
    rose(
        st,
        id,
        v3(x0 - 0.06, vault * 0.62, 0.0),
        (nave_w * 0.32).min(vault * 0.16),
    );
    if round_apse {
        let c = Xf::at(v3(x1, 0.0, 0.0));
        half_cylinder(
            st,
            c,
            width * 0.5,
            -FOUNDATION_M,
            aisle,
            10,
            WALL,
            STONE_TINT,
        );
        half_cone(
            st,
            c,
            width * 0.5 + 0.6,
            aisle,
            nave_w * 0.5,
            aisle_top,
            10,
            ROOF,
        );
        half_cylinder(
            st,
            c,
            nave_w * 0.5,
            aisle_top - 0.5,
            vault,
            8,
            WALL,
            STONE_TINT,
        );
        half_cone(st, c, nave_w * 0.5 + 0.6, vault, 0.3, ridge, 8, ROOF);
        if p["chapels"].as_bool().unwrap_or(false) {
            for k in 0..5 {
                let a = -PI * 0.5 + PI * (k as f32 + 0.5) / 5.0;
                let cc = v3(
                    x1 + a.cos() * (width * 0.5 + 2.0),
                    0.0,
                    a.sin() * (width * 0.5 + 2.0),
                );
                cylinder(
                    st,
                    Xf::at(cc),
                    3.2,
                    -FOUNDATION_M,
                    aisle - 1.0,
                    8,
                    WALL,
                    STONE_TINT,
                );
                cone(st, Xf::at(cc), 3.6, aisle - 1.0, aisle + 2.5, 8, ROOF);
            }
        }
    }
    let t_len = f(p, "transept_m", 0.0);
    if t_len > width {
        let tw = f(p, "transept_width_m", nave_w + 2.0);
        let tx = x0 + length * f(p, "transept_at", 0.55);
        let t = Xf::new(PI * 0.5, v3(tx, 0.0, 0.0));
        bx(st, t, Vec3::ZERO, v3(t_len, vault, tw), WALL, STONE_TINT);
        gable_roof_x(
            st,
            t,
            -t_len * 0.5,
            t_len * 0.5,
            tw * 0.5 + 0.6,
            vault,
            ridge,
            ROOF,
            true,
            true,
        );
    }
    let mut top = ridge;
    if let Some(towers) = p["west_towers"].as_array() {
        for tower in towers {
            let size = f(tower, "size", 12.0);
            let h = f(tower, "height", vault + 20.0);
            let side_z = (if s(tower, "side", "north") == "north" {
                -1.0
            } else {
                1.0
            }) * (width * 0.5 - size * 0.5);
            let tc = v3(x0 + size * 0.5 - 1.0, 0.0, side_z);
            bx(st, id, tc, v3(size, h, size), WALL, STONE_TINT);
            lancet_x(
                st,
                id,
                tc.x - size * 0.5 - 0.05,
                tc.z,
                h * 0.72,
                h * 0.93,
                size * 0.3,
                -1.0,
            );
            let free_side = if side_z != 0.0 { side_z.signum() } else { 1.0 };
            lancet(
                st,
                id,
                tc.x,
                tc.z + free_side * (size * 0.5 + 0.05),
                h * 0.72,
                h * 0.93,
                size * 0.3,
                free_side,
            );
            let spire = f(tower, "spire_m", 0.0);
            match s(tower, "top", "flat") {
                "pyramid" | "spire" => {
                    let sh = spire.max(size * 0.7);
                    pyramid(st, Xf::at(tc), size * 0.5 + 0.4, h, h + sh, ROOF);
                    top = top.max(h + sh);
                }
                _ => top = top.max(h),
            }
        }
    }
    let crossing = &p["crossing"];
    if crossing.is_object() && !crossing.as_object().is_some_and(|o| o.is_empty()) {
        let cx = x0 + length * f(p, "transept_at", 0.55);
        let cs = f(crossing, "size", nave_w + 2.0);
        let ch = f(crossing, "height", ridge + 10.0);
        let cc = Xf::at(v3(cx, 0.0, 0.0));
        box_(
            st,
            cc,
            v3(0.0, ridge - 4.0, 0.0),
            v3(cs, ch - ridge + 4.0, cs),
            WALL,
            STONE_TINT,
            ridge - 4.0,
        );
        let spire = f(crossing, "spire_m", 0.0);
        if spire > 0.0 {
            cone(st, cc, cs * 0.5, ch, ch + spire, 8, ROOF);
        }
        top = top.max(ch + spire);
    }
    top
}

fn church(st: &mut St, p: &Value, t: Xf) -> f32 {
    let length = f(p, "length_m", 30.0);
    let width = f(p, "width_m", 14.0);
    let h = f(p, "height_m", 12.0);
    let ridge = h + width * 0.45;
    let choir_len = length * 0.3;
    let x0 = -length * 0.5;
    let xc = length * 0.5 - choir_len;
    bx(
        st,
        t,
        v3((x0 + xc) * 0.5, 0.0, 0.0),
        v3(xc - x0, h, width),
        WALL,
        STONE_TINT,
    );
    gable_roof_x(
        st,
        t,
        x0,
        xc,
        width * 0.5 + 0.5,
        h,
        ridge,
        ROOF,
        true,
        false,
    );
    let cw = width * 0.72;
    let ch = h * 0.9;
    let round = s(p, "apse", "flat") == "round";
    let mut x1 = length * 0.5;
    if round {
        x1 -= cw * 0.5;
    }
    bx(
        st,
        t,
        v3((xc + x1) * 0.5, 0.0, 0.0),
        v3(x1 - xc, ch, cw),
        WALL,
        STONE_TINT,
    );
    gable_roof_x(
        st,
        t,
        xc - 0.3,
        x1,
        cw * 0.5 + 0.5,
        ch,
        ch + cw * 0.45,
        ROOF,
        false,
        !round,
    );
    if round {
        let a = t.then(Xf::at(v3(x1, 0.0, 0.0)));
        half_cylinder(st, a, cw * 0.5, -FOUNDATION_M, ch, 8, WALL, STONE_TINT);
        half_cone(st, a, cw * 0.5 + 0.5, ch, 0.2, ch + cw * 0.45, 8, ROOF);
    }
    let mut top = ridge;
    let tower = &p["tower"];
    if tower.is_object() && !tower.as_object().is_some_and(|o| o.is_empty()) {
        let size = f(tower, "size", 6.0);
        let th = f(tower, "height", h + 12.0);
        let tx = if s(tower, "at", "west") == "west" {
            x0 - size * 0.5 + 1.0
        } else {
            xc
        };
        let tc = t.then(Xf::at(v3(tx, 0.0, 0.0)));
        bx(st, tc, Vec3::ZERO, v3(size, th, size), WALL, STONE_TINT);
        let spire = f(tower, "spire_m", size * 0.9).max(size * 0.6);
        pyramid(st, tc, size * 0.5 + 0.3, th, th + spire, ROOF);
        top = top.max(th + spire);
    }
    top
}

fn abbey(st: &mut St, p: &Value) -> f32 {
    let width = f(p, "width_m", 16.0);
    let cl = &p["cloister"];
    let size = f(cl, "size", 30.0);
    let side = if s(cl, "side", "south") == "north" {
        -1.0
    } else {
        1.0
    };
    let total = width + size + 4.0;
    let church_z = -side * (total * 0.5 - width * 0.5);
    let top = church(st, p, Xf::at(v3(0.0, 0.0, church_z)));
    let cz = side * (total * 0.5 - size * 0.5);
    let range_w = 8.0;
    for k in 0..4 {
        let yaw = k as f32 * PI * 0.5;
        let off = v3(yaw.cos(), 0.0, yaw.sin()) * (size * 0.5 - range_w * 0.5);
        let t = Xf::new(-yaw + PI * 0.5, v3(0.0, 0.0, cz) + off);
        bx(
            st,
            t,
            Vec3::ZERO,
            v3(size, 8.0, range_w),
            MASONRY,
            RUBBLE_TINT,
        );
        gable_roof_x(
            st,
            t,
            -size * 0.5,
            size * 0.5,
            range_w * 0.5 + 0.4,
            8.0,
            12.0,
            ROOF_TILE,
            false,
            false,
        );
    }
    top
}

fn castle(st: &mut St, p: &Value) -> f32 {
    let h = f(p, "height_m", 10.0);
    let thick = f(p, "thickness_m", 3.0);
    let tr = f(p, "tower_radius_m", 5.0);
    let th = f(p, "tower_height_m", h + 6.0);
    let pts: Vec<Vec3> = p["ring"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|q| v3(num(&q[0], 0.0), 0.0, -num(&q[1], 0.0)))
                .collect()
        })
        .unwrap_or_default();
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        let d = b - a;
        let t = Xf::new(-d.z.atan2(d.x), (a + b) * 0.5);
        bx(
            st,
            t,
            Vec3::ZERO,
            v3(d.length(), h, thick),
            MASONRY,
            STONE_TINT,
        );
        cylinder(
            st,
            Xf::at(a),
            tr,
            -FOUNDATION_M,
            th,
            12,
            MASONRY,
            STONE_TINT,
        );
        cone(st, Xf::at(a), tr * 1.12, th, th + tr * 1.5, 12, ROOF);
    }
    let mut top = th + tr * 1.5;
    let keep = &p["keep"];
    if keep.as_object().is_some_and(|o| !o.is_empty()) {
        let kc = Xf::at(v3(num(&keep["at"][0], 0.0), 0.0, -num(&keep["at"][1], 0.0)));
        let kr = f(keep, "radius_m", 7.0);
        let kh = f(keep, "height_m", 28.0);
        cylinder(st, kc, kr, -FOUNDATION_M, kh, 16, MASONRY, STONE_TINT);
        cone(st, kc, kr * 1.1, kh, kh + kr * 1.6, 16, ROOF);
        top = top.max(kh + kr * 1.6);
    }
    if let Some(halls) = p["halls"].as_array() {
        for hall in halls {
            let t = Xf::new(
                num(&hall[5], 0.0).to_radians(),
                v3(num(&hall[0], 0.0), 0.0, -num(&hall[1], 0.0)),
            );
            let (hl, hd, hh) = (num(&hall[2], 10.0), num(&hall[3], 8.0), num(&hall[4], 8.0));
            bx(st, t, Vec3::ZERO, v3(hl, hh, hd), MASONRY, STONE_TINT);
            gable_roof_x(
                st,
                t,
                -hl * 0.5,
                hl * 0.5,
                hd * 0.5 + 0.5,
                hh,
                hh + hd * 0.5,
                ROOF,
                true,
                true,
            );
        }
    }
    top
}

fn belfry(st: &mut St, p: &Value) -> f32 {
    let size = f(p, "size", 10.0);
    let h = f(p, "height", 30.0);
    let id = Xf::ID;
    bx(st, id, Vec3::ZERO, v3(size, h, size), WALL, STONE_TINT);
    for side in [-1.0_f32, 1.0] {
        lancet(
            st,
            id,
            0.0,
            side * (size * 0.5 + 0.05),
            h * 0.8,
            h * 0.95,
            size * 0.28,
            side,
        );
        lancet_x(
            st,
            id,
            side * (size * 0.5 + 0.05),
            0.0,
            h * 0.8,
            h * 0.95,
            size * 0.28,
            side,
        );
        lancet(
            st,
            id,
            0.0,
            side * (size * 0.5 + 0.05),
            h * 0.45,
            h * 0.55,
            size * 0.1,
            side,
        );
    }
    match s(p, "top", "pyramid") {
        "turrets" => {
            let t = (size * 0.16).max(2.5);
            let th = (size * 0.18).max(3.0);
            for sx in [-1.0_f32, 1.0] {
                for sz in [-1.0_f32, 1.0] {
                    box_(
                        st,
                        id,
                        v3(sx * (size - t) * 0.5, h, sz * (size - t) * 0.5),
                        v3(t, th, t),
                        WALL,
                        STONE_TINT,
                        h,
                    );
                }
            }
            box_(
                st,
                id,
                v3(0.0, h, 0.0),
                v3(size - 2.0 * t, 1.2, size - 2.0 * t),
                MASONRY,
                STONE_TINT,
                h,
            );
            h + th
        }
        "lantern" => {
            box_(
                st,
                id,
                v3(0.0, h, 0.0),
                v3(size * 0.55, size * 0.6, size * 0.55),
                WALL,
                STONE_TINT,
                h,
            );
            pyramid(st, id, size * 0.35, h + size * 0.6, h + size * 1.3, ROOF);
            h + size * 1.3
        }
        _ => {
            pyramid(st, id, size * 0.5 + 0.4, h, h + size * 0.9, ROOF);
            h + size * 0.9
        }
    }
}

fn hall(st: &mut St, p: &Value) -> f32 {
    let length = f(p, "length_m", 50.0);
    let width = f(p, "width_m", 20.0);
    let h = f(p, "height_m", 12.0);
    let id = Xf::ID;
    let eave = h * 0.4;
    bx(
        st,
        id,
        Vec3::ZERO,
        v3(length, eave, width),
        MASONRY,
        RUBBLE_TINT,
    );
    gable_roof_x(
        st,
        id,
        -length * 0.5 - 0.5,
        length * 0.5 + 0.5,
        width * 0.5 + 0.8,
        eave,
        h,
        ROOF_TILE,
        true,
        true,
    );
    let arches = ((length / 6.0) as i32).max(2);
    for side in [-1.0_f32, 1.0] {
        for k in 0..arches {
            let ax = -length * 0.5 + length * (k as f32 + 0.5) / arches as f32;
            lancet(
                st,
                id,
                ax,
                side * (width * 0.5 + 0.05),
                0.0,
                eave * 0.85,
                (length / arches as f32 * 0.6).min(3.6),
                side,
            );
        }
        lancet_x(
            st,
            id,
            side * (length * 0.5 + 0.05),
            0.0,
            0.0,
            eave * 0.85,
            (width * 0.25).min(4.0),
            side,
        );
    }
    h
}

fn palace(st: &mut St, p: &Value) -> f32 {
    let length = f(p, "length_m", 35.0);
    let width = f(p, "width_m", 12.0);
    let h = f(p, "height_m", 11.0);
    let id = Xf::ID;
    bx(st, id, Vec3::ZERO, v3(length, h, width), WALL, STONE_TINT);
    gable_roof_x(
        st,
        id,
        -length * 0.5,
        length * 0.5,
        width * 0.5 + 0.5,
        h,
        h + width * 0.55,
        ROOF,
        true,
        true,
    );
    let bays = ((length / 5.0) as i32).max(2);
    for side in [-1.0_f32, 1.0] {
        for k in 0..bays {
            let wx = -length * 0.5 + length * (k as f32 + 0.5) / bays as f32;
            lancet(
                st,
                id,
                wx,
                side * (width * 0.5 + 0.05),
                h * 0.14,
                h * 0.4,
                1.3,
                side,
            );
            lancet(
                st,
                id,
                wx,
                side * (width * 0.5 + 0.05),
                h * 0.56,
                h * 0.86,
                1.3,
                side,
            );
        }
    }
    if let Some(ranges) = p["ranges"].as_array() {
        for r in ranges {
            let t = Xf::new(
                num(&r[5], 0.0).to_radians(),
                v3(num(&r[0], 0.0), 0.0, -num(&r[1], 0.0)),
            );
            let (rl, rw, rh) = (num(&r[2], 10.0), num(&r[3], 8.0), num(&r[4], 8.0));
            bx(st, t, Vec3::ZERO, v3(rl, rh, rw), WALL, STONE_TINT);
            gable_roof_x(
                st,
                t,
                -rl * 0.5,
                rl * 0.5,
                rw * 0.5 + 0.4,
                rh,
                rh + rw * 0.5,
                ROOF,
                true,
                true,
            );
        }
    }
    h + width * 0.55
}

fn enclosure(st: &mut St, p: &Value) -> f32 {
    let length = f(p, "length_m", 60.0);
    let width = f(p, "width_m", 40.0);
    let h = f(p, "height_m", 6.0);
    let id = Xf::ID;
    for side in [-1.0_f32, 1.0] {
        bx(
            st,
            id,
            v3(0.0, 0.0, side * width * 0.5),
            v3(length, h, 1.4),
            MASONRY,
            RUBBLE_TINT,
        );
        bx(
            st,
            id,
            v3(side * length * 0.5, 0.0, 0.0),
            v3(1.4, h, width),
            MASONRY,
            RUBBLE_TINT,
        );
    }
    let mut top = h;
    if p["ranges"].as_bool().unwrap_or(false) {
        let rw = 10.0_f32.min(width * 0.22);
        for side in [-1.0_f32, 1.0] {
            let t = Xf::at(v3(0.0, 0.0, side * (width * 0.5 - rw * 0.5 - 0.7)));
            let rl = length * 0.8;
            bx(st, t, Vec3::ZERO, v3(rl, h + 3.0, rw), MASONRY, RUBBLE_TINT);
            gable_roof_x(
                st,
                t,
                -rl * 0.5,
                rl * 0.5,
                rw * 0.5 + 0.4,
                h + 3.0,
                h + 3.0 + rw * 0.5,
                ROOF_TILE,
                true,
                true,
            );
            top = h + 3.0 + rw * 0.5;
        }
    }
    top
}

/// Earth and timber work: trapezoidal bank along `ring`, topped by a palisade.
fn earthwork(st: &mut St, p: &Value) -> f32 {
    let pts: Vec<Vec3> = p["ring"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|q| v3(num(&q[0], 0.0), 0.0, -num(&q[1], 0.0)))
                .collect()
        })
        .unwrap_or_default();
    let bank = f(p, "bank_m", 4.0);
    let base = f(p, "base_m", 9.0);
    let crest = base * 0.3;
    let palisade = f(p, "palisade_m", 2.5);
    let closed = p["closed"].as_bool().unwrap_or(false);
    let count = if closed {
        pts.len()
    } else {
        pts.len().saturating_sub(1)
    };
    for i in 0..count {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        let d = b - a;
        let length = d.length() + base * 0.25;
        let t = Xf::new(-d.z.atan2(d.x), (a + b) * 0.5);
        let (hx, hb, hc) = (length * 0.5, base * 0.5, crest * 0.5);
        let reference = v3(0.0, bank * 0.3, 0.0);
        for side in [-1.0_f32, 1.0] {
            quad_oriented(
                st,
                t,
                v3(-hx, -2.0, side * hb),
                v3(hx, -2.0, side * hb),
                v3(hx, bank, side * hc),
                v3(-hx, bank, side * hc),
                RUBBLE,
                EARTH_TINT,
                reference,
            );
        }
        quad_oriented(
            st,
            t,
            v3(-hx, bank, -hc),
            v3(hx, bank, -hc),
            v3(hx, bank, hc),
            v3(-hx, bank, hc),
            RUBBLE,
            EARTH_TINT,
            Vec3::ZERO,
        );
        for sx in [-hx, hx] {
            quad_oriented(
                st,
                t,
                v3(sx, -2.0, -hb),
                v3(sx, -2.0, hb),
                v3(sx, bank, hc),
                v3(sx, bank, -hc),
                RUBBLE,
                EARTH_TINT,
                v3(0.0, bank * 0.3, 0.0),
            );
        }
        if palisade > 0.0 {
            box_(
                st,
                t,
                v3(0.0, bank, 0.0),
                v3(d.length(), palisade, 0.45),
                TIMBER,
                WOOD_TINT,
                bank - 0.5,
            );
        }
    }
    bank + palisade
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn church_footprint_adds_the_west_tower() {
        let m = json!({"model": "church", "params": {"length_m": 20, "width_m": 9, "tower": {"at": "west", "size": 6}}});
        assert_eq!(footprint(&m), Vec2::new(26.0, 9.0));
    }

    #[test]
    fn castle_footprint_covers_ring_and_towers() {
        let m = json!({"model": "castle", "params": {"ring": [[-24, -18], [24, -18], [24, 18], [-24, 18]], "tower_radius_m": 4.5}});
        assert_eq!(footprint(&m), Vec2::new(57.0, 45.0));
    }

    #[test]
    fn every_model_builds_outward_triangles() {
        let models = [
            json!({"model": "gothic_cathedral", "params": {"length_m": 58, "width_m": 46, "nave_width_m": 13, "vault_m": 32, "ridge_m": 40, "aisle_m": 14, "apse": "round", "chapels": true, "open_west": true, "bays": 4, "transept_m": 60, "crossing": {"size": 14, "height": 60, "spire_m": 20}, "west_towers": [{"size": 12, "height": 50, "top": "spire", "side": "north"}]}}),
            json!({"model": "church", "params": {"length_m": 30, "width_m": 12, "apse": "round", "tower": {"at": "west", "size": 6}}}),
            json!({"model": "abbey", "params": {"length_m": 40, "width_m": 16, "cloister": {"size": 30}}}),
            json!({"model": "castle", "params": {"ring": [[-24, -18], [24, -18], [24, 18], [-24, 18]], "keep": {"at": [0, 0]}, "halls": [[0, 8, 36, 10, 13, 0]]}}),
            json!({"model": "belfry", "params": {"size": 10, "height": 40, "top": "turrets"}}),
            json!({"model": "hall", "params": {}}),
            json!({"model": "royal_palace", "params": {"ranges": [[10, 5, 20, 8, 9, 30]]}}),
            json!({"model": "enclosure", "params": {"ranges": true}}),
            json!({"model": "earthwork", "params": {"ring": [[-20, 0], [0, 10], [20, 0]]}}),
            json!({"model": "gate_tower", "params": {"size": 12, "height": 20}}),
        ];
        for m in models {
            let built = build(&m);
            assert!(built.mesh.triangles() > 8, "{}", m["model"]);
            assert!(built.top > 5.0, "{}", m["model"]);
            assert_eq!(built.mesh.positions.len(), built.mesh.colors.len());
            for n in &built.mesh.normals {
                let l = Vec3::from(*n).length();
                assert!((l - 1.0).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn box_faces_point_away_from_the_centre() {
        let mut st = St::default();
        bx(&mut st, Xf::ID, Vec3::ZERO, v3(4.0, 6.0, 2.0), WALL, WHITE);
        let centre = v3(0.0, -FOUNDATION_M * 0.5 + 3.0, 0.0);
        for tri in 0..st.0.triangles() {
            let a = Vec3::from(st.0.positions[tri * 3]);
            let n = Vec3::from(st.0.normals[tri * 3]);
            assert!(n.dot(a - centre) > -1e-3);
        }
    }
}
