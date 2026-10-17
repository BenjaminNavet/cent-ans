//! Sun, ambient light, sky colour and distance fog of a battle (port of the light and fog parts of
//! game/scripts/battle/battle_atmosphere.gd and battle_time_of_day.gd).
//!
//! The sun follows the time of day of the simulation (`BattleSim::day_phase`): the preset of
//! `data/fx/atmosphere.json` `time_of_day` whose `phases` contains the phase gives elevation,
//! azimuth, colour and energy; the weather block (`battle.<weather>`) scales energy and elevation
//! and tints the light. Fog density and colour per weather are the `PRESETS` of the GDScript.
//! Fog, ambient and sky values come from `data/fx/battle_decor_look.json` (`look`); the sky dome
//! itself is in `horizon`. The HDRI sky, sun disc, SSAO and LUT grading are in
//! `battle/scene/lighting.rs`; rain and snow in `precipitation`, volumetric fog in `volumetric`,
//! horizon smoke in `horizon_smoke` (plugins added here, data `battle_weather_fx.json`).

use bevy::light::{DirectionalLightShadowMap, GlobalAmbientLight};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use sim_battle::Weather;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::paths;

use super::look::DecorLook;

/// Lux per unit of Godot light energy (clear midday: 1.75 -> 14 000 lux, the previous value).
pub const LUX_PER_ENERGY: f32 = 8000.0;

/// Weather of a key of `AtmosphereState::weather` (`clear` when unknown).
pub fn weather_from_key(key: &str) -> Weather {
    match key {
        "rain" => Weather::Rain,
        "fog" => Weather::Fog,
        "snow" => Weather::Snow,
        _ => Weather::Clear,
    }
}

pub fn weather_key(weather: Weather) -> &'static str {
    match weather {
        Weather::Clear => "clear",
        Weather::Rain => "rain",
        Weather::Fog => "fog",
        Weather::Snow => "snow",
    }
}

/// What the sun and fog currently are (read by the dump).
#[derive(Resource, Debug, Clone, Default)]
pub struct AtmosphereState {
    pub time_key: String,
    pub phase: String,
    pub weather: String,
    pub fog_density: f32,
    pub sun_illuminance: f32,
    pub sun_elevation_deg: f32,
    pub sun_azimuth_deg: f32,
    pub sun_direction: [f32; 3],
    pub fog_on_camera: bool,
    /// `data/fx/atmosphere.json`, read once.
    pub rules: serde_json::Value,
}

/// The sun of the battle.
#[derive(Component)]
pub struct BattleSun;

/// Resolved light of a time of day under a weather.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunLight {
    pub elevation_deg: f32,
    pub azimuth_deg: f32,
    pub color: [f32; 3],
    pub energy: f32,
}

fn num(value: &serde_json::Value, default: f32) -> f32 {
    value.as_f64().map_or(default, |v| v as f32)
}

fn rgb(value: &serde_json::Value) -> [f32; 3] {
    [
        num(&value[0], 1.0),
        num(&value[1], 1.0),
        num(&value[2], 1.0),
    ]
}

/// Preset key (`morning`, `midday`, `evening`) whose `phases` contains `phase`; midday otherwise.
pub fn time_key_for(rules: &serde_json::Value, phase: &str) -> String {
    if let Some(presets) = rules["time_of_day"].as_object() {
        for (key, preset) in presets {
            if preset["phases"]
                .as_array()
                .is_some_and(|phases| phases.iter().any(|p| p.as_str() == Some(phase)))
            {
                return key.clone();
            }
        }
    }
    "midday".to_owned()
}

/// Sun of a time-of-day preset under a weather (elevation, azimuth, tinted colour, energy).
pub fn sun_light(rules: &serde_json::Value, time_key: &str, weather: Weather) -> SunLight {
    let preset = &rules["time_of_day"][time_key];
    let wx = &rules["battle"][weather_key(weather)];
    let tint = rgb(&wx["sun_tint"]);
    let color = rgb(&preset["sun_color"]);
    SunLight {
        elevation_deg: num(&preset["sun_elevation"], 36.0) * num(&wx["sun_elevation_scale"], 1.0),
        azimuth_deg: num(&preset["sun_azimuth"], 218.0),
        color: [color[0] * tint[0], color[1] * tint[1], color[2] * tint[2]],
        energy: num(&preset["sun_energy"], 1.75) * num(&wx["sun_energy_scale"], 1.0),
    }
}

/// Direction the sunlight travels (Godot: `R_y(yaw) * R_x(-elevation) * -Z`).
pub fn light_direction(elevation_deg: f32, azimuth_deg: f32) -> Vec3 {
    let (e, a) = (elevation_deg.to_radians(), azimuth_deg.to_radians());
    Vec3::new(-e.cos() * a.sin(), -e.sin(), -e.cos() * a.cos())
}

/// Weather of the sky of the battle: a historical map is rendered under the sky of its end (the
/// shower of its start falls from it, see `precipitation`); `CENT_ANS_BATTLE_WEATHER` wins.
pub fn sky_weather(sim: &sim_battle::BattleSim) -> Weather {
    let forced = std::env::var("CENT_ANS_BATTLE_WEATHER").is_ok_and(|w| !w.is_empty());
    if forced {
        return sim.weather();
    }
    sim.scenario_final_weather().unwrap_or(sim.weather())
}

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        if let Some(fx) = super::weather_fx::WeatherFxLook::load() {
            app.insert_resource(fx);
        }
        app.add_plugins((
            super::precipitation::PrecipitationPlugin,
            super::volumetric::VolumetricPlugin,
            super::height_fog::HeightFogPlugin,
            super::horizon_silhouettes::HorizonSilhouettesPlugin,
            super::horizon_smoke::HorizonSmokePlugin,
        ))
        .register_dump(dump_weather);
        app.insert_resource(DirectionalLightShadowMap { size: 4096 })
            .add_systems(
                Update,
                spawn_atmosphere.run_if(resource_added::<BattleSession>),
            )
            .add_systems(
                Update,
                (fog_on_camera, follow_day)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<AtmosphereState>),
            )
            .add_systems(OnExit(AppState::Battle), clear_state);
    }
}

fn clear_state(mut commands: Commands) {
    commands.remove_resource::<AtmosphereState>();
}

fn read_rules() -> serde_json::Value {
    paths::find_root()
        .ok()
        .and_then(|root| std::fs::read(paths::data_dir(&root).join("fx/atmosphere.json")).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub(crate) fn apply_sun(
    state: &mut AtmosphereState,
    sun: SunLight,
    light: &mut DirectionalLight,
    t: &mut Transform,
) {
    let dir = light_direction(sun.elevation_deg, sun.azimuth_deg);
    *t = Transform::IDENTITY.looking_to(dir, Vec3::Y);
    light.illuminance = (sun.energy * LUX_PER_ENERGY).max(500.0);
    light.color = Color::srgb(sun.color[0], sun.color[1], sun.color[2]);
    state.sun_illuminance = light.illuminance;
    state.sun_elevation_deg = sun.elevation_deg;
    state.sun_azimuth_deg = sun.azimuth_deg;
    state.sun_direction = dir.to_array();
}

fn ambient_for(look: &DecorLook, weather: Weather, elevation_deg: f32) -> GlobalAmbientLight {
    let a = &look.ambient;
    let low_sun =
        (a.low_sun_floor + elevation_deg / a.low_sun_span_deg).clamp(a.low_sun_floor, 1.0);
    GlobalAmbientLight {
        color: Color::srgb(a.color[0], a.color[1], a.color[2]),
        brightness: a.lux * look.fog_of(weather).ambient * low_sun,
        ..default()
    }
}

fn spawn_atmosphere(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    look: Res<DecorLook>,
) {
    let sim = session.sim_mut();
    let weather = sky_weather(sim);
    let phase = sim.day_phase().key.clone();
    let rules = read_rules();
    let time_key = time_key_for(&rules, &phase);
    let sun = sun_light(&rules, &time_key, weather);
    let fog = look.fog_of(weather);
    let mut state = AtmosphereState {
        time_key,
        phase,
        weather: weather_key(weather).to_owned(),
        fog_density: fog.density,
        rules,
        ..default()
    };
    let mut light = DirectionalLight {
        shadow_maps_enabled: true,
        ..default()
    };
    let mut transform = Transform::IDENTITY;
    apply_sun(&mut state, sun, &mut light, &mut transform);
    commands.spawn((
        Name::new("BattleSun"),
        BattleSun,
        light,
        // Four cascades out to 450 m (the default 150 m left the mid-distance view without shadows).
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: 4,
            first_cascade_far_bound: 18.0,
            maximum_distance: 450.0,
            overlap_proportion: 0.25,
            ..default()
        }
        .build(),
        transform,
        DespawnOnExit(AppState::Battle),
    ));
    commands.insert_resource(ambient_for(&look, weather, sun.elevation_deg));
    let horizon = look.sky_of(weather).horizon;
    commands.insert_resource(ClearColor(Color::srgb(horizon[0], horizon[1], horizon[2])));
    commands.insert_resource(state);
}

/// The camera fog: exponential veil, or the distance-based aerial perspective when the weather
/// has an `aerial` look (`FogFalloff::Atmospheric`, sun-side glow through the directional colour).
pub fn distance_fog(fog: &super::look::FogLook, density: f32, horizon: bool) -> DistanceFog {
    let color = Color::srgb(fog.color[0], fog.color[1], fog.color[2]);
    match &fog.aerial {
        Some(a) => DistanceFog {
            color,
            directional_light_color: if a.sun_exponent > 0.0 {
                Color::srgb(a.sun_color[0], a.sun_color[1], a.sun_color[2])
            } else {
                Color::NONE
            },
            directional_light_exponent: a.sun_exponent.max(1.0),
            falloff: FogFalloff::from_visibility_colors(
                if horizon {
                    a.horizon_visibility_m
                } else {
                    a.visibility_m
                },
                Color::srgb(a.extinction[0], a.extinction[1], a.extinction[2]),
                Color::srgb(a.inscattering[0], a.inscattering[1], a.inscattering[2]),
            ),
        },
        None => DistanceFog {
            color,
            falloff: FogFalloff::Exponential { density },
            ..default()
        },
    }
}

/// Distance fog on the battle camera (spawned by the scene lot).
fn fog_on_camera(
    mut commands: Commands,
    mut state: ResMut<AtmosphereState>,
    look: Res<DecorLook>,
    horizon: Option<Res<super::horizon::HorizonStats>>,
    cameras: Query<Entity, (With<BattleCamera>, Without<DistanceFog>)>,
) {
    let weather = weather_from_key(&state.weather);
    let fog = look.fog_of(weather);
    let horizon_active = horizon.is_some_and(|h| h.active);
    let density = if horizon_active {
        fog.horizon_density
    } else {
        fog.density
    };
    state.fog_density = density;
    for camera in &cameras {
        commands
            .entity(camera)
            .insert(distance_fog(fog, density, horizon_active));
        state.fog_on_camera = true;
    }
}

/// The sun follows the phase of the day once per second.
fn follow_day(
    time: Res<Time>,
    mut timer: Local<f32>,
    mut session: ResMut<BattleSession>,
    mut state: ResMut<AtmosphereState>,
    mut suns: Query<(&mut DirectionalLight, &mut Transform), With<BattleSun>>,
    look: Res<DecorLook>,
    mut commands: Commands,
) {
    *timer += time.delta_secs();
    if *timer < 1.0 {
        return;
    }
    *timer = 0.0;
    let sim = session.sim_mut();
    let weather = weather_from_key(&state.weather);
    let phase = sim.day_phase().key.clone();
    if phase == state.phase {
        return;
    }
    let key = time_key_for(&state.rules, &phase);
    state.phase = phase;
    if key == state.time_key {
        return;
    }
    state.time_key = key;
    let sun = sun_light(&state.rules, &state.time_key, weather);
    for (mut light, mut transform) in &mut suns {
        apply_sun(&mut state, sun, &mut light, &mut transform);
    }
    commands.insert_resource(ambient_for(&look, weather, sun.elevation_deg));
}

/// Dump section `battle_weather`: precipitation, volumetric fog, horizon smoke.
fn dump_weather(world: &World) -> (String, serde_json::Value) {
    let rain = world.get_resource::<super::precipitation::PrecipitationStats>();
    let vol = world.get_resource::<super::volumetric::VolumetricStats>();
    let smoke = world.get_resource::<super::horizon_smoke::HorizonSmokeStats>();
    let mut fog_cameras = 0;
    let mut volumetric_suns = 0;
    let mut fog_volumes = 0;
    let mut precipitation_entities = 0;
    for entity in world.iter_entities() {
        fog_cameras += usize::from(
            entity.contains::<BattleCamera>() && entity.contains::<bevy::light::VolumetricFog>(),
        );
        volumetric_suns += usize::from(
            entity.contains::<BattleSun>() && entity.contains::<bevy::light::VolumetricLight>(),
        );
        fog_volumes += usize::from(entity.contains::<bevy::light::FogVolume>());
        precipitation_entities +=
            usize::from(entity.contains::<super::precipitation::Precipitation>());
    }
    (
        "battle_weather".to_owned(),
        serde_json::json!({
            "data_loaded": world.contains_resource::<super::weather_fx::WeatherFxLook>(),
            "weather": world.get_resource::<AtmosphereState>().map(|a| a.weather.clone()),
            "precipitation": rain.map(|r| serde_json::json!({
                "kind": r.kind, "drops": r.drops, "shower": r.shower, "centre": r.centre,
                "entities": precipitation_entities,
            })),
            "volumetric": vol.map(|v| serde_json::json!({
                "mode": v.mode, "enabled": v.enabled, "density": v.density,
                "anisotropy": v.anisotropy, "volumes": v.volumes, "ground_mist": v.ground_mist,
                "cameras_with_fog": fog_cameras, "volumetric_suns": volumetric_suns,
                "fog_volume_entities": fog_volumes,
            })),
            "height_fog": world.get_resource::<super::height_fog::HeightFogStats>().map(|h| serde_json::json!({
                "active": h.active, "ground_mean": h.ground_mean, "camera_y": h.camera_y,
                "density": h.density, "base_density": h.base_density,
            })),
            "horizon_silhouettes": world.get_resource::<super::horizon_silhouettes::HorizonSilhouettes>().map(|s| serde_json::json!({
                "villages": s.villages, "castles": s.castles, "triangles": s.triangles,
                "sites": s.sites.len(),
                "distances_m": s.sites.iter().map(|p| p.xz().distance(s.centre)).collect::<Vec<_>>(),
            })),
            "horizon_smoke": smoke.map(|s| serde_json::json!({
                "columns": s.columns, "distances_m": s.distances, "light_tint": s.light_tint,
            })),
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
    fn phases_map_to_presets() {
        let rules = rules();
        assert_eq!(time_key_for(&rules, "dawn"), "morning");
        assert_eq!(time_key_for(&rules, "afternoon"), "midday");
        assert_eq!(time_key_for(&rules, "dusk"), "evening");
        assert_eq!(time_key_for(&rules, "unknown"), "midday");
    }

    #[test]
    fn overcast_weather_dims_the_sun() {
        let rules = rules();
        let clear = sun_light(&rules, "midday", Weather::Clear);
        let rain = sun_light(&rules, "midday", Weather::Rain);
        assert!((clear.elevation_deg - 36.0).abs() < 1e-3);
        assert!(rain.energy < clear.energy * 0.5);
    }

    #[test]
    fn light_travels_downwards_away_from_the_sun() {
        let d = light_direction(36.0, 218.0);
        assert!((d.length() - 1.0).abs() < 1e-5);
        assert!(d.y < 0.0);
        // Azimuth 218: the sun sits towards +sin/+cos of 218 deg (-x, -z side flipped).
        assert!(d.x > 0.0 && d.z > 0.0);
    }
}
