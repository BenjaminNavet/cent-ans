//! Battle camera lighting: exposure, AgX tonemapping, bloom, SSAO, colour grading and the sky
//! light of the HDRI panorama (port of the environment half of game/scripts/battle/battle_atmosphere.gd,
//! of `AtmosphereLibrary.apply_to_environment` and of game/shaders/hdri_sky.gdshader).
//!
//! Godot lit the battle with a sky-sourced ambient: the Poly Haven panorama of the weather /
//! season / hour (`data/fx/atmosphere.json`, `battle.<weather>.sky.<season>`, overridden by
//! `time_of_day.<key>.sky`), turned so that its painted sun falls on the sun, exposed by
//! `sky_luminance / mean`, clipped, desaturated, hazed at the horizon and faded to a ground colour
//! below it. Here the same panorama is resampled on the CPU into a small cubemap that
//! `GeneratedEnvironmentMapLight` filters into diffuse + specular light; a neutral share
//! (`1 - sky_contribution`, fog colour lightened) goes to a per-camera `AmbientLight`.
//!
//! Units: the decor sun is `energy * 8000 lux`; `exposure_ev100` 11.05 makes one Godot energy unit
//! light a white Lambert surface to 1.0, as in Godot (which scales light energy by pi). The sky
//! light intensities are therefore multiplied by `1.2 * 2^ev100` (the inverse of the exposure).
//!
//! Grading: Godot baked a 3D LUT (season, weather and hour grades, `GradeLut` in core). Bevy's
//! `ColorGrading` takes the product of their contrast and saturation and the summed temperature
//! and tint; per-channel lift / gain / shadow and highlight toning are not ported.
//!
//! SSAO needs `Msaa::Off` in Bevy: it is on only when the player's battle MSAA is off (the
//! settings lot owns `Msaa`). The instanced soldiers and plants are drawn after the prepass, so
//! they neither occlude nor receive it.

use bevy::anti_alias::smaa::Smaa;
use bevy::asset::RenderAssetUsages;
use bevy::camera::{Exposure, Hdr};
use bevy::light::{AmbientLight, GeneratedEnvironmentMapLight};
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel};
use bevy::post_process::bloom::{Bloom, BloomPrefilter};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection, Tonemapping};
use serde::Deserialize;
use sim_battle::{BattleSeason, Weather};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::decor::atmosphere::AtmosphereState;
use crate::battle::decor::look::DecorLook;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::paths;

const DATA_FILE: &str = "fx/battle_lighting.json";

#[derive(Debug, Clone, Deserialize)]
pub struct BloomLook {
    pub enabled: bool,
    pub intensity: f32,
    pub threshold: f32,
    pub threshold_softness: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SsaoLook {
    pub enabled: bool,
    pub quality: String,
    pub object_thickness: f32,
    pub radius: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkyLightLook {
    pub face_px: u32,
    pub ambient_boost: f32,
    pub sky_contribution: f32,
    pub ambient_lighten: f32,
    pub max_luminance: f32,
    pub ground_darken: f32,
    pub horizon_floor: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GradeLook {
    pub temperature_scale: f32,
    pub strength: f32,
}

/// `data/fx/battle_lighting.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
pub struct LightingLook {
    pub exposure_ev100: f32,
    pub tonemapping: String,
    pub bloom: BloomLook,
    pub ssao: SsaoLook,
    pub sky_light: SkyLightLook,
    pub grade: GradeLook,
}

impl LightingLook {
    pub fn load() -> Option<LightingLook> {
        let root = paths::find_root().ok()?;
        let bytes = std::fs::read(paths::data_dir(&root).join(DATA_FILE)).ok()?;
        match serde_json::from_slice(&bytes) {
            Ok(look) => Some(look),
            Err(error) => {
                error!("{DATA_FILE}: {error}");
                None
            }
        }
    }

    /// Inverse of the camera exposure: multiplies light given in "Godot units" (radiance 1.0).
    pub fn unit(&self) -> f32 {
        1.2 * 2f32.powf(self.exposure_ev100)
    }

    fn tonemapping(&self) -> Tonemapping {
        match self.tonemapping.as_str() {
            "tony_mc_mapface" => Tonemapping::TonyMcMapface,
            "aces" => Tonemapping::AcesFitted,
            "reinhard" => Tonemapping::ReinhardLuminance,
            "none" => Tonemapping::None,
            _ => Tonemapping::AgX,
        }
    }
}

/// Marks the battle camera once its post-processing is set up.
#[derive(Component)]
struct Lit;

/// What the sky light currently is (read by the dump).
#[derive(Resource, Debug, Clone, Default)]
pub struct SkyLightState {
    pub sky_id: String,
    pub path: String,
    /// `(time_key, weather)` the cubemap was built for.
    pub built_for: Option<(String, String)>,
    pub cube: Option<Handle<Image>>,
    pub panorama: Option<Handle<Image>>,
    pub env_intensity: f32,
    pub ambient_brightness: f32,
    pub mean_radiance: [f32; 3],
    pub contrast: f32,
    pub saturation: f32,
    pub temperature: f32,
    pub ssao: bool,
    pub missing: bool,
}

pub struct BattleLightingPlugin;

impl Plugin for BattleLightingPlugin {
    fn build(&self, app: &mut App) {
        app.register_dump(dump)
            .add_systems(OnEnter(AppState::Battle), init_lighting)
            .add_systems(
                Update,
                (
                    light_camera,
                    ssao_follows_msaa,
                    choose_sky,
                    build_sky_light,
                    shadow_probe,
                )
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<LightingLook>),
            )
            .add_systems(OnExit(AppState::Battle), clear_lighting);
    }
}

fn init_lighting(mut commands: Commands) {
    if let Some(look) = LightingLook::load() {
        commands.insert_resource(look);
    }
    commands.insert_resource(SkyLightState::default());
}

fn clear_lighting(mut commands: Commands) {
    commands.remove_resource::<SkyLightState>();
}

fn weather_of(key: &str) -> Weather {
    match key {
        "rain" => Weather::Rain,
        "fog" => Weather::Fog,
        "snow" => Weather::Snow,
        _ => Weather::Clear,
    }
}

fn season_key(season: BattleSeason) -> &'static str {
    match season {
        BattleSeason::Spring => "spring",
        BattleSeason::Summer => "summer",
        BattleSeason::Autumn => "autumn",
        BattleSeason::Winter => "winter",
    }
}

fn num(value: &serde_json::Value, default: f32) -> f32 {
    value.as_f64().map_or(default, |v| v as f32)
}

/// Grades applied in order by Godot: season (`battle_seasons`, else `seasons`), weather, hour.
pub fn grades<'a>(
    rules: &'a serde_json::Value,
    weather: &str,
    season: &str,
    time_key: &str,
) -> Vec<&'a serde_json::Value> {
    let season_grade = if rules["battle_seasons"][season]["grade"].is_object() {
        &rules["battle_seasons"][season]["grade"]
    } else {
        &rules["seasons"][season]
    };
    vec![
        season_grade,
        &rules["battle"][weather]["grade"],
        &rules["time_of_day"][time_key]["grade"],
    ]
}

/// Contrast and saturation multiplied, temperature and tint summed over the grades.
pub fn combined_grade(grades: &[&serde_json::Value]) -> (f32, f32, f32, f32) {
    let (mut contrast, mut saturation, mut temperature, mut tint) = (1.0, 1.0, 0.0, 0.0);
    for grade in grades {
        contrast *= num(&grade["contrast"], 1.0);
        saturation *= num(&grade["saturation"], 1.0);
        temperature += num(&grade["temperature"], 0.0);
        tint += num(&grade["tint"], 0.0);
    }
    (contrast, saturation, temperature, tint)
}

/// Sky id of a battle: `time_of_day.<key>.sky.<weather>.<season>`, else `battle.<weather>.sky.<season>`.
pub fn sky_id(rules: &serde_json::Value, weather: &str, season: &str, time_key: &str) -> String {
    let by_hour = &rules["time_of_day"][time_key]["sky"][weather][season];
    let id = by_hour
        .as_str()
        .or_else(|| rules["battle"][weather]["sky"][season].as_str())
        .unwrap_or("");
    if rules["skies"][id].is_object() {
        id.to_owned()
    } else {
        rules["battle"][weather]["sky"][season]
            .as_str()
            .unwrap_or("")
            .to_owned()
    }
}

#[allow(clippy::too_many_arguments)]
fn light_camera(
    mut commands: Commands,
    look: Res<LightingLook>,
    cameras: Query<Entity, (With<BattleCamera>, Without<Lit>)>,
) {
    for camera in &cameras {
        let mut entity = commands.entity(camera);
        entity.insert((
            Lit,
            look.tonemapping(),
            Exposure {
                ev100: look.exposure_ev100,
            },
            ColorGrading::default(),
        ));
        // Bloom needs an HDR view; the HUD camera draws over the battle camera on the same
        // target with `ClearColorConfig::None`, which only composes when both have the same HDR
        // setting (a lone HDR battle camera comes out black). Off unless the data asks for it.
        if look.bloom.enabled {
            entity.insert((
                Hdr,
                Bloom {
                    intensity: look.bloom.intensity,
                    prefilter: BloomPrefilter {
                        threshold: look.bloom.threshold,
                        threshold_softness: look.bloom.threshold_softness,
                    },
                    ..Bloom::NATURAL
                },
            ));
        }
    }
}

/// SSAO (and SMAA in place of the MSAA it excludes) only while the battle MSAA is off.
fn ssao_follows_msaa(
    mut commands: Commands,
    look: Res<LightingLook>,
    mut state: ResMut<SkyLightState>,
    cameras: Query<(Entity, &Msaa, Has<ScreenSpaceAmbientOcclusion>), With<Lit>>,
) {
    for (camera, msaa, has_ssao) in &cameras {
        let want = look.ssao.enabled && *msaa == Msaa::Off;
        if want && !has_ssao {
            let quality_level = match look.ssao.quality.as_str() {
                "low" => ScreenSpaceAmbientOcclusionQualityLevel::Low,
                "high" => ScreenSpaceAmbientOcclusionQualityLevel::High,
                "ultra" => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
                _ => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            };
            commands.entity(camera).insert((
                ScreenSpaceAmbientOcclusion {
                    quality_level,
                    constant_object_thickness: look.ssao.object_thickness,
                    radius: look.ssao.radius,
                },
                Smaa::default(),
            ));
        } else if !want && has_ssao {
            commands
                .entity(camera)
                .remove::<(ScreenSpaceAmbientOcclusion, Smaa)>();
        }
        state.ssao = want;
    }
}

/// Picks the panorama of the weather / season / hour and starts loading it; sets the grading.
fn choose_sky(
    asset_server: Res<AssetServer>,
    look: Res<LightingLook>,
    atmosphere: Option<Res<AtmosphereState>>,
    mut session: ResMut<BattleSession>,
    mut state: ResMut<SkyLightState>,
    mut gradings: Query<&mut ColorGrading, With<Lit>>,
) {
    let Some(atmosphere) = atmosphere else {
        return;
    };
    let key = (atmosphere.time_key.clone(), atmosphere.weather.clone());
    if state.built_for.as_ref() == Some(&key) && state.panorama.is_some() || state.missing {
        // Grading may still need applying to a camera spawned later.
        apply_grade(&look, &state, &mut gradings);
        return;
    }
    let season = season_key(session.sim_mut().field().season);
    let rules = &atmosphere.rules;
    let id = sky_id(rules, &key.1, season, &key.0);
    let (contrast, saturation, temperature, tint) =
        combined_grade(&grades(rules, &key.1, season, &key.0));
    let s = look.grade.strength;
    state.contrast = 1.0 + (contrast - 1.0) * s;
    state.saturation = 1.0 + (saturation - 1.0) * s;
    state.temperature = temperature * look.grade.temperature_scale * s;
    let _ = tint;
    apply_grade(&look, &state, &mut gradings);
    if id == state.sky_id && state.panorama.is_some() {
        // Same panorama, new light (hour or weather): rebuild the cube only.
        state.built_for = None;
        return;
    }
    let path = rules["skies"][id.as_str()]["path"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches("res://assets/")
        .to_owned();
    state.sky_id = id;
    if path.is_empty() {
        state.missing = true;
        return;
    }
    state.panorama = Some(asset_server.load(path.clone()));
    state.path = path;
    state.built_for = None;
}

fn apply_grade(
    look: &LightingLook,
    state: &SkyLightState,
    gradings: &mut Query<&mut ColorGrading, With<Lit>>,
) {
    let _ = look;
    for mut grading in gradings.iter_mut() {
        let wanted = ColorGrading::with_identical_sections(
            ColorGradingGlobal {
                temperature: state.temperature,
                ..default()
            },
            ColorGradingSection {
                saturation: state.saturation,
                contrast: state.contrast,
                ..default()
            },
        );
        if grading.global.temperature != wanted.global.temperature
            || grading.midtones.contrast != wanted.midtones.contrast
            || grading.midtones.saturation != wanted.midtones.saturation
        {
            *grading = wanted;
        }
    }
}

/// Resamples the loaded panorama into a cubemap and puts the sky light on the camera.
#[allow(clippy::too_many_arguments)]
fn build_sky_light(
    mut commands: Commands,
    look: Res<LightingLook>,
    decor: Option<Res<DecorLook>>,
    atmosphere: Option<Res<AtmosphereState>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<SkyLightState>,
    cameras: Query<Entity, With<Lit>>,
) {
    let (Some(atmosphere), Some(decor)) = (atmosphere, decor) else {
        return;
    };
    let key = (atmosphere.time_key.clone(), atmosphere.weather.clone());
    if state.built_for.as_ref() == Some(&key) {
        return;
    }
    let Some(panorama) = state.panorama.as_ref().and_then(|h| images.get(h.id())) else {
        return;
    };
    let Some(pano) = Panorama::from_image(panorama) else {
        error!("battle sky {}: unsupported panorama format", state.path);
        state.missing = true;
        return;
    };
    let rules = &atmosphere.rules;
    let weather = weather_of(&key.1);
    let wx = &rules["battle"][key.1.as_str()];
    let sky = &rules["skies"][state.sky_id.as_str()];
    let to_sun = -Vec3::from_array(atmosphere.sun_direction);
    let fog = decor.fog_of(weather);
    let horizon = decor.sky_of(weather).horizon;
    let sl = &look.sky_light;
    let darken = |c: f32| c * (1.0 - sl.ground_darken);
    let params = SkyParams {
        exposure: num(&wx["sky_luminance"], 0.6) / num(&sky["mean"], 1.0).max(0.05),
        saturation: num(&wx["sky_saturation"], 1.0),
        tint: srgb3(rgb_of(&wx["sky_tint"])),
        haze_amount: num(&wx["haze"], 0.35),
        haze_color: srgb3(fog.color),
        ground_color: srgb3([darken(horizon[0]), darken(horizon[1]), darken(horizon[2])]),
        max_luminance: sl.max_luminance,
        horizon_floor: sl.horizon_floor,
        yaw: to_sun.x.atan2(-to_sun.z) - num(&sky["sun_azimuth_deg"], 0.0).to_radians(),
    };
    let size = sl.face_px.max(16);
    let (cube, mean) = cube_image(&pano, &params, size);
    let handle = images.add(cube);
    let energy = fog.ambient * sl.ambient_boost;
    let env_intensity = look.unit() * energy * sl.sky_contribution;
    let l = sl.ambient_lighten;
    let lighten = |c: f32| c + (1.0 - c) * l;
    let ambient = AmbientLight {
        color: Color::srgb(
            lighten(fog.color[0]),
            lighten(fog.color[1]),
            lighten(fog.color[2]),
        ),
        brightness: look.unit() * energy * (1.0 - sl.sky_contribution),
        ..default()
    };
    for camera in &cameras {
        commands.entity(camera).insert((
            GeneratedEnvironmentMapLight {
                environment_map: handle.clone(),
                intensity: env_intensity,
                ..default()
            },
            ambient.clone(),
        ));
    }
    if cameras.is_empty() {
        return;
    }
    state.cube = Some(handle);
    state.env_intensity = env_intensity;
    state.ambient_brightness = ambient.brightness;
    state.mean_radiance = mean.to_array();
    state.built_for = Some(key);
}

fn rgb_of(value: &serde_json::Value) -> [f32; 3] {
    [
        num(&value[0], 1.0),
        num(&value[1], 1.0),
        num(&value[2], 1.0),
    ]
}

/// sRGB triple to linear (Godot `source_color` uniforms).
fn srgb3(c: [f32; 3]) -> Vec3 {
    let lin = Color::srgb(c[0], c[1], c[2]).to_linear();
    Vec3::new(lin.red, lin.green, lin.blue)
}

/// Parameters of `hdri_sky.gdshader` (linear colours).
#[derive(Debug, Clone, Copy)]
pub struct SkyParams {
    pub exposure: f32,
    pub saturation: f32,
    pub tint: Vec3,
    pub haze_amount: f32,
    pub haze_color: Vec3,
    pub ground_color: Vec3,
    pub max_luminance: f32,
    pub horizon_floor: f32,
    /// Panorama turn (radians) so that its painted sun falls on the light.
    pub yaw: f32,
}

/// An equirectangular RGB panorama in linear floats.
pub struct Panorama {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<[f32; 3]>,
}

impl Panorama {
    fn from_image(image: &Image) -> Option<Panorama> {
        let data = image.data.as_ref()?;
        let width = image.width() as usize;
        let height = image.height() as usize;
        let read =
            |at: usize| f32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
        let rgb = match image.texture_descriptor.format {
            TextureFormat::Rgba32Float => (0..width * height)
                .map(|i| [read(i * 16), read(i * 16 + 4), read(i * 16 + 8)])
                .collect(),
            _ => return None,
        };
        Some(Panorama { width, height, rgb })
    }

    /// Bilinear sample at `(u, v)` in `0..1` (u repeats, v clamps).
    pub fn sample(&self, u: f32, v: f32) -> Vec3 {
        let x = u * self.width as f32 - 0.5;
        let y = (v * self.height as f32 - 0.5).clamp(0.0, (self.height - 1) as f32);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let w = self.width as i64;
        let xi = |x: f32| (x as i64).rem_euclid(w) as usize;
        let yi = |y: f32| (y as usize).min(self.height - 1);
        let at = |x: usize, y: usize| Vec3::from_array(self.rgb[y * self.width + x]);
        let (xa, xb, ya, yb) = (xi(x0), xi(x0 + 1.0), yi(y0), yi(y0 + 1.0));
        let top = at(xa, ya).lerp(at(xb, ya), fx);
        let bottom = at(xa, yb).lerp(at(xb, yb), fx);
        top.lerp(bottom, fy)
    }
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Radiance of the sky in world direction `dir` (port of `sky()` in hdri_sky.gdshader, cubemap
/// pass: no sun disc).
pub fn sky_radiance(pano: &Panorama, p: &SkyParams, dir: Vec3) -> Vec3 {
    let h = dir.y;
    let sky_dir = Vec3::new(dir.x, h.max(p.horizon_floor), dir.z).normalize();
    let azimuth = sky_dir.x.atan2(-sky_dir.z) - p.yaw;
    let u = (azimuth / std::f32::consts::TAU + 0.5).rem_euclid(1.0);
    let v = sky_dir.y.clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
    let mut col = pano.sample(u, v);
    let peak = col.max_element() * p.exposure;
    if peak > p.max_luminance {
        col *= p.max_luminance / peak;
    }
    let grey = col.dot(Vec3::new(0.2126, 0.7152, 0.0722));
    col = Vec3::splat(grey).lerp(col, p.saturation) * p.tint;
    let band = (1.0 - h.clamp(0.0, 1.0)).powf(8.0);
    let haze = p.haze_color * grey.max(0.2) / p.haze_color.dot(Vec3::splat(0.333)).max(0.05);
    col = col.lerp(haze, band * p.haze_amount);
    if h < 0.0 {
        col = col.lerp(p.ground_color * grey.max(0.3), smoothstep(0.0, -0.12, h));
    }
    col * p.exposure
}

/// World direction of texel `(x, y)` of cube face `face` (wgpu face order +X -X +Y -Y +Z -Z;
/// Bevy samples cubemaps with z negated, so the face direction's z is flipped back).
pub fn cube_dir(face: usize, x: u32, y: u32, size: u32) -> Vec3 {
    let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
    let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
    let d = match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    let d = d.normalize();
    Vec3::new(d.x, d.y, -d.z)
}

/// IEEE half of a finite non-negative float (enough for sky radiance).
pub fn f16_bits(value: f32) -> u16 {
    let v = value.clamp(0.0, 65000.0);
    if v < 6.1e-5 {
        return (v / 5.96e-8).round() as u16;
    }
    let bits = v.to_bits();
    let exp = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mant = (bits >> 13) & 0x3ff;
    ((exp as u32) << 10 | mant) as u16
}

/// Six-face Rgba16Float cubemap of the sky and its mean radiance.
fn cube_image(pano: &Panorama, params: &SkyParams, size: u32) -> (Image, Vec3) {
    let mut bytes = Vec::with_capacity((size * size * 6 * 8) as usize);
    let mut sum = Vec3::ZERO;
    for face in 0..6 {
        for y in 0..size {
            for x in 0..size {
                let col = sky_radiance(pano, params, cube_dir(face, x, y, size));
                sum += col;
                for c in [col.x, col.y, col.z, 1.0] {
                    bytes.extend_from_slice(&f16_bits(c).to_le_bytes());
                }
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    (image, sum / (size * size * 6) as f32)
}

/// Dev hook: `CENT_ANS_SHADOW_PROBE=1` hangs a 3 m cube 4 m above the camera pivot, once, to check
/// the cast shadows in proof shots.
fn shadow_probe(
    mut commands: Commands,
    mut done: Local<bool>,
    camera: Option<Res<crate::battle::scene::camera::BattleCameraState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *done || std::env::var("CENT_ANS_SHADOW_PROBE").is_err() {
        return;
    }
    let Some(camera) = camera else {
        return;
    };
    *done = true;
    commands.spawn((
        Name::new("ShadowProbe"),
        Mesh3d(meshes.add(Cuboid::new(3.0, 3.0, 3.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        Transform::from_translation(camera.target + Vec3::new(0.0, 5.5, 0.0)),
        DespawnOnExit(AppState::Battle),
    ));
    // A soldier-sized pole beside it (the thin-caster case).
    commands.spawn((
        Name::new("ShadowProbePole"),
        Mesh3d(meshes.add(Cuboid::new(0.4, 1.8, 0.4))),
        MeshMaterial3d(materials.add(Color::srgb(0.9, 0.9, 0.9))),
        Transform::from_translation(camera.target + Vec3::new(-6.0, 0.9, 0.0)),
        DespawnOnExit(AppState::Battle),
    ));
}

/// Dump section `battle_lighting`.
pub fn dump(world: &World) -> (String, serde_json::Value) {
    let state = world.get_resource::<SkyLightState>();
    let look = world.get_resource::<LightingLook>();
    let mut cameras = 0;
    let mut env = 0;
    let mut ssao = 0;
    for entity in world.iter_entities() {
        if entity.contains::<Lit>() {
            cameras += 1;
            env += usize::from(entity.contains::<GeneratedEnvironmentMapLight>());
            ssao += usize::from(entity.contains::<ScreenSpaceAmbientOcclusion>());
        }
    }
    (
        "battle_lighting".to_owned(),
        serde_json::json!({
            "loaded": look.is_some(),
            "exposure_ev100": look.map(|l| l.exposure_ev100),
            "tonemapping": look.map(|l| l.tonemapping.clone()),
            "lit_cameras": cameras,
        "shadow_unclipped_depth": crate::battle::units::skinned::UNCLIPPED_DEPTH.load(std::sync::atomic::Ordering::Relaxed),
        "shadow_draws_total": crate::battle::units::skinned::SHADOW_DRAWS.load(std::sync::atomic::Ordering::Relaxed),
        "shadow_casters": crate::battle::units::skinned::SHADOW_CASTERS.load(std::sync::atomic::Ordering::Relaxed),
            "cameras_with_sky_light": env,
            "cameras_with_ssao": ssao,
            "sky_id": state.map(|s| s.sky_id.clone()),
            "sky_built": state.is_some_and(|s| s.built_for.is_some()),
            "sky_missing": state.is_some_and(|s| s.missing),
            "env_intensity": state.map(|s| s.env_intensity),
            "ambient_brightness": state.map(|s| s.ambient_brightness),
            "mean_radiance": state.map(|s| s.mean_radiance),
            "grade": state.map(|s| serde_json::json!({ "contrast": s.contrast, "saturation": s.saturation, "temperature": s.temperature })),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> serde_json::Value {
        let root = paths::find_root().unwrap();
        let bytes = std::fs::read(paths::data_dir(&root).join("fx/atmosphere.json")).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[test]
    fn data_file_parses() {
        let look = LightingLook::load().expect("battle_lighting.json");
        assert!((look.unit() - 1.2 * 2f32.powf(look.exposure_ev100)).abs() < 1e-3);
        assert_eq!(look.tonemapping(), Tonemapping::AgX);
    }

    #[test]
    fn sky_follows_weather_season_and_hour() {
        let rules = rules();
        assert_eq!(sky_id(&rules, "clear", "summer", "midday"), "clear_midday");
        assert_eq!(sky_id(&rules, "clear", "summer", "morning"), "winter_clear");
        let id = sky_id(&rules, "rain", "summer", "midday");
        assert!(rules["skies"][id.as_str()].is_object(), "{id}");
    }

    #[test]
    fn grades_compose_like_the_lut() {
        let rules = rules();
        let (contrast, saturation, temperature, _) =
            combined_grade(&grades(&rules, "clear", "summer", "midday"));
        // summer 1.03 * clear 1.05 * midday 1.02; saturation 1.04 * 0.98.
        assert!((contrast - 1.03 * 1.05 * 1.02).abs() < 1e-4);
        assert!((saturation - 1.04 * 0.98).abs() < 1e-4);
        assert!((temperature - 0.23).abs() < 1e-4);
    }

    #[test]
    fn cube_faces_cover_the_sphere() {
        let size = 8;
        assert!(cube_dir(2, 4, 4, size).y > 0.9);
        assert!(cube_dir(3, 4, 4, size).y < -0.9);
        assert!(cube_dir(0, 4, 4, size).x > 0.9);
        // Bevy negates z when sampling: face +Z holds world -Z.
        assert!(cube_dir(4, 4, 4, size).z < -0.9);
    }

    #[test]
    fn half_floats_round_trip() {
        assert_eq!(f16_bits(1.0), 0x3c00);
        assert_eq!(f16_bits(0.5), 0x3800);
        assert_eq!(f16_bits(2.0), 0x4000);
        assert_eq!(f16_bits(0.0), 0);
    }

    #[test]
    fn sky_radiance_is_clipped_and_fades_to_ground() {
        let pano = Panorama {
            width: 4,
            height: 2,
            rgb: vec![[100.0, 100.0, 100.0]; 4]
                .into_iter()
                .chain(vec![[0.2, 0.3, 0.5]; 4])
                .collect(),
        };
        let p = SkyParams {
            exposure: 0.5,
            saturation: 1.0,
            tint: Vec3::ONE,
            haze_amount: 0.0,
            haze_color: Vec3::ONE,
            ground_color: Vec3::new(0.1, 0.1, 0.05),
            max_luminance: 2.5,
            horizon_floor: 0.015,
            yaw: 0.0,
        };
        let up = sky_radiance(&pano, &p, Vec3::Y);
        assert!(up.max_element() <= 2.5 + 1e-3, "{up}");
        let down = sky_radiance(&pano, &p, Vec3::NEG_Y);
        assert!(
            down.z < down.x * 1.01,
            "ground colour below the horizon: {down}"
        );
    }
}
