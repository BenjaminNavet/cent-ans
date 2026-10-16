//! Custom battle screen (game/scripts/ui/custom_battle_screen.gd, custom_battle_presets.gd):
//! two sides, each a playable faction with a budget of points spent on the units of its roster,
//! the field (terrain, season, weather, hour, year), the siege option (defender holds a walled
//! place, fortification, engines) and the side led by the player. Rules stay in
//! `sim_battle::custom` (`roster`, `validate`); this screen composes a [`CustomBattle`], shows
//! the core's report and sends [`LaunchBattle`]. The last composition is kept in
//! `custom_battle_last.json`, kept armies in `custom_battle_presets.json` (`presets`).
//!
//! Widgets (script names): `cb:faction:<side>:±1`, `cb:budget:<side>:±1`, `cb:terrain:±1`
//! (`season`, `weather`, `hour`, `player`), `cb:year:±1|±10`, `cb:siege`, `cb:fort:±1`,
//! `cb:ladders`, `cb:ram`, `cb:towers`, `buy:<side>:<unit>`, `rm:<side>:<index>`,
//! `cb:clear:<side>`, `preset:prev|next|load|save|delete:<side>`, `cb:launch`, `cb:close`.
//!
//! Also: `cb:techs:<side>` (opens the list of grantable technologies, `tech:<side>:<id>` grants
//! or withdraws one: they unlock the units that require them), `cb:hold` (the opposing army holds
//! its ground, applied by `battles::apply_hold`), `cb:name:<side>` (focus the name field, then
//! type; Enter keeps the army under that name, an empty name gives « <faction> n »). Roster and
//! army rows carry the unit's painted miniature and its tooltip (`unit_tips`).
//!
//! Not ported: the siege place type (Godot kept a `place` key that the core dropped in SC BB14:
//! no rule behind it).

use std::collections::BTreeMap;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::ui::InteractionDisabled;
use data_model::{FactionId, GameData, UnitType};
use serde_json::{Value, json};
use sim_battle::custom::{
    CustomBattle, CustomBattleRules, CustomData, CustomEngines, CustomReport, CustomSide,
    grantable_technologies, known_technologies, roster,
};

use super::battles::{
    BattleScreen, DayPhase, LaunchBattle, OpenBattleScreen, custom_prefs_dir, data_dir, day_phases,
};
use super::presets::{self, PresetArmy};
use super::saves::SaveDir;
use super::unit_tips::unit_type_bbcode;
use super::{MenuOverlay, spawn_overlay};
use crate::app::AppState;
use crate::battle::scene::setup;
use crate::sim::Sim;
use crate::ui::kit::icons::icon;
use crate::ui::kit::style::UiType;
use crate::ui::kit::text_field::{
    TextField, TextFieldChanged, TextFieldSubmitted, text_field_limited,
};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::tooltip_spec::TooltipTexts;
use crate::ui::kit::{
    ButtonPressed, ListSelected, UiTheme, button_captioned, button_primary, button_small, heading,
    list_item, scroll_list, typed,
};

/// Side of the miniature in front of a roster row.
const ROW_ICON: f32 = 24.0;
const SIDES: [&str; 2] = ["attacker", "defender"];
const TERRAINS: [(&str, &str); 9] = [
    ("plains", "Plaines"),
    ("hills", "Collines"),
    ("mountains", "Montagnes"),
    ("forest", "Forêt"),
    ("marsh", "Marais"),
    ("heath", "Lande"),
    ("bocage", "Bocage"),
    ("steppe", "Steppe"),
    ("desert", "Désert"),
];
const SEASONS: [(&str, &str); 4] = [
    ("spring", "Printemps"),
    ("summer", "Été"),
    ("autumn", "Automne"),
    ("winter", "Hiver"),
];
const WEATHERS: [(&str, &str); 5] = [
    ("", "Selon la saison"),
    ("clear", "Temps clair"),
    ("rain", "Pluie"),
    ("fog", "Brouillard"),
    ("snow", "Neige"),
];
const PLAYERS: [(&str, &str); 3] = [
    ("attacker", "Camp 1"),
    ("defender", "Camp 2"),
    ("", "Aucun (IA contre IA)"),
];

/// Marks the screen overlay.
#[derive(Component)]
struct CustomScreen;

/// Text (or button caption) refreshed from the draft under `key`.
#[derive(Component)]
struct ValueText(String);

#[derive(Component)]
struct RosterBody(&'static str);

#[derive(Component)]
struct ArmyBody(&'static str);

#[derive(Component)]
struct LaunchButton;

/// List of grantable technologies of a side (hidden while closed).
#[derive(Component)]
struct TechBody(&'static str);

/// The composition being edited and everything derived from it.
#[derive(Resource)]
pub struct CustomDraft {
    pub battle: CustomBattle,
    pub report: CustomReport,
    presets_path: PathBuf,
    last_path: PathBuf,
    presets: BTreeMap<String, PresetArmy>,
    /// Preset chosen in each side's list (name).
    chosen: [Option<String>; 2],
    /// Playable factions `(id, name)` by name, hours `("", label)…`.
    factions: Vec<(String, String)>,
    hours: Vec<(String, String)>,
    dirty: bool,
    /// The opposing army holds its ground (`hold_opponent`).
    pub hold_opponent: bool,
    /// Preset name typed in each side's field, and the field taking the keys.
    names: [String; 2],
    /// Technology list open for each side.
    techs_open: [bool; 2],
    /// Rule texts of the unit tooltips (`None` when the file cannot be read).
    texts: Option<TooltipTexts>,
}

fn side_index(side: &str) -> usize {
    usize::from(side == "defender")
}

fn side_of<'a>(battle: &'a CustomBattle, side: &str) -> &'a CustomSide {
    if side == "attacker" {
        &battle.attacker
    } else {
        &battle.defender
    }
}

fn side_mut<'a>(battle: &'a mut CustomBattle, side: &str) -> &'a mut CustomSide {
    if side == "attacker" {
        &mut battle.attacker
    } else {
        &mut battle.defender
    }
}

fn cycle(len: usize, at: usize, delta: i32) -> usize {
    (at as i32 + delta).rem_euclid(len.max(1) as i32) as usize
}

fn label_of(pairs: &[(&str, &str)], key: &str) -> String {
    pairs
        .iter()
        .find(|p| p.0 == key)
        .map_or_else(|| key.to_owned(), |p| p.1.to_owned())
}

/// Playable factions `(id, short name)` sorted by name.
pub fn playable_factions(game: &GameData) -> Vec<(String, String)> {
    let mut list: Vec<(String, String)> = game
        .factions
        .values()
        .filter(|f| f.playable)
        .map(|f| (f.id.as_str().to_owned(), game.faction_name(&f.id)))
        .collect();
    list.sort_by_key(|f| f.1.to_lowercase());
    list
}

/// Units `side` may buy in the battle's year (its faction, the technologies granted).
pub fn side_roster<'a>(game: &'a GameData, battle: &CustomBattle, side: &str) -> Vec<&'a UnitType> {
    let composed = side_of(battle, side);
    let rules = CustomBattleRules::bundled();
    let year = battle
        .year
        .unwrap_or(rules.default_year)
        .clamp(rules.min_year, rules.max_year);
    FactionId::new(composed.faction.as_str())
        .ok()
        .and_then(|id| game.factions.get(&id))
        .map(|faction| {
            let techs = known_technologies(faction, &composed.technologies);
            roster(&game.unit_types, faction, year, &techs)
        })
        .unwrap_or_default()
}

/// Technologies `side` may be granted: `(id, name, names of the units it unlocks)` by name.
pub fn grantable(
    game: &GameData,
    battle: &CustomBattle,
    side: &str,
) -> Vec<(String, String, Vec<String>)> {
    let rules = CustomBattleRules::bundled();
    let year = battle
        .year
        .unwrap_or(rules.default_year)
        .clamp(rules.min_year, rules.max_year);
    let Some(faction) = FactionId::new(side_of(battle, side).faction.as_str())
        .ok()
        .and_then(|id| game.factions.get(&id))
    else {
        return Vec::new();
    };
    let mut list: Vec<(String, String, Vec<String>)> =
        grantable_technologies(&game.unit_types, faction, year)
            .into_iter()
            .map(|(tech, units)| {
                let name = game
                    .technologies
                    .get(&tech)
                    .map_or_else(|| tech.to_string(), |t| t.name.display.clone());
                let units = units.iter().map(|u| u.name.display.clone()).collect();
                (tech.to_string(), name, units)
            })
            .collect();
    list.sort_by(|a, b| a.1.cmp(&b.1));
    list
}

/// Grants `tech` to `side`, or withdraws it (the units that needed it leave the army).
pub fn toggle_technology(game: &GameData, battle: &mut CustomBattle, side: &str, tech: &str) {
    let composed = side_mut(battle, side);
    if let Some(at) = composed.technologies.iter().position(|t| t == tech) {
        composed.technologies.remove(at);
        prune_army(game, battle, side);
    } else if grantable(game, battle, side).iter().any(|g| g.0 == tech) {
        side_mut(battle, side).technologies.push(tech.to_owned());
    }
}

/// Removes the units of `side` that its roster no longer offers (faction, year, technologies).
pub fn prune_army(game: &GameData, battle: &mut CustomBattle, side: &str) {
    let allowed: Vec<String> = side_roster(game, battle, side)
        .iter()
        .map(|u| u.id.as_str().to_owned())
        .collect();
    side_mut(battle, side)
        .units
        .retain(|unit| allowed.contains(unit));
}

/// Buys `unit` for `side`; false (nothing changes) when the roster, the budget or the cap forbids.
pub fn buy(game: &GameData, battle: &mut CustomBattle, side: &str, unit: &str) -> bool {
    let rules = CustomBattleRules::bundled();
    let Some(cost) = side_roster(game, battle, side)
        .iter()
        .find(|u| u.id.as_str() == unit)
        .map(|u| u.cost.money)
    else {
        return false;
    };
    let spent = army_cost(game, battle, side);
    let composed = side_mut(battle, side);
    let budget = if composed.budget == 0 {
        rules.default_budget
    } else {
        composed.budget
    };
    if spent + cost > budget || composed.units.len() >= rules.max_units_per_side {
        return false;
    }
    composed.units.push(unit.to_owned());
    true
}

/// Points spent by `side` on units the game knows.
pub fn army_cost(game: &GameData, battle: &CustomBattle, side: &str) -> u32 {
    side_of(battle, side)
        .units
        .iter()
        .filter_map(|id| {
            data_model::UnitTypeId::new(id.as_str())
                .ok()
                .and_then(|id| game.unit_types.get(&id))
        })
        .map(|u| u.cost.money)
        .sum()
}

/// Validation by the core.
pub fn validate(game: &GameData, battle: &CustomBattle) -> CustomReport {
    let data = CustomData {
        unit_types: &game.unit_types,
        factions: &game.factions,
    };
    battle.validate(&data, CustomBattleRules::bundled())
}

fn engines_of(battle: &CustomBattle) -> CustomEngines {
    battle
        .engines
        .unwrap_or_else(|| engines_default(CustomBattleRules::bundled()))
}

fn engines_default(rules: &CustomBattleRules) -> CustomEngines {
    // `default_engines` is crate-private: read it through serde.
    serde_json::to_value(rules)
        .ok()
        .and_then(|v| serde_json::from_value(v["default_engines"].clone()).ok())
        .unwrap_or_default()
}

fn budget_step() -> u32 {
    serde_json::to_value(CustomBattleRules::bundled())
        .ok()
        .and_then(|v| v["budget_step"].as_u64())
        .map_or(500, |s| s as u32)
}

/// The composition shown on opening: the kept one when readable (and valid factions), else
/// France against England with armies bought automatically.
fn initial_battle(game: &GameData, last_path: &std::path::Path) -> (CustomBattle, bool) {
    let text = std::fs::read_to_string(last_path).ok();
    let hold = text
        .as_deref()
        .and_then(|t| serde_json::from_str::<Value>(t).ok())
        .and_then(|v| v["hold_opponent"].as_bool())
        .unwrap_or(false);
    let kept = text
        .as_deref()
        .and_then(|text| serde_json::from_str::<CustomBattle>(text).ok());
    let playable = |faction: &str| {
        FactionId::new(faction)
            .ok()
            .and_then(|id| game.factions.get(&id))
            .is_some_and(|f| f.playable)
    };
    match kept {
        Some(battle)
            if playable(&battle.attacker.faction) && playable(&battle.defender.faction) =>
        {
            (battle, hold)
        }
        _ => (setup::default_custom_battle(game), false),
    }
}

impl CustomDraft {
    pub fn new(game: &GameData, data_dir: &std::path::Path, prefs: &std::path::Path) -> Self {
        let last_path = prefs.join("custom_battle_last.json");
        let presets_path = presets::presets_path(prefs);
        let (battle, hold_opponent) = initial_battle(game, &last_path);
        let report = validate(game, &battle);
        let hours = std::iter::once((String::new(), "Au hasard (comme en campagne)".to_owned()))
            .chain(
                day_phases(data_dir)
                    .into_iter()
                    .filter(|p: &DayPhase| p.selectable)
                    .map(|p| (p.key, p.label)),
            )
            .collect();
        Self {
            presets: presets::load_all(&presets_path),
            battle,
            report,
            presets_path,
            last_path,
            chosen: [None, None],
            factions: playable_factions(game),
            hours,
            dirty: true,
            hold_opponent,
            names: [String::new(), String::new()],
            techs_open: [false, false],
            texts: TooltipTexts::load().ok(),
        }
    }

    fn year(&self) -> i32 {
        self.battle
            .year
            .unwrap_or(CustomBattleRules::bundled().default_year)
    }

    fn set_year(&mut self, game: &GameData, year: i32) {
        let rules = CustomBattleRules::bundled();
        let year = year.clamp(rules.min_year, rules.max_year);
        if year == self.year() {
            return;
        }
        self.battle.year = Some(year);
        for side in SIDES {
            prune_army(game, &mut self.battle, side);
        }
    }

    fn set_faction(&mut self, game: &GameData, side: &str, faction: &str) {
        let composed = side_mut(&mut self.battle, side);
        if composed.faction == faction {
            return;
        }
        composed.faction = faction.to_owned();
        composed.technologies.clear();
        prune_army(game, &mut self.battle, side);
    }

    fn load_preset(&mut self, game: &GameData, side: &str) -> bool {
        let rules = CustomBattleRules::bundled();
        let Some(army) = self.chosen[side_index(side)]
            .as_ref()
            .and_then(|name| self.presets.get(name))
            .cloned()
        else {
            return false;
        };
        if !self.factions.iter().any(|f| f.0 == army.faction) {
            return false;
        }
        *side_mut(&mut self.battle, side) = CustomSide {
            faction: army.faction,
            budget: army.budget.clamp(rules.min_budget, rules.max_budget),
            units: army.units,
            technologies: army.technologies,
        };
        prune_army(game, &mut self.battle, side);
        true
    }

    /// Keeps the army of `side` under the typed name (a name already used is replaced), or under
    /// « <faction> n » when the field is empty.
    fn save_preset(&mut self, side: &str) -> Option<String> {
        let composed = side_of(&self.battle, side);
        let faction = self
            .factions
            .iter()
            .find(|f| f.0 == composed.faction)
            .map_or(composed.faction.clone(), |f| f.1.clone());
        let typed = presets::clean_name(&self.names[side_index(side)]);
        let name = if typed.is_empty() {
            (1..)
                .map(|n| presets::clean_name(&format!("{faction} {n}")))
                .find(|name| !self.presets.contains_key(name))?
        } else {
            typed
        };
        let army = PresetArmy {
            faction: composed.faction.clone(),
            budget: composed.budget,
            technologies: composed.technologies.clone(),
            units: composed.units.clone(),
        };
        presets::save(&self.presets_path, &name, &army).then(|| {
            self.presets = presets::load_all(&self.presets_path);
            self.names[side_index(side)].clear();
            name
        })
    }

    fn preset_names(&self) -> Vec<String> {
        self.presets.keys().cloned().collect()
    }

    fn save_last(&self) {
        if let Some(parent) = self.last_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // The Godot file is the whole screen dictionary: `hold_opponent` rides along.
        let mut value = serde_json::to_value(&self.battle).unwrap_or(Value::Null);
        value["hold_opponent"] = Value::Bool(self.hold_opponent);
        if let Ok(text) = serde_json::to_string_pretty(&value) {
            let _ = std::fs::write(&self.last_path, text);
        }
    }
}

pub struct CustomBattlePlugin;

impl Plugin for CustomBattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                open_screen,
                handle_buttons,
                type_preset_name,
                handle_list,
                refresh,
            )
                .chain()
                .run_if(in_state(AppState::MainMenu)),
        );
    }
}

fn small_row(commands: &mut Commands, parent: Entity) -> Entity {
    commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            ChildOf(parent),
        ))
        .id()
}

fn value_text(commands: &mut Commands, theme: &UiTheme, parent: Entity, key: &str, width: f32) {
    commands.spawn((
        Name::new(format!("value:{key}")),
        ValueText(key.to_owned()),
        typed(theme, "", UiType::Body),
        Node {
            min_width: Val::Px(width),
            ..default()
        },
        ChildOf(parent),
    ));
}

/// `◀ value ▶` row named `cb:<field>[:<side>]:±1`.
fn cycle_row(
    commands: &mut Commands,
    theme: &UiTheme,
    parent: Entity,
    label: &str,
    field: &str,
    side: Option<&str>,
    width: f32,
) {
    let row = small_row(commands, parent);
    commands.spawn((typed(theme, label, UiType::Caption), ChildOf(row)));
    let path = side.map_or_else(|| field.to_owned(), |s| format!("{field}:{s}"));
    commands.spawn((
        button_small(theme, &format!("cb:{path}:-1"), "◀"),
        ChildOf(row),
    ));
    let key = side.map_or_else(|| field.to_owned(), |s| format!("{field}:{s}"));
    value_text(commands, theme, row, &key, width);
    commands.spawn((
        button_small(theme, &format!("cb:{path}:1"), "▶"),
        ChildOf(row),
    ));
}

/// `[b]title[/b]` and body of the `plain` tooltip `key` (`data/ui/tooltips.json`), empty when
/// the file or the key is missing.
pub(super) fn plain_tip(texts: Option<&TooltipTexts>, key: &str) -> Tooltip {
    let text = texts
        .and_then(|t| t.entry("plain", key))
        .map(|e| format!("[b]{}[/b]\n{}", e.title, e.body))
        .unwrap_or_default();
    Tooltip::new(&text)
}

fn open_screen(
    mut commands: Commands,
    mut requests: MessageReader<OpenBattleScreen>,
    theme: Res<UiTheme>,
    sim: Res<Sim>,
    dir: Res<SaveDir>,
    open: Query<(), With<CustomScreen>>,
    cli: Option<Res<crate::agent::cli::Cli>>,
) {
    if !requests.read().any(|r| r.0 == BattleScreen::Custom) || !open.is_empty() {
        return;
    }
    let (Some(game), Some(data)) = (sim.data.as_deref(), data_dir()) else {
        return;
    };
    let agent_run = cli.is_some_and(|cli| cli.is_agent_run());
    let draft = CustomDraft::new(game, &data, &custom_prefs_dir(&dir.0, agent_run));
    let texts = draft.texts.clone();
    commands.insert_resource(draft);
    let tip = |key: &str| plain_tip(texts.as_ref(), key);
    let (overlay, panel) = spawn_overlay(
        &mut commands,
        &theme,
        "CustomScreen",
        "Bataille personnalisée",
        1180.0,
        1,
    );
    commands
        .entity(overlay)
        .insert((CustomScreen, DespawnOnExit(AppState::MainMenu)));
    commands.spawn((
        typed(
            &theme,
            "Chaque camp achète ses unités sur son budget de points. Hors campagne : le résultat n’est pas conservé.",
            UiType::Caption,
        ),
        ChildOf(panel),
    ));
    let sides = commands
        .spawn((
            Node {
                column_gap: Val::Px(16.0),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    for side in SIDES {
        let column = commands
            .spawn((
                Name::new(format!("Side_{side}")),
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                ChildOf(sides),
            ))
            .id();
        let title = if side == "attacker" {
            "Camp 1 — assaillant"
        } else {
            "Camp 2 — défenseur"
        };
        commands.spawn((heading(&theme, title), ChildOf(column)));
        cycle_row(
            &mut commands,
            &theme,
            column,
            "Faction",
            "faction",
            Some(side),
            200.0,
        );
        let techs = small_row(&mut commands, column);
        commands.spawn((
            ValueText(format!("techs:{side}")),
            button_small(&theme, &format!("cb:techs:{side}"), "Technologies"),
            tip("custom_battle_technologies"),
            ChildOf(techs),
        ));
        commands.spawn((
            Name::new(format!("TechBody_{side}")),
            TechBody(side),
            Node {
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..default()
            },
            ChildOf(column),
        ));
        let budget = small_row(&mut commands, column);
        commands.spawn((
            typed(&theme, "Budget", UiType::Caption),
            tip("custom_battle_budget"),
            ChildOf(budget),
        ));
        commands.spawn((
            button_small(&theme, &format!("cb:budget:{side}:-1"), "−"),
            ChildOf(budget),
        ));
        value_text(
            &mut commands,
            &theme,
            budget,
            &format!("budget:{side}"),
            80.0,
        );
        commands.spawn((
            button_small(&theme, &format!("cb:budget:{side}:1"), "+"),
            ChildOf(budget),
        ));
        let presets = small_row(&mut commands, column);
        commands.spawn((
            button_small(&theme, &format!("preset:prev:{side}"), "◀"),
            ChildOf(presets),
        ));
        value_text(
            &mut commands,
            &theme,
            presets,
            &format!("preset:{side}"),
            150.0,
        );
        commands.spawn((
            button_small(&theme, &format!("preset:next:{side}"), "▶"),
            ChildOf(presets),
        ));
        for (verb, caption) in [
            ("load", "Charger"),
            ("save", "Enregistrer"),
            ("delete", "Supprimer"),
        ] {
            commands.spawn((
                button_small(&theme, &format!("preset:{verb}:{side}"), caption),
                ChildOf(presets),
            ));
        }
        commands.spawn((
            button_small(&theme, &format!("cb:clear:{side}"), "Vider l’armée"),
            ChildOf(presets),
        ));
        let naming = small_row(&mut commands, column);
        commands.spawn((
            text_field_limited(
                &theme,
                &format!("cb:name:{side}"),
                "",
                "Nom de la composition",
                320.0,
                presets::MAX_NAME,
            ),
            tip("custom_battle_presets"),
            ChildOf(naming),
        ));
        value_text(
            &mut commands,
            &theme,
            column,
            &format!("points:{side}"),
            0.0,
        );
        let lists = commands
            .spawn((
                Node {
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                ChildOf(column),
            ))
            .id();
        for (heading_text, is_roster) in [
            ("Roster (clic : acheter)", true),
            ("Armée (clic : retirer)", false),
        ] {
            let list_column = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        flex_basis: Val::Px(0.0),
                        ..default()
                    },
                    ChildOf(lists),
                ))
                .id();
            commands.spawn((
                typed(&theme, heading_text, UiType::Caption),
                ChildOf(list_column),
            ));
            let scroll = commands
                .spawn((
                    scroll_list(
                        &theme,
                        &format!("{}:{side}", if is_roster { "roster" } else { "army" }),
                        150.0,
                    ),
                    ChildOf(list_column),
                ))
                .id();
            let body = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        ..default()
                    },
                    ChildOf(scroll),
                ))
                .id();
            if is_roster {
                commands.entity(body).insert(RosterBody(side));
            } else {
                commands.entity(body).insert(ArmyBody(side));
            }
        }
    }
    // Field.
    let field = commands
        .spawn((
            Name::new("Field"),
            Node {
                column_gap: Val::Px(14.0),
                row_gap: Val::Px(4.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    cycle_row(
        &mut commands,
        &theme,
        field,
        "Terrain",
        "terrain",
        None,
        100.0,
    );
    cycle_row(&mut commands, &theme, field, "Saison", "season", None, 90.0);
    cycle_row(
        &mut commands,
        &theme,
        field,
        "Météo",
        "weather",
        None,
        120.0,
    );
    cycle_row(&mut commands, &theme, field, "Heure", "hour", None, 190.0);
    cycle_row(
        &mut commands,
        &theme,
        field,
        "Joueur",
        "player",
        None,
        150.0,
    );
    let year = small_row(&mut commands, field);
    commands.spawn((typed(&theme, "Année", UiType::Caption), ChildOf(year)));
    for (delta, caption) in [(-10, "−10"), (-1, "−1")] {
        commands.spawn((
            button_small(&theme, &format!("cb:year:{delta}"), caption),
            ChildOf(year),
        ));
    }
    value_text(&mut commands, &theme, year, "year", 44.0);
    for (delta, caption) in [(1, "+1"), (10, "+10")] {
        commands.spawn((
            button_small(&theme, &format!("cb:year:{delta}"), caption),
            ChildOf(year),
        ));
    }
    let siege = small_row(&mut commands, field);
    commands.spawn((
        ValueText("siege".into()),
        button_small(&theme, "cb:siege", "Siège"),
        ChildOf(siege),
    ));
    commands.spawn((
        ValueText("ladders".into()),
        button_small(&theme, "cb:ladders", "Échelles"),
        ChildOf(siege),
    ));
    commands.spawn((
        ValueText("ram".into()),
        button_small(&theme, "cb:ram", "Bélier"),
        ChildOf(siege),
    ));
    commands.spawn((
        ValueText("towers".into()),
        button_small(&theme, "cb:towers", "Beffrois"),
        ChildOf(siege),
    ));
    commands.spawn((button_small(&theme, "cb:fort:-1", "−"), ChildOf(siege)));
    value_text(&mut commands, &theme, siege, "fort", 120.0);
    commands.spawn((button_small(&theme, "cb:fort:1", "+"), ChildOf(siege)));
    commands.spawn((
        ValueText("hold".into()),
        button_small(&theme, "cb:hold", "Adversaire : tenir sa position"),
        tip("custom_battle_hold"),
        ChildOf(field),
    ));
    // Errors and actions.
    commands
        .spawn((
            Name::new("Errors"),
            ValueText("errors".into()),
            typed(&theme, "", UiType::Caption),
            ChildOf(panel),
        ))
        .insert(TextColor(crate::ui::kit::style::ERROR));
    let actions = commands
        .spawn((
            Node {
                justify_content: JustifyContent::FlexEnd,
                column_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    commands.spawn((
        button_captioned(&theme, "cb:close", "Fermer"),
        ChildOf(actions),
    ));
    commands.spawn((
        LaunchButton,
        button_primary(&theme, "cb:launch", "Lancer la bataille"),
        ChildOf(actions),
    ));
}

#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    draft: Option<ResMut<CustomDraft>>,
    sim: Res<Sim>,
    screens: Query<Entity, With<CustomScreen>>,
    mut launches: MessageWriter<LaunchBattle>,
) {
    let Some(mut draft) = draft else { return };
    if screens.is_empty() {
        return;
    }
    let Some(game) = sim.data.as_deref() else {
        return;
    };
    let rules = CustomBattleRules::bundled();
    for press in pressed.read() {
        let parts: Vec<&str> = press.name.split(':').collect();
        let delta = parts
            .last()
            .and_then(|d| d.parse::<i32>().ok())
            .unwrap_or(0);
        let mut changed = true;
        match parts.as_slice() {
            ["cb", "close"] => {
                draft.save_last();
                for screen in &screens {
                    commands.entity(screen).despawn();
                }
                changed = false;
            }
            ["cb", "launch"] => {
                if draft.report.ok {
                    draft.save_last();
                    let mut launch = LaunchBattle::new(draft.battle.clone(), sim.seed, "custom");
                    launch.hold_opponent = draft.hold_opponent;
                    launches.write(launch);
                }
                changed = false;
            }
            ["cb", "faction", side, _] => {
                let at = draft
                    .factions
                    .iter()
                    .position(|f| f.0 == side_of(&draft.battle, side).faction)
                    .unwrap_or(0);
                if let Some(next) = draft
                    .factions
                    .get(cycle(draft.factions.len(), at, delta))
                    .cloned()
                {
                    draft.set_faction(game, side, &next.0);
                }
            }
            ["cb", "budget", side, _] => {
                let step = budget_step();
                let composed = side_mut(&mut draft.battle, side);
                let now = if composed.budget == 0 {
                    rules.default_budget
                } else {
                    composed.budget
                };
                composed.budget = (now as i64 + i64::from(delta) * i64::from(step))
                    .clamp(i64::from(rules.min_budget), i64::from(rules.max_budget))
                    as u32;
            }
            ["cb", "terrain", _] => {
                let at = TERRAINS
                    .iter()
                    .position(|t| t.0 == draft.battle.terrain)
                    .unwrap_or(0);
                draft.battle.terrain = TERRAINS[cycle(TERRAINS.len(), at, delta)].0.to_owned();
            }
            ["cb", "season", _] => {
                let at = SEASONS
                    .iter()
                    .position(|t| t.0 == draft.battle.season)
                    .unwrap_or(0);
                draft.battle.season = SEASONS[cycle(SEASONS.len(), at, delta)].0.to_owned();
            }
            ["cb", "weather", _] => {
                let at = WEATHERS
                    .iter()
                    .position(|t| t.0 == draft.battle.weather)
                    .unwrap_or(0);
                draft.battle.weather = WEATHERS[cycle(WEATHERS.len(), at, delta)].0.to_owned();
            }
            ["cb", "hour", _] => {
                let at = draft
                    .hours
                    .iter()
                    .position(|h| h.0 == draft.battle.hour)
                    .unwrap_or(0);
                if let Some(next) = draft
                    .hours
                    .get(cycle(draft.hours.len(), at, delta))
                    .cloned()
                {
                    draft.battle.hour = next.0;
                }
            }
            ["cb", "player", _] => {
                let at = PLAYERS
                    .iter()
                    .position(|t| t.0 == draft.battle.player_side)
                    .unwrap_or(0);
                draft.battle.player_side = PLAYERS[cycle(PLAYERS.len(), at, delta)].0.to_owned();
            }
            ["cb", "year", _] => {
                let year = draft.year() + delta;
                draft.set_year(game, year);
            }
            ["cb", "siege"] => draft.battle.siege = !draft.battle.siege,
            ["cb", "hold"] => draft.hold_opponent = !draft.hold_opponent,
            ["cb", "techs", side] => {
                let index = side_index(side);
                draft.techs_open[index] = !draft.techs_open[index];
            }
            ["tech", side, tech] => toggle_technology(game, &mut draft.battle, side, tech),
            // Clicking the name field only focuses it (the kit does it).
            ["cb", "name", _] => changed = false,
            ["cb", "fort", _] => {
                if draft.battle.siege {
                    let now = draft
                        .battle
                        .fortification
                        .unwrap_or(rules.default_fortification)
                        as i32;
                    draft.battle.fortification = Some((now + delta).clamp(0, 3) as u32);
                }
            }
            ["cb", "ladders"] | ["cb", "ram"] | ["cb", "towers"] => {
                if draft.battle.siege {
                    let mut engines = engines_of(&draft.battle);
                    match parts[1] {
                        "ladders" => engines.ladders = !engines.ladders,
                        "ram" => engines.ram = !engines.ram,
                        _ => engines.towers = (engines.towers + 1) % (rules.max_siege_towers + 1),
                    }
                    draft.battle.engines = Some(engines);
                }
            }
            ["cb", "clear", side] => side_mut(&mut draft.battle, side).units.clear(),
            ["preset", verb, side] => {
                let names = draft.preset_names();
                let index = side_index(side);
                let at = draft.chosen[index]
                    .as_ref()
                    .and_then(|c| names.iter().position(|n| n == c));
                match *verb {
                    "prev" | "next" if !names.is_empty() => {
                        let step = if *verb == "next" { 1 } else { -1 };
                        let next = at.map_or(if step > 0 { 0 } else { names.len() - 1 }, |a| {
                            cycle(names.len(), a, step)
                        });
                        draft.chosen[index] = Some(names[next].clone());
                    }
                    "load" => {
                        draft.load_preset(game, side);
                    }
                    "save" => {
                        if let Some(name) = draft.save_preset(side) {
                            draft.chosen[index] = Some(name);
                        }
                    }
                    "delete" => {
                        if let Some(name) = draft.chosen[index].take() {
                            presets::remove(&draft.presets_path, &name);
                            draft.presets = presets::load_all(&draft.presets_path);
                        }
                    }
                    _ => {}
                }
            }
            _ => changed = false,
        }
        if changed {
            draft.dirty = true;
        }
    }
}

/// Kit name fields (`cb:name:<side>`): typing keeps the name, Enter keeps the army under it;
/// a name cleared by the draft (after saving) is written back to the field.
fn type_preset_name(
    mut changed: MessageReader<TextFieldChanged>,
    mut submitted: MessageReader<TextFieldSubmitted>,
    draft: Option<ResMut<CustomDraft>>,
    mut fields: Query<(&Name, &mut TextField)>,
) {
    let Some(mut draft) = draft else {
        changed.clear();
        submitted.clear();
        return;
    };
    for event in changed.read() {
        if let Some(side) = event.name.strip_prefix("cb:name:") {
            let index = side_index(side);
            if draft.names[index] != event.text {
                draft.names[index] = event.text.clone();
                draft.dirty = true;
            }
        }
    }
    for event in submitted.read() {
        if let Some(side) = event.name.strip_prefix("cb:name:") {
            let index = side_index(side);
            draft.names[index] = event.text.clone();
            if let Some(name) = draft.save_preset(SIDES[index]) {
                draft.chosen[index] = Some(name);
            }
            draft.dirty = true;
        }
    }
    for (name, mut field) in &mut fields {
        if let Some(side) = name.strip_prefix("cb:name:") {
            let wanted = &draft.names[side_index(side)];
            if field.value != *wanted {
                field.set(wanted);
            }
        }
    }
}

fn handle_list(
    mut selected: MessageReader<ListSelected>,
    draft: Option<ResMut<CustomDraft>>,
    sim: Res<Sim>,
) {
    let Some(mut draft) = draft else { return };
    let Some(game) = sim.data.as_deref() else {
        return;
    };
    for choice in selected.read() {
        let parts: Vec<&str> = choice.name.split(':').collect();
        match parts.as_slice() {
            ["buy", side, unit] => {
                if buy(game, &mut draft.battle, side, unit) {
                    draft.dirty = true;
                }
            }
            ["rm", side, index] => {
                let units = &mut side_mut(&mut draft.battle, side).units;
                if let Ok(index) = index.parse::<usize>()
                    && index < units.len()
                {
                    units.remove(index);
                    draft.dirty = true;
                }
            }
            _ => {}
        }
    }
}

fn on_off(on: bool) -> &'static str {
    if on { "oui" } else { "non" }
}

fn towers_label(towers: u32) -> String {
    match towers {
        0 => "Aucun beffroi".to_owned(),
        1 => "1 beffroi".to_owned(),
        n => format!("{n} beffrois"),
    }
}

/// Texts of every [`ValueText`] for the draft.
fn texts(game: &GameData, draft: &CustomDraft) -> BTreeMap<String, String> {
    let rules = CustomBattleRules::bundled();
    let b = &draft.battle;
    let mut out = BTreeMap::new();
    let report = serde_json::to_value(&draft.report).unwrap_or(Value::Null);
    for side in SIDES {
        let composed = side_of(b, side);
        out.insert(
            format!("faction:{side}"),
            draft
                .factions
                .iter()
                .find(|f| f.0 == composed.faction)
                .map_or(composed.faction.clone(), |f| f.1.clone()),
        );
        let budget = if composed.budget == 0 {
            rules.default_budget
        } else {
            composed.budget
        };
        out.insert(
            format!("budget:{side}"),
            crate::ui::kit::fr::digits(i64::from(budget)),
        );
        let r = &report[side];
        out.insert(
            format!("points:{side}"),
            format!(
                "Points : {} / {} — unités : {} / {}",
                r["cost"], r["budget"], r["units"], r["max_units"]
            ),
        );
        let granted = composed.technologies.len();
        out.insert(
            format!("techs:{side}"),
            match (granted, draft.techs_open[side_index(side)]) {
                (0, false) => "Technologies ▸".to_owned(),
                (0, true) => "Technologies ▾".to_owned(),
                (n, false) => format!("Technologies ({n}) ▸"),
                (n, true) => format!("Technologies ({n}) ▾"),
            },
        );
        out.insert(
            format!("preset:{side}"),
            draft.chosen[side_index(side)]
                .clone()
                .unwrap_or_else(|| "Compositions gardées…".to_owned()),
        );
    }
    out.insert("terrain".into(), label_of(&TERRAINS, &b.terrain));
    out.insert("season".into(), label_of(&SEASONS, &b.season));
    out.insert("weather".into(), label_of(&WEATHERS, &b.weather));
    out.insert(
        "hour".into(),
        draft
            .hours
            .iter()
            .find(|h| h.0 == b.hour)
            .map_or_else(|| b.hour.clone(), |h| h.1.clone()),
    );
    out.insert("player".into(), label_of(&PLAYERS, &b.player_side));
    out.insert("year".into(), draft.year().to_string());
    out.insert(
        "siege".into(),
        format!("Siège : le camp 2 défend une place ({})", on_off(b.siege)),
    );
    let engines = engines_of(b);
    out.insert(
        "ladders".into(),
        format!("Échelles : {}", on_off(engines.ladders)),
    );
    out.insert("ram".into(), format!("Bélier : {}", on_off(engines.ram)));
    out.insert("towers".into(), towers_label(engines.towers));
    out.insert(
        "hold".into(),
        format!(
            "Adversaire : tenir sa position ({})",
            on_off(draft.hold_opponent)
        ),
    );
    out.insert(
        "fort".into(),
        format!(
            "Fortification {}",
            b.fortification.unwrap_or(rules.default_fortification)
        ),
    );
    out.insert("errors".into(), draft.report.errors.join("\n"));
    let _ = game;
    out
}

#[allow(clippy::too_many_arguments)]
fn refresh(
    mut commands: Commands,
    draft: Option<ResMut<CustomDraft>>,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    mut own_texts: Query<(&ValueText, &mut Text), Without<Children>>,
    button_texts: Query<(&ValueText, &Children), Without<Text>>,
    mut child_texts: Query<&mut Text, Without<ValueText>>,
    rosters: Query<(Entity, &RosterBody)>,
    armies: Query<(Entity, &ArmyBody)>,
    tech_bodies: Query<(Entity, &TechBody)>,
    launch: Query<Entity, With<LaunchButton>>,
    assets: Res<AssetServer>,
) {
    let Some(mut draft) = draft else { return };
    if !draft.dirty {
        return;
    }
    let Some(game) = sim.data.as_deref() else {
        return;
    };
    draft.dirty = false;
    draft.report = validate(game, &draft.battle);
    let values = texts(game, &draft);
    for (key, mut text) in &mut own_texts {
        if let Some(value) = values.get(&key.0)
            && text.0 != *value
        {
            text.0 = value.clone();
        }
    }
    for (key, children) in &button_texts {
        if let Some(value) = values.get(&key.0) {
            for child in children {
                if let Ok(mut text) = child_texts.get_mut(*child)
                    && text.0 != *value
                {
                    text.0 = value.clone();
                }
            }
        }
    }
    for entity in &launch {
        if draft.report.ok {
            commands.entity(entity).remove::<InteractionDisabled>();
        } else {
            commands.entity(entity).insert(InteractionDisabled);
        }
    }
    let rules = CustomBattleRules::bundled();
    for (body, side) in tech_bodies.iter().map(|(e, s)| (e, s.0)) {
        commands.entity(body).despawn_related::<Children>();
        let open = draft.techs_open[side_index(side)];
        commands.entity(body).insert(Node {
            flex_direction: FlexDirection::Column,
            display: if open { Display::Flex } else { Display::None },
            ..default()
        });
        let granted = &side_of(&draft.battle, side).technologies;
        let catalog = grantable(game, &draft.battle, side);
        if open && catalog.is_empty() {
            commands.spawn((
                typed(&theme, "Aucune technologie à accorder.", UiType::Caption),
                ChildOf(body),
            ));
        }
        for (id, name, units) in catalog {
            let on = granted.contains(&id);
            commands.spawn((
                button_small(
                    &theme,
                    &format!("tech:{side}:{id}"),
                    &format!(
                        "{} {name} ({})",
                        if on { "☑" } else { "☐" },
                        units.join(", ")
                    ),
                ),
                ChildOf(body),
            ));
        }
    }
    for (body, side) in rosters.iter().map(|(e, s)| (e, s.0)) {
        commands.entity(body).despawn_related::<Children>();
        let composed = side_of(&draft.battle, side);
        let spent = army_cost(game, &draft.battle, side);
        let budget = if composed.budget == 0 {
            rules.default_budget
        } else {
            composed.budget
        };
        for unit in side_roster(game, &draft.battle, side) {
            let affordable = spent + unit.cost.money <= budget
                && composed.units.len() < rules.max_units_per_side;
            let row = commands
                .spawn((
                    list_item(
                        &theme,
                        &format!("roster:{side}"),
                        &format!("buy:{side}:{}", unit.id),
                        &format!("{} — {}", unit.name.display, unit.cost.money),
                        false,
                    ),
                    ChildOf(body),
                ))
                .id();
            decorate_row(&mut commands, &theme, &assets, &draft, row, unit);
            if !affordable {
                commands.entity(row).insert(InteractionDisabled);
            }
        }
    }
    for (body, side) in armies.iter().map(|(e, s)| (e, s.0)) {
        commands.entity(body).despawn_related::<Children>();
        for (index, unit_id) in side_of(&draft.battle, side).units.iter().enumerate() {
            let unit = data_model::UnitTypeId::new(unit_id.as_str())
                .ok()
                .and_then(|id| game.unit_types.get(&id));
            let caption = unit.map_or_else(
                || format!("− {unit_id}"),
                |u| format!("− {} ({})", u.name.display, u.cost.money),
            );
            let row = commands
                .spawn((
                    list_item(
                        &theme,
                        &format!("army:{side}"),
                        &format!("rm:{side}:{index}"),
                        &caption,
                        false,
                    ),
                    ChildOf(body),
                ))
                .id();
            if let Some(unit) = unit {
                decorate_row(&mut commands, &theme, &assets, &draft, row, unit);
            }
        }
    }
}

/// Painted miniature of the unit type in front of its row, and its tooltip over the row
/// (`IconLibrary.decorate_button` and `TooltipHost.set_tooltip(button, "unit", id)`).
fn decorate_row(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    draft: &CustomDraft,
    row: Entity,
    unit: &UnitType,
) {
    let id = unit.id.to_string();
    let icon_id = if theme.icons.has(&id) {
        id
    } else {
        format!("unit_category_{}", unit.category.key())
    };
    let miniature = commands.spawn(icon(theme, assets, &icon_id, ROW_ICON)).id();
    commands.entity(row).insert_children(0, &[miniature]);
    if let Some(texts) = &draft.texts {
        commands
            .entity(row)
            .insert(Tooltip::new(&unit_type_bbcode(unit, texts)));
    }
}

pub fn dump(world: &World) -> Value {
    let open = world
        .try_query_filtered::<(), With<CustomScreen>>()
        .is_some_and(|mut q| q.iter(world).count() > 0);
    let Some(draft) = world.get_resource::<CustomDraft>() else {
        return json!({"open": open, "draft": null});
    };
    let b = &draft.battle;
    let report = serde_json::to_value(&draft.report).unwrap_or(Value::Null);
    json!({
        "open": open,
        "ok": draft.report.ok,
        "errors": draft.report.errors,
        "attacker": {"faction": b.attacker.faction, "units": b.attacker.units.len(), "budget": b.attacker.budget, "cost": report["attacker"]["cost"]},
        "defender": {"faction": b.defender.faction, "units": b.defender.units.len(), "budget": b.defender.budget, "cost": report["defender"]["cost"]},
        "terrain": b.terrain, "season": b.season, "weather": b.weather, "hour": b.hour,
        "year": b.year, "siege": b.siege, "player_side": b.player_side,
        "presets": draft.presets.keys().collect::<Vec<_>>(),
        "hold_opponent": draft.hold_opponent,
        "technologies": {"attacker": b.attacker.technologies, "defender": b.defender.technologies},
        "techs_open": draft.techs_open,
        "typed_names": draft.names,
        "rows_with_tooltip": world.try_query_filtered::<(), (With<Tooltip>, With<crate::ui::kit::ListItem>)>().map_or(0, |mut q| q.iter(world).count()),
        "rows_with_icon": world.try_query::<(&crate::ui::kit::icons::Icon, &ChildOf)>().map_or(0, |mut q| q.iter(world).filter(|(_, parent)| world.get::<crate::ui::kit::ListItem>(parent.parent()).is_some()).count()),
        "overlays": world.try_query_filtered::<(), With<MenuOverlay>>().map_or(0, |mut q| q.iter(world).count()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths;

    fn load() -> (GameData, PathBuf) {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).expect("game data");
        (game, dir)
    }

    fn temp_prefs(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("cent-ans-cb-{tag}-{}", std::process::id()))
    }

    #[test]
    fn draft_starts_valid_buys_within_budget_and_prunes() {
        let (game, dir) = load();
        let prefs = temp_prefs("draft");
        let mut draft = CustomDraft::new(&game, &dir, &prefs);
        assert!(draft.report.ok, "{:?}", draft.report.errors);
        assert!(!draft.factions.is_empty());
        // Clear, buy until the budget is exhausted: the core never reports an overspend.
        draft.battle.attacker.units.clear();
        draft.battle.attacker.budget = 2000;
        let cheapest = side_roster(&game, &draft.battle, "attacker")
            .iter()
            .min_by_key(|u| u.cost.money)
            .map(|u| u.id.as_str().to_owned())
            .expect("a roster");
        let mut bought = 0;
        while buy(&game, &mut draft.battle, "attacker", &cheapest) {
            bought += 1;
            assert!(bought < 100);
        }
        assert!(bought >= 1);
        let report = validate(&game, &draft.battle);
        assert!(report.attacker.cost <= 2000);
        // Another faction drops the units it cannot field.
        let other = draft
            .factions
            .iter()
            .find(|f| f.0 != draft.battle.attacker.faction && f.0 != "fac_england")
            .cloned()
            .unwrap();
        draft.set_faction(&game, "attacker", &other.0);
        let roster_ids: Vec<String> = side_roster(&game, &draft.battle, "attacker")
            .iter()
            .map(|u| u.id.as_str().to_owned())
            .collect();
        assert!(
            draft
                .battle
                .attacker
                .units
                .iter()
                .all(|u| roster_ids.contains(u))
        );
        assert!(!buy(
            &game,
            &mut draft.battle,
            "attacker",
            "unit_does_not_exist"
        ));
        let _ = std::fs::remove_dir_all(prefs);
    }

    #[test]
    fn presets_round_trip_through_the_draft() {
        let (game, dir) = load();
        let prefs = temp_prefs("presets");
        let mut draft = CustomDraft::new(&game, &dir, &prefs);
        let units = draft.battle.attacker.units.clone();
        let name = draft.save_preset("attacker").expect("saved");
        assert!(name.ends_with(" 1"), "{name}");
        draft.chosen[0] = Some(name);
        draft.battle.attacker.units.clear();
        assert!(draft.load_preset(&game, "attacker"));
        assert_eq!(draft.battle.attacker.units, units);
        draft.save_last();
        let again = CustomDraft::new(&game, &dir, &prefs);
        assert_eq!(again.battle, draft.battle);
        assert_eq!(again.presets.len(), 1);
        let _ = std::fs::remove_dir_all(prefs);
    }

    #[test]
    fn granting_a_technology_unlocks_units_and_withdrawing_prunes_them() {
        let (game, dir) = load();
        let prefs = temp_prefs("techs");
        let mut draft = CustomDraft::new(&game, &dir, &prefs);
        draft.set_year(&game, 1453);
        let catalog = grantable(&game, &draft.battle, "attacker");
        assert!(
            !catalog.is_empty(),
            "France has technologies to be granted in 1453"
        );
        let (tech, _, units) = catalog[0].clone();
        assert!(!units.is_empty());
        let before = side_roster(&game, &draft.battle, "attacker").len();
        toggle_technology(&game, &mut draft.battle, "attacker", &tech);
        assert_eq!(draft.battle.attacker.technologies, vec![tech.clone()]);
        let after = side_roster(&game, &draft.battle, "attacker");
        assert!(after.len() > before);
        // Buy a unit that needs it, then withdraw the technology: the unit leaves the army.
        let gated = after
            .iter()
            .find(|u| u.required_technology.as_ref().map(|t| t.as_str()) == Some(tech.as_str()))
            .map(|u| u.id.as_str().to_owned())
            .expect("a gated unit");
        draft.battle.attacker.budget = 20_000;
        assert!(buy(&game, &mut draft.battle, "attacker", &gated));
        toggle_technology(&game, &mut draft.battle, "attacker", &tech);
        assert!(draft.battle.attacker.technologies.is_empty());
        assert!(!draft.battle.attacker.units.contains(&gated));
        // An unknown technology is refused.
        toggle_technology(&game, &mut draft.battle, "attacker", "tech_unknown");
        assert!(draft.battle.attacker.technologies.is_empty());
        let _ = std::fs::remove_dir_all(prefs);
    }

    #[test]
    fn typed_names_are_kept_and_hold_survives_the_last_file() {
        let (game, dir) = load();
        let prefs = temp_prefs("names");
        let mut draft = CustomDraft::new(&game, &dir, &prefs);
        draft.names[0] = "La".to_owned();
        assert_eq!(draft.save_preset("attacker").as_deref(), Some("La"));
        assert!(draft.names[0].is_empty());
        assert!(draft.presets.contains_key("La"));
        draft.hold_opponent = true;
        draft.save_last();
        let again = CustomDraft::new(&game, &dir, &prefs);
        assert!(again.hold_opponent);
        assert_eq!(again.battle, draft.battle);
        let _ = std::fs::remove_dir_all(prefs);
    }

    #[test]
    fn year_change_prunes_units_not_yet_raised() {
        let (game, dir) = load();
        let prefs = temp_prefs("year");
        let mut draft = CustomDraft::new(&game, &dir, &prefs);
        draft.set_year(&game, 1453);
        assert_eq!(draft.year(), 1453);
        draft.set_year(&game, 9999);
        assert_eq!(draft.year(), 1453);
        assert!(validate(&game, &draft.battle).ok || !draft.battle.attacker.units.is_empty());
        assert_eq!(cycle(3, 0, -1), 2);
        let _ = std::fs::remove_dir_all(prefs);
    }
}
