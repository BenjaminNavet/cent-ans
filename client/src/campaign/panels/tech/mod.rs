//! Campaign panel: technologies (lot ptech, ADR 0357). Ports `game/scripts/ui/tech_panel.gd`
//! (tabs, research status, 3-slot queue ribbon, filters) and `tech_tree_view.gd` (columns by
//! tier, node states, foot pills, prerequisite links, path highlight, rich tooltip).
//!
//! Data: `game_api::tech_tree` (replaces `get_tech_tree`, `get_research*`). Choosing a research
//! submits `Order::Research` (or `QueueResearch` with the queue toggle / Shift) to the `Sim`,
//! exactly as `godot-bridge` does. Opens on `OpenPanel::Tech` or key F4 (toggle); Escape, the
//! close button or any other central panel (`Diplomacy`, `CloseAll`) closes it.
//!
//! Widget names (agent `click`): `tech:<technology id>`, `tech:tab_military|civil|medicine`,
//! `tech:filter_reach|unlock|known`, `tech:queue_mode`, `tech:close`.
//!
//! Not ported: the free-text search box (Bevy UI has no text field yet); the tree is not
//! re-centred on the current research (the first available node of the open branch is not
//! scrolled to).

pub mod layout;

use std::collections::{HashMap, HashSet};

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Button, ScrollArea};
use game_api::tech::{TechNode, TechTree};
use sim_campaign::Order;

use super::OpenPanel;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::paths;
use crate::sim::Sim;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::{
    TabSelected, ToggleChanged, button_small, caption, checkbox, gauge, panel_sized, tab_bar,
    tab_pane,
};
use crate::ui::kit::{ButtonPressed, UiTheme};
use layout::{DIM_ALPHA, Filter};

const BRANCHES: [(&str, &str, &str); 3] = [
    ("military", "tech:tab_military", "Militaire"),
    ("civil", "tech:tab_civil", "Civil"),
    ("medicine", "tech:tab_medicine", "Médecine"),
];
const TAB_GROUP: &str = "tech_branch";
const TREE_HEIGHT: f32 = 340.0;
const PANEL_WIDTH: f32 = 1180.0;

pub struct PanelTechPlugin;

impl Plugin for PanelTechPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TechPanel>()
            .init_resource::<EntityIcons>()
            .add_systems(Startup, scan_icons)
            .add_systems(
                Update,
                (
                    open_close,
                    watch_turn,
                    rebuild,
                    handle_buttons,
                    hover_focus,
                    refresh_visuals,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), close_on_exit)
            .register_dump(dump_tech);
    }
}

/// State of the screen, kept across rebuilds.
#[derive(Resource, Default)]
pub struct TechPanel {
    pub open: bool,
    root: Option<Entity>,
    /// Index into [`BRANCHES`].
    branch: usize,
    filter: Filter,
    queue_mode: bool,
    focus: Option<String>,
    dirty: bool,
    /// Turn the tree was built for.
    turn: u32,
    tree: Option<TechTree>,
    /// Bumped by every rebuild so [`refresh_visuals`] repaints.
    epoch: u32,
    /// Last submitted order outcome, for the dump.
    last_order: String,
}

/// Ids with a painted miniature in `icons/entity/` (technologies, units, buildings).
#[derive(Resource, Default)]
struct EntityIcons(HashSet<String>);

impl EntityIcons {
    fn path(&self, id: &str) -> Option<String> {
        self.0
            .contains(id)
            .then(|| format!("icons/entity/{id}.png"))
    }
}

fn scan_icons(mut icons: ResMut<EntityIcons>) {
    let Ok(root) = paths::find_root() else {
        return;
    };
    let dir = paths::asset_root(&root).join("icons/entity");
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(stem) = name.strip_suffix(".png") {
                icons.0.insert(stem.to_owned());
            }
        }
    }
}

// ---- components ---------------------------------------------------------------------------

#[derive(Component)]
struct TechRoot;

/// A node button; carries what the filter and the path highlight need.
#[derive(Component)]
struct TechNodeView {
    id: String,
    state: &'static str,
    matches_filter_data: NodeFilterData,
}

struct NodeFilterData {
    node: TechNode,
}

#[derive(Component)]
struct DimText {
    id: String,
    base: Color,
}

#[derive(Component)]
struct DimImage {
    id: String,
}

#[derive(Component)]
struct LinkView {
    from: String,
    to: String,
    known: bool,
}

// ---- systems ------------------------------------------------------------------------------

fn open_close(
    mut panel: ResMut<TechPanel>,
    mut messages: MessageReader<OpenPanel>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for message in messages.read() {
        match message {
            OpenPanel::Tech => {
                panel.open = true;
                panel.dirty = true;
            }
            OpenPanel::Diplomacy | OpenPanel::CloseAll => panel.open = false,
            _ => {}
        }
    }
    if keys.just_pressed(KeyCode::F4) {
        panel.open = !panel.open;
        panel.dirty = panel.open;
    }
    if panel.open && keys.just_pressed(KeyCode::Escape) {
        panel.open = false;
    }
}

fn watch_turn(mut panel: ResMut<TechPanel>, sim: Res<Sim>) {
    if panel.open
        && let Some(state) = &sim.state
        && state.turn != panel.turn
    {
        panel.dirty = true;
    }
}

fn close_on_exit(mut commands: Commands, mut panel: ResMut<TechPanel>) {
    if let Some(root) = panel.root.take() {
        commands.entity(root).despawn();
    }
    panel.open = false;
}

/// Spawns or despawns the screen to match `open` / `dirty`.
fn rebuild(
    mut commands: Commands,
    mut panel: ResMut<TechPanel>,
    sim: Res<Sim>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    icons: Res<EntityIcons>,
) {
    if !panel.open {
        if let Some(root) = panel.root.take() {
            commands.entity(root).despawn();
            panel.tree = None;
        }
        return;
    }
    if !panel.dirty && panel.root.is_some() {
        return;
    }
    panel.dirty = false;
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return;
    };
    let Some(tree) = game_api::tech_tree(data, state, &sim.player) else {
        return;
    };
    if let Some(root) = panel.root.take() {
        commands.entity(root).despawn();
    }
    let faction = data
        .factions
        .get(&sim.player)
        .map_or(sim.player.as_str(), |f| f.name.display.as_str())
        .to_owned();
    panel.turn = state.turn;
    panel.epoch += 1;
    let root = spawn_screen(
        &mut commands,
        &theme,
        &assets,
        &icons,
        &tree,
        &faction,
        state.turn,
        &panel,
    );
    panel.root = Some(root);
    panel.tree = Some(tree);
}

#[allow(clippy::too_many_arguments)]
fn spawn_screen(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    icons: &EntityIcons,
    tree: &TechTree,
    faction: &str,
    turn: u32,
    panel: &TechPanel,
) -> Entity {
    let (cards, end_line) = layout::ribbon(
        tree.research.as_ref(),
        &tree.queue,
        tree.points_per_turn,
        turn,
    );
    let theme = theme.clone();
    commands
        .spawn((
            Name::new("TechPanel"),
            TechRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            UiTier::Panel.index(),
        ))
        .with_children(|root| {
            root.spawn(panel_sized(&theme, PANEL_WIDTH))
                .with_children(|page| {
                    page.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|bar| {
                        bar.spawn((
                            Name::new("TechTitle"),
                            Text::new(format!("Technologies — {faction}")),
                            theme.title_font(UiType::Title.size()),
                            TextColor(style::INK_TITLE),
                        ));
                        bar.spawn(button_small(&theme, "tech:close", "Fermer"));
                    });
                    // Research status: current research with its gauge, or the idle line.
                    page.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|status| {
                        match &tree.research {
                            None => {
                                status.spawn((
                                    Name::new("TechIdle"),
                                    Text::new("Aucun savoir à l'étude"),
                                    theme.text_font(false, false, UiType::Caption.size()),
                                    TextColor(style::RUBRIC),
                                ));
                                let ratio = if tree.reserve_cap > 0 {
                                    tree.reserve_points as f32 / tree.reserve_cap as f32
                                } else {
                                    0.0
                                };
                                status.spawn(caption(
                                    &theme,
                                    &format!(
                                        "{} points par tour s’accumulent en réserve ({} / {}) : choisissez un savoir.",
                                        tree.points_per_turn, tree.reserve_points, tree.reserve_cap
                                    ),
                                ));
                                status.spawn(gauge(&theme, "tech:reserve_gauge", ratio, 420.0));
                            }
                            Some(research) => {
                                status.spawn(caption(
                                    &theme,
                                    &format!(
                                        "À l'étude : {} — {} / {} pts, {} par tour",
                                        research.name,
                                        research.progress,
                                        research.cost,
                                        research.points_per_turn
                                    ),
                                ));
                            }
                        }
                    });
                    // Queue ribbon: 3 cards, then the end-of-queue line.
                    if tree.research.is_some() || !tree.queue.is_empty() {
                        page.spawn(Node {
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|ribbon| {
                            for (index, card) in cards.iter().enumerate() {
                                ribbon
                                    .spawn((
                                        Name::new(format!("tech:ribbon_{index}")),
                                        Node {
                                            flex_grow: 1.0,
                                            flex_basis: Val::Px(0.0),
                                            flex_direction: FlexDirection::Column,
                                            row_gap: Val::Px(2.0),
                                            padding: UiRect::all(Val::Px(5.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(style::PARCHMENT_LIGHT),
                                        BorderColor::all(style::INK_SOFT),
                                    ))
                                    .with_children(|slot| {
                                        slot.spawn(caption(&theme, &card.caption));
                                        slot.spawn(gauge(
                                            &theme,
                                            &format!("tech:ribbon_gauge_{index}"),
                                            card.ratio,
                                            260.0,
                                        ));
                                        slot.spawn(caption(&theme, &card.turns));
                                    });
                            }
                        });
                        if let Some(end) = &end_line {
                            page.spawn(caption(&theme, end));
                        }
                    }
                    // Filters and queue mode.
                    page.spawn(Node {
                        column_gap: Val::Px(14.0),
                        flex_wrap: FlexWrap::Wrap,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(checkbox(&theme, "tech:filter_reach", "À portée", panel.filter.reach));
                        row.spawn(checkbox(
                            &theme,
                            "tech:filter_unlock",
                            "Débloque une unité ou un bâtiment",
                            panel.filter.unlock,
                        ));
                        row.spawn(checkbox(
                            &theme,
                            "tech:filter_known",
                            "Estomper les acquis",
                            panel.filter.fade_known,
                        ));
                        row.spawn(checkbox(
                            &theme,
                            "tech:queue_mode",
                            "Mettre en file (ou Maj+clic)",
                            panel.queue_mode,
                        ));
                    });
                    let tabs: Vec<(&str, &str)> =
                        BRANCHES.iter().map(|(_, name, label)| (*name, *label)).collect();
                    page.spawn(tab_bar(&theme, TAB_GROUP, &tabs, panel.branch));
                    for (index, (branch, tab_name, _)) in BRANCHES.iter().enumerate() {
                        let nodes: Vec<&TechNode> =
                            tree.techs.iter().filter(|t| t.branch == *branch).collect();
                        page.spawn(tab_pane(TAB_GROUP, tab_name, index == panel.branch))
                            .with_children(|pane| {
                                spawn_branch(pane, &theme, assets, icons, &nodes, branch);
                            });
                    }
                });
        })
        .id()
}

fn fill_of(shown: &str) -> Color {
    let lerp = |a: Color, b: Color, t: f32| -> Color {
        let (a, b) = (a.to_srgba(), b.to_srgba());
        Color::srgb(
            a.red + (b.red - a.red) * t,
            a.green + (b.green - a.green) * t,
            a.blue + (b.blue - a.blue) * t,
        )
    };
    match shown {
        "known" => lerp(style::PARCHMENT_LIGHT, style::WAX_GREEN, 0.38),
        "researching" => style::GOLD_PALE,
        "queued" => lerp(style::PARCHMENT_LIGHT, style::GOLD_PALE, 0.55),
        "available" => style::PARCHMENT_LIGHT,
        _ => lerp(style::PARCHMENT_DARK, style::INK_FADED, 0.25),
    }
}

fn spawn_branch(
    pane: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    icons: &EntityIcons,
    nodes: &[&TechNode],
    branch: &str,
) {
    let canvas = layout::layout(nodes);
    let links = layout::links(nodes, &canvas);
    let known: HashMap<&str, bool> = nodes
        .iter()
        .map(|n| (n.id.as_str(), n.state == "known"))
        .collect();
    pane.spawn((
        Name::new(format!("tech:scroll_{branch}")),
        ScrollArea,
        ScrollPosition::default(),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(TREE_HEIGHT),
            overflow: Overflow::scroll(),
            ..default()
        },
        BackgroundColor(style::PARCHMENT.with_alpha(0.5)),
    ))
    .with_children(|scroll| {
        scroll
            .spawn(Node {
                width: Val::Px(canvas.width),
                height: Val::Px(canvas.height),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|surface| {
                for header in &canvas.headers {
                    surface.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(header.x),
                            top: Val::Px(6.0),
                            width: Val::Px(layout::NODE_W),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        children![(
                            Text::new(format!("Rang {}", header.tier)),
                            theme.text_font(false, false, UiType::Caption.size()),
                            TextColor(style::INK_SOFT),
                        )],
                    ));
                }
                // Links first: the nodes are drawn over them.
                for link in &links {
                    let is_known = known.get(link.from.as_str()).copied().unwrap_or(false);
                    for seg in &link.segments {
                        surface.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(seg.x),
                                top: Val::Px(seg.y),
                                width: Val::Px(seg.w),
                                height: Val::Px(seg.h),
                                ..default()
                            },
                            BackgroundColor(link_color(is_known, false, false)),
                            LinkView {
                                from: link.from.clone(),
                                to: link.to.clone(),
                                known: is_known,
                            },
                        ));
                    }
                }
                let by_id: HashMap<&str, &&TechNode> =
                    nodes.iter().map(|n| (n.id.as_str(), n)).collect();
                for placed in &canvas.placed {
                    spawn_node(
                        surface,
                        theme,
                        assets,
                        icons,
                        by_id[placed.id.as_str()],
                        placed.x,
                        placed.y,
                    );
                }
            });
    });
}

fn link_color(known: bool, lit: bool, dimmed: bool) -> Color {
    let mut color = if lit {
        style::GOLD
    } else if known {
        style::WAX_GREEN.with_alpha(0.95)
    } else {
        style::INK_SOFT.with_alpha(0.55)
    };
    if dimmed && !lit {
        let a = color.alpha();
        color = color.with_alpha(a * 0.35);
    }
    color
}

fn spawn_node(
    surface: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    icons: &EntityIcons,
    node: &TechNode,
    x: f32,
    y: f32,
) {
    let shown = layout::shown_state(node);
    let glyph = layout::glyph(shown, node.queue_position);
    let title = if glyph.is_empty() {
        node.name.clone()
    } else {
        format!("{glyph} {}", node.name)
    };
    let locked = node.state == "locked";
    let ink = if locked { style::INK_FADED } else { style::INK };
    let border = match node.state.as_str() {
        "researching" => style::GOLD,
        "locked" => style::INK_FADED,
        _ => style::INK_SOFT,
    };
    let mut entity = surface.spawn((
        Name::new(format!("tech:{}", node.id)),
        Button,
        Hovered::default(),
        TechNodeView {
            id: node.id.clone(),
            state: shown,
            matches_filter_data: NodeFilterData { node: node.clone() },
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(layout::NODE_W),
            height: Val::Px(layout::NODE_H),
            padding: UiRect::all(Val::Px(4.0)),
            border: UiRect::all(Val::Px(if node.state == "researching" {
                3.0
            } else {
                2.0
            })),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(1.0),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(fill_of(shown)),
        BorderColor::all(border),
        crate::ui::kit::tooltip::Tooltip::new(&layout::tooltip_bbcode(node)),
    ));
    if node.state != "available" {
        entity.insert(InteractionDisabled);
    }
    entity.with_children(|button| {
        // Head: icon + name / cost line.
        button
            .spawn(Node {
                column_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|head| {
                if let Some(path) = icons.path(&node.id) {
                    head.spawn((
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        ImageNode::new(assets.load(path)),
                        DimImage {
                            id: node.id.clone(),
                        },
                    ));
                }
                head.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 1.0,
                    min_width: Val::Px(0.0),
                    ..default()
                })
                .with_children(|text| {
                    text.spawn(dim_text(
                        theme,
                        &node.id,
                        &layout::short(&title, 24),
                        ink,
                        true,
                    ));
                    text.spawn(dim_text(
                        theme,
                        &node.id,
                        &layout::second_line(node),
                        ink,
                        false,
                    ));
                });
            });
        // Foot: up to 2 unlock pills, then the key effect.
        button
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(6.0),
                bottom: Val::Px(2.0),
                width: Val::Px(layout::NODE_W - 16.0),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|foot| {
                foot.spawn(Node {
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    height: Val::Px(18.0),
                    overflow: Overflow::clip(),
                    ..default()
                })
                .with_children(|pills| {
                    let all: Vec<(&str, &str, bool)> = node
                        .unit_unlocks
                        .iter()
                        .map(|u| (u.id.as_str(), u.name.as_str(), true))
                        .chain(
                            node.building_unlocks
                                .iter()
                                .map(|b| (b.id.as_str(), b.name.as_str(), false)),
                        )
                        .collect();
                    for (id, name, is_unit) in all.iter().take(2) {
                        let mut label = layout::short(name, 13);
                        if let Some(path) = icons.path(id) {
                            pills.spawn((
                                Node {
                                    width: Val::Px(16.0),
                                    height: Val::Px(16.0),
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                                ImageNode::new(assets.load(path)),
                                DimImage {
                                    id: node.id.clone(),
                                },
                            ));
                        } else {
                            label = format!("{} {label}", if *is_unit { "unité" } else { "bât." });
                        }
                        pills.spawn(dim_text(theme, &node.id, &label, style::INK_SOFT, false));
                    }
                    if all.len() > 2 {
                        pills.spawn(dim_text(
                            theme,
                            &node.id,
                            &format!("+{}", all.len() - 2),
                            style::INK_SOFT,
                            false,
                        ));
                    }
                });
                let effect = node
                    .effects
                    .first()
                    .map(layout::effect_label)
                    .unwrap_or_default();
                let effect_ink = if locked {
                    style::INK_FADED
                } else {
                    style::WAX_GREEN
                };
                foot.spawn(dim_text(
                    theme,
                    &node.id,
                    &layout::short(&effect, 28),
                    effect_ink,
                    false,
                ));
            });
    });
}

fn dim_text(theme: &UiTheme, id: &str, content: &str, color: Color, bold: bool) -> impl Bundle {
    (
        Text::new(content),
        theme.text_font(bold, false, UiType::Caption.size() - 1.0),
        TextColor(color),
        DimText {
            id: id.to_owned(),
            base: color,
        },
    )
}

/// Reacts to node clicks, tabs, filters, the queue toggle and the close button.
#[allow(clippy::too_many_arguments)]
fn handle_buttons(
    mut panel: ResMut<TechPanel>,
    mut pressed: MessageReader<ButtonPressed>,
    mut toggled: MessageReader<ToggleChanged>,
    mut tabs: MessageReader<TabSelected>,
    mut sim: ResMut<Sim>,
    keys: Res<ButtonInput<KeyCode>>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for tab in tabs.read() {
        if tab.group == TAB_GROUP
            && let Some(index) = BRANCHES.iter().position(|(_, name, _)| *name == tab.name)
        {
            panel.branch = index;
        }
    }
    for change in toggled.read() {
        match change.name.as_str() {
            "tech:filter_reach" => panel.filter.reach = change.on,
            "tech:filter_unlock" => panel.filter.unlock = change.on,
            "tech:filter_known" => panel.filter.fade_known = change.on,
            "tech:queue_mode" => panel.queue_mode = change.on,
            _ => {}
        }
    }
    if !panel.open {
        pressed.clear();
        return;
    }
    for press in pressed.read() {
        let Some(key) = press.name.strip_prefix("tech:") else {
            continue;
        };
        if key == "close" {
            panel.open = false;
            continue;
        }
        let Some(node) = panel
            .tree
            .as_ref()
            .and_then(|t| t.techs.iter().find(|n| n.id == key))
            .cloned()
        else {
            continue;
        };
        if node.state != "available" {
            continue;
        }
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let queue = panel.queue_mode || shift;
        let Ok(technology) = data_model::TechnologyId::new(node.id.clone()) else {
            continue;
        };
        let order = if queue {
            Order::QueueResearch { technology }
        } else {
            Order::Research { technology }
        };
        let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
            continue;
        };
        match state.submit_order(&data, order) {
            Ok(()) => {
                panel.last_order =
                    format!("{} {}", if queue { "queue" } else { "research" }, node.id);
                panel.dirty = true;
            }
            Err(error) => {
                panel.last_order = format!("refused {}: {error}", node.id);
                toasts.write(ToastRequest::new(&error.to_string()));
            }
        }
    }
}

fn hover_focus(mut panel: ResMut<TechPanel>, nodes: Query<(&Hovered, &TechNodeView)>) {
    let focus = nodes
        .iter()
        .find(|(hovered, _)| hovered.get())
        .map(|(_, view)| view.id.clone());
    if focus != panel.focus {
        panel.focus = focus;
    }
}

/// Dims filtered-out nodes and nodes off the hovered path; recolours the links on the path.
#[allow(clippy::type_complexity)]
fn refresh_visuals(
    panel: Res<TechPanel>,
    mut last: Local<(Option<String>, Filter, u32)>,
    mut nodes: Query<(&TechNodeView, &mut BackgroundColor)>,
    mut texts: Query<(&DimText, &mut TextColor)>,
    mut images: Query<(&DimImage, &mut ImageNode)>,
    mut links: Query<(&LinkView, &mut BackgroundColor), Without<TechNodeView>>,
) {
    if panel.root.is_none()
        || (last.0 == panel.focus && last.1 == panel.filter && last.2 == panel.epoch)
    {
        return;
    }
    *last = (panel.focus.clone(), panel.filter, panel.epoch);
    let Some(tree) = &panel.tree else {
        return;
    };
    let path: Option<HashSet<String>> = panel.focus.as_ref().map(|id| {
        let branch = tree
            .techs
            .iter()
            .find(|t| &t.id == id)
            .map(|t| t.branch.clone());
        let nodes: Vec<&TechNode> = tree
            .techs
            .iter()
            .filter(|t| Some(&t.branch) == branch.as_ref())
            .collect();
        layout::path_of(&nodes, id)
    });
    let dimmed = |id: &str, node: &TechNode| -> bool {
        (panel.filter.active() && !panel.filter.matches(node))
            || path.as_ref().is_some_and(|p| !p.contains(id))
    };
    let mut dimmed_ids: HashSet<String> = HashSet::new();
    for (view, mut fill) in &mut nodes {
        let is_dim = dimmed(&view.id, &view.matches_filter_data.node);
        let alpha = if is_dim { DIM_ALPHA } else { 1.0 };
        fill.0 = fill_of(view.state).with_alpha(alpha);
        if is_dim {
            dimmed_ids.insert(view.id.clone());
        }
    }
    for (text, mut color) in &mut texts {
        let alpha = if dimmed_ids.contains(&text.id) {
            DIM_ALPHA
        } else {
            1.0
        };
        color.0 = text.base.with_alpha(alpha);
    }
    for (image, mut node) in &mut images {
        let alpha = if dimmed_ids.contains(&image.id) {
            DIM_ALPHA
        } else {
            1.0
        };
        node.color = Color::WHITE.with_alpha(alpha);
    }
    for (link, mut fill) in &mut links {
        let lit = path
            .as_ref()
            .is_some_and(|p| p.contains(&link.from) && p.contains(&link.to));
        fill.0 = link_color(link.known, lit, path.is_some());
    }
}

/// `--dump-state` section `tech`.
fn dump_tech(world: &World) -> (String, serde_json::Value) {
    use serde_json::json;
    let panel = world.resource::<TechPanel>();
    let mut nodes = 0;
    let mut by_state: HashMap<&str, usize> = HashMap::new();
    let mut dimmed = 0;
    let mut links = 0;
    let mut buttons: Vec<String> = Vec::new();
    if let Some(mut q) = world.try_query::<(&TechNodeView, &BackgroundColor)>() {
        for (view, fill) in q.iter(world) {
            nodes += 1;
            *by_state.entry(view.state).or_default() += 1;
            if fill.0.alpha() < 0.99 {
                dimmed += 1;
            }
            if buttons.len() < 3 && view.state == "available" {
                buttons.push(view.id.clone());
            }
        }
    }
    if let Some(mut q) = world.try_query::<&LinkView>() {
        links = q.iter(world).count();
    }
    let research = panel.tree.as_ref().and_then(|t| t.research.as_ref());
    let sim = world.resource::<Sim>();
    let sim_research = match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => state.research_info(data, &sim.player).map(|info| {
            json!({
                "technology": info.technology.as_str(),
                "progress": info.progress,
                "cost": info.cost,
                "turns_left": (info.turns_left != u32::MAX).then_some(info.turns_left),
            })
        }),
        _ => None,
    };
    let queue: Vec<&str> = panel
        .tree
        .as_ref()
        .map(|t| t.queue.iter().map(|q| q.id.as_str()).collect())
        .unwrap_or_default();
    (
        "tech".to_owned(),
        json!({
            "open": panel.open,
            "branch": BRANCHES[panel.branch].0,
            "node_widgets": nodes,
            "node_states": by_state,
            "dimmed": dimmed,
            "links": links,
            "queue_mode": panel.queue_mode,
            "filter": { "reach": panel.filter.reach, "unlock": panel.filter.unlock, "fade_known": panel.filter.fade_known },
            "focus": panel.focus,
            "points_per_turn": panel.tree.as_ref().map(|t| t.points_per_turn),
            "panel_research": research.map(|r| r.technology.as_str()),
            "queue": queue,
            "sim_research": sim_research,
            "available_sample": buttons,
            "last_order": panel.last_order,
        }),
    )
}
