//! Order icons of the bottom band (Godot `battle_hud.gd` `COMMANDS`, `_build_mode_buttons`,
//! `command_button_state`, `mode_button_state`) and « Retraite générale » with its confirmation
//! (`ask_withdraw_all`).
//!
//! Two rows of five square buttons: formation (the button of [`super::formation_menu`], placed in
//! the first cell through [`HudSlot::Formation`]), fire at will, halt, pursue, withdraw; then the
//! five unit modes (run, guard, skirmish, melee, breach). Under them the wax « Retraite générale »
//! button asks before sending `Command::Withdraw` for every regiment still on the field.
//!
//! No rule here: a click sends the same commands as the orders lot's hotkeys (`orders::input`
//! `unit_order`) through [`IssueBattleCommand`]; the core accepts or refuses them. The gold /
//! greyed look reads the simulation (who can shoot, which modes a regiment may take).

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};
use sim_battle::{Command, UnitMode};

use super::geometry::HudSlot;
use super::{BuildCtx, HudData};
use crate::app::AppState;
use crate::battle::orders::BattleSelection;
use crate::battle::orders::input::{BattleViews, IssueBattleCommand};
use crate::battle::orders::select::pursuit_groups;
use crate::battle::scene::BattleSession;
use crate::settings::{KeyMap, Settings};
use crate::ui::kit::icons::icon;
use crate::ui::kit::modal::{ModalQueue, ModalResult, ModalSpec, PRIORITY_DECISION};
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::{ButtonPressed, UiTheme};

/// Size of one order button.
pub const CELL: Vec2 = Vec2::new(44.0, 36.0);
const GAP: f32 = 4.0;
const COLUMNS: usize = 5;
/// Width of the grid (five cells and their gaps).
pub const GRID_WIDTH: f32 = COLUMNS as f32 * CELL.x + (COLUMNS as f32 - 1.0) * GAP;
const WITHDRAW_ALL_H: f32 = 26.0;
const WITHDRAW_ALL: &str = "WithdrawAll";
const WITHDRAW_ALL_KEY: &str = "battle_withdraw_all";
const INK: Color = Color::srgb(0.22, 0.14, 0.07);
const INK_DISABLED: Color = Color::srgba(0.50, 0.44, 0.36, 0.55);
const FILL: Color = style::PARCHMENT_LIGHT;
const FILL_DISABLED: Color = Color::srgba(0.85, 0.8, 0.75, 0.55);
const FILL_ON: Color = Color::srgb(0.93, 0.83, 0.5);
const FILL_HOVER: Color = Color::srgb(0.99, 0.95, 0.84);
const WAX_FILL: Color = Color::srgb(0.50, 0.10, 0.07);
const WAX_HOVER: Color = Color::srgb(0.64, 0.16, 0.10);

// The two rows and « Retraite générale » fit the band.
const _: () = assert!(2.0 * CELL.y + GAP + 5.0 + WITHDRAW_ALL_H <= super::geometry::BAND_INNER);

/// An order of the grid (`Command_<id>` / `Mode_<id>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    FireAtWill,
    Halt,
    Pursue,
    Withdraw,
    Mode(UnitMode),
}

impl Order {
    pub const fn key(self) -> &'static str {
        match self {
            Order::FireAtWill => "fire_at_will",
            Order::Halt => "halt",
            Order::Pursue => "pursue",
            Order::Withdraw => "withdraw",
            Order::Mode(UnitMode::Run) => "run",
            Order::Mode(UnitMode::Guard) => "guard",
            Order::Mode(UnitMode::Skirmish) => "skirmish",
            Order::Mode(UnitMode::Melee) => "melee",
            Order::Mode(UnitMode::Breach) => "breach",
        }
    }

    fn node_name(self) -> String {
        match self {
            Order::Mode(_) => format!("Mode_{}", self.key()),
            _ => format!("Command_{}", self.key()),
        }
    }

    fn icon_id(self) -> String {
        match self {
            Order::Mode(_) => format!("battle_mode_{}", self.key()),
            _ => format!("battle_{}", self.key()),
        }
    }

    /// Text glyph when the ink icon is missing (pursuit has none).
    fn glyph(self) -> &'static str {
        match self {
            Order::FireAtWill => "➶",
            Order::Halt => "✋",
            Order::Pursue => "»",
            Order::Withdraw => "↩",
            Order::Mode(_) => "•",
        }
    }

    /// Key map action of the order (`settings::ACTIONS`); none for withdraw and breach.
    fn action(self) -> Option<String> {
        match self {
            Order::Withdraw | Order::Mode(UnitMode::Breach) => None,
            _ => Some(format!("battle_{}", self.key())),
        }
    }

    fn title(self) -> &'static str {
        match self {
            Order::FireAtWill => "Tir à volonté",
            Order::Halt => "Halte",
            Order::Pursue => "Poursuivre",
            Order::Withdraw => "Retraite",
            Order::Mode(mode) => super::mode_icons::label_of(Order::Mode(mode).key()),
        }
    }

    fn body(self) -> String {
        match self {
            Order::FireAtWill => {
                "Tir à volonté ou tir retenu. Doré : tir à volonté pour tous les tireurs choisis."
                    .to_owned()
            }
            Order::Halt => "Arrêter la sélection sur place.".to_owned(),
            Order::Pursue => "La sélection (cavalerie surtout) pourchasse les fuyards ennemis les plus proches. Un ordre par fuyard, le plus proche de chaque régiment.".to_owned(),
            Order::Withdraw => "Faire quitter le champ aux régiments choisis.".to_owned(),
            Order::Mode(_) => format!(
                "{}\nDoré : toutes les unités choisies qui le peuvent l’ont pris.",
                super::mode_icons::tip_of(self.key())
            ),
        }
    }
}

/// The cells after the formation button, in grid order.
pub const ORDERS: [Order; 9] = [
    Order::FireAtWill,
    Order::Halt,
    Order::Pursue,
    Order::Withdraw,
    Order::Mode(UnitMode::Run),
    Order::Mode(UnitMode::Guard),
    Order::Mode(UnitMode::Skirmish),
    Order::Mode(UnitMode::Melee),
    Order::Mode(UnitMode::Breach),
];

/// Look of one button for the current selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OrderState {
    pub able: bool,
    pub on: bool,
}

#[derive(Component)]
struct OrderButton(Order);
#[derive(Component)]
struct OrderInk(Order);
#[derive(Component)]
struct OrderKey(Order);
#[derive(Component)]
struct WithdrawAllButton;

/// States of the grid (dump, tests) and the orders sent from it.
#[derive(Resource, Default, Debug)]
pub struct CommandGrid {
    pub states: Vec<(Order, OrderState)>,
    /// Keys of the orders sent by a click, in order (dump).
    pub sent: Vec<String>,
    /// « Retraite générale » confirmed.
    pub withdrawn_all: bool,
    revision: Option<(u64, Vec<u32>)>,
}

pub struct CommandsPlugin;

impl Plugin for CommandsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandGrid>()
            .add_systems(OnEnter(AppState::Battle), reset)
            .add_systems(
                Update,
                (refresh_states, clicks, withdraw_all_flow, paint, key_labels)
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

fn reset(mut grid: ResMut<CommandGrid>) {
    *grid = CommandGrid::default();
}

/// The grid and « Retraite générale », as the last child of the band `panel`.
pub fn spawn_grid(commands: &mut Commands, panel: Entity, ctx: &BuildCtx) {
    let column = commands
        .spawn((
            Name::new("CommandGrid"),
            Node {
                width: Val::Px(GRID_WIDTH),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(5.0),
                ..default()
            },
            ChildOf(panel),
        ))
        .id();
    let grid = commands
        .spawn((
            Name::new("Commands"),
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::px(COLUMNS as u16, CELL.x),
                grid_auto_rows: vec![GridTrack::px(CELL.y)],
                column_gap: Val::Px(GAP),
                row_gap: Val::Px(GAP),
                ..default()
            },
            ChildOf(column),
        ))
        .id();
    commands.spawn((
        Name::new("FormationCell"),
        HudSlot::Formation,
        Node {
            width: Val::Px(CELL.x),
            height: Val::Px(CELL.y),
            ..default()
        },
        ChildOf(grid),
    ));
    for order in ORDERS {
        spawn_button(commands, grid, ctx.theme, ctx.assets, order);
    }
    let withdraw = commands
        .spawn((
            Name::new(WITHDRAW_ALL),
            WithdrawAllButton,
            Button,
            Hovered::default(),
            Tooltip::new(
                "[b]Sonner la retraite[/b]\nToute l’armée quitte le champ (confirmation demandée).",
            ),
            Node {
                height: Val::Px(WITHDRAW_ALL_H),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(WAX_FILL),
            BorderColor::all(style::WAX_DARK),
            ChildOf(column),
        ))
        .id();
    commands.spawn((
        Text::new("Retraite générale"),
        ctx.theme.text_font(true, false, UiType::Caption.size()),
        TextColor(Color::srgb(0.98, 0.92, 0.8)),
        Pickable::IGNORE,
        ChildOf(withdraw),
    ));
}

/// One square order button: ink icon (or glyph), key letter in the corner.
fn spawn_button(
    commands: &mut Commands,
    grid: Entity,
    theme: &UiTheme,
    assets: &AssetServer,
    order: Order,
) {
    let button = commands
        .spawn((
            Name::new(order.node_name()),
            OrderButton(order),
            Button,
            Hovered::default(),
            Tooltip::new(&tip(order, "")),
            Node {
                width: Val::Px(CELL.x),
                height: Val::Px(CELL.y),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(3.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(FILL),
            BorderColor::all(style::INK_SOFT),
            ChildOf(grid),
        ))
        .id();
    let icon_id = order.icon_id();
    if theme.icons.get(&icon_id).is_some() {
        commands.spawn((
            icon(theme, assets, &icon_id, 24.0),
            OrderInk(order),
            Pickable::IGNORE,
            ChildOf(button),
        ));
    } else {
        commands.spawn((
            Text::new(order.glyph()),
            OrderInk(order),
            theme.text_font(true, false, 20.0),
            TextColor(INK),
            Pickable::IGNORE,
            ChildOf(button),
        ));
    }
    commands.spawn((
        OrderKey(order),
        Text::new(""),
        theme.text_font(false, false, 10.0),
        TextColor(style::INK_FADED),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(3.0),
            top: Val::Px(0.0),
            ..default()
        },
        Pickable::IGNORE,
        ChildOf(button),
    ));
}

fn tip(order: Order, key: &str) -> String {
    let hint = if key.is_empty() {
        String::new()
    } else {
        format!("\n[i]Touche : {key}[/i]")
    };
    format!("[b]{}[/b]\n{}{hint}", order.title(), order.body())
}

/// States of the orders for `selected` (`command_button_state` + `mode_button_state`): fire at
/// will is on when every chosen shooter fires at will, a mode when every chosen regiment that may
/// take it has it; halt / pursue / withdraw only need a regiment on the field.
pub fn order_states(
    units: &[&sim_battle::Unit],
    modes_of: impl Fn(&sim_battle::Unit) -> Vec<UnitMode>,
) -> Vec<(Order, OrderState)> {
    let present = !units.is_empty();
    ORDERS
        .iter()
        .map(|&order| {
            let state = match order {
                Order::FireAtWill => {
                    let shooters: Vec<_> = units.iter().filter(|u| u.can_shoot()).collect();
                    OrderState {
                        able: !shooters.is_empty(),
                        on: !shooters.is_empty() && shooters.iter().all(|u| u.fire_at_will),
                    }
                }
                Order::Mode(mode) => {
                    let able: Vec<_> = units
                        .iter()
                        .filter(|u| modes_of(u).contains(&mode))
                        .collect();
                    OrderState {
                        able: !able.is_empty(),
                        on: !able.is_empty() && able.iter().all(|u| mode_flag(u, mode)),
                    }
                }
                _ => OrderState {
                    able: present,
                    on: false,
                },
            };
            (order, state)
        })
        .collect()
}

fn mode_flag(unit: &sim_battle::Unit, mode: UnitMode) -> bool {
    match mode {
        UnitMode::Run => unit.mode_run,
        UnitMode::Guard => unit.guard,
        UnitMode::Skirmish => unit.skirmish,
        UnitMode::Melee => unit.melee_mode,
        UnitMode::Breach => unit.breach,
    }
}

/// Own regiments of the selection still on the field.
fn chosen(session: &mut BattleSession, selection: &[u32]) -> Vec<u32> {
    let player = session.player;
    session
        .sim_mut()
        .units()
        .iter()
        .filter(|u| selection.contains(&u.id) && u.present() && Some(u.side) == player)
        .map(|u| u.id)
        .collect()
}

fn refresh_states(
    mut grid: ResMut<CommandGrid>,
    mut session: ResMut<BattleSession>,
    selection: Res<BattleSelection>,
    data: Res<HudData>,
) {
    let key = (data.revision, selection.units.clone());
    if grid.revision.as_ref() == Some(&key) {
        return;
    }
    let ids = chosen(&mut session, &selection.units);
    let sim = session.sim_mut();
    let units: Vec<&sim_battle::Unit> =
        sim.units().iter().filter(|u| ids.contains(&u.id)).collect();
    let states = order_states(&units, |u| sim.available_modes(u));
    grid.revision = Some(key);
    if grid.states != states {
        grid.states = states;
    }
}

/// The commands a click on `order` sends for the regiments `ids` (same as the hotkeys).
pub fn commands_for(
    order: Order,
    units: &[&sim_battle::Unit],
    modes_of: impl Fn(&sim_battle::Unit) -> Vec<UnitMode>,
    pursuit: impl FnOnce(&[u32]) -> Vec<(u32, Vec<u32>)>,
) -> Vec<Command> {
    let ids: Vec<u32> = units.iter().map(|u| u.id).collect();
    if ids.is_empty() {
        return Vec::new();
    }
    match order {
        Order::Halt => vec![Command::Halt { units: ids }],
        Order::Withdraw => vec![Command::Withdraw { units: ids }],
        Order::FireAtWill => {
            let shooters: Vec<u32> = units
                .iter()
                .filter(|u| u.can_shoot())
                .map(|u| u.id)
                .collect();
            if shooters.is_empty() {
                return Vec::new();
            }
            let enabled = units.iter().any(|u| u.can_shoot() && !u.fire_at_will);
            vec![Command::FireAtWill {
                units: shooters,
                enabled,
            }]
        }
        Order::Pursue => pursuit(&ids)
            .into_iter()
            .map(|(target, chasers)| Command::Pursue {
                units: chasers,
                target,
                queue: false,
            })
            .collect(),
        Order::Mode(mode) => {
            let able: Vec<&&sim_battle::Unit> = units
                .iter()
                .filter(|u| modes_of(u).contains(&mode))
                .collect();
            if able.is_empty() {
                return Vec::new();
            }
            let enabled = able.iter().any(|u| !mode_flag(u, mode));
            vec![Command::SetMode {
                units: able.iter().map(|u| u.id).collect(),
                mode,
                enabled,
            }]
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn clicks(
    mut pressed: MessageReader<ButtonPressed>,
    buttons: Query<&OrderButton>,
    withdraw: Query<(), With<WithdrawAllButton>>,
    mut session: ResMut<BattleSession>,
    selection: Res<BattleSelection>,
    views: Option<Res<BattleViews>>,
    mut issue: MessageWriter<IssueBattleCommand>,
    mut queue: ResMut<ModalQueue>,
    mut grid: ResMut<CommandGrid>,
) {
    for press in pressed.read() {
        if withdraw.get(press.entity).is_ok() {
            if !session.snapshot.finished && !session.replaying {
                queue.request(
                    WITHDRAW_ALL_KEY,
                    PRIORITY_DECISION,
                    ModalSpec {
                        yes_label: "Sonner la retraite".to_owned(),
                        no_label: "Tenir le champ".to_owned(),
                        ..ModalSpec::confirm(
                            "Sonner la retraite générale ?",
                            "Tous vos régiments encore en ordre quittent le champ. La bataille sera perdue, mais l’ost sera sauf.",
                        )
                    },
                );
            }
            continue;
        }
        let Ok(OrderButton(order)) = buttons.get(press.entity) else {
            continue;
        };
        let order = *order;
        if grid.states.iter().any(|(o, s)| *o == order && !s.able) {
            continue;
        }
        let ids = chosen(&mut session, &selection.units);
        let player = session.player;
        let sim = session.sim_mut();
        let units: Vec<&sim_battle::Unit> =
            sim.units().iter().filter(|u| ids.contains(&u.id)).collect();
        let sent = commands_for(
            order,
            &units,
            |u| sim.available_modes(u),
            |ids| match (player, views.as_deref()) {
                (Some(side), Some(views)) => pursuit_groups(&views.0, side, ids),
                _ => Vec::new(),
            },
        );
        for command in sent {
            issue.write(IssueBattleCommand(command));
        }
        grid.sent.push(order.key().to_owned());
        grid.revision = None;
    }
}

fn withdraw_all_flow(
    mut results: MessageReader<ModalResult>,
    mut session: ResMut<BattleSession>,
    mut issue: MessageWriter<IssueBattleCommand>,
    mut grid: ResMut<CommandGrid>,
) {
    for result in results.read() {
        if result.key != WITHDRAW_ALL_KEY || !result.confirmed {
            continue;
        }
        let player = session.player;
        let units: Vec<u32> = session
            .sim_mut()
            .units()
            .iter()
            .filter(|u| Some(u.side) == player && u.present() && !u.synthetic)
            .map(|u| u.id)
            .collect();
        if !units.is_empty() {
            issue.write(IssueBattleCommand(Command::Withdraw { units }));
            grid.withdrawn_all = true;
        }
    }
}

#[allow(clippy::type_complexity)]
fn paint(
    grid: Res<CommandGrid>,
    mut buttons: Query<(&OrderButton, &Hovered, &mut BackgroundColor), Without<WithdrawAllButton>>,
    mut inks: Query<(&OrderInk, Option<&mut ImageNode>, Option<&mut TextColor>)>,
    mut withdraw: Query<(&Hovered, &mut BackgroundColor), With<WithdrawAllButton>>,
) {
    let state_of = |order: Order| {
        grid.states
            .iter()
            .find(|(o, _)| *o == order)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    };
    for (button, hovered, mut background) in &mut buttons {
        let state = state_of(button.0);
        let wanted = if !state.able {
            FILL_DISABLED
        } else if state.on {
            FILL_ON
        } else if hovered.get() {
            FILL_HOVER
        } else {
            FILL
        };
        if background.0 != wanted {
            background.0 = wanted;
        }
    }
    for (ink, image, text) in &mut inks {
        let colour = if state_of(ink.0).able {
            INK
        } else {
            INK_DISABLED
        };
        if let Some(mut image) = image {
            let tint = if state_of(ink.0).able {
                Color::WHITE
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.4)
            };
            if image.color != tint {
                image.color = tint;
            }
        }
        if let Some(mut text) = text
            && text.0 != colour
        {
            text.0 = colour;
        }
    }
    for (hovered, mut background) in &mut withdraw {
        let wanted = if hovered.get() { WAX_HOVER } else { WAX_FILL };
        if background.0 != wanted {
            background.0 = wanted;
        }
    }
}

/// Key letters and tooltips follow the key map and the keyboard layout.
fn key_labels(
    keymap: Res<KeyMap>,
    settings: Option<Res<Settings>>,
    mut keys: Query<(&OrderKey, &mut Text)>,
    mut tips: Query<(&OrderButton, &mut Tooltip)>,
    mut done: Local<bool>,
) {
    if *done && !keymap.is_changed() && !settings.as_ref().is_some_and(|s| s.is_changed()) {
        return;
    }
    if keys.is_empty() {
        return;
    }
    *done = true;
    let azerty = super::orders_bar::is_azerty(settings.as_deref());
    let label = |order: Order| {
        order.action().map_or(String::new(), |action| {
            super::orders_bar::action_label(&keymap, &action, azerty)
        })
    };
    for (key, mut text) in &mut keys {
        let value = label(key.0);
        // A long label (two chords) does not fit the corner; the tooltip keeps it.
        let shown = if value.chars().count() > 3 {
            String::new()
        } else {
            value
        };
        if text.0 != shown {
            text.0 = shown;
        }
    }
    for (button, mut tooltip) in &mut tips {
        let wanted = tip(button.0, &label(button.0));
        if tooltip.bbcode != wanted {
            tooltip.bbcode = wanted;
        }
    }
}

pub fn dump(world: &World) -> Value {
    let Some(grid) = world.get_resource::<CommandGrid>() else {
        return Value::Null;
    };
    let states: serde_json::Map<String, Value> = grid
        .states
        .iter()
        .map(|(o, s)| (o.key().to_owned(), json!({ "able": s.able, "on": s.on })))
        .collect();
    json!({
        "states": states,
        "sent": grid.sent,
        "withdrawn_all": grid.withdrawn_all,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_selected_greys_everything() {
        let states = order_states(&[], |_| Vec::new());
        assert_eq!(states.len(), ORDERS.len());
        assert!(states.iter().all(|(_, s)| !s.able && !s.on));
        assert!(commands_for(Order::Halt, &[], |_| Vec::new(), |_| Vec::new()).is_empty());
    }

    #[test]
    fn order_keys_and_names_are_distinct() {
        let mut names: Vec<String> = ORDERS.iter().map(|o| o.node_name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ORDERS.len());
        assert_eq!(Order::Mode(UnitMode::Run).node_name(), "Mode_run");
        assert_eq!(Order::Halt.icon_id(), "battle_halt");
        assert!(Order::Withdraw.action().is_none());
    }
}
