//! Procedural meshes of the river crossings (port of `bridge_meshes.gd`): arched stone bridge with
//! cutwaters, timber bridge on pile bents, boat bridge, ferry, ford and gate bridge.
//!
//! Local frame: X across the river, Z along the current, Y up, origin on the centre line at water
//! level. Two surfaces per mesh: 0 stone, 1 wood. Pure geometry (no Bevy asset), testable.
//! Seeds: a splitmix stream seeded by `seed % 3` (not Godot's PCG), so the little variations of
//! piles and boats differ from the Godot ones but stay deterministic.

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::math::{Vec2, Vec3};
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use super::placement::Rng;

pub const STONE: usize = 0;
pub const WOOD: usize = 1;
/// Bottom of the piers and abutments, under the deepest carved bed.
pub const FOOT: f32 = -0.42;

/// Triangle soup of one surface (flat normals).
#[derive(Default, Clone)]
pub struct Surface {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

impl Surface {
    fn push(&mut self, v: Vec3, n: Vec3) {
        self.positions.push(v.to_array());
        self.normals.push(n.to_array());
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Triangles, counter-clockwise seen from outside.
    pub fn triangles(&self) -> usize {
        self.positions.len() / 3
    }

    /// Bevy mesh with box-mapped UVs (the Godot material is world triplanar), `scale` = texture
    /// repeats per world unit of the mesh frame.
    pub fn to_mesh(&self, scale: f32) -> Mesh {
        let uvs: Vec<[f32; 2]> = self
            .positions
            .iter()
            .zip(&self.normals)
            .map(|(p, n)| {
                let (ax, ay, az) = (n[0].abs(), n[1].abs(), n[2].abs());
                if ay >= ax && ay >= az {
                    [p[0] * scale, p[2] * scale]
                } else if ax >= az {
                    [p[2] * scale, p[1] * scale]
                } else {
                    [p[0] * scale, p[1] * scale]
                }
            })
            .collect();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone())
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(
            (0..self.positions.len() as u32).collect::<Vec<_>>(),
        ))
    }
}

/// Quad facing away from `inner`.
fn quad(s: &mut Surface, a: Vec3, mut b: Vec3, c: Vec3, mut d: Vec3, inner: Vec3) {
    let mut n = (b - a).cross(c - a);
    if n.dot((a + c) * 0.5 - inner) < 0.0 {
        std::mem::swap(&mut b, &mut d);
        n = -n;
    }
    let n = n.normalize_or_zero();
    for v in [a, b, c, a, c, d] {
        s.push(v, n);
    }
}

fn tri(s: &mut Surface, a: Vec3, mut b: Vec3, mut c: Vec3, inner: Vec3) {
    let mut n = (b - a).cross(c - a);
    if n.dot((a + b + c) / 3.0 - inner) < 0.0 {
        std::mem::swap(&mut b, &mut c);
        n = -n;
    }
    let n = n.normalize_or_zero();
    for v in [a, b, c] {
        s.push(v, n);
    }
}

fn box_at(s: &mut Surface, center: Vec3, half: Vec3, yaw: f32) {
    let basis = bevy::math::Quat::from_rotation_y(yaw);
    let c = |sx: f32, sy: f32, sz: f32| {
        center + basis * Vec3::new(sx * half.x, sy * half.y, sz * half.z)
    };
    let p000 = c(-1.0, -1.0, -1.0);
    let p100 = c(1.0, -1.0, -1.0);
    let p010 = c(-1.0, 1.0, -1.0);
    let p110 = c(1.0, 1.0, -1.0);
    let p001 = c(-1.0, -1.0, 1.0);
    let p101 = c(1.0, -1.0, 1.0);
    let p011 = c(-1.0, 1.0, 1.0);
    let p111 = c(1.0, 1.0, 1.0);
    quad(s, p010, p110, p111, p011, center);
    quad(s, p001, p101, p100, p000, center);
    quad(s, p001, p011, p111, p101, center);
    quad(s, p100, p110, p010, p000, center);
    quad(s, p101, p111, p110, p100, center);
    quad(s, p000, p010, p011, p001, center);
}

fn cross2(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Ear clipping of a simple polygon (any winding); triangle index triples.
pub fn triangulate(outline: &[Vec2]) -> Vec<[usize; 3]> {
    let n = outline.len();
    if n < 3 {
        return Vec::new();
    }
    let area: f32 = (0..n)
        .map(|i| cross2(outline[i], outline[(i + 1) % n]))
        .sum();
    let mut ids: Vec<usize> = if area >= 0.0 {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };
    let mut out = Vec::new();
    let mut guard = 0;
    while ids.len() > 3 && guard < n * n {
        guard += 1;
        let m = ids.len();
        let mut clipped = false;
        for i in 0..m {
            let (ia, ib, ic) = (ids[(i + m - 1) % m], ids[i], ids[(i + 1) % m]);
            let (a, b, c) = (outline[ia], outline[ib], outline[ic]);
            if cross2(b - a, c - b) <= 1e-12 {
                continue;
            }
            let inside = ids.iter().any(|&k| {
                if k == ia || k == ib || k == ic {
                    return false;
                }
                let p = outline[k];
                cross2(b - a, p - a) >= 0.0
                    && cross2(c - b, p - b) >= 0.0
                    && cross2(a - c, p - c) >= 0.0
            });
            if !inside {
                out.push([ia, ib, ic]);
                ids.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            // Degenerate (collinear) remainder: drop a vertex to terminate.
            ids.remove(0);
        }
    }
    if ids.len() == 3 {
        out.push([ids[0], ids[1], ids[2]]);
    }
    out
}

/// Extrusion along Z (-half_z..half_z) of a simple outline in the XY plane.
pub fn extrude(s: &mut Surface, outline: &[Vec2], half_z: f32) {
    let triangles = triangulate(outline);
    if triangles.is_empty() {
        return;
    }
    let n = outline.len();
    let area: f32 = (0..n)
        .map(|i| cross2(outline[i], outline[(i + 1) % n]))
        .sum();
    let clockwise = area < 0.0;
    for side in [1.0f32, -1.0] {
        for t in &triangles {
            let (a, mut b, mut c) = (outline[t[0]], outline[t[1]], outline[t[2]]);
            let ccw = cross2(b - a, c - a) > 0.0;
            if ccw != (side > 0.0) {
                std::mem::swap(&mut b, &mut c);
            }
            for p in [a, b, c] {
                s.push(
                    Vec3::new(p.x, p.y, side * half_z),
                    Vec3::new(0.0, 0.0, side),
                );
            }
        }
    }
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let edge = b - a;
        if edge.length_squared() < 1e-10 {
            continue;
        }
        let mut out = Vec2::new(edge.y, -edge.x).normalize();
        if clockwise {
            out = -out;
        }
        let va = Vec3::new(a.x, a.y, half_z);
        let vb = Vec3::new(b.x, b.y, half_z);
        let vc = Vec3::new(b.x, b.y, -half_z);
        let vd = Vec3::new(a.x, a.y, -half_z);
        let normal = Vec3::new(out.x, out.y, 0.0);
        let face_n = (vb - va).cross(vc - va);
        let order = if face_n.dot(normal) > 0.0 {
            [va, vb, vc, va, vc, vd]
        } else {
            [va, vc, vb, va, vd, vc]
        };
        for v in order {
            s.push(v, normal);
        }
    }
}

pub fn span_length(width: f32) -> f32 {
    width + 0.5 + 0.25 * width
}

pub fn deck_width(width: f32) -> f32 {
    0.15 + 0.05 * width
}

fn stone_bridge(stone: &mut Surface, width: f32, gate: bool) {
    let length = span_length(width);
    let half_w = deck_width(width) * if gate { 1.35 } else { 1.0 } * 0.5;
    let deck_top = 0.05 + 0.02 * width;
    let span = width * 1.02 + 0.06;
    let arches = ((span / 0.26).round() as i32).clamp(1, 13) as usize;
    let pier = (span * 0.16 / arches as f32).clamp(0.025, 0.07);
    let opening = (span - pier * (arches - 1) as f32) / arches as f32;
    let spring = -0.035;
    let crown = deck_top - 0.022;
    let mut outline = vec![
        Vec2::new(-length * 0.5, deck_top),
        Vec2::new(0.0, deck_top + 0.012),
        Vec2::new(length * 0.5, deck_top),
        Vec2::new(length * 0.5, FOOT),
    ];
    let mut x = span * 0.5;
    outline.push(Vec2::new(x, FOOT));
    for i in 0..arches {
        let right = x;
        let left = x - opening;
        outline.push(Vec2::new(right, spring));
        let segments = 6;
        for sidx in 1..segments {
            let t = sidx as f32 / segments as f32;
            let angle = t * PI;
            let px = right + (left - right) * (0.5 - 0.5 * angle.cos());
            let rise = angle.sin().powf(0.8);
            outline.push(Vec2::new(px, spring + (crown - spring) * rise));
        }
        outline.push(Vec2::new(left, spring));
        outline.push(Vec2::new(left, FOOT));
        x = left - pier;
        if i < arches - 1 {
            outline.push(Vec2::new(x, FOOT));
        }
    }
    outline.push(Vec2::new(-length * 0.5, FOOT));
    extrude(stone, &outline, half_w);
    // Cutwaters at the foot of each pier.
    x = span * 0.5 - opening;
    for _ in 0..arches - 1 {
        let cx = x - pier * 0.5;
        for side in [1.0f32, -1.0] {
            let tip = Vec3::new(cx, 0.0, side * (half_w + pier * 1.3));
            let a = Vec3::new(cx - pier * 0.5, 0.0, side * half_w);
            let b = Vec3::new(cx + pier * 0.5, 0.0, side * half_w);
            let top = spring + 0.03;
            let inner = Vec3::new(cx, (top + FOOT) * 0.5, side * half_w);
            quad(
                stone,
                Vec3::new(a.x, FOOT, a.z),
                Vec3::new(a.x, top, a.z),
                Vec3::new(tip.x, top, tip.z),
                Vec3::new(tip.x, FOOT, tip.z),
                inner,
            );
            quad(
                stone,
                Vec3::new(tip.x, FOOT, tip.z),
                Vec3::new(tip.x, top, tip.z),
                Vec3::new(b.x, top, b.z),
                Vec3::new(b.x, FOOT, b.z),
                inner,
            );
            tri(
                stone,
                Vec3::new(a.x, top, a.z),
                Vec3::new(b.x, top, b.z),
                Vec3::new(tip.x, top, tip.z),
                Vec3::new(cx, FOOT, side * half_w),
            );
        }
        x -= opening + pier;
    }
    let parapet_h = if gate { 0.05 } else { 0.022 };
    let parapet_t = if gate { 0.02 } else { 0.012 };
    for side in [1.0f32, -1.0] {
        let z = side * (half_w - parapet_t);
        box_at(
            stone,
            Vec3::new(0.0, deck_top + parapet_h * 0.5, z),
            Vec3::new(length * 0.5, parapet_h * 0.5, parapet_t),
            0.0,
        );
        if gate {
            let merlons = (length / 0.07) as usize;
            for m in (0..merlons).step_by(2) {
                let mx = -length * 0.5 + (m as f32 + 0.5) * length / merlons as f32;
                box_at(
                    stone,
                    Vec3::new(mx, deck_top + parapet_h + 0.012, z),
                    Vec3::new(length * 0.25 / merlons as f32, 0.012, parapet_t),
                    0.0,
                );
            }
        }
    }
    if gate {
        for end in [1.0f32, -1.0] {
            box_at(
                stone,
                Vec3::new(end * (length * 0.5 - 0.06), deck_top * 0.5 + 0.06, 0.0),
                Vec3::new(0.07, deck_top * 0.5 + 0.12, half_w + 0.03),
                0.0,
            );
        }
    }
}

fn wood_bridge(stone: &mut Surface, wood: &mut Surface, width: f32, rng: &mut Rng) {
    let length = span_length(width);
    let half_w = deck_width(width) * 0.45;
    let deck_top = 0.045 + 0.015 * width;
    box_at(
        wood,
        Vec3::new(0.0, deck_top - 0.007, 0.0),
        Vec3::new(length * 0.5, 0.007, half_w),
        0.0,
    );
    for end in [1.0f32, -1.0] {
        box_at(
            stone,
            Vec3::new(
                end * (length * 0.5 - 0.1),
                (deck_top - 0.014 + FOOT) * 0.5,
                0.0,
            ),
            Vec3::new(0.1, (deck_top - 0.014 - FOOT) * 0.5, half_w * 1.1),
            0.0,
        );
    }
    let bents = ((width / 0.13) as usize).max(2);
    for b in 0..bents {
        let bx = -width * 0.5 + (b as f32 + 0.5) * width / bents as f32 + rng.range(-0.01, 0.01);
        for pz in [-0.7f32, 0.0, 0.7] {
            box_at(
                wood,
                Vec3::new(bx, (deck_top - 0.014 + FOOT) * 0.5, pz * half_w),
                Vec3::new(0.007, (deck_top - 0.014 - FOOT) * 0.5, 0.007),
                0.0,
            );
        }
        box_at(
            wood,
            Vec3::new(bx, deck_top - 0.018, 0.0),
            Vec3::new(0.008, 0.005, half_w * 1.05),
            0.0,
        );
    }
    for side in [1.0f32, -1.0] {
        let z = side * (half_w - 0.004);
        box_at(
            wood,
            Vec3::new(0.0, deck_top + 0.018, z),
            Vec3::new(length * 0.5, 0.003, 0.003),
            0.0,
        );
        let posts = (length / 0.09) as usize;
        for p in 0..=posts {
            let px = -length * 0.5 + p as f32 * length / posts.max(1) as f32;
            box_at(
                wood,
                Vec3::new(px, deck_top + 0.009, z),
                Vec3::new(0.003, 0.009, 0.003),
                0.0,
            );
        }
    }
}

fn hull(s: &mut Surface, center: Vec3, half_x: f32, half_z: f32, height: f32) {
    let top = center.y + height * 0.5;
    let bottom = center.y - height * 0.5;
    let ring_top = [
        Vec3::new(0.0, top, half_z),
        Vec3::new(half_x, top, half_z * 0.6),
        Vec3::new(half_x, top, -half_z * 0.6),
        Vec3::new(0.0, top, -half_z),
        Vec3::new(-half_x, top, -half_z * 0.6),
        Vec3::new(-half_x, top, half_z * 0.6),
    ];
    let ring_bottom = [
        Vec3::new(0.0, bottom, half_z * 0.8),
        Vec3::new(half_x * 0.6, bottom, half_z * 0.5),
        Vec3::new(half_x * 0.6, bottom, -half_z * 0.5),
        Vec3::new(0.0, bottom, -half_z * 0.8),
        Vec3::new(-half_x * 0.6, bottom, -half_z * 0.5),
        Vec3::new(-half_x * 0.6, bottom, half_z * 0.5),
    ];
    let offset = Vec3::new(center.x, 0.0, center.z);
    for i in 0..6 {
        let a = ring_top[i] + offset;
        let b = ring_top[(i + 1) % 6] + offset;
        let c = ring_bottom[(i + 1) % 6] + offset;
        let d = ring_bottom[i] + offset;
        quad(s, a, d, c, b, center);
    }
    for i in 1..5 {
        tri(
            s,
            ring_top[0] + offset,
            ring_top[i + 1] + offset,
            ring_top[i] + offset,
            Vec3::new(center.x, bottom - 1.0, center.z),
        );
    }
}

fn boat_bridge(stone: &mut Surface, wood: &mut Surface, width: f32, rng: &mut Rng) {
    let length = span_length(width);
    let half_w = deck_width(width) * 0.45;
    let deck_top = 0.02;
    box_at(
        wood,
        Vec3::new(0.0, deck_top - 0.005, 0.0),
        Vec3::new(length * 0.5, 0.005, half_w),
        0.0,
    );
    let boats = ((width / 0.075) as usize).max(3);
    for b in 0..boats {
        let bx = -width * 0.5 + (b as f32 + 0.5) * width / boats as f32;
        hull(
            wood,
            Vec3::new(bx, -0.012, rng.range(-0.01, 0.01)),
            0.026,
            half_w * 1.6,
            0.02,
        );
    }
    for side in [1.0f32, -1.0] {
        box_at(
            wood,
            Vec3::new(0.0, deck_top + 0.012, side * (half_w - 0.003)),
            Vec3::new(length * 0.5, 0.0025, 0.0025),
            0.0,
        );
    }
    for end in [1.0f32, -1.0] {
        box_at(
            stone,
            Vec3::new(end * (length * 0.5 - 0.08), (deck_top + FOOT) * 0.5, 0.0),
            Vec3::new(0.08, (deck_top - FOOT) * 0.5, half_w * 1.1),
            0.0,
        );
    }
}

fn ferry(wood: &mut Surface, width: f32, rng: &mut Rng) {
    let side = if rng.f32() < 0.5 { 1.0 } else { -1.0 };
    let barge_x = side * width * rng.range(0.18, 0.32);
    box_at(
        wood,
        Vec3::new(barge_x, 0.004, 0.0),
        Vec3::new(0.09, 0.012, 0.05),
        0.0,
    );
    box_at(
        wood,
        Vec3::new(barge_x, 0.02, 0.0),
        Vec3::new(0.09, 0.004, 0.003),
        0.0,
    );
    box_at(
        wood,
        Vec3::new(0.0, 0.035, 0.0),
        Vec3::new(width * 0.5 + 0.12, 0.0015, 0.0015),
        0.0,
    );
    for end in [1.0f32, -1.0] {
        let jx = end * (width * 0.5 + 0.02);
        box_at(
            wood,
            Vec3::new(jx, 0.012, 0.0),
            Vec3::new(0.07, 0.004, 0.035),
            0.0,
        );
        for p in [-1.0f32, 1.0] {
            box_at(
                wood,
                Vec3::new(jx - end * 0.05, (0.012 + FOOT) * 0.5, p * 0.028),
                Vec3::new(0.004, (0.012 - FOOT) * 0.5, 0.004),
                0.0,
            );
        }
        box_at(
            wood,
            Vec3::new(end * (width * 0.5 + 0.12), 0.03, 0.0),
            Vec3::new(0.004, 0.04, 0.004),
            0.0,
        );
    }
}

fn ford(stone: &mut Surface, wood: &mut Surface, width: f32, rng: &mut Rng) {
    let stones = ((width / 0.07) as usize).max(4);
    for sidx in 0..stones {
        let sx = -width * 0.5 + (sidx as f32 + 0.5) * width / stones as f32;
        let size = rng.range(0.014, 0.024);
        let z = rng.range(-0.03, 0.03);
        let depth = size * rng.range(0.7, 1.2);
        let yaw = rng.f32() * TAU;
        box_at(
            stone,
            Vec3::new(sx, -0.012, z),
            Vec3::new(size, 0.012, depth),
            yaw,
        );
    }
    for end in [1.0f32, -1.0] {
        for p in [-1.0f32, 1.0] {
            box_at(
                wood,
                Vec3::new(end * (width * 0.5 + 0.05), 0.02, p * 0.05),
                Vec3::new(0.004, 0.05, 0.004),
                0.0,
            );
        }
    }
}

/// Cache key: width rounded to 0.05, three variants per structure.
pub fn cache_key(structure: &str, width: f32, seed: u64) -> (String, i32, u64) {
    (
        structure.to_string(),
        ((width.max(0.1) / 0.05).round() as i32).max(2),
        seed % 3,
    )
}

/// The stone and wood surfaces of a crossing. `structure`: stone, wood, boats, ferry, ford, gate.
pub fn build_surfaces(structure: &str, width: f32, seed: u64) -> [Surface; 2] {
    let width = ((width / 0.05).round() * 0.05).max(0.1);
    let mut stone = Surface::default();
    let mut wood = Surface::default();
    let mut rng = Rng::new(seed % 3);
    match structure {
        "wood" => wood_bridge(&mut stone, &mut wood, width, &mut rng),
        "boats" => boat_bridge(&mut stone, &mut wood, width, &mut rng),
        "ferry" => ferry(&mut wood, width, &mut rng),
        "ford" => ford(&mut stone, &mut wood, width, &mut rng),
        "gate" => stone_bridge(&mut stone, width, true),
        _ => stone_bridge(&mut stone, width, false),
    }
    [stone, wood]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ear_clipping_covers_a_concave_polygon() {
        // L shape of area 3.
        let l = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        let tris = triangulate(&l);
        assert_eq!(tris.len(), 4);
        let area: f32 = tris
            .iter()
            .map(|t| cross2(l[t[1]] - l[t[0]], l[t[2]] - l[t[0]]).abs() * 0.5)
            .sum();
        assert!((area - 3.0).abs() < 1e-4);
        // Clockwise input works too.
        let mut cw = l;
        cw.reverse();
        assert_eq!(triangulate(&cw).len(), 4);
    }

    #[test]
    fn every_structure_builds_closed_triangle_soup() {
        for structure in ["stone", "wood", "boats", "ferry", "ford", "gate"] {
            for width in [0.15, 0.4, 1.2, 3.0] {
                let [stone, wood] = build_surfaces(structure, width, 7);
                assert!(!stone.is_empty() || !wood.is_empty(), "{structure} {width}");
                for s in [&stone, &wood] {
                    assert_eq!(s.positions.len() % 3, 0);
                    assert_eq!(s.positions.len(), s.normals.len());
                    assert!(s.positions.iter().flatten().all(|v| v.is_finite()));
                }
            }
        }
    }

    #[test]
    fn stone_bridge_arch_count_follows_the_width() {
        let small = build_surfaces("stone", 0.2, 0)[STONE].triangles();
        let wide = build_surfaces("stone", 2.5, 0)[STONE].triangles();
        assert!(wide > small * 2, "{small} {wide}");
    }

    #[test]
    fn variants_cycle_with_the_seed() {
        assert_eq!(cache_key("wood", 0.5, 4), cache_key("wood", 0.5, 7));
        assert_ne!(cache_key("wood", 0.5, 4), cache_key("wood", 0.5, 5));
    }
}
