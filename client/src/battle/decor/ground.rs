//! Textured splat ground of the battlefield (port of battle_terrain_splat.gd and
//! battle_ground_textures.gd, `battle_ground.gdshader`).
//!
//! The terrain mesh carries per-vertex splat weights (COLOR = grass, forest understory, dirt,
//! mud; UV_1 = stone, plough); [`ground.wesl`](./ground.wesl) blends the layers of the regional
//! texture pack of the biome (`data/art/tx_battle_b<biome>_pack.json`, a vertical strip of 13
//! square layers) on top of Bevy's PBR lighting (sun, shadows, fog). Not ported: normal maps of
//! the pack, the micro-grain pack, EP6 field parcels (wheat / stubble) and the grass-flatten map.

use std::path::Path;

use bevy::asset::embedded_asset;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use sim_battle::AreaKind;
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
fn polyline_distance(points: &[(f64, f64)], x: f64, z: f64) -> f64 {
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
        } else if matches!(area.kind, AreaKind::Hamlet | AreaKind::Farmstead) && area.contains(x, z)
        {
            dirt = dirt.max(0.45);
        }
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
    /// Seasonal tint of grass (rgb), snow cover 0..1 (a).
    pub season: Vec4,
    /// Water colour (rgb), distance where textures fade to their mean (a).
    pub water: Vec4,
}

impl Default for GroundParams {
    fn default() -> Self {
        GroundParams {
            layers: [Vec4::new(0.3, 0.3, 0.3, 5.0); LAYER_COUNT],
            season: Vec4::new(1.0, 1.0, 1.0, 0.0),
            water: Vec4::new(0.12, 0.28, 0.42, 160.0),
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct GroundExtension {
    #[uniform(100)]
    pub params: GroundParams,
    #[texture(101)]
    #[sampler(102)]
    pub albedo: Option<Handle<Image>>,
}

impl MaterialExtension for GroundExtension {
    fn fragment_shader() -> ShaderRef {
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

/// Ground statistics for the dump.
#[derive(Resource, Debug, Clone, Default)]
pub struct GroundStats {
    pub biome: u32,
    pub pack_loaded: bool,
    pub vertices: usize,
    /// Mean splat weight: grass, forest, dirt, mud, stone, plough.
    pub splat_share: [f32; 6],
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
        }
    }
}

pub struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "ground.wesl");
        app.add_plugins(MaterialPlugin::<GroundMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
