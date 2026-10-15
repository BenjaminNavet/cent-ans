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
use crate::campaign::provinces::{MapMode, MapModeState, RegionLabelObstacles, SetMapMode};
use crate::sim::Sim;
use crate::ui::kit::modal::ModalQueue;
use crate::ui::kit::panel_stack::{PanelTier, StackedPanel};
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_primary, button_small, panel_inset, set_disabled};
use crate::ui::kit::{ButtonPressed, UiTheme, fr};
use bevy::ui::{ComputedNode, UiGlobalTransform};

pub mod briefing;
pub mod buttons;
pub mod capture;
pub mod cluster;
pub mod decision;
pub mod flow;
pub mod help;
pub mod hint;
pub mod hover;
pub mod integ;
pub mod minimap;
pub mod missions;
pub mod ransom;
pub mod replay;
pub mod report;
pub mod text;
pub mod wait;

use decision::OpenDecision;
use flow::TurnFlow;

/// Minimum seconds between two refreshes of the views while orders keep changing the state.
const REFRESH_SECONDS: f32 = 0.25;
/// Minimum seconds between two forecasts (a forecast simulates the next turn).
const FORECAST_SECONDS: f32 = 1.0;
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

/// Column at the top right (under the top bar) that stacks the minimap, the missions and the
/// next-step hint.
#[derive(Component)]
pub struct RightColumn;

#[derive(Component)]
struct ModeLegend;

#[derive(Component)]
struct EndTurnButton;

/// The group of alerts a pill stands for.
#[derive(Component, Clone)]
struct AlertTarget(cluster::Group);

/// Next alert to visit, per kind (a click on a pill walks its group in turn).
#[derive(Resource, Default)]
struct AlertCursor(std::collections::HashMap<String, usize>);

/// Row of the panel buttons (the ransom button joins it when there are captives).
#[derive(Component)]
pub struct PanelButtons;

/// Number of columns of the bottom-right panel buttons grid (10 buttons: two full rows, never an
/// orphan « Codex » on a second line).
pub const PANEL_BUTTON_COLUMNS: u16 = 5;

/// Gap kept between the bottom-right group and a side panel / the window edge.
const BOTTOM_RIGHT_MARGIN: f32 = 10.0;

/// The bottom-right group (alerts, panel buttons, end of turn): moves left of an open side panel.
#[derive(Component)]
struct BottomRight;

/// HUD element region names must keep clear of (published to `RegionLabelObstacles`).
#[derive(Component)]
pub struct LabelObstacle;

/// Top bar, to size the label obstacles' top inset.
#[derive(Component)]
struct TopBarMark;

/// `right` offset of the bottom-right group given the width of the widest side panel docked on
/// the right (`panel_right` = its own right offset). Pure, tested.
pub fn bottom_right_offset(side_panel_width: Option<(f32, f32)>) -> f32 {
    side_panel_width.map_or(BOTTOM_RIGHT_MARGIN, |(width, right)| {
        right + width + BOTTOM_RIGHT_MARGIN
    })
}

#[derive(Component, Clone)]
struct ArmyTarget(String);

pub struct CampaignHudPlugin;

impl Plugin for CampaignHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudData>()
            .init_resource::<AlertCursor>()
            .configure_sets(
                Update,
                (HudSet::Data, HudSet::Flow, HudSet::Ui)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_plugins((
                flow::plugin,
                decision::plugin,
                capture::plugin,
                replay::plugin,
                minimap::plugin,
                missions::plugin,
                ransom::plugin,
                hint::plugin,
                hover::plugin,
                help::plugin,
                briefing::plugin,
                integ::plugin,
                buttons::plugin,
                wait::plugin,
            ))
            .register_dump(dump_hud)
            .register_dump(dump_layout)
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
                    avoid_side_panels,
                    publish_label_obstacles,
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

/// Shifts the bottom-right group left of an open docked side panel (province, recruitment…),
/// which would otherwise cover the panel buttons.
fn avoid_side_panels(
    panels: Query<(&StackedPanel, &Node, &ComputedNode)>,
    mut group: Query<&mut Node, (With<BottomRight>, Without<StackedPanel>)>,
) {
    let side = panels
        .iter()
        .filter(|(panel, node, _)| {
            panel.tier == PanelTier::Docked
                && node.display != Display::None
                && node.position_type == PositionType::Absolute
                && node.top != Val::Auto
                && matches!(node.right, Val::Px(r) if r > 0.0)
                && node.left == Val::Auto
        })
        .filter_map(|(_, node, computed)| match node.right {
            Val::Px(right) => Some((computed.size().x * computed.inverse_scale_factor(), right)),
            _ => None,
        })
        .fold(None::<(f32, f32)>, |best, item| match best {
            Some(b) if b.0 + b.1 >= item.0 + item.1 => Some(b),
            _ => Some(item),
        });
    let wanted = Val::Px(bottom_right_offset(side));
    for mut node in &mut group {
        if node.right != wanted {
            node.right = wanted;
        }
    }
}

/// Publishes the HUD rectangles (logical px) that region names avoid: the top bar height and the
/// elements marked [`LabelObstacle`] (minimap frame with its map-mode buttons, bottom groups).
fn publish_label_obstacles(
    obstacles: Option<ResMut<RegionLabelObstacles>>,
    marked: Query<(&ComputedNode, &UiGlobalTransform, &InheritedVisibility), With<LabelObstacle>>,
    top: Query<&ComputedNode, With<TopBarMark>>,
) {
    let Some(mut obstacles) = obstacles else {
        return;
    };
    let rects: Vec<Rect> = marked
        .iter()
        .filter(|(node, _, vis)| vis.get() && node.size().x > 1.0)
        .map(|(node, transform, _)| ui_rect(node, transform))
        .collect();
    let top_inset = top
        .single()
        .map_or(0.0, |node| node.size().y * node.inverse_scale_factor());
    if obstacles.rects != rects {
        obstacles.rects = rects;
    }
    if (obstacles.top_inset - top_inset).abs() > 0.5 {
        obstacles.top_inset = top_inset;
    }
}

/// Logical-pixel rectangle of a laid-out UI node.
fn ui_rect(node: &ComputedNode, transform: &UiGlobalTransform) -> Rect {
    let center = transform.translation * node.inverse_scale_factor();
    let half = node.size() * node.inverse_scale_factor() * 0.5;
    Rect::from_corners(center - half, center + half)
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
                TopBarMark,
                HudWidget,
                Hovered::default(),
                panel_inset(&theme),
            ))
            .insert(Node {
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
            })
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
            });
            // Right column: minimap, missions, hint (each module fills it).
            root.spawn((
                Name::new("hud:right_column"),
                RightColumn,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(46.0),
                    right: Val::Px(10.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            // Army strip, bottom left.
            root.spawn((
                Name::new("hud:army_strip"),
                ArmyStrip,
                LabelObstacle,
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
                    max_width: Val::Percent(30.0),
                    ..default()
                },
            ));
            // Panels and end-turn button, bottom right.
            root.spawn((
                Name::new("hud:bottom_right"),
                BottomRight,
                LabelObstacle,
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(10.0),
                    right: Val::Px(BOTTOM_RIGHT_MARGIN),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ))
            .with_children(|column| {
                // Alert pills, above the bell (end_turn_cluster.gd).
                column.spawn((
                    Name::new("hud:alerts"),
                    AlertList,
                    HudWidget,
                    Hovered::default(),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::FlexEnd,
                        row_gap: Val::Px(3.0),
                        ..default()
                    },
                ));
                column
                    .spawn((
                        PanelButtons,
                        Node {
                            display: Display::Grid,
                            grid_template_columns: RepeatedGridTrack::auto(PANEL_BUTTON_COLUMNS),
                            column_gap: Val::Px(4.0),
                            row_gap: Val::Px(4.0),
                            justify_content: JustifyContent::End,
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        for (name, caption_fr) in [
                            ("hud:open_diplomacy", "Diplomatie"),
                            ("hud:open_tech", "Techniques"),
                            ("hud:open_recruitment", "Recrutement"),
                            ("hud:open_faction", "Ma faction"),
                            ("hud:open_council", "Conseil"),
                            ("hud:open_feudal", "Féodalité"),
                            ("hud:open_chronicle", "Chronique"),
                            ("hud:open_codex", "Codex"),
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
    let groups = cluster::group_alerts(&readiness.alerts);
    commands.entity(list).with_children(|column| {
        // The most urgent pill sits right above the bell: the column lists them bottom-up.
        for group in groups.iter().rev() {
            let tint = if group.danger() {
                style::POOR
            } else {
                style::FAIR
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
                        button_small(
                            &theme,
                            &format!("hud:alert:{}", group.kind),
                            &format!(
                                "{} ({})",
                                cluster::short_label(&group.kind),
                                group.alerts.len()
                            ),
                        ),
                        HudWidget,
                        AlertTarget(group.clone()),
                        Tooltip::new(&group.tooltip()),
                    ));
                });
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
    children: Query<&Children>,
    mut all_text: Query<&mut Text>,
    mut last_running: Local<Option<bool>>,
) {
    let running = flow.running();
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
    mut codex: MessageWriter<crate::codex::OpenCodex>,
    mut modes: MessageWriter<SetMapMode>,
    mut select_province: MessageWriter<SelectProvince>,
    mut decisions: MessageWriter<OpenDecision>,
    mut selected_army: ResMut<SelectedArmy>,
    mut cursor: ResMut<AlertCursor>,
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
            "hud:open_faction" => {
                panels.write(OpenPanel::Faction(sim.player.as_str().to_owned()));
            }
            "hud:open_council" => {
                panels.write(OpenPanel::Council);
            }
            "hud:open_feudal" => {
                panels.write(OpenPanel::Feudal);
            }
            "hud:open_chronicle" => {
                panels.write(OpenPanel::Chronicle);
            }
            "hud:open_codex" => {
                codex.write(crate::codex::OpenCodex::default());
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
                    let group = &target.0;
                    let slot = cursor.0.entry(group.kind.clone()).or_insert(0);
                    let alert = &group.alerts[*slot % group.alerts.len()];
                    *slot += 1;
                    activate_alert(
                        alert,
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
pub(super) fn activate_alert(
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
/// Dump section `campaign_hud_layout`: laid-out rectangles (logical px) of the bottom HUD, the rows of the
/// panel buttons, overlaps with the docked side panels and the minimap, and the region names
/// drawn under an obstacle (must be 0).
fn dump_layout(world: &World) -> (String, Value) {
    let mut named: std::collections::BTreeMap<String, [f32; 4]> = Default::default();
    let mut side_panels: Vec<(String, Rect)> = Vec::new();
    let mut button_tops: Vec<f32> = Vec::new();
    for entity in world.iter_entities() {
        let (Some(name), Some(node), Some(transform)) = (
            entity.get::<Name>(),
            entity.get::<ComputedNode>(),
            entity.get::<UiGlobalTransform>(),
        ) else {
            continue;
        };
        if node.size().x < 1.0 {
            continue;
        }
        let rect = ui_rect(node, transform);
        let key = name.as_str();
        if matches!(
            key,
            "hud:bottom_right" | "hud:minimap" | "hud:top_bar" | "hud:army_strip" | "hud:end_turn"
        ) || key.starts_with("hud:open_")
        {
            named.insert(
                key.to_owned(),
                [rect.min.x, rect.min.y, rect.max.x, rect.max.y],
            );
            if key.starts_with("hud:open_") {
                button_tops.push((rect.min.y * 2.0).round() / 2.0);
            }
        }
        if let Some(panel) = entity.get::<StackedPanel>()
            && panel.tier == PanelTier::Docked
        {
            side_panels.push((panel.id.clone(), rect));
        }
    }
    button_tops.sort_by(f32::total_cmp);
    button_tops.dedup_by(|a, b| (*a - *b).abs() < 2.0);
    let group = named
        .get("hud:bottom_right")
        .map(|r| Rect::new(r[0], r[1], r[2], r[3]));
    let covered_by: Vec<String> = group
        .map(|g| {
            side_panels
                .iter()
                .filter(|(_, r)| !g.intersect(*r).is_empty())
                .map(|(id, _)| id.clone())
                .collect()
        })
        .unwrap_or_default();
    let stats = world.get_resource::<crate::campaign::provinces::labels::RegionLabelStats>();
    let obstacles = world.get_resource::<RegionLabelObstacles>();
    let names_under_obstacle = match (stats, obstacles) {
        (Some(stats), Some(obstacles)) => stats
            .rects
            .iter()
            .filter(|r| {
                obstacles.rects.iter().any(|o| !o.intersect(**r).is_empty())
                    || r.min.y < obstacles.top_inset
            })
            .count(),
        _ => 0,
    };
    (
        "campaign_hud_layout".to_owned(),
        json!({
            "rects": named,
            "panel_button_rows": button_tops.len(),
            "bottom_right_covered_by_panels": covered_by,
            "label_obstacles": obstacles.map_or(0, |o| o.rects.len()),
            "label_top_inset": obstacles.map_or(0.0, |o| o.top_inset),
            "region_names_on_screen": stats.map_or(0, |s| s.rects.len()),
            "region_names_under_obstacle": names_under_obstacle,
        }),
    )
}

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
            "orders_dropped": flow.orders_dropped,
            "last_dropped": flow.last_dropped,
            "modal_active": queue.map(|q| q.active_key().to_owned()),
        }),
    )
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn bottom_right_group_clears_a_side_panel() {
        assert_eq!(bottom_right_offset(None), BOTTOM_RIGHT_MARGIN);
        // Panel 400 px wide, itself 8 px from the edge: the group starts 418 px from the edge.
        assert_eq!(bottom_right_offset(Some((400.0, 8.0))), 418.0);
    }

    #[test]
    fn panel_buttons_fill_whole_grid_rows() {
        // 8 fixed + Objectifs + Traditions + Rançon: never a lone button on the last row of 5.
        assert_eq!(PANEL_BUTTON_COLUMNS, 5);
    }
}
