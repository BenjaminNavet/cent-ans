//! Campaign weather: clouds and cloud shadows, rain / snow, atmosphere (distance fog), the
//! `FogOfWar` data resource and the season banner.
//!
//! Ports `game/scripts/map/campaign_weather_view.gd` (per-turn weather from the core, cloud
//! curves, precipitation near the camera), `precipitation_profile.gd`, `season_banner.gd`; the
//! fog-of-war data side of `minimap_controller.gd` / `map_readability.gd` (`fog`).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.
//!
//! The weather itself is computed by the core (`sim_campaign::weather::weather_map`, ADR 0027):
//! nothing is invented here. Clouds, shadows and particles are plain entities (no terrain-shader
//! change: the masks, wet ground and valley mist of the Godot terrain shader belong to the look lot).
//! `CENT_ANS_WEATHER=rain|snow|fog|storm|clear` forces a kind everywhere (`--map-weather`).

pub mod atmosphere;
pub mod banner;
pub mod clouds;
pub mod fog;
pub mod model;
pub mod precip;
pub mod sky;

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::provinces::data::ProvinceMap;
use crate::sim::Sim;

pub use fog::FogOfWar;
pub use model::{CloudTuning, Entry, Kind, PrecipitationProfile, WeatherMap};

/// Environment variable that forces one weather kind on every province.
pub const WEATHER_ENV: &str = "CENT_ANS_WEATHER";

/// Weather of every province for the current turn (from the core), plus the morning-fog clock.
#[derive(Resource, Debug, Default)]
pub struct CampaignWeather {
    pub by_province: WeatherMap,
    /// Turn the map was computed for.
    pub turn: Option<u32>,
    pub forced: Option<Kind>,
    /// Incremented when the map changed (clouds are rebuilt).
    pub revision: u64,
    /// Seconds since the weather of this turn appeared (morning fog lifts with it).
    pub fog_clock: f32,
}

impl CampaignWeather {
    pub fn entry(&self, province_id: &str) -> Entry {
        self.by_province
            .get(province_id)
            .copied()
            .unwrap_or_default()
    }

    /// Weather at map pixel `px` (`weather_at`): clear off the provinces.
    pub fn at_px(&self, map: &ProvinceMap, px: Vec2) -> Entry {
        let index = map.index_at(px.x, px.y);
        map.province(index)
            .map(|p| self.entry(&p.id))
            .unwrap_or_default()
    }
}

/// Tuning loaded from `data/` when the campaign opens.
#[derive(Resource, Debug, Clone, Default)]
pub struct WeatherTuning {
    pub clouds: CloudTuning,
    pub precipitation: PrecipitationProfile,
}

/// What the camera looks at, refreshed every frame (consumed by clouds, precipitation, fog).
#[derive(Resource, Debug, Clone, Default)]
pub struct WeatherView {
    pub focus: Vec3,
    pub distance: f32,
    pub focus_entry: Entry,
    /// 1 at the start of the turn, 0 once the morning fog has lifted.
    pub morning_mist: f32,
}

pub struct CampaignWeatherPlugin;

impl Plugin for CampaignWeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CampaignWeather>()
            .init_resource::<WeatherTuning>()
            .init_resource::<WeatherView>()
            .init_resource::<FogOfWar>()
            .init_resource::<fog::FogStats>()
            .add_systems(OnEnter(AppState::Campaign), reset_on_enter)
            .add_systems(
                Update,
                (refresh_weather, update_view, fog::refresh_fog)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
        clouds::build(app);
        precip::build(app);
        atmosphere::build(app);
        sky::build(app);
        banner::build(app);
        app.register_dump(dump_weather);
    }
}

fn reset_on_enter(mut weather: ResMut<CampaignWeather>, mut tuning: ResMut<WeatherTuning>) {
    *weather = CampaignWeather {
        revision: weather.revision + 1,
        ..default()
    };
    if let Ok(root) = crate::paths::find_root() {
        tuning.clouds = CloudTuning::load(&root);
        tuning.precipitation = PrecipitationProfile::load(&root);
    }
}

/// Forced kind from the environment (`CENT_ANS_WEATHER`).
fn forced_kind() -> Option<Kind> {
    std::env::var(WEATHER_ENV)
        .ok()
        .and_then(|v| Kind::from_key(&v))
}

/// Reads the core's weather once per turn (and on load: the turn changes or the state is new).
fn refresh_weather(
    sim: Res<Sim>,
    map: Option<Res<ProvinceMap>>,
    mut weather: ResMut<CampaignWeather>,
) {
    let (Some(state), Some(data)) = (sim.state.as_ref(), sim.data.as_ref()) else {
        return;
    };
    let forced = forced_kind();
    let turn = state.turn();
    if weather.turn == Some(turn) && weather.forced == forced && !weather.by_province.is_empty() {
        return;
    }
    let mut by_province = WeatherMap::new();
    match forced {
        Some(kind) => {
            if let Some(map) = map.as_ref() {
                for feature in map.provinces() {
                    by_province.insert(
                        feature.id.clone(),
                        Entry {
                            kind,
                            intensity: 0.8,
                        },
                    );
                }
            }
        }
        None => {
            for (id, w) in sim_campaign::weather::weather_map(data, state) {
                by_province.insert(
                    id.as_str().to_owned(),
                    Entry {
                        kind: Kind::from_key(w.kind.key()).unwrap_or_default(),
                        intensity: w.intensity as f32,
                    },
                );
            }
        }
    }
    if by_province.is_empty() {
        return; // province map not loaded yet (forced) or no data
    }
    let new_turn = weather.turn != Some(turn);
    weather.by_province = by_province;
    weather.turn = Some(turn);
    weather.forced = forced;
    weather.revision += 1;
    if new_turn {
        weather.fog_clock = 0.0; // a new turn: the morning fog forms again
    }
}

fn update_view(
    time: Res<Time<Real>>,
    rig: Option<Res<CampaignRig>>,
    map: Option<Res<ProvinceMap>>,
    tuning: Res<WeatherTuning>,
    mut weather: ResMut<CampaignWeather>,
    mut view: ResMut<WeatherView>,
) {
    weather.fog_clock += time.delta_secs();
    view.morning_mist = tuning.clouds.morning_mist(weather.fog_clock);
    let (Some(rig), Some(map)) = (rig, map) else {
        return;
    };
    view.focus = rig.focus;
    view.distance = rig.distance;
    view.focus_entry = weather.at_px(&map, Vec2::new(rig.focus.x, rig.focus.z));
}

fn dump_weather(world: &World) -> (String, Value) {
    let weather = world.resource::<CampaignWeather>();
    let view = world.resource::<WeatherView>();
    let mut counts = serde_json::Map::new();
    for kind in Kind::ALL {
        let n = weather
            .by_province
            .values()
            .filter(|e| e.kind == kind)
            .count();
        counts.insert(kind.key().to_owned(), json!(n));
    }
    (
        "campaign_weather".to_owned(),
        json!({
            "turn": weather.turn,
            "forced": weather.forced.map(Kind::key),
            "provinces": weather.by_province.len(),
            "kinds": counts,
            "revision": weather.revision,
            "view": {
                "focus_kind": view.focus_entry.kind.key(),
                "focus_intensity": view.focus_entry.intensity,
                "distance": view.distance,
                "morning_mist": view.morning_mist,
            },
            "clouds": clouds::dump(world),
            "precipitation": precip::dump(world),
            "atmosphere": atmosphere::dump(world),
            "sky": sky::dump(world),
            "banner": banner::dump(world),
            "fog_of_war": fog::dump(world),
        }),
    )
}
