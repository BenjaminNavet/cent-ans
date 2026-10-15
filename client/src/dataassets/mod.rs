//! Client visual/UI data as typed Bevy assets (ADR 0363, spike `bm/dataassets`).
//!
//! `data/` is registered as the asset source `data://` ([`DataSourcePlugin`], added *before*
//! `DefaultPlugins`). Three files are loaded through `bevy_common_assets::JsonAssetPlugin` and
//! collected by a `bevy_asset_loader` loading state ([`DataLoad`]):
//! - `fx/campaign_lighting.json` -> [`CampaignLightingDoc`] (typed [`CampaignLighting`], resource
//!   [`CampaignLightingLive`], plus the raw document);
//! - `fx/atmosphere.json` -> [`AtmosphereRules`] (JSON document, battle sun and ambient);
//! - `art/town_look.json` -> [`TownLookDoc`] (JSON document, `TownLook::from_json`).
//!
//! With the `file_watcher` cargo feature (included in `dev`) a saved edit reloads the file and
//! the systems below react to `AssetEvent::Modified`. The campaign readers that parse these two
//! files synchronously at startup (`LightData`, `FogLook`, `ShadowTuning`; kept for tests and
//! headless dumps) are rebuilt from the reloaded documents with the same `from_json`, so each file
//! has one parser per consumer and one live source. Schema validation (`data/schemas`) is
//! unchanged: the JSON files are the same, validated by the same Python tool.
//! Game rules stay in `core/`.

use bevy::asset::AssetLoadFailedEvent;
use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_common_assets::json::JsonAssetPlugin;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::battle::decor::atmosphere::{
    AtmosphereState, BattleSun, apply_sun, sun_light, time_key_for,
};
use crate::campaign::terrain::lighting::LightData;
use crate::campaign::terrain::shadows::ShadowTuning;
use crate::campaign::towns::layer::TownsWorld;
use crate::campaign::towns::look::TownLook;
use crate::campaign::weather::atmosphere::FogLook;

/// Name of the asset source rooted at `data/` (`data://fx/atmosphere.json`).
pub const SOURCE: &str = "data";

/// Registers the `data://` asset source. Must be added before `DefaultPlugins`.
pub struct DataSourcePlugin {
    pub data_dir: std::path::PathBuf,
}

impl Plugin for DataSourcePlugin {
    fn build(&self, app: &mut App) {
        let dir = self.data_dir.to_string_lossy().into_owned();
        app.register_asset_source(
            AssetSourceId::from(SOURCE),
            AssetSourceBuilder::platform_default(&dir, None),
        );
    }
}

/// Loading state of the data collection.
#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
pub enum DataLoad {
    #[default]
    Loading,
    Ready,
    /// A file is missing or malformed: the error is logged (`DATA_ERROR`) and dumped.
    Failed,
}

/// `data/fx/campaign_lighting.json` as loaded: the raw document (read by the campaign
/// `from_json` readers) and its typed view.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct CampaignLightingDoc {
    pub doc: Value,
    pub lighting: CampaignLighting,
}

impl<'de> Deserialize<'de> for CampaignLightingDoc {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let doc = Value::deserialize(deserializer)?;
        let lighting = serde_json::from_value(doc.clone()).map_err(serde::de::Error::custom)?;
        Ok(CampaignLightingDoc { doc, lighting })
    }
}

/// Typed view of `data/fx/campaign_lighting.json` (schema `fx_campaign_lighting.schema.json`).
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct CampaignLighting {
    pub transition_s: Transition,
    pub elevation_bounds_deg: [f32; 2],
    pub turn_variation: TurnVariation,
    pub ambient: Ambient,
    pub exposure: Exposure,
    pub aerial: Aerial,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Transition {
    pub season: f32,
    pub turn: f32,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct TurnVariation {
    pub enabled: bool,
    pub elevation_deg: f32,
    pub azimuth_deg: f32,
    pub warmth: f32,
    pub energy: f32,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Ambient {
    pub color: [f32; 3],
    pub energy: f32,
    pub sky_contribution: f32,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Exposure {
    pub tonemap_exposure: f32,
    pub tonemap_white: f32,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Aerial {
    pub low_pitch_deg: f32,
    pub top_pitch_deg: f32,
    pub blue: [f32; 3],
    pub blue_mix: f32,
    pub density: [f32; 2],
    pub sky_affect: f32,
}

/// `data/fx/atmosphere.json`, kept as a JSON document (the readers index it by key).
#[derive(Asset, TypePath, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct AtmosphereRules(pub Value);

/// `data/art/town_look.json`, parsed by `TownLook::from_json`.
#[derive(Asset, TypePath, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct TownLookDoc(pub Value);

/// The files collected by the loading state; the handles stay alive so that the watcher keeps
/// reloading them.
#[derive(AssetCollection, Resource)]
pub struct ClientData {
    #[asset(path = "data://fx/campaign_lighting.json")]
    pub lighting: Handle<CampaignLightingDoc>,
    #[asset(path = "data://fx/atmosphere.json")]
    pub atmosphere: Handle<AtmosphereRules>,
    #[asset(path = "data://art/town_look.json")]
    pub town_look: Handle<TownLookDoc>,
}

/// Current campaign lighting (updated on `Added` / `Modified`).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct CampaignLightingLive(pub CampaignLighting);

/// What the reload systems did (read by the dump and the tests).
#[derive(Resource, Debug, Default)]
pub struct ReloadStats {
    pub lighting_loads: u32,
    pub lighting_reloads: u32,
    pub atmosphere_reloads: u32,
    pub town_look_reloads: u32,
    /// Seconds since startup when the loading state reached `Ready`.
    pub ready_at_s: f32,
    /// Seconds since startup at the first `Update` frame.
    pub first_frame_at_s: f32,
    /// Wall clock (ms since epoch) of the last `Modified` event.
    pub last_reload_unix_ms: u128,
    pub error: Option<String>,
    /// Load errors seen so far (`path: cause`).
    pub errors: Vec<String>,
}

pub struct DataAssetsPlugin;

impl Plugin for DataAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            JsonAssetPlugin::<CampaignLightingDoc>::new(&["json"]),
            JsonAssetPlugin::<AtmosphereRules>::new(&["json"]),
            JsonAssetPlugin::<TownLookDoc>::new(&["json"]),
        ))
        .init_state::<DataLoad>()
        .init_resource::<ReloadStats>()
        .add_loading_state(
            LoadingState::new(DataLoad::Loading)
                .continue_to_state(DataLoad::Ready)
                .on_failure_continue_to_state(DataLoad::Failed)
                .load_collection::<ClientData>(),
        )
        .add_systems(
            Update,
            (
                collect_failures,
                report_failure
                    .run_if(in_state(DataLoad::Failed))
                    .run_if(|stats: Res<ReloadStats>| stats.error.is_none()),
            )
                .chain(),
        )
        .add_systems(
            Update,
            |mut stats: ResMut<ReloadStats>, time: Res<Time<Real>>| {
                if stats.first_frame_at_s == 0.0 {
                    stats.first_frame_at_s = time.elapsed_secs();
                }
            },
        )
        .add_systems(
            OnEnter(DataLoad::Ready),
            |mut stats: ResMut<ReloadStats>, time: Res<Time<Real>>| {
                stats.ready_at_s = time.elapsed_secs();
            },
        )
        .add_systems(
            Update,
            (
                apply_lighting,
                apply_atmosphere,
                apply_town_look,
                refresh_campaign_readers,
            )
                .run_if(in_state(DataLoad::Ready)),
        )
        .register_dump(dump_data);
    }
}

/// Collects the load errors of the three data types (path and cause), whatever the state.
fn collect_failures(
    mut lighting: MessageReader<AssetLoadFailedEvent<CampaignLightingDoc>>,
    mut atmosphere: MessageReader<AssetLoadFailedEvent<AtmosphereRules>>,
    mut town_look: MessageReader<AssetLoadFailedEvent<TownLookDoc>>,
    mut stats: ResMut<ReloadStats>,
) {
    let mut push = |path: String, error: String| {
        let line = format!("{path}: {error}");
        if !stats.errors.contains(&line) {
            stats.errors.push(line);
        }
    };
    for e in lighting.read() {
        push(e.path.to_string(), e.error.to_string());
    }
    for e in atmosphere.read() {
        push(e.path.to_string(), e.error.to_string());
    }
    for e in town_look.read() {
        push(e.path.to_string(), e.error.to_string());
    }
}

/// Files of [`ClientData`], relative to `data/`.
const FILES: [&str; 3] = [
    "fx/campaign_lighting.json",
    "fx/atmosphere.json",
    "art/town_look.json",
];

fn report_failure(mut stats: ResMut<ReloadStats>) {
    // The loader only says "failed": name the missing files explicitly.
    if let Ok(root) = crate::paths::find_root() {
        for relative in FILES {
            let path = crate::paths::data_dir(&root).join(relative);
            if !path.exists() {
                stats
                    .errors
                    .push(format!("missing file: {}", path.display()));
            }
        }
    }
    let message = if stats.errors.is_empty() {
        "client data failed to load (see log)".to_owned()
    } else {
        stats.errors.join("; ")
    };
    error!("DATA_ERROR: {message}");
    stats.error = Some(message);
}

fn apply_lighting(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<CampaignLightingDoc>>,
    assets: Res<Assets<CampaignLightingDoc>>,
    data: Res<ClientData>,
    mut stats: ResMut<ReloadStats>,
) {
    for event in events.read() {
        let (loaded, modified) = match event {
            AssetEvent::LoadedWithDependencies { id } => (*id == data.lighting.id(), false),
            AssetEvent::Modified { id } => (false, *id == data.lighting.id()),
            _ => (false, false),
        };
        if !(loaded || modified) {
            continue;
        }
        if let Some(lighting) = assets.get(&data.lighting) {
            commands.insert_resource(CampaignLightingLive(lighting.lighting.clone()));
            if modified {
                stats.lighting_reloads += 1;
                stats.last_reload_unix_ms = now_ms();
            } else {
                stats.lighting_loads += 1;
            }
        }
    }
}

/// The campaign readers of `fx/campaign_lighting.json` and `fx/atmosphere.json` (sun and grading
/// `LightData`, aerial fog `FogLook`, sun shadows `ShadowTuning`) follow a reload of either file;
/// they keep their startup value (synchronous read) until the first `Modified`.
#[allow(clippy::too_many_arguments)]
fn refresh_campaign_readers(
    mut lighting_events: MessageReader<AssetEvent<CampaignLightingDoc>>,
    mut atmosphere_events: MessageReader<AssetEvent<AtmosphereRules>>,
    lighting: Res<Assets<CampaignLightingDoc>>,
    atmosphere: Res<Assets<AtmosphereRules>>,
    data: Res<ClientData>,
    light_data: Option<ResMut<LightData>>,
    fog: Option<ResMut<FogLook>>,
    shadows: Option<ResMut<ShadowTuning>>,
) {
    let mut modified = false;
    for event in lighting_events.read() {
        modified |= matches!(event, AssetEvent::Modified { id } if *id == data.lighting.id());
    }
    for event in atmosphere_events.read() {
        modified |= matches!(event, AssetEvent::Modified { id } if *id == data.atmosphere.id());
    }
    let (true, Some(lighting), Some(atmosphere)) = (
        modified,
        lighting.get(&data.lighting),
        atmosphere.get(&data.atmosphere),
    ) else {
        return;
    };
    if let Some(mut light_data) = light_data {
        *light_data = LightData::from_json(&atmosphere.0, &lighting.doc);
    }
    if let Some(mut fog) = fog {
        *fog = FogLook::from_json(&atmosphere.0, &lighting.doc);
    }
    if let Some(mut shadows) = shadows {
        *shadows = ShadowTuning::from_json(&lighting.doc["shadows"]);
    }
}

/// Battle sun and rules follow `fx/atmosphere.json`: on `Modified`, the new document replaces
/// `AtmosphereState::rules` and the sun is recomputed in place.
fn apply_atmosphere(
    mut events: MessageReader<AssetEvent<AtmosphereRules>>,
    assets: Res<Assets<AtmosphereRules>>,
    data: Res<ClientData>,
    state: Option<ResMut<AtmosphereState>>,
    mut suns: Query<(&mut DirectionalLight, &mut Transform), With<BattleSun>>,
    mut stats: ResMut<ReloadStats>,
    mut pending: Local<bool>,
) {
    for event in events.read() {
        if matches!(event, AssetEvent::Modified { id } if *id == data.atmosphere.id()) {
            *pending = true;
            stats.atmosphere_reloads += 1;
            stats.last_reload_unix_ms = now_ms();
        }
    }
    let (Some(mut state), Some(rules)) = (state, assets.get(&data.atmosphere)) else {
        return;
    };
    // Also adopt the asset the first time a battle starts (the battle plugin read the file itself).
    if !*pending && !state.is_added() {
        return;
    }
    *pending = false;
    state.rules = rules.0.clone();
    let key = time_key_for(&state.rules, &state.phase);
    state.time_key = key;
    let weather = match state.weather.as_str() {
        "rain" => sim_battle::Weather::Rain,
        "fog" => sim_battle::Weather::Fog,
        "snow" => sim_battle::Weather::Snow,
        _ => sim_battle::Weather::Clear,
    };
    let sun = sun_light(&state.rules, &state.time_key, weather);
    for (mut light, mut transform) in &mut suns {
        apply_sun(&mut state, sun, &mut light, &mut transform);
    }
}

fn apply_town_look(
    mut events: MessageReader<AssetEvent<TownLookDoc>>,
    assets: Res<Assets<TownLookDoc>>,
    data: Res<ClientData>,
    world: Option<ResMut<TownsWorld>>,
    mut stats: ResMut<ReloadStats>,
    mut pending: Local<bool>,
) {
    for event in events.read() {
        if matches!(event, AssetEvent::Modified { id } if *id == data.town_look.id()) {
            *pending = true;
            stats.town_look_reloads += 1;
            stats.last_reload_unix_ms = now_ms();
        }
    }
    // Adopt the asset when the towns world is (re)built, and on every reload.
    let Some(mut world) = world else { return };
    if !(*pending || world.is_added()) {
        return;
    }
    if let Some(doc) = assets.get(&data.town_look) {
        *pending = false;
        world.look = TownLook::from_json(&doc.0);
    }
}

/// Modification time of a `data/` file (ms since epoch), for the hot reload delay measure.
fn mtime_ms(relative: &str) -> Option<u128> {
    let root = crate::paths::find_root().ok()?;
    let modified = std::fs::metadata(crate::paths::data_dir(&root).join(relative))
        .ok()?
        .modified()
        .ok()?;
    Some(
        modified
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

fn dump_data(world: &World) -> (String, Value) {
    let stats = world.resource::<ReloadStats>();
    let lighting = world.get_resource::<CampaignLightingLive>().map(|l| &l.0);
    let state = world
        .get_resource::<State<DataLoad>>()
        .map(|s| format!("{:?}", s.get()));
    let sun = world.get_resource::<AtmosphereState>().map(|a| {
        json!({"elevation_deg": a.sun_elevation_deg, "azimuth_deg": a.sun_azimuth_deg,
               "illuminance": a.sun_illuminance, "time_key": a.time_key})
    });
    let town = world
        .get_resource::<TownsWorld>()
        .map(|t| json!({"ribbon_lift_world": t.look.ribbon_lift_world, "masonry": t.look.masonry}));
    (
        "dataassets".into(),
        json!({
            "state": state,
            "error": stats.error,
            "lighting_loads": stats.lighting_loads,
            "lighting_reloads": stats.lighting_reloads,
            "atmosphere_reloads": stats.atmosphere_reloads,
            "town_look_reloads": stats.town_look_reloads,
            "ready_at_s": stats.ready_at_s,
            "first_frame_at_s": stats.first_frame_at_s,
            "last_reload_unix_ms": stats.last_reload_unix_ms,
            "file_mtime_unix_ms": {
                "campaign_lighting": mtime_ms("fx/campaign_lighting.json"),
                "atmosphere": mtime_ms("fx/atmosphere.json"),
                "town_look": mtime_ms("art/town_look.json"),
            },
            "campaign_lighting": lighting.map(|l| json!({
                "ambient_energy": l.ambient.energy,
                "ambient_color": l.ambient.color,
                "tonemap_exposure": l.exposure.tonemap_exposure,
                "season_transition_s": l.transition_s.season,
            })),
            "battle_sun": sun,
            "town_look": town,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(relative: &str) -> Vec<u8> {
        let root = crate::paths::find_root().unwrap();
        std::fs::read(crate::paths::data_dir(&root).join(relative)).unwrap()
    }

    #[test]
    fn real_campaign_lighting_parses_into_the_typed_asset() {
        let lighting: CampaignLighting =
            serde_json::from_slice(&real("fx/campaign_lighting.json")).unwrap();
        assert!(lighting.transition_s.season > lighting.transition_s.turn);
        assert!(lighting.elevation_bounds_deg[0] < lighting.elevation_bounds_deg[1]);
        assert!(lighting.aerial.low_pitch_deg < lighting.aerial.top_pitch_deg);
    }

    #[test]
    fn json_documents_keep_the_readers_working() {
        let rules: AtmosphereRules = serde_json::from_slice(&real("fx/atmosphere.json")).unwrap();
        assert_eq!(time_key_for(&rules.0, "dawn"), "morning");
        let doc: TownLookDoc = serde_json::from_slice(&real("art/town_look.json")).unwrap();
        assert_eq!(
            TownLook::from_json(&doc.0),
            TownLook::load(&crate::paths::data_dir(&crate::paths::find_root().unwrap()))
        );
    }

    #[test]
    fn campaign_readers_agree_with_the_sync_loaders() {
        let root = crate::paths::find_root().unwrap();
        let lighting: CampaignLightingDoc =
            serde_json::from_slice(&real("fx/campaign_lighting.json")).unwrap();
        let rules: AtmosphereRules = serde_json::from_slice(&real("fx/atmosphere.json")).unwrap();
        assert_eq!(
            ShadowTuning::from_json(&lighting.doc["shadows"]),
            ShadowTuning::load(&root)
        );
        let fog = FogLook::from_json(&rules.0, &lighting.doc);
        assert_eq!(fog.density, FogLook::load(&root).density);
        let light = LightData::from_json(&rules.0, &lighting.doc);
        assert_eq!(light.bounds, LightData::load(&root).bounds);
    }

    #[test]
    fn listed_files_exist() {
        let root = crate::paths::find_root().unwrap();
        for relative in FILES {
            assert!(
                crate::paths::data_dir(&root).join(relative).is_file(),
                "{relative}"
            );
        }
    }
}
