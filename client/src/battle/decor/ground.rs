//! Textured splat ground of the battlefield (port of battle_terrain_splat.gd and
//! battle_ground_textures.gd, `battle_ground.gdshader`).
//!
//! The terrain mesh carries per-vertex splat weights (COLOR = grass, forest understory, dirt,
//! mud; UV_1 = stone, plough); [`ground.wesl`](./ground.wesl) blends the layers of the regional
//! texture pack of the biome (`data/art/tx_battle_b<biome>_pack.json`, a vertical strip of 13
//! square layers, albedo + normal) on top of Bevy's PBR lighting (sun, shadows, fog), with the
//! micro-grain pack (`micro_ground`), the EP6 decor fields baked at 4 m (`decor_fields`), the R2
//! relief map baked on the field grid, and trample (snow / mud) read from the grass-flatten map
//! of `flatten.rs`. Season tint, snow and wetness follow `battle_terrain.gd` `_build_material`.

use std::path::Path;

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use sim_battle::AreaKind;

use crate::app::AppState;
use sim_battle::{BattleSeason, Battlefield, Ground, Weather};

/// Layers of a pack, in the fixed order of `data/fx/battle_ground_layers.json` `roles`.
pub const LAYER_COUNT: usize = 13;

/// Splat weights of a ground point (they sum to 1 on land, 0 on deep water).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Splat {
    pub grass: f32,
    pub forest: f32,
    pub dirt: f32,
    pub mud: f32,
    pub stone: f32,
    pub plough: f32,
}

impl Splat {
    /// Vertex COLOR attribute.
    pub fn color(self) -> [f32; 4] {
        [self.grass, self.forest, self.dirt, self.mud]
    }

    /// Vertex UV_1 attribute.
    pub fn uv_b(self) -> [f32; 2] {
        [self.stone, self.plough]
    }
}

fn smooth(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Distance from `(x, z)` to the polyline `points`.
pub(super) fn polyline_distance(points: &[(f64, f64)], x: f64, z: f64) -> f64 {
    let mut best = f64::MAX;
    for pair in points.windows(2) {
        let (ax, az) = pair[0];
        let (bx, bz) = pair[1];
        let (dx, dz) = (bx - ax, bz - az);
        let len2 = dx * dx + dz * dz;
        let t = if len2 > 0.0 {
            (((x - ax) * dx + (z - az) * dz) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        best = best.min((x - (ax + dx * t)).hypot(z - (az + dz * t)));
    }
    best
}

/// `StampMap::stamp_soft_disc` falloff: `1 - smooth(radius - feather / 2, radius + feather, d)`.
fn soft_disc(radius: f64, feather: f64, d: f64) -> f64 {
    1.0 - smooth(radius - feather * 0.5, radius + feather, d)
}

/// Feathered coverage (0..1) of the discs `zones` at `(x, z)`.
fn zone_cover(zones: &[sim_battle::Zone], x: f64, z: f64) -> f64 {
    let mut cover: f64 = 0.0;
    for zone in zones {
        let d = (x - zone.x).hypot(z - zone.z);
        let feather = (zone.radius * 0.2).clamp(3.0, 14.0);
        cover = cover.max(smooth(
            zone.radius + feather * 0.3,
            zone.radius - feather,
            d,
        ));
    }
    cover
}

/// Trodden grass and bare earth "in places" under a hamlet, church, manor, farm or camp
/// (`_stamp_decor`: 0.25 hamlet / church, 0.35 other sites). Godot's rectangle is broken by a 3 m
/// jitter and the noise of the dirt smoothstep; here the edge is feathered over 7 m, so the
/// site is never a sharp rectangle.
fn site_dirt(area: &sim_battle::Area, x: f64, z: f64) -> f64 {
    let inside = -area.footprint().signed_distance(x, z);
    let level = if matches!(area.kind, AreaKind::Hamlet | AreaKind::Church) {
        0.25
    } else {
        0.35
    };
    level * smooth(-1.5, 5.5, inside)
}

/// Splat weights of the ground at `(x, z)`.
pub fn splat_at(field: &Battlefield, x: f64, z: f64) -> Splat {
    if field.water_at(x, z) == Some(false) {
        return Splat::default();
    }
    let forest = zone_cover(&field.forests, x, z).max(zone_cover(&field.forest_parts, x, z));
    let mud = zone_cover(&field.mud, x, z)
        .max(zone_cover(&field.mud_parts, x, z))
        .max(zone_cover(&field.pools, x, z) * 0.8);
    let mut dirt: f64 = 0.0;
    for road in &field.roads {
        let d = polyline_distance(&road.points, x, z);
        dirt = dirt.max(smooth(road.width * 0.5 + 1.2, road.width * 0.5 - 0.6, d));
    }
    let mut plough: f64 = 0.0;
    for area in &field.decor.areas {
        if area.kind == AreaKind::Ploughland && area.contains(x, z) {
            plough = 1.0;
        } else if !matches!(
            area.kind,
            AreaKind::Vineyard | AreaKind::Meadow | AreaKind::Orchard | AreaKind::Ploughland
        ) {
            dirt = dirt.max(site_dirt(area, x, z));
        }
    }
    // Farm yards and the surroundings of houses and mills: beaten earth.
    for house in &field.decor.buildings {
        let d = (x - house.x).hypot(z - house.z);
        dirt = dirt.max(soft_disc(house.width * 0.5 + 3.0, 3.0, d) * 0.7);
    }
    for camp in &field.decor.camps {
        let radius = camp.area.length.min(camp.area.width) * 0.5;
        let d = (x - camp.area.x).hypot(z - camp.area.z);
        dirt = dirt.max(soft_disc(radius, radius * 0.6, d) * 0.55);
    }
    // Ford and banks: pebbles.
    let stone = if field.water_at(x, z) == Some(true) {
        1.0
    } else {
        0.0
    };
    let mut weights = [1.0 - forest.max(mud), forest, dirt, mud, stone, plough];
    // Strongest feature wins over the grass base, the rest is renormalised.
    let features: f64 = weights[1..].iter().sum();
    weights[0] = (1.0 - features).max(0.0);
    let sum: f64 = weights.iter().sum::<f64>().max(1e-6);
    let [grass, forest, dirt, mud, stone, plough] = weights.map(|w| (w / sum) as f32);
    Splat {
        grass,
        forest,
        dirt,
        mud,
        stone,
        plough,
    }
}

/// Uniform block of `ground.wesl` (`GroundParams`).
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct GroundParams {
    /// Mean linear colour of each layer (fade to it at a distance), a = tile size (m).
    pub layers: [Vec4; LAYER_COUNT],
    /// Grass tint (rgb, `grass_tint` of battle_terrain_splat.gd), snow cover 0..1 (a).
    pub season: Vec4,
    /// Water colour (rgb), distance where textures fade to their mean (a).
    pub water: Vec4,
    /// x: normal-map strength, y: distance where it fades out, z: 1 with a normal map,
    /// w: brightness variation of the macro noise.
    pub detail: Vec4,
    /// Rectangle (x, z, width, depth) of the decor-field map (`decor_fields`, 4 m texels).
    pub rect: Vec4,
    /// Rectangle (x0, z0, x1, z1) without procedural fields (centre of the field).
    pub calm: Vec4,
    /// x: micro grain on, y: strength, z: distance (m), w: generated ground (`tx_ground`).
    pub micro: Vec4,
    /// Micro layer of each ground layer (16 floats).
    pub micro_layer: [Vec4; 4],
    /// Micro tile size (m) of each ground layer (16 floats).
    pub micro_tile: [Vec4; 4],
    /// x: outcrops, y: detail bump, z: relief map on, w: snow line (m).
    pub relief: Vec4,
    /// Rectangle (x, z, width, depth) of the relief map (texels centred on the 10 m grid).
    pub relief_rect: Vec4,
    /// x: wetness, y: trample on, z: decor fields on, w: decor saturation.
    pub weather: Vec4,
    /// Rectangle (x, z, width, depth) of the trample map (the flattened-grass map).
    pub trample_rect: Vec4,
    /// x: far flatten, y: its start (m), z: its end (m), w: near-detail distance (m).
    pub far: Vec4,
}

impl Default for GroundParams {
    fn default() -> Self {
        GroundParams {
            layers: [Vec4::new(0.3, 0.3, 0.3, 5.0); LAYER_COUNT],
            season: Vec4::new(0.9, 1.0, 0.8, 0.0),
            water: Vec4::new(0.12, 0.28, 0.42, 520.0),
            detail: Vec4::new(0.7, 140.0, 0.0, 0.28),
            rect: Vec4::new(-400.0, -400.0, 2000.0, 1600.0),
            calm: Vec4::new(150.0, 60.0, 1050.0, 740.0),
            micro: Vec4::new(0.0, 0.7, 32.0, 1.0),
            micro_layer: [Vec4::ZERO; 4],
            micro_tile: [Vec4::splat(0.5); 4],
            relief: Vec4::new(0.0, 0.7, 0.0, 10000.0),
            relief_rect: Vec4::ZERO,
            weather: Vec4::new(0.0, 0.0, 0.0, 0.6),
            trample_rect: Vec4::ZERO,
            far: Vec4::new(0.8, 28.0, 180.0, 45.0),
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct GroundExtension {
    #[uniform(100)]
    pub params: GroundParams,
    #[texture(101, dimension = "2d_array")]
    #[sampler(102)]
    pub albedo: Option<Handle<Image>>,
    #[texture(103, dimension = "2d_array")]
    pub normal: Option<Handle<Image>>,
    /// Micro grain pack (`micro_ground`): albedo and normal arrays (read with the albedo sampler,
    /// same descriptor: Metal allows 16 samplers per stage).
    #[texture(105, dimension = "2d_array")]
    pub micro_albedo: Option<Handle<Image>>,
    #[texture(107, dimension = "2d_array")]
    pub micro_normal: Option<Handle<Image>>,
    /// EP6 decor fields (RGBA8, nearest): kind / 8, yaw / pi, edge ramp, coverage.
    #[texture(109)]
    pub decor_fields: Option<Handle<Image>>,
    /// R2 relief map (RGBA8): slope, hollow at 30 m, hollow at 90 m.
    #[texture(111)]
    pub relief: Option<Handle<Image>>,
    /// Trodden ground: the flattened-grass map of `flatten.rs` (R).
    #[texture(113)]
    pub trample: Option<Handle<Image>>,
}

impl MaterialExtension for GroundExtension {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/ground.wesl".into()
    }

    /// The same file also writes the G-buffer (`PREPASS_PIPELINE`) when Solari makes the deferred
    /// renderer the default: one shared material computation, two render paths.
    fn deferred_fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/ground.wesl".into()
    }
}

pub type GroundMaterial = ExtendedMaterial<StandardMaterial, GroundExtension>;

/// Biome (1..14) of a battle terrain key (`data/fx/battle_ground_layers.json`
/// `terrain_biomes`, default biome otherwise).
pub fn biome_for_terrain(data_dir: &Path, terrain_key: &str) -> u32 {
    let spec = read_json(&data_dir.join("fx/battle_ground_layers.json"));
    spec.as_ref()
        .and_then(|s| s["terrain_biomes"][terrain_key].as_u64())
        .or_else(|| spec.as_ref().and_then(|s| s["default_biome"].as_u64()))
        .unwrap_or(2) as u32
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Pack of a biome: image path (under the asset root) and layer means / tile sizes.
#[derive(Debug, Clone)]
pub struct GroundPack {
    pub albedo_path: String,
    pub normal_path: Option<String>,
    pub layers: Vec<Vec4>,
    pub biome: u32,
}

/// Loads the pack manifest of `biome`, falling back on the default biome (2).
pub fn load_pack(data_dir: &Path, biome: u32) -> Option<GroundPack> {
    for candidate in [biome, 2] {
        let manifest =
            read_json(&data_dir.join(format!("art/tx_battle_b{candidate:02}_pack.json")));
        let Some(manifest) = manifest else { continue };
        let albedo = manifest["albedo"]
            .as_str()?
            .trim_start_matches("res://assets/");
        let normal = manifest["normal"]
            .as_str()
            .map(|n| n.trim_start_matches("res://assets/").to_owned());
        let rows = manifest["layers"].as_array()?;
        if rows.len() != LAYER_COUNT {
            continue;
        }
        let layers = rows
            .iter()
            .map(|row| {
                let mean = &row["mean_linear"];
                Vec4::new(
                    mean[0].as_f64().unwrap_or(0.3) as f32,
                    mean[1].as_f64().unwrap_or(0.3) as f32,
                    mean[2].as_f64().unwrap_or(0.3) as f32,
                    row["tile_m"].as_f64().unwrap_or(5.0) as f32,
                )
            })
            .collect();
        return Some(GroundPack {
            albedo_path: albedo.to_owned(),
            normal_path: normal,
            layers,
            biome: candidate,
        });
    }
    None
}

/// Seasonal tint of the grass relative to the summer ground, and the snow cover.
pub fn season_tint(field: &Battlefield, weather: Weather) -> Vec4 {
    let base = super::super::scene::terrain::season_ground(BattleSeason::Summer);
    let now = super::super::scene::terrain::season_ground(field.season);
    let snowy = matches!(weather, Weather::Snow) || matches!(field.ground, Ground::Snowy);
    let tint = |i: usize| {
        if matches!(field.season, BattleSeason::Winter) {
            1.0
        } else {
            (now[i] / base[i]).clamp(0.6, 1.4)
        }
    };
    let snow = if snowy {
        0.85
    } else if matches!(field.season, BattleSeason::Winter) {
        0.6
    } else {
        0.0
    };
    Vec4::new(tint(0), tint(1), tint(2), snow)
}

/// Battle biome values of `battle_terrain.gd` `BIOMES` (relief strength, rock outcrops, snow line).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainLook {
    pub relief: f32,
    pub rocks: f32,
    pub snow_line: f32,
}

/// `BIOMES[terrain_key]` (plains when unknown).
pub fn terrain_look(terrain_key: &str) -> TerrainLook {
    let (relief, rocks, snow_line) = match terrain_key {
        "heath" => (0.6, 1.2, 10000.0),
        "bocage" => (0.7, 0.6, 10000.0),
        "forest" => (0.9, 0.8, 10000.0),
        "hills" => (1.5, 1.3, 10000.0),
        "mountains" => (3.4, 2.2, 150.0),
        "marsh" => (0.2, 0.2, 10000.0),
        _ => (0.45, 0.6, 10000.0),
    };
    TerrainLook {
        relief,
        rocks,
        snow_line,
    }
}

/// Grass tint, snow cover and wetness of the ground (`_build_material` of
/// battle_terrain_splat.gd): snowy or soaked ground, rain, snow, fog, season of a clear day.
pub fn ground_weather(field: &Battlefield, weather: Weather) -> (Vec3, f32, f32) {
    let default_tint = Vec3::new(0.9, 1.0, 0.8);
    if matches!(field.ground, Ground::Snowy) && weather != Weather::Snow {
        return (Vec3::new(0.85, 0.82, 0.7), 0.75, 0.0);
    }
    let wet = if matches!(field.ground, Ground::Muddy) && weather != Weather::Rain {
        0.45
    } else {
        0.0
    };
    if weather == Weather::Clear {
        match field.season {
            BattleSeason::Winter => return (Vec3::new(0.92, 0.9, 0.72), 0.0, wet),
            BattleSeason::Autumn => return (Vec3::new(1.0, 0.95, 0.72), 0.0, wet),
            BattleSeason::Summer if field.terrain.key() != "marsh" => {
                return (Vec3::new(0.94, 1.0, 0.78), 0.0, wet);
            }
            _ => {}
        }
    }
    match weather {
        Weather::Rain => (Vec3::new(0.9, 0.95, 0.85), 0.0, 0.75),
        Weather::Snow => (Vec3::new(0.85, 0.85, 0.78), 0.85, wet),
        Weather::Fog => (default_tint, 0.0, wet.max(0.25)),
        Weather::Clear => (default_tint, 0.0, wet),
    }
}

/// Texel (m) of the decor-field map (`SPLAT_TEXEL`).
pub const DECOR_TEXEL: f64 = 4.0;
/// Margin (m) of the decor-field map around the field (`SPLAT_RECT`).
pub const DECOR_MARGIN: f64 = 400.0;

/// Decor-field map (`_stamp_decor` of battle_terrain_splat.gd): RGBA8 texels over the field and
/// its margin. R = kind / 8 (1 ploughed, 2 corn, 3 meadow or orchard, 4 sown, 5 vineyard,
/// 6 stubble), G = yaw / pi, B = distance to the edge / 8 m, A = 255 on a field, 128 under the
/// other decor areas (no procedural fields there), 0 elsewhere.
#[derive(Debug, Clone)]
pub struct DecorFields {
    pub width: usize,
    pub height: usize,
    pub rect: Vec4,
    pub bytes: Vec<u8>,
    /// Texels of each kind (index = code, 0 = clear area only).
    pub kinds: [usize; 7],
}

/// Field code of a decor area (0: clear ground only, not a field).
pub fn field_code(area: &sim_battle::Area) -> u8 {
    use sim_battle::FieldState;
    match area.kind {
        AreaKind::Ploughland => match area.state {
            Some(FieldState::Crop) => 2,
            Some(FieldState::Sown) => 4,
            Some(FieldState::Stubble) => 6,
            _ => 1,
        },
        AreaKind::Meadow | AreaKind::Orchard => 3,
        AreaKind::Vineyard => 5,
        _ => 0,
    }
}

impl DecorFields {
    /// Field under `(x, z)` (nearest texel, like the shader without its edge warp): the code
    /// (2 corn, 4 sown, 6 stubble...) and the edge ramp 0..1 (1 deeper than 8 m inside). `None`
    /// outside the map or off the fields (clear areas included).
    pub fn field_at(&self, x: f32, z: f32) -> Option<(u8, f32)> {
        let ix = ((x - self.rect.x) / DECOR_TEXEL as f32).floor();
        let iz = ((z - self.rect.y) / DECOR_TEXEL as f32).floor();
        if ix < 0.0 || iz < 0.0 || ix as usize >= self.width || iz as usize >= self.height {
            return None;
        }
        let i = (iz as usize * self.width + ix as usize) * 4;
        (self.bytes[i + 3] == 255).then(|| {
            let code = (f32::from(self.bytes[i]) / 255.0 * 8.0).round() as u8;
            (code, f32::from(self.bytes[i + 2]) / 255.0)
        })
    }
}

pub fn bake_decor_fields(field: &Battlefield) -> DecorFields {
    let x0 = -DECOR_MARGIN;
    let z0 = -DECOR_MARGIN;
    let width = ((field.width + 2.0 * DECOR_MARGIN) / DECOR_TEXEL) as usize;
    let height = ((field.depth + 2.0 * DECOR_MARGIN) / DECOR_TEXEL) as usize;
    let mut bytes = vec![0u8; width * height * 4];
    let mut kinds = [0usize; 7];
    for area in &field.decor.areas {
        let code = field_code(area);
        let (hx, hz) = (area.length * 0.5, area.width * 0.5);
        let reach = hx.hypot(hz);
        let yaw01 = area.yaw.rem_euclid(std::f64::consts::PI) / std::f64::consts::PI;
        let (sin, cos) = area.yaw.sin_cos();
        let lo = |c: f64, o: f64| (((c - reach - o) / DECOR_TEXEL) as i64).max(0) as usize;
        let hi = |c: f64, o: f64, n: usize| {
            (((c + reach - o) / DECOR_TEXEL) as i64 + 1).clamp(0, n as i64 - 1) as usize
        };
        for iz in lo(area.z, z0)..=hi(area.z, z0, height) {
            for ix in lo(area.x, x0)..=hi(area.x, x0, width) {
                let (dx, dz) = (
                    x0 + ix as f64 * DECOR_TEXEL - area.x,
                    z0 + iz as f64 * DECOR_TEXEL - area.z,
                );
                // Rotated by -yaw: the length runs along (cos yaw, sin yaw).
                let (lx, lz) = (dx * cos + dz * sin, -dx * sin + dz * cos);
                let edge = (hx - lx.abs()).min(hz - lz.abs());
                if edge < 0.0 {
                    continue;
                }
                let i = (iz * width + ix) * 4;
                if code == 0 {
                    if bytes[i + 3] == 0 {
                        bytes[i + 3] = 128;
                        kinds[0] += 1;
                    }
                    continue;
                }
                if bytes[i + 3] != 255 {
                    kinds[code as usize] += 1;
                }
                bytes[i] = (f64::from(code) / 8.0 * 255.0).round() as u8;
                bytes[i + 1] = (yaw01 * 255.0).round() as u8;
                bytes[i + 2] = ((edge / 8.0).clamp(0.0, 1.0) * 255.0).round() as u8;
                bytes[i + 3] = 255;
            }
        }
    }
    DecorFields {
        width,
        height,
        rect: Vec4::new(
            x0 as f32,
            z0 as f32,
            (width as f64 * DECOR_TEXEL) as f32,
            (height as f64 * DECOR_TEXEL) as f32,
        ),
        bytes,
        kinds,
    }
}

/// Relief map R2 (`relief_from_heights` of battle_terrain_field.rs) on the field's height grid:
/// r = slope (1 = 50 %), g = hollow (> 0.5) or bump (< 0.5) at 30 m, b = same at 90 m.
/// Returns the bytes and the rectangle (texels centred on the grid points).
pub fn bake_relief(field: &Battlefield) -> (Vec<u8>, Vec4) {
    let (hw, hh) = (field.nx, field.nz);
    let mut bytes = vec![0u8; hw * hh * 4];
    let texel = field.resolution;
    let at = |ix: i64, iz: i64| -> f64 {
        let z = iz.clamp(0, hh as i64 - 1) as usize;
        let x = ix.clamp(0, hw as i64 - 1) as usize;
        field.heights[z * hw + x]
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
    let half = (texel * 0.5) as f32;
    let rect = Vec4::new(
        -half,
        -half,
        (hw as f64 * texel) as f32,
        (hh as f64 * texel) as f32,
    );
    (bytes, rect)
}

/// Micro grain of the ground (`micro_for` of battle_ground_textures.gd): arrays and, per ground
/// layer (in the order of `roles`), its micro layer and tile size.
#[derive(Debug, Clone, Default)]
pub struct MicroPack {
    pub albedo_path: String,
    pub normal_path: String,
    pub grid: (u32, u32),
    pub layer_of: Vec<f32>,
    pub tile_of: Vec<f32>,
}

pub fn load_micro(data_dir: &Path, biome: u32) -> Option<MicroPack> {
    let spec = read_json(&data_dir.join("fx/battle_ground_layers.json"))?;
    let micro = &spec["micro"];
    let manifest = read_json(&data_dir.join(micro["manifest"].as_str()?))?;
    let path = |key: &str| {
        manifest[key]
            .as_str()
            .map(|p| p.trim_start_matches("res://assets/").to_owned())
    };
    let grid = manifest["grid"].as_array()?;
    let grid = (
        grid.first()?.as_u64()? as u32,
        grid.get(1)?.as_u64()? as u32,
    );
    let layers = manifest["layers"].as_array()?;
    let find = |id: &str| layers.iter().find(|l| l["id"].as_str() == Some(id));
    let default_id = micro["default"].as_str().unwrap_or("");
    let dry = micro["dry_biomes"]
        .as_array()
        .is_some_and(|b| b.iter().any(|v| v.as_u64() == Some(u64::from(biome))));
    let mut pack = MicroPack {
        albedo_path: path("albedo")?,
        normal_path: path("normal")?,
        grid,
        ..default()
    };
    for role in spec["roles"].as_array()? {
        let role = role.as_str().unwrap_or("");
        let mut grain = micro["role_grain"][role].as_str().unwrap_or(default_id);
        if dry && let Some(g) = micro["dry_role_grain"][role].as_str() {
            grain = g;
        }
        let layer = find(grain).or_else(|| find(default_id));
        pack.layer_of
            .push(layer.and_then(|l| l["layer"].as_f64()).unwrap_or(0.0) as f32);
        pack.tile_of
            .push(layer.and_then(|l| l["tile_m"].as_f64()).unwrap_or(0.5) as f32);
    }
    Some(pack)
}

/// Packs up to 16 floats into 4 vec4 (shader arrays).
pub fn pack16(values: &[f32], fill: f32) -> [Vec4; 4] {
    let mut out = [Vec4::splat(fill); 4];
    for (i, v) in values.iter().take(16).enumerate() {
        out[i / 4][i % 4] = *v;
    }
    out
}

/// Rearranges a `cols` x `rows` grid of square layers into a vertical strip (layers one under the
/// other, row by row as Godot slices them), so `strip_to_array` can turn it into an array.
pub fn grid_to_strip(image: &mut Image, cols: u32, rows: u32) -> bool {
    let started = std::time::Instant::now();
    let name = format!(
        "grid_to_strip {}x{} ({cols}x{rows})",
        image.width(),
        image.height()
    );
    let ok = grid_to_strip_inner(image, cols, rows);
    crate::agent::texture_census::record(name, started, image.data.as_ref().map_or(0, Vec::len));
    ok
}

fn grid_to_strip_inner(image: &mut Image, cols: u32, rows: u32) -> bool {
    if cols <= 1 {
        return true;
    }
    let Some(data) = image.data.as_ref() else {
        return false;
    };
    let (w, h) = (image.width(), image.height());
    if w % cols != 0 || h % rows != 0 || w / cols != h / rows {
        return false;
    }
    let bpp = data.len() / (w * h) as usize;
    let size = (w / cols) as usize;
    let mut out = Vec::with_capacity(data.len());
    for row in 0..rows as usize {
        for col in 0..cols as usize {
            for y in 0..size {
                let start = ((row * size + y) * w as usize + col * size) * bpp;
                out.extend_from_slice(&data[start..start + size * bpp]);
            }
        }
    }
    image.data = Some(out);
    image.texture_descriptor.size = Extent3d {
        width: size as u32,
        height: (size * (cols * rows) as usize) as u32,
        depth_or_array_layers: 1,
    };
    true
}

/// A grid texture (`cols` x `rows` layers) waiting to be turned into a mip-mapped array.
#[derive(Debug, Clone)]
pub struct PendingGrid {
    pub handle: Handle<Image>,
    pub grid: (u32, u32),
    pub anisotropy: u16,
}

#[derive(Resource, Default, Debug)]
pub struct PendingGrids(pub Vec<PendingGrid>);

/// The ground arrays (albedo 73 MiB, normals, micro grain ~85 MiB with mips) are only read by the
/// GPU: with `RENDER_WORLD` only, Bevy moves the pixels out of the main world at upload instead of
/// keeping a second copy in RAM. No CPU consumer reads them (the ground has no CPU sampler; the
/// siege backdrop and the town atlas use `strip_to_array` on their own images and are untouched).
fn drop_cpu_copy(image: &mut Image) {
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
}

fn convert_grids(
    mut commands: Commands,
    mut pending: ResMut<PendingGrids>,
    mut images: ResMut<Assets<Image>>,
    mut stats: Option<ResMut<GroundStats>>,
) {
    pending.0.retain(|grid| {
        let Some(mut image) = images.get_mut(&grid.handle) else {
            return true;
        };
        let (cols, rows) = grid.grid;
        let ok = grid_to_strip(&mut image, cols, rows)
            && strip_to_array(&mut image, cols * rows, grid.anisotropy);
        if ok {
            drop_cpu_copy(&mut image);
        }
        if let Some(stats) = stats.as_mut() {
            stats.micro_arrays += usize::from(ok);
        }
        if !ok {
            warn!("battle ground: texture grid has an unexpected shape, not converted");
        }
        false
    });
    if pending.0.is_empty() {
        commands.remove_resource::<PendingGrids>();
    }
}

fn data_image(width: usize, height: usize, bytes: Vec<u8>, nearest: bool) -> Image {
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    let filter = if nearest {
        ImageFilterMode::Nearest
    } else {
        ImageFilterMode::Linear
    };
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: filter,
        min_filter: filter,
        ..default()
    });
    image
}

/// Everything the battle ground material needs, built from the field (called by
/// `scene::terrain`): the material, its statistics and the textures still to convert.
#[allow(clippy::too_many_arguments)]
pub fn build_ground_material(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    assets: &AssetServer,
    look: &super::look::DecorLook,
    data_dir: &Path,
    field: &Battlefield,
    weather: Weather,
    splat_colors: &[[f32; 4]],
    splat_b: &[[f32; 2]],
) -> GroundMaterial {
    let biome = biome_for_terrain(data_dir, field.terrain.key());
    let (tint, snow, wetness) = ground_weather(field, weather);
    let terrain = terrain_look(field.terrain.key());
    let mut params = GroundParams {
        season: tint.extend(snow),
        ..default()
    };
    params.water.w = look.ground.texture_fade_m;
    params.detail = Vec4::new(
        look.ground.normal_strength,
        look.ground.normal_fade_m,
        0.0,
        look.ground.macro_contrast,
    );
    let mut snow_line = terrain.snow_line;
    if field.season != BattleSeason::Winter && snow_line < 10000.0 {
        snow_line *= 2.2;
    }
    params.relief = Vec4::new(
        ((terrain.rocks - 0.7) / 1.5).clamp(0.0, 1.0),
        0.5 + 0.5 * (terrain.relief / 3.4).clamp(0.0, 1.0),
        1.0,
        snow_line,
    );
    let trample_on = matches!(field.ground, Ground::Snowy | Ground::Muddy)
        || matches!(weather, Weather::Snow | Weather::Rain);
    params.weather = Vec4::new(wetness, f32::from(u8::from(trample_on)), 1.0, 0.6);
    let (w, d) = (field.width as f32, field.depth as f32);
    // Calm centre: the Godot rectangle (150, 60, 1050, 740) of a 1200 x 800 m field, scaled.
    params.calm = Vec4::new(w * 0.125, d * 0.075, w * 0.875, d * 0.925);
    let mut albedo = None;
    let mut normal = None;
    let pack = load_pack(data_dir, biome);
    let pack_loaded = pack.is_some();
    let mut sources = ArraySources::default();
    if let Some(pack) = pack {
        for (slot, layer) in params.layers.iter_mut().zip(&pack.layers) {
            *slot = *layer;
        }
        let layers = pack.layers.len() as u32;
        let anisotropy = look.ground.anisotropy;
        let albedo_handle = ground_array(
            assets,
            data_dir,
            &mut sources,
            ArraySpec {
                slot: GroundSlot::Albedo,
                path: pack.albedo_path,
                srgb: true,
                grid: (1, layers),
                anisotropy,
            },
        );
        albedo = Some(albedo_handle);
        if let Some(path) = pack.normal_path {
            let handle = ground_array(
                assets,
                data_dir,
                &mut sources,
                ArraySpec {
                    slot: GroundSlot::Normal,
                    path,
                    srgb: false,
                    grid: (1, layers),
                    anisotropy,
                },
            );
            params.detail.z = 1.0;
            normal = Some(handle);
        }
    } else {
        warn!("battle ground: no texture pack for biome {biome}, flat colours");
    }
    // Micro grain (generated grounds only, like Godot's `micro_on`).
    let mut micro_albedo = None;
    let mut micro_normal = None;
    let micro = if pack_loaded {
        load_micro(data_dir, biome)
    } else {
        None
    };
    if let Some(micro) = &micro {
        let spec = |slot, path: &String, srgb| ArraySpec {
            slot,
            path: path.clone(),
            srgb,
            grid: micro.grid,
            anisotropy: look.ground.anisotropy,
        };
        let a = ground_array(
            assets,
            data_dir,
            &mut sources,
            spec(GroundSlot::MicroAlbedo, &micro.albedo_path, true),
        );
        let n = ground_array(
            assets,
            data_dir,
            &mut sources,
            spec(GroundSlot::MicroNormal, &micro.normal_path, false),
        );
        params.micro.x = 1.0;
        params.micro_layer = pack16(&micro.layer_of, 0.0);
        params.micro_tile = pack16(&micro.tile_of, 0.5);
        micro_albedo = Some(a);
        micro_normal = Some(n);
    }
    params.micro.w = f32::from(u8::from(pack_loaded));
    sources.insert_into(commands);
    let fields = bake_decor_fields(field);
    params.rect = fields.rect;
    let (relief_bytes, relief_rect) = bake_relief(field);
    params.relief_rect = relief_rect;
    let mut stats = GroundStats::of(biome, pack_loaded, splat_colors, splat_b);
    stats.micro = micro.is_some();
    stats.decor_kinds = fields.kinds;
    stats.decor_size = [fields.width, fields.height];
    stats.relief_size = [field.nx, field.nz];
    stats.tint = tint.to_array();
    stats.snow = snow;
    stats.wetness = wetness;
    stats.trample_on = trample_on;
    stats.outcrops = params.relief.x;
    commands.insert_resource(stats);
    let decor = images.add(data_image(fields.width, fields.height, fields.bytes, true));
    let relief = images.add(data_image(field.nx, field.nz, relief_bytes, false));
    GroundMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            reflectance: 0.05,
            ..default()
        },
        extension: GroundExtension {
            params,
            albedo,
            normal,
            micro_albedo,
            micro_normal,
            decor_fields: Some(decor),
            relief: Some(relief),
            trample: None,
        },
    }
}

/// Loads a data texture (normal map) without sRGB decoding.
fn load_linear(assets: &AssetServer, path: String) -> Handle<Image> {
    assets
        .load_builder()
        .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| settings.is_srgb = false)
        .load(path)
}

/// Binds the flattened-grass map (`flatten.rs`) to the ground as its trample map, once it exists.
fn bind_trample(
    map: Option<Res<super::flatten::FlattenMap>>,
    terrain: Query<&MeshMaterial3d<GroundMaterial>>,
    mut materials: ResMut<Assets<GroundMaterial>>,
    mut stats: Option<ResMut<GroundStats>>,
) {
    let Some(map) = map else { return };
    for handle in &terrain {
        let Some(material) = materials.get(&handle.0) else {
            continue;
        };
        if material.extension.trample.is_some() {
            continue;
        }
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        material.extension.trample = Some(map.image.clone());
        material.extension.params.trample_rect = map.rect();
        if let Some(stats) = stats.as_mut() {
            stats.trample_bound = true;
        }
    }
}

/// Ground statistics for the dump.
#[derive(Resource, Debug, Clone, Default)]
pub struct GroundStats {
    pub biome: u32,
    pub pack_loaded: bool,
    pub vertices: usize,
    /// Mean splat weight: grass, forest, dirt, mud, stone, plough.
    pub splat_share: [f32; 6],
    /// Texture strips turned into mip-mapped arrays (albedo, normal).
    pub arrays_converted: usize,
    pub mip_levels: u32,
    /// Micro grain pack bound, and its arrays converted.
    pub micro: bool,
    pub micro_arrays: usize,
    /// Baked KTX2 arrays loaded, and the ones that fell back on the JPG strip.
    pub ktx2_loaded: usize,
    pub ktx2_fallback: usize,
    /// Decor-field texels per kind (0 = clear area), map size.
    pub decor_kinds: [usize; 7],
    pub decor_size: [usize; 2],
    pub relief_size: [usize; 2],
    pub tint: [f32; 3],
    pub snow: f32,
    pub wetness: f32,
    pub trample_on: bool,
    pub trample_bound: bool,
    pub outcrops: f32,
}

impl GroundStats {
    pub fn of(biome: u32, pack_loaded: bool, colors: &[[f32; 4]], uvs_b: &[[f32; 2]]) -> Self {
        let mut share = [0.0f32; 6];
        for (c, b) in colors.iter().zip(uvs_b) {
            for (slot, value) in share.iter_mut().zip([c[0], c[1], c[2], c[3], b[0], b[1]]) {
                *slot += value;
            }
        }
        let n = colors.len().max(1) as f32;
        GroundStats {
            biome,
            pack_loaded,
            vertices: colors.len(),
            splat_share: share.map(|v| (v / n * 1000.0).round() / 1000.0),
            ..default()
        }
    }
}

/// A stacked texture strip (layers one under the other) to turn into a mip-mapped 2D array.
#[derive(Debug, Clone)]
pub struct PendingStrip {
    pub handle: Handle<Image>,
    pub layers: u32,
    pub anisotropy: u16,
}

/// Strips waiting for their image to be loaded.
#[derive(Resource, Default, Debug)]
pub struct PendingStrips(pub Vec<PendingStrip>);

/// Box-filtered mip chain of one RGBA8 layer (`size` square), all levels concatenated.
pub fn mip_chain(layer: &[u8], size: usize) -> Vec<u8> {
    let mut out = layer.to_vec();
    let mut previous = layer.to_vec();
    let mut dim = size;
    while dim > 1 {
        let next_dim = dim / 2;
        let mut next = vec![0u8; next_dim * next_dim * 4];
        for y in 0..next_dim {
            for x in 0..next_dim {
                for c in 0..4 {
                    let at = |dx: usize, dy: usize| {
                        u32::from(previous[((y * 2 + dy) * dim + x * 2 + dx) * 4 + c])
                    };
                    next[(y * next_dim + x) * 4 + c] =
                        ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
                }
            }
        }
        out.extend_from_slice(&next);
        previous = next;
        dim = next_dim;
    }
    out
}

/// Number of mip levels of a square texture of side `size`.
pub fn mip_count(size: u32) -> u32 {
    32 - size.max(1).leading_zeros()
}

/// Turns a loaded strip into a repeating, mip-mapped, anisotropic 2D array (layers stay apart, no
/// seam from the atlas). Returns false when the image does not have the expected shape.
pub fn strip_to_array(image: &mut Image, layers: u32, anisotropy: u16) -> bool {
    let started = std::time::Instant::now();
    let name = format!(
        "strip_to_array {}x{} ({layers} layers)",
        image.width(),
        image.height()
    );
    let ok = strip_to_array_inner(image, layers, anisotropy);
    crate::agent::texture_census::record(name, started, image.data.as_ref().map_or(0, Vec::len));
    ok
}

fn strip_to_array_inner(image: &mut Image, layers: u32, anisotropy: u16) -> bool {
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    let Some(data) = image.data.take() else {
        return false;
    };
    let size = image.width();
    let layer_bytes = (size * size * 4) as usize;
    if size == 0
        || !size.is_power_of_two()
        || image.height() != size * layers
        || data.len() != layer_bytes * layers as usize
    {
        image.data = Some(data);
        return false;
    }
    let mut out = Vec::with_capacity(data.len() * 4 / 3 + 64);
    for layer in data.chunks_exact(layer_bytes) {
        out.extend_from_slice(&mip_chain(layer, size as usize));
    }
    image.data = Some(out);
    if image.reinterpret_stacked_2d_as_array(layers).is_err() {
        return false;
    }
    image.texture_descriptor.mip_level_count = mip_count(size);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: anisotropy.max(1),
        ..default()
    });
    true
}

/// Asset source holding the baked KTX2 arrays (`out/ktx2_cache/`, never committed; generated by
/// `uv run --project tools cent-ans textures bake-ktx2`). Added before `DefaultPlugins`.
pub const CACHE_SOURCE: &str = "ktx2cache";

/// Set to disable the baked arrays and build the ground from the JPG strips (A/B comparison).
pub const NO_KTX2_ENV: &str = "CENT_ANS_NO_KTX2";

/// Directory of the baked arrays under the repository root.
pub fn cache_dir(root: &Path) -> std::path::PathBuf {
    root.join("out/ktx2_cache")
}

/// Registers the `ktx2cache://` asset source.
pub struct KtxCacheSourcePlugin {
    pub cache_dir: std::path::PathBuf,
}

impl Plugin for KtxCacheSourcePlugin {
    fn build(&self, app: &mut App) {
        use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
        let dir = self.cache_dir.to_string_lossy().into_owned();
        app.register_asset_source(
            AssetSourceId::from(CACHE_SOURCE),
            AssetSourceBuilder::platform_default(&dir, None),
        );
    }
}

/// Material slot an array is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundSlot {
    Albedo,
    Normal,
    MicroAlbedo,
    MicroNormal,
}

/// One ground array to load: source strip (under the asset root), its layer grid and colour space.
struct ArraySpec {
    slot: GroundSlot,
    path: String,
    srgb: bool,
    /// (columns, rows) of the strip / grid; `(1, n)` is a vertical strip.
    grid: (u32, u32),
    anisotropy: u16,
}

/// Repeating, mip-aware, anisotropic sampler of the ground arrays.
fn array_sampler(anisotropy: u16) -> bevy::image::ImageSampler {
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: anisotropy.max(1),
        ..default()
    })
}

/// A baked KTX2 array being loaded; the JPG strip is the fallback if it fails.
#[derive(Debug, Clone)]
struct PendingKtx {
    handle: Handle<Image>,
    slot: GroundSlot,
    fallback_path: String,
    srgb: bool,
    grid: (u32, u32),
    anisotropy: u16,
    requested: std::time::Instant,
}

#[derive(Resource, Default, Debug)]
struct PendingKtxs(Vec<PendingKtx>);

/// Loads gathered while the material is built, turned into pending resources at the end.
#[derive(Default)]
struct ArraySources {
    ktx: Vec<PendingKtx>,
    strips: Vec<PendingStrip>,
    grids: Vec<PendingGrid>,
}

impl ArraySources {
    fn insert_into(self, commands: &mut Commands) {
        if !self.ktx.is_empty() {
            commands.insert_resource(PendingKtxs(self.ktx));
        }
        if !self.strips.is_empty() {
            commands.insert_resource(PendingStrips(self.strips));
        }
        if !self.grids.is_empty() {
            commands.insert_resource(PendingGrids(self.grids));
        }
    }
}

/// Path of the baked array of `path` in the cache, if the file exists and the cache is not disabled.
fn baked_path(data_dir: &Path, path: &str) -> Option<String> {
    if std::env::var_os(NO_KTX2_ENV).is_some() {
        return None;
    }
    let relative = Path::new(path).with_extension("ktx2");
    let root = data_dir.parent()?;
    cache_dir(root)
        .join(&relative)
        .is_file()
        .then(|| format!("{CACHE_SOURCE}://{}", relative.to_string_lossy()))
}

/// Starts loading a ground array: the baked KTX2 when the cache has it, else the JPG strip (converted
/// on the CPU by `convert_strips` / `convert_grids`).
fn ground_array(
    assets: &AssetServer,
    data_dir: &Path,
    sources: &mut ArraySources,
    spec: ArraySpec,
) -> Handle<Image> {
    let ArraySpec {
        slot,
        path,
        srgb,
        grid,
        anisotropy,
    } = spec;
    if let Some(baked) = baked_path(data_dir, &path) {
        let handle = load_baked(assets, baked, srgb, anisotropy);
        sources.ktx.push(PendingKtx {
            handle: handle.clone(),
            slot,
            fallback_path: path,
            srgb,
            grid,
            anisotropy,
            requested: std::time::Instant::now(),
        });
        return handle;
    }
    load_strip(assets, path, srgb, grid, anisotropy, sources)
}

/// KTX2 array: sampler set by the loader, pixels only on the GPU.
fn load_baked(assets: &AssetServer, path: String, srgb: bool, anisotropy: u16) -> Handle<Image> {
    assets
        .load_builder()
        .with_settings(move |settings: &mut bevy::image::ImageLoaderSettings| {
            settings.is_srgb = srgb;
            settings.sampler = array_sampler(anisotropy);
            settings.asset_usage = RenderAssetUsages::RENDER_WORLD;
        })
        .load(path)
}

/// JPG strip, queued for the CPU conversion into a mip-mapped array.
fn load_strip(
    assets: &AssetServer,
    path: String,
    srgb: bool,
    grid: (u32, u32),
    anisotropy: u16,
    sources: &mut ArraySources,
) -> Handle<Image> {
    let handle = if srgb {
        assets.load(path)
    } else {
        load_linear(assets, path)
    };
    if grid.0 > 1 {
        sources.grids.push(PendingGrid {
            handle: handle.clone(),
            grid,
            anisotropy,
        });
    } else {
        sources.strips.push(PendingStrip {
            handle: handle.clone(),
            layers: grid.1,
            anisotropy,
        });
    }
    handle
}

/// Follows the baked arrays: counts the loaded ones and, when one fails, binds the JPG strip to
/// its slot instead (the old CPU path).
#[allow(clippy::too_many_arguments)]
fn poll_baked(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut pending: ResMut<PendingKtxs>,
    images: Res<Assets<Image>>,
    mut materials: ResMut<Assets<GroundMaterial>>,
    mut stats: Option<ResMut<GroundStats>>,
    mut strips: Option<ResMut<PendingStrips>>,
    mut grids: Option<ResMut<PendingGrids>>,
) {
    use bevy::asset::LoadState;
    let mut fallback = ArraySources::default();
    pending.0.retain(|ktx| match assets.load_state(&ktx.handle) {
        LoadState::Loaded => {
            let name = format!("ktx2 array load {:?}", ktx.slot);
            let image = images.get(&ktx.handle);
            let bytes = image.map_or(0, crate::agent::texture_census::gpu_bytes);
            crate::agent::texture_census::record(name, ktx.requested, bytes);
            if let Some(stats) = stats.as_mut() {
                stats.ktx2_loaded += 1;
                if let Some(image) = image {
                    stats.mip_levels = stats
                        .mip_levels
                        .max(image.texture_descriptor.mip_level_count);
                }
            }
            false
        }
        LoadState::Failed(error) => {
            warn!(
                "battle ground: baked array of {} failed ({error}), using the JPG strip",
                ktx.fallback_path
            );
            let handle = load_strip(
                &assets,
                ktx.fallback_path.clone(),
                ktx.srgb,
                ktx.grid,
                ktx.anisotropy,
                &mut fallback,
            );
            for (_, material) in materials.iter_mut() {
                let extension = &mut material.extension;
                let slot = match ktx.slot {
                    GroundSlot::Albedo => &mut extension.albedo,
                    GroundSlot::Normal => &mut extension.normal,
                    GroundSlot::MicroAlbedo => &mut extension.micro_albedo,
                    GroundSlot::MicroNormal => &mut extension.micro_normal,
                };
                if slot.as_ref() == Some(&ktx.handle) {
                    *slot = Some(handle.clone());
                }
            }
            if let Some(stats) = stats.as_mut() {
                stats.ktx2_fallback += 1;
            }
            false
        }
        _ => true,
    });
    match strips.as_mut() {
        Some(existing) => existing.0.append(&mut fallback.strips),
        None if !fallback.strips.is_empty() => {
            commands.insert_resource(PendingStrips(std::mem::take(&mut fallback.strips)));
        }
        None => {}
    }
    match grids.as_mut() {
        Some(existing) => existing.0.append(&mut fallback.grids),
        None if !fallback.grids.is_empty() => {
            commands.insert_resource(PendingGrids(std::mem::take(&mut fallback.grids)));
        }
        None => {}
    }
    if pending.0.is_empty() {
        commands.remove_resource::<PendingKtxs>();
    }
}

fn convert_strips(
    mut commands: Commands,
    mut pending: ResMut<PendingStrips>,
    mut images: ResMut<Assets<Image>>,
    mut stats: Option<ResMut<GroundStats>>,
) {
    pending.0.retain(|strip| {
        let Some(mut image) = images.get_mut(&strip.handle) else {
            return true;
        };
        let ok = strip_to_array(&mut image, strip.layers, strip.anisotropy);
        if ok {
            drop_cpu_copy(&mut image);
        }
        if let Some(stats) = stats.as_mut() {
            stats.arrays_converted += usize::from(ok);
            stats.mip_levels = stats
                .mip_levels
                .max(image.texture_descriptor.mip_level_count);
        }
        if !ok {
            warn!("battle ground: texture strip has an unexpected shape, not converted");
        }
        false
    });
    if pending.0.is_empty() {
        commands.remove_resource::<PendingStrips>();
    }
}

pub struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "ground.wesl");
        app.add_plugins(MaterialPlugin::<GroundMaterial>::default())
            .add_systems(
                Update,
                (
                    poll_baked.run_if(resource_exists::<PendingKtxs>),
                    convert_strips.run_if(resource_exists::<PendingStrips>),
                    convert_grids.run_if(resource_exists::<PendingGrids>),
                    bind_trample.run_if(resource_exists::<super::flatten::FlattenMap>),
                ),
            )
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<PendingGrids>();
                commands.remove_resource::<PendingKtxs>();
            });
        crate::agent::dump::DumpAppExt::register_dump(app, dump_ground);
    }
}

fn dump_ground(world: &World) -> (String, serde_json::Value) {
    let stats = world.get_resource::<GroundStats>();
    (
        "battle_ground".to_owned(),
        serde_json::json!(stats.map(|g| serde_json::json!({
            "biome": g.biome, "pack_loaded": g.pack_loaded, "arrays_converted": g.arrays_converted,
            "micro": g.micro, "micro_arrays": g.micro_arrays,
            "ktx2_loaded": g.ktx2_loaded, "ktx2_fallback": g.ktx2_fallback,
            "decor_fields": {"size": g.decor_size, "clear": g.decor_kinds[0],
                "ploughed": g.decor_kinds[1], "corn": g.decor_kinds[2], "meadow": g.decor_kinds[3],
                "sown": g.decor_kinds[4], "vineyard": g.decor_kinds[5], "stubble": g.decor_kinds[6]},
            "relief_size": g.relief_size, "outcrops": g.outcrops,
            "grass_tint": g.tint, "snow": g.snow, "wetness": g.wetness,
            "trample_on": g.trample_on, "trample_bound": g.trample_bound,
        }))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_chain_halves_down_to_one_pixel() {
        let layer = vec![200u8; 8 * 8 * 4];
        let chain = mip_chain(&layer, 8);
        assert_eq!(chain.len(), (64 + 16 + 4 + 1) * 4);
        assert!(chain.iter().all(|v| *v == 200));
        assert_eq!(mip_count(1024), 11);
        assert_eq!(mip_count(8), 4);
    }

    #[test]
    fn polyline_distance_is_zero_on_the_line() {
        let line = [(0.0, 0.0), (10.0, 0.0)];
        assert!(polyline_distance(&line, 5.0, 0.0) < 1e-9);
        assert!((polyline_distance(&line, 5.0, 3.0) - 3.0).abs() < 1e-9);
        assert!((polyline_distance(&line, 13.0, 4.0) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn smooth_goes_from_zero_to_one_either_way() {
        assert_eq!(smooth(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smooth(0.0, 1.0, 2.0), 1.0);
        assert_eq!(smooth(1.0, 0.0, 2.0), 0.0);
        assert_eq!(smooth(1.0, 0.0, -1.0), 1.0);
    }

    /// Resolves the `@if(PREPASS_PIPELINE)` / `@if(!PREPASS_PIPELINE)` single-statement lines of
    /// the WESL source for one render path (what Bevy's shader-def preprocessing does).
    fn resolve_path(source: &str, deferred: bool) -> String {
        let mut out = String::new();
        let mut keep_next: Option<bool> = None;
        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(cond) = trimmed
                .strip_prefix("@if(")
                .and_then(|rest| rest.strip_suffix(')'))
            {
                let negated = cond.starts_with('!');
                assert_eq!(cond.trim_start_matches('!'), "PREPASS_PIPELINE");
                keep_next = Some(deferred != negated);
                continue;
            }
            if keep_next.take().unwrap_or(true) {
                out.push_str(line);
            }
            out.push('\n');
        }
        out
    }

    fn validate_ground_shader(deferred: bool) {
        let source = include_str!("ground.wesl");
        let body_start = source.find("struct GroundParams").unwrap();
        let stubs = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv_b: vec2<f32>,
    @location(3) color: vec4<f32>,
};
struct FragmentOutput { @location(0) color: vec4<f32>, @location(1) deferred: vec4<u32>, };
struct Mat { base_color: vec4<f32>, perceptual_roughness: f32, };
struct PbrInput { material: Mat, world_normal: vec3<f32>, N: vec3<f32>, };
struct View { world_position: vec3<f32>, };
@group(0) @binding(0) var<uniform> view: View;
fn pbr_input_from_standard_material(in: VertexOutput, f: bool) -> PbrInput {
    return PbrInput(Mat(vec4<f32>(1.0), 1.0), in.world_normal, in.world_normal);
}
fn apply_pbr_lighting(p: PbrInput) -> vec4<f32> { return p.material.base_color * dot(p.N, vec3<f32>(0.0, 1.0, 0.0)); }
fn main_pass_post_lighting_processing(p: PbrInput, c: vec4<f32>) -> vec4<f32> { return c; }
fn deferred_output(in: VertexOutput, p: PbrInput) -> FragmentOutput {
    return FragmentOutput(p.material.base_color, vec4<u32>(0u));
}
fn apply_decals(p: ptr<function, PbrInput>) {}
"#;
        let wgsl = format!(
            "{stubs}{}",
            resolve_path(&source[body_start..], deferred)
                .replace("constants::MATERIAL_BIND_GROUP", "2")
        );
        let module = naga::front::wgsl::parse_str(&wgsl)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&wgsl)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}"));
    }

    /// `ground.wesl` validates (naga) once Bevy's imports are replaced by stubs, on the forward
    /// path and on the deferred one (G-buffer output, Solari).
    #[test]
    fn ground_shader_validates() {
        validate_ground_shader(false);
    }

    #[test]
    fn ground_shader_validates_deferred() {
        validate_ground_shader(true);
    }

    #[test]
    fn decor_fields_and_relief_bake_on_the_default_field() {
        use crate::battle::scene::setup;
        use crate::paths;
        use data_model::GameData;
        let root = paths::find_root().unwrap();
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).unwrap();
        setup::install_battle_rules(&dir);
        let custom = setup::default_custom_battle(&game);
        let sim = setup::build_sim(&game, &dir, &custom, 7).unwrap();
        let field = sim.field();
        let fields = bake_decor_fields(field);
        assert_eq!(fields.bytes.len(), fields.width * fields.height * 4);
        let painted: usize = fields.kinds[1..].iter().sum();
        let expected = field
            .decor
            .areas
            .iter()
            .filter(|a| field_code(a) > 0)
            .count();
        assert_eq!(painted > 0, expected > 0, "{:?}", fields.kinds);
        let (relief, rect) = bake_relief(field);
        assert_eq!(relief.len(), field.nx * field.nz * 4);
        assert!(rect.z > field.width as f32 - 1.0);
        assert!(load_micro(&dir, 2).is_some_and(|m| m.layer_of.len() == LAYER_COUNT));
    }

    #[test]
    fn site_dirt_has_no_sharp_rectangle_edge() {
        use crate::battle::scene::setup;
        use crate::paths;
        use data_model::GameData;
        let root = paths::find_root().unwrap();
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).unwrap();
        setup::install_battle_rules(&dir);
        let custom = setup::default_custom_battle(&game);
        let sim = setup::build_sim(&game, &dir, &custom, 7).unwrap();
        let field = sim.field();
        let mut sites = 0;
        for area in field.decor.areas.iter().filter(|a| {
            !matches!(
                a.kind,
                AreaKind::Vineyard | AreaKind::Meadow | AreaKind::Orchard | AreaKind::Ploughland
            )
        }) {
            sites += 1;
            // Walk across the long axis, 0.5 m steps, through the edge: no jump above 0.05 (a 0.35 level over 7 m, steepest 0.04 per step; a hard edge jumps 0.25-0.35).
            let (sin, cos) = area.yaw.sin_cos();
            let half = area.length * 0.5;
            let mut previous: Option<f32> = None;
            let mut t = -half - 8.0;
            while t < half + 8.0 {
                let dirt = site_dirt(area, area.x + t * cos, area.z + t * sin) as f32;
                if let Some(p) = previous {
                    assert!(
                        (dirt - p).abs() < 0.05,
                        "{:?} jump {p} -> {dirt}",
                        area.kind
                    );
                }
                previous = Some(dirt);
                t += 0.5;
            }
        }
        assert!(
            sites > 0 || field.decor.areas.is_empty(),
            "no site on the seed field"
        );
    }

    #[test]
    fn grid_to_strip_stacks_the_cells_row_by_row() {
        let mut data = vec![0u8; 4 * 2 * 4];
        for y in 0..2 {
            for x in 0..4 {
                data[(y * 4 + x) * 4] = (x / 2) as u8;
            }
        }
        let mut image = Image::new(
            Extent3d {
                width: 4,
                height: 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        assert!(grid_to_strip(&mut image, 2, 1));
        assert_eq!((image.width(), image.height()), (2, 4));
        let bytes = image.data.unwrap();
        assert_eq!(bytes[0], 0);
        assert_eq!(bytes[2 * 2 * 4], 1);
    }
}
