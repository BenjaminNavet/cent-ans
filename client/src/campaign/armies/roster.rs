//! « Montre des hommes d'armes » (key U, action `map_toggle_units`): the player's armies and
//! agents as a list in the style of Total War. Port of game/scripts/map/unit_roster_controller.gd.
//! Everything comes from the simulation (`game_api::army::army_sheet`, `agents_of`); a click on a
//! row selects the army and sends `FocusCamera::Army` (an agent: `FocusCamera::Point` on his
//! colony). Armies without movement left are listed dimmed (caption `—`), alerts and sort modes
//! (`Alertes`, `Effectif`, `Mouvement`, `Lieu`) and the three filters follow UX5-U.
//!
//! Widget names: `roster:army:<id>`, `roster:agent:<id>`, `roster:sort:alert|men|move|place`,
//! `roster:filter:leaderless|siege|spent`, `roster:fold:armies|agents`, `roster:close`.
//! Closes on Escape (before the pause menu). Dev hook: `CENT_ANS_OPEN_ROSTER=1` opens it on
//! entering the campaign.
//!
//! Lot armies6: each army row is a two-level card like Godot's — wax seal of the leader (living
//! portrait, else the arms of the faction, hatched empty seal without a leader), title, pictogram
//! chips with tooltips (state, alerts), men / morale / supply, a range gauge bar and the place.
//! Tab / Shift+Tab (`campaign_next_idle` / `campaign_prev_idle`) walk the armies of the open
//! list in the displayed order ([`next_in_cycle`]); `camera::focus` leaves the key to the list
//! while it is open.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use data_model::{GameData, ProvinceId};
use game_api::army::{ArmySheet, army_sheet};
use serde_json::{Value, json};
use sim_campaign::CampaignState;

use super::sea::Glides;
use super::{ArmyCommand, SelectedArmy};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::FocusCamera;
use crate::menu::KeyEdges;
use crate::settings::KeyMap;
use crate::sim::Sim;
use crate::ui::kit::close_button;
use crate::ui::kit::style::UiType;
use crate::ui::kit::text_field::{TextInputFocus, keyboard_free};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_small, caption, panel_sized, scroll_list};
use crate::ui::kit::{ButtonPressed, UiTheme, fr, style, typed};

const PANEL_WIDTH: f32 = 360.0;
const LIST_HEIGHT: f32 = 380.0;
/// UX5-U alert thresholds (display only: the state comes from the core).
pub const LOW_SUPPLY: u8 = 40;
pub const UNDERSTRENGTH_RATIO: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Alert,
    Men,
    Move,
    Place,
}

impl SortMode {
    const ALL: [SortMode; 4] = [
        SortMode::Alert,
        SortMode::Men,
        SortMode::Move,
        SortMode::Place,
    ];

    fn key(self) -> &'static str {
        match self {
            SortMode::Alert => "alert",
            SortMode::Men => "men",
            SortMode::Move => "move",
            SortMode::Place => "place",
        }
    }

    fn label(self) -> &'static str {
        match self {
            SortMode::Alert => "Alertes",
            SortMode::Men => "Effectif",
            SortMode::Move => "Mouvement",
            SortMode::Place => "Lieu",
        }
    }

    fn parse(key: &str) -> Option<SortMode> {
        Self::ALL.into_iter().find(|m| m.key() == key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Leaderless,
    Siege,
    Spent,
}

impl Filter {
    const ALL: [Filter; 3] = [Filter::Leaderless, Filter::Siege, Filter::Spent];

    fn key(self) -> &'static str {
        match self {
            Filter::Leaderless => "leaderless",
            Filter::Siege => "siege",
            Filter::Spent => "spent",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Filter::Leaderless => "Sans chef",
            Filter::Siege => "En siège",
            Filter::Spent => "À bout",
        }
    }
}

/// What an army is doing (`state` of `army_entry`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmyState {
    Stationed,
    Embarked,
    Siege,
    /// Entrenched in the open: exposed.
    Camp,
    Entrenched,
    March,
}

impl ArmyState {
    pub fn label(self) -> &'static str {
        match self {
            ArmyState::Stationed => "Stationné",
            ArmyState::Embarked => "Embarqué, en mer",
            ArmyState::Siege => "Au siège",
            ArmyState::Camp => "Campement retranché, exposé en rase campagne",
            ArmyState::Entrenched => "Retranché",
            ArmyState::March => "En marche",
        }
    }
}

/// One army row.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmyEntry {
    pub id: String,
    pub title: String,
    pub men: u32,
    pub max_men: u32,
    pub morale: u32,
    pub supply: u8,
    pub state: ArmyState,
    pub alerts: Vec<&'static str>,
    pub leaderless: bool,
    pub left: u32,
    /// Movement left over the allowance, 0-1.
    pub ratio: f32,
    pub place: String,
    pub range: String,
    pub units: usize,
    /// Commander (character id) and faction, for the wax seal.
    pub general_id: Option<String>,
    pub faction: String,
    /// Tooltip of the row (`tooltip` of `army_entry`).
    pub tooltip: String,
}

/// Alert word (`_alert_text`, short).
pub fn alert_text(alert: &str) -> &'static str {
    match alert {
        "leaderless" => "sans chef",
        "supply" => "vivres bas",
        "strength" => "sous-effectif",
        "spent" => "à bout",
        _ => "",
    }
}

/// Entry of an army from its sheet (`army_entry`). `embarked`: sailing a voyage.
pub fn army_entry(
    sheet: &ArmySheet,
    place: String,
    embarked: bool,
    units_morale: u32,
) -> ArmyEntry {
    let leaderless = sheet.general.is_none();
    let title = if sheet.general_name.is_empty() {
        "Ost sans chef".to_owned()
    } else {
        format!("Ost de {}", sheet.general_name)
    };
    let left = sheet.movement_left;
    let allowance = sheet.movement_max.max(1);
    let mut ratio = (left as f32 / allowance as f32).clamp(0.0, 1.0);
    let range = if embarked {
        ratio = 0.0;
        "En mer".to_owned()
    } else if left == 0 {
        "Plus de mouvement cette saison".to_owned()
    } else {
        format!(
            "Portée ≈ {} km de plaine ({} %)",
            sheet.movement_km.round() as i64,
            (ratio * 100.0).round() as i64
        )
    };
    let state = if embarked {
        ArmyState::Embarked
    } else if sheet.stance == "siege" {
        ArmyState::Siege
    } else if sheet.stance == "entrenched" && sheet.settlement.is_none() {
        ArmyState::Camp
    } else if sheet.stance == "entrenched" {
        ArmyState::Entrenched
    } else if sheet.marching {
        ArmyState::March
    } else {
        ArmyState::Stationed
    };
    let mut alerts = Vec::new();
    if leaderless {
        alerts.push("leaderless");
    }
    if sheet.supply < LOW_SUPPLY {
        alerts.push("supply");
    }
    if sheet.max_strength > 0
        && (sheet.strength as f32) < UNDERSTRENGTH_RATIO * sheet.max_strength as f32
    {
        alerts.push("strength");
    }
    if left == 0 && !embarked {
        alerts.push("spent");
    }
    let detail = format!(
        "{} · {} hommes · {}",
        fr::count(sheet.units.len() as i64, "unité", None),
        fr::digits(i64::from(sheet.strength)),
        place
    );
    let tooltip = format!("{title}\n{detail}\n{range}\nClic : sélectionner et centrer la carte.");
    ArmyEntry {
        id: sheet.id.clone(),
        title,
        men: sheet.strength,
        max_men: sheet.max_strength,
        morale: units_morale,
        supply: sheet.supply,
        state,
        alerts,
        leaderless,
        left,
        ratio,
        place,
        range,
        units: sheet.units.len(),
        general_id: sheet.general.clone(),
        faction: sheet.faction.clone(),
        tooltip,
    }
}

/// Mean morale of the regiments (0 for none).
pub fn mean_morale(sheet: &ArmySheet) -> u32 {
    if sheet.units.is_empty() {
        return 0;
    }
    let sum: u32 = sheet.units.iter().map(|u| u32::from(u.morale)).sum();
    (sum as f32 / sheet.units.len() as f32).round() as u32
}

/// Filters (all the checked conditions) then sort (`_sorted_filtered`).
pub fn sorted_filtered(
    entries: &[ArmyEntry],
    mode: SortMode,
    filters: &[bool; 3],
) -> Vec<ArmyEntry> {
    let mut result: Vec<ArmyEntry> = entries
        .iter()
        .filter(|e| !filters[0] || e.leaderless)
        .filter(|e| !filters[1] || e.state == ArmyState::Siege)
        .filter(|e| !filters[2] || e.alerts.contains(&"spent"))
        .cloned()
        .collect();
    result.sort_by(|a, b| {
        let ordering = match mode {
            SortMode::Men => b.men.cmp(&a.men),
            SortMode::Move => b.ratio.total_cmp(&a.ratio),
            SortMode::Place => a.place.to_lowercase().cmp(&b.place.to_lowercase()),
            SortMode::Alert => b.alerts.len().cmp(&a.alerts.len()),
        };
        ordering.then_with(|| a.id.cmp(&b.id))
    });
    result
}

/// One agent row.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentEntry {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub range: String,
    pub ratio: f32,
    /// Map pixel of his colony.
    pub point: Option<[f32; 2]>,
}

fn agent_kind(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "spy" => ("E", "Espion"),
        "emissary" => ("H", "Héraut"),
        "preacher" => ("P", "Prédicateur"),
        "merchant" => ("M", "Marchand"),
        _ => ("✦", "Agent"),
    }
}

fn agent_entries(
    data: &GameData,
    state: &CampaignState,
    player: &data_model::FactionId,
) -> Vec<AgentEntry> {
    state
        .agents_of(player)
        .into_iter()
        .map(|(id, agent)| {
            let (letter, label) = agent_kind(agent.kind.key());
            let mut detail = format!(
                "{} · niveau {}",
                data.settlement_name(&agent.location),
                agent.level
            );
            if agent.acted {
                detail.push_str(" — a agi cette saison");
            }
            let allowance = state.agent_movement_allowance(data, agent.kind).max(1);
            let ratio = (agent.movement_points as f32 / allowance as f32).clamp(0.0, 1.0);
            let range = if agent.movement_points == 0 {
                "Plus de mouvement cette saison".to_owned()
            } else {
                let targets = state
                    .agent_reachable(data, id)
                    .keys()
                    .filter(|s| **s != agent.location)
                    .count();
                format!(
                    "Portée : {} à portée",
                    fr::count(targets as i64, "colonie", None)
                )
            };
            AgentEntry {
                id: id.to_string(),
                title: format!("{letter} {} — {label}", agent.name),
                detail,
                range,
                ratio,
                point: data.settlement_point(&agent.location),
            }
        })
        .collect()
}

/// State of the list, kept across rebuilds.
#[derive(Resource, Default)]
pub struct Roster {
    pub open: bool,
    root: Option<Entity>,
    sort: SortMode,
    filters: [bool; 3],
    fold_armies: bool,
    fold_agents: bool,
    dirty: bool,
    /// Turn and selection the list was built for.
    built_for: (u32, Option<String>),
    pub rows: usize,
    pub army_rows: usize,
    pub agent_rows: usize,
    pub foot: String,
    pub army_ids: Vec<String>,
    pub last_focus: Option<String>,
    pub builds: u32,
    /// Rows whose seal shows a portrait, and Tab cycles done (dump).
    pub seals: usize,
    pub cycles: u32,
}

#[derive(Component)]
struct RosterRoot;

pub struct RosterPlugin;

impl Plugin for RosterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Roster>()
            .add_systems(OnEnter(AppState::Campaign), reset_on_enter)
            .add_systems(
                Update,
                (
                    open_close.before(crate::menu::pause::pause_escape),
                    tab_cycle,
                    handle_buttons,
                    watch_state,
                    rebuild,
                    row_visuals,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), close_on_exit)
            .register_dump(dump_roster);
    }
}

fn reset_on_enter(mut roster: ResMut<Roster>) {
    *roster = Roster::default();
    if std::env::var_os("CENT_ANS_OPEN_ROSTER").is_some() {
        roster.open = true;
        roster.dirty = true;
    }
}

fn close_on_exit(mut commands: Commands, mut roster: ResMut<Roster>) {
    if let Some(root) = roster.root.take() {
        commands.entity(root).despawn();
    }
    roster.open = false;
}

fn open_close(
    mut roster: ResMut<Roster>,
    keymap: Res<KeyMap>,
    keys: Res<ButtonInput<KeyCode>>,
    mut edges: ResMut<KeyEdges>,
    focus: Option<Res<TextInputFocus>>,
) {
    if keyboard_free(focus) && keymap.just_pressed("map_toggle_units", &keys) {
        roster.open = !roster.open;
        roster.dirty = roster.open;
    }
    if roster.open && edges.escape {
        roster.open = false;
        edges.escape = false;
    }
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut roster: ResMut<Roster>,
    sim: Res<Sim>,
    mut focus: MessageWriter<FocusCamera>,
    mut commands: MessageWriter<ArmyCommand>,
) {
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("roster:") else {
            continue;
        };
        if !roster.open {
            continue;
        }
        if name == "close" {
            roster.open = false;
        } else if let Some(mode) = name.strip_prefix("sort:").and_then(SortMode::parse) {
            roster.sort = mode;
        } else if let Some(key) = name.strip_prefix("filter:") {
            if let Some(index) = Filter::ALL.iter().position(|f| f.key() == key) {
                roster.filters[index] = !roster.filters[index];
            }
        } else if let Some(section) = name.strip_prefix("fold:") {
            match section {
                "armies" => roster.fold_armies = !roster.fold_armies,
                _ => roster.fold_agents = !roster.fold_agents,
            }
        } else if let Some(id) = name.strip_prefix("army:") {
            if let Some(army) = sim_campaign::ArmyId::parse(id) {
                commands.write(ArmyCommand::Select(Some(army)));
                focus.write(FocusCamera::Army(id.to_owned()));
                roster.last_focus = Some(format!("army:{id}"));
            }
        } else if let Some(id) = name.strip_prefix("agent:") {
            let point = sim
                .data
                .as_ref()
                .zip(sim.state.as_ref())
                .and_then(|(data, state)| {
                    let agent = sim_campaign::AgentId::parse(id).and_then(|a| state.agent(&a))?;
                    data.settlement_point(&agent.location)
                });
            if let Some(point) = point {
                focus.write(FocusCamera::Point(Vec2::new(point[0], point[1])));
                roster.last_focus = Some(format!("agent:{id}"));
            }
        }
        roster.dirty = true;
    }
}

/// Rebuilds when the turn or the selected army changed under the open list.
fn watch_state(mut roster: ResMut<Roster>, sim: Res<Sim>, selected: Res<SelectedArmy>) {
    if !roster.open {
        return;
    }
    let key = (
        sim.state.as_ref().map_or(0, |s| s.turn),
        selected.0.as_ref().map(|a| a.as_str().to_owned()),
    );
    if sim.is_changed() || key != roster.built_for {
        roster.dirty = true;
    }
}

fn place_of(data: &GameData, sheet: &ArmySheet) -> String {
    if let Some(settlement) = sheet.settlement.as_deref()
        && let Ok(id) = data_model::SettlementId::new(settlement)
    {
        return data.settlement_name(&id);
    }
    match sheet
        .location_province
        .as_deref()
        .and_then(|p| ProvinceId::new(p).ok())
    {
        Some(province) => format!("en campagne, {}", data.province_name(&province)),
        None => "en campagne".to_owned(),
    }
}

/// Entries of the player's armies.
pub fn player_entries(sim: &Sim, glides: &Glides) -> Vec<ArmyEntry> {
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return Vec::new();
    };
    state
        .armies()
        .iter()
        .filter(|(_, army)| army.faction == sim.player)
        .filter_map(|(id, _)| {
            let sheet = army_sheet(data, state, id)?;
            let embarked = glides.active.contains_key(id);
            Some(army_entry(
                &sheet,
                place_of(data, &sheet),
                embarked,
                mean_morale(&sheet),
            ))
        })
        .collect()
}

/// Pictogram chip of a state or an alert (`_state_glyph`, `_alert_glyph`): the ink icon
/// `hud_<glyph>` when it exists, else a letter.
#[derive(Clone, Copy)]
struct Chip {
    icon: &'static str,
    letter: &'static str,
}

fn state_chip(state: ArmyState) -> Chip {
    match state {
        ArmyState::Siege => Chip {
            icon: "hud_stance_siege",
            letter: "⚔",
        },
        ArmyState::Camp | ArmyState::Entrenched => Chip {
            icon: "hud_stance_entrenched",
            letter: "▲",
        },
        ArmyState::March => Chip {
            icon: "hud_movement",
            letter: "»",
        },
        ArmyState::Embarked => Chip {
            icon: "",
            letter: "≈",
        },
        ArmyState::Stationed => Chip {
            icon: "",
            letter: "·",
        },
    }
}

fn alert_chip(alert: &str) -> Chip {
    match alert {
        "supply" => Chip {
            icon: "hud_supply",
            letter: "!",
        },
        "strength" => Chip {
            icon: "",
            letter: "▾",
        },
        "spent" => Chip {
            icon: "hud_movement",
            letter: "!",
        },
        _ => Chip {
            icon: "",
            letter: "!",
        },
    }
}

/// Tooltip of a state chip (`_state_text`).
pub fn state_tip(state: ArmyState) -> &'static str {
    state.label()
}

/// Tooltip of an alert chip (`_alert_text`).
pub fn alert_tip(alert: &str, entry: &ArmyEntry) -> String {
    match alert {
        "leaderless" => "Sans chef : nommez un capitaine depuis la cour.".to_owned(),
        "supply" => format!("Vivres bas : {} % (seuil {LOW_SUPPLY} %).", entry.supply),
        "strength" => format!(
            "Sous-effectif : {}/{} hommes (moins de {} % du complet).",
            fr::digits(i64::from(entry.men)),
            fr::digits(i64::from(entry.max_men)),
            (UNDERSTRENGTH_RATIO * 100.0).round() as i64
        ),
        "spent" => "Plus de mouvement cette saison.".to_owned(),
        other => other.to_owned(),
    }
}

/// Selectable army row (look driven by [`row_visuals`]).
#[derive(Component)]
struct RosterRow {
    selected: bool,
}

const SEAL_RADIUS: f32 = 17.0;
const GAUGE_WIDTH: f32 = 52.0;

fn small_text(theme: &UiTheme, content: &str, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(content.to_owned()),
        theme.text_font(false, false, size),
        TextColor(color),
    )
}

/// One pictogram chip with its tooltip; boxed (rubric frame) for alerts.
#[allow(clippy::too_many_arguments)]
fn spawn_chip(
    line: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    name: String,
    chip: Chip,
    tip: &str,
    color: Color,
    boxed: bool,
) {
    let has_icon = !chip.icon.is_empty() && theme.icons.has(chip.icon);
    let mut holder = line.spawn((
        Name::new(name),
        Tooltip::new(tip),
        Node {
            padding: UiRect::all(Val::Px(2.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(3.0)),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(if boxed {
            style::PARCHMENT_LIGHT
        } else {
            Color::NONE
        }),
        BorderColor::all(if boxed { color } else { Color::NONE }),
    ));
    holder.with_children(|holder| {
        if has_icon {
            holder.spawn(crate::ui::kit::icons::icon_tinted(
                theme, assets, chip.icon, 16.0, color,
            ));
        } else {
            holder.spawn(small_text(
                theme,
                chip.letter,
                UiType::Caption.size(),
                color,
            ));
        }
    });
}

/// Wax seal of the leader: portrait, else the arms of the faction; hatched-looking empty seal
/// (faded disc, rubric rim) without a leader.
fn spawn_seal(
    row: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    entry: &ArmyEntry,
    portrait: &str,
) {
    let size = SEAL_RADIUS * 2.0 + 4.0;
    let empty = entry.leaderless;
    let mut seal = row.spawn((
        Name::new(format!("roster:seal:{}", entry.id)),
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(if empty { 1.5 } else { 2.5 })),
            border_radius: BorderRadius::MAX,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(if empty {
            style::PARCHMENT_DARK
        } else {
            style::WAX
        }),
        BorderColor::all(if empty { style::RUBRIC } else { style::GOLD }),
    ));
    seal.with_children(|seal| {
        if empty {
            seal.spawn(small_text(
                theme,
                "?",
                UiType::Body.size(),
                style::INK_FADED,
            ));
        } else if !portrait.is_empty() {
            seal.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(assets.load(portrait.to_owned())),
            ));
        } else {
            seal.spawn((
                Node {
                    width: Val::Percent(70.0),
                    height: Val::Percent(70.0),
                    ..default()
                },
                ImageNode::new(assets.load(format!("heraldry/{}.png", entry.faction))),
            ));
        }
    });
}

/// Two-level army card (`_add_army_row`).
#[allow(clippy::too_many_arguments)]
fn spawn_army_row(
    list: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    entry: &ArmyEntry,
    selected: bool,
    portrait: &str,
) {
    let faded = entry.ratio <= 0.0;
    let ink = if faded { style::INK_FADED } else { style::INK };
    let title = format!("{}{}", if selected { "▶ " } else { "" }, entry.title);
    list.spawn((
        Name::new(format!("roster:army:{}", entry.id)),
        bevy::ui_widgets::Button,
        Hovered::default(),
        RosterRow { selected },
        Tooltip::new(&entry.tooltip),
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(66.0),
            padding: UiRect::all(Val::Px(5.0)),
            column_gap: Val::Px(6.0),
            border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(style::PARCHMENT_LIGHT),
        BorderColor::all(style::PARCHMENT_DARK),
    ))
    .with_children(|row| {
        spawn_seal(row, theme, assets, entry, portrait);
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            min_width: Val::Px(0.0),
            row_gap: Val::Px(1.0),
            overflow: Overflow::clip_x(),
            ..default()
        })
        .with_children(|column| {
            column
                .spawn(Node {
                    column_gap: Val::Px(3.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|line| {
                    line.spawn((
                        Node {
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            min_width: Val::Px(0.0),
                            overflow: Overflow::clip_x(),
                            ..default()
                        },
                        children![small_text(theme, &title, UiType::Body.size(), ink)],
                    ));
                    spawn_chip(
                        line,
                        theme,
                        assets,
                        format!("State_{:?}", entry.state),
                        state_chip(entry.state),
                        state_tip(entry.state),
                        style::INK_SOFT,
                        false,
                    );
                    for alert in &entry.alerts {
                        spawn_chip(
                            line,
                            theme,
                            assets,
                            format!("Alert_{alert}"),
                            alert_chip(alert),
                            &alert_tip(alert, entry),
                            style::RUBRIC,
                            true,
                        );
                    }
                });
            column
                .spawn(Node {
                    column_gap: Val::Px(6.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|line| {
                    let stats = format!(
                        "{}/{} · moral {} · vivres {} %",
                        fr::digits(i64::from(entry.men)),
                        fr::digits(i64::from(entry.max_men)),
                        entry.morale,
                        entry.supply
                    );
                    line.spawn((
                        Node {
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            min_width: Val::Px(0.0),
                            overflow: Overflow::clip_x(),
                            ..default()
                        },
                        children![small_text(
                            theme,
                            &stats,
                            UiType::Caption.size(),
                            style::INK_SOFT
                        )],
                    ));
                    line.spawn((
                        Name::new(format!("roster:gauge:{}", entry.id)),
                        Node {
                            width: Val::Px(GAUGE_WIDTH),
                            height: Val::Px(8.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(style::PARCHMENT_DARK),
                        children![(
                            Node {
                                width: Val::Percent(entry.ratio.clamp(0.0, 1.0) * 100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(style::gauge_color(entry.ratio)),
                        )],
                    ));
                });
            column.spawn((
                Node {
                    overflow: Overflow::clip_x(),
                    ..default()
                },
                children![small_text(
                    theme,
                    &entry.place,
                    UiType::Caption.size() - 1.0,
                    style::INK_FADED
                )],
            ));
        });
    });
}

/// Hover and selection look of the army rows (`card_box` hover gold / pressed wax).
fn row_visuals(mut rows: Query<(&RosterRow, &Hovered, &mut BackgroundColor, &mut BorderColor)>) {
    for (row, hovered, mut background, mut border) in &mut rows {
        let (fill, rim) = if row.selected {
            (style::GOLD_PALE, style::WAX)
        } else if hovered.get() {
            (style::PARCHMENT_LIGHT, style::GOLD)
        } else {
            (style::PARCHMENT_LIGHT, style::PARCHMENT_DARK)
        };
        if background.0 != fill {
            background.0 = fill;
        }
        let wanted = BorderColor::all(rim);
        if *border != wanted {
            *border = wanted;
        }
    }
}

/// Army after (`step` 1) or before (`step` -1) `current` in the displayed order, cyclically
/// (`CampaignHotkeys.next_in_cycle`): an absent `current` gives the first (next) or the last
/// (previous); `None` for an empty list.
pub fn next_in_cycle(ids: &[String], current: Option<&str>, step: i32) -> Option<String> {
    if ids.is_empty() {
        return None;
    }
    let index = current.and_then(|c| ids.iter().position(|id| id == c));
    let next = match index {
        None if step >= 0 => 0,
        None => ids.len() - 1,
        Some(i) => {
            (i as i32 + if step >= 0 { 1 } else { -1 }).rem_euclid(ids.len() as i32) as usize
        }
    };
    ids.get(next).cloned()
}

/// Tab / Shift+Tab walk the armies of the open list (selection and camera as a click).
fn tab_cycle(
    keymap: Res<KeyMap>,
    keys: Res<ButtonInput<KeyCode>>,
    focus_text: Option<Res<TextInputFocus>>,
    mut roster: ResMut<Roster>,
    selected: Res<SelectedArmy>,
    mut focus: MessageWriter<FocusCamera>,
    mut commands: MessageWriter<ArmyCommand>,
) {
    if !roster.open || roster.army_ids.is_empty() || !keyboard_free(focus_text) {
        return;
    }
    let step = if keymap.just_pressed("campaign_prev_idle", &keys) {
        -1
    } else if keymap.just_pressed("campaign_next_idle", &keys) {
        1
    } else {
        return;
    };
    let current = selected.0.as_ref().map(|a| a.as_str());
    let Some(target) = next_in_cycle(&roster.army_ids, current, step) else {
        return;
    };
    if let Some(army) = sim_campaign::ArmyId::parse(&target) {
        commands.write(ArmyCommand::Select(Some(army)));
        focus.write(FocusCamera::Army(target.clone()));
        roster.last_focus = Some(format!("army:{target}"));
        roster.cycles += 1;
        roster.dirty = true;
    }
}

/// Portrait asset path of the commander of `entry` (empty: none, the arms are used).
fn portrait_of(
    sim: &Sim,
    bank: Option<&crate::campaign::panels::court::portrait::PortraitBank>,
    entry: &ArmyEntry,
) -> String {
    let (Some(bank), Some(general)) = (bank, entry.general_id.as_deref()) else {
        return String::new();
    };
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return String::new();
    };
    data_model::CharacterId::new(general)
        .ok()
        .and_then(|id| game_api::court::character_sheet(data, state, &id))
        .map(|sheet| {
            bank.resolve(&crate::campaign::panels::court::sheet_subject(&sheet))
                .path
        })
        .unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    mut roster: ResMut<Roster>,
    sim: Res<Sim>,
    glides: Res<Glides>,
    selected: Res<SelectedArmy>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    bank: Option<Res<crate::campaign::panels::court::portrait::PortraitBank>>,
) {
    if !roster.open {
        if let Some(root) = roster.root.take() {
            commands.entity(root).despawn();
        }
        return;
    }
    if !roster.dirty && roster.root.is_some() {
        return;
    }
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    roster.dirty = false;
    if let Some(root) = roster.root.take() {
        commands.entity(root).despawn();
    }
    let all = player_entries(&sim, &glides);
    let armies = sorted_filtered(&all, roster.sort, &roster.filters);
    let agents = agent_entries(data, state, &sim.player);
    let summary = state.faction_summary(&sim.player);
    let foot = format!(
        "Montre du royaume : {} hommes · {} · entretien {} livres par saison",
        fr::digits(all.iter().map(|e| i64::from(e.men)).sum()),
        fr::count(all.len() as i64, "ost", None),
        fr::digits(summary.as_ref().map_or(0, |s| s.army_upkeep)),
    );
    roster.built_for = (
        state.turn,
        selected.0.as_ref().map(|a| a.as_str().to_owned()),
    );
    roster.army_rows = armies.len();
    roster.agent_rows = agents.len();
    roster.rows = if roster.fold_armies { 0 } else { armies.len() }
        + if roster.fold_agents { 0 } else { agents.len() };
    roster.foot = foot.clone();
    roster.army_ids = armies.iter().map(|e| e.id.clone()).collect();
    roster.builds += 1;
    let count_text = if armies.len() == all.len() {
        armies.len().to_string()
    } else {
        format!("{}/{}", armies.len(), all.len())
    };
    let portraits: Vec<String> = armies
        .iter()
        .map(|entry| portrait_of(&sim, bank.as_deref(), entry))
        .collect();
    roster.seals = armies
        .iter()
        .zip(&portraits)
        .filter(|(entry, path)| !entry.leaderless && !path.is_empty())
        .count();
    let theme = theme.clone();
    let assets = assets.clone();
    let selected_id = selected.0.as_ref().map(|a| a.as_str().to_owned());
    let (sort, filters) = (roster.sort, roster.filters);
    let (fold_armies, fold_agents) = (roster.fold_armies, roster.fold_agents);
    let root = commands
        .spawn((
            Name::new("UnitRoster"),
            RosterRoot,
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            crate::ui::kit::layout::FitZone::Side,
            GlobalZIndex(20),
        ))
        .with_children(|root| {
            root.spawn(panel_sized(&theme, PANEL_WIDTH))
                .with_children(|panel| {
                    panel.spawn(typed(&theme, "Montre des hommes d’armes", UiType::Title));
                    panel.spawn(close_button(&theme, "roster:close"));
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(4.0),
                            row_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|bar| {
                            bar.spawn(caption(&theme, "Trier"));
                            for mode in SortMode::ALL {
                                let label = if mode == sort {
                                    format!("• {}", mode.label())
                                } else {
                                    mode.label().to_owned()
                                };
                                bar.spawn(button_small(
                                    &theme,
                                    &format!("roster:sort:{}", mode.key()),
                                    &label,
                                ));
                            }
                        });
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(4.0),
                            row_gap: Val::Px(4.0),
                            ..default()
                        })
                        .with_children(|bar| {
                            bar.spawn(caption(&theme, "Montrer"));
                            for (index, filter) in Filter::ALL.into_iter().enumerate() {
                                let label = if filters[index] {
                                    format!("• {}", filter.label())
                                } else {
                                    filter.label().to_owned()
                                };
                                bar.spawn(button_small(
                                    &theme,
                                    &format!("roster:filter:{}", filter.key()),
                                    &label,
                                ));
                            }
                        });
                    panel
                        .spawn(scroll_list(&theme, "RosterList", LIST_HEIGHT))
                        .with_children(|list| {
                            list.spawn(button_small(
                                &theme,
                                "roster:fold:armies",
                                &format!(
                                    "{} Armées ({count_text})",
                                    if fold_armies { "▸" } else { "▾" }
                                ),
                            ));
                            if !fold_armies {
                                for (entry, portrait) in armies.iter().zip(&portraits) {
                                    let on = selected_id.as_deref() == Some(entry.id.as_str());
                                    spawn_army_row(list, &theme, &assets, entry, on, portrait);
                                }
                                if armies.is_empty() {
                                    list.spawn(caption(
                                        &theme,
                                        if all.is_empty() {
                                            "Aucune armée en campagne."
                                        } else {
                                            "Aucun ost ne correspond aux filtres."
                                        },
                                    ));
                                }
                            }
                            list.spawn(button_small(
                                &theme,
                                "roster:fold:agents",
                                &format!(
                                    "{} Agents ({})",
                                    if fold_agents { "▸" } else { "▾" },
                                    agents.len()
                                ),
                            ));
                            if !fold_agents {
                                for entry in &agents {
                                    list.spawn(button_small(
                                        &theme,
                                        &format!("roster:agent:{}", entry.id),
                                        &format!(
                                            "{}\n{}\n{}",
                                            entry.title, entry.detail, entry.range
                                        ),
                                    ));
                                }
                                if agents.is_empty() {
                                    list.spawn(caption(
                                        &theme,
                                        "Aucun agent à votre service (G pour recruter).",
                                    ));
                                }
                            }
                        });
                    panel.spawn(caption(&theme, &foot));
                });
        })
        .id();
    roster.root = Some(root);
}

fn dump_roster(world: &World) -> (String, Value) {
    let roster = world.resource::<Roster>();
    (
        "campaign_roster".into(),
        json!({
            "open": roster.open,
            "rows": roster.rows,
            "army_rows": roster.army_rows,
            "agent_rows": roster.agent_rows,
            "foot": roster.foot,
            "army_ids": roster.army_ids,
            "last_focus": roster.last_focus,
            "builds": roster.builds,
            "sort": roster.sort.key(),
            "seals_with_portrait": roster.seals,
            "tab_cycles": roster.cycles,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(id: &str, general: Option<&str>, left: u32, supply: u8, strength: u32) -> ArmySheet {
        ArmySheet {
            id: id.into(),
            name: id.into(),
            faction: "fac_france".into(),
            faction_name: "France".into(),
            general: general.map(str::to_owned),
            general_name: general.unwrap_or("").to_owned(),
            units: Vec::new(),
            strength,
            max_strength: 1000,
            position: [0.0, 0.0],
            settlement: None,
            location: None,
            location_province: None,
            movement_left: left,
            movement_max: 100,
            movement_km: 40.0,
            supply,
            stance: "normal".into(),
            marching: false,
            idle: false,
            recruit_blocker: None,
            merge_targets: Vec::new(),
        }
    }

    fn entry(id: &str, general: Option<&str>, left: u32, supply: u8, strength: u32) -> ArmyEntry {
        army_entry(
            &sheet(id, general, left, supply, strength),
            "Paris".into(),
            false,
            70,
        )
    }

    #[test]
    fn alerts_follow_the_thresholds() {
        let e = entry("army_1", None, 0, 30, 500);
        assert_eq!(e.alerts, vec!["leaderless", "supply", "strength", "spent"]);
        assert_eq!(e.title, "Ost sans chef");
        let ok = entry("army_2", Some("Jean"), 50, 90, 900);
        assert!(ok.alerts.is_empty());
        assert_eq!(ok.title, "Ost de Jean");
        assert!(ok.range.starts_with("Portée ≈ 40 km de plaine (50 %)"));
    }

    #[test]
    fn state_follows_stance_and_voyage() {
        let mut s = sheet("army_1", Some("Jean"), 50, 90, 900);
        s.stance = "entrenched".into();
        assert_eq!(
            army_entry(&s, String::new(), false, 0).state,
            ArmyState::Camp
        );
        s.settlement = Some("set_paris".into());
        assert_eq!(
            army_entry(&s, String::new(), false, 0).state,
            ArmyState::Entrenched
        );
        s.stance = "siege".into();
        assert_eq!(
            army_entry(&s, String::new(), false, 0).state,
            ArmyState::Siege
        );
        let at_sea = army_entry(&s, String::new(), true, 0);
        assert_eq!(at_sea.state, ArmyState::Embarked);
        assert_eq!(at_sea.range, "En mer");
        assert_eq!(at_sea.ratio, 0.0);
    }

    #[test]
    fn sorts_and_filters() {
        let list = vec![
            entry("army_1", Some("A"), 50, 90, 900),
            entry("army_2", None, 0, 90, 900),
            entry("army_3", Some("C"), 100, 20, 300),
        ];
        let by_alert = sorted_filtered(&list, SortMode::Alert, &[false; 3]);
        assert_eq!(by_alert[0].id, "army_2");
        let by_men = sorted_filtered(&list, SortMode::Men, &[false; 3]);
        assert_eq!(by_men.last().unwrap().id, "army_3");
        let by_move = sorted_filtered(&list, SortMode::Move, &[false; 3]);
        assert_eq!(by_move[0].id, "army_3");
        let leaderless = sorted_filtered(&list, SortMode::Alert, &[true, false, false]);
        assert_eq!(leaderless.len(), 1);
        let spent = sorted_filtered(&list, SortMode::Alert, &[false, false, true]);
        assert_eq!(spent[0].id, "army_2");
    }
}
