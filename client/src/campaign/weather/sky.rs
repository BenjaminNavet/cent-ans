//! High sky and low atmosphere props of `map_atmosphere.gd` / `campaign_weather_view.gd`:
//! a cirrus layer, rain curtains over rainy provinces, river-bank fog banks that lift with the
//! morning, and storm lightning (flashes of the sun light). Settings come from
//! `data/fx/map_atmosphere.json` (`cirrus`, `rain_curtains`, `river_fog`).
//!
//! The Godot shaders are replaced by unlit blended `StandardMaterial` quads with procedural
//! textures (scrolled through `uv_transform`); no terrain-shader change. Everything fades out
//! on the parchment view through `ZoomView::strategic` and by camera-distance bands.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use super::model::{Kind, hash01, smoothstep};
use super::{CampaignWeather, WeatherView};
use crate::app::AppState;
use crate::campaign::camera::CampaignCamera;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::terrain::CampaignSun;
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::season::SeasonWeights;
use crate::campaign::terrain::surface::GroundHeight;

/// Storm lightning only strikes when the camera is closer than this (`_update_lightning`).
pub const LIGHTNING_MAX_DISTANCE: f32 = 700.0;
/// Duration of one flash (seconds).
pub const FLASH_SECONDS: f32 = 0.18;

/// Fade 0 -> 1 between `fade_in` then 1 -> 0 between `fade_out` (`band_fade`).
pub fn band_fade(distance: f32, fade_in: [f32; 2], fade_out: [f32; 2]) -> f32 {
    smoothstep(fade_in[0], fade_in[1], distance)
        * (1.0 - smoothstep(fade_out[0], fade_out[1], distance))
}

/// Share of the season blend (spring, summer, autumn, winter) for a 4-value table.
pub fn season_share(weights: Vec4, shares: [f32; 4]) -> f32 {
    weights.dot(Vec4::from_array(shares)).clamp(0.0, 1.0)
}

/// Sun-light multiplier during a flash: flickers between 2.6 and 1.4 every 0.04 s, 1 when over
/// (`_flash_timer` of the Godot view, `timer` = seconds left).
pub fn flash_factor(timer: f32) -> f32 {
    if timer <= 0.0 {
        1.0
    } else if (timer % 0.08) > 0.04 {
        2.6
    } else {
        1.4
    }
}

/// Fog bank positions: one every `step_px` along rivers of importance >= `min_importance`,
/// at most `max_banks` (evenly thinned, deterministic) (`river_bank_points`).
pub fn river_bank_points(
    rivers: &[(i64, Vec<Vec2>)],
    min_importance: i64,
    step_px: f32,
    max_banks: usize,
) -> Vec<Vec2> {
    let mut points = Vec::new();
    for (importance, line) in rivers {
        if *importance < min_importance {
            continue;
        }
        let mut carried = 0.0;
        for pair in line.windows(2) {
            carried += pair[1].distance(pair[0]);
            if carried >= step_px {
                carried = 0.0;
                points.push(pair[1]);
            }
        }
    }
    if max_banks == 0 {
        return Vec::new();
    }
    if points.len() <= max_banks {
        return points;
    }
    let stride = points.len() as f32 / max_banks as f32;
    (0..max_banks)
        .map(|i| points[(i as f32 * stride) as usize])
        .collect()
}

/// One rain curtain to place: map position, width, random phase, storm flag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurtainPlacement {
    pub position: Vec2,
    pub width: f32,
    pub rand: f32,
    pub storm: bool,
}

/// Curtains for a weather: `per_province` per rainy or stormy province (centroid plus a
/// deterministic offset), storms first, at most `max` (`rain_placements`).
/// `provinces` = (id, kind, centroid).
pub fn rain_placements(
    provinces: &[(String, Kind, Vec2)],
    per_province: usize,
    max: usize,
    width: [f32; 2],
    seed: u64,
) -> Vec<CurtainPlacement> {
    let mut sorted: Vec<&(String, Kind, Vec2)> = provinces.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::new();
    let mut counter = seed.wrapping_mul(1_000_003);
    for storm_pass in [true, false] {
        let wanted = if storm_pass { Kind::Storm } else { Kind::Rain };
        for (_, kind, centroid) in &sorted {
            if *kind != wanted {
                continue;
            }
            for _ in 0..per_province {
                if out.len() >= max {
                    return out;
                }
                counter += 5;
                let offset = Vec2::new(hash01(counter) - 0.5, hash01(counter + 1) - 0.5) * 36.0;
                out.push(CurtainPlacement {
                    position: *centroid + offset,
                    width: width[0] + (width[1] - width[0]) * hash01(counter + 2),
                    rand: hash01(counter + 3),
                    storm: storm_pass,
                });
            }
        }
    }
    out
}

/// Settings of `data/fx/map_atmosphere.json` (Godot defaults when a key is missing).
#[derive(Resource, Debug, Clone)]
pub struct SkyTuning {
    /// `focus_clear`: radius / feather per camera distance (0 = nothing cleared).
    pub focus_radius_per_distance: f32,
    pub focus_feather_per_distance: f32,
    pub cirrus_enabled: bool,
    pub cirrus_height: f32,
    pub cirrus_opacity: f32,
    pub cirrus_in: [f32; 2],
    pub cirrus_out: [f32; 2],
    pub cirrus_color: [f32; 3],
    pub rain_enabled: bool,
    pub rain_max: usize,
    pub rain_per_province: usize,
    pub rain_height_top: f32,
    pub rain_width: [f32; 2],
    pub rain_opacity: f32,
    pub rain_in: [f32; 2],
    pub rain_out: [f32; 2],
    pub rain_slant: f32,
    pub rain_speed: f32,
    pub rain_color: [f32; 3],
    pub storm_color: [f32; 3],
    pub rain_seed: u64,
    pub fog_enabled: bool,
    pub fog_min_importance: i64,
    pub fog_step_px: f32,
    pub fog_max_banks: usize,
    pub fog_size: [f32; 2],
    pub fog_lift: f32,
    pub fog_opacity: f32,
    pub fog_in: [f32; 2],
    pub fog_out: [f32; 2],
    pub fog_season_share: [f32; 4],
    pub fog_lifted_share: f32,
    pub fog_color: [f32; 3],
    pub fog_seed: u64,
}

impl Default for SkyTuning {
    fn default() -> Self {
        SkyTuning {
            focus_radius_per_distance: 0.0,
            focus_feather_per_distance: 0.2,
            cirrus_enabled: true,
            cirrus_height: 96.0,
            cirrus_opacity: 0.09,
            cirrus_in: [420.0, 800.0],
            cirrus_out: [1000.0, 1250.0],
            cirrus_color: [0.96, 0.97, 1.0],
            rain_enabled: true,
            rain_max: 160,
            rain_per_province: 2,
            rain_height_top: 34.0,
            rain_width: [26.0, 50.0],
            rain_opacity: 0.3,
            rain_in: [190.0, 330.0],
            rain_out: [1000.0, 1250.0],
            rain_slant: 0.18,
            rain_speed: 1.6,
            rain_color: [0.62, 0.67, 0.76],
            storm_color: [0.34, 0.36, 0.43],
            rain_seed: 1,
            fog_enabled: true,
            fog_min_importance: 3,
            fog_step_px: 38.0,
            fog_max_banks: 220,
            fog_size: [20.0, 34.0],
            fog_lift: 1.4,
            fog_opacity: 0.34,
            fog_in: [120.0, 260.0],
            fog_out: [700.0, 950.0],
            fog_season_share: [0.35, 0.1, 0.8, 1.0],
            fog_lifted_share: 0.1,
            fog_color: [0.86, 0.87, 0.86],
            fog_seed: 1,
        }
    }
}

pub(super) fn num(block: &Value, key: &str, default: f32) -> f32 {
    block
        .get(key)
        .and_then(Value::as_f64)
        .map_or(default, |v| v as f32)
}

pub(super) fn arr<const N: usize>(block: &Value, key: &str, default: [f32; N]) -> [f32; N] {
    let Some(values) = block.get(key).and_then(Value::as_array) else {
        return default;
    };
    if values.len() != N {
        return default;
    }
    let mut out = default;
    for (slot, value) in out.iter_mut().zip(values) {
        if let Some(v) = value.as_f64() {
            *slot = v as f32;
        }
    }
    out
}

pub(super) fn flag(block: &Value, key: &str) -> bool {
    block.get(key).and_then(Value::as_bool).unwrap_or(true)
}

impl SkyTuning {
    pub fn from_json(root: &Value) -> Self {
        let d = SkyTuning::default();
        let null = Value::Null;
        let cirrus = root.get("cirrus").unwrap_or(&null);
        let rain = root.get("rain_curtains").unwrap_or(&null);
        let fog = root.get("river_fog").unwrap_or(&null);
        let focus = root.get("focus_clear").unwrap_or(&null);
        SkyTuning {
            focus_radius_per_distance: num(
                focus,
                "radius_per_distance",
                d.focus_radius_per_distance,
            ),
            focus_feather_per_distance: num(
                focus,
                "feather_per_distance",
                d.focus_feather_per_distance,
            ),
            cirrus_enabled: flag(cirrus, "enabled"),
            cirrus_height: num(cirrus, "height", d.cirrus_height),
            cirrus_opacity: num(cirrus, "opacity", d.cirrus_opacity),
            cirrus_in: arr(cirrus, "distance_in", d.cirrus_in),
            cirrus_out: arr(cirrus, "distance_out", d.cirrus_out),
            cirrus_color: arr(cirrus, "color", d.cirrus_color),
            rain_enabled: flag(rain, "enabled"),
            rain_max: num(rain, "max_curtains", d.rain_max as f32) as usize,
            rain_per_province: num(rain, "per_province", d.rain_per_province as f32) as usize,
            rain_height_top: num(rain, "height_top", d.rain_height_top),
            rain_width: arr(rain, "width", d.rain_width),
            rain_opacity: num(rain, "opacity", d.rain_opacity),
            rain_in: arr(rain, "distance", d.rain_in),
            rain_out: arr(rain, "distance_out", d.rain_out),
            rain_slant: num(rain, "slant", d.rain_slant),
            rain_speed: num(rain, "speed", d.rain_speed),
            rain_color: arr(rain, "color", d.rain_color),
            storm_color: arr(rain, "storm_color", d.storm_color),
            rain_seed: num(rain, "seed", d.rain_seed as f32) as u64,
            fog_enabled: flag(fog, "enabled"),
            fog_min_importance: num(fog, "min_importance", d.fog_min_importance as f32) as i64,
            fog_step_px: num(fog, "step_px", d.fog_step_px),
            fog_max_banks: num(fog, "max_banks", d.fog_max_banks as f32) as usize,
            fog_size: arr(fog, "size", d.fog_size),
            fog_lift: num(fog, "lift", d.fog_lift),
            fog_opacity: num(fog, "opacity", d.fog_opacity),
            fog_in: arr(fog, "distance", d.fog_in),
            fog_out: arr(fog, "distance_out", d.fog_out),
            fog_season_share: arr(fog, "season_share", d.fog_season_share),
            fog_lifted_share: num(fog, "lifted_share", d.fog_lifted_share),
            fog_color: arr(fog, "color", d.fog_color),
            fog_seed: num(fog, "seed", d.fog_seed as f32) as u64,
        }
    }
}

// --- procedural textures ---------------------------------------------------------------

pub(super) fn repeat_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    })
}

fn white_image(width: u32, height: u32, alpha: impl Fn(u32, u32) -> f32) -> Image {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let a = (alpha(x, y).clamp(0.0, 1.0) * 255.0) as u8;
            data.extend_from_slice(&[255, 255, 255, a]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = repeat_sampler();
    image
}

/// Tileable value noise on a `cells_x` x `cells_y` lattice (wraps around).
pub(super) fn tile_noise(u: f32, v: f32, cells_x: u32, cells_y: u32, seed: u64) -> f32 {
    let (gx, gy) = (u * cells_x as f32, v * cells_y as f32);
    let (ix, iy) = (gx.floor() as u32, gy.floor() as u32);
    let (fx, fy) = (gx.fract(), gy.fract());
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let lattice = |x: u32, y: u32| {
        hash01(seed + u64::from(x % cells_x) * 7919 + u64::from(y % cells_y) * 104_729)
    };
    let top = lattice(ix, iy) * (1.0 - sx) + lattice(ix + 1, iy) * sx;
    let bottom = lattice(ix, iy + 1) * (1.0 - sx) + lattice(ix + 1, iy + 1) * sx;
    top * (1.0 - sy) + bottom * sy
}

/// Cirrus alpha at texel (u, v): stretched streaks (few cells across, many along).
pub fn cirrus_alpha(u: f32, v: f32) -> f32 {
    let streaks = tile_noise(u, v, 3, 14, 11) * 0.65 + tile_noise(u, v, 6, 28, 23) * 0.35;
    smoothstep(0.45, 0.8, streaks)
}

/// Number of streak columns across a curtain texture (`width_units * 1.3` in the Godot shader,
/// about 45-80 columns on the 34-64 unit curtains).
pub const RAIN_COLUMNS: f32 = 48.0;

/// Rain density at texel (u, v) (v = 0 at the foot): thin streaks (one column each, cut by a
/// noise along the fall) over a slow veil of denser sheets, as the Godot `density` before the
/// vertical / side fades (those live in the curtain mesh's vertex colours, they must not scroll).
pub fn rain_alpha(u: f32, v: f32) -> f32 {
    let scaled = u * RAIN_COLUMNS;
    let column = scaled.floor() as u64;
    let line = 1.0 - (scaled.fract() - 0.5).abs() * 2.0;
    let along = tile_noise(0.0, v, 1, 8, column * 13 + 5);
    let streak = smoothstep(0.35, 1.0, line) * smoothstep(0.45, 0.9, along);
    let veil = 0.35 + 0.65 * tile_noise(u, v, 3, 2, 41);
    (veil * 0.55 + streak * 0.6).clamp(0.0, 1.0)
}

/// Vertical / side envelope of a curtain at (x in -0.5..0.5, y in 0..1): denser towards the
/// cloud base, faded at the top and the sides (`vertical * top_fade * side`).
pub fn curtain_envelope(x: f32, y: f32) -> f32 {
    let vertical = smoothstep(0.0, 0.85, y) * 0.75 + 0.25;
    let top_fade = 1.0 - smoothstep(0.88, 1.0, y);
    let side = smoothstep(0.0, 0.3, 0.5 - x.abs());
    vertical * top_fade * side
}

/// `focus_clear`: 0 in front of the focus point, 1 beyond `radius + feather` (ground distance).
pub fn focus_clear(xz: Vec2, focus: Vec2, radius: f32, feather: f32) -> f32 {
    if radius <= 0.0 {
        return 1.0;
    }
    smoothstep(radius, radius + feather.max(1.0), xz.distance(focus))
}

/// Number of opacity steps of the shared curtain / fog materials (focus clearing is quantised so
/// that a few materials serve every prop).
pub const LEVELS: usize = 8;

/// Curtain quad with the envelope in vertex colours; origin at the bottom centre, facing +Z.
fn curtain_mesh() -> Mesh {
    use bevy::mesh::{Indices, PrimitiveTopology};
    const N: usize = 8;
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    for row in 0..=N {
        for col in 0..=N {
            let (x, y) = (col as f32 / N as f32 - 0.5, row as f32 / N as f32);
            positions.push([x, y, 0.0]);
            uvs.push([x + 0.5, 1.0 - y]);
            colors.push([1.0, 1.0, 1.0, curtain_envelope(x, y)]);
        }
    }
    let mut indices = Vec::new();
    for row in 0..N {
        for col in 0..N {
            let a = (row * (N + 1) + col) as u32;
            let b = a + 1;
            let c = a + (N + 1) as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, d, a, d, c]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 0.0, 1.0]; (N + 1) * (N + 1)],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

// --- scene -------------------------------------------------------------------------------

#[derive(Component)]
struct Cirrus;

/// A curtain (rain) or bank (fog) standing on the ground: re-seated when the relief changes.
#[derive(Component, Debug, Clone)]
struct Grounded {
    xz: Vec2,
    /// Rain: top of the curtain (world y) it spans up to; fog: none.
    top: Option<f32>,
    lift: f32,
    base_width: f32,
    base_height: f32,
}

#[derive(Component)]
struct RainCurtain;

/// Storm curtains use the darker material set.
#[derive(Component)]
struct RainKind {
    storm: bool,
}

#[derive(Component)]
struct FogBank;

#[derive(Resource)]
struct SkyAssets {
    /// Quad whose origin is at the bottom edge, facing +Z.
    curtain: Handle<Mesh>,
    /// Fog quad (plain, no envelope).
    fog_quad: Handle<Mesh>,
    plane: Handle<Mesh>,
    cirrus: Handle<StandardMaterial>,
    /// One material per opacity level (focus clearing), index 0 = invisible.
    rain: Vec<Handle<StandardMaterial>>,
    storm: Vec<Handle<StandardMaterial>>,
    fog: Vec<Handle<StandardMaterial>>,
}

/// Counters exposed by the dump.
#[derive(Resource, Debug, Default)]
pub struct SkyStats {
    pub cirrus_alpha: f32,
    pub curtains: usize,
    pub rain_alpha: f32,
    pub fog_banks: usize,
    pub fog_alpha: f32,
    pub flashes: u32,
    pub flashing: bool,
    pub last_flash_factor: f32,
    /// Smallest focus-clear factor over the props (0 = something is cleared in front of the focus).
    pub min_clear: f32,
}

/// River centre lines (importance, points), read once from the water data.
#[derive(Resource, Debug, Default)]
struct RiverLines(Vec<(i64, Vec<Vec2>)>);

fn setup_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cirrus_image = images.add(white_image(256, 256, |x, y| {
        cirrus_alpha(x as f32 / 256.0, y as f32 / 256.0)
    }));
    let rain_image = images.add(white_image(256, 128, |x, y| {
        rain_alpha(x as f32 / 256.0, y as f32 / 128.0)
    }));
    let fog_image = images.add(white_image(64, 64, |x, y| {
        let (u, v) = (x as f32 / 64.0, y as f32 / 64.0);
        let r = ((u - 0.5).powi(2) + ((v - 0.5) * 1.6).powi(2)).sqrt() * 2.0;
        (1.0 - smoothstep(0.2, 1.0, r)) * (0.6 + 0.5 * tile_noise(u, v, 4, 4, 3))
    }));
    let material = |image: &Handle<Image>, color: Color| StandardMaterial {
        base_color: color,
        base_color_texture: Some(image.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        double_sided: true,
        ..default()
    };
    commands.insert_resource(SkyAssets {
        curtain: meshes.add(curtain_mesh()),
        fog_quad: meshes.add(
            Rectangle::new(1.0, 1.0)
                .mesh()
                .build()
                .translated_by(Vec3::Y * 0.5),
        ),
        plane: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
        cirrus: materials.add(material(&cirrus_image, Color::srgba(1.0, 1.0, 1.0, 0.0))),
        rain: (0..LEVELS)
            .map(|_| materials.add(material(&rain_image, Color::srgba(0.6, 0.65, 0.75, 0.0))))
            .collect(),
        storm: (0..LEVELS)
            .map(|_| materials.add(material(&rain_image, Color::srgba(0.34, 0.36, 0.43, 0.0))))
            .collect(),
        fog: (0..LEVELS)
            .map(|_| materials.add(material(&fog_image, Color::srgba(0.86, 0.87, 0.86, 0.0))))
            .collect(),
    });
}

fn load_tuning(mut tuning: ResMut<SkyTuning>) {
    if let Ok(root) = crate::paths::find_root() {
        let path = crate::paths::data_dir(&root).join("fx/map_atmosphere.json");
        if let Some(value) = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        {
            *tuning = SkyTuning::from_json(&value);
        }
    }
}

fn spawn_cirrus(
    mut commands: Commands,
    tuning: Res<SkyTuning>,
    map: Option<Res<ProvinceMap>>,
    assets: Option<Res<SkyAssets>>,
    existing: Query<(), With<Cirrus>>,
) {
    let (Some(map), Some(assets)) = (map, assets) else {
        return;
    };
    if !tuning.cirrus_enabled || !existing.is_empty() {
        return;
    }
    let (w, h) = (map.width as f32, map.height as f32);
    commands.spawn((
        Name::new("Cirrus"),
        Cirrus,
        Mesh3d(assets.plane.clone()),
        MeshMaterial3d(assets.cirrus.clone()),
        Transform::from_translation(Vec3::new(w * 0.5, tuning.cirrus_height, h * 0.5))
            .with_scale(Vec3::new(w, 1.0, h)),
        Visibility::Hidden,
        NotShadowCaster,
        DespawnOnExit(AppState::Campaign),
    ));
}

/// Reads only `rivers_render.json` (the water plugin parses the whole map directory; loading
/// `WaterData` a second time here cost the coastline / lake / road parsing again).
fn load_rivers(mut rivers: ResMut<RiverLines>) {
    if !rivers.0.is_empty() {
        return;
    }
    if let Ok(root) = crate::paths::find_root() {
        let path = crate::paths::data_dir(&root).join("map/rivers_render.json");
        if let Some(rendered) = std::fs::read(path)
            .ok()
            .and_then(|bytes| data_model::map_geo::parse_rendered_rivers(&bytes).ok())
        {
            rivers.0 = rendered
                .rivers
                .iter()
                .map(|r| {
                    let points = r.points.iter().map(|p| Vec2::new(p[0], p[1])).collect();
                    (r.importance, points)
                })
                .collect();
        }
    }
}

/// Rebuilds the rain curtains when the weather changes, the fog banks once.
#[allow(clippy::too_many_arguments)]
fn sync_props(
    mut commands: Commands,
    weather: Res<CampaignWeather>,
    tuning: Res<SkyTuning>,
    rivers: Res<RiverLines>,
    map: Option<Res<ProvinceMap>>,
    assets: Option<Res<SkyAssets>>,
    curtains: Query<Entity, With<RainCurtain>>,
    banks: Query<(), With<FogBank>>,
    mut built: Local<u64>,
) {
    let (Some(map), Some(assets)) = (map, assets) else {
        return;
    };
    if !tuning.fog_enabled || !banks.is_empty() {
    } else if !rivers.0.is_empty() {
        let points = river_bank_points(
            &rivers.0,
            tuning.fog_min_importance,
            tuning.fog_step_px,
            tuning.fog_max_banks,
        );
        for (i, point) in points.iter().enumerate() {
            let seed = tuning.fog_seed * 977 + i as u64 * 3;
            let width =
                tuning.fog_size[0] + (tuning.fog_size[1] - tuning.fog_size[0]) * hash01(seed);
            commands.spawn((
                Name::new("RiverFog"),
                FogBank,
                Grounded {
                    xz: *point,
                    top: None,
                    lift: tuning.fog_lift,
                    base_width: width,
                    base_height: width * 0.28,
                },
                Mesh3d(assets.fog_quad.clone()),
                MeshMaterial3d(assets.fog[LEVELS - 1].clone()),
                Transform::from_translation(Vec3::new(point.x, tuning.fog_lift, point.y))
                    .with_scale(Vec3::new(width, width * 0.28, 1.0)),
                Visibility::Hidden,
                NotShadowCaster,
                DespawnOnExit(AppState::Campaign),
            ));
        }
    }
    if *built == weather.revision || weather.by_province.is_empty() {
        return;
    }
    *built = weather.revision;
    for entity in &curtains {
        commands.entity(entity).despawn();
    }
    if !tuning.rain_enabled {
        return;
    }
    let provinces: Vec<(String, Kind, Vec2)> = map
        .provinces()
        .iter()
        .map(|f| {
            (
                f.id.clone(),
                weather.entry(&f.id).kind,
                Vec2::from(f.centroid),
            )
        })
        .collect();
    let placements = rain_placements(
        &provinces,
        tuning.rain_per_province,
        tuning.rain_max,
        tuning.rain_width,
        tuning.rain_seed,
    );
    for p in placements {
        commands.spawn((
            Name::new("RainCurtain"),
            RainCurtain,
            Grounded {
                xz: p.position,
                top: Some(tuning.rain_height_top),
                lift: 0.0,
                base_width: p.width,
                base_height: tuning.rain_height_top,
            },
            Mesh3d(assets.curtain.clone()),
            MeshMaterial3d(if p.storm {
                assets.storm[LEVELS - 1].clone()
            } else {
                assets.rain[LEVELS - 1].clone()
            }),
            RainKind { storm: p.storm },
            Transform::from_translation(Vec3::new(p.position.x, 0.0, p.position.y))
                .with_scale(Vec3::new(p.width, tuning.rain_height_top, 1.0)),
            Visibility::Hidden,
            NotShadowCaster,
            DespawnOnExit(AppState::Campaign),
        ));
    }
}

/// Opacity, scrolling and facing of the sky props every frame.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_props(
    time: Res<Time<Real>>,
    view: Res<WeatherView>,
    zoom: Res<ZoomView>,
    tuning: Res<SkyTuning>,
    seasons: Res<SeasonWeights>,
    assets: Option<Res<SkyAssets>>,
    ground: GroundHeight,
    relief: Option<Res<ReliefState>>,
    camera: Query<&Transform, (With<CampaignCamera>, Without<Grounded>, Without<Cirrus>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stats: ResMut<SkyStats>,
    mut cirrus: Query<&mut Visibility, (With<Cirrus>, Without<Grounded>)>,
    mut props: Query<(
        &Grounded,
        &mut Transform,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
        Option<&RainCurtain>,
        Option<&FogBank>,
        Option<&RainKind>,
    )>,
    mut seated_for: Local<f32>,
) {
    let Some(assets) = assets else {
        return;
    };
    let clear = 1.0 - zoom.strategic;
    let distance = view.distance;
    let seconds = time.elapsed_secs();

    let cirrus_alpha =
        tuning.cirrus_opacity * band_fade(distance, tuning.cirrus_in, tuning.cirrus_out) * clear;
    let rain_alpha =
        tuning.rain_opacity * band_fade(distance, tuning.rain_in, tuning.rain_out) * clear;
    let morning = view.morning_mist;
    let lifted = tuning.fog_lifted_share;
    let fog_alpha = tuning.fog_opacity
        * band_fade(distance, tuning.fog_in, tuning.fog_out)
        * clear
        * (lifted + (1.0 - lifted) * morning)
        * season_share(seasons.weights, tuning.fog_season_share);

    if let Some(mut m) = materials.get_mut(&assets.cirrus) {
        let c = tuning.cirrus_color;
        m.base_color = Color::srgba(c[0], c[1], c[2], cirrus_alpha);
        m.uv_transform = Affine2::from_scale_angle_translation(
            Vec2::new(1.0, 1.0),
            0.0,
            Vec2::new(seconds * 0.0006, seconds * 0.0002),
        );
    }
    for (handles, color, storm_factor) in [
        (&assets.rain, tuning.rain_color, 1.0),
        (&assets.storm, tuning.storm_color, 1.3),
    ] {
        for (level, handle) in handles.iter().enumerate() {
            if let Some(mut m) = materials.get_mut(handle) {
                let share = level as f32 / (LEVELS - 1) as f32;
                m.base_color = Color::srgba(
                    color[0],
                    color[1],
                    color[2],
                    (rain_alpha * storm_factor * share).clamp(0.0, 1.0),
                );
                // uv' = (u + slant * v, v - t * speed): slanted streaks falling down.
                m.uv_transform = Affine2::from_cols(
                    Vec2::new(1.0, 0.0),
                    Vec2::new(tuning.rain_slant, 1.0),
                    Vec2::new(0.0, seconds * tuning.rain_speed),
                );
            }
        }
    }
    for (level, handle) in assets.fog.iter().enumerate() {
        if let Some(mut m) = materials.get_mut(handle) {
            let c = tuning.fog_color;
            let share = level as f32 / (LEVELS - 1) as f32;
            m.base_color = Color::srgba(c[0], c[1], c[2], (fog_alpha * share).clamp(0.0, 1.0));
        }
    }
    for mut visibility in &mut cirrus {
        let want = if cirrus_alpha > 0.002 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }

    let scale = relief.as_ref().map_or(0.0, |r| r.vertical_scale);
    let reseat = ground.is_ready() && (*seated_for - scale).abs() > scale.abs() * 0.005 + 1e-6;
    if reseat {
        *seated_for = scale;
    }
    let cam = camera.single().map(|t| t.translation).ok();
    stats.cirrus_alpha = cirrus_alpha;
    stats.rain_alpha = rain_alpha;
    stats.fog_alpha = fog_alpha;
    stats.curtains = 0;
    stats.fog_banks = 0;
    let radius = distance * tuning.focus_radius_per_distance;
    let feather = (distance * tuning.focus_feather_per_distance).max(1.0);
    let focus_xz = Vec2::new(view.focus.x, view.focus.z);
    stats.min_clear = 1.0;
    for (grounded, mut transform, mut visibility, mut material, curtain, bank, kind) in &mut props {
        if curtain.is_some() {
            stats.curtains += 1;
        }
        if bank.is_some() {
            stats.fog_banks += 1;
        }
        let alpha = if curtain.is_some() {
            rain_alpha
        } else {
            fog_alpha
        };
        let clear_here = focus_clear(grounded.xz, focus_xz, radius, feather);
        stats.min_clear = stats.min_clear.min(clear_here);
        let level = (clear_here * (LEVELS - 1) as f32).round() as usize;
        let want = if alpha > 0.004 && level > 0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        let handles = if bank.is_some() {
            &assets.fog
        } else if kind.is_some_and(|k| k.storm) {
            &assets.storm
        } else {
            &assets.rain
        };
        if material.0 != handles[level] {
            material.0 = handles[level].clone();
        }
        if reseat {
            let y = ground.at(grounded.xz.x, grounded.xz.y).unwrap_or(0.0);
            transform.translation = Vec3::new(grounded.xz.x, y + grounded.lift, grounded.xz.y);
            transform.scale = match grounded.top {
                Some(top) => Vec3::new(grounded.base_width, (top - y).max(8.0), 1.0),
                None => Vec3::new(grounded.base_width, grounded.base_height, 1.0),
            };
        }
        if want == Visibility::Inherited
            && let Some(cam) = cam
        {
            let to_cam = cam - transform.translation;
            transform.rotation = Quat::from_rotation_y(to_cam.x.atan2(to_cam.z));
        }
    }
}

/// Storm lightning: while the camera is near a stormy focus, the sun light flashes (two-level
/// flicker) every 3-9 s, like `_update_lightning`.
#[derive(Default)]
struct LightningState {
    timer: f32,
    next: f32,
    base: f32,
    draws: u64,
}

fn update_lightning(
    time: Res<Time<Real>>,
    view: Res<WeatherView>,
    mut stats: ResMut<SkyStats>,
    mut sun: Query<&mut DirectionalLight, With<CampaignSun>>,
    mut state: Local<LightningState>,
) {
    let Ok(mut light) = sun.single_mut() else {
        return;
    };
    let delta = time.delta_secs();
    if state.timer > 0.0 {
        state.timer -= delta;
        if state.timer <= 0.0 {
            light.illuminance = state.base;
            stats.flashing = false;
            stats.last_flash_factor = 1.0;
        } else {
            let factor = flash_factor(state.timer);
            light.illuminance = state.base * factor;
            stats.last_flash_factor = factor;
        }
        return;
    }
    if view.focus_entry.kind != Kind::Storm || view.distance > LIGHTNING_MAX_DISTANCE {
        return;
    }
    state.next -= delta;
    if state.next <= 0.0 {
        state.draws += 1;
        state.next = 3.0 + 6.0 * hash01(state.draws * 7 + 1);
        state.base = light.illuminance;
        state.timer = FLASH_SECONDS;
        stats.flashes += 1;
        stats.flashing = true;
    }
}

fn reset(mut stats: ResMut<SkyStats>) {
    *stats = SkyStats::default();
}

pub fn dump(world: &World) -> Value {
    let s = world.resource::<SkyStats>();
    json!({
        "cirrus_alpha": s.cirrus_alpha,
        "rain_curtains": s.curtains, "rain_alpha": s.rain_alpha,
        "fog_banks": s.fog_banks, "fog_alpha": s.fog_alpha,
        "lightning_flashes": s.flashes, "flashing": s.flashing,
        "min_focus_clear": s.min_clear,
    })
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<SkyTuning>()
        .init_resource::<SkyStats>()
        .init_resource::<RiverLines>()
        .add_systems(Startup, setup_assets)
        .add_systems(
            OnEnter(AppState::Campaign),
            (load_tuning, reset, load_rivers).chain(),
        )
        .add_systems(
            Update,
            (spawn_cirrus, sync_props, update_props, update_lightning)
                .chain()
                .after(super::update_view)
                .run_if(in_state(AppState::Campaign)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_fade_rises_then_falls() {
        let (i, o) = ([100.0, 200.0], [500.0, 600.0]);
        assert_eq!(band_fade(50.0, i, o), 0.0);
        assert_eq!(band_fade(300.0, i, o), 1.0);
        assert_eq!(band_fade(700.0, i, o), 0.0);
        assert!(band_fade(150.0, i, o) > 0.0 && band_fade(150.0, i, o) < 1.0);
    }

    #[test]
    fn season_share_blends_weights() {
        let shares = [0.35, 0.1, 0.8, 1.0];
        assert!((season_share(Vec4::new(0.0, 0.0, 0.0, 1.0), shares) - 1.0).abs() < 1e-6);
        assert!((season_share(Vec4::new(0.0, 1.0, 0.0, 0.0), shares) - 0.1).abs() < 1e-6);
    }

    #[test]
    fn bank_points_follow_step_and_cap() {
        let line: Vec<Vec2> = (0..100).map(|i| Vec2::new(i as f32 * 10.0, 0.0)).collect();
        let rivers = vec![(5, line.clone()), (1, line)];
        let points = river_bank_points(&rivers, 3, 38.0, 1000);
        assert!(points.len() > 15 && points.len() < 30);
        assert_eq!(river_bank_points(&rivers, 3, 38.0, 5).len(), 5);
        assert!(river_bank_points(&rivers, 3, 38.0, 0).is_empty());
        assert!(river_bank_points(&rivers, 9, 38.0, 100).is_empty());
    }

    #[test]
    fn curtains_storms_first_and_capped() {
        let provinces = vec![
            ("b".to_owned(), Kind::Rain, Vec2::new(10.0, 10.0)),
            ("a".to_owned(), Kind::Storm, Vec2::new(500.0, 500.0)),
            ("c".to_owned(), Kind::Clear, Vec2::new(0.0, 0.0)),
        ];
        let all = rain_placements(&provinces, 2, 160, [26.0, 50.0], 77);
        assert_eq!(all.len(), 4);
        assert!(all[0].storm && all[1].storm && !all[2].storm);
        assert!(all.iter().all(|p| (26.0..=50.0).contains(&p.width)));
        assert_eq!(all, rain_placements(&provinces, 2, 160, [26.0, 50.0], 77));
        assert_eq!(rain_placements(&provinces, 2, 3, [26.0, 50.0], 77).len(), 3);
    }

    #[test]
    fn flash_flickers_then_ends() {
        assert_eq!(flash_factor(0.0), 1.0);
        assert_eq!(flash_factor(0.06), 2.6);
        assert_eq!(flash_factor(0.02), 1.4);
    }

    #[test]
    fn tuning_reads_the_godot_data_file() {
        let json: Value = serde_json::from_str(
            r#"{"cirrus":{"opacity":0.2,"distance_in":[1,2]},"rain_curtains":{"seed":77,"enabled":false}}"#,
        )
        .unwrap();
        let t = SkyTuning::from_json(&json);
        assert_eq!(t.cirrus_opacity, 0.2);
        assert_eq!(t.cirrus_in, [1.0, 2.0]);
        assert_eq!(t.rain_seed, 77);
        assert!(!t.rain_enabled && t.fog_enabled);
        assert_eq!(t.fog_max_banks, 220);
    }

    #[test]
    fn textures_have_content() {
        let cirrus: f32 = (0..256).map(|i| cirrus_alpha(i as f32 / 256.0, 0.37)).sum();
        assert!(cirrus > 1.0);
        let rain: f32 = (0..128).map(|i| rain_alpha(0.5, i as f32 / 128.0)).sum();
        assert!(rain > 1.0);
        // the envelope kills the sides and the top, keeps the middle
        assert_eq!(curtain_envelope(0.5, 0.5), 0.0);
        assert!(curtain_envelope(0.0, 0.6) > 0.5 && curtain_envelope(0.0, 1.0) == 0.0);
    }

    #[test]
    fn focus_clear_opens_a_hole_around_the_focus() {
        let focus = Vec2::new(100.0, 100.0);
        assert_eq!(focus_clear(focus, focus, 50.0, 20.0), 0.0);
        assert_eq!(focus_clear(Vec2::new(400.0, 100.0), focus, 50.0, 20.0), 1.0);
        let mid = focus_clear(Vec2::new(160.0, 100.0), focus, 50.0, 20.0);
        assert!(mid > 0.0 && mid < 1.0);
        assert_eq!(focus_clear(focus, focus, 0.0, 20.0), 1.0);
    }
}
