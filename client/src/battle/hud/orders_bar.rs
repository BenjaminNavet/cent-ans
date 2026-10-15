//! The leader's seal and orders bar (bottom left). Port of leader_orders_bar.gd and of the seal of
//! battle_hud.gd (`_build_leader_seal`, drawn like general_seal.gd: a wax disc, the portrait in a
//! medallion, a ring coloured by the morale of the leader's guard, « ★ command » under it).
//!
//! No rule here: the list of orders, their availability, the reason of a refusal and the cooldown
//! come from [`sim_battle::BattleSim::leader_orders`] (read through its serialised form, some
//! fields being crate-private), refreshed every 0.2 s; a click or a shortcut sends
//! `Command::LeaderOrder` through [`IssueBattleCommand`]. In a siege a « Incendier » button is
//! added at the end of the bar: target and refusal come from [`sim_battle::BattleSim::burn_choice`]
//! and the button sends the `Command::Burn` it proposes.
//!
//! Shortcuts are the physical keys Z X V B (orders in the order of the bar) and I (burn), as in
//! Godot; the labels shown are the QWERTY ones (Godot read the keyboard layout).
//!
//! Click on the seal selects the leader's regiment, a double click centres the camera on it.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use serde_json::{Value, json};
use sim_battle::{Command, SideId};

use super::{BattleHudRoot, HudData};
use crate::app::AppState;
use crate::battle::orders::input::IssueBattleCommand;
use crate::battle::orders::{BattleSelection, SelectBattleUnits};
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCameraState;
use crate::paths;
use crate::ui::kit::icons::icon;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::caption;
use crate::ui::kit::{ButtonPressed, UiTheme};

pub const SEAL_SIZE: f32 = 88.0;
/// Left margin of the card band (room for the seal).
pub const BAND_LEFT_MARGIN: f32 = 8.0 + SEAL_SIZE + 8.0;
const BUTTON_W: f32 = 96.0;
const BUTTON_H: f32 = 54.0;
/// Bottom margin of the bar: above the card band and its ability row.
const BOTTOM_MARGIN: f32 = 166.0;
const REFRESH_S: f32 = 0.2;
const DOUBLE_CLICK_S: f32 = 0.35;
/// Physical keys of the orders, in the order of the bar.
pub const HOTKEYS: [KeyCode; 4] = [KeyCode::KeyZ, KeyCode::KeyX, KeyCode::KeyV, KeyCode::KeyB];
pub const BURN_HOTKEY: KeyCode = KeyCode::KeyI;
const BURN_ID: &str = "burn";
const BURN_NAME: &str = "Incendier";
const BURN_DESCRIPTION: &str = "Un régiment porte la torche à la maison ou à la porte la plus proche à sa portée (la garnison atteint ses faubourgs de partout). La pluie et la neige gênent la mise à feu ; le feu gagne ensuite les maisons voisines sous le vent.";
const INK: Color = Color::srgb(0.22, 0.14, 0.07);
const INK_MUTED: Color = Color::srgb(0.42, 0.34, 0.26);

/// Letter of a physical key for the labels (QWERTY).
pub fn key_label(key: KeyCode) -> &'static str {
    match key {
        KeyCode::KeyZ => "Z",
        KeyCode::KeyX => "X",
        KeyCode::KeyV => "V",
        KeyCode::KeyB => "B",
        KeyCode::KeyI => "I",
        _ => "",
    }
}

/// Fallback glyph by order kind (`GLYPHS`).
pub fn glyph_of(kind: &str) -> &'static str {
    match kind {
        "war_cry" => "✠",
        "rally" => "⚑",
        "dismount" => "♞",
        "pavise" => "▮",
        "no_quarter" => "⚔",
        "burn" => "♨",
        _ => "✦",
    }
}

/// One order of the bar (`OrderView`, or the burn order).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderState {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub label: String,
    pub description: String,
    pub icon: String,
    pub available: bool,
    pub reason: String,
    pub cooldown: f64,
    pub cooldown_remaining: f64,
    pub uses_per_battle: Option<u32>,
}

impl OrderState {
    pub fn from_json(value: &Value) -> Self {
        let text = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let number = |key: &str| value.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        Self {
            id: text("id"),
            kind: text("kind"),
            name: text("name"),
            label: text("label"),
            description: text("description"),
            icon: text("icon"),
            available: value
                .get("available")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            reason: text("reason"),
            cooldown: number("cooldown"),
            cooldown_remaining: number("cooldown_remaining"),
            uses_per_battle: value
                .get("uses_per_battle")
                .and_then(Value::as_u64)
                .map(|n| n as u32),
        }
    }

    /// Tooltip (`OrderButton._make_custom_tooltip`).
    pub fn tip(&self, hotkey: &str) -> String {
        let mut lines = vec![format!(
            "[b]{}[/b]   [color=#6b5a45][{hotkey}][/color]",
            self.name
        )];
        if self.label != self.name {
            lines.push(format!("[i]{}[/i]", self.label));
        }
        lines.push(self.description.clone());
        let mut facts = Vec::new();
        if self.cooldown > 0.0 {
            facts.push(format!("recharge {} s", self.cooldown as i64));
        }
        if let Some(n) = self.uses_per_battle {
            facts.push(format!("{n} fois par bataille"));
        }
        if !facts.is_empty() {
            lines.push(format!("[color=#6b5a45]{}[/color]", facts.join(" · ")));
        }
        if !self.available {
            lines.push(format!(
                "[color=#8a2a1a]Indisponible : {}[/color]",
                self.reason
            ));
        }
        lines.join("\n")
    }

    /// Share of the cooldown that remains, 0..1.
    pub fn cooldown_fraction(&self) -> f32 {
        if self.cooldown <= 0.0 {
            0.0
        } else {
            (self.cooldown_remaining / self.cooldown).clamp(0.0, 1.0) as f32
        }
    }
}

/// The « Incendier » order as a button: description and target distance when possible.
pub fn burn_view(available: bool, reason: &str, target: &str, distance_m: f64) -> OrderState {
    let mut description = BURN_DESCRIPTION.to_owned();
    if available {
        description += &format!("\nCible : {target}, à {} m.", distance_m.round() as i64);
    }
    OrderState {
        id: BURN_ID.into(),
        kind: "burn".into(),
        name: BURN_NAME.into(),
        label: BURN_NAME.into(),
        description,
        icon: "burn".into(),
        available,
        reason: reason.to_owned(),
        ..default()
    }
}

/// What the bar shows now.
#[derive(Resource, Default, Debug)]
pub struct LeaderOrders {
    pub orders: Vec<OrderState>,
    /// `Some` in a siege.
    pub burn: Option<OrderState>,
    /// The `Command::Burn` the core proposes.
    pub burn_command: Option<Command>,
    pub timer: f32,
    /// Revision of what the buttons show (bumped on change).
    pub revision: u64,
    pub sent: Vec<String>,
}

/// The general of the player's army.
#[derive(Resource, Default, Debug, Clone)]
pub struct LeaderSeal {
    pub name: String,
    pub character: String,
    pub command: u8,
    pub has_general: bool,
    last_click: Option<f32>,
    pub clicks: u32,
}

#[derive(Component)]
struct OrderButton(String);

#[derive(Component)]
struct OrderShade(String);

#[derive(Component)]
struct OrderTimer(String);

#[derive(Component)]
struct OrdersPanel;

#[derive(Component)]
struct SealNode;

pub struct OrdersBarPlugin;

impl Plugin for OrdersBarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LeaderOrders>()
            .init_resource::<LeaderSeal>()
            .add_systems(OnEnter(AppState::Battle), reset)
            .add_systems(
                Update,
                (
                    spawn,
                    refresh_orders,
                    rebuild_buttons,
                    order_input,
                    update_buttons,
                    seal_clicks,
                    update_seal,
                )
                    .chain()
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

fn reset(mut orders: ResMut<LeaderOrders>, mut seal: ResMut<LeaderSeal>) {
    *orders = LeaderOrders::default();
    *seal = LeaderSeal::default();
}

fn portrait_exists(id: &str) -> bool {
    !id.is_empty()
        && paths::find_root()
            .map(|root| {
                paths::asset_root(&root)
                    .join(format!("portraits/{id}.png"))
                    .exists()
            })
            .unwrap_or(false)
}

/// Spawns the seal and the (empty) bar once the HUD root exists.
#[allow(clippy::too_many_arguments)]
fn spawn(
    mut commands: Commands,
    roots: Query<Entity, With<BattleHudRoot>>,
    existing: Query<(), With<SealNode>>,
    mut session: ResMut<BattleSession>,
    mut seal: ResMut<LeaderSeal>,
    data: Res<HudData>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
) {
    let Ok(root) = roots.single() else { return };
    if !existing.is_empty() {
        return;
    }
    let player = session.player.unwrap_or(SideId::Attacker);
    if let Some(general) = session.sim_mut().setup().side(player).general.clone() {
        seal.name = general.name;
        seal.character = general.character;
        seal.command = general.command;
        seal.has_general = true;
    }
    let wax = data.colors[0].darker(0.45).mix(&style::WAX, 0.5);
    let seal_node = commands
        .spawn((
            Name::new("LeaderSeal"),
            SealNode,
            Button,
            Hovered::default(),
            Tooltip::new("[b]Pas de chef[/b]\nCette armée n’a pas de chef."),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                bottom: Val::Px(6.0),
                width: Val::Px(SEAL_SIZE),
                height: Val::Px(SEAL_SIZE),
                border: UiRect::all(Val::Px(5.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(wax),
            BorderColor::all(Color::srgb(0.2, 0.6, 0.2)),
            ChildOf(root),
        ))
        .id();
    let inner = commands
        .spawn((
            Name::new("SealMedallion"),
            Node {
                width: Val::Px(SEAL_SIZE - 22.0),
                height: Val::Px(SEAL_SIZE - 22.0),
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow::clip(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(style::PARCHMENT_DARK),
            BorderColor::all(style::GOLD_PALE),
            Pickable::IGNORE,
            ChildOf(seal_node),
        ))
        .id();
    if portrait_exists(&seal.character) {
        commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ImageNode::new(assets.load(format!("portraits/{}.png", seal.character))),
            Pickable::IGNORE,
            ChildOf(inner),
        ));
    } else {
        commands.spawn((
            Text::new("★"),
            theme.text_font(true, false, 26.0),
            TextColor(style::WAX_DARK),
            Pickable::IGNORE,
            ChildOf(inner),
        ));
    }
    commands.spawn((
        Name::new("SealCommand"),
        Text::new(if seal.command > 0 {
            format!("★ {}", seal.command)
        } else {
            String::new()
        }),
        theme.text_font(true, false, 14.0),
        TextColor(Color::srgb(1.0, 0.84, 0.45)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(-4.0),
            ..default()
        },
        Pickable::IGNORE,
        ChildOf(seal_node),
    ));
    // The (empty) bar; buttons are added when the first orders are read.
    let panel = commands
        .spawn((
            Name::new("LeaderOrdersBar"),
            OrdersPanel,
            crate::ui::kit::widgets::panel_page(&theme),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                bottom: Val::Px(BOTTOM_MARGIN),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                display: Display::None,
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    commands.spawn((caption(&theme, "Ordres du chef"), ChildOf(panel)));
    commands.spawn((
        Name::new("OrderRow"),
        Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            ..default()
        },
        ChildOf(panel),
    ));
}

/// Reads the orders every 0.2 s.
fn refresh_orders(
    time: Res<Time<Real>>,
    selection: Res<BattleSelection>,
    mut session: ResMut<BattleSession>,
    mut orders: ResMut<LeaderOrders>,
) {
    orders.timer -= time.delta_secs();
    if orders.timer > 0.0 {
        return;
    }
    orders.timer = REFRESH_S;
    let player = session.player.unwrap_or(SideId::Attacker);
    let sim = session.sim_mut();
    let views: Vec<OrderState> = sim
        .leader_orders(player)
        .iter()
        .filter_map(|view| serde_json::to_value(view).ok())
        .map(|value| OrderState::from_json(&value))
        .collect();
    let (burn, command) = if sim.fire_rules().is_some() && sim.siege().is_some() {
        match sim.burn_choice(player, &selection.units) {
            Ok(choice) => {
                let target = match choice.house {
                    Some(_) if choice.suburb => "une maison du faubourg",
                    Some(_) => "une maison",
                    None => "la porte",
                };
                (
                    Some(burn_view(true, "", target, choice.distance_m)),
                    Some(Command::Burn {
                        units: vec![choice.unit],
                        house: choice.house,
                        gate: choice.house.is_none(),
                    }),
                )
            }
            Err(error) => (Some(burn_view(false, &error.to_string(), "", 0.0)), None),
        }
    } else {
        (None, None)
    };
    if views != orders.orders || burn != orders.burn {
        orders.revision += 1;
    }
    orders.orders = views;
    orders.burn = burn;
    orders.burn_command = command;
}

/// Creates a button per order the first time it is seen.
#[allow(clippy::too_many_arguments)]
fn rebuild_buttons(
    mut commands: Commands,
    orders: Res<LeaderOrders>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    rows: Query<(Entity, Option<&Children>, &Name)>,
    buttons: Query<&OrderButton>,
) {
    if !orders.is_changed() {
        return;
    }
    let Some((row, _, _)) = rows.iter().find(|(_, _, name)| name.as_str() == "OrderRow") else {
        return;
    };
    let have: Vec<&str> = buttons.iter().map(|b| b.0.as_str()).collect();
    let all = orders.orders.iter().chain(orders.burn.iter());
    for (index, order) in all.enumerate() {
        if have.contains(&order.id.as_str()) {
            continue;
        }
        let hotkey = if order.id == BURN_ID {
            key_label(BURN_HOTKEY)
        } else {
            HOTKEYS.get(index).map_or("", |k| key_label(*k))
        };
        spawn_button(&mut commands, row, &theme, &assets, order, hotkey);
    }
}

fn spawn_button(
    commands: &mut Commands,
    row: Entity,
    theme: &UiTheme,
    assets: &AssetServer,
    order: &OrderState,
    hotkey: &str,
) {
    let id = order.id.clone();
    let button = commands
        .spawn((
            Name::new(format!("order:{id}")),
            OrderButton(id.clone()),
            Button,
            Hovered::default(),
            Tooltip::new(&order.tip(hotkey)),
            Node {
                width: Val::Px(BUTTON_W),
                height: Val::Px(BUTTON_H),
                border: UiRect::all(Val::Px(1.0)),
                overflow: Overflow::clip(),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(style::PARCHMENT_LIGHT),
            BorderColor::all(style::INK_SOFT),
            ChildOf(row),
        ))
        .id();
    let icon_id = format!("order_{}", order.icon);
    if !order.icon.is_empty() && theme.icons.get(&icon_id).is_some() {
        commands.spawn((
            icon(theme, assets, &icon_id, 28.0),
            Pickable::IGNORE,
            ChildOf(button),
        ));
    } else {
        commands.spawn((
            Text::new(glyph_of(&order.kind)),
            theme.text_font(true, false, UiType::Title.size()),
            TextColor(INK),
            Pickable::IGNORE,
            ChildOf(button),
        ));
    }
    commands.spawn((
        Text::new(order.name.clone()),
        theme.text_font(false, false, UiType::Caption.size()),
        TextColor(INK),
        TextLayout::justify(Justify::Center),
        Pickable::IGNORE,
        ChildOf(button),
    ));
    commands.spawn((
        Text::new(hotkey),
        theme.text_font(false, false, UiType::Caption.size()),
        TextColor(INK_MUTED),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(5.0),
            top: Val::Px(1.0),
            ..default()
        },
        Pickable::IGNORE,
        ChildOf(button),
    ));
    commands.spawn((
        OrderShade(id.clone()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            height: Val::Percent(0.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.07, 0.04, 0.35)),
        Pickable::IGNORE,
        ChildOf(button),
    ));
    commands.spawn((
        OrderTimer(id),
        Text::new(""),
        theme.text_font(true, false, UiType::Caption.size()),
        TextColor(Color::srgb(1.0, 0.95, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(5.0),
            top: Val::Px(0.0),
            ..default()
        },
        Pickable::IGNORE,
        ChildOf(button),
    ));
}

/// The order to give for `id`: the command, or the reason it cannot be given.
pub fn command_for(
    orders: &LeaderOrders,
    id: &str,
    selection: &[u32],
    present: &[u32],
) -> Result<Command, String> {
    if id == BURN_ID {
        return match (&orders.burn, &orders.burn_command) {
            (Some(burn), Some(command)) if burn.available => Ok(command.clone()),
            (Some(burn), _) => Err(format!("Incendier : {}", burn.reason)),
            _ => Err("Incendier : pas de siège".into()),
        };
    }
    let units: Vec<u32> = selection
        .iter()
        .copied()
        .filter(|u| present.contains(u))
        .collect();
    Ok(Command::LeaderOrder {
        side: None,
        order: id.to_owned(),
        units,
    })
}

#[allow(clippy::too_many_arguments)]
fn order_input(
    mut pressed: MessageReader<ButtonPressed>,
    keys: Res<ButtonInput<KeyCode>>,
    selection: Res<BattleSelection>,
    data: Res<HudData>,
    mut orders: ResMut<LeaderOrders>,
    mut issue: MessageWriter<IssueBattleCommand>,
    mut toasts: MessageWriter<crate::ui::kit::modal::ToastRequest>,
) {
    let mut wanted: Vec<String> = pressed
        .read()
        .filter_map(|p| p.name.strip_prefix("order:").map(str::to_owned))
        .collect();
    let blocked = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
    ]);
    if !blocked {
        for (index, key) in HOTKEYS.iter().enumerate() {
            if keys.just_pressed(*key)
                && let Some(order) = orders.orders.get(index)
            {
                wanted.push(order.id.clone());
            }
        }
        if keys.just_pressed(BURN_HOTKEY) && orders.burn.is_some() {
            wanted.push(BURN_ID.to_owned());
        }
    }
    let present: Vec<u32> = data
        .own
        .iter()
        .filter(|u| u.present)
        .map(|u| u.id)
        .collect();
    for id in wanted {
        match command_for(&orders, &id, &selection.units, &present) {
            Ok(command) => {
                orders.sent.push(id);
                issue.write(IssueBattleCommand(command));
                orders.timer = 0.0;
            }
            Err(reason) => {
                toasts.write(crate::ui::kit::modal::ToastRequest::new(&reason));
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_buttons(
    orders: Res<LeaderOrders>,
    mut buttons: Query<(&OrderButton, &mut BackgroundColor, &mut Tooltip)>,
    mut shades: Query<(&OrderShade, &mut Node)>,
    mut timers: Query<(&OrderTimer, &mut Text)>,
) {
    if !orders.is_changed() {
        return;
    }
    let find = |id: &str| {
        orders
            .orders
            .iter()
            .chain(orders.burn.iter())
            .enumerate()
            .find(|(_, o)| o.id == id)
    };
    for (button, mut background, mut tooltip) in &mut buttons {
        if let Some((index, order)) = find(&button.0) {
            let hotkey = if order.id == BURN_ID {
                key_label(BURN_HOTKEY)
            } else {
                HOTKEYS.get(index).map_or("", |k| key_label(*k))
            };
            *background = BackgroundColor(if order.available {
                style::PARCHMENT_LIGHT
            } else {
                Color::srgba(0.85, 0.8, 0.75, 0.6)
            });
            let tip = order.tip(hotkey);
            if tooltip.bbcode != tip {
                tooltip.bbcode = tip;
            }
        }
    }
    for (shade, mut node) in &mut shades {
        if let Some((_, order)) = find(&shade.0) {
            let fraction = if order.cooldown_remaining > 0.0 {
                order.cooldown_fraction()
            } else {
                0.0
            };
            node.height = Val::Percent(fraction * 100.0);
        }
    }
    for (timer, mut text) in &mut timers {
        if let Some((_, order)) = find(&timer.0) {
            let shown = if order.cooldown_remaining > 0.0 && order.cooldown > 0.0 {
                format!("{} s", order.cooldown_remaining.ceil() as i64)
            } else {
                String::new()
            };
            if text.0 != shown {
                text.0 = shown;
            }
        }
    }
}

/// Visibility of the bar: orders or a siege, battle not finished.
fn show_bar(orders: &LeaderOrders, finished: bool) -> bool {
    (!orders.orders.is_empty() || orders.burn.is_some()) && !finished
}

fn seal_clicks(
    mut pressed: MessageReader<ButtonPressed>,
    time: Res<Time<Real>>,
    data: Res<HudData>,
    mut seal: ResMut<LeaderSeal>,
    mut select: MessageWriter<SelectBattleUnits>,
    camera: Option<ResMut<BattleCameraState>>,
) {
    let mut camera = camera;
    for press in pressed.read() {
        if press.name != "LeaderSeal" {
            continue;
        }
        seal.clicks += 1;
        let Some(leader) = data.own.iter().find(|u| u.is_general) else {
            continue;
        };
        select.write(SelectBattleUnits {
            units: vec![leader.id],
            additive: false,
        });
        let now = time.elapsed_secs();
        let double = seal.last_click.is_some_and(|at| now - at <= DOUBLE_CLICK_S);
        seal.last_click = Some(now);
        if double && let Some(camera) = camera.as_deref_mut() {
            camera.target.x = leader.x as f32;
            camera.target.z = leader.z as f32;
        }
    }
}

/// Morale ring, tooltip and the bar's visibility.
#[allow(clippy::type_complexity)]
fn update_seal(
    data: Res<HudData>,
    seal: Res<LeaderSeal>,
    orders: Res<LeaderOrders>,
    mut seals: Query<(&mut BorderColor, &mut Tooltip), With<SealNode>>,
    mut panels: Query<&mut Node, With<OrdersPanel>>,
) {
    for mut node in &mut panels {
        let display = if show_bar(&orders, data.finished) {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    if !data.is_changed() {
        return;
    }
    let leader = data.own.iter().find(|u| u.is_general);
    for (mut border, mut tooltip) in &mut seals {
        let morale = leader.map_or(1.0, |u| u.morale / 100.0).clamp(0.0, 1.0);
        *border = BorderColor::all(style::gauge_color(morale));
        if !seal.has_general {
            continue;
        }
        let mut body = format!("Commandement {} / 10", seal.command);
        if let Some(unit) = leader {
            body += &format!(
                "\nGarde du chef : {} hommes · moral {}",
                unit.soldiers, unit.morale as i64
            );
            if !unit.present {
                body += "\nLe chef est tombé ou a quitté le champ.";
            }
        }
        let tip = format!(
            "[b]{}[/b]\n{body}\n[i]clic : sélectionner sa garde · double clic : y aller[/i]",
            seal.name
        );
        if tooltip.bbcode != tip {
            tooltip.bbcode = tip;
        }
    }
}

pub fn dump(world: &World) -> Value {
    let orders = world.resource::<LeaderOrders>();
    let seal = world.resource::<LeaderSeal>();
    json!({
        "orders": orders.orders.iter().map(|o| json!({
            "id": o.id, "name": o.name, "label": o.label, "available": o.available,
            "reason": o.reason, "cooldown": o.cooldown,
        })).collect::<Vec<_>>(),
        "burn": orders.burn.as_ref().map(|b| json!({"available": b.available, "reason": b.reason})),
        "sent": orders.sent,
        "seal": {"general": seal.name, "command": seal.command, "clicks": seal.clicks},
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(id: &str, available: bool) -> OrderState {
        OrderState {
            id: id.into(),
            kind: "war_cry".into(),
            name: "Cri de guerre".into(),
            label: "Montjoie !".into(),
            description: "Le chef crie.".into(),
            available,
            reason: "recharge, encore 4 s".into(),
            cooldown: 30.0,
            cooldown_remaining: 7.5,
            uses_per_battle: Some(2),
            ..default()
        }
    }

    #[test]
    fn tooltip_lists_label_facts_and_refusal() {
        let tip = order("war_cry", false).tip("Z");
        assert!(tip.contains("[Z]"));
        assert!(tip.contains("[i]Montjoie !"));
        assert!(tip.contains("recharge 30 s · 2 fois par bataille"));
        assert!(tip.contains("Indisponible : recharge, encore 4 s"));
        assert!((order("a", true).cooldown_fraction() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn leader_order_command_keeps_present_selected_units() {
        let orders = LeaderOrders::default();
        let command = command_for(&orders, "war_cry", &[3, 4, 9], &[3, 9]).unwrap();
        match command {
            Command::LeaderOrder { order, units, .. } => {
                assert_eq!(order, "war_cry");
                assert_eq!(units, vec![3, 9]);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn burn_needs_a_proposal() {
        let mut orders = LeaderOrders::default();
        assert!(command_for(&orders, "burn", &[], &[]).is_err());
        orders.burn = Some(burn_view(false, "rien à portée", "", 0.0));
        assert_eq!(
            command_for(&orders, "burn", &[], &[]).unwrap_err(),
            "Incendier : rien à portée"
        );
        orders.burn = Some(burn_view(true, "", "la porte", 41.6));
        orders.burn_command = Some(Command::Burn {
            units: vec![2],
            house: None,
            gate: true,
        });
        assert!(command_for(&orders, "burn", &[], &[]).is_ok());
        assert!(orders.burn.as_ref().unwrap().description.contains("à 42 m"));
    }

    #[test]
    fn bar_hides_when_finished_or_empty() {
        let mut orders = LeaderOrders::default();
        assert!(!show_bar(&orders, false));
        orders.orders.push(order("a", true));
        assert!(show_bar(&orders, false));
        assert!(!show_bar(&orders, true));
    }
}
