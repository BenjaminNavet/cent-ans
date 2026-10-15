//! « Batailles de démonstration » (game/scripts/ui/battle_demos_menu.gd): the list of
//! `data/ui/battle_demos.json` and the hour of the battle (kept from one demo to the next).
//!
//! Differences from Godot: a demo there starts from the 1337 campaign (main armies, the real
//! province or landmark town besieged). Without a campaign here, a field demo is the default
//! custom battle (France against England) and a siege demo the same armies with the defender
//! holding a walled place (`siege`, default fortification and engines); the `province` /
//! `landmark` / `engines` entries of the file are not used (the besieger is `attacker` when the
//! file gives one, France otherwise).

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
            launches.write(LaunchBattle {
                config: demo_battle(game, demo, &memory.demo_hour),
                seed: sim.seed,
                label: format!("demo:{id}"),
            });
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
}
