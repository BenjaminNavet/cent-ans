//! Procedural tree meshes of the map (`vegetation_meshes.gd`): deciduous (three bumpy masses on a
//! trunk), conifer (three stacked cones), their far variants (one mass / one cone, ~20 triangles),
//! hedge shrub and far hedge. Total height 1.0, the instance sets the real size.
//!
//! Vertex colour: RGB = base albedo (tinted per instance by the foliage shader), A = wind sway
//! weight (0 at the foot, 1 at the top). UV0 = (species id, 1 crown / 0 trunk). Normals are
//! "inflated" (from the centre of each mass): soft foliage shading.
//!
//! These are the fallback shapes when a species has no glb (`render::request_model`) and the
//! hedge shrubs; the Blender `campaign_trees.glb` loader of Godot is replaced by the DN lod2 glb
//! models of `render.rs`.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;

pub const BARK: [f32; 3] = [0.23, 0.17, 0.11];
pub const LEAF_DARK: [f32; 3] = [0.065, 0.10, 0.04];
pub const LEAF_LIGHT: [f32; 3] = [0.185, 0.225, 0.085];
pub const NEEDLE_DARK: [f32; 3] = [0.04, 0.09, 0.05];
pub const NEEDLE_LIGHT: [f32; 3] = [0.10, 0.17, 0.08];

/// Species id written in UV.x (read by the foliage shader for the seasonal tint).
pub const ESSENCE_OAK: f32 = 0.0;
pub const ESSENCE_BEECH: f32 = 1.0;
pub const ESSENCE_FIR: f32 = 2.0;
pub const ESSENCE_HEDGE: f32 = 3.0;

/// Procedural shapes, in the order of the Godot cache keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    Deciduous,
    Conifer,
    DeciduousLow,
    ConiferLow,
    Hedge,
    HedgeLow,
}

impl Shape {
    pub const ALL: [Shape; 6] = [
        Shape::Deciduous,
        Shape::Conifer,
        Shape::DeciduousLow,
        Shape::ConiferLow,
        Shape::Hedge,
        Shape::HedgeLow,
    ];

    /// Far variant of a detailed shape (itself when already far).
    pub fn far(self) -> Shape {
        match self {
            Shape::Deciduous => Shape::DeciduousLow,
            Shape::Conifer => Shape::ConiferLow,
            Shape::Hedge => Shape::HedgeLow,
            other => other,
        }
    }
}

/// CPU-side triangle soup (non indexed, like `SurfaceTool`).
#[derive(Default, Clone)]
pub struct Soup {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
}

impl Soup {
    fn push(&mut self, p: Vec3, n: Vec3, color: [f32; 4], uv: [f32; 2]) {
        self.positions.push(p.to_array());
        self.normals.push(n.to_array());
        self.colors.push(color);
        self.uvs.push(uv);
    }

    pub fn triangle_count(&self) -> usize {
        self.positions.len() / 3
    }

    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh
    }
}

fn rgba(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// `_hash01`: fract(sin(p . (12.9898, 78.233, 37.719)) * 43758.5453), deterministic.
fn hash01(p: Vec3) -> f32 {
    let h = (p.dot(Vec3::new(12.9898, 78.233, 37.719)) as f64).sin() * 43758.5453;
    (h - h.floor()) as f32
}

/// Faces of an icosahedron (20 triangles) or once subdivided (80), unit vertices.
fn icosphere_faces(fine: bool) -> Vec<[Vec3; 3]> {
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let v: Vec<Vec3> = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ]
    .iter()
    .map(|p| Vec3::from_array(*p).normalize())
    .collect();
    const IDX: [[usize; 3]; 20] = [
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    let mut faces = Vec::new();
    for f in IDX {
        let (a, b, c) = (v[f[0]], v[f[1]], v[f[2]]);
        if !fine {
            faces.push([a, b, c]);
            continue;
        }
        let ab = ((a + b) * 0.5).normalize();
        let bc = ((b + c) * 0.5).normalize();
        let ca = ((c + a) * 0.5).normalize();
        faces.push([a, ab, ca]);
        faces.push([b, bc, ab]);
        faces.push([c, ca, bc]);
        faces.push([ab, bc, ca]);
    }
    faces
}

fn add_trunk(soup: &mut Soup, radius: f32, height: f32, sides: u32) {
    for i in 0..sides {
        let a0 = TAU * i as f32 / sides as f32;
        let a1 = TAU * (i + 1) as f32 / sides as f32;
        let p0 = Vec3::new(a0.cos(), 0.0, a0.sin());
        let p1 = Vec3::new(a1.cos(), 0.0, a1.sin());
        let b0 = p0 * radius;
        let b1 = p1 * radius;
        let t0 = p0 * radius * 0.7 + Vec3::Y * height;
        let t1 = p1 * radius * 0.7 + Vec3::Y * height;
        for (p, n, a) in [
            (b0, p0, 0.0),
            (t1, p1, 0.2),
            (b1, p1, 0.0),
            (b0, p0, 0.0),
            (t0, p0, 0.2),
            (t1, p1, 0.2),
        ] {
            soup.push(p - Vec3::Y * 0.05, n, rgba(BARK, a), [ESSENCE_OAK, 0.0]);
        }
    }
}

/// Bumpy icosahedron blob with spherical normals; the underside and inside are darker.
fn add_blob(soup: &mut Soup, center: Vec3, radii: Vec3, seed: f32, fine: bool, essence: f32) {
    for face in icosphere_faces(fine) {
        for corner in [0, 2, 1] {
            let dir = face[corner];
            let bump = 0.85 + 0.3 * hash01(dir * 7.0 + Vec3::splat(seed));
            let p = center + dir * radii * bump;
            let light = (0.5 + 0.5 * dir.y).clamp(0.0, 1.0);
            let t = light * 0.8 + 0.2 * hash01(dir * 3.0 + Vec3::splat(seed));
            soup.push(
                p,
                (dir + Vec3::Y * 0.25).normalize(),
                rgba(mix(LEAF_DARK, LEAF_LIGHT, t), p.y.clamp(0.0, 1.0)),
                [essence, 1.0],
            );
        }
    }
}

fn add_cone(soup: &mut Soup, y0: f32, y1: f32, radius: f32, sides: u32, twist: f32) {
    let apex = Vec3::new(0.0, y1, 0.0);
    let slope = radius / (y1 - y0);
    for i in 0..sides {
        let a0 = TAU * i as f32 / sides as f32 + twist;
        let a1 = TAU * (i + 1) as f32 / sides as f32 + twist;
        let am = (a0 + a1) * 0.5;
        let r0 = radius * (0.9 + 0.2 * hash01(Vec3::new(a0, y0, 1.0)));
        let r1 = radius * (0.9 + 0.2 * hash01(Vec3::new(a1, y0, 1.0)));
        let b0 = Vec3::new(a0.cos() * r0, y0, a0.sin() * r0);
        let b1 = Vec3::new(a1.cos() * r1, y0, a1.sin() * r1);
        let n0 = Vec3::new(a0.cos(), slope, a0.sin()).normalize();
        let n1 = Vec3::new(a1.cos(), slope, a1.sin()).normalize();
        let nm = Vec3::new(am.cos(), slope, am.sin()).normalize();
        for (p, n, c) in [
            (b0, n0, NEEDLE_DARK),
            (b1, n1, NEEDLE_DARK),
            (apex, nm, NEEDLE_LIGHT),
        ] {
            soup.push(p, n, rgba(c, p.y.clamp(0.0, 1.0)), [ESSENCE_FIR, 1.0]);
        }
        // Underside of the skirt (seen at grazing angles): one triangle towards the axis.
        let inner = Vec3::new(0.0, y0 + (y1 - y0) * 0.2, 0.0);
        let under = mix(NEEDLE_DARK, [0.0; 3], 0.35);
        for p in [b0, inner, b1] {
            soup.push(
                p,
                Vec3::NEG_Y,
                rgba(under, p.y.clamp(0.0, 1.0)),
                [ESSENCE_FIR, 1.0],
            );
        }
    }
}

fn hedge_low_soup() -> Soup {
    let mut soup = Soup::default();
    let top = Vec3::new(0.0, 0.56, 0.0);
    let ring = [
        Vec3::new(0.55, 0.28, 0.0),
        Vec3::new(0.0, 0.28, 0.2),
        Vec3::new(-0.55, 0.28, 0.0),
        Vec3::new(0.0, 0.28, -0.2),
    ];
    let bottom = Vec3::ZERO;
    let uv = [ESSENCE_HEDGE, 1.0];
    for i in 0..4 {
        let (a, b) = (ring[i], ring[(i + 1) % 4]);
        for v in [top, a, b] {
            soup.push(
                v,
                (v - Vec3::new(0.0, 0.2, 0.0)).normalize(),
                rgba(LEAF_LIGHT, v.y),
                uv,
            );
        }
        for v in [bottom, b, a] {
            soup.push(
                v,
                (v - Vec3::new(0.0, 0.3, 0.0)).normalize(),
                rgba(LEAF_DARK, v.y),
                uv,
            );
        }
    }
    soup
}

/// Triangle soup of a procedural shape.
pub fn build(shape: Shape) -> Soup {
    let mut soup = Soup::default();
    match shape {
        Shape::Deciduous => {
            add_trunk(&mut soup, 0.055, 0.42, 5);
            add_blob(
                &mut soup,
                Vec3::new(0.0, 0.66, 0.0),
                Vec3::new(0.42, 0.33, 0.42),
                11.0,
                false,
                ESSENCE_OAK,
            );
            add_blob(
                &mut soup,
                Vec3::new(0.2, 0.52, 0.09),
                Vec3::new(0.28, 0.22, 0.28),
                23.0,
                false,
                ESSENCE_OAK,
            );
            add_blob(
                &mut soup,
                Vec3::new(-0.17, 0.50, -0.14),
                Vec3::new(0.27, 0.21, 0.27),
                37.0,
                false,
                ESSENCE_OAK,
            );
        }
        Shape::Conifer => {
            add_trunk(&mut soup, 0.04, 0.2, 5);
            add_cone(&mut soup, 0.12, 0.62, 0.30, 7, 0.0);
            add_cone(&mut soup, 0.36, 0.86, 0.23, 7, 0.45);
            add_cone(&mut soup, 0.58, 1.0, 0.15, 7, 0.9);
        }
        Shape::DeciduousLow => add_blob(
            &mut soup,
            Vec3::new(0.0, 0.6, 0.0),
            Vec3::new(0.48, 0.38, 0.48),
            11.0,
            false,
            ESSENCE_OAK,
        ),
        Shape::ConiferLow => add_cone(&mut soup, 0.1, 1.0, 0.30, 5, 0.0),
        Shape::Hedge => add_blob(
            &mut soup,
            Vec3::new(0.0, 0.28, 0.0),
            Vec3::new(0.55, 0.28, 0.2),
            51.0,
            false,
            ESSENCE_HEDGE,
        ),
        Shape::HedgeLow => soup = hedge_low_soup(),
    }
    soup
}

/// Bevy mesh of a procedural shape.
pub fn mesh(shape: Shape) -> Mesh {
    build(shape).into_mesh()
}

/// Impostor quad appended to near trees (UV2 = corner + (0, 2) in Godot): corners of the
/// 2-triangle billboard, in Godot's order.
pub fn impostor_corners() -> [[f32; 2]; 6] {
    [
        [0.0, 0.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_counts_match_godot() {
        // icosahedron 20 faces; trunk 5 sides x 2.
        assert_eq!(build(Shape::DeciduousLow).triangle_count(), 20);
        assert_eq!(build(Shape::Deciduous).triangle_count(), 10 + 60);
        // cone: 2 triangles per side (skirt + underside).
        assert_eq!(build(Shape::ConiferLow).triangle_count(), 10);
        assert_eq!(build(Shape::Conifer).triangle_count(), 10 + 3 * 14);
        assert_eq!(build(Shape::HedgeLow).triangle_count(), 8);
        assert_eq!(build(Shape::Hedge).triangle_count(), 20);
    }

    #[test]
    fn shapes_have_unit_height_and_alpha_in_range() {
        for shape in Shape::ALL {
            let soup = build(shape);
            let top = soup.positions.iter().map(|p| p[1]).fold(0.0f32, f32::max);
            assert!(top <= 1.05 && top > 0.3, "{shape:?} height {top}");
            assert!(soup.colors.iter().all(|c| (0.0..=1.0).contains(&c[3])));
            assert_eq!(soup.positions.len(), soup.normals.len());
        }
    }

    #[test]
    fn fine_icosphere_has_80_faces() {
        assert_eq!(icosphere_faces(false).len(), 20);
        assert_eq!(icosphere_faces(true).len(), 80);
    }

    #[test]
    fn far_maps_detailed_to_low() {
        assert_eq!(Shape::Conifer.far(), Shape::ConiferLow);
        assert_eq!(Shape::HedgeLow.far(), Shape::HedgeLow);
    }
}
