//! Fresh water and wetlands of the campaign map (lot campwater2). Port of
//! `game/scripts/map/freshwater_layer.gd` + `freshwater_surface.gdshader` +
//! `freshwater_pool.gdshader` (rendering only).
//!
//! * salt pans: grid of rectangles (pale dykes between them) in every ellipse of `salt_pans`;
//! * torrents: foam ribbons on steep river stretches, flow animated by the shader;
//! * waterfalls and fords: the glb of `art/dn_manifest.json` (ids of `map_freshwater.json`
//!   `models`), a flat strip for a missing ford model;
//! * ponds and lagoons: discs seeded in the `wetlands` ellipses, in two block levels around the
//!   camera focus (`patch`: big blobs magnified with the distance, `near`: real scale), one merged
//!   mesh per block instead of a `MultiMesh`.
//!
//! Draping follows `water/drape.rs`: vertices carry metres, `fresh.wesl` lifts them on the
//! exaggerated relief. Departures from Godot: the reed clumps (`freshwater_reeds.gdshader`, cards)
//! are counted but not drawn (vegetation lot); no exclusion circles around the 1:1 towns; the
//! substitute waterfall column is not ported (the glb exists); the pseudo-random draws are not those
//! of Godot's `RandomNumberGenerator`; the glacier line (`_apply_glacier`) belongs to the terrain
//! lot (`coastterr`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::gltf::GltfAssetLabel;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::{Shader, ShaderRef};
use bevy::state::state_scoped::DespawnOnExit;
use bevy::world_serialization::WorldAssetRoot;
use serde_json::{Value, json};

use super::RebuildWater;
use super::drape::{Drape, sync_drape};
use super::height::{GroundHeight as WaterGround, HeightSampler};
use super::materials::linear;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::landmask::{LandMask, decode};
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::markers::placement::stable_hash;
use crate::campaign::terrain::relief_gpu::ReliefParams;
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::GroundHeight;
use crate::paths;

const FRESH_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0c01");

/// World units are map pixels; one map pixel is about 719 m (`METERS_PER_UNIT` of Godot).
const METERS_PER_UNIT: f32 = 719.0;
const LEVEL_PATCH: usize = 0;
const LEVEL_NEAR: usize = 1;
const POND_SIDES: usize = 16;

/// Modes of `fresh.wesl`.
const MODE_SALT: f32 = 0.0;
const MODE_FOAM: f32 = 1.0;
const MODE_FORD: f32 = 2.0;
const MODE_POND: f32 = 3.0;

// --- Material ------------------------------------------------------------------------------------

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct FreshParams {
    pub foam_color: Vec4,
    pub sky_color: Vec4,
    /// x: mode, y: fade start, z: fade end, w: bias factor.
    pub a: Vec4,
    /// x: flow speed, y: pond scale, z: pond alpha, w: pond sky reflection.
    pub b: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FreshMaterial {
    #[uniform(0)]
    pub params: FreshParams,
    #[uniform(1)]
    pub relief: ReliefParams,
    #[texture(2, sample_type = "float", filterable = false)]
    pub relief_floor: Handle<Image>,
}

impl Drape for FreshMaterial {
    fn set_relief(&mut self, params: ReliefParams, floor: &Handle<Image>) {
        self.relief = params;
        self.relief_floor = floor.clone();
    }
    fn relief(&self) -> (ReliefParams, &Handle<Image>) {
        (self.relief, &self.relief_floor)
    }
}

impl FreshMaterial {
    fn new(mode: f32, render: &Render) -> Self {
        Self {
            params: FreshParams {
                foam_color: linear([0.88, 0.92, 0.94], 1.0),
                sky_color: linear([0.30, 0.42, 0.48], 1.0),
                a: Vec4::new(mode, render.fade_start, render.fade_end, 0.004),
                b: Vec4::new(0.6, 1.0, 1.0, 0.25),
            },
            relief: ReliefParams::default(),
            relief_floor: Handle::default(),
        }
    }
}

impl Material for FreshMaterial {
    fn vertex_shader() -> ShaderRef {
        FRESH_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        FRESH_SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn depth_bias(&self) -> f32 {
        // Godot `render_priority = -1`: drawn before the other transparent surfaces.
        -1.0
    }
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

// --- Data ----------------------------------------------------------------------------------------

/// `map_freshwater.json` -> `render`.
#[derive(Debug, Clone)]
pub struct Render {
    pub fade_start: f32,
    pub fade_end: f32,
    pub patch_max_distance: f32,
    pub near_max_distance: f32,
    pub patch_block_units: f32,
    pub patch_spacing_units: f32,
    pub near_block_units: f32,
    pub near_spacing_units: f32,
    pub max_blocks_per_frame: usize,
    pub max_cached_blocks: usize,
    pub max_instances: usize,
    pub scale_near_distance: f32,
    pub scale_far_distance: f32,
    pub far_scale: f32,
    pub model_magnify: f32,
    pub pool_far_scale: f32,
    pub seed: u64,
}

impl Render {
    fn parse(value: &Value) -> Render {
        let f = |key: &str, default: f64| value.get(key).and_then(Value::as_f64).unwrap_or(default);
        Render {
            fade_start: f("fade_start", 950.0) as f32,
            fade_end: f("fade_end", 1200.0) as f32,
            patch_max_distance: f("patch_max_distance", 120.0) as f32,
            near_max_distance: f("near_max_distance", 4.0) as f32,
            patch_block_units: f("patch_block_units", 16.0) as f32,
            patch_spacing_units: f("patch_spacing_units", 0.8) as f32,
            near_block_units: f("near_block_units", 16.0) as f32,
            near_spacing_units: f("near_spacing_units", 0.8) as f32,
            max_blocks_per_frame: f("max_blocks_per_frame", 2.0) as usize,
            max_cached_blocks: f("max_cached_blocks", 220.0) as usize,
            max_instances: f("max_instances", 60000.0) as usize,
            scale_near_distance: f("scale_near_distance", 3.0) as f32,
            scale_far_distance: f("scale_far_distance", 60.0) as f32,
            far_scale: f("far_scale", 36.0) as f32,
            model_magnify: f("model_magnify", 6.0) as f32,
            pool_far_scale: f("pool_far_scale", 2.0) as f32,
            seed: f("seed", 1.0) as u64,
        }
    }

    pub fn block_units(&self, level: usize) -> f32 {
        if level == LEVEL_NEAR {
            self.near_block_units
        } else {
            self.patch_block_units
        }
    }

    fn spacing(&self, level: usize) -> f32 {
        if level == LEVEL_NEAR {
            self.near_spacing_units
        } else {
            self.patch_spacing_units
        }
    }

    /// Magnification of the patch-level blobs at camera distance `d` (1 up close).
    pub fn scale_for(&self, d: f32) -> f32 {
        let near = self.scale_near_distance;
        let far = self.scale_far_distance.max(near * 1.01);
        let t = ((d.max(1e-3) / near).ln() / (far / near).ln()).clamp(0.0, 1.0);
        1.0 + (self.far_scale - 1.0) * smooth(t)
    }
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge0 == edge1 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    smooth(((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0))
}

/// Reed family of a wetland (`reeds.kinds`).
#[derive(Debug, Clone, Copy)]
pub struct ReedKind {
    pub cover: f32,
    pub pool_share: f32,
    pub tint: [f32; 3],
}

/// One ellipse of `freshwater_px.json` (wetlands and salt pans).
#[derive(Debug, Clone)]
pub struct Ellipse {
    pub id: String,
    pub kind: String,
    pub tint: String,
    pub density: f32,
    pub lagoon: bool,
    pub center: Vec2,
    pub radii: Vec2,
    pub angle_deg: f32,
}

impl Ellipse {
    fn parse(value: &Value) -> Option<Ellipse> {
        let pair = |key: &str| -> Option<Vec2> {
            let a = value.get(key)?.as_array()?;
            Some(Vec2::new(
                a.first()?.as_f64()? as f32,
                a.get(1)?.as_f64()? as f32,
            ))
        };
        let text = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        };
        Some(Ellipse {
            id: text("id"),
            kind: text("kind"),
            tint: value
                .get("tint")
                .and_then(Value::as_str)
                .unwrap_or("grey")
                .to_owned(),
            density: value.get("density").and_then(Value::as_f64).unwrap_or(0.5) as f32,
            lagoon: value
                .get("lagoon")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            center: pair("center")?,
            radii: pair("radii_px")?,
            angle_deg: value
                .get("angle_deg")
                .and_then(Value::as_f64)
                .unwrap_or(0.0) as f32,
        })
    }

    /// Normalised radius (0 at the centre, 1 on the edge) of `p` (`FreshwaterLayer.ellipse_radius`).
    pub fn radius_at(&self, p: Vec2) -> f32 {
        let angle = self.angle_deg.to_radians();
        let d = p - self.center;
        let (dx, dy_north) = (d.x, -d.y);
        let u = dx * angle.cos() + dy_north * angle.sin();
        let v = -dx * angle.sin() + dy_north * angle.cos();
        ((u / self.radii.x).powi(2) + (v / self.radii.y).powi(2)).sqrt()
    }
}

/// A waterfall or ford site.
#[derive(Debug, Clone)]
pub struct Site {
    pub id: String,
    pub px: Vec2,
    pub extra_m: f32,
}

/// One torrent stretch.
#[derive(Debug, Clone)]
pub struct Torrent {
    pub slope: f32,
    pub points: Vec<Vec2>,
}

/// Everything read from `map_freshwater.json` and `freshwater_px.json`.
#[derive(Debug, Clone, Default)]
pub struct FreshData {
    pub render: Option<Render>,
    pub kinds: HashMap<String, ReedKind>,
    pub pool_diameter_m: f32,
    pub foam_width: f32,
    pub models: HashMap<String, String>,
    pub wetlands: Vec<Ellipse>,
    pub salt_pans: Vec<Ellipse>,
    pub waterfalls: Vec<Site>,
    pub fords: Vec<Site>,
    pub torrents: Vec<Torrent>,
}

impl FreshData {
    pub fn parse(config: &Value, sites: &Value) -> FreshData {
        let list = |key: &str| -> Vec<Value> {
            sites
                .get(key)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let reeds = config.get("reeds").cloned().unwrap_or(Value::Null);
        let mut kinds = HashMap::new();
        if let Some(map) = reeds.get("kinds").and_then(Value::as_object) {
            for (name, kind) in map {
                let tint = kind
                    .get("tint")
                    .and_then(Value::as_array)
                    .map(|t| {
                        std::array::from_fn(|i| {
                            t.get(i).and_then(Value::as_f64).unwrap_or(0.5) as f32
                        })
                    })
                    .unwrap_or([0.55, 0.55, 0.35]);
                kinds.insert(
                    name.clone(),
                    ReedKind {
                        cover: kind.get("cover").and_then(Value::as_f64).unwrap_or(0.3) as f32,
                        pool_share: kind
                            .get("pool_share")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.1) as f32,
                        tint,
                    },
                );
            }
        }
        let site = |value: &Value, extra: &str| -> Option<Site> {
            let px = value.get("px")?.as_array()?;
            Some(Site {
                id: value.get("id")?.as_str()?.to_owned(),
                px: Vec2::new(px.first()?.as_f64()? as f32, px.get(1)?.as_f64()? as f32),
                extra_m: value.get(extra).and_then(Value::as_f64).unwrap_or(0.0) as f32,
            })
        };
        FreshData {
            render: config.get("render").map(Render::parse),
            kinds,
            pool_diameter_m: reeds
                .get("pool_diameter_m")
                .and_then(Value::as_f64)
                .unwrap_or(60.0) as f32,
            foam_width: config
                .pointer("/torrent/foam_width_units")
                .and_then(Value::as_f64)
                .unwrap_or(0.18) as f32,
            models: config
                .get("models")
                .and_then(Value::as_object)
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                        .collect()
                })
                .unwrap_or_default(),
            wetlands: list("wetlands").iter().filter_map(Ellipse::parse).collect(),
            salt_pans: list("salt_pans")
                .iter()
                .filter_map(Ellipse::parse)
                .collect(),
            waterfalls: list("waterfalls")
                .iter()
                .filter_map(|v| site(v, "height_m"))
                .collect(),
            fords: list("fords")
                .iter()
                .filter_map(|v| site(v, "width_m"))
                .collect(),
            torrents: list("torrents")
                .iter()
                .filter_map(|v| {
                    let points: Vec<Vec2> = v
                        .get("points")?
                        .as_array()?
                        .iter()
                        .filter_map(|p| {
                            Some(Vec2::new(
                                p.get(0)?.as_f64()? as f32,
                                p.get(1)?.as_f64()? as f32,
                            ))
                        })
                        .collect();
                    Some(Torrent {
                        slope: v.get("slope").and_then(Value::as_f64).unwrap_or(0.05) as f32,
                        points,
                    })
                })
                .collect(),
        }
    }

    fn load(root: &Path) -> Option<FreshData> {
        let data = paths::data_dir(root);
        let read = |rel: &str| -> Option<Value> {
            serde_json::from_slice(&std::fs::read(data.join(rel)).ok()?).ok()
        };
        let parsed = FreshData::parse(
            &read("map/map_freshwater.json")?,
            &read("map/freshwater_px.json")?,
        );
        parsed.render.is_some().then_some(parsed)
    }

    /// Path (relative to the asset root `game/assets`) of the glb of `models[key]`, when the
    /// catalogue lists it and the file exists (LOD 0).
    fn model_path(&self, root: &Path, manifest: &Value, key: &str) -> Option<String> {
        let id = self.models.get(key)?;
        let file = manifest
            .pointer(&format!("/assets/{id}/files/0"))?
            .as_str()?;
        let rel = format!("models/{file}");
        root.join("game/assets").join(&rel).is_file().then_some(rel)
    }
}

// --- Seeding (pure) ------------------------------------------------------------------------------

/// SplitMix64: small deterministic generator for the seeding.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }
}

/// One pond or lagoon disc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pool {
    pub pos: Vec2,
    pub diameter_m: f32,
    pub color: [f32; 3],
    pub seed: f32,
}

/// Seeding of one block: the ponds and the number of reed clumps (not drawn, see the module doc).
pub fn seed_block(
    data: &FreshData,
    render: &Render,
    land: &LandMask,
    level: usize,
    key: IVec2,
) -> (Vec<Pool>, usize) {
    let size = render.block_units(level);
    let origin = Vec2::new(key.x as f32 * size, key.y as f32 * size);
    let in_rect = |p: Vec2, grow: f32| {
        p.x >= origin.x - grow
            && p.x <= origin.x + size + grow
            && p.y >= origin.y - grow
            && p.y <= origin.y + size + grow
    };
    let local: Vec<&Ellipse> = data
        .wetlands
        .iter()
        .filter(|s| in_rect(s.center, s.radii.max_element()))
        .collect();
    let mut pools = Vec::new();
    let mut reeds = 0;
    if local.is_empty() {
        return (pools, reeds);
    }
    let mixed = (key.x as i64).wrapping_mul(73_856_093)
        ^ (key.y as i64).wrapping_mul(19_349_663)
        ^ (level as i64).wrapping_mul(83_492_791);
    let mut rng = Rng((mixed as u64).wrapping_add(render.seed));
    let spacing = render.spacing(level);
    let cells = (size / spacing).ceil() as i32;
    for ix in 0..cells {
        for iz in 0..cells {
            let p = origin
                + Vec2::new(
                    (ix as f32 + rng.next()) * spacing,
                    (iz as f32 + rng.next()) * spacing,
                );
            if !in_rect(p, 0.0) || p.x >= origin.x + size || p.y >= origin.y + size {
                continue;
            }
            let mut best = 0.0;
            let mut pick: Option<(&Ellipse, f32)> = None;
            for site in &local {
                let r = site.radius_at(p);
                if r >= 1.0 {
                    continue;
                }
                let cover = data.kinds.get(&site.kind).map_or(0.3, |k| k.cover);
                let weight = cover * site.density * smoothstep(1.0, 0.6, r);
                if weight > best {
                    best = weight;
                    pick = Some((site, r));
                }
            }
            let roll = rng.next();
            let Some((site, pick_r)) = pick else { continue };
            if roll > best || !land.is_land(p.x, p.y) {
                continue;
            }
            let kind = data.kinds.get(&site.kind);
            let mut pool_share = kind.map_or(0.1, |k| k.pool_share);
            if site.lagoon {
                pool_share = if pick_r < 0.62 { 0.4 } else { 0.0 };
            }
            if rng.next() < pool_share {
                let diameter = data.pool_diameter_m
                    * rng.range(0.6, 1.8)
                    * if site.lagoon { 3.0 } else { 1.0 };
                let color = if site.lagoon {
                    [0.30, 0.46, 0.46]
                } else {
                    [0.10, 0.17, 0.17]
                };
                pools.push(Pool {
                    pos: p,
                    diameter_m: diameter,
                    color,
                    seed: rng.next(),
                });
            } else if !site.lagoon || pick_r > 0.5 {
                // Clump size, jitter, phase: drawn to keep the sequence of the Godot seeding.
                rng.next();
                rng.next();
                rng.next();
                reeds += 1;
            }
        }
    }
    (pools, reeds)
}

// --- Meshes --------------------------------------------------------------------------------------

#[derive(Default)]
struct MeshParts {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshParts {
    fn push(&mut self, position: [f32; 3], offset: [f32; 2], uv: [f32; 2], color: [f32; 4]) {
        self.positions.push(position);
        self.normals.push([offset[0], 0.0, offset[1]]);
        self.uvs.push(uv);
        self.colors.push(color);
    }

    fn into_mesh(self) -> Option<Mesh> {
        if self.positions.is_empty() {
            return None;
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        Some(mesh)
    }
}

fn seat_m(ground: &dyn HeightSampler, p: Vec2) -> f32 {
    ground.seat_height_m(p.x, p.y).max(0.0)
}

/// Salt pans: grid of rectangles with pale dykes between them inside every ellipse.
pub fn salt_pan_mesh(
    data: &FreshData,
    render: &Render,
    land: &LandMask,
    ground: &dyn HeightSampler,
) -> (Option<Mesh>, usize) {
    let mut parts = MeshParts::default();
    let mut rng = Rng(render.seed + 17);
    let mut cells_drawn = 0;
    for pan in &data.salt_pans {
        let angle = pan.angle_deg.to_radians();
        let cell = (pan.radii.min_element() / 5.0).max(0.12);
        let rose = pan.tint == "rose";
        let nx = (pan.radii.x * 2.0 / cell).ceil() as i32;
        let nz = (pan.radii.y * 2.0 / cell).ceil() as i32;
        let (basis, perp) = (
            Vec2::new(angle.cos(), angle.sin()),
            Vec2::new(-angle.sin(), angle.cos()),
        );
        for ix in 0..nx {
            for iz in 0..nz {
                let local = Vec2::new(
                    -pan.radii.x + (ix as f32 + 0.5) * cell,
                    -pan.radii.y + (iz as f32 + 0.5) * cell,
                );
                if (local / pan.radii).length() > 1.0 {
                    continue;
                }
                let world = pan.center
                    + Vec2::new(
                        local.x * angle.cos() - local.y * angle.sin(),
                        local.x * angle.sin() + local.y * angle.cos(),
                    );
                if !land.is_land(world.x, world.y) {
                    continue;
                }
                let half = cell * 0.44;
                let tone = rng.next();
                let color = if rose && tone < 0.7 {
                    lerp4([0.74, 0.46, 0.50, 0.9], [0.80, 0.62, 0.60, 0.9], tone)
                } else {
                    lerp4([0.55, 0.62, 0.62, 0.9], [0.86, 0.86, 0.82, 0.9], tone)
                };
                let base = parts.positions.len() as u32;
                let y = seat_m(ground, world);
                for corner in [
                    Vec2::new(-1.0, -1.0),
                    Vec2::new(1.0, -1.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(-1.0, 1.0),
                ] {
                    let p = world + basis * corner.x * half + perp * corner.y * half;
                    parts.push(
                        [p.x, y, p.y],
                        [0.0, 0.0],
                        [corner.x * 0.5 + 0.5, corner.y * 0.5 + 0.5],
                        color,
                    );
                }
                parts.indices.extend_from_slice(&[
                    base,
                    base + 1,
                    base + 2,
                    base,
                    base + 2,
                    base + 3,
                ]);
                cells_drawn += 1;
            }
        }
    }
    (parts.into_mesh(), cells_drawn)
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

/// Foam ribbons on the steep river stretches, widened with the slope.
pub fn torrent_mesh(data: &FreshData, ground: &dyn HeightSampler) -> Option<Mesh> {
    let mut parts = MeshParts::default();
    for run in &data.torrents {
        let n = run.points.len();
        if n < 2 {
            continue;
        }
        let half = data.foam_width * 0.5 * (0.7 + run.slope * 4.0).clamp(0.7, 1.6);
        let mut along = 0.0;
        let mut previous: Option<u32> = None;
        for i in 0..n {
            let p = run.points[i];
            let q = run.points[(i + 1).min(n - 1)];
            let r = run.points[i.saturating_sub(1)];
            let dir = (q - r).normalize_or_zero();
            let perp = Vec2::new(-dir.y, dir.x) * half;
            if i > 0 {
                along += p.distance(r) / (half * 4.0);
            }
            let alpha = (0.55 + run.slope * 3.0).clamp(0.5, 1.0);
            let base = parts.positions.len() as u32;
            let y = seat_m(ground, p);
            parts.push(
                [p.x + perp.x, y, p.y + perp.y],
                [0.0, 0.0],
                [along, 0.0],
                [1.0, 1.0, 1.0, alpha],
            );
            parts.push(
                [p.x - perp.x, y, p.y - perp.y],
                [0.0, 0.0],
                [along, 1.0],
                [1.0, 1.0, 1.0, alpha],
            );
            if let Some(prev) = previous {
                parts
                    .indices
                    .extend_from_slice(&[prev, prev + 1, base, prev + 1, base + 1, base]);
            }
            previous = Some(base);
        }
    }
    parts.into_mesh()
}

/// Flat strip standing in for a ford without a glb (`_place_fords`).
fn ford_strip(site: &Site, ground: &dyn HeightSampler) -> Option<Mesh> {
    let half_length = if site.extra_m > 0.0 {
        site.extra_m
    } else {
        1500.0
    } / METERS_PER_UNIT
        * 0.5;
    let half_width = (half_length * 0.08).max(0.03);
    let y = seat_m(ground, site.px);
    let mut parts = MeshParts::default();
    for (dx, dz, u, v) in [
        (-1.0, -1.0, 0.0, 0.0),
        (1.0, -1.0, 1.0, 0.0),
        (1.0, 1.0, 1.0, 1.0),
        (-1.0, 1.0, 0.0, 1.0),
    ] {
        parts.push(
            [site.px.x + dx * half_length, y, site.px.y + dz * half_width],
            [0.0, 0.0],
            [u, v],
            [1.0; 4],
        );
    }
    parts.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    parts.into_mesh()
}

/// One block of ponds: every disc is a 16-sector fan (centre vertex + rim), the corner offset in
/// the normal so the shader can magnify the discs of the `patch` level about their centre.
pub fn pond_mesh(pools: &[Pool], ground: &dyn HeightSampler) -> Option<Mesh> {
    let mut parts = MeshParts::default();
    for pool in pools {
        let radius = 0.5 * pool.diameter_m / METERS_PER_UNIT;
        let y = seat_m(ground, pool.pos);
        let color = [pool.color[0], pool.color[1], pool.color[2], pool.seed];
        let base = parts.positions.len() as u32;
        parts.push([pool.pos.x, y, pool.pos.y], [0.0, 0.0], [0.5, 0.5], color);
        for i in 0..POND_SIDES {
            let angle = std::f32::consts::TAU * i as f32 / POND_SIDES as f32;
            let (s, c) = angle.sin_cos();
            parts.push(
                [pool.pos.x, y, pool.pos.y],
                [c * radius, s * radius],
                [c * 0.5 + 0.5, s * 0.5 + 0.5],
                color,
            );
        }
        for i in 0..POND_SIDES as u32 {
            parts.indices.extend_from_slice(&[
                base,
                base + 1 + i,
                base + 1 + (i + 1) % POND_SIDES as u32,
            ]);
        }
    }
    parts.into_mesh()
}

// --- Plugin --------------------------------------------------------------------------------------

/// Marker of every static entity of the layer (cleared on rebuild).
#[derive(Component)]
struct FreshEntity;

/// A glb standing on the ground (waterfall, ford): re-seated when the relief exaggeration changes.
#[derive(Component)]
struct FreshModel {
    px: Vec2,
    metres: f32,
}

/// A statics mesh hidden beyond `fade_end` (`FreshwaterLayer._static_root.visible`).
#[derive(Component)]
struct FreshStatic;

struct Block {
    entity: Option<Entity>,
    pools: usize,
    last_seen: u64,
}

/// State of the layer, built on entering the campaign.
#[derive(Resource)]
pub struct FreshLayer {
    data: FreshData,
    render: Render,
    land: LandMask,
    ground: Arc<dyn HeightSampler>,
    pond_materials: [Handle<FreshMaterial>; 2],
    blocks: HashMap<(usize, i32, i32), Block>,
    frame: u64,
    total_pools: usize,
    pub stats: FreshStats,
}

/// Counters for the dump.
#[derive(Debug, Clone, Default)]
pub struct FreshStats {
    pub salt_cells: usize,
    pub torrent_runs: usize,
    pub torrent_vertices: usize,
    pub waterfall_models: usize,
    pub waterfall_missing: usize,
    pub ford_models: usize,
    pub ford_strips: usize,
    pub blocks: usize,
    pub pools: usize,
    pub reeds_counted: usize,
    pub seed_ms_max: f32,
    pub patch_alpha: f32,
    pub near_alpha: f32,
    pub patch_scale: f32,
    pub statics_visible: bool,
}

pub struct FreshwaterPlugin;

impl Plugin for FreshwaterPlugin {
    fn build(&self, app: &mut App) {
        app.register_dump(dump_fresh);
        let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() else {
            return;
        };
        let source = super::drape::with_relief(include_str!("shaders/fresh.wesl"));
        if let Err(error) = shaders.insert(
            FRESH_SHADER.id(),
            Shader::from_wesl(source, "water/fresh.wesl"),
        ) {
            error!("water: shader water/fresh.wesl: {error:?}");
        }
        app.add_plugins(MaterialPlugin::<FreshMaterial>::default())
            .add_systems(PostUpdate, sync_drape::<FreshMaterial>)
            .add_systems(OnEnter(AppState::Campaign), build_fresh)
            .add_systems(
                Update,
                (rebuild_fresh, update_ponds, seat_models)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

fn build_fresh(
    mut commands: Commands,
    ground: Res<WaterGround>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FreshMaterial>>,
    assets: Res<AssetServer>,
    old: Query<Entity, With<FreshEntity>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<FreshLayer>();
    let Ok(root) = paths::find_root() else { return };
    let Some(data) = FreshData::load(&root) else {
        info!("freshwater: no data/map/map_freshwater.json or freshwater_px.json");
        return;
    };
    let render = data.render.clone().expect("checked by load");
    let land = std::fs::File::open(paths::data_dir(&root).join("map/land_mask.png"))
        .ok()
        .and_then(decode)
        .unwrap_or_default();
    let manifest: Value = std::fs::read(paths::data_dir(&root).join("art/dn_manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null);
    let sampler = ground.0.clone();
    let mut stats = FreshStats::default();

    let mut spawn_static = |commands: &mut Commands,
                            name: &str,
                            mesh: Mesh,
                            mode: f32,
                            materials: &mut Assets<FreshMaterial>| {
        commands.spawn((
            Name::new(name.to_owned()),
            FreshEntity,
            FreshStatic,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(FreshMaterial::new(mode, &render))),
            Transform::default(),
            NotShadowCaster,
            NoFrustumCulling,
            DespawnOnExit(AppState::Campaign),
        ));
    };
    let (pans, cells) = salt_pan_mesh(&data, &render, &land, sampler.as_ref());
    stats.salt_cells = cells;
    if let Some(mesh) = pans {
        spawn_static(&mut commands, "SaltPans", mesh, MODE_SALT, &mut materials);
    }
    if let Some(mesh) = torrent_mesh(&data, sampler.as_ref()) {
        stats.torrent_runs = data.torrents.iter().filter(|t| t.points.len() >= 2).count();
        stats.torrent_vertices = mesh.count_vertices();
        spawn_static(&mut commands, "Torrents", mesh, MODE_FOAM, &mut materials);
    }
    let magnify = render.model_magnify / METERS_PER_UNIT;
    let place = |commands: &mut Commands, site: &Site, path: String, prefix: &str, scale: f32| {
        let handle = assets.load(GltfAssetLabel::Scene(0).from_asset(path));
        let yaw = (stable_hash(&site.id) % 628) as f32 / 100.0;
        commands.spawn((
            Name::new(format!("{prefix}_{}", site.id)),
            FreshEntity,
            FreshStatic,
            FreshModel {
                px: site.px,
                metres: seat_m(sampler.as_ref(), site.px),
            },
            WorldAssetRoot(handle),
            Transform::from_xyz(
                site.px.x,
                seat_m(sampler.as_ref(), site.px) * super::height::HEIGHT_SCALE,
                site.px.y,
            )
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(scale)),
            DespawnOnExit(AppState::Campaign),
        ));
    };
    match data.model_path(&root, &manifest, "waterfall") {
        Some(path) => {
            for site in &data.waterfalls {
                place(&mut commands, site, path.clone(), "Waterfall", magnify);
                stats.waterfall_models += 1;
            }
        }
        None => stats.waterfall_missing = data.waterfalls.len(),
    }
    match data.model_path(&root, &manifest, "ford") {
        Some(path) => {
            for site in &data.fords {
                place(&mut commands, site, path.clone(), "Ford", magnify);
                stats.ford_models += 1;
            }
        }
        None => {
            for site in &data.fords {
                if let Some(mesh) = ford_strip(site, sampler.as_ref()) {
                    spawn_static(
                        &mut commands,
                        &format!("Ford_{}", site.id),
                        mesh,
                        MODE_FORD,
                        &mut materials,
                    );
                    stats.ford_strips += 1;
                }
            }
        }
    }
    let pond_materials =
        [LEVEL_PATCH, LEVEL_NEAR].map(|_| materials.add(FreshMaterial::new(MODE_POND, &render)));
    commands.insert_resource(FreshLayer {
        data,
        render,
        land,
        ground: sampler,
        pond_materials,
        blocks: HashMap::new(),
        frame: 0,
        total_pools: 0,
        stats,
    });
}

/// Rebuilds the layer after [`RebuildWater`] (the terrain replaced the ground sampler).
fn rebuild_fresh(
    mut requests: MessageReader<RebuildWater>,
    mut commands: Commands,
    layer: Option<Res<FreshLayer>>,
    blocks: Query<Entity, With<PondBlock>>,
) {
    if requests.read().count() == 0 {
        return;
    }
    for entity in &blocks {
        commands.entity(entity).despawn();
    }
    if layer.is_some() {
        commands.run_system_cached(build_fresh);
    }
}

#[derive(Component)]
struct PondBlock;

/// Streams the pond blocks around the rig focus and fades them with the distance
/// (`FreshwaterLayer.update_view`).
fn update_ponds(
    mut commands: Commands,
    layer: Option<ResMut<FreshLayer>>,
    rig: Option<Res<CampaignRig>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FreshMaterial>>,
    mut visibility: Query<&mut Visibility, With<FreshStatic>>,
    mut blocks_visibility: Query<&mut Visibility, (With<PondBlock>, Without<FreshStatic>)>,
) {
    let (Some(mut layer), Some(rig)) = (layer, rig) else {
        return;
    };
    let layer = &mut *layer;
    layer.frame += 1;
    let render = layer.render.clone();
    let distance = rig.distance;
    let at = Vec2::new(rig.focus.x, rig.focus.z);
    let statics_visible = distance < render.fade_end;
    layer.stats.statics_visible = statics_visible;
    for mut v in &mut visibility {
        *v = if statics_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let patch_max = render.patch_max_distance;
    let near_max = render.near_max_distance;
    let far = distance >= patch_max;
    let patch_alpha = smoothstep(near_max * 0.8, near_max * 1.4, distance)
        * (1.0 - smoothstep(patch_max * 0.75, patch_max, distance));
    let near_alpha = 1.0 - smoothstep(near_max, near_max * 1.7, distance);
    let scale = render.scale_for(distance);
    layer.stats.patch_alpha = patch_alpha;
    layer.stats.near_alpha = near_alpha;
    layer.stats.patch_scale = scale.min(render.pool_far_scale);
    for (level, alpha, pool_scale) in [
        (LEVEL_PATCH, patch_alpha, scale.min(render.pool_far_scale)),
        (LEVEL_NEAR, near_alpha, 1.0),
    ] {
        if let Some(mut material) = materials.get_mut(&layer.pond_materials[level]) {
            material.params.b.y = pool_scale;
            material.params.b.z = alpha;
        }
    }
    let mut wanted: HashMap<(usize, i32, i32), ()> = HashMap::new();
    if !far {
        let mut created = 0;
        for (level, alpha) in [(LEVEL_PATCH, patch_alpha), (LEVEL_NEAR, near_alpha)] {
            if alpha <= 0.01 {
                continue;
            }
            let size = render.block_units(level);
            let radius = if level == LEVEL_PATCH {
                (distance * 1.8).clamp(20.0, 140.0)
            } else {
                (distance * 1.6).clamp(2.5, 5.0)
            };
            let lo = ((at - Vec2::splat(radius)) / size).floor().as_ivec2();
            let hi = ((at + Vec2::splat(radius)) / size).floor().as_ivec2();
            let mut missing: Vec<(f32, IVec2)> = Vec::new();
            for ty in lo.y..=hi.y {
                for tx in lo.x..=hi.x {
                    let origin = Vec2::new(tx as f32, ty as f32) * size;
                    let nearest = at.clamp(origin, origin + Vec2::splat(size));
                    let d = nearest.distance(at);
                    if d > radius {
                        continue;
                    }
                    wanted.insert((level, tx, ty), ());
                    if !layer.blocks.contains_key(&(level, tx, ty)) {
                        missing.push((d, IVec2::new(tx, ty)));
                    }
                }
            }
            missing.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (_, key) in missing {
                if created >= render.max_blocks_per_frame
                    || layer.total_pools > render.max_instances
                {
                    break;
                }
                let started = std::time::Instant::now();
                let (pools, reeds) = seed_block(&layer.data, &render, &layer.land, level, key);
                let entity = pond_mesh(&pools, layer.ground.as_ref()).map(|mesh| {
                    commands
                        .spawn((
                            Name::new(format!("Ponds_{level}_{}_{}", key.x, key.y)),
                            FreshEntity,
                            PondBlock,
                            Mesh3d(meshes.add(mesh)),
                            MeshMaterial3d(layer.pond_materials[level].clone()),
                            Transform::default(),
                            NotShadowCaster,
                            NoFrustumCulling,
                            DespawnOnExit(AppState::Campaign),
                        ))
                        .id()
                });
                layer.total_pools += pools.len();
                layer.stats.reeds_counted += reeds;
                layer.stats.seed_ms_max = layer
                    .stats
                    .seed_ms_max
                    .max(started.elapsed().as_secs_f32() * 1000.0);
                layer.blocks.insert(
                    (level, key.x, key.y),
                    Block {
                        entity,
                        pools: pools.len(),
                        last_seen: layer.frame,
                    },
                );
                created += 1;
            }
        }
    }
    let frame = layer.frame;
    for (key, block) in layer.blocks.iter_mut() {
        let shown = wanted.contains_key(key);
        if shown {
            block.last_seen = frame;
        }
        if let Some(entity) = block.entity
            && let Ok(mut v) = blocks_visibility.get_mut(entity)
        {
            *v = if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    // Evict the least recently seen blocks beyond the cache limit.
    if layer.blocks.len() > render.max_cached_blocks {
        let mut keys: Vec<((usize, i32, i32), u64)> = layer
            .blocks
            .iter()
            .map(|(k, b)| (*k, b.last_seen))
            .collect();
        keys.sort_by_key(|(_, seen)| *seen);
        let excess = layer.blocks.len() - render.max_cached_blocks;
        for (key, _) in keys.into_iter().take(excess) {
            if let Some(block) = layer.blocks.remove(&key) {
                layer.total_pools = layer.total_pools.saturating_sub(block.pools);
                if let Some(entity) = block.entity {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
    layer.stats.blocks = layer.blocks.len();
    layer.stats.pools = layer.blocks.values().map(|b| b.pools).sum();
}

/// Seats the glb models on the displayed ground, again whenever the relief exaggeration changes.
fn seat_models(
    ground: GroundHeight,
    relief: Option<Res<ReliefState>>,
    mut models: Query<(&FreshModel, &mut Transform)>,
    added: Query<(), Added<FreshModel>>,
    mut done: Local<bool>,
) {
    let changed = relief.as_ref().is_some_and(|r| r.is_changed()) || !added.is_empty();
    if *done && !changed {
        return;
    }
    if !ground.is_ready() {
        return;
    }
    for (model, mut transform) in &mut models {
        transform.translation.y = ground
            .at(model.px.x, model.px.y)
            .unwrap_or(model.metres * super::height::HEIGHT_SCALE);
    }
    *done = true;
}

fn dump_fresh(world: &World) -> (String, Value) {
    let value = match world.get_resource::<FreshLayer>() {
        None => json!({"built": false}),
        Some(layer) => {
            let s = &layer.stats;
            json!({
                "built": true,
                "data": {"wetlands": layer.data.wetlands.len(), "salt_pans": layer.data.salt_pans.len(),
                         "waterfalls": layer.data.waterfalls.len(), "fords": layer.data.fords.len(),
                         "torrents": layer.data.torrents.len()},
                "land_mask": layer.land.is_loaded(),
                "statics": {"salt_cells": s.salt_cells, "torrent_runs": s.torrent_runs,
                            "torrent_vertices": s.torrent_vertices, "waterfall_models": s.waterfall_models,
                            "waterfall_missing_model": s.waterfall_missing, "ford_models": s.ford_models,
                            "ford_strips": s.ford_strips, "visible": s.statics_visible},
                "ponds": {"blocks": s.blocks, "pools": s.pools, "reeds_counted_not_drawn": s.reeds_counted,
                          "patch_alpha": s.patch_alpha, "near_alpha": s.near_alpha,
                          "patch_scale": s.patch_scale, "seed_ms_max": s.seed_ms_max},
            })
        }
    };
    ("campaign_freshwater".into(), value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::water::height::FlatGround;

    fn sample_data() -> FreshData {
        let config = json!({
            "render": {"seed": 4417, "near_block_units": 2.0, "near_spacing_units": 0.07},
            "reeds": {"pool_diameter_m": 60.0, "kinds": {
                "ponds": {"cover": 0.45, "pool_share": 0.2, "tint": [0.36, 0.44, 0.16]}}},
            "torrent": {"foam_width_units": 0.18},
        });
        let sites = json!({
            "wetlands": [{"id": "dombes", "kind": "ponds", "density": 0.9, "lagoon": false,
                          "center": [10.0, 10.0], "radii_px": [6.0, 5.0], "angle_deg": 15.0}],
            "salt_pans": [{"id": "pan", "tint": "rose", "center": [30.0, 30.0], "radii_px": [2.0, 1.5], "angle_deg": 0.0}],
            "torrents": [{"river": "R", "slope": 0.1, "points": [[0.0, 0.0], [1.0, 0.0], [2.0, 0.5]]}],
            "fords": [{"id": "gue", "px": [5.0, 5.0], "width_m": 1500}],
            "waterfalls": [{"id": "chute", "px": [6.0, 6.0], "height_m": 20}],
        });
        FreshData::parse(&config, &sites)
    }

    fn all_land() -> LandMask {
        LandMask::new(64, 64, vec![255; 64 * 64])
    }

    #[test]
    fn parses_the_data_files() {
        let data = sample_data();
        assert_eq!(data.wetlands.len(), 1);
        assert_eq!(data.salt_pans.len(), 1);
        assert_eq!(data.torrents[0].points.len(), 3);
        assert_eq!(data.fords[0].extra_m, 1500.0);
        assert_eq!(data.render.as_ref().unwrap().seed, 4417);
        assert!((data.kinds["ponds"].cover - 0.45).abs() < 1e-6);
    }

    #[test]
    fn ellipse_radius_is_zero_at_the_centre_and_one_on_the_edge() {
        let data = sample_data();
        let site = &data.wetlands[0];
        assert!(site.radius_at(site.center) < 1e-6);
        let angle = site.angle_deg.to_radians();
        // The major axis points east rotated by `angle` towards the north (map y goes down).
        let edge = site.center + Vec2::new(angle.cos(), -angle.sin()) * site.radii.x;
        assert!((site.radius_at(edge) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn seeding_is_deterministic_and_stays_in_the_block() {
        let data = sample_data();
        let render = data.render.clone().unwrap();
        let land = all_land();
        // The wetland spans the map px 4..16, block (0, 0) of the patch level covers 0..16.
        let (a, reeds_a) = seed_block(&data, &render, &land, LEVEL_PATCH, IVec2::ZERO);
        let (b, reeds_b) = seed_block(&data, &render, &land, LEVEL_PATCH, IVec2::ZERO);
        assert_eq!(a, b);
        assert_eq!(reeds_a, reeds_b);
        let size = render.block_units(LEVEL_PATCH);
        assert!(
            a.iter()
                .all(|p| p.pos.x >= 0.0 && p.pos.x < size && p.pos.y >= 0.0 && p.pos.y < size)
        );
        assert!(a.iter().all(|p| data.wetlands[0].radius_at(p.pos) < 1.0));
        assert!(
            !a.is_empty() || reeds_a > 0,
            "a dense wetland seeds something"
        );
        // Sea: nothing.
        let (sea, sea_reeds) = seed_block(
            &data,
            &render,
            &LandMask::new(64, 64, vec![0; 64 * 64]),
            LEVEL_PATCH,
            IVec2::ZERO,
        );
        assert!(sea.is_empty() && sea_reeds == 0);
        // A block far from every wetland is empty.
        let (far, _) = seed_block(&data, &render, &land, LEVEL_PATCH, IVec2::new(50, 50));
        assert!(far.is_empty());
    }

    #[test]
    fn scale_grows_with_the_distance() {
        let render = sample_data().render.unwrap();
        assert_eq!(render.scale_for(1.0), 1.0);
        assert!((render.scale_for(60.0) - render.far_scale).abs() < 1e-3);
        assert!(render.scale_for(10.0) > 1.0 && render.scale_for(10.0) < render.far_scale);
    }

    #[test]
    fn meshes_have_the_expected_vertex_counts() {
        let data = sample_data();
        let render = data.render.clone().unwrap();
        let ground = FlatGround { altitude_m: 10.0 };
        let (pans, cells) = salt_pan_mesh(&data, &render, &all_land(), &ground);
        assert!(cells > 10);
        assert_eq!(pans.unwrap().count_vertices(), cells * 4);
        let foam = torrent_mesh(&data, &ground).unwrap();
        assert_eq!(foam.count_vertices(), 6);
        let pool = Pool {
            pos: Vec2::new(1.0, 2.0),
            diameter_m: 60.0,
            color: [0.1; 3],
            seed: 0.3,
        };
        let ponds = pond_mesh(&[pool, pool], &ground).unwrap();
        assert_eq!(ponds.count_vertices(), 2 * (POND_SIDES + 1));
        assert!(pond_mesh(&[], &ground).is_none());
    }
}
