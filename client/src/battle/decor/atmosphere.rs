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
    /// Colour of the air at this hour (`time_of_day.<key>.air`, ADR 0755).
    pub air: HourAir,
    /// `data/fx/atmosphere.json`, read once.
    pub rules: serde_json::Value,
}

/// Colour of the air at an hour of the day (`time_of_day.<key>.air` of `atmosphere.json`): the
/// low morning haze is gold-pink, the dusk one orange, midday leaves the neutral blue of the
/// weather. Every field is optional (absent = the weather's own value).
#[derive(Debug, Clone, PartialEq)]
pub struct HourAir {
    /// Fog / horizon haze colour (sRGB).
    pub color: Option<[f32; 3]>,
    /// Aerial-perspective in-scattering (sRGB).
    pub inscattering: Option<[f32; 3]>,
    /// Glow of the fog on the sun side (sRGB).
    pub sun_color: Option<[f32; 3]>,
    /// Ambient light colour (sRGB).
    pub ambient_color: Option<[f32; 3]>,
    /// Factor on the aerial visibility distance (< 1: thicker air).
    pub visibility_scale: f32,
    /// Factor on the ambient light brightness.
    pub ambient_scale: f32,
    /// Share of `color` taken under a weather without aerial perspective (rain, fog, snow).
    pub overcast_share: f32,
}

impl Default for HourAir {
    fn default() -> Self {
        HourAir {
            color: None,
            inscattering: None,
            sun_color: None,
            ambient_color: None,
            visibility_scale: 1.0,
            ambient_scale: 1.0,
            overcast_share: 0.0,
        }
    }
}

fn opt_rgb(value: &serde_json::Value) -> Option<[f32; 3]> {
    value.is_array().then(|| rgb(value))
}

/// Air of a time-of-day preset (neutral when the preset has no `air`).
pub fn hour_air(rules: &serde_json::Value, time_key: &str) -> HourAir {
    let air = &rules["time_of_day"][time_key]["air"];
    HourAir {
        color: opt_rgb(&air["color"]),
        inscattering: opt_rgb(&air["inscattering"]),
        sun_color: opt_rgb(&air["sun_color"]),
        ambient_color: opt_rgb(&air["ambient_color"]),
        visibility_scale: num(&air["visibility_scale"], 1.0),
        ambient_scale: num(&air["ambient_scale"], 1.0),
        overcast_share: num(&air["overcast_share"], 0.0),
    }
}

impl HourAir {
    /// `base` pulled towards the hour's haze colour (fully under a clear sky, `overcast_share`
    /// under the other weathers).
    pub fn tint(&self, base: [f32; 3], weather: Weather) -> [f32; 3] {
        let Some(color) = self.color else {
            return base;
        };
        let share = if weather == Weather::Clear {
            1.0
        } else {
            self.overcast_share
        };
        [
            base[0] + (color[0] - base[0]) * share,
            base[1] + (color[1] - base[1]) * share,
            base[2] + (color[2] - base[2]) * share,
        ]
    }
}

/// Colour of a black body at `kelvin`, brightest channel 1 (Tanner Helland's fit, 1000-12000 K):
/// the colour temperature of the sun of an hour (`sun_kelvin`).
pub fn kelvin_rgb(kelvin: f32) -> [f32; 3] {
    let t = kelvin.clamp(1000.0, 12000.0) / 100.0;
    let r = if t <= 66.0 {
        255.0
    } else {
        329.698_73 * (t - 60.0).powf(-0.133_204_76)
    };
    let g = if t <= 66.0 {
        99.470_8 * t.ln() - 161.119_57
    } else {
        288.122_17 * (t - 60.0).powf(-0.075_514_85)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_73 * (t - 10.0).ln() - 305.044_8
    };
    [r, g, b].map(|c: f32| (c / 255.0).clamp(0.0, 1.0))
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

/// Preset key (`dawn`, `morning`, `midday`, `afternoon`, `dusk`, `night`) whose `phases` contains `phase`; midday otherwise.
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
    // A colour temperature (`sun_kelvin`) wins over the explicit `sun_color`.
    let color = match preset["sun_kelvin"].as_f64() {
        Some(kelvin) => kelvin_rgb(kelvin as f32),
        None => rgb(&preset["sun_color"]),
    };
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

fn ambient_for(
    look: &DecorLook,
    weather: Weather,
    elevation_deg: f32,
    air: &HourAir,
) -> GlobalAmbientLight {
    let a = &look.ambient;
    let low_sun =
        (a.low_sun_floor + elevation_deg / a.low_sun_span_deg).clamp(a.low_sun_floor, 1.0);
    let c = air.ambient_color.unwrap_or(a.color);
    GlobalAmbientLight {
        color: Color::srgb(c[0], c[1], c[2]),
        brightness: a.lux * look.fog_of(weather).ambient * low_sun * air.ambient_scale,
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
        air: hour_air(&rules, &time_key),
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
    commands.insert_resource(ambient_for(&look, weather, sun.elevation_deg, &state.air));
    let horizon = state.air.tint(look.sky_of(weather).horizon, weather);
    commands.insert_resource(ClearColor(Color::srgb(horizon[0], horizon[1], horizon[2])));
    commands.insert_resource(state);
}

/// The camera fog: exponential veil, or the distance-based aerial perspective when the weather
/// has an `aerial` look (`FogFalloff::Atmospheric`, sun-side glow through the directional colour).
pub fn distance_fog(
    fog: &super::look::FogLook,
    density: f32,
    horizon: bool,
    air: &HourAir,
    weather: Weather,
) -> DistanceFog {
    let tinted = air.tint(fog.color, weather);
    let color = Color::srgb(tinted[0], tinted[1], tinted[2]);
    match &fog.aerial {
        Some(a) => {
            let sun = air.sun_color.unwrap_or(a.sun_color);
            let scatter = air.inscattering.unwrap_or(a.inscattering);
            DistanceFog {
                color,
                directional_light_color: if a.sun_exponent > 0.0 {
                    Color::srgb(sun[0], sun[1], sun[2])
                } else {
                    Color::NONE
                },
                directional_light_exponent: a.sun_exponent.max(1.0),
                falloff: FogFalloff::from_visibility_colors(
                    air.visibility_scale
                        * if horizon {
                            a.horizon_visibility_m
                        } else {
                            a.visibility_m
                        },
                    Color::srgb(a.extinction[0], a.extinction[1], a.extinction[2]),
                    Color::srgb(scatter[0], scatter[1], scatter[2]),
                ),
            }
        }
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
        commands.entity(camera).insert(distance_fog(
            fog,
            density,
            horizon_active,
            &state.air,
            weather,
        ));
        state.fog_on_camera = true;
    }
}

/// The sun follows the phase of the day once per second.
#[allow(clippy::too_many_arguments)]
fn follow_day(
    time: Res<Time>,
    mut timer: Local<f32>,
    mut session: ResMut<BattleSession>,
    mut state: ResMut<AtmosphereState>,
    mut suns: Query<(&mut DirectionalLight, &mut Transform), With<BattleSun>>,
    mut fogs: Query<&mut DistanceFog, With<BattleCamera>>,
    horizon_stats: Option<Res<super::horizon::HorizonStats>>,
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
    state.air = hour_air(&state.rules, &state.time_key);
    let sun = sun_light(&state.rules, &state.time_key, weather);
    for (mut light, mut transform) in &mut suns {
        apply_sun(&mut state, sun, &mut light, &mut transform);
    }
    commands.insert_resource(ambient_for(&look, weather, sun.elevation_deg, &state.air));
    let horizon = state.air.tint(look.sky_of(weather).horizon, weather);
    commands.insert_resource(ClearColor(Color::srgb(horizon[0], horizon[1], horizon[2])));
    // The air of the new hour on the camera fog (the fog weather keeps its own falloff).
    let fog_look = look.fog_of(weather);
    for mut fog in &mut fogs {
        let new = distance_fog(
            fog_look,
            state.fog_density,
            horizon_stats.as_ref().is_some_and(|h| h.active),
            &state.air,
            weather,
        );
        if fog_look.aerial.is_some() {
            *fog = new;
        } else {
            fog.color = new.color;
        }
    }
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
    fn same_inputs_give_the_same_sun() {
        let rules = rules();
        assert_eq!(
            sun_light(&rules, "dusk", Weather::Fog),
            sun_light(&rules, "dusk", Weather::Fog)
        );
    }

    #[test]
    fn phases_map_to_presets() {
        let rules = rules();
        assert_eq!(time_key_for(&rules, "dawn"), "dawn");
        assert_eq!(time_key_for(&rules, "afternoon"), "afternoon");
        assert_eq!(time_key_for(&rules, "dusk"), "dusk");
        assert_eq!(time_key_for(&rules, "unknown"), "midday");
    }

    #[test]
    fn overcast_weather_dims_the_sun() {
        let rules = rules();
        let clear = sun_light(&rules, "midday", Weather::Clear);
        let rain = sun_light(&rules, "midday", Weather::Rain);
        assert!((clear.elevation_deg - 32.0).abs() < 1e-3);
        assert!(rain.energy < clear.energy * 0.5);
    }

    #[test]
    fn colour_temperature_orders_the_suns_of_the_day() {
        let (dusk, noon) = (kelvin_rgb(2700.0), kelvin_rgb(5600.0));
        assert!(dusk[0] > dusk[1] && dusk[1] > dusk[2], "{dusk:?}");
        assert!(dusk[2] < 0.5 && noon[2] > 0.8, "{dusk:?} {noon:?}");
        let rules = rules();
        let colour = |key| sun_light(&rules, key, Weather::Clear).color;
        assert!(colour("dawn")[2] < colour("midday")[2]);
        assert!(colour("dusk")[2] < colour("afternoon")[2]);
    }

    #[test]
    fn every_hour_has_its_own_light_and_the_dusk_is_not_dark() {
        let rules = rules();
        let sun = |key| sun_light(&rules, key, Weather::Clear);
        // Low sun at the edges of the day (long shadows), high at midday.
        assert!(sun("dawn").elevation_deg < 12.0 && sun("dusk").elevation_deg < 12.0);
        assert!(sun("midday").elevation_deg > sun("morning").elevation_deg);
        // Ground irradiance (energy x sin elevation) of the golden hours stays readable.
        let irradiance = |key| sun(key).energy * sun(key).elevation_deg.to_radians().sin();
        assert!(irradiance("dusk") > 0.25 * irradiance("midday"));
        assert!(irradiance("dawn") > 0.25 * irradiance("midday"));
        // The air is coloured at dawn and dusk, neutral at midday.
        assert!(hour_air(&rules, "dawn").color.is_some());
        assert!(hour_air(&rules, "dusk").color.is_some());
        assert_eq!(hour_air(&rules, "midday"), HourAir::default());
        let dusk = hour_air(&rules, "dusk").tint([0.7, 0.76, 0.84], Weather::Clear);
        assert!(dusk[0] > dusk[2], "orange haze at dusk: {dusk:?}");
        // Under rain the hour only tints a little.
        let rain = hour_air(&rules, "dusk").tint([0.52, 0.55, 0.59], Weather::Rain);
        assert!((rain[0] - 0.52).abs() < 0.2);
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
