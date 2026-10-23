//! Keyboard and gamepad focus of the kit (ADR 0753): a visible focus ring on one kit button
//! (or slider) at a time, moved by Tab / Shift+Tab (reading order) and the arrows (spatial),
//! activated by Enter / Space, left by Escape (handled by the screens as before).
//!
//! Scope: with a modal, a central window or a front-end menu, only the topmost layer takes the
//! focus (by `GlobalZIndex`), so a modal never lets the focus leave it. Over the bare campaign /
//! battle HUD every visible button is one scope (the HUD is made of many layers: order bar,
//! unit cards, formation menu, launchers; ADR 0775). The focus keys are always live in the
//! front-end menus, under a modal and with a central window open; over the bare HUD they are
//! live only once the player engages the focus with **F9** (ADR 0775: F8 is the developer
//! console; Tab keeps its shortcut `campaign_next_idle` there, the arrows keep panning the
//! camera). The mouse (click) or Escape gives them back.
//!
//! The first focus lands on the safe default action of the screen: a button carrying
//! [`FocusDefault`] (the confirming button of a modal) before the first in reading order.
//!
//! Gamepad: D-pad or left stick = arrows, A (south) = Enter, B (east) = Escape, any D-pad press
//! or stick push engages. Not tested on hardware (the pure logic is).
//! Sliders: Left / Right change the focused slider by 5 % of its range.
//!
//! The pure geometry ([`pick_direction`], [`reading_order`], [`next_in_order`]) is tested; the
//! systems only gather what is on screen.

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::ui_widgets::{Activate, Button, SetSliderValue, Slider, SliderRange, SliderValueChange};
use serde_json::{Value, json};

use super::layout_probe::rect;
use super::style;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;

/// Key that engages / releases the focus over the bare HUD (F8 is the developer console).
pub const ENGAGE_KEY: KeyCode = KeyCode::F9;
/// Left stick deflection that counts as a direction press.
const STICK_THRESHOLD: f32 = 0.6;

/// Marks the button the keyboard focus lands on first (the safe, confirming action of a dialog).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FocusDefault;
/// Rows closer than this (px) count as one line in reading order.
const ROW_TOLERANCE: f32 = 10.0;

/// A direction of spatial navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

type Rect = [f32; 4];

fn center(r: Rect) -> Vec2 {
    Vec2::new((r[0] + r[2]) * 0.5, (r[1] + r[3]) * 0.5)
}

/// Best neighbour of `from` in `dir` among `others`: the nearest along the direction, penalising
/// the sideways offset (so a column of buttons is walked in order and a row too). `None` at the
/// edge.
pub fn pick_direction(from: Rect, dir: Dir, others: &[(Entity, Rect)]) -> Option<Entity> {
    let origin = center(from);
    let mut best: Option<(f32, Entity)> = None;
    for (entity, r) in others {
        let c = center(*r);
        let d = c - origin;
        let (along, side) = match dir {
            Dir::Right => (d.x, d.y.abs()),
            Dir::Left => (-d.x, d.y.abs()),
            Dir::Down => (d.y, d.x.abs()),
            Dir::Up => (-d.y, d.x.abs()),
        };
        // Must be ahead, and more ahead than aside (a 45° cone, a little forgiving).
        if along <= 1.0 || side > along * 1.6 + 24.0 {
            continue;
        }
        let score = along + side * 2.0;
        if best.is_none_or(|(s, _)| score < s) {
            best = Some((score, *entity));
        }
    }
    best.map(|(_, e)| e)
}

/// Entities sorted in reading order (rows top to bottom, left to right inside a row).
pub fn reading_order(items: &[(Entity, Rect)]) -> Vec<Entity> {
    let mut sorted: Vec<(Entity, Vec2)> = items.iter().map(|(e, r)| (*e, center(*r))).collect();
    sorted.sort_by(|a, b| {
        if (a.1.y - b.1.y).abs() > ROW_TOLERANCE {
            a.1.y.total_cmp(&b.1.y)
        } else {
            a.1.x.total_cmp(&b.1.x)
        }
    });
    sorted.into_iter().map(|(e, _)| e).collect()
}

/// The entity after (or before) `current` in `order`, wrapping; the first (last) when `current`
/// is not in the list.
pub fn next_in_order(order: &[Entity], current: Option<Entity>, backwards: bool) -> Option<Entity> {
    if order.is_empty() {
        return None;
    }
    let position = current.and_then(|c| order.iter().position(|e| *e == c));
    let index = match (position, backwards) {
        (Some(i), false) => (i + 1) % order.len(),
        (Some(i), true) => (i + order.len() - 1) % order.len(),
        (None, false) => 0,
        (None, true) => order.len() - 1,
    };
    Some(order[index])
}

/// Direction of a left-stick deflection (`None` inside the dead zone).
pub fn stick_direction(stick: Vec2) -> Option<Dir> {
    if stick.length() < STICK_THRESHOLD {
        return None;
    }
    if stick.x.abs() >= stick.y.abs() {
        Some(if stick.x > 0.0 { Dir::Right } else { Dir::Left })
    } else {
        Some(if stick.y > 0.0 { Dir::Up } else { Dir::Down })
    }
}

/// The first focus of a screen: the first default (in reading order) among `order`, else the
/// first (the last when going backwards).
pub fn initial_pick(order: &[Entity], defaults: &[Entity], backwards: bool) -> Option<Entity> {
    order
        .iter()
        .find(|e| defaults.contains(e))
        .copied()
        .or_else(|| next_in_order(order, None, backwards))
}

/// State of the focus.
#[derive(Resource, Default, Debug)]
pub struct UiFocus {
    pub current: Option<Entity>,
    /// The focus keys are live over the bare HUD (F9).
    pub engaged: bool,
    /// The ring is drawn (hidden again by a mouse click).
    pub visible: bool,
    /// Moves done and activations sent (dump).
    pub moves: u32,
    pub activations: u32,
    ring_on: Option<Entity>,
    stick: Option<Dir>,
    scope_z: i32,
    candidates: usize,
    live: bool,
}

pub struct FocusPlugin;

impl Plugin for FocusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiFocus>()
            .register_dump(dump)
            .add_systems(PreUpdate, navigate.after(bevy::input::InputSystems))
            .add_systems(PostUpdate, draw_ring.before(bevy::ui::UiSystems::Layout));
    }
}

/// One screen element the focus may land on.
struct Candidate {
    entity: Entity,
    rect: Rect,
    z: i32,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn gather(
    nodes: &Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        (
            Or<(With<Button>, With<Slider>)>,
            Without<InteractionDisabled>,
        ),
    >,
    parents: &Query<&ChildOf>,
    zs: &Query<&GlobalZIndex>,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    for (entity, node, transform, visible) in nodes {
        if node.size().x < 1.0 || node.size().y < 1.0 || visible.is_some_and(|v| !v.get()) {
            continue;
        }
        // z of the layer: the highest `GlobalZIndex` among the node and its ancestors.
        let mut z = zs.get(entity).map_or(i32::MIN, |z| z.0);
        let mut at = entity;
        while let Ok(parent) = parents.get(at) {
            at = parent.parent();
            if let Ok(zi) = zs.get(at) {
                z = z.max(zi.0);
            }
        }
        out.push(Candidate {
            entity,
            rect: rect(node, transform),
            z: if z == i32::MIN { 0 } else { z },
        });
    }
    out
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn navigate(
    mut commands: Commands,
    mut focus: ResMut<UiFocus>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut key_events: MessageReader<KeyboardInput>,
    mut mouse_events: MessageReader<MouseButtonInput>,
    gamepads: Query<&Gamepad>,
    nodes: Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&InheritedVisibility>,
        ),
        (
            Or<(With<Button>, With<Slider>)>,
            Without<InteractionDisabled>,
        ),
    >,
    parents: Query<&ChildOf>,
    zs: Query<&GlobalZIndex>,
    sliders: Query<(&SliderRange, Has<Slider>)>,
    defaults: Query<(), With<FocusDefault>>,
    state: Option<Res<State<AppState>>>,
    stack: Option<Res<super::panel_stack::PanelStack>>,
    modals: Option<Res<super::modal::ModalQueue>>,
    text: Option<Res<super::text_field::TextInputFocus>>,
) {
    // A click gives the screen back to the mouse (the ring hides; over the HUD the keys too).
    let clicked = mouse_events.read().any(|e| e.state == ButtonState::Pressed);
    let engage_pressed = key_events
        .read()
        .any(|e| e.state == ButtonState::Pressed && !e.repeat && e.key_code == ENGAGE_KEY);
    if clicked {
        focus.visible = false;
        focus.engaged = false;
    }
    if text.is_some_and(|t| t.0.is_some()) {
        return; // typing: every key belongs to the field
    }

    let pad = |button: GamepadButton| gamepads.iter().any(|g| g.just_pressed(button));
    let mut dir: Option<Dir> = None;
    let mut tab: Option<bool> = None; // Some(backwards)
    let mut activate = false;
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if keys.just_pressed(KeyCode::ArrowUp) || pad(GamepadButton::DPadUp) {
        dir = Some(Dir::Up);
    } else if keys.just_pressed(KeyCode::ArrowDown) || pad(GamepadButton::DPadDown) {
        dir = Some(Dir::Down);
    } else if keys.just_pressed(KeyCode::ArrowLeft) || pad(GamepadButton::DPadLeft) {
        dir = Some(Dir::Left);
    } else if keys.just_pressed(KeyCode::ArrowRight) || pad(GamepadButton::DPadRight) {
        dir = Some(Dir::Right);
    }
    let stick = gamepads
        .iter()
        .find_map(|g| stick_direction(g.left_stick()));
    let stick_edge = stick.filter(|s| Some(*s) != focus.stick);
    focus.stick = stick;
    let mut from_stick = false;
    if dir.is_none() && stick_edge.is_some() {
        dir = stick_edge;
        from_stick = true;
    }
    if keys.just_pressed(KeyCode::Tab) {
        tab = Some(shift);
    }
    if keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || keys.just_pressed(KeyCode::Space)
        || pad(GamepadButton::South)
    {
        activate = true;
    }
    if pad(GamepadButton::East) {
        keys.press(KeyCode::Escape); // B = Escape for every screen
    }
    let pad_nav = from_stick
        || dir.is_some()
            && gamepads.iter().any(|g| {
                g.get_just_pressed().any(|b| {
                    matches!(
                        b,
                        GamepadButton::DPadUp
                            | GamepadButton::DPadDown
                            | GamepadButton::DPadLeft
                            | GamepadButton::DPadRight
                    )
                })
            });
    if engage_pressed {
        focus.engaged = !focus.engaged;
        focus.visible = focus.engaged;
        if !focus.engaged {
            focus.current = None;
        }
    }
    let nothing = dir.is_none() && tab.is_none() && !activate && !engage_pressed;
    if nothing && !focus.engaged && !focus.visible {
        focus.live = false;
        return;
    }

    let candidates = gather(&nodes, &parents, &zs);
    let top = candidates.iter().map(|c| c.z).max();
    let Some(top) = top else {
        focus.current = None;
        focus.candidates = 0;
        return;
    };
    // Live without F9: front-end menus, a modal, a central window.
    let in_game = state
        .as_ref()
        .is_some_and(|s| matches!(s.get(), AppState::Campaign | AppState::Battle));
    let always = !in_game
        || modals.is_some_and(|m| m.is_busy())
        || stack.is_some_and(|s| s.has_modal_open() || s.has_central_open());
    // A modal / window / menu keeps the focus on its own layer; the bare HUD is one scope.
    let layer: Vec<(Entity, Rect)> = candidates
        .iter()
        .filter(|c| !always || c.z == top)
        .map(|c| (c.entity, c.rect))
        .collect();
    focus.scope_z = top;
    focus.candidates = layer.len();
    let default_buttons: Vec<Entity> = layer
        .iter()
        .map(|(e, _)| *e)
        .filter(|e| defaults.contains(*e))
        .collect();
    focus.live = always || focus.engaged || pad_nav;
    if pad_nav {
        focus.engaged = true;
    }
    if !focus.live {
        return;
    }
    // A key that is ours must not also pan the camera or trigger a shortcut this frame.
    let consume = |keys: &mut ButtonInput<KeyCode>, list: &[KeyCode]| {
        for key in list {
            keys.reset(*key);
        }
    };

    // The focus left the top layer (a modal opened over it): restart there.
    if focus
        .current
        .is_some_and(|c| !layer.iter().any(|(e, _)| *e == c))
    {
        focus.current = None;
    }
    let order = reading_order(&layer);
    if let Some(backwards) = tab {
        focus.current = if focus.current.is_none() {
            initial_pick(&order, &default_buttons, backwards)
        } else {
            next_in_order(&order, focus.current, backwards)
        };
        focus.visible = true;
        focus.moves += 1;
        consume(&mut keys, &[KeyCode::Tab]);
    } else if let Some(direction) = dir {
        let slider_step = match (focus.current, direction) {
            (Some(c), Dir::Left | Dir::Right) => sliders
                .get(c)
                .ok()
                .filter(|(_, is_slider)| *is_slider)
                .map(|(range, _)| {
                    let span = (range.end() - range.start()).abs();
                    if direction == Dir::Right {
                        span * 0.05
                    } else {
                        -span * 0.05
                    }
                }),
            _ => None,
        };
        if let (Some(c), Some(step)) = (focus.current, slider_step) {
            commands.trigger(SetSliderValue {
                entity: c,
                change: SliderValueChange::Relative(step),
            });
        } else {
            let from = focus
                .current
                .and_then(|c| layer.iter().find(|(e, _)| *e == c))
                .map(|(_, r)| *r);
            let next = match from {
                Some(from) => {
                    let others: Vec<_> = layer
                        .iter()
                        .copied()
                        .filter(|(e, _)| Some(*e) != focus.current)
                        .collect();
                    pick_direction(from, direction, &others).or(focus.current)
                }
                None => initial_pick(&order, &default_buttons, false),
            };
            if next != focus.current {
                focus.moves += 1;
            }
            focus.current = next;
        }
        focus.visible = true;
        consume(
            &mut keys,
            &[
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
            ],
        );
    } else if activate && focus.visible {
        if let Some(current) = focus.current {
            commands.trigger(Activate { entity: current });
            focus.activations += 1;
            consume(
                &mut keys,
                &[KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space],
            );
        }
    } else if engage_pressed && focus.current.is_none() && focus.engaged {
        focus.current = initial_pick(&order, &default_buttons, false);
    }
    if keys.just_pressed(KeyCode::Escape) && focus.engaged && !always {
        // Escape over the bare HUD gives the keys back (the pause menu still opens).
        focus.engaged = false;
        focus.visible = false;
        focus.current = None;
    }
}

/// Draws the focus ring as an `Outline` on the focused node.
fn draw_ring(mut commands: Commands, mut focus: ResMut<UiFocus>, nodes: Query<(), With<Node>>) {
    let want = if focus.visible && focus.live {
        focus.current.filter(|e| nodes.contains(*e))
    } else {
        None
    };
    if focus.ring_on == want {
        return;
    }
    if let Some(old) = focus.ring_on
        && let Ok(mut entity) = commands.get_entity(old)
    {
        entity.remove::<Outline>();
    }
    if let Some(new) = want {
        commands.entity(new).insert(Outline {
            width: Val::Px(3.0),
            offset: Val::Px(1.0),
            color: style::GOLD_PALE,
        });
    }
    focus.ring_on = want;
}

fn dump(world: &World) -> (String, Value) {
    let focus = world.resource::<UiFocus>();
    let name = focus
        .current
        .and_then(|e| world.get::<Name>(e))
        .map(|n| n.to_string());
    (
        "ui_focus".to_owned(),
        json!({
            "current": name,
            "engage_key": format!("{ENGAGE_KEY:?}"),
            "engaged": focus.engaged,
            "visible": focus.visible,
            "live": focus.live,
            "scope_z": focus.scope_z,
            "candidates": focus.candidates,
            "moves": focus.moves,
            "activations": focus.activations,
            "ring": focus.ring_on.is_some(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(i: u32) -> Entity {
        Entity::from_raw_u32(i).expect("entity")
    }

    fn column() -> Vec<(Entity, Rect)> {
        (0..4)
            .map(|i| {
                (
                    e(i + 1),
                    [10.0, 10.0 + 40.0 * i as f32, 110.0, 40.0 + 40.0 * i as f32],
                )
            })
            .collect()
    }

    #[test]
    fn arrows_walk_a_column_in_order() {
        let items = column();
        let down = pick_direction(items[0].1, Dir::Down, &items[1..]);
        assert_eq!(down, Some(e(2)));
        assert_eq!(pick_direction(items[3].1, Dir::Down, &items[..3]), None);
        assert_eq!(
            pick_direction(items[2].1, Dir::Up, &[items[0], items[1], items[3]]),
            Some(e(2))
        );
    }

    #[test]
    fn arrows_prefer_the_aligned_neighbour() {
        let from = [100.0, 100.0, 200.0, 130.0];
        let aligned = (e(1), [300.0, 100.0, 400.0, 130.0]);
        let lower = (e(2), [210.0, 150.0, 300.0, 180.0]);
        assert_eq!(
            pick_direction(from, Dir::Right, &[lower, aligned]),
            Some(e(1))
        );
        assert_eq!(pick_direction(from, Dir::Left, &[lower, aligned]), None);
    }

    #[test]
    fn stick_has_a_dead_zone_and_picks_the_dominant_axis() {
        assert_eq!(stick_direction(Vec2::new(0.2, 0.3)), None);
        assert_eq!(stick_direction(Vec2::new(0.9, 0.3)), Some(Dir::Right));
        assert_eq!(stick_direction(Vec2::new(-0.8, 0.1)), Some(Dir::Left));
        assert_eq!(stick_direction(Vec2::new(0.1, 0.9)), Some(Dir::Up));
        assert_eq!(stick_direction(Vec2::new(0.1, -0.9)), Some(Dir::Down));
    }

    #[test]
    fn first_focus_prefers_the_default_action() {
        let order = vec![e(1), e(2), e(3)];
        assert_eq!(initial_pick(&order, &[e(3)], false), Some(e(3)));
        assert_eq!(initial_pick(&order, &[], false), Some(e(1)));
        assert_eq!(initial_pick(&order, &[], true), Some(e(3)));
        assert_eq!(initial_pick(&[], &[e(3)], false), None);
    }

    #[test]
    fn reading_order_groups_rows_and_tab_wraps() {
        let items = vec![
            (e(3), [200.0, 14.0, 260.0, 40.0]),
            (e(1), [10.0, 10.0, 70.0, 40.0]),
            (e(2), [10.0, 100.0, 70.0, 130.0]),
        ];
        let order = reading_order(&items);
        assert_eq!(order, vec![e(1), e(3), e(2)]);
        assert_eq!(next_in_order(&order, Some(e(2)), false), Some(e(1)));
        assert_eq!(next_in_order(&order, Some(e(1)), true), Some(e(2)));
        assert_eq!(next_in_order(&order, None, false), Some(e(1)));
        assert_eq!(next_in_order(&[], None, false), None);
    }
}
