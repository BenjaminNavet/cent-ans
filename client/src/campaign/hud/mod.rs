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
use crate::campaign::provinces::{MapMode, RegionLabelObstacles, SetMapMode};
use crate::sim::Sim;
use crate::ui::kit::layout;
use crate::ui::kit::modal::ModalQueue;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::button_small;
use crate::ui::kit::{ButtonPressed, UiTheme, fr};
use bevy::ui::{ComputedNode, UiGlobalTransform};

pub mod bar;
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
pub mod keys;
pub mod minimap;
pub mod missions;
pub mod ransom;
pub mod replay;
pub mod report;
pub mod seal;
pub mod text;
pub mod wait;

use bar::{FieldImage, FieldText};
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

/// Which figure a top-bar chip shows (the chip carries it with its tooltip; the text child is
/// `bar::FieldText`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum HudField {
    Faction,
    Date,
    Treasury,
    Balance,
    Manpower,
    Prestige,
    Unrest,
}

/// Marks HUD widgets: while the pointer is on one, the map does not take the click.
#[derive(Component)]
struct HudWidget;

#[derive(Component)]
struct AlertList;

#[derive(Component)]
struct ArmyStrip;

/// Column at the top left (under the top bar) that stacks the next-step hint (« Conseil ») and
/// the missions tracker (Godot: journal and advisor bubble).
#[derive(Component)]
pub struct AdvisorColumn;

/// Bottom-right corner slot the minimap fills.
#[derive(Component)]
pub struct MinimapSlot;

#[derive(Component)]
pub struct EndTurnButton;

/// The group of alerts a pill stands for.
#[derive(Component, Clone)]
struct AlertTarget(cluster::Group);

/// Next alert to visit, per kind (a click on a pill walks its group in turn).
#[derive(Resource, Default)]
struct AlertCursor(std::collections::HashMap<String, usize>);

/// Launcher of the windows in the top bar (the objectives, traditions and ransom buttons join it).
#[derive(Component)]
pub struct PanelButtons;

/// The bottom-right cluster (alerts, seal, minimap): its size is published in `HudReserve` so
/// side panels end above it and bottom panels stop left of it.
#[derive(Component)]
struct BottomRight;

/// HUD element region names must keep clear of (published to `RegionLabelObstacles`).
#[derive(Component)]
pub struct LabelObstacle;

/// Top bar, to size the label obstacles' top inset.
#[derive(Component)]
struct TopBarMark;

#[derive(Component, Clone)]
struct ArmyTarget(String);

pub struct CampaignHudPlugin;

impl Plugin for CampaignHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudData>()
            .init_resource::<bar::BarCompact>()
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
                    bar::fit_top_bar,
                    seal::derive_seal_images,
                    seal::update_seal,
                    seal::pill_visuals,
                    bar::launcher_visuals,
                    handle_clicks,
                    publish_reserve,
                    yield_to_tutorial,
                    publish_label_obstacles,
                )
                    .chain()
                    .in_set(HudSet::Ui),
            )
            .add_systems(
                Update,
                open_menu
                    .before(crate::menu::pause::pause_escape)
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(
                Update,
                block_map_clicks
                    .before(ProvincePickSet)
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

/// The guided tutorial's card (`progress/tutorial.rs`, root `TutorialCard`) takes the top-left
/// corner: the « Conseil » column steps aside while it is shown (both say what to do next).
fn yield_to_tutorial(
    roots: Query<&Name, (With<Node>, Without<ChildOf>)>,
    mut column: Query<&mut Node, With<AdvisorColumn>>,
) {
    let tutorial = roots.iter().any(|name| name.as_str() == "TutorialCard");
    let display = if tutorial {
        Display::None
    } else {
        Display::Flex
    };
    for mut node in &mut column {
        if node.display != display {
            node.display = display;
        }
    }
}

/// « Menu » of the top bar = the pause key (`campaign_pause`): same priority rules (an open
/// panel closes first, a modal keeps it).
fn open_menu(mut pressed: MessageReader<ButtonPressed>, mut edges: ResMut<crate::menu::KeyEdges>) {
    if pressed.read().any(|p| p.name == "hud:menu") {
        edges.escape = true;
    }
}

/// Publishes the size of the bottom-right cluster (`ui::kit::layout::HudReserve`): docked
/// panels fit around it instead of the cluster dodging them.
fn publish_reserve(
    group: Query<&ComputedNode, With<BottomRight>>,
    mut reserve: ResMut<layout::HudReserve>,
) {
    let Ok(node) = group.single() else { return };
    let size = node.size() * node.inverse_scale_factor();
    // Rounded to whole pixels so a sub-pixel relayout does not move every panel each frame.
    let size = size.round();
    if reserve.bottom_right != size {
        reserve.bottom_right = size;
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

fn spawn_hud(
    mut commands: Commands,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    keymap: Option<Res<crate::settings::KeyMap>>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    let keys = bar::KeyHints::new(keymap.as_deref(), settings.as_deref());
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
            bar::spawn_top_bar(root, &theme, &assets, &keys);
            // Advisor column, top left under the bar (Godot: journal + « Conseil » bubble).
            root.spawn((
                Name::new("hud:advisor_column"),
                AdvisorColumn,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(layout::PANEL_TOP),
                    left: Val::Px(layout::EDGE),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: Val::Px(layout::GAP),
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
                    bottom: Val::Px(layout::EDGE),
                    left: Val::Px(layout::EDGE),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    max_width: Val::Percent(34.0),
                    ..default()
                },
            ));
            // Bottom right: alerts above the season seal, then the minimap in the corner.
            root.spawn((
                Name::new("hud:bottom_right"),
                BottomRight,
                LabelObstacle,
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(layout::EDGE),
                    right: Val::Px(layout::EDGE),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexEnd,
                    column_gap: Val::Px(layout::GAP),
                    ..default()
                },
            ))
            .with_children(|cluster| {
                cluster
                    .spawn((
                        Name::new("hud:seal_column"),
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexEnd,
                            row_gap: Val::Px(layout::GAP),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|column| {
                        column.spawn((
                            Name::new("hud:alerts"),
                            AlertList,
                            HudWidget,
                            Hovered::default(),
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::FlexEnd,
                                row_gap: Val::Px(seal::PILL_GAP),
                                ..default()
                            },
                        ));
                        column
                            .spawn((
                                Name::new("hud:seal_slot"),
                                Node {
                                    width: Val::Px(seal::SEAL_SIZE + 8.0),
                                    justify_content: JustifyContent::Center,
                                    margin: UiRect::right(Val::Px(4.0)),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ))
                            .with_children(|slot| {
                                seal::spawn_seal(slot, &theme, &assets, &keys);
                            });
                    });
                cluster.spawn((Name::new("hud:minimap_slot"), MinimapSlot, Node::default()));
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn update_fields(
    hud: Res<HudData>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    mut fields: Query<(&HudField, &Children, &mut Tooltip)>,
    mut texts: Query<(&mut Text, &mut TextColor), With<FieldText>>,
    mut images: Query<&mut ImageNode, With<FieldImage>>,
    compact: Res<bar::BarCompact>,
    mut shown: Local<(String, String)>,
) {
    if !hud.is_changed() && !compact.is_changed() {
        return;
    }
    let Some(top) = &hud.top else { return };
    let [prestige_tip, men_tip, unrest_tip] = text::forecast_tooltips(hud.forecast.as_ref());
    let arrow = |gauge: Option<&game_api::hud::GaugeView>| {
        gauge.map_or_else(String::new, |g| format!(" {}", text::arrow(g)))
    };
    let forecast = hud.forecast.as_ref();
    let realm = format!(
        "{} · {}",
        fr::count(top.provinces_count as i64, "province", None),
        fr::count(top.armies_count as i64, "armée", None)
    );
    // Shield of the faction and icon of the season follow the state.
    let shield = format!("heraldry/{}.png", top.faction_id);
    let season = format!("hud_season_{}", top.season);
    let new_images = (shield.clone(), season.clone());
    let reload_images = *shown != new_images;
    *shown = new_images;
    for (field, children, mut tooltip) in &mut fields {
        let (value, ink, tip) = match field {
            HudField::Faction => (
                top.faction_name.clone(),
                style::INK_TITLE,
                format!(
                    "[b]{}[/b]\n{realm}\nClic : panneau de faction.",
                    top.faction_name
                ),
            ),
            HudField::Date => (
                format!("{} {}", text::season_fr(&top.season), top.year),
                style::INK,
                format!(
                    "[b]{}[/b] · tour {}\nDifficulté : {}",
                    top.date_label, top.turn, top.difficulty
                ),
            ),
            HudField::Treasury => (
                fr::amount(top.treasury),
                if top.treasury < 0 {
                    fr::LOSS_COLOR
                } else {
                    style::INK
                },
                hud.economy.as_ref().map_or_else(
                    || "[b]Trésor[/b]".to_owned(),
                    |economy| text::treasury_tooltip(economy, &hud.top_provinces),
                ),
            ),
            HudField::Balance => (
                if compact.0 {
                    fr::signed(top.net_income)
                } else {
                    format!("{} / saison", fr::signed(top.net_income))
                },
                fr::color_of(top.net_income),
                hud.economy
                    .as_ref()
                    .map_or_else(|| "[b]Solde[/b]".to_owned(), text::budget_tooltip),
            ),
            HudField::Manpower => (
                format!(
                    "{}{}",
                    fr::digits(top.manpower as i64),
                    arrow(forecast.map(|f| &f.men))
                ),
                style::INK,
                format!("[b]Hommes[/b]\n{men_tip}"),
            ),
            HudField::Prestige => (
                format!("{}{}", top.prestige, arrow(forecast.map(|f| &f.prestige))),
                style::INK,
                format!("[b]Prestige[/b]\n{prestige_tip}"),
            ),
            HudField::Unrest => (
                format!(
                    "{} %{}",
                    top.mean_unrest,
                    arrow(forecast.map(|f| &f.unrest))
                ),
                style::INK,
                format!("[b]Troubles[/b] (moyenne des provinces)\n{unrest_tip}"),
            ),
        };
        for child in children.iter() {
            if let Ok((mut content, mut color)) = texts.get_mut(child) {
                if content.0 != value {
                    content.0 = value.clone();
                }
                color.0 = ink;
            }
            if reload_images && let Ok(mut image) = images.get_mut(child) {
                match field {
                    HudField::Faction => image.image = assets.load(shield.clone()),
                    HudField::Date => {
                        if let Some(entry) = theme.icons.get(&season) {
                            image.image = assets.load(entry.path.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        if tooltip.bbcode != tip {
            tooltip.bbcode = tip;
        }
    }
}

/// Short kinds shown at most (the rest is summed on a last « +n » line, Godot's fan has room
/// for about as many).
const MAX_ALERT_PILLS: usize = 6;

fn rebuild_alerts(
    mut commands: Commands,
    hud: Res<HudData>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
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
    let hidden: usize = groups
        .iter()
        .skip(MAX_ALERT_PILLS)
        .map(|g| g.alerts.len())
        .sum();
    commands.entity(list).with_children(|column| {
        if hidden > 0 {
            column.spawn((
                Text::new(format!("+ {} autres avis", hidden)),
                theme.text_font(false, true, UiType::Caption.size()),
                TextColor(style::PARCHMENT_LIGHT),
                TextShadow::default(),
            ));
        }
        // The most urgent pill sits right above the seal: the column lists them bottom-up.
        for group in groups.iter().take(MAX_ALERT_PILLS).rev() {
            column.spawn((
                seal::alert_pill(
                    &theme,
                    &assets,
                    &format!("hud:alert:{}", group.kind),
                    &group.kind,
                    cluster::short_label(&group.kind),
                    group.alerts.len(),
                    group.danger(),
                ),
                HudWidget,
                AlertTarget(group.clone()),
                Tooltip::new(&group.tooltip()),
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

/// Mouse click on the treasury field: the same message as a script's `click hud:treasury`.
fn treasury_click(
    click: On<bevy::picking::events::PointerClick>,
    mut pressed: MessageWriter<ButtonPressed>,
) {
    if click.button == PointerButton::Primary {
        pressed.write(ButtonPressed {
            entity: click.entity,
            name: "hud:treasury".to_owned(),
        });
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
            "hud:faction" => {
                panels.write(OpenPanel::Faction(sim.player.as_str().to_owned()));
            }
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
            "hud:treasury" => {
                panels.write(OpenPanel::Economy);
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
/// Dump section `campaign_hud_layout`: laid-out rectangles (logical px) of the HUD blocks, the
/// launcher, the panels placed by `FitZone` and their overlaps with the top bar and the
/// bottom-right cluster (must be empty), and the region names drawn under an obstacle (must be 0).
fn dump_layout(world: &World) -> (String, Value) {
    let mut named: std::collections::BTreeMap<String, [f32; 4]> = Default::default();
    let mut fitted: Vec<(String, Rect)> = Vec::new();
    let mut launchers = 0;
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
        if entity.contains::<bar::LauncherButton>() && key.starts_with("hud:open_") {
            launchers += 1;
        }
        if matches!(
            key,
            "hud:bottom_right"
                | "hud:minimap"
                | "hud:top_bar"
                | "hud:army_strip"
                | "hud:end_turn"
                | "hud:alerts"
                | "hud:launcher"
                | "hud:advisor_column"
                | "hud:hint"
                | "hud:missions"
        ) {
            named.insert(
                key.to_owned(),
                [rect.min.x, rect.min.y, rect.max.x, rect.max.y],
            );
        }
        if entity.contains::<layout::FitZone>()
            && entity
                .get::<Node>()
                .is_some_and(|n| n.display != Display::None)
        {
            fitted.push((key.to_owned(), rect));
        }
    }
    let block = |key: &str| named.get(key).map(|r| Rect::new(r[0], r[1], r[2], r[3]));
    let mut overlaps: Vec<String> = Vec::new();
    for (panel, rect) in &fitted {
        for hud in ["hud:top_bar", "hud:bottom_right"] {
            if let Some(b) = block(hud) {
                let i = b.intersect(*rect);
                if i.width() > 1.0 && i.height() > 1.0 {
                    overlaps.push(format!("{panel} × {hud}"));
                }
            }
        }
    }
    // The bar's content must fit on one line: the launcher ends inside the bar.
    let bar_fits = match (block("hud:top_bar"), block("hud:launcher")) {
        (Some(bar), Some(launcher)) => launcher.max.x <= bar.max.x + 0.5,
        _ => false,
    };
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
    let reserve = world.resource::<layout::HudReserve>();
    (
        "campaign_hud_layout".to_owned(),
        json!({
            "rects": named,
            "launchers": launchers,
            "top_bar_fits": bar_fits,
            "reserve_bottom_right": [reserve.bottom_right.x, reserve.bottom_right.y],
            "fitted_panels": fitted.iter().map(|(n, r)| json!({"name": n, "rect": [r.min.x, r.min.y, r.max.x, r.max.y]})).collect::<Vec<_>>(),
            "panel_overlaps": overlaps,
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
