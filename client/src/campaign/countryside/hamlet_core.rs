//! Hamlet cores (VIS-L2, ADR 0824): a ground decal under every hamlet (beaten earth, kitchen
//! gardens, orchard, a few red roofs seen from above) instead of big isolated houses.
//!
//! The decal is a flat grid draped on the displayed ground, textured with an image generated at
//! load from the palettes of `hamlet_core` in `map_countryside.json` (four palettes, picked by a
//! hash of the hamlet). It is a ground feature, not a built mesh: the buildings themselves are the
//! dwelling props of the countryside layer, capped at three times their real size.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::LoadedCountryside;
use super::config::CorePalette;
use super::placement::{Terrain, cell_of};
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::terrain::surface::GroundHeight;

/// Decals created per frame at most.
const SPAWN_PER_FRAME: usize = 6;
/// Lift above the ground (world units), against z-fighting with the terrain.
const LIFT: f32 = 2.0e-5;

/// Live decals and shared assets.
#[derive(Resource, Default)]
pub struct HamletCores {
    live: HashMap<(i32, i32), Entity>,
    materials: Vec<Handle<StandardMaterial>>,
    relief_stamp: Option<super::ReliefVersion>,
}

pub fn reset(mut commands: Commands, mut cores: ResMut<HamletCores>) {
    for (_, e) in cores.live.drain() {
        commands.entity(e).despawn();
    }
    cores.materials.clear();
    cores.relief_stamp = None;
}

fn hash2(x: i32, y: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA6B) ^ salt;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Value noise in [0, 1] on a lattice of `freq` cells over the unit square.
fn noise(u: f32, v: f32, freq: f32, salt: u32) -> f32 {
    let (x, y) = (u * freq, v * freq);
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = hash2(xi, yi, salt);
    let b = hash2(xi + 1, yi, salt);
    let c = hash2(xi, yi + 1, salt);
    let d = hash2(xi + 1, yi + 1, salt);
    (a + (b - a) * sx) + ((c + (d - c) * sx) - (a + (b - a) * sx)) * sy
}

/// RGBA8 pixels of one decal: warm beaten earth in the middle, a ring of plots (gardens, orchard)
/// and a handful of red roofs near the centre; alpha falls to zero at the edge.
pub fn core_pixels(palette: &CorePalette, size: usize, salt: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(size * size * 4);
    for py in 0..size {
        for px in 0..size {
            let (u, v) = (
                (px as f32 + 0.5) / size as f32,
                (py as f32 + 0.5) / size as f32,
            );
            let (dx, dy) = (u - 0.5, v - 0.5);
            let r = (dx * dx + dy * dy).sqrt() * 2.0;
            // Ragged outline.
            let edge = 0.86 + 0.14 * (noise(u, v, 5.0, salt) - 0.5) * 2.0;
            let inside = 1.0 - ((r - 0.45 * edge) / (0.55 * edge)).clamp(0.0, 1.0);
            if inside <= 0.0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            // Plots: a coarse patchwork, gardens (rows) and orchard (dots).
            let plot = noise(u, v, 7.0, salt ^ 0x51);
            let rows = ((u * 90.0 + v * 30.0).sin() * 0.5 + 0.5) * 0.25;
            let dots = if noise(u, v, 38.0, salt ^ 0x9) > 0.72 {
                0.35
            } else {
                0.0
            };
            let mut rgb = palette.earth;
            let plot_weight = ((r - 0.30) / 0.2).clamp(0.0, 1.0);
            if plot_weight > 0.0 {
                let target = if plot > 0.5 {
                    let k = rows;
                    [
                        palette.garden[0] * (1.0 - k),
                        palette.garden[1] * (1.0 - k * 0.5),
                        palette.garden[2] * (1.0 - k),
                    ]
                } else {
                    let k = dots;
                    [
                        palette.orchard[0] * (1.0 - k),
                        palette.orchard[1] * (1.0 - k),
                        palette.orchard[2] * (1.0 - k),
                    ]
                };
                for c in 0..3 {
                    rgb[c] += (target[c] - rgb[c]) * plot_weight;
                }
            }
            // Red roofs: small gabled rectangles on a jittered ring near the centre.
            let ring_count = 5;
            let mut alpha = inside * (0.55 + 0.4 * (1.0 - r).clamp(0.0, 1.0));
            for k in 0..ring_count {
                let a = std::f32::consts::TAU * (k as f32 + hash2(k, 3, salt) * 0.7) / ring_count as f32;
                let rr = 0.14 + 0.12 * hash2(k, 5, salt);
                let (cx, cy) = (a.cos() * rr * 0.5, a.sin() * rr * 0.5);
                let (hx, hy) = (0.030 + 0.012 * hash2(k, 7, salt), 0.016 + 0.006 * hash2(k, 9, salt));
                if (dx - cx).abs() < hx && (dy - cy).abs() < hy {
                    let ridge = 1.0 - ((dy - cy).abs() / hy) * 0.35;
                    let tone = 0.85 + 0.25 * hash2(k, 11, salt);
                    rgb = [
                        palette.roof[0] * ridge * tone,
                        palette.roof[1] * ridge * tone,
                        palette.roof[2] * ridge * tone,
                    ];
                    alpha = 1.0;
                }
            }
            let grain = 0.94 + 0.12 * noise(u, v, 60.0, salt ^ 0x77);
            for c in rgb {
                out.push(((c * grain).clamp(0.0, 1.0) * 255.0) as u8);
            }
            out.push((alpha.clamp(0.0, 1.0) * 255.0) as u8);
        }
    }
    out
}

fn build_materials(
    cfg: &super::config::HamletCore,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Vec<Handle<StandardMaterial>> {
    cfg.palettes
        .iter()
        .enumerate()
        .map(|(i, palette)| {
            let size = cfg.texture_px.max(16);
            let image = Image::new(
                Extent3d {
                    width: size as u32,
                    height: size as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                core_pixels(palette, size, 0x1000 + i as u32 * 977),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            materials.add(StandardMaterial {
                base_color_texture: Some(images.add(image)),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 1.0,
                reflectance: 0.0,
                depth_bias: 4.0,
                double_sided: true,
                cull_mode: None,
                ..default()
            })
        })
        .collect()
}

/// Square grid of `n` x `n` cells with half-side `half` (world units) around (cx, cz), vertex
/// heights from `y_of`.
pub fn draped_grid(
    centre: Vec2,
    half: f32,
    n: usize,
    y_of: &dyn Fn(f32, f32) -> f32,
    yaw: f32,
) -> Mesh {
    let (sin, cos) = yaw.sin_cos();
    let mut positions = Vec::with_capacity((n + 1) * (n + 1));
    let mut uvs = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let (u, v) = (i as f32 / n as f32, j as f32 / n as f32);
            let (lx, lz) = ((u - 0.5) * 2.0 * half, (v - 0.5) * 2.0 * half);
            let (x, z) = (centre.x + lx * cos - lz * sin, centre.y + lx * sin + lz * cos);
            positions.push([x, y_of(x, z) + LIFT, z]);
            uvs.push([u, v]);
        }
    }
    let mut indices = Vec::with_capacity(n * n * 6);
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u32;
            let b = a + 1;
            let c = a + (n + 1) as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    let count = positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0_f32, 1.0, 0.0]; count]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Streams the decals around the rig: creates those of the hamlets within reach, removes the far
/// ones, rebuilds all when the relief changes.
#[allow(clippy::too_many_arguments)]
pub fn sync_cores(
    mut commands: Commands,
    loaded: Option<Res<LoadedCountryside>>,
    rig: Option<Res<CampaignRig>>,
    relief: Option<Res<super::ReliefVersion>>,
    ground: GroundHeight,
    mut cores: ResMut<HamletCores>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let (Some(loaded), Some(rig)) = (loaded, rig) else {
        return;
    };
    let cfg = &loaded.config;
    let core = &cfg.hamlet_core;
    if core.radius_m <= 0.0 || core.palettes.is_empty() || !ground.is_ready() {
        return;
    }
    if relief.as_deref().copied() != cores.relief_stamp {
        for (_, e) in cores.live.drain() {
            commands.entity(e).despawn();
        }
        cores.relief_stamp = relief.as_deref().copied();
    }
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    let reach = rig.distance * 1.6 + 2.0;
    if rig.distance > core.max_distance {
        for (_, e) in cores.live.drain() {
            commands.entity(e).despawn();
        }
        return;
    }
    if cores.materials.is_empty() {
        cores.materials = build_materials(core, &mut images, &mut materials);
    }
    let terrain = &loaded.terrain;
    let side = cfg.render.cell_px;
    let (lo, hi) = (
        cell_of(focus - Vec2::splat(reach), side),
        cell_of(focus + Vec2::splat(reach), side),
    );
    let half = core.radius_m / cfg.meters_per_px;
    let mut wanted: Vec<(Vec2, (i32, i32))> = Vec::new();
    for cy in lo.y..=hi.y {
        for cx in lo.x..=hi.x {
            for village in terrain.villages_in(IVec2::new(cx, cy)) {
                if !core.kinds.contains(&village.kind) || village.px.distance(focus) > reach {
                    continue;
                }
                let key = (village.px.x.round() as i32, village.px.y.round() as i32);
                wanted.push((village.px, key));
            }
        }
    }
    let keep: std::collections::HashSet<(i32, i32)> = wanted.iter().map(|(_, k)| *k).collect();
    let stale: Vec<(i32, i32)> = cores
        .live
        .keys()
        .filter(|k| !keep.contains(k))
        .copied()
        .collect();
    for key in stale {
        if let Some(e) = cores.live.remove(&key) {
            commands.entity(e).despawn();
        }
    }
    let mut created = 0;
    for (px, key) in wanted {
        if created >= SPAWN_PER_FRAME {
            break;
        }
        if cores.live.contains_key(&key)
            || !terrain.is_land(px)
            || terrain.in_lake(px)
            || terrain.river_sd(px) < 1.0
            || terrain.splat(px)[2] > core.forest_max
        {
            continue;
        }
        let y_of = |x: f32, z: f32| ground.at(x, z).unwrap_or(0.0);
        let pick = (hash2(key.0, key.1, 0xA5) * cores.materials.len() as f32) as usize;
        let material = cores.materials[pick.min(cores.materials.len() - 1)].clone();
        let yaw = hash2(key.0, key.1, 0x3B) * std::f32::consts::TAU;
        let mesh = draped_grid(px, half, core.grid.max(2), &y_of, yaw);
        let entity = commands
            .spawn((
                Name::new("HamletCore"),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                NotShadowCaster,
                Transform::IDENTITY,
            ))
            .id();
        cores.live.insert(key, entity);
        created += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> CorePalette {
        CorePalette {
            id: "t".into(),
            earth: [0.5, 0.4, 0.3],
            garden: [0.4, 0.5, 0.2],
            orchard: [0.3, 0.4, 0.2],
            roof: [0.65, 0.3, 0.18],
        }
    }

    #[test]
    fn decal_is_transparent_at_the_corner_and_opaque_in_the_core() {
        let px = core_pixels(&palette(), 64, 1);
        let alpha = |x: usize, y: usize| px[(y * 64 + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0);
        assert!(alpha(32, 32) > 100);
    }

    #[test]
    fn decal_has_warm_roof_pixels() {
        let px = core_pixels(&palette(), 128, 1);
        let roofs = px
            .chunks(4)
            .filter(|p| p[3] == 255 && p[0] > 130 && p[1] < 100)
            .count();
        assert!(roofs > 20, "red roofs visible from above, got {roofs}");
    }

    #[test]
    fn draped_grid_follows_the_ground() {
        let mesh = draped_grid(Vec2::ZERO, 1.0, 4, &|x, _| x, 0.0);
        let ys: Vec<f32> = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap()
            .iter()
            .map(|p| p[1])
            .collect();
        assert_eq!(ys.len(), 25);
        assert!(ys.iter().cloned().fold(f32::MIN, f32::max) > 0.9);
    }

    /// Inventory (ADR 0824): no built house or garden-tree mesh in the town layer, no potager prop.
    #[test]
    fn no_code_built_habitat_geometry() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/campaign");
        assert!(
            !root.join("towns").join(["shap", "es.rs"].concat()).exists(),
            "towns/shapes.rs must stay removed"
        );
        let banned = [
            ["trees", "_mesh"].concat(),
            ["house_block", "_mesh"].concat(),
            ["garden", "_trees_mesh"].concat(),
            ["hamlet", "_garden"].concat(),
        ];
        for dir in ["towns", "countryside"] {
            for entry in std::fs::read_dir(root.join(dir)).unwrap().flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    for token in &banned {
                        assert!(!text.contains(token.as_str()), "{path:?} contains {token}");
                    }
                }
            }
        }
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/map/map_countryside.json");
        let text = std::fs::read_to_string(data).unwrap();
        assert!(!text.contains(&banned[3]), "potager prop removed from the data");
    }
}
