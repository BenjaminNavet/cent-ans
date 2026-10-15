//! Small procedural shapes of the town layer: the house block with a gable roof (prism) that
//! replaces the plain box at far detail, and the garden trees of a town merged in one mesh.
//! Port of `TownBuilder.block_mesh` (walls plastered, tiled roof) and of the plan trees.

use bevy::asset::RenderAssetUsages;
use bevy::math::{Vec2, Vec3};
use bevy::mesh::{Mesh, PrimitiveTopology};

use super::rng::hash01;

/// Triangle soup with flat normals and vertex colours.
#[derive(Default)]
struct Soup {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
}

impl Soup {
    /// Triangle whose normal points away from `inside`.
    fn tri(&mut self, a: Vec3, mut b: Vec3, mut c: Vec3, color: [f32; 4], inside: Vec3) {
        let mut n = (b - a).cross(c - a);
        if n.length_squared() < 1e-12 {
            return;
        }
        if n.dot((a + b + c) / 3.0 - inside) < 0.0 {
            std::mem::swap(&mut b, &mut c);
            n = -n;
        }
        let n = n.normalize().to_array();
        for p in [a, b, c] {
            self.positions.push(p.to_array());
            self.normals.push(n);
            self.colors.push(color);
        }
    }

    fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: [f32; 4], inside: Vec3) {
        self.tri(a, b, c, color, inside);
        self.tri(a, c, d, color, inside);
    }

    fn into_mesh(self) -> Mesh {
        let n = self.positions.len();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0_f32, 0.0]; n]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh
    }
}

/// Eave height of the unit house block (share of its height).
pub const EAVE: f32 = 0.52;
/// Foundation of the unit house block (below the ground, share of its height).
pub const FOUNDATION: f32 = -0.15;
/// Vertex colours: tint over the atlas texture, alpha the atlas layer (Plaster, RoofTile; see
/// `towns::atlas::layer_alpha`).
const WALL: [f32; 4] = [1.0, 0.97, 0.92, 0.5 / 16.0];
const ROOF: [f32; 4] = [1.0, 1.0, 1.0, 7.5 / 16.0];

/// Unit house block (1 x 1 x 1, gable along X, ground at 0, foundation at -0.15): plastered
/// walls and a tiled gable roof. Scaled to (front, height, depth) by the caller.
pub fn house_block_mesh() -> Mesh {
    let mut s = Soup::default();
    let inside = Vec3::new(0.0, 0.4, 0.0);
    let c = [
        Vec2::new(-0.5, -0.5),
        Vec2::new(0.5, -0.5),
        Vec2::new(0.5, 0.5),
        Vec2::new(-0.5, 0.5),
    ];
    for i in 0..4 {
        let (a, b) = (c[i], c[(i + 1) % 4]);
        s.quad(
            Vec3::new(a.x, FOUNDATION, a.y),
            Vec3::new(b.x, FOUNDATION, b.y),
            Vec3::new(b.x, EAVE, b.y),
            Vec3::new(a.x, EAVE, a.y),
            WALL,
            inside,
        );
    }
    // Gables at both ends along X.
    for sx in [-0.5_f32, 0.5] {
        s.tri(
            Vec3::new(sx, EAVE, -0.5),
            Vec3::new(sx, EAVE, 0.5),
            Vec3::new(sx, 1.0, 0.0),
            WALL,
            inside,
        );
    }
    // Two slopes towards the ridge.
    for sz in [-0.5_f32, 0.5] {
        s.quad(
            Vec3::new(-0.5, EAVE, sz),
            Vec3::new(0.5, EAVE, sz),
            Vec3::new(0.5, 1.0, 0.0),
            Vec3::new(-0.5, 1.0, 0.0),
            ROOF,
            inside,
        );
    }
    s.into_mesh()
}

/// Number of triangles of one garden tree.
pub const TREE_TRIANGLES: usize = 4 * 2 + 3 * 8 * 2;

/// Garden trees merged in one mesh: trunk (4 sides) and lumpy crown (three rings), size and tint
/// from a hash of the position. `trees`: local x, y (south) and a base height that is ignored;
/// `y_of` gives the displayed ground height (local metres).
pub fn trees_mesh(trees: &[Vec3], y_of: &dyn Fn(Vec2) -> f32) -> Mesh {
    let mut s = Soup::default();
    for t in trees {
        let key =
            (t.x * 7.3).round() as i32 as u32 ^ ((t.y * 5.1).round() as i32 as u32).rotate_left(16);
        let (h1, h2, h3) = (hash01(key, 1), hash01(key, 2), hash01(key, 3));
        let ground = y_of(Vec2::new(t.x, t.y)) - 0.2;
        let height = 5.5 + 4.0 * h1;
        let crown_r = 2.2 + 1.6 * h2;
        let trunk_top = height * 0.45;
        let trunk_r = 0.22 + 0.12 * h3;
        let green = [0.20 + 0.10 * h2, 0.32 + 0.12 * h1, 0.14 + 0.06 * h3, 1.0];
        let bark = [0.30, 0.22, 0.15, 1.0];
        let base = Vec3::new(t.x, ground, t.y);
        let inside = base + Vec3::new(0.0, trunk_top * 0.5, 0.0);
        for k in 0..4 {
            let a0 = std::f32::consts::FRAC_PI_2 * k as f32 + 0.4;
            let a1 = a0 + std::f32::consts::FRAC_PI_2;
            let p0 = Vec3::new(a0.cos() * trunk_r, 0.0, a0.sin() * trunk_r);
            let p1 = Vec3::new(a1.cos() * trunk_r, 0.0, a1.sin() * trunk_r);
            s.quad(
                base + p0,
                base + p1,
                base + p1 + Vec3::Y * trunk_top,
                base + p0 + Vec3::Y * trunk_top,
                bark,
                inside,
            );
        }
        // Crown: bottom apex, three rings, top apex.
        let centre = base + Vec3::new(0.0, trunk_top + (height - trunk_top) * 0.45, 0.0);
        let rings = [
            (trunk_top * 0.9, crown_r * 0.62),
            (trunk_top + (height - trunk_top) * 0.4, crown_r),
            (trunk_top + (height - trunk_top) * 0.78, crown_r * 0.66),
        ];
        let ring_pt = |r: usize, k: usize| {
            let a = std::f32::consts::TAU * k as f32 / 8.0 + 0.2 * r as f32;
            base + Vec3::new(a.cos() * rings[r].1, rings[r].0, a.sin() * rings[r].1)
        };
        let bottom = base + Vec3::new(0.0, trunk_top * 0.75, 0.0);
        let top = base + Vec3::new(0.0, height, 0.0);
        for k in 0..8 {
            let k1 = (k + 1) % 8;
            s.tri(bottom, ring_pt(0, k), ring_pt(0, k1), green, centre);
            for r in 0..2 {
                s.quad(
                    ring_pt(r, k),
                    ring_pt(r, k1),
                    ring_pt(r + 1, k1),
                    ring_pt(r + 1, k),
                    green,
                    centre,
                );
            }
            s.tri(top, ring_pt(2, k), ring_pt(2, k1), green, centre);
        }
    }
    s.into_mesh()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex_count(m: &Mesh) -> usize {
        m.count_vertices()
    }

    #[test]
    fn house_block_has_walls_gables_and_roof() {
        let m = house_block_mesh();
        // 4 wall quads + 2 gable triangles + 2 roof quads.
        assert_eq!(vertex_count(&m), (4 * 2 + 2 + 2 * 2) * 3);
    }

    #[test]
    fn trees_merge_into_one_mesh_and_follow_the_ground() {
        let trees = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 5.0, 0.0)];
        let m = trees_mesh(&trees, &|p| p.x);
        // Trunk 4 quads (8 triangles) + crown 8 * (1 + 2 * 2 + 1) triangles, two trees.
        assert_eq!(vertex_count(&m), 2 * (8 + 8 * 6) * 3);
        assert_eq!(TREE_TRIANGLES, 8 + 48);
        let ys: Vec<f32> = m
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap()
            .iter()
            .map(|p| p[1])
            .collect();
        let max_second_tree = ys[ys.len() / 2..].iter().cloned().fold(f32::MIN, f32::max);
        assert!(max_second_tree > 10.0, "second tree stands on the slope");
    }
}
