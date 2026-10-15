//! « Batailles de démonstration » (game/scripts/ui/battle_demos_menu.gd): the list of
//! `data/ui/battle_demos.json` and the hour of the battle (kept from one demo to the next).
//!
//! As in Godot (`battle_scene.gd begin_demo`), a demo starts from a throw-away 1337 campaign
//! (France as the player, the player's own campaign is never touched): the most numerous army of
//! France and of England; a field demo opposes them, a siege demo stages the siege of the
//! `province` (France first, England when France cannot) or of the `landmark` town (drawn from
//! its plan, war declared on its holder, besieger = `attacker`, France by default), then adds the
//! `engines` of the file to the besieger. The launch package is the core's own
//! (`game_api::flow::battle_launch`), handed over like a campaign battle. When the staging
//! fails (data without the armies or the place), the demo falls back to a custom battle
//! ([`demo_battle`]).

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};
use sim_battle::custom::{CustomBattle, CustomBattleRules};

use super::battles::{
    BattleMemory, BattleScreen, DayPhase, LaunchBattle, OpenBattleScreen, data_dir, day_phases,
};
use super::spawn_overlay;
use crate::app::AppState;
use crate::battle::scene::setup;
use crate::sim::Sim;
use crate::ui::kit::style::UiType;
use crate::ui::kit::{ButtonPressed, UiTheme, button_captioned, button_small, scroll_list, typed};

/// A demo of `battle_demos.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct Demo {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub siege: bool,
    pub attacker: Option<String>,
}

/// Demos declared by the file (empty when it is missing).
pub fn load_demos(data_dir: &std::path::Path) -> Vec<Demo> {
    let parsed = std::fs::read_to_string(data_dir.join("ui/battle_demos.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let Some(list) = parsed.as_ref().and_then(|v| v["demos"].as_array()) else {
        return Vec::new();
    };
    let text = |d: &Value, key: &str| d[key].as_str().unwrap_or_default().to_owned();
    list.iter()
        .filter(|d| d["id"].is_string())
        .map(|d| Demo {
            id: text(d, "id"),
            label: text(d, "label"),
            detail: text(d, "detail"),
            siege: d["kind"].as_str() == Some("siege"),
            attacker: d["attacker"].as_str().map(str::to_owned),
        })
        .collect()
}

/// The custom battle standing for `demo` (see the module docs), with `hour` (phase key or empty).
pub fn demo_battle(game: &data_model::GameData, demo: &Demo, hour: &str) -> CustomBattle {
    let mut battle = setup::default_custom_battle(game);
    battle.hour = hour.to_owned();
    if demo.siege {
        let rules = CustomBattleRules::bundled();
        battle.siege = true;
        battle.fortification = Some(rules.default_fortification);
        if let Some(attacker) = &demo.attacker {
            let defender = if attacker == setup::DEFAULT_ATTACKER {
                setup::DEFAULT_DEFENDER
            } else {
                setup::DEFAULT_ATTACKER
            };
            battle.attacker = setup::compose_army(
                game,
                attacker,
                rules.default_budget,
                rules.max_units_per_side,
                rules.default_year,
            );
            battle.defender = setup::compose_army(
                game,
                defender,
                rules.default_budget,
                rules.max_units_per_side,
                rules.default_year,
            );
        }
    }
    battle
}

/// Everything a siege demo may name in `battle_demos.json`.
fn demo_extras(
    data_dir: &std::path::Path,
    id: &str,
) -> (Option<String>, Option<String>, Vec<String>) {
    let parsed = std::fs::read_to_string(data_dir.join("ui/battle_demos.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let entry = parsed
        .as_ref()
        .and_then(|v| v["demos"].as_array())
        .and_then(|list| list.iter().find(|d| d["id"].as_str() == Some(id)));
    let text = |key: &str| entry.and_then(|d| d[key].as_str()).map(str::to_owned);
    let engines = entry
        .and_then(|d| d["engines"].as_array())
        .map(|list| {
            list.iter()
                .filter_map(|e| e.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    (text("province"), text("landmark"), engines)
}

/// Id of the most numerous army of `faction` (first on a tie, as the Godot `main_armies`).
fn main_army(state: &sim_campaign::CampaignState, faction: &str) -> Option<sim_campaign::ArmyId> {
    let mut best: Option<(&sim_campaign::ArmyId, usize)> = None;
    for (id, army) in &state.armies {
        if army.faction.as_str() == faction && best.is_none_or(|(_, n)| army.units.len() > n) {
            best = Some((id, army.units.len()));
        }
    }
    best.map(|(id, _)| id.clone())
}

/// The launch package of `demo` on a throw-away 1337 campaign (see the module docs).
pub fn stage_demo(
    game: &data_model::GameData,
    demo: &Demo,
    data_dir: &std::path::Path,
    hour: &str,
    seed: u64,
) -> Result<game_api::flow::BattleLaunch, String> {
    let (province, landmark, engines) = demo_extras(data_dir, &demo.id);
    let player = data_model::FactionId::new(setup::DEFAULT_ATTACKER).map_err(|e| e.to_string())?;
    let mut state =
        sim_campaign::CampaignState::new_1337(game, player, seed).map_err(|e| e.to_string())?;
    sim_campaign::feudal::sync_suzerains(&mut state, game);
    let france = main_army(&state, setup::DEFAULT_ATTACKER).ok_or("pas d'armée française")?;
    let england = main_army(&state, setup::DEFAULT_DEFENDER).ok_or("pas d'armée anglaise")?;
    let stage_error = |e: sim_campaign::BattleRequestError| e.to_string();
    let index = if !demo.siege {
        state
            .debug_stage_battle(&france, &england)
            .map_err(stage_error)?
    } else if let Some(landmark) = &landmark {
        let besieger = if demo.attacker.as_deref() == Some(setup::DEFAULT_DEFENDER) {
            &england
        } else {
            &france
        };
        state
            .debug_stage_landmark_siege(game, besieger, landmark)
            .map_err(stage_error)?
    } else if let Some(province) = &province {
        let province = data_model::ProvinceId::new(province.as_str()).map_err(|e| e.to_string())?;
        match state.debug_stage_siege(game, &france, &province) {
            Ok(index) => index,
            Err(_) => state
                .debug_stage_siege(game, &england, &province)
                .map_err(stage_error)?,
        }
    } else {
        return Err("siège sans province ni ville".to_owned());
    };
    let mut launch = game_api::flow::battle_launch(game, &state, index)?;
    if !hour.is_empty() {
        launch.hour = sim_battle::TimeOfDayRules::bundled()
            .start_hour_of(hour)
            .ok_or_else(|| format!("heure inconnue : {hour}"))?;
    }
    if launch.setup.siege.is_some() {
        for id in &engines {
            let Some(unit_type) = data_model::UnitTypeId::new(id.as_str())
                .ok()
                .and_then(|id| game.unit_types.get(&id))
            else {
                continue;
            };
            launch
                .setup
                .attacker
                .units
                .push(sim_battle::UnitSetup::from_unit_type(
                    unit_type,
                    unit_type.soldiers,
                    unit_type.stats.morale,
                    0,
                ));
        }
    }
    Ok(launch)
}

#[derive(Component)]
struct DemosScreen;

#[derive(Component)]
struct HourText;

/// Demos shown on the screen and the hours it offers.
#[derive(Resource, Default)]
struct DemoList {
    demos: Vec<Demo>,
    /// `("", "Au hasard (comme en campagne)")` then the selectable phases.
    hours: Vec<(String, String)>,
}

pub struct DemosPlugin;

impl Plugin for DemosPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DemoList>().add_systems(
            Update,
            (open_screen, handle_buttons).run_if(in_state(AppState::MainMenu)),
        );
    }
}

fn open_screen(
    mut commands: Commands,
    mut requests: MessageReader<OpenBattleScreen>,
    theme: Res<UiTheme>,
    memory: Res<BattleMemory>,
    mut list: ResMut<DemoList>,
    open: Query<(), With<DemosScreen>>,
) {
    if !requests.read().any(|r| r.0 == BattleScreen::Demos) || !open.is_empty() {
        return;
    }
    let dir = data_dir();
    list.demos = dir.as_deref().map(load_demos).unwrap_or_default();
    let phases: Vec<DayPhase> = dir.as_deref().map(day_phases).unwrap_or_default();
    list.hours = std::iter::once((String::new(), "Au hasard (comme en campagne)".to_owned()))
        .chain(
            phases
                .into_iter()
                .filter(|p| p.selectable)
                .map(|p| (p.key, p.label)),
        )
        .collect();
    let (overlay, panel) = spawn_overlay(
        &mut commands,
        &theme,
        "DemosScreen",
        "Batailles de démonstration",
        720.0,
        1,
    );
    commands
        .entity(overlay)
        .insert((DemosScreen, DespawnOnExit(AppState::MainMenu)));
    commands.spawn((
        typed(
            &theme,
            "Hors campagne : le résultat n’est pas conservé.",
            UiType::Caption,
        ),
        ChildOf(panel),
    ));
    let row = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    commands.spawn((
        typed(&theme, "Heure de la bataille", UiType::Body),
        ChildOf(row),
    ));
    commands.spawn((button_small(&theme, "demo_hour:-1", "◀"), ChildOf(row)));
    let current = list
        .hours
        .iter()
        .find(|h| h.0 == memory.demo_hour)
        .map_or("", |h| h.1.as_str());
    commands.spawn((
        Name::new("DemoHour"),
        HourText,
        typed(&theme, current, UiType::Body),
        ChildOf(row),
    ));
    commands.spawn((button_small(&theme, "demo_hour:1", "▶"), ChildOf(row)));
    let scroll = commands
        .spawn((scroll_list(&theme, "demos:list", 340.0), ChildOf(panel)))
        .id();
    if list.demos.is_empty() {
        commands.spawn((
            typed(&theme, "Aucune démonstration n’est déclarée.", UiType::Body),
            ChildOf(scroll),
        ));
    }
    for demo in &list.demos {
        commands.spawn((
            button_captioned(&theme, &format!("Demo_{}", demo.id), &demo.label),
            ChildOf(scroll),
        ));
        commands.spawn((
            typed(&theme, &demo.detail, UiType::Caption),
            ChildOf(scroll),
        ));
    }
    commands.spawn((
        button_captioned(&theme, "demos:close", "Fermer"),
        ChildOf(panel),
    ));
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    mut memory: ResMut<BattleMemory>,
    list: Res<DemoList>,
    sim: Res<Sim>,
    screens: Query<Entity, With<DemosScreen>>,
    mut hour_text: Query<&mut Text, With<HourText>>,
    mut launches: MessageWriter<LaunchBattle>,
) {
    if screens.is_empty() {
        return;
    }
    for press in pressed.read() {
        let name = press.name.as_str();
        if name == "demos:close" {
            for screen in &screens {
                commands.entity(screen).despawn();
            }
        } else if let Some(delta) = name.strip_prefix("demo_hour:") {
            let delta: i32 = delta.parse().unwrap_or(0);
            let count = list.hours.len() as i32;
            let at = list
                .hours
                .iter()
                .position(|h| h.0 == memory.demo_hour)
                .unwrap_or(0) as i32;
            if let Some((key, label)) = list
                .hours
                .get((at + delta).rem_euclid(count.max(1)) as usize)
            {
                memory.demo_hour = key.clone();
                for mut text in &mut hour_text {
                    text.0 = label.clone();
                }
            }
        } else if let Some(id) = name.strip_prefix("Demo_")
            && let Some(demo) = list.demos.iter().find(|d| d.id == id)
            && let Some(game) = sim.data.as_deref()
        {
            let mut launch = LaunchBattle::new(
                demo_battle(game, demo, &memory.demo_hour),
                sim.seed,
                &format!("demo:{id}"),
            );
            match data_dir()
                .ok_or_else(|| "dossier data introuvable".to_owned())
                .and_then(|dir| stage_demo(game, demo, &dir, &memory.demo_hour, sim.seed))
            {
                Ok(staged) => launch.staged = Some(Box::new(staged)),
                Err(message) => warn!("demo {id}: staging failed ({message}), custom battle used"),
            }
            launches.write(launch);
        }
    }
}

pub fn dump(world: &World) -> Value {
    let list = world.resource::<DemoList>();
    let open = world
        .try_query_filtered::<(), With<DemosScreen>>()
        .is_some_and(|mut q| q.iter(world).count() > 0);
    json!({
        "open": open,
        "demos": list.demos.iter().map(|d| d.id.clone()).collect::<Vec<_>>(),
        "hours": list.hours.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths;

    #[test]
    fn demos_load_and_build_valid_battles() {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let demos = load_demos(&dir);
        assert!(demos.len() >= 4, "{demos:?}");
        assert!(demos.iter().any(|d| d.siege) && demos.iter().any(|d| !d.siege));
        let (game, _) = data_model::GameData::load(&dir).expect("game data");
        for demo in &demos {
            let battle = demo_battle(&game, demo, "dawn");
            assert_eq!(battle.siege, demo.siege);
            assert_eq!(battle.hour, "dawn");
            let data = sim_battle::custom::CustomData {
                unit_types: &game.unit_types,
                factions: &game.factions,
            };
            let report = battle.validate(&data, CustomBattleRules::bundled());
            assert!(report.ok, "{}: {:?}", demo.id, report.errors);
        }
    }

    #[test]
    fn staged_demos_keep_province_landmark_engines_and_attacker() {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let (game, _) = data_model::GameData::load(&dir).expect("game data");
        let demos = load_demos(&dir);
        for demo in &demos {
            let launch = stage_demo(&game, demo, &dir, "", 1337)
                .unwrap_or_else(|e| panic!("{}: {e}", demo.id));
            assert_eq!(launch.setup.siege.is_some(), demo.siege, "{}", demo.id);
            let (province, landmark, engines) = demo_extras(&dir, &demo.id);
            if landmark.is_some() {
                assert!(
                    launch.setup.siege_layout.is_some(),
                    "{}: town from its plan",
                    demo.id
                );
                let besieger = demo.attacker.as_deref().unwrap_or("fac_france");
                assert_eq!(launch.setup.attacker.faction, besieger, "{}", demo.id);
            }
            if demo.siege {
                let names: Vec<&str> = launch
                    .setup
                    .attacker
                    .units
                    .iter()
                    .map(|u| u.unit_type.as_str())
                    .collect();
                for engine in &engines {
                    assert!(names.contains(&engine.as_str()), "{}: {engine}", demo.id);
                }
            }
            if let Some(province) = province {
                assert_eq!(launch.setup.province, province, "{}", demo.id);
            }
            // The package builds a simulation.
            let sim = crate::flow::launch::build_sim(&game_api::flow::BattleLaunch {
                hour: launch.hour,
                ..launch.clone()
            });
            assert!(sim.is_ok(), "{}: {:?}", demo.id, sim.err());
        }
        // The hour of the demo overrides the drawn one.
        let field = demos.iter().find(|d| !d.siege).expect("a field demo");
        let dawn = stage_demo(&game, field, &dir, "dawn", 1337).expect("dawn");
        let dusk = stage_demo(&game, field, &dir, "dusk", 1337).expect("dusk");
        assert_ne!(dawn.hour, dusk.hour);
    }
}
