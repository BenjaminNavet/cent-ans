//! « Batailles historiques » (game/scripts/ui/historical_battles_menu.gd): Crécy, Poitiers,
//! Azincourt, read from `data/battle_maps/` ([`sim_battle::HistoricalMap`], what
//! `BattleSim.list_historical` lists in Godot). Per battle: date, place, weather, summary, both
//! armies, and « Mener <camp> » / « Regarder ».
//!
//! Difference from Godot: the scripted scenario (orders of battle, waves, relief of the site) is
//! built by `BattleSim.setup_historical`, which the battle scene of the client cannot do yet
//! ([`PendingBattle`] only carries a [`CustomBattle`]). Here the battle is the custom battle of
//! the two real factions: year, terrain, season, weather and start hour of the map, armies bought
//! on budgets proportional to the real numbers (the larger army gets the maximum budget). The
//! choice is also published as [`super::battles::PendingHistorical`] for the battle lot.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use data_model::GameData;
use serde_json::{Value, json};
use sim_battle::custom::{CustomBattle, CustomBattleRules};
use sim_battle::{HistoricalMap, SideId};

use super::battles::{
    BattleScreen, LaunchBattle, OpenBattleScreen, data_dir, day_phases, phase_of_hour,
};
use super::{MenuOverlay, spawn_overlay};
use crate::app::AppState;
use crate::battle::scene::setup;
use crate::sim::Sim;
use crate::ui::kit::style::UiType;
use crate::ui::kit::{ButtonPressed, UiTheme, button_captioned, button_small, scroll_list, typed};

/// Historical maps of `data/battle_maps/` sorted by year (`decor_plan*` files are not maps).
pub fn load_maps(data_dir: &std::path::Path) -> Vec<HistoricalMap> {
    let Ok(entries) = std::fs::read_dir(data_dir.join("battle_maps")) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    let mut maps: Vec<HistoricalMap> = paths
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("decor_plan"))
        })
        .filter_map(|p| {
            let text = std::fs::read_to_string(p).ok()?;
            match HistoricalMap::from_json(&text) {
                Ok(map) => Some(map),
                Err(error) => {
                    warn!("battle map {}: {error}", p.display());
                    None
                }
            }
        })
        .collect();
    maps.sort_by_key(|m| m.year);
    maps
}

/// Men of one side as the menu counts them (`map_summary`: blocks of regiments, 100 by default).
pub fn soldiers(map: &HistoricalMap, side: SideId) -> u32 {
    map.armies
        .side(side)
        .regiments
        .iter()
        .map(|b| b.count.max(1) * b.soldiers.unwrap_or(100))
        .sum()
}

/// « les Anglais », « les Français », « l’armée de X ».
pub fn the_army(faction_name: &str) -> String {
    match faction_name {
        "Angleterre" => "les Anglais".to_owned(),
        "France" => "les Français".to_owned(),
        other => format!("l’armée de {other}"),
    }
}

fn army_line(map: &HistoricalMap, side: SideId) -> String {
    let army = map.armies.side(side);
    let role = if side == SideId::Attacker {
        "attaque"
    } else {
        "défend"
    };
    let name = if army.army.is_empty() {
        &army.faction_name
    } else {
        &army.army
    };
    let general = army.general.as_ref().map_or("", |g| g.name.as_str());
    let mut line = format!(
        "{name} ({role}) — {general}, environ {} hommes",
        soldiers(map, side)
    );
    if army.waves.len() > 1 {
        line.push_str(&format!(" en {} batailles successives", army.waves.len()));
    }
    line
}

/// The custom battle standing for `map`, the player leading `side` (`attacker`, `defender`,
/// empty to watch). See the module docs for what it keeps of the map.
pub fn to_custom(
    game: &GameData,
    data_dir: &std::path::Path,
    map: &HistoricalMap,
    side: &str,
) -> Result<CustomBattle, String> {
    let rules = CustomBattleRules::bundled();
    // Terrain and season are crate-private in the core: read them through serde.
    let raw = serde_json::to_value(map).map_err(|e| e.to_string())?;
    let key = |value: &Value| value.as_str().unwrap_or_default().to_owned();
    let largest = soldiers(map, SideId::Attacker)
        .max(soldiers(map, SideId::Defender))
        .max(1);
    let year = map.year.clamp(rules.min_year, rules.max_year);
    let side_of = |id: SideId| {
        let budget = (u64::from(rules.max_budget) * u64::from(soldiers(map, id))
            / u64::from(largest)) as u32;
        setup::compose_army(
            game,
            &map.armies.side(id).faction,
            budget.clamp(rules.min_budget, rules.max_budget),
            rules.max_units_per_side,
            year,
        )
    };
    let battle = CustomBattle {
        attacker: side_of(SideId::Attacker),
        defender: side_of(SideId::Defender),
        terrain: key(&raw["terrain"]),
        season: key(&raw["season"]),
        weather: key(&raw["weather"]["start"]),
        hour: phase_of_hour(&day_phases(data_dir), map.start_hour),
        siege: false,
        fortification: None,
        player_side: side.to_owned(),
        year: Some(year),
        engines: None,
    };
    let data = sim_battle::custom::CustomData {
        unit_types: &game.unit_types,
        factions: &game.factions,
    };
    let report = battle.validate(&data, rules);
    if report.ok {
        Ok(battle)
    } else {
        Err(report.errors.join(" ; "))
    }
}

#[derive(Component)]
struct HistoricalScreen;

#[derive(Resource, Default)]
struct HistoricalList {
    maps: Vec<HistoricalMap>,
}

pub struct HistoricalPlugin;

impl Plugin for HistoricalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HistoricalList>().add_systems(
            Update,
            (open_screen, handle_buttons).run_if(in_state(AppState::MainMenu)),
        );
    }
}

fn open_screen(
    mut commands: Commands,
    mut requests: MessageReader<OpenBattleScreen>,
    theme: Res<UiTheme>,
    mut list: ResMut<HistoricalList>,
    open: Query<(), With<HistoricalScreen>>,
) {
    if !requests.read().any(|r| r.0 == BattleScreen::Historical) || !open.is_empty() {
        return;
    }
    list.maps = data_dir().as_deref().map(load_maps).unwrap_or_default();
    let (overlay, panel) = spawn_overlay(
        &mut commands,
        &theme,
        "HistoricalScreen",
        "Batailles historiques",
        780.0,
        1,
    );
    commands
        .entity(overlay)
        .insert((HistoricalScreen, DespawnOnExit(AppState::MainMenu)));
    commands.spawn((
        typed(
            &theme,
            "Le champ réel, les armées de ce jour-là, la météo et l’heure : menez l’un des camps, ou regardez. Hors campagne : le résultat n’est pas conservé.",
            UiType::Caption,
        ),
        ChildOf(panel),
    ));
    let scroll = commands
        .spawn((
            scroll_list(&theme, "historical:list", 500.0),
            ChildOf(panel),
        ))
        .id();
    if list.maps.is_empty() {
        commands.spawn((
            typed(
                &theme,
                "Aucune bataille historique n’a été trouvée.",
                UiType::Body,
            ),
            ChildOf(scroll),
        ));
    }
    for map in &list.maps {
        spawn_entry(&mut commands, &theme, scroll, map);
    }
    commands.spawn((
        button_captioned(&theme, "historical:close", "Fermer"),
        ChildOf(panel),
    ));
}

fn spawn_entry(commands: &mut Commands, theme: &UiTheme, parent: Entity, map: &HistoricalMap) {
    commands.spawn((
        typed(
            theme,
            &format!("{} — {}", map.name, map.date_fr()),
            UiType::Heading,
        ),
        ChildOf(parent),
    ));
    commands.spawn((
        typed(
            theme,
            &format!(
                "{} ({}) · {}",
                map.site.place, map.province_name, map.weather.label
            ),
            UiType::Caption,
        ),
        ChildOf(parent),
    ));
    commands.spawn((typed(theme, &map.summary, UiType::Body), ChildOf(parent)));
    commands.spawn((
        typed(
            theme,
            &format!(
                "{}\n{}",
                army_line(map, SideId::Attacker),
                army_line(map, SideId::Defender)
            ),
            UiType::Caption,
        ),
        ChildOf(parent),
    ));
    let row = commands
        .spawn((
            Node {
                column_gap: Val::Px(10.0),
                margin: UiRect::bottom(Val::Px(12.0)),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    for side in ["defender", "attacker", ""] {
        let (name, caption) = match side {
            "" => (format!("Battle_{}_watch", map.id), "Regarder".to_owned()),
            _ => {
                let id = if side == "attacker" {
                    SideId::Attacker
                } else {
                    SideId::Defender
                };
                (
                    format!("Battle_{}_{side}", map.id),
                    format!("Mener {}", the_army(&map.armies.side(id).faction_name)),
                )
            }
        };
        commands.spawn((button_small(theme, &name, &caption), ChildOf(row)));
    }
}

fn handle_buttons(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    list: Res<HistoricalList>,
    sim: Res<Sim>,
    screens: Query<Entity, With<HistoricalScreen>>,
    mut launches: MessageWriter<LaunchBattle>,
    mut toasts: MessageWriter<crate::ui::kit::modal::ToastRequest>,
) {
    if screens.is_empty() {
        return;
    }
    for press in pressed.read() {
        let name = press.name.as_str();
        if name == "historical:close" {
            for screen in &screens {
                commands.entity(screen).despawn();
            }
            continue;
        }
        let Some(rest) = name.strip_prefix("Battle_") else {
            continue;
        };
        let Some((id, side)) = rest.rsplit_once('_') else {
            continue;
        };
        let side = if side == "watch" { "" } else { side };
        let (Some(map), Some(game), Some(dir)) = (
            list.maps.iter().find(|m| m.id == id),
            sim.data.as_deref(),
            data_dir(),
        ) else {
            continue;
        };
        match to_custom(game, &dir, map, side) {
            Ok(config) => {
                launches.write(LaunchBattle::new(
                    config,
                    sim.seed,
                    &format!("historical:{id}:{side}"),
                ));
            }
            Err(message) => {
                warn!("historical {id}: {message}");
                toasts.write(crate::ui::kit::modal::ToastRequest::new(&message));
            }
        }
    }
}

pub fn dump(world: &World) -> Value {
    let list = world.resource::<HistoricalList>();
    let open = world
        .try_query_filtered::<(), With<HistoricalScreen>>()
        .is_some_and(|mut q| q.iter(world).count() > 0);
    json!({
        "open": open,
        "maps": list.maps.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
        "overlay_count": world.try_query_filtered::<(), With<MenuOverlay>>().map_or(0, |mut q| q.iter(world).count()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths;

    #[test]
    fn maps_load_in_year_order_and_build_valid_battles() {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let maps = load_maps(&dir);
        let ids: Vec<&str> = maps.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["crecy", "poitiers", "azincourt"]);
        let (game, _) = GameData::load(&dir).expect("game data");
        for map in &maps {
            let battle = to_custom(&game, &dir, map, "defender").expect(&map.id);
            assert_eq!(battle.player_side, "defender");
            assert_eq!(battle.year, Some(map.year));
            assert_eq!(battle.terrain, "plains");
            assert!(!battle.hour.is_empty(), "{}", map.id);
            assert!(battle.attacker.units.len() > 5 && battle.defender.units.len() > 5);
            assert!(
                battle.attacker.budget != battle.defender.budget
                    || soldiers(map, SideId::Attacker) == soldiers(map, SideId::Defender)
            );
        }
        assert_eq!(the_army("Angleterre"), "les Anglais");
    }
}
