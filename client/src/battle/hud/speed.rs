//! Top bar (title, clock, time left, balance of forces, « Quitter ») and the speed controls
//! (pause, ×0,5, ×1, ×2, ×4). Port of battle_hud.gd `_build_top_bar`, `_build_corner`,
//! `set_clock`, `set_time_left` and the pause / speed keys of battle_scene.gd (Space, + and −).
//!
//! Speed and pause are the session's (`BattleSession::speed`, `paused`); the simulation steps
//! with them in `battle::scene`.

use bevy::prelude::*;
use serde_json::{Value, json};

use super::tips::speed_tip;
use super::{BuildCtx, HudData};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::flow::LeaveBattle;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::modal::{ModalQueue, ModalResult, ModalSpec, PRIORITY_DECISION};
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{Toggled, button_small, heading, panel_note, toggle_button};

/// Width of the speed column (the minimap sits to its left).
pub const SPEED_PANEL_WIDTH: f32 = 72.0;
/// Height of the speed column as laid out (5 buttons).
pub const SPEED_COLUMN_HEIGHT: f32 = 174.0;

/// Selectable speeds (`BattleScene.SPEEDS`).
pub const SPEEDS: [f64; 4] = [0.5, 1.0, 2.0, 4.0];
const SPEED_KEYS: [&str; 5] = ["pause", "0.5", "1", "2", "4"];
const SPEED_CAPTIONS: [&str; 5] = ["Pause", "×0,5", "×1", "×2", "×4"];
const QUIT_KEY: &str = "battle_quit";

/// `mm:ss`.
pub fn clock_text(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// « Nuit dans mm:ss » (`BattleHud.time_left_text`).
pub fn time_left_text(seconds: f64) -> String {
    let total = seconds.max(0.0).ceil() as u64;
    format!("Nuit dans {:02}:{:02}", total / 60, total % 60)
}

/// Index of the active button: 0 = pause, 1.. = [`SPEEDS`].
pub fn active_index(paused: bool, speed: f64) -> usize {
    if paused {
        0
    } else {
        SPEEDS
            .iter()
            .position(|s| (s - speed).abs() < 1e-9)
            .map_or(2, |i| i + 1)
    }
}

/// Next speed index after `delta` steps (clamped), from the current speed.
pub fn step_speed(speed: f64, delta: i32) -> f64 {
    let current = SPEEDS
        .iter()
        .position(|s| (s - speed).abs() < 1e-9)
        .unwrap_or(1) as i32;
    SPEEDS[(current + delta).clamp(0, SPEEDS.len() as i32 - 1) as usize]
}

/// Share of the own side in the balance bar, 0..1.
pub fn balance_share(own: u32, enemy: u32) -> f32 {
    let total = (own + enemy).max(1);
    own as f32 / total as f32
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum TopText {
    Clock,
    TimeLeft,
    Balance,
}
#[derive(Component)]
struct BalanceOwnFill;
#[derive(Component)]
struct SpeedButton(usize);

pub struct SpeedPlugin;

impl Plugin for SpeedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                speed_keys.run_if(crate::ui::kit::text_field::keyboard_free),
                speed_input,
                quit_flow,
                update_top_bar,
                sync_speed_buttons,
            )
                .chain()
                .run_if(in_state(AppState::Battle))
                .run_if(resource_exists::<BattleSession>),
        );
    }
}

/// Spawns the top bar.
pub fn spawn_top_bar(commands: &mut Commands, root: Entity, ctx: &BuildCtx) {
    let wrapper = commands
        .spawn((
            Name::new("TopBarWrapper"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(root),
        ))
        .id();
    let frame = commands
        .spawn((panel_note(ctx.theme), Name::new("TopBar"), ChildOf(wrapper)))
        .id();
    let bar = commands
        .spawn((
            Name::new("TopBarRow"),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(18.0),
                ..default()
            },
            ChildOf(frame),
        ))
        .id();
    commands.spawn((heading(ctx.theme, "Bataille"), ChildOf(bar)));
    commands.spawn((
        TopText::Clock,
        Text::new("00:00"),
        ctx.theme.text_font(false, false, UiType::Body.size()),
        TextColor(style::INK),
        ChildOf(bar),
    ));
    commands.spawn((
        TopText::TimeLeft,
        Tooltip::new("[b]Temps restant[/b]\nÀ la nuit, le défenseur l’emporte."),
        Text::new(""),
        ctx.theme.text_font(false, true, UiType::Caption.size()),
        TextColor(style::INK_SOFT),
        ChildOf(bar),
    ));
    let balance = commands
        .spawn((
            Name::new("Balance"),
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(2.0),
                ..default()
            },
            ChildOf(bar),
        ))
        .id();
    commands.spawn((
        TopText::Balance,
        Text::new(""),
        ctx.theme.text_font(false, false, UiType::Caption.size()),
        TextColor(style::INK),
        ChildOf(balance),
    ));
    let track = commands
        .spawn((
            Node {
                width: Val::Px(220.0),
                height: Val::Px(12.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(ctx.data.colors[1]),
            BorderColor::all(style::INK_SOFT),
            ChildOf(balance),
        ))
        .id();
    commands.spawn((
        BalanceOwnFill,
        Node {
            width: Val::Percent(50.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(ctx.data.colors[0]),
        ChildOf(track),
    ));
    commands.spawn((
        button_small(ctx.theme, "battle:quit", "Quitter"),
        ChildOf(bar),
    ));
}

/// Spawns the speed column, bottom right.
pub fn spawn_speed_panel(commands: &mut Commands, root: Entity, ctx: &BuildCtx) {
    let wrapper = commands
        .spawn((
            Name::new("SpeedControls"),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                bottom: Val::Px(6.0),
                width: Val::Px(SPEED_PANEL_WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                align_items: AlignItems::Stretch,
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    for (index, key) in SPEED_KEYS.iter().enumerate() {
        commands.spawn((
            toggle_button(
                ctx.theme,
                &format!("speed:{key}"),
                SPEED_CAPTIONS[index],
                index == 2,
            ),
            SpeedButton(index),
            Tooltip::new(speed_tip(key)),
            ChildOf(wrapper),
        ));
    }
}

fn apply(session: &mut BattleSession, index: usize) {
    if index == 0 {
        session.paused = !session.paused;
    } else {
        session.paused = false;
        session.speed = SPEEDS[index - 1];
    }
}

fn speed_input(
    mut pressed: MessageReader<ButtonPressed>,
    mut session: ResMut<BattleSession>,
    mut queue: ResMut<ModalQueue>,
) {
    for press in pressed.read() {
        if let Some(key) = press.name.strip_prefix("speed:")
            && let Some(index) = SPEED_KEYS.iter().position(|k| *k == key)
        {
            apply(&mut session, index);
        } else if press.name == "battle:quit" && !session.snapshot.finished {
            queue.request(
                QUIT_KEY,
                PRIORITY_DECISION,
                ModalSpec::confirm(
                    "Quitter la bataille ?",
                    "Le combat sera compté comme une [b]retraite[/b].",
                ),
            );
        }
    }
}

/// Pause and speed keys (`battle_pause`, `battle_speed_up`, `battle_speed_down`).
fn speed_keys(
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<crate::settings::KeyMap>,
    mut session: ResMut<BattleSession>,
) {
    if keymap.just_pressed("battle_pause", &keys) {
        session.paused = !session.paused;
    }
    if keymap.just_pressed("battle_speed_up", &keys) {
        session.paused = false;
        session.speed = step_speed(session.speed, 1);
    }
    if keymap.just_pressed("battle_speed_down", &keys) {
        session.paused = false;
        session.speed = step_speed(session.speed, -1);
    }
}

fn quit_flow(mut results: MessageReader<ModalResult>, mut leave: MessageWriter<LeaveBattle>) {
    for result in results.read() {
        if result.key == QUIT_KEY && result.confirmed {
            leave.write(LeaveBattle { abandoned: true });
        }
    }
}

fn update_top_bar(
    data: Res<HudData>,
    mut texts: Query<(&TopText, &mut Text)>,
    mut fill: Query<&mut Node, With<BalanceOwnFill>>,
) {
    if !data.is_changed() {
        return;
    }
    for (field, mut text) in &mut texts {
        let value = match field {
            TopText::Clock => clock_text(data.elapsed_s),
            TopText::TimeLeft => time_left_text(data.time_left_s),
            TopText::Balance => format!(
                "{} {} · {} {}",
                data.faction_names[0],
                super::view::number(f64::from(data.strength[0])),
                data.faction_names[1],
                super::view::number(f64::from(data.strength[1])),
            ),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    if let Ok(mut node) = fill.single_mut() {
        node.width = Val::Percent(balance_share(data.strength[0], data.strength[1]) * 100.0);
    }
}

fn sync_speed_buttons(
    session: Res<BattleSession>,
    mut buttons: Query<(&SpeedButton, &mut Toggled)>,
) {
    let active = active_index(session.paused, session.speed);
    for (button, mut toggled) in &mut buttons {
        let on = button.0 == active;
        if toggled.0 != on {
            toggled.0 = on;
        }
    }
}

pub fn dump(world: &World) -> Value {
    let Some(session) = world.get_resource::<BattleSession>() else {
        return json!({ "active": false });
    };
    json!({
        "paused": session.paused,
        "speed": session.speed,
        "active_button": SPEED_KEYS[active_index(session.paused, session.speed)],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_and_time_left() {
        assert_eq!(clock_text(125.9), "02:05");
        assert_eq!(time_left_text(721.2), "Nuit dans 12:02");
        assert_eq!(time_left_text(-3.0), "Nuit dans 00:00");
    }

    #[test]
    fn active_button_follows_pause_then_speed() {
        assert_eq!(active_index(true, 2.0), 0);
        assert_eq!(active_index(false, 0.5), 1);
        assert_eq!(active_index(false, 1.0), 2);
        assert_eq!(active_index(false, 4.0), 4);
    }

    #[test]
    fn speed_steps_are_clamped() {
        assert_eq!(step_speed(1.0, 1), 2.0);
        assert_eq!(step_speed(4.0, 1), 4.0);
        assert_eq!(step_speed(0.5, -1), 0.5);
    }

    #[test]
    fn balance_share_handles_empty_sides() {
        assert_eq!(balance_share(0, 0), 0.0);
        assert_eq!(balance_share(30, 10), 0.75);
    }
}
