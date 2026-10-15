//! Campaign side of the audio: what the Godot `AudioDirector` / `CampaignAmbience` read from the
//! campaign scene, here read from the Bevy resources (the audio folder owns the glue, the
//! campaign lots are not touched).
//!
//! - [`MusicHints`]: at war (relations of the player, refreshed with the turn), court open
//!   (`OpenPanel::Council` until another panel opens or `CloseAll`).
//! - [`AmbienceInput`]: sea / forest / fields / town shares around the camera focus, zoom
//!   distance, season (`Sim`), weather at the focus (`WeatherView`). The Godot shares come from the
//!   splat map; the client has none, so forest and fields come from the terrain of the provinces
//!   under a 3x3 sample (`terrain_shares`), sea from the province raster, the town share from the
//!   nearest capital.
//! - [`PlayTurnSfx`] on `TurnAdvanced`, `ui_open` when a panel opens.

use bevy::prelude::*;
use serde_json::{Value, json};

use super::ambience::AmbienceEnv;
use super::{AmbienceInput, MusicHints, PlaySfx, PlayTurnSfx};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::hud::flow::TurnAdvanced;
use crate::campaign::panels::OpenPanel;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::weather::WeatherView;

/// Share of forest and of fields (birds, crickets) of a province terrain.
pub fn terrain_shares(terrain: &str) -> (f32, f32) {
    match terrain {
        "forest" => (0.85, 0.1),
        "bocage" => (0.35, 0.65),
        "hills" => (0.3, 0.5),
        "mountains" => (0.3, 0.1),
        "marsh" => (0.1, 0.2),
        "heath" => (0.05, 0.4),
        "plains" => (0.05, 0.9),
        "steppe" => (0.0, 0.7),
        "desert" => (0.0, 0.05),
        _ => (0.1, 0.6),
    }
}

/// Season key of the simulation for the ambience tables.
pub fn season_key(season: sim_campaign::Season) -> &'static str {
    match season {
        sim_campaign::Season::Spring => "spring",
        sim_campaign::Season::Summer => "summer",
        sim_campaign::Season::Autumn => "autumn",
        sim_campaign::Season::Winter => "winter",
    }
}

/// Shares of sea, forest, fields and town around `center` (map pixels) for a camera `distance`
/// (`sample_environment`): 3x3 points `radius` apart.
pub fn sample_environment(map: &ProvinceMap, center: Vec2, distance: f32) -> AmbienceEnv {
    let radius = (distance * 0.4).clamp(6.0, 320.0);
    let (mut sea, mut forest, mut fields) = (0.0, 0.0, 0.0);
    let mut nearest = f32::INFINITY;
    for iy in 0..3 {
        for ix in 0..3 {
            let point = center + Vec2::new((ix as f32 - 1.0) * radius, (iy as f32 - 1.0) * radius);
            match map.province(map.index_at(point.x, point.y)) {
                None => sea += 1.0,
                Some(province) => {
                    let (f, c) = terrain_shares(&province.terrain);
                    forest += f;
                    fields += c;
                    let capital = Vec2::from(province.capital_px);
                    nearest = nearest.min(capital.distance(center));
                }
            }
        }
    }
    let town = if nearest.is_finite() {
        (1.0 - nearest / (radius * 0.9 + 4.0)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    AmbienceEnv {
        sea: sea / 9.0,
        forest: forest / 9.0,
        fields: fields / 9.0,
        town,
        distance,
        ..AmbienceEnv::default()
    }
}

/// Last values pushed (dump) and the court flag.
#[derive(Resource, Default)]
pub struct CampaignAudioState {
    court_open: bool,
    last_turn: Option<u32>,
    pub turn_sfx: usize,
    pub panel_sfx: usize,
}

pub struct CampaignAudioPlugin;

impl Plugin for CampaignAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CampaignAudioState>()
            .add_message::<TurnAdvanced>()
            .add_message::<OpenPanel>()
            .add_systems(OnExit(AppState::Campaign), silence_campaign)
            .add_systems(
                Update,
                (feed_hints, feed_ambience, turn_sfx, panel_open_sfx)
                    .run_if(in_state(AppState::Campaign)),
            )
            .register_dump(dump_campaign_audio);
    }
}

fn silence_campaign(
    mut input: ResMut<AmbienceInput>,
    mut hints: ResMut<MusicHints>,
    mut state: ResMut<CampaignAudioState>,
) {
    input.active = false;
    state.court_open = false;
    hints.court_open = false;
}

fn player_at_war(sim: &crate::sim::Sim) -> bool {
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return false;
    };
    state
        .diplomacy_view(data, &sim.player)
        .iter()
        .any(|entry| entry.relation == sim_campaign::RelationKind::War)
}

fn feed_hints(
    mut panels: MessageReader<OpenPanel>,
    sim: Option<Res<crate::sim::Sim>>,
    mut hints: ResMut<MusicHints>,
    mut state: ResMut<CampaignAudioState>,
) {
    for panel in panels.read() {
        state.court_open = matches!(panel, OpenPanel::Council);
    }
    let turn = sim
        .as_ref()
        .and_then(|sim| sim.state.as_ref())
        .map(|state| state.turn());
    let mut at_war = hints.at_war;
    if let Some(sim) = sim.as_ref()
        && (state.last_turn != turn || sim.is_changed())
    {
        at_war = player_at_war(sim);
        state.last_turn = turn;
    }
    if hints.court_open != state.court_open || hints.at_war != at_war {
        hints.court_open = state.court_open;
        hints.at_war = at_war;
    }
}

fn feed_ambience(
    sim: Option<Res<crate::sim::Sim>>,
    rig: Option<Res<CampaignRig>>,
    map: Option<Res<ProvinceMap>>,
    view: Option<Res<WeatherView>>,
    mut input: ResMut<AmbienceInput>,
) {
    let (Some(rig), Some(map)) = (rig, map) else {
        input.active = false;
        return;
    };
    let mut env = sample_environment(&map, Vec2::new(rig.focus.x, rig.focus.z), rig.distance);
    if let Some(state) = sim.as_ref().and_then(|sim| sim.state.as_ref()) {
        env.season = season_key(state.season()).to_string();
    }
    if let Some(view) = view {
        env.weather = view.focus_entry.kind.key().to_string();
    }
    if !input.active || input.env != env {
        input.active = true;
        input.env = env;
    }
}

fn turn_sfx(
    mut turns: MessageReader<TurnAdvanced>,
    mut out: MessageWriter<PlayTurnSfx>,
    mut state: ResMut<CampaignAudioState>,
) {
    for turn in turns.read() {
        let kinds = turn
            .events
            .iter()
            .filter_map(|event| {
                serde_json::to_value(event.kind)
                    .ok()?
                    .as_str()
                    .map(String::from)
            })
            .collect();
        out.write(PlayTurnSfx { event_kinds: kinds });
        state.turn_sfx += 1;
    }
}

/// Opening a panel plays `ui_open` (kit sound of the panel slide-in).
fn panel_open_sfx(
    mut panels: MessageReader<OpenPanel>,
    mut sfx: MessageWriter<PlaySfx>,
    mut state: ResMut<CampaignAudioState>,
) {
    for panel in panels.read() {
        if !matches!(panel, OpenPanel::CloseAll) {
            sfx.write(PlaySfx::new("ui_open"));
            state.panel_sfx += 1;
        }
    }
}

fn dump_campaign_audio(world: &World) -> (String, Value) {
    let (Some(state), Some(input), Some(hints)) = (
        world.get_resource::<CampaignAudioState>(),
        world.get_resource::<AmbienceInput>(),
        world.get_resource::<MusicHints>(),
    ) else {
        return ("campaign_audio".into(), Value::Null);
    };
    let levels = super::ambience::layer_levels(&input.env);
    let layers: serde_json::Map<String, Value> = super::ambience::LAYERS
        .iter()
        .zip(levels)
        .map(|(name, level)| {
            (
                (*name).to_string(),
                json!((level * 1000.0).round() / 1000.0),
            )
        })
        .collect();
    (
        "campaign_audio".into(),
        json!({
            "hints": {"at_war": hints.at_war, "court_open": hints.court_open, "faction": hints.faction},
            "ambience_active": input.active,
            "env": {
                "sea": input.env.sea, "forest": input.env.forest, "fields": input.env.fields,
                "town": input.env.town, "distance": input.env.distance,
                "season": input.env.season, "weather": input.env.weather,
            },
            "layer_levels": layers,
            "turn_sfx": state.turn_sfx,
            "panel_open_sfx": state.panel_sfx,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_shares_are_bounded() {
        for terrain in [
            "forest",
            "plains",
            "hills",
            "mountains",
            "marsh",
            "heath",
            "bocage",
            "steppe",
            "desert",
            "?",
        ] {
            let (forest, fields) = terrain_shares(terrain);
            assert!((0.0..=1.0).contains(&forest) && (0.0..=1.0).contains(&fields));
        }
        assert!(terrain_shares("forest").0 > terrain_shares("plains").0);
    }

    #[test]
    fn seasons_map_to_the_ambience_keys() {
        assert_eq!(season_key(sim_campaign::Season::Winter), "winter");
        assert_eq!(season_key(sim_campaign::Season::Spring), "spring");
    }
}
