//! Procedural meshes of the war and plague scars (port of `war_scar_meshes.gd`): boxes, beams and
//! domes with vertex colours, one matt double-sided material, no texture. Metres for the plague
//! (pit, cart), units of the army figure frame for the battlefield (mound, debris, crow).

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::math::{Mat3, Vec3};
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

pub const EARTH: [f32; 3] = [0.27, 0.2, 0.14];
pub const EARTH_DARK: [f32; 3] = [0.12, 0.09, 0.07];
pub const LIME: [f32; 3] = [0.78, 0.76, 0.7];
pub const SHROUD: [f32; 3] = [0.7, 0.67, 0.6];
pub const WOOD: [f32; 3] = [0.33, 0.24, 0.15];
pub const WOOD_DARK: [f32; 3] = [0.2, 0.14, 0.09];
pub const WOOD_FRESH: [f32; 3] = [0.62, 0.48, 0.3];
pub const IRON: [f32; 3] = [0.3, 0.3, 0.32];
pub const CROW: [f32; 3] = [0.03, 0.03, 0.035];
pub const SHIELD_COLORS: [[f32; 3]; 4] = [
    [0.55, 0.1, 0.08],
    [0.12, 0.2, 0.5],
    [0.8, 0.76, 0.66],
    [0.7, 0.55, 0.15],
];

/// Deterministic [0, 1) of (seed, index) (`Hash.vec01`).
pub fn hash01(seed: u64, i: u64) -> f32 {
    let mut x = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(i.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(0x2545_F491_4F6C_DD1D);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    ((x >> 40) as f32) / 16_777_216.0
}

#[derive(Default)]
pub struct Soup {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
}

impl Soup {
    fn vertex(&mut self, p: Vec3, n: Vec3, c: [f32; 3]) {
        self.positions.push(p.to_array());
        self.normals.push(n.to_array());
        self.colors.push([c[0], c[1], c[2], 1.0]);
    }

    pub fn triangles(&self) -> usize {
        self.positions.len() / 3
    }

    pub fn into_mesh(self) -> Mesh {
        let count = self.positions.len() as u32;
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32((0..count).collect()))
    }

    /// Box of `size` centred on `center`, turned by `basis`.
    pub fn cuboid(&mut self, center: Vec3, size: Vec3, color: [f32; 3], basis: Mat3) {
        let h = size * 0.5;
        let faces = [
            (Vec3::X, Vec3::Y, Vec3::Z),
            (Vec3::NEG_X, Vec3::Y, Vec3::NEG_Z),
            (Vec3::Y, Vec3::Z, Vec3::X),
            (Vec3::NEG_Y, Vec3::NEG_Z, Vec3::X),
            (Vec3::Z, Vec3::Y, Vec3::NEG_X),
            (Vec3::NEG_Z, Vec3::Y, Vec3::X),
        ];
        for (n, u, v) in faces {
            let corners = [n - u - v, n + u - v, n + u + v, n - u + v];
            let normal = (basis * n).normalize();
            for k in [0, 1, 2, 0, 2, 3] {
                self.vertex(center + basis * (corners[k] * h), normal, color);
            }
        }
    }

    /// Bar of section `thickness` between two points.
    pub fn beam(&mut self, from: Vec3, to: Vec3, thickness: f32, color: [f32; 3]) {
        let axis = to - from;
        let length = axis.length();
        if length < 1e-5 {
            return;
        }
        let y = axis / length;
        let reference = if y.dot(Vec3::NEG_Z).abs() < 0.9 {
            Vec3::NEG_Z
        } else {
            Vec3::X
        };
        let x = y.cross(reference).normalize();
        self.cuboid(
            (from + to) * 0.5,
            Vec3::new(thickness, length, thickness),
            color,
            Mat3::from_cols(x, y, x.cross(y)),
        );
    }

    /// Wooden cross planted at `foot`.
    pub fn cross(&mut self, foot: Vec3, height: f32, color: [f32; 3], yaw: f32) {
        let t = height * 0.09;
        let side = Vec3::new(yaw.cos(), 0.0, yaw.sin());
        self.beam(foot, foot + Vec3::Y * height, t, color);
        let bar = foot + Vec3::Y * height * 0.7;
        self.beam(
            bar - side * height * 0.25,
            bar + side * height * 0.25,
            t,
            color,
        );
    }

    /// Lumpy cap of earth.
    pub fn dome(&mut self, center: Vec3, radius: f32, height: f32, color: [f32; 3], seed: u64) {
        let rings = 4usize;
        let segments = 12usize;
        let mut rows: Vec<Vec<Vec3>> = Vec::new();
        for r in 0..=rings {
            let t = r as f32 / rings as f32;
            let mut row = Vec::new();
            for s in 0..segments {
                let a = TAU * s as f32 / segments as f32;
                let wobble = 1.0
                    + if r > 0 {
                        0.16 * (hash01(seed, (r * 31 + s) as u64) - 0.5)
                    } else {
                        0.0
                    };
                let rr = radius * (t * PI * 0.5).sin() * wobble;
                row.push(
                    center + Vec3::new(a.cos() * rr, height * (t * PI * 0.5).cos(), a.sin() * rr),
                );
            }
            rows.push(row);
        }
        for r in 0..rings {
            for s in 0..segments {
                let s2 = (s + 1) % segments;
                let shade = 0.85 + 0.3 * hash01(seed, (500 + r * 17 + s) as u64);
                let quad = [rows[r][s], rows[r][s2], rows[r + 1][s2], rows[r + 1][s]];
                let mut normal = (quad[2] - quad[0])
                    .cross(quad[1] - quad[0])
                    .normalize_or_zero();
                if normal.y < 0.0 {
                    normal = -normal;
                }
                let c = [color[0] * shade, color[1] * shade, color[2] * shade];
                for k in [0, 1, 2, 0, 2, 3] {
                    self.vertex(quad[k], normal, c);
                }
            }
        }
    }
}

fn rot_z(angle: f32) -> Mat3 {
    Mat3::from_rotation_z(angle)
}

fn rot_x(angle: f32) -> Mat3 {
    Mat3::from_rotation_x(angle)
}

/// Mass grave: fresh dark earth, shrouded bodies and lime in the open part, big cross at the head.
/// Length along +X (about 6.5 m), origin at the centre on the ground.
pub fn plague_pit() -> Mesh {
    let mut s = Soup::default();
    let id = Mat3::IDENTITY;
    s.cuboid(
        Vec3::new(0.0, 0.1, 0.0),
        Vec3::new(6.6, 0.2, 3.3),
        EARTH,
        id,
    );
    s.cuboid(
        Vec3::new(-1.1, 0.26, 0.0),
        Vec3::new(3.6, 0.32, 2.5),
        EARTH_DARK,
        id,
    );
    let lighter = [
        EARTH_DARK[0] + 0.06,
        EARTH_DARK[1] + 0.06,
        EARTH_DARK[2] + 0.06,
    ];
    s.cuboid(
        Vec3::new(-1.1, 0.46, 0.0),
        Vec3::new(2.9, 0.16, 1.7),
        lighter,
        id,
    );
    s.cuboid(
        Vec3::new(1.9, 0.22, 0.0),
        Vec3::new(2.2, 0.06, 2.3),
        [0.05, 0.04, 0.035],
        id,
    );
    for k in 0..3u64 {
        let skew = Mat3::from_rotation_y(0.25 * (hash01(21, k) - 0.5));
        s.cuboid(
            Vec3::new(1.25 + 0.65 * k as f32, 0.36, 0.25 * (hash01(22, k) - 0.5)),
            Vec3::new(0.5, 0.26, 1.75),
            SHROUD,
            skew,
        );
    }
    s.cuboid(
        Vec3::new(2.75, 0.27, -0.75),
        Vec3::new(0.6, 0.04, 0.6),
        LIME,
        id,
    );
    s.cross(Vec3::new(-3.5, 0.0, 0.0), 3.2, WOOD_FRESH, PI * 0.5);
    s.into_mesh()
}

/// Cart of the dead (the Godot layer reuses the folk model): a plank bed under a shroud heap.
pub fn dead_cart() -> Mesh {
    let mut s = Soup::default();
    let id = Mat3::IDENTITY;
    s.cuboid(Vec3::new(0.0, 0.8, 0.0), Vec3::new(3.2, 0.2, 1.7), WOOD, id);
    for side in [-1.0f32, 1.0] {
        s.cuboid(
            Vec3::new(0.0, 1.15, side * 0.8),
            Vec3::new(3.2, 0.5, 0.1),
            WOOD_DARK,
            id,
        );
        for end in [-1.0f32, 1.0] {
            s.cuboid(
                Vec3::new(end * 0.9, 0.5, side * 0.95),
                Vec3::new(0.9, 0.9, 0.12),
                WOOD_DARK,
                id,
            );
        }
    }
    s.cuboid(
        Vec3::new(-0.1, 1.45, 0.0),
        Vec3::new(2.4, 0.5, 1.2),
        SHROUD,
        id,
    );
    s.beam(
        Vec3::new(1.6, 0.9, 0.0),
        Vec3::new(3.6, 0.7, 0.0),
        0.14,
        WOOD,
    );
    s.into_mesh()
}

/// Mound of the dead: fresh earth, two crosses on top.
pub fn mound(radius: f32, height: f32) -> Mesh {
    let mut s = Soup::default();
    s.dome(Vec3::ZERO, radius, height, EARTH, 3);
    s.cross(Vec3::new(0.0, height * 0.96, 0.0), height * 1.5, WOOD, 0.3);
    s.cross(
        Vec3::new(radius * 0.45, height * 0.72, radius * 0.2),
        height * 1.1,
        WOOD,
        -0.5,
    );
    s.into_mesh()
}

/// Battle debris, variant 0 stuck spear, 1 broken shield, 2 arrows, 3 torn standard, 4 cart wheel,
/// at scale `unit` (about the height of a man).
pub fn debris(variant: usize, unit: f32) -> Mesh {
    let mut s = Soup::default();
    match variant % 5 {
        0 => {
            s.beam(
                Vec3::ZERO,
                Vec3::new(0.28, 1.25, 0.1) * unit,
                0.05 * unit,
                WOOD,
            );
            s.cuboid(
                Vec3::new(0.3, 1.33, 0.11) * unit,
                Vec3::new(0.07, 0.2, 0.03) * unit,
                IRON,
                rot_z(-0.22),
            );
        }
        1 => {
            let tilt = rot_x(-1.2) * rot_z(0.2);
            s.cuboid(
                Vec3::new(0.0, 0.1, 0.0) * unit,
                Vec3::new(0.55, 0.7, 0.05) * unit,
                SHIELD_COLORS[0],
                tilt,
            );
            s.cuboid(
                Vec3::new(0.5, 0.06, 0.3) * unit,
                Vec3::new(0.3, 0.42, 0.05) * unit,
                SHIELD_COLORS[2],
                rot_x(-1.45),
            );
        }
        2 => {
            for k in 0..4u64 {
                let foot = Vec3::new(0.3 * (k as f32 - 1.5), 0.0, 0.2 * hash01(31, k)) * unit;
                s.beam(
                    foot,
                    foot + Vec3::new(
                        0.12 * (hash01(32, k) - 0.5),
                        0.62,
                        0.2 * (hash01(33, k) - 0.5),
                    ) * unit,
                    0.025 * unit,
                    WOOD_FRESH,
                );
            }
        }
        3 => {
            s.beam(
                Vec3::ZERO,
                Vec3::new(-0.35, 1.5, 0.0) * unit,
                0.05 * unit,
                WOOD,
            );
            s.cuboid(
                Vec3::new(-0.12, 1.2, 0.0) * unit,
                Vec3::new(0.5, 0.36, 0.02) * unit,
                SHIELD_COLORS[1],
                rot_z(0.23),
            );
        }
        _ => {
            let lean = rot_x(-1.15);
            for k in 0..8 {
                let a = TAU * k as f32 / 8.0;
                let a2 = TAU * (k + 1) as f32 / 8.0;
                let hub = Vec3::new(0.0, 0.2 * unit, 0.0);
                let p1 = lean * (Vec3::new(a.cos(), a.sin(), 0.0) * 0.5 * unit) + hub;
                let p2 = lean * (Vec3::new(a2.cos(), a2.sin(), 0.0) * 0.5 * unit) + hub;
                s.beam(p1, p2, 0.06 * unit, WOOD_DARK);
                if k % 2 == 0 {
                    s.beam(hub, p1, 0.035 * unit, WOOD);
                }
            }
        }
    }
    s.into_mesh()
}

/// Crow in flight (wingspan `span`), wings in an open V, beak towards +Z.
pub fn crow(span: f32) -> Mesh {
    let mut s = Soup::default();
    let half = span * 0.5;
    for side in [-1.0f32, 1.0] {
        for p in [
            Vec3::new(0.0, 0.0, 0.18 * span),
            Vec3::new(side * half, 0.12 * span, -0.05 * span),
            Vec3::new(0.0, 0.0, -0.16 * span),
        ] {
            s.vertex(p, Vec3::Y, CROW);
        }
    }
    for p in [
        Vec3::new(-0.05 * span, 0.0, 0.1 * span),
        Vec3::new(0.05 * span, 0.0, 0.1 * span),
        Vec3::new(0.0, 0.0, -0.36 * span),
    ] {
        s.vertex(p, Vec3::Y, CROW);
    }
    s.into_mesh()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangles(mesh: &Mesh) -> usize {
        mesh.indices().map_or(0, |i| i.len() / 3)
    }

    #[test]
    fn meshes_have_geometry_and_vertex_colours() {
        for mesh in [
            plague_pit(),
            dead_cart(),
            mound(3.0, 1.2),
            crow(2.0),
            debris(0, 4.3),
            debris(1, 4.3),
            debris(2, 4.3),
            debris(3, 4.3),
            debris(4, 4.3),
        ] {
            assert!(triangles(&mesh) >= 1);
            assert!(mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_some());
        }
        assert!(triangles(&plague_pit()) > 100);
        assert_eq!(triangles(&crow(1.0)), 3);
    }

    #[test]
    fn hash_is_stable_and_in_range() {
        assert_eq!(hash01(3, 4), hash01(3, 4));
        assert!((0..200).all(|i| (0.0..1.0).contains(&hash01(9, i))));
    }
}
