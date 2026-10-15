//! Marks on the field and on the camera: numbered waypoints of queued orders, ghosts at the
//! arrival of the orders given, and camera bookmarks (Ctrl+F2..F4 saves, F2..F4 recalls).
//!
//! Ports the queue part of game/scripts/battle/battle_path_preview.gd (`_number`, `_ghost`) and
//! `use_bookmark` of battle_input.gd with `save_bookmark` / `recall_bookmark` of battle_camera.gd.
//! Pure presentation: the queue comes from the simulation (`Unit::order_queue`).

use bevy::prelude::*;
use sim_battle::{QueuedOrder, SideId};

use super::BattleSelection;
use super::overlay::{OrdersGizmos, arrow_head, ground_line, outline_rect};
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCamera, BattleCameraState};
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::style::UiTier;
use crate::ui::kit::UiTheme;

const LIFT: f32 = 0.5;
const RED: Color = Color::srgba(0.9, 0.12, 0.08, 0.95);
/// Waypoints drawn for at most this many regiments.
const MAX_UNITS: usize = 6;
/// Camera closer than this (m): marks hidden, as the paths of the orders.
const NEAR_HIDE_M: f32 = 30.0;
/// Duration of a bookmark glide (s).
const GLIDE_S: f32 = 0.6;

/// One numbered waypoint to show this frame (world point, number).
#[derive(Resource, Debug, Default)]
pub struct Waypoints {
    pub labels: Vec<(Vec3, u32)>,
    pub ghosts: usize,
    pub legs: usize,
}

#[derive(Component)]
pub struct WaypointLabel(pub usize);

/// Draws the legs of the queued orders, the arrival ghosts and fills [`Waypoints`].
pub fn draw_marks(
    mut gizmos: Gizmos<OrdersGizmos>,
    mut session: ResMut<BattleSession>,
    selection: Res<BattleSelection>,
    mut waypoints: ResMut<Waypoints>,
    camera_state: Option<Res<BattleCameraState>>,
) {
    waypoints.labels.clear();
    waypoints.ghosts = 0;
    waypoints.legs = 0;
    let Some(player) = session.player else { return };
    if camera_state.is_some_and(|c| c.distance < NEAR_HIDE_M) {
        return;
    }
    let rgb = session.colors[usize::from(player == SideId::Defender)];
    let color = Color::srgb(rgb[0], rgb[1], rgb[2]).mix(&Color::WHITE, 0.3);
    let sim = session.sim_mut();
    let field = sim.field();
    let at = |x: f64, z: f64| Vec3::new(x as f32, field.walk_height(x, z) as f32 + LIFT, z as f32);
    for unit in sim
        .units()
        .iter()
        .filter(|u| selection.units.contains(&u.id) && u.present() && !u.withdrawing)
        .take(MAX_UNITS)
    {
        let here = Vec2::new(unit.x as f32, unit.z as f32);
        let (width, depth) = unit.extent();
        let mut last = here;
        let mut end_facing = unit.destination_facing.map(|f| f as f32);
        if let Some(target) = unit.target.and_then(|id| sim.unit(id)).filter(|t| t.present()) {
            last = Vec2::new(target.x as f32, target.z as f32);
        } else if let Some((x, z)) = unit.destination {
            last = Vec2::new(x as f32, z as f32);
        }
        if unit.order_queue.is_empty() {
            // One order: a ghost at the arrival of a move.
            if unit.target.is_none()
                && let Some((x, z)) = unit.destination
            {
                let facing = end_facing.unwrap_or_else(|| {
                    let dir = last - here;
                    dir.x.atan2(dir.y)
                });
                outline_rect(
                    &mut gizmos,
                    field,
                    Vec2::new(x as f32, z as f32),
                    facing,
                    width as f32,
                    depth as f32,
                    color.with_alpha(0.5),
                );
                waypoints.ghosts += 1;
            }
            continue;
        }
        // Queued orders, numbered from the current one (1).
        let mut number = 1;
        if last != here {
            waypoints.labels.push((at(f64::from(last.x), f64::from(last.y)), 1));
            number = 2;
        }
        let mut last_was_move = false;
        for order in &unit.order_queue {
            let (to, red) = match order {
                QueuedOrder::Move { x, z, facing, .. } => {
                    end_facing = facing.map(|f| f as f32);
                    last_was_move = true;
                    (Vec2::new(*x as f32, *z as f32), false)
                }
                QueuedOrder::Attack { target, .. } => {
                    last_was_move = false;
                    let Some(foe) = sim.unit(*target).filter(|t| t.present()) else {
                        continue;
                    };
                    (Vec2::new(foe.x as f32, foe.z as f32), true)
                }
            };
            let tint = if red { RED } else { color.with_alpha(0.8) };
            gizmos.linestrip(ground_line(field, last, to, LIFT), tint);
            arrow_head(&mut gizmos, field, last, to, tint);
            waypoints.legs += 1;
            waypoints.labels.push((at(f64::from(to.x), f64::from(to.y)), number));
            number += 1;
            last = to;
        }
        if last_was_move {
            let facing = end_facing.unwrap_or(0.0);
            outline_rect(
                &mut gizmos,
                field,
                last,
                facing,
                width as f32,
                depth as f32,
                color.with_alpha(0.5),
            );
            waypoints.ghosts += 1;
        }
    }
}

/// Number labels (UI text projected on the screen), a small pool grown on demand.
pub fn update_labels(
    mut commands: Commands,
    waypoints: Res<Waypoints>,
    theme: Res<UiTheme>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
    mut labels: Query<(Entity, &WaypointLabel, &mut Node, &mut Text, &mut Visibility)>,
) {
    let Ok((camera, transform)) = cameras.single() else {
        return;
    };
    let have = labels.iter().count();
    for index in have..waypoints.labels.len() {
        commands.spawn((
            Name::new("WaypointLabel"),
            WaypointLabel(index),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Text::new(""),
            theme.text_font(true, false, 22.0),
            TextColor(Color::srgb(1.0, 0.95, 0.75)),
            TextShadow::default(),
            Visibility::Hidden,
            GlobalZIndex(UiTier::Hud.z() - 1),
            Pickable::IGNORE,
            DespawnOnExit(crate::app::AppState::Battle),
        ));
    }
    for (_, label, mut node, mut text, mut visibility) in &mut labels {
        let projected = waypoints
            .labels
            .get(label.0)
            .and_then(|(world, n)| Some((camera.world_to_viewport(transform, *world).ok()?, *n)));
        match projected {
            Some((pos, number)) => {
                node.left = Val::Px(pos.x - 7.0);
                node.top = Val::Px(pos.y - 14.0);
                let caption = number.to_string();
                if text.0 != caption {
                    text.0 = caption;
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

// ----- camera bookmarks --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark {
    pub target: Vec3,
    pub yaw: f32,
    pub distance: f32,
}

/// Bookmarks F2..F4 (index 0..2) and the glide in progress.
#[derive(Resource, Debug, Default)]
pub struct CameraBookmarks {
    pub marks: [Option<Mark>; 3],
    glide: Option<(Mark, Mark, f32)>,
}

fn shortest_yaw(from: f32, to: f32) -> f32 {
    let delta = (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    from + delta
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Slot (2..=4) of an F2..F4 key.
fn slot_of(key: KeyCode) -> Option<usize> {
    match key {
        KeyCode::F2 => Some(2),
        KeyCode::F3 => Some(3),
        KeyCode::F4 => Some(4),
        _ => None,
    }
}

pub fn bookmark_keys(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut bookmarks: ResMut<CameraBookmarks>,
    state: Option<ResMut<BattleCameraState>>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    let Some(mut state) = state else { return };
    let ctrl = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let other = keys.any_pressed([
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
    ]);
    if !other {
        for key in keys.get_just_pressed() {
            let Some(slot) = slot_of(*key) else { continue };
            if ctrl {
                bookmarks.marks[slot - 2] = Some(Mark {
                    target: state.target,
                    yaw: state.yaw,
                    distance: state.target_distance,
                });
                bookmarks.glide = None;
                toasts.write(ToastRequest::new(&format!(
                    "Signet de caméra F{slot} enregistré (Ctrl+F{slot})."
                )));
            } else if let Some(mark) = bookmarks.marks[slot - 2] {
                let from = Mark {
                    target: state.target,
                    yaw: state.yaw,
                    distance: state.target_distance,
                };
                bookmarks.glide = Some((from, mark, 0.0));
            } else {
                toasts.write(ToastRequest::new(&format!(
                    "Signet F{slot} vide : Ctrl+F{slot} l’enregistre."
                )));
            }
        }
    }
    if let Some((from, to, t)) = bookmarks.glide {
        let t = t + time.delta_secs() / GLIDE_S;
        let k = smooth(t);
        state.target = from.target.lerp(to.target, k);
        let end_yaw = shortest_yaw(from.yaw, to.yaw);
        state.yaw = from.yaw + (end_yaw - from.yaw) * k;
        state.target_distance = from.distance + (to.distance - from.distance) * k;
        bookmarks.glide = (t < 1.0).then_some((from, to, t));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_glides_the_short_way_round() {
        let to = 0.1_f32;
        let from = std::f32::consts::TAU - 0.1;
        let end = shortest_yaw(from, to);
        assert!((end - from - 0.2).abs() < 1e-4);
    }

    #[test]
    fn f_keys_map_to_slots_two_to_four() {
        assert_eq!(slot_of(KeyCode::F2), Some(2));
        assert_eq!(slot_of(KeyCode::F4), Some(4));
        assert_eq!(slot_of(KeyCode::F1), None);
    }
}
