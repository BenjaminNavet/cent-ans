//! Branched battle trees (port of game/scripts/battle/battle_trees.gd and the
//! `battle_tree_bark` / `battle_tree_foliage` shaders; textures of tree_textures.gd, TX T3).
//!
//! Skeleton: trunk, main limbs, branches, twigs (recursive, one seed per species), kept inside the
//! crown envelope of the species; the end twigs carry sprays of two cards (one along the twig, one
//! facing outward) with rounded normals and a baked self-shade in the vertex colour (darker core
//! and bottom of the crown). Winter: bare twigs (`twig_spray.png`), the marcescent oak keeps its
//! dead leaves (`dead_leaves_oak.png`).
//!
//! Levels of detail (Bevy `VisibilityRange`, dithered cross-fade like `battle_tree_lod`): full mesh
//! up to `LOD1_DISTANCE`, light mesh (same skeleton, half the sprays enlarged, thin branches
//! without bark) up to the impostors of `far_woods.rs` (third level).
//!
//! Textures: the regional bark of the species (`tx_bark_pack`, albedo + normal) and its leaves
//! sheet (`tx_leaves_pack`) when `tree_species.json` names them, otherwise the photographed
//! `bark_brown_02` and the `leaf_spray_<species>.png` card. The layers are cut out of the packs
//! once loaded into plain 2D images (mip-mapped; leaves desaturated like the shader's
//! `decor_saturation` and with the alpha raised in the small mips so the crowns do not empty out
//! at a distance), so Bevy's standard pipeline (shadows, prepass, alpha mask) draws them.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Full mesh up to this distance (m), then the light mesh.
pub const LOD1_DISTANCE: f32 = 120.0;
/// Bushes and hedges: full mesh up close only.
pub const BUSH_LOD1_DISTANCE: f32 = 60.0;
/// Width (m) of the dithered cross-fade between two levels.
pub const LOD_BAND: f32 = 16.0;
/// Part of the saturation kept by the foliage (DA bible 3.3).
pub const DECOR_SATURATION: f32 = 0.6;

/// Species parameters (`SPECIES` of battle_trees.gd).
#[derive(Debug, Clone, Copy)]
pub struct TreeSpecies {
    pub name: &'static str,
    pub height: f32,
    pub width: f32,
    pub crown_base: f32,
    pub trunk: f32,
    pub radius: f32,
    pub limbs: u32,
    pub spread: f32,
    pub leader: bool,
    pub levels: u32,
    pub card: f32,
    pub gnarl: f32,
    pub rise: f32,
    pub leafy: u32,
    pub bark: [f32; 3],
    pub smooth: f32,
    pub leaf: [f32; 3],
    pub marcescent: bool,
}

pub const TREE_SPECIES: [TreeSpecies; 7] = [
    TreeSpecies {
        name: "oak",
        height: 15.0,
        width: 14.5,
        crown_base: 3.6,
        trunk: 3.4,
        radius: 0.46,
        limbs: 5,
        spread: 0.95,
        leader: false,
        levels: 3,
        card: 3.3,
        gnarl: 0.4,
        rise: 0.12,
        leafy: 3,
        bark: [0.74, 0.7, 0.64],
        smooth: 0.0,
        leaf: [0.9, 0.96, 0.82],
        marcescent: true,
    },
    TreeSpecies {
        name: "beech",
        height: 19.0,
        width: 12.5,
        crown_base: 4.8,
        trunk: 5.2,
        radius: 0.38,
        limbs: 4,
        spread: 0.72,
        leader: true,
        levels: 3,
        card: 3.1,
        gnarl: 0.14,
        rise: 0.25,
        leafy: 3,
        bark: [0.92, 0.92, 0.9],
        smooth: 1.0,
        leaf: [1.0, 1.0, 0.8],
        marcescent: false,
    },
    TreeSpecies {
        name: "ash",
        height: 18.0,
        width: 10.5,
        crown_base: 6.0,
        trunk: 6.2,
        radius: 0.32,
        limbs: 4,
        spread: 0.62,
        leader: true,
        levels: 3,
        card: 2.6,
        gnarl: 0.2,
        rise: 0.35,
        leafy: 2,
        bark: [0.9, 0.88, 0.84],
        smooth: 0.5,
        leaf: [1.0, 1.06, 0.88],
        marcescent: false,
    },
    TreeSpecies {
        name: "poplar",
        height: 22.0,
        width: 9.0,
        crown_base: 5.0,
        trunk: 6.0,
        radius: 0.42,
        limbs: 6,
        spread: 0.45,
        leader: true,
        levels: 3,
        card: 2.8,
        gnarl: 0.3,
        rise: 0.5,
        leafy: 3,
        bark: [0.7, 0.68, 0.64],
        smooth: 0.2,
        leaf: [0.94, 1.0, 0.82],
        marcescent: false,
    },
    TreeSpecies {
        name: "willow",
        height: 8.5,
        width: 7.5,
        crown_base: 2.6,
        trunk: 2.5,
        radius: 0.52,
        limbs: 12,
        spread: 0.5,
        leader: false,
        levels: 1,
        card: 2.1,
        gnarl: 0.06,
        rise: 0.55,
        leafy: 4,
        bark: [0.68, 0.66, 0.62],
        smooth: 0.0,
        leaf: [0.95, 1.0, 0.96],
        marcescent: false,
    },
    TreeSpecies {
        name: "fruit",
        height: 5.6,
        width: 6.2,
        crown_base: 1.8,
        trunk: 1.6,
        radius: 0.17,
        limbs: 4,
        spread: 1.0,
        leader: false,
        levels: 2,
        card: 2.0,
        gnarl: 0.45,
        rise: 0.1,
        leafy: 3,
        bark: [0.7, 0.66, 0.6],
        smooth: 0.0,
        leaf: [0.95, 1.0, 0.84],
        marcescent: false,
    },
    TreeSpecies {
        name: "bush",
        height: 2.4,
        width: 3.0,
        crown_base: 0.2,
        trunk: 0.0,
        radius: 0.06,
        limbs: 6,
        spread: 0.55,
        leader: false,
        levels: 1,
        card: 1.8,
        gnarl: 0.3,
        rise: 0.3,
        leafy: 3,
        bark: [0.7, 0.66, 0.6],
        smooth: 0.0,
        leaf: [0.9, 0.96, 0.82],
        marcescent: false,
    },
];

pub fn tree_species(name: &str) -> &'static TreeSpecies {
    TREE_SPECIES
        .iter()
        .find(|s| s.name == name)
        .unwrap_or(&TREE_SPECIES[0])
}

/// Deterministic generator of the skeleton (same draw at every level of detail).
struct Rng(u64);

impl Rng {
    fn new(name: &str) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in name.bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
        Rng(h | 1)
    }

    fn randf(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.randf()
    }
}

#[derive(Default)]
struct Buffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

/// Bark and foliage meshes of one species at one level.
pub struct TreeMeshes {
    /// `None` when the level draws no bark (light bush).
    pub bark: Option<Mesh>,
    pub leaves: Mesh,
    pub clusters: usize,
    pub bark_quads: usize,
}

struct Ctx<'a> {
    p: &'a TreeSpecies,
    rng: Rng,
    lod: u32,
    bark: Buffers,
    leaves: Buffers,
    center: Vec3,
    radii: Vec3,
    clusters: usize,
    bark_quads: usize,
}

/// Mesh of `species` at level `lod` (0 full, 1 light); `winter` only changes the textures.
pub fn build_tree(species: &TreeSpecies, lod: u32) -> TreeMeshes {
    let height = species.height;
    let base = species.crown_base;
    let mut ctx = Ctx {
        p: species,
        rng: Rng::new(species.name),
        lod,
        bark: Buffers::default(),
        leaves: Buffers::default(),
        center: Vec3::new(0.0, (base + height) * 0.5, 0.0),
        radii: Vec3::new(
            species.width * 0.5,
            (height - base) * 0.5,
            species.width * 0.5,
        ),
        clusters: 0,
        bark_quads: 0,
    };
    let reach = ctx.radii.x.max(ctx.radii.y);
    let foot = Vec3::new(0.0, -0.4, 0.0);
    if species.trunk > 0.0 {
        let lean =
            Vec3::new(ctx.rng.range(-0.07, 0.07), 1.0, ctx.rng.range(-0.07, 0.07)).normalize();
        let top = foot + lean * (species.trunk + 0.4);
        cylinder(
            &mut ctx,
            foot,
            top,
            species.radius * 1.15,
            species.radius * 0.8,
            0,
        );
        if species.name == "willow" {
            // Pollard willow: swollen head from which the straight shoots grow.
            cylinder(
                &mut ctx,
                top - Vec3::Y * 0.2,
                top + Vec3::Y * 0.35,
                species.radius * 1.25,
                species.radius * 0.9,
                0,
            );
        }
        for i in 0..species.limbs {
            let az = TAU * i as f32 / species.limbs as f32 + ctx.rng.range(-0.4, 0.4);
            let out = Vec3::new(az.cos(), 0.0, az.sin());
            let tilt = species.spread * ctx.rng.range(0.75, 1.15);
            let dir = (Vec3::Y * tilt.cos() + out * tilt.sin()).normalize();
            let t = if i > 1 { ctx.rng.range(0.0, 0.3) } else { 0.0 };
            let start = top.lerp(foot, t);
            let length = reach * ctx.rng.range(0.75, 1.0);
            branch(&mut ctx, start, dir, length, species.radius * 0.62, 1);
        }
        if species.leader {
            branch(
                &mut ctx,
                top,
                lean,
                (height - species.trunk) * 0.8,
                species.radius * 0.7,
                1,
            );
        }
    } else {
        // Bush: stems from the ground.
        for i in 0..species.limbs {
            let az = TAU * i as f32 / species.limbs as f32 + ctx.rng.range(-0.5, 0.5);
            let tilt = species.spread * ctx.rng.range(0.5, 1.2);
            let dir = (Vec3::Y * tilt.cos() + Vec3::new(az.cos(), 0.0, az.sin()) * tilt.sin())
                .normalize();
            let start = Vec3::new(ctx.rng.range(-0.2, 0.2), -0.1, ctx.rng.range(-0.2, 0.2));
            branch(&mut ctx, start, dir, reach * 1.2, species.radius, 1);
        }
    }
    let bark = (!ctx.bark.indices.is_empty()).then(|| {
        let mut mesh = into_mesh(std::mem::take(&mut ctx.bark), false);
        if let Err(error) = mesh.generate_tangents() {
            warn!("battle trees: no tangents for the bark: {error}");
        }
        mesh
    });
    TreeMeshes {
        bark,
        leaves: into_mesh(std::mem::take(&mut ctx.leaves), true),
        clusters: ctx.clusters,
        bark_quads: ctx.bark_quads,
    }
}

fn into_mesh(b: Buffers, colors: bool) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, b.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, b.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, b.uvs);
    if colors {
        mesh = mesh.with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, b.colors);
    }
    mesh.with_inserted_indices(Indices::U32(b.indices))
}

/// Rotates `v` round the unit `axis` by `angle` (Godot `Vector3.rotated`).
fn rotated(v: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    Quat::from_axis_angle(axis, angle) * v
}

fn branch(ctx: &mut Ctx, start: Vec3, mut dir: Vec3, length: f32, radius: f32, level: u32) {
    let p = ctx.p;
    let segs: usize = if level <= 1 { 3 } else { 2 };
    let mut points = vec![start];
    let mut dirs = Vec::with_capacity(segs);
    let mut pos = start;
    for s in 0..segs {
        let jitter = Vec3::new(
            ctx.rng.range(-1.0, 1.0),
            ctx.rng.range(-1.0, 1.0),
            ctx.rng.range(-1.0, 1.0),
        ) * p.gnarl
            * 0.5;
        dir = (dir + jitter + Vec3::Y * p.rise * 0.3).normalize_or(Vec3::Y);
        let mut next = pos + dir * length / segs as f32;
        // Crown envelope: a twig that leaves it is brought back (a slightly bumpy edge).
        let rel = (next - ctx.center) / ctx.radii;
        let limit = 1.0 + ctx.rng.range(-0.12, 0.05);
        if rel.length() > limit {
            next = ctx.center + rel.normalize() * limit * ctx.radii;
            if (next - pos).length() > 0.05 {
                dir = (next - pos).normalize();
            }
        }
        let r0 = radius * (1.0 - 0.35 * s as f32 / segs as f32);
        let r1 = radius * (1.0 - 0.35 * (s + 1) as f32 / segs as f32);
        let drawn_level = if ctx.lod == 0 { 2 } else { 1 };
        if level <= drawn_level {
            cylinder(ctx, pos, next, r0, r1, level);
        }
        pos = next;
        points.push(pos);
        dirs.push(dir);
    }
    if level < p.levels {
        for c in 0..3usize {
            let idx = (segs as i64 - c as i64).clamp(1, segs as i64) as usize;
            let at = points[idx];
            let along = dirs[idx - 1];
            let reference = if along.y.abs() < 0.95 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let side = along.cross(reference).normalize();
            let side = rotated(side, along, ctx.rng.randf() * TAU);
            let tilt = p.spread * ctx.rng.range(0.55, 1.0);
            let child = (along * tilt.cos() + side * tilt.sin()).normalize();
            let child_length = length * ctx.rng.range(0.55, 0.72);
            branch(ctx, at, child, child_length, radius * 0.55, level + 1);
        }
    } else {
        // End twig: sprays at the tip and along it.
        let leafy = p.leafy.max(1);
        for k in 0..leafy {
            let t = 1.0 - k as f32 / leafy as f32 * 0.6;
            let idx = ((t * segs as f32).round() as usize).clamp(1, segs);
            let at = points[idx - 1].lerp(points[idx], ctx.rng.range(0.4, 1.0));
            cluster(ctx, at, dirs[idx - 1]);
        }
    }
}

/// Spray: one card along the twig (foot on the twig) and one facing outward.
fn cluster(ctx: &mut Ctx, at: Vec3, along: Vec3) {
    let p = ctx.p;
    let index = ctx.clusters;
    ctx.clusters += 1;
    let mut size = p.card * ctx.rng.range(0.8, 1.2);
    let mut outward = (at - ctx.center) / ctx.radii;
    if outward.length() < 0.05 {
        outward = Vec3::Y;
    }
    let o = (outward.normalize() * 0.6 + along * 0.4).normalize_or(Vec3::Y);
    let spin = ctx.rng.randf() * TAU;
    let shade = ctx.rng.range(0.88, 1.08);
    let hue = Vec3::new(ctx.rng.range(0.96, 1.04), 1.0, ctx.rng.range(0.92, 1.03));
    if ctx.lod == 1 {
        // Light: one spray in two, enlarged (same mass seen from afar).
        if index % 2 == 1 {
            return;
        }
        size *= 1.45;
    }
    let reference = if o.y.abs() < 0.95 { Vec3::Y } else { Vec3::X };
    let side = rotated(o.cross(reference).normalize(), o, spin);
    let up = o;
    let corners = [
        at - side * size * 0.5 - up * size * 0.15,
        at + side * size * 0.5 - up * size * 0.15,
        at + side * size * 0.5 + up * size * 0.85,
        at - side * size * 0.5 + up * size * 0.85,
    ];
    card(ctx, corners, at, shade, hue);
    let a = side;
    let b = o.cross(a).normalize();
    let mid = at + o * size * 0.25;
    let corners = [
        mid - a * size * 0.5 - b * size * 0.5,
        mid + a * size * 0.5 - b * size * 0.5,
        mid + a * size * 0.5 + b * size * 0.5,
        mid - a * size * 0.5 + b * size * 0.5,
    ];
    card(ctx, corners, at, shade, hue);
}

fn card(ctx: &mut Ctx, corners: [Vec3; 4], cluster: Vec3, shade: f32, hue: Vec3) {
    let p = ctx.p;
    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let base = ctx.leaves.positions.len() as u32;
    for (v, uv) in corners.iter().zip(uvs) {
        let rel = (*v - ctx.center) / ctx.radii;
        let n = (rel.normalize_or(Vec3::Y)
            + (*v - cluster).normalize_or(Vec3::Y) * 0.4
            + Vec3::Y * 0.25)
            .normalize_or(Vec3::Y);
        // Self-shade: darker core and bottom of the crown.
        let depth = rel.length().clamp(0.0, 1.0);
        let low = ((v.y - p.crown_base) / (p.height - p.crown_base).max(0.5)).clamp(0.0, 1.0);
        let k = (0.5 + 0.5 * depth.powf(1.3)) * (0.78 + 0.22 * low) * shade;
        ctx.leaves.positions.push(v.to_array());
        ctx.leaves.normals.push(n.to_array());
        ctx.leaves.uvs.push(uv);
        ctx.leaves
            .colors
            .push([k * hue.x, k * hue.y, k * hue.z, 1.0]);
    }
    ctx.leaves
        .indices
        .extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
}

fn cylinder(ctx: &mut Ctx, a: Vec3, b: Vec3, ra: f32, rb: f32, level: u32) {
    let mut sides: u32 = [8, 6, 4][level.min(2) as usize];
    if ctx.lod == 1 {
        sides = sides.saturating_sub(3).max(3);
    }
    if ctx.p.trunk <= 0.0 {
        // Bushes (hedges: thousands): 3-sided stems at full detail, none further.
        if ctx.lod == 1 {
            return;
        }
        sides = 3;
    }
    ctx.bark_quads += sides as usize;
    let axis = (b - a).normalize_or(Vec3::Y);
    let reference = if axis.dot(Vec3::NEG_Z).abs() < 0.9 {
        Vec3::NEG_Z
    } else {
        Vec3::X
    };
    let u = axis.cross(reference).normalize();
    let w = axis.cross(u).normalize();
    let length = a.distance(b);
    let circ = (ra * TAU).max(0.3);
    for i in 0..sides {
        let t0 = TAU * i as f32 / sides as f32;
        let t1 = TAU * (i + 1) as f32 / sides as f32;
        let n0 = u * t0.cos() + w * t0.sin();
        let n1 = u * t1.cos() + w * t1.sin();
        let pts = [a + n0 * ra, b + n0 * rb, b + n1 * rb, a + n1 * ra];
        let u0 = i as f32 / sides as f32 * circ;
        let u1 = (i + 1) as f32 / sides as f32 * circ;
        let uv = [[u0, length / 1.5], [u0, 0.0], [u1, 0.0], [u1, length / 1.5]];
        let ns = [n0, n0, n1, n1];
        let base = ctx.bark.positions.len() as u32;
        for k in 0..4 {
            ctx.bark.positions.push(pts[k].to_array());
            ctx.bark.normals.push(ns[k].to_array());
            ctx.bark.uvs.push(uv[k]);
        }
        ctx.bark
            .indices
            .extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
}

// --- Textures -------------------------------------------------------------------------------

/// What to do with a source texture once loaded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TexturePrep {
    /// Leaf card: desaturated, alpha raised in the small mips.
    Leaves,
    /// Bark albedo; `smooth` (beech) pulls it towards a light grey.
    Bark { smooth: f32 },
    /// Bark normal map; its relief is flattened by `smooth`.
    BarkNormal { smooth: f32 },
}

/// A layer to cut out of a loaded texture into `target`.
#[derive(Debug, Clone)]
pub struct PendingTreeTexture {
    pub source: Handle<Image>,
    pub target: Handle<Image>,
    /// Grid of the source (columns, rows) and layer (row by row); (1, 1) and 0 for a plain image.
    pub grid: (u32, u32),
    pub layer: u32,
    pub prep: TexturePrep,
    /// Micro-bark grain (`tx_micro_bark.jpg`) baked into the albedo of a regional bark.
    pub micro: Option<Handle<Image>>,
}

/// Strength of the micro-bark grain (`MICRO_FADE.x` of tree_textures.gd).
pub const MICRO_BARK_STRENGTH: f32 = 0.7;
/// Repeats of the micro grain over a bark cell (Godot `UV * 3.7`; a whole number here so the
/// baked cell still tiles without a seam).
pub const MICRO_BARK_REPEAT: f32 = 4.0;

/// Multiplies the bark `pixels` (RGBA8 sRGB, `size` square) by the micro grain `grain` (its
/// 8-bit luminance, `grain_size` square, bilinear): `1 + (g - 0.5) * 1.6 * strength` in linear
/// light, as `battle_tree_bark.gdshader` does near the camera. The mip chain fades the grain with
/// the distance (the shader fades it between 3 and 14 m).
pub fn bake_micro_bark(pixels: &mut [u8], size: u32, grain: &[u8], grain_size: u32) {
    let (n, m) = (size as usize, grain_size as usize);
    if m == 0 || grain.len() < m * m {
        return;
    }
    for y in 0..n {
        for x in 0..n {
            let fx = ((x as f32 + 0.5) / n as f32 * MICRO_BARK_REPEAT).fract() * m as f32 - 0.5;
            let fy = ((y as f32 + 0.5) / n as f32 * MICRO_BARK_REPEAT).fract() * m as f32 - 0.5;
            let (x0, y0) = (fx.floor(), fy.floor());
            let (tx, ty) = (fx - x0, fy - y0);
            let at = |ix: f32, iy: f32| {
                let (ix, iy) = (
                    ix.rem_euclid(m as f32) as usize,
                    iy.rem_euclid(m as f32) as usize,
                );
                f32::from(grain[iy * m + ix]) / 255.0
            };
            let top = at(x0, y0) + (at(x0 + 1.0, y0) - at(x0, y0)) * tx;
            let bottom = at(x0, y0 + 1.0) + (at(x0 + 1.0, y0 + 1.0) - at(x0, y0 + 1.0)) * tx;
            let g = top + (bottom - top) * ty;
            let gain = (1.0 + (g - 0.5) * 1.6 * MICRO_BARK_STRENGTH).max(0.0);
            let px = &mut pixels[(y * n + x) * 4..(y * n + x) * 4 + 3];
            for c in px.iter_mut() {
                *c = linear_to_srgb(srgb_to_linear(*c) * gain);
            }
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct PendingTreeTextures(pub Vec<PendingTreeTexture>);

/// Cuts layer `layer` of a `cols` x `rows` grid of square RGBA8 cells; `None` on a bad shape.
pub fn cut_layer(
    data: &[u8],
    width: u32,
    height: u32,
    grid: (u32, u32),
    layer: u32,
) -> Option<(Vec<u8>, u32)> {
    let (cols, rows) = grid;
    if cols == 0
        || rows == 0
        || !width.is_multiple_of(cols)
        || !height.is_multiple_of(rows)
        || layer >= cols * rows
    {
        return None;
    }
    let size = width / cols;
    if height / rows != size || data.len() != (width * height * 4) as usize {
        return None;
    }
    let (col, row) = ((layer % cols) as usize, (layer / cols) as usize);
    let size_us = size as usize;
    let mut out = Vec::with_capacity(size_us * size_us * 4);
    for y in 0..size_us {
        let start = ((row * size_us + y) * width as usize + col * size_us) * 4;
        out.extend_from_slice(&data[start..start + size_us * 4]);
    }
    Some((out, size))
}

fn srgb_to_linear(v: u8) -> f32 {
    let c = f32::from(v) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

/// Applies `prep` to RGBA8 pixels (sRGB for the albedos).
pub fn prepare_pixels(pixels: &mut [u8], prep: TexturePrep) {
    for px in pixels.as_chunks_mut::<4>().0 {
        match prep {
            TexturePrep::Leaves | TexturePrep::Bark { .. } => {
                let rgb = [px[0], px[1], px[2]].map(srgb_to_linear);
                let luma = rgb[0] * 0.3 + rgb[1] * 0.59 + rgb[2] * 0.11;
                let out = match prep {
                    TexturePrep::Bark { smooth } => {
                        rgb.map(|c| c + (luma * 1.25 - c) * smooth * 0.7)
                    }
                    _ => rgb.map(|c| luma + (c - luma) * DECOR_SATURATION),
                };
                px[0] = linear_to_srgb(out[0]);
                px[1] = linear_to_srgb(out[1]);
                px[2] = linear_to_srgb(out[2]);
            }
            TexturePrep::BarkNormal { smooth } => {
                let depth = 1.0 - 0.7 * smooth;
                for c in px.iter_mut().take(2) {
                    let n = (f32::from(*c) / 255.0 - 0.5) * depth + 0.5;
                    *c = (n * 255.0).round() as u8;
                }
            }
        }
    }
}

/// Mip chain of a square RGBA8 layer. For the leaves (Mask(0.5) cards), two corrections against
/// the pale solid cards seen from afar:
/// - the colour of each level is averaged weighted by alpha (the colour of the transparent texels
///   must not bleed into the leaves);
/// - the alpha of each level is scaled so that the share of texels above the 0.5 threshold stays
///   the one of level 0 (a fixed boost per level, as in the old foliage shader, made every card
///   solid from level 3 on: 36 % coverage x 1.35^l > 50 %).
pub fn tree_mips(layer: &[u8], size: u32, leaves: bool) -> (Vec<u8>, u32) {
    let levels = super::ground::mip_count(size);
    if !leaves {
        return (super::ground::mip_chain(layer, size as usize), levels);
    }
    let covered = |alpha: &[u8], scale: f32| {
        alpha
            .iter()
            .filter(|a| f32::from(**a) * scale >= 127.5)
            .count() as f32
            / alpha.len().max(1) as f32
    };
    let base_alpha: Vec<u8> = layer.chunks_exact(4).map(|px| px[3]).collect();
    let target = covered(&base_alpha, 1.0);
    let mut out = layer.to_vec();
    let mut previous = layer.to_vec();
    let mut dim = size as usize;
    while dim > 1 {
        let next_dim = dim / 2;
        let mut next = vec![0u8; next_dim * next_dim * 4];
        for y in 0..next_dim {
            for x in 0..next_dim {
                let mut sum = [0.0f32; 3];
                let mut weight = 0.0f32;
                let mut alpha = 0.0f32;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let at = ((y * 2 + dy) * dim + x * 2 + dx) * 4;
                    let a = f32::from(previous[at + 3]);
                    for c in 0..3 {
                        sum[c] += f32::from(previous[at + c]) * (a + 1.0);
                    }
                    weight += a + 1.0;
                    alpha += a;
                }
                let o = (y * next_dim + x) * 4;
                for c in 0..3 {
                    next[o + c] = (sum[c] / weight).round().min(255.0) as u8;
                }
                next[o + 3] = (alpha / 4.0).round() as u8;
            }
        }
        // Scale this level's alpha to the base coverage (bisection on the factor, 1 to 16).
        let alphas: Vec<u8> = next.chunks_exact(4).map(|px| px[3]).collect();
        let (mut lo, mut hi) = (1.0f32, 16.0f32);
        for _ in 0..12 {
            let mid = (lo + hi) * 0.5;
            if covered(&alphas, mid) < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let mut shown = next.clone();
        for px in shown.chunks_exact_mut(4) {
            px[3] = (f32::from(px[3]) * hi).min(255.0) as u8;
        }
        // The next level is built from the unscaled alpha (no compounding).
        out.extend_from_slice(&shown);
        previous = next;
        dim = next_dim;
    }
    (out, levels)
}

/// Placeholder image (one white texel) replaced once the source is loaded.
pub fn placeholder(srgb: bool) -> Image {
    Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![255, 255, 255, 255],
        if srgb {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        },
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Statistics of the prepared textures (dump).
#[derive(Resource, Debug, Clone, Default)]
pub struct TreeTextureStats {
    pub prepared: usize,
    pub failed: usize,
}

/// Materials whose textures are filled in later by [`prepare_tree_textures`]. Bevy does not rebuild
/// a material's bind group when a texture it points to is replaced in place (the 1 x 1 white
/// placeholder stays bound: pale solid cards), so the materials are touched once textures land.
#[derive(Resource, Default)]
pub struct TreeMaterials {
    pub wind: Vec<Handle<super::wind::WindMaterial>>,
    pub standard: Vec<Handle<StandardMaterial>>,
}

pub fn prepare_tree_textures(
    mut commands: Commands,
    mut pending: ResMut<PendingTreeTextures>,
    mut images: ResMut<Assets<Image>>,
    mut stats: ResMut<TreeTextureStats>,
    tree_materials: Option<Res<TreeMaterials>>,
    mut wind_materials: ResMut<Assets<super::wind::WindMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut touch: Local<u8>,
) {
    let prepared_before = stats.prepared;
    pending.0.retain(|job| {
        let Some(source) = images.get(&job.source) else {
            return true;
        };
        // The micro grain must be loaded too (its luminance, one byte per texel).
        let grain: Option<(Vec<u8>, u32)> = match &job.micro {
            Some(handle) => match images.get(handle) {
                None => return true,
                Some(micro) => micro.data.as_ref().and_then(|data| {
                    let stride = micro.texture_descriptor.format.block_copy_size(None)? as usize;
                    (micro.width() == micro.height() && stride > 0).then(|| {
                        (
                            data.iter().step_by(stride).copied().collect::<Vec<u8>>(),
                            micro.width(),
                        )
                    })
                }),
            },
            None => None,
        };
        let srgb = source.texture_descriptor.format.is_srgb();
        let cut = source.data.as_ref().and_then(|data| {
            if source.texture_descriptor.format.block_copy_size(None) != Some(4) {
                return None;
            }
            cut_layer(data, source.width(), source.height(), job.grid, job.layer)
        });
        let Some((mut pixels, size)) = cut.filter(|(_, size)| size.is_power_of_two()) else {
            stats.failed += 1;
            warn!("battle trees: texture layer {} not usable", job.layer);
            return false;
        };
        prepare_pixels(&mut pixels, job.prep);
        if let Some((grain, grain_size)) = &grain
            && matches!(job.prep, TexturePrep::Bark { .. })
        {
            bake_micro_bark(&mut pixels, size, grain, *grain_size);
        }
        let (data, levels) = tree_mips(&pixels, size, job.prep == TexturePrep::Leaves);
        let mut image = Image::new(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data[..(size * size * 4) as usize].to_vec(),
            if srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            },
            RenderAssetUsages::RENDER_WORLD,
        );
        // `Image::new` only accepts the base level; the chain replaces it afterwards.
        image.data = Some(data);
        image.texture_descriptor.mip_level_count = levels;
        let address = if job.prep == TexturePrep::Leaves {
            ImageAddressMode::ClampToEdge
        } else {
            ImageAddressMode::Repeat
        };
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: address,
            address_mode_v: address,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            mipmap_filter: ImageFilterMode::Linear,
            anisotropy_clamp: if job.prep == TexturePrep::Leaves {
                1
            } else {
                8
            },
            ..default()
        });
        if let Some(mut target) = images.get_mut(&job.target) {
            *target = image;
            stats.prepared += 1;
        }
        false
    });
    if stats.prepared != prepared_before {
        *touch = 4;
    }
    // Touched on a few frames: the render world uploads the new images a frame or two later.
    if *touch > 0 {
        *touch -= 1;
        if let Some(handles) = tree_materials {
            for handle in &handles.wind {
                if let Some(material) = wind_materials.get_mut(handle) {
                    // `AssetMut` only queues `Modified` once dereferenced mutably.
                    material.into_inner();
                }
            }
            for handle in &handles.standard {
                if let Some(material) = standard_materials.get_mut(handle) {
                    material.into_inner();
                }
            }
        }
    }
    if pending.0.is_empty() && *touch == 0 {
        commands.remove_resource::<PendingTreeTextures>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_species_builds_both_levels() {
        for species in &TREE_SPECIES {
            let full = build_tree(species, 0);
            let light = build_tree(species, 1);
            assert!(full.clusters > 0, "{}", species.name);
            assert_eq!(full.clusters, light.clusters, "same skeleton");
            assert!(full.leaves.count_vertices() > light.leaves.count_vertices());
            if species.trunk > 0.0 {
                assert!(full.bark_quads > light.bark_quads, "{}", species.name);
                assert!(light.bark.is_some());
            } else {
                assert!(light.bark.is_none(), "light bush has no stems");
            }
            for bark in [&full.bark, &light.bark].into_iter().flatten() {
                assert!(
                    bark.count_vertices() > 0,
                    "{}: empty bark mesh",
                    species.name
                );
            }
        }
    }

    #[test]
    fn sprays_stay_near_the_crown_envelope() {
        let oak = tree_species("oak");
        let tree = build_tree(oak, 0);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            tree.leaves.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions")
        };
        let top = positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        assert!(top < oak.height + oak.card * 1.2, "top {top}");
        assert!(top > oak.height * 0.7, "top {top}");
    }

    #[test]
    fn cut_layer_picks_the_cell_row_by_row() {
        // 2 x 2 grid of 2 px cells, each cell filled with its index.
        let mut data = vec![0u8; 4 * 4 * 4];
        for y in 0..4 {
            for x in 0..4 {
                let cell = (y / 2) * 2 + x / 2;
                data[(y * 4 + x) * 4] = cell as u8;
            }
        }
        for layer in 0..4 {
            let (cut, size) = cut_layer(&data, 4, 4, (2, 2), layer).unwrap();
            assert_eq!(size, 2);
            assert!(cut.as_chunks::<4>().0.iter().all(|px| px[0] == layer as u8));
        }
        assert!(cut_layer(&data, 4, 4, (2, 2), 4).is_none());
    }

    #[test]
    fn leaf_mips_keep_their_coverage() {
        // 8 x 8, one texel in three opaque: coverage stays near a third at every level, it never
        // goes solid; the colour of the transparent texels does not bleed in.
        let size = 8usize;
        let mut layer = vec![0u8; size * size * 4];
        for (i, px) in layer.chunks_exact_mut(4).enumerate() {
            if i % 3 == 0 {
                px.copy_from_slice(&[10, 120, 10, 255]);
            } else {
                px.copy_from_slice(&[255, 255, 255, 0]);
            }
        }
        let (mips, levels) = tree_mips(&layer, size as u32, true);
        assert_eq!(levels, 4);
        // Level 2 (2 x 2) starts after 64 + 16 texels.
        let level2 = &mips[(64 + 16) * 4..(64 + 16 + 4) * 4];
        assert!(level2.chunks_exact(4).any(|px| px[3] < 255), "not solid");
        for px in level2.chunks_exact(4) {
            assert!(px[0] < 200, "no white bleed: {px:?}");
        }
    }

    #[test]
    fn leaves_are_desaturated() {
        let mut px = [40u8, 200, 40, 255];
        prepare_pixels(&mut px, TexturePrep::Leaves);
        assert!(px[1] < 200 && px[0] > 40);
        let mut n = [255u8, 0, 255, 255];
        prepare_pixels(&mut n, TexturePrep::BarkNormal { smooth: 1.0 });
        assert!(n[0] < 200 && n[1] > 50);
    }

    #[test]
    fn micro_bark_grain_modulates_the_albedo_around_its_mean() {
        let base = vec![128u8; 4 * 4 * 4];
        let bake = |grain: u8| {
            let mut px = base.clone();
            bake_micro_bark(&mut px, 4, &[grain; 4], 2);
            px[0]
        };
        assert!(bake(255) > 128 && bake(0) < 128);
        assert!((i32::from(bake(128)) - 128).abs() <= 1);
    }
}
