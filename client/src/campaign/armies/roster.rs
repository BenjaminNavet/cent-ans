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
//! Not ported: wax-seal portraits of the leaders, alert pictograms with tooltips (alerts are a
//! word in the row), range gauge bars (a percentage in the text), Tab cycling inside the list.

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
use crate::ui::kit::style::UiType;
use crate::ui::kit::widgets::{button_small, caption, panel_sized, scroll_list};
use crate::ui::kit::{ButtonPressed, UiTheme, fr, typed};

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

    fn glyph(self) -> &'static str {
        match self {
            ArmyState::Stationed => "·",
            ArmyState::Embarked => "≈",
            ArmyState::Siege => "⚔",
            ArmyState::Camp | ArmyState::Entrenched => "▲",
            ArmyState::March => "»",
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
                    handle_buttons,
                    watch_state,
                    rebuild,
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
) {
    if keymap.just_pressed("map_toggle_units", &keys) {
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

fn army_caption(entry: &ArmyEntry, selected: bool) -> String {
    let mark = if selected { "▶ " } else { "" };
    let dim = if entry.ratio <= 0.0 { "— " } else { "" };
    let alerts: Vec<&str> = entry.alerts.iter().map(|a| alert_text(a)).collect();
    let mut head = format!(
        "{mark}{dim}{}   {} {}",
        entry.title,
        entry.state.glyph(),
        entry.state.label()
    );
    if !alerts.is_empty() {
        head.push_str(&format!("   ! {}", alerts.join(", ")));
    }
    format!(
        "{head}\n{}/{} hommes · moral {} · vivres {} %\n{} · {}",
        fr::digits(i64::from(entry.men)),
        fr::digits(i64::from(entry.max_men)),
        entry.morale,
        entry.supply,
        entry.place,
        entry.range
    )
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    mut roster: ResMut<Roster>,
    sim: Res<Sim>,
    glides: Res<Glides>,
    selected: Res<SelectedArmy>,
    theme: Res<UiTheme>,
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
    let theme = theme.clone();
    let selected_id = selected.0.as_ref().map(|a| a.as_str().to_owned());
    let (sort, filters) = (roster.sort, roster.filters);
    let (fold_armies, fold_agents) = (roster.fold_armies, roster.fold_agents);
    let root = commands
        .spawn((
            Name::new("UnitRoster"),
            RosterRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(72.0),
                ..default()
            },
            GlobalZIndex(20),
        ))
        .with_children(|root| {
            root.spawn(panel_sized(&theme, PANEL_WIDTH))
                .with_children(|panel| {
                    panel.spawn(typed(&theme, "Montre des hommes d’armes", UiType::Title));
                    panel.spawn(button_small(&theme, "roster:close", "Fermer"));
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
                                for entry in &armies {
                                    let on = selected_id.as_deref() == Some(entry.id.as_str());
                                    list.spawn(button_small(
                                        &theme,
                                        &format!("roster:army:{}", entry.id),
                                        &army_caption(entry, on),
                                    ));
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
