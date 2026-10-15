//! Campaign HUD: top bar, alerts, end-turn button and flow, army strip, map-mode bar.
//!
//! Ports game/scripts/map/hud_controller.gd, map_ui.gd (top bar, mode filter), ui/top_bar_fit.gd,
//! ui/alerts.gd (+ alert_routes.gd), ui/army_strip.gd, ui/end_turn_cluster.gd, ui/season_report.gd
//! and the basics of the chronicle decision popup. Data comes from `game-api` (`hud::top_bar`,
//! `economy`, `forecast`, `turn_readiness`, `army_sheet`); the end of turn is the core's
//! `end_turn_with(ai::plan_turn)` on a worker thread, as `godot-bridge` does.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.
//!
//! Not ported: the minimap (`campaign_minimap.gd`, `minimap_controller.gd`) — needs the terrain
//! render-to-texture; see `docs/wip/bm-hud.md`.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use game_api::hud::{EconomyView, ForecastView, ProvinceIncome};
use game_api::{Alert, TopBar, TurnReadiness};
use serde_json::{Value, json};
use sim_campaign::ArmyId;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::panels::OpenPanel;
use crate::campaign::provinces::picker::{ProvinceClickGuard, ProvincePickSet, SelectProvince};
use crate::campaign::provinces::{MapMode, MapModeState, SetMapMode};
use crate::sim::Sim;
use crate::ui::kit::modal::ModalQueue;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_primary, button_small, caption, panel_inset, set_disabled};
use crate::ui::kit::{ButtonPressed, UiTheme, fr};

pub mod decision;
pub mod flow;
pub mod report;
pub mod text;

use decision::OpenDecision;
use flow::TurnFlow;

/// Minimum seconds between two refreshes of the views while orders keep changing the state.
const REFRESH_SECONDS: f32 = 0.25;
/// Minimum seconds between two forecasts (a forecast simulates the next turn).
const FORECAST_SECONDS: f32 = 1.0;
const MAX_ALERTS: usize = 8;
const MAX_ARMIES: usize = 12;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum HudSet {
    Data,
    Flow,
    Ui,
}

/// One army of the strip.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmyEntry {
    pub id: String,
    pub name: String,
    pub men: u32,
    pub idle: bool,
    pub marching: bool,
    pub besieging: bool,
}

/// Cached views of the player's faction, refreshed when `Sim` changes (the forecast, which
/// simulates a turn, at most once a second and only for a new revision).
#[derive(Resource, Default)]
pub struct HudData {
    pub top: Option<TopBar>,
    pub economy: Option<EconomyView>,
    pub top_provinces: Vec<ProvinceIncome>,
    pub readiness: Option<TurnReadiness>,
    pub forecast: Option<ForecastView>,
    pub armies: Vec<ArmyEntry>,
    /// Bumped by every refresh of the views.
    pub revision: u64,
    forecast_revision: u64,
    forecasts_computed: u64,
    refreshes: u64,
    last_refresh: f32,
    last_forecast: f32,
    dirty: bool,
}

impl HudData {
    /// Asks for an immediate refresh at the next frame (after a turn).
    pub fn force_refresh(&mut self) {
        self.dirty = true;
        self.last_refresh = f32::NEG_INFINITY;
        self.last_forecast = f32::NEG_INFINITY;
    }

    /// Readiness computed now from `sim` (not from the cache).
    pub fn fresh_readiness(&self, sim: &Sim) -> Option<TurnReadiness> {
        let (data, state) = (sim.data.as_ref()?, sim.state.as_ref()?);
        Some(game_api::turn_readiness(data, state, &sim.player))
    }

    /// Factions at war with or allied to the player (the « Le monde » interest filter).
    pub fn interest_factions(&self) -> Vec<String> {
        self.top.as_ref().map_or_else(Vec::new, |top| {
            top.at_war_with.iter().chain(&top.allies).cloned().collect()
        })
    }
}

/// Which figure a top-bar text shows.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum HudField {
    Faction,
    Date,
    Treasury,
    Balance,
    Manpower,
    Prestige,
    Unrest,
    Realm,
}

impl HudField {
    const ALL: [(HudField, &'static str); 8] = [
        (HudField::Faction, "hud:faction"),
        (HudField::Date, "hud:date"),
        (HudField::Treasury, "hud:treasury"),
        (HudField::Balance, "hud:balance"),
        (HudField::Manpower, "hud:manpower"),
        (HudField::Prestige, "hud:prestige"),
        (HudField::Unrest, "hud:unrest"),
        (HudField::Realm, "hud:realm"),
    ];
}

/// Marks HUD widgets: while the pointer is on one, the map does not take the click.
#[derive(Component)]
struct HudWidget;

#[derive(Component)]
struct AlertList;

#[derive(Component)]
struct ArmyStrip;

#[derive(Component)]
struct ModeLegend;

#[derive(Component)]
struct EndTurnButton;

#[derive(Component)]
struct TurnWait;

/// What an alert button does.
#[derive(Component, Clone)]
struct AlertTarget(Alert);

#[derive(Component, Clone)]
struct ArmyTarget(String);

pub struct CampaignHudPlugin;

impl Plugin for CampaignHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudData>()
            .configure_sets(
                Update,
                (HudSet::Data, HudSet::Flow, HudSet::Ui)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_plugins((flow::plugin, decision::plugin))
            .register_dump(dump_hud)
            .add_systems(OnEnter(AppState::Campaign), spawn_hud)
            .add_systems(Update, refresh_data.in_set(HudSet::Data))
            .add_systems(
                Update,
                (
                    update_fields,
                    rebuild_alerts,
                    rebuild_armies,
                    update_end_turn,
                    update_legend,
                    handle_clicks,
                )
                    .chain()
                    .in_set(HudSet::Ui),
            )
            .add_systems(
                Update,
                block_map_clicks
                    .before(ProvincePickSet)
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

fn refresh_data(sim: Res<Sim>, time: Res<Time>, mut hud: ResMut<HudData>) {
    let now = time.elapsed_secs();
    if sim.is_changed() {
        hud.dirty = true;
    }
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    if hud.dirty && (hud.top.is_none() || now - hud.last_refresh >= REFRESH_SECONDS) {
        let player = &sim.player;
        hud.top = game_api::top_bar(data, state, player);
        hud.economy = game_api::economy(data, state, player);
        hud.top_provinces = game_api::hud::income_top_provinces(data, state, player, 3);
        hud.readiness = Some(game_api::turn_readiness(data, state, player));
        hud.armies = game_api::army_ids(state)
            .iter()
            .filter_map(|id| ArmyId::parse(id))
            .filter_map(|id| game_api::army_sheet(data, state, &id))
            .filter(|sheet| sheet.faction == player.as_str())
            .map(|sheet| ArmyEntry {
                men: sheet.strength,
                idle: sheet.idle,
                marching: sheet.marching,
                besieging: sheet.stance == "siege",
                name: sheet.name,
                id: sheet.id,
            })
            .collect();
        hud.dirty = false;
        hud.last_refresh = now;
        hud.revision += 1;
        hud.refreshes += 1;
    }
    if hud.forecast_revision != hud.revision
        && (hud.forecast.is_none() || now - hud.last_forecast >= FORECAST_SECONDS)
    {
        hud.forecast = game_api::forecast(data, state, &sim.player);
        hud.forecast_revision = hud.revision;
        hud.last_forecast = now;
        hud.forecasts_computed += 1;
        // The tooltips follow the forecast: make the UI sync run.
        hud.set_changed();
    }
}

fn block_map_clicks(
    widgets: Query<&Hovered, With<HudWidget>>,
    mut guard: ResMut<ProvinceClickGuard>,
) {
    if widgets.iter().any(Hovered::get) {
        guard.blocked = true;
    }
}

fn spawn_hud(mut commands: Commands, theme: Res<UiTheme>) {
    commands
        .spawn((
            Name::new("hud:root"),
            DespawnOnExit(AppState::Campaign),
            UiTier::Hud.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            // Top bar.
            root.spawn((
                Name::new("hud:top_bar"),
                HudWidget,
                Hovered::default(),
                panel_inset(&theme),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(22.0),
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                    ..default()
                },
            ))
            .with_children(|bar| {
                for (field, name) in HudField::ALL {
                    let font = if field == HudField::Faction {
                        theme.title_font(UiType::Heading.size())
                    } else {
                        theme.text_font(false, false, UiType::Body.size())
                    };
                    bar.spawn((
                        Name::new(name),
                        field,
                        Text::new(""),
                        font,
                        TextColor(style::INK),
                        Tooltip::new(""),
                    ));
                }
            });
            // Map-mode filter bar and its legend, under the top bar.
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(46.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|column| {
                column
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        for mode in MapMode::ALL {
                            row.spawn((
                                button_small(
                                    &theme,
                                    &format!("hud:mode:{}", mode.key()),
                                    mode.label_fr(),
                                ),
                                HudWidget,
                                Tooltip::new(mode.tooltip_fr()),
                            ));
                        }
                    });
                column.spawn((
                    Name::new("hud:mode_legend"),
                    ModeLegend,
                    Text::new(""),
                    theme.text_font(false, true, UiType::Caption.size()),
                    TextColor(style::INK),
                ));
                column.spawn((
                    Name::new("hud:alerts"),
                    AlertList,
                    HudWidget,
                    Hovered::default(),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                ));
            });
            // Army strip, bottom left.
            root.spawn((
                Name::new("hud:army_strip"),
                ArmyStrip,
                HudWidget,
                Hovered::default(),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(10.0),
                    left: Val::Px(10.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    max_width: Val::Percent(60.0),
                    ..default()
                },
            ));
            // Panels and end-turn button, bottom right.
            root.spawn(Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                right: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|column| {
                column.spawn((
                    Name::new("hud:turn_wait"),
                    TurnWait,
                    Text::new("Tour en cours…"),
                    theme.text_font(false, true, UiType::Heading.size()),
                    TextColor(style::RUBRIC),
                    Visibility::Hidden,
                ));
                column
                    .spawn(Node {
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        for (name, caption_fr) in [
                            ("hud:open_diplomacy", "Diplomatie"),
                            ("hud:open_tech", "Techniques"),
                            ("hud:open_recruitment", "Recrutement"),
                        ] {
                            row.spawn((button_small(&theme, name, caption_fr), HudWidget));
                        }
                    });
                column.spawn((
                    button_primary(&theme, "hud:end_turn", "Fin de saison"),
                    HudWidget,
                    EndTurnButton,
                    Tooltip::new("Terminer la saison (Entrée)"),
                ));
            });
        });
}

fn update_fields(
    hud: Res<HudData>,
    mut fields: Query<(&HudField, &mut Text, &mut TextColor, &mut Tooltip)>,
) {
    if !hud.is_changed() {
        return;
    }
    let Some(top) = &hud.top else { return };
    let [prestige_tip, men_tip, unrest_tip] = text::forecast_tooltips(hud.forecast.as_ref());
    let arrow = |gauge: Option<&game_api::hud::GaugeView>| {
        gauge.map_or_else(String::new, |g| format!(" {}", text::arrow(g)))
    };
    let forecast = hud.forecast.as_ref();
    for (field, mut content, mut color, mut tooltip) in &mut fields {
        let (value, ink, tip) = match field {
            HudField::Faction => (top.faction_name.clone(), style::INK_TITLE, String::new()),
            HudField::Date => (
                format!(
                    "{} {} · tour {}",
                    text::season_fr(&top.season),
                    top.year,
                    top.turn
                ),
                style::INK,
                format!("[b]{}[/b]\nDifficulté : {}", top.date_label, top.difficulty),
            ),
            HudField::Treasury => (
                format!("Trésor : {}", fr::amount(top.treasury)),
                if top.treasury < 0 {
                    fr::LOSS_COLOR
                } else {
                    style::INK
                },
                hud.economy.as_ref().map_or_else(String::new, |economy| {
                    text::treasury_tooltip(economy, &hud.top_provinces)
                }),
            ),
            HudField::Balance => (
                format!("Solde : {} / saison", fr::signed(top.net_income)),
                fr::color_of(top.net_income),
                hud.economy
                    .as_ref()
                    .map_or_else(String::new, text::budget_tooltip),
            ),
            HudField::Manpower => (
                format!(
                    "Hommes : {}{}",
                    fr::digits(top.manpower as i64),
                    arrow(forecast.map(|f| &f.men))
                ),
                style::INK,
                men_tip.clone(),
            ),
            HudField::Prestige => (
                format!(
                    "Prestige : {}{}",
                    top.prestige,
                    arrow(forecast.map(|f| &f.prestige))
                ),
                style::INK,
                prestige_tip.clone(),
            ),
            HudField::Unrest => (
                format!(
                    "Troubles : {} %{}",
                    top.mean_unrest,
                    arrow(forecast.map(|f| &f.unrest))
                ),
                style::INK,
                unrest_tip.clone(),
            ),
            HudField::Realm => (
                format!(
                    "{} · {}",
                    fr::count(top.provinces_count as i64, "province", None),
                    fr::count(top.armies_count as i64, "armée", None)
                ),
                style::INK,
                String::new(),
            ),
        };
        if content.0 != value {
            content.0 = value;
        }
        color.0 = ink;
        if tooltip.bbcode != tip {
            tooltip.bbcode = tip;
        }
    }
}

fn rebuild_alerts(
    mut commands: Commands,
    hud: Res<HudData>,
    theme: Res<UiTheme>,
    list: Query<Entity, With<AlertList>>,
    mut built: Local<u64>,
) {
    if *built == hud.revision {
        return;
    }
    let (Some(readiness), Ok(list)) = (&hud.readiness, list.single()) else {
        return;
    };
    *built = hud.revision;
    commands.entity(list).despawn_children();
    let alerts = &readiness.alerts;
    commands.entity(list).with_children(|column| {
        for alert in alerts.iter().take(MAX_ALERTS) {
            let tint = match alert.severity {
                game_api::turn::Severity::Danger => style::POOR,
                game_api::turn::Severity::Warning => style::FAIR,
                game_api::turn::Severity::Info => style::GOOD,
            };
            column
                .spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(5.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Node {
                            width: Val::Px(9.0),
                            height: Val::Px(9.0),
                            ..default()
                        },
                        BackgroundColor(tint),
                    ));
                    row.spawn((
                        button_small(&theme, &format!("hud:alert:{}", alert.id), &alert.text),
                        HudWidget,
                        AlertTarget(alert.clone()),
                    ));
                });
        }
        if alerts.len() > MAX_ALERTS {
            column.spawn(caption(
                &theme,
                &format!("… et {} autres avis", alerts.len() - MAX_ALERTS),
            ));
        }
    });
}

fn rebuild_armies(
    mut commands: Commands,
    hud: Res<HudData>,
    theme: Res<UiTheme>,
    strip: Query<Entity, With<ArmyStrip>>,
    mut built: Local<u64>,
) {
    if *built == hud.revision {
        return;
    }
    let Ok(strip) = strip.single() else { return };
    *built = hud.revision;
    commands.entity(strip).despawn_children();
    commands.entity(strip).with_children(|row| {
        for army in hud.armies.iter().take(MAX_ARMIES) {
            let status = if army.besieging {
                "siège"
            } else if army.marching {
                "en marche"
            } else if army.idle {
                "sans ordre"
            } else {
                "au repos"
            };
            row.spawn((
                button_small(
                    &theme,
                    &format!("hud:army:{}", army.id),
                    &format!("{} · {} h", army.name, fr::digits(army.men as i64)),
                ),
                HudWidget,
                ArmyTarget(army.id.clone()),
                Tooltip::new(&format!("[b]{}[/b]\n{status}", army.name)),
            ));
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn update_end_turn(
    hud: Res<HudData>,
    flow: Res<TurnFlow>,
    mut commands: Commands,
    buttons: Query<Entity, With<EndTurnButton>>,
    mut wait: Query<&mut Visibility, With<TurnWait>>,
    children: Query<&Children>,
    mut all_text: Query<&mut Text>,
    mut last_running: Local<Option<bool>>,
) {
    let running = flow.running();
    if let Ok(mut visibility) = wait.single_mut() {
        *visibility = if running {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let Ok(button) = buttons.single() else { return };
    let caption_fr = match (&hud.top, running) {
        (_, true) => "Tour en cours…".to_owned(),
        (Some(top), false) => format!("Fin de saison — {}", top.date_label),
        _ => "Fin de saison".to_owned(),
    };
    if *last_running != Some(running) {
        *last_running = Some(running);
        set_disabled(&mut commands, button, running);
    }
    if let Ok(kids) = children.get(button) {
        for kid in kids.iter() {
            if let Ok(mut text) = all_text.get_mut(kid)
                && text.0 != caption_fr
            {
                text.0 = caption_fr.clone();
            }
        }
    }
}

fn update_legend(mode: Option<Res<MapModeState>>, mut legend: Query<&mut Text, With<ModeLegend>>) {
    let current = mode.map_or(MapMode::Political, |m| m.mode);
    if let Ok(mut text) = legend.single_mut() {
        let value = format!("{} : {}", current.label_fr(), current.tooltip_fr());
        if text.0 != value {
            text.0 = value;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_clicks(
    mut pressed: MessageReader<ButtonPressed>,
    alerts: Query<&AlertTarget>,
    armies: Query<&ArmyTarget>,
    mut panels: MessageWriter<OpenPanel>,
    mut modes: MessageWriter<SetMapMode>,
    mut select_province: MessageWriter<SelectProvince>,
    mut decisions: MessageWriter<OpenDecision>,
    mut selected_army: ResMut<SelectedArmy>,
    sim: Res<Sim>,
) {
    for press in pressed.read() {
        match press.name.as_str() {
            "hud:open_diplomacy" => {
                panels.write(OpenPanel::Diplomacy);
            }
            "hud:open_tech" => {
                panels.write(OpenPanel::Tech);
            }
            "hud:open_recruitment" => {
                // The recruitment screen opens on a province: the selected army's, else the capital.
                let province = selected_army
                    .0
                    .as_ref()
                    .and_then(|id| {
                        let (data, state) = (sim.data.as_ref()?, sim.state.as_ref()?);
                        game_api::army_sheet(data, state, id)?.location_province
                    })
                    .unwrap_or_default();
                panels.write(OpenPanel::Recruitment(province));
            }
            name => {
                if let Some(key) = name.strip_prefix("hud:mode:")
                    && let Some(mode) = MapMode::from_key(key)
                {
                    modes.write(SetMapMode(mode));
                } else if let Ok(target) = alerts.get(press.entity) {
                    activate_alert(
                        &target.0,
                        &mut panels,
                        &mut select_province,
                        &mut decisions,
                        &mut selected_army,
                    );
                } else if let Ok(target) = armies.get(press.entity) {
                    if let Some(id) = ArmyId::parse(&target.0) {
                        selected_army.0 = Some(id);
                    }
                    panels.write(OpenPanel::Army(target.0.clone()));
                }
            }
        }
    }
}

/// `alert_routes.gd`: an alert leads to its army, its province or the screen that handles it.
fn activate_alert(
    alert: &Alert,
    panels: &mut MessageWriter<OpenPanel>,
    select_province: &mut MessageWriter<SelectProvince>,
    decisions: &mut MessageWriter<OpenDecision>,
    selected_army: &mut SelectedArmy,
) {
    if alert.kind == "chronicle_decision"
        && let Some(id) = alert
            .id
            .strip_prefix("chronicle:")
            .and_then(|n| n.parse().ok())
    {
        decisions.write(OpenDecision::Id(id));
        return;
    }
    if let Some(army) = &alert.army {
        if let Some(id) = ArmyId::parse(army) {
            selected_army.0 = Some(id);
        }
        panels.write(OpenPanel::Army(army.clone()));
        return;
    }
    if let Some(province) = &alert.province {
        select_province.write(SelectProvince(Some(province.clone())));
        panels.write(OpenPanel::Province(province.clone()));
        return;
    }
    match alert.kind.as_str() {
        "research_idle" => {
            panels.write(OpenPanel::Tech);
        }
        "diplomacy_offer" => {
            panels.write(OpenPanel::Diplomacy);
        }
        _ => {}
    }
}

/// `--dump-state` section `hud`.
fn dump_hud(world: &World) -> (String, Value) {
    let hud = world.resource::<HudData>();
    let flow = world.resource::<TurnFlow>();
    let queue = world.get_resource::<ModalQueue>();
    let alerts: Vec<Value> = hud
        .readiness
        .as_ref()
        .map(|r| {
            r.alerts
                .iter()
                .map(|a| {
                    json!({
                        "kind": a.kind, "text": a.text, "blocking": a.blocking,
                        "province": a.province, "army": a.army,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let top = hud.top.as_ref().map(|t| {
        json!({
            "turn": t.turn, "date": t.date_label, "season": t.season,
            "treasury": t.treasury, "net_income": t.net_income, "manpower": t.manpower,
            "prestige": t.prestige, "unrest": t.mean_unrest,
            "provinces": t.provinces_count, "armies": t.armies_count,
        })
    });
    (
        "hud".to_owned(),
        json!({
            "top": top,
            "alerts": alerts,
            "alerts_blocked": hud.readiness.as_ref().map(|r| r.blocked),
            "armies": hud.armies.iter().map(|a| json!({"id": a.id, "name": a.name, "men": a.men, "idle": a.idle})).collect::<Vec<_>>(),
            "forecast": hud.forecast.as_ref().map(|f| json!({
                "prestige_next": f.prestige.next, "men_next": f.men.next, "unrest_next": f.unrest.next,
            })),
            "refreshes": hud.refreshes,
            "forecasts_computed": hud.forecasts_computed,
            "turn_running": flow.running(),
            "turns_played": flow.turns_played,
            "last_turn_events": flow.last_events.len(),
            "last_report_counts": flow.last_report_counts,
            "confirm_shown": flow.confirm_shown,
            "turn_error": flow.error,
            "modal_active": queue.map(|q| q.active_key().to_owned()),
        }),
    )
}
