//! Selection, hover, movement orders, path preview, reachable area and order ripples.
//! Port of `army_movement_controller.gd`, `army_movement_path.gd`, `army_movement_bubble.gd`,
//! `reachable_markers.gd`, `order_ripple.gd`. Orders are submitted to `CampaignState` exactly like
//! `godot-bridge` `campaign_sim_movement.rs::run_order_json`.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};
use sim_campaign::{ArmyId, MoveOrderTarget, Order, OrderOutcome};

use super::markers::{ArmyMarker, ArmyPlate, camera_distance, screen_rect};
use super::path::{PathView, preview_from_plan};
use super::pick::{Pickable, pick_scored};
use super::projection::{HeightSampler, px_to_world, ray_ground};
use super::views::ArmyViews;
use super::{HoveredArmy, SelectedArmy};
use crate::sim::Sim;

const PATH_COLOR: Color = Color::srgba(0.88, 0.57, 0.26, 0.9);
const RIPPLE_SECONDS: f32 = 0.8;
/// Cap on the cells drawn for the reachable area.
const MAX_BUBBLE_CELLS: usize = 4000;

/// Intent of the player (mouse, keys, script, tests).
#[derive(Message, Debug, Clone, PartialEq)]
pub enum ArmyCommand {
    Select(Option<ArmyId>),
    /// March to a map pixel.
    Move {
        army: ArmyId,
        target: [f32; 2],
    },
    /// Close in on an enemy army and fight.
    Attack {
        army: ArmyId,
        target: ArmyId,
    },
}

/// Result of the last order, as `run_order_json` reports it.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct OrderLog {
    pub issued: u32,
    pub accepted: u32,
    pub last: Option<OrderResult>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderResult {
    pub kind: &'static str,
    pub army: String,
    pub ok: bool,
    pub error: String,
    pub stop: String,
    pub cost: u32,
    pub walked: usize,
    pub planned_cells: usize,
    pub position: [f32; 2],
    pub movement_left: u32,
}

#[derive(Resource, Default)]
pub struct PathPreviewState {
    pub view: Option<PathView>,
    target: Option<[i32; 2]>,
}

#[derive(Resource, Default)]
pub struct Reachable {
    pub army: Option<ArmyId>,
    pub cells: Vec<Vec3>,
    pub count: usize,
    pub next_count: usize,
    pub budget: u32,
}

#[derive(Resource, Default)]
pub struct Ripples(Vec<(Vec3, f32)>);

pub fn reset(
    mut selected: ResMut<SelectedArmy>,
    mut hovered: ResMut<HoveredArmy>,
    mut preview: ResMut<PathPreviewState>,
    mut reachable: ResMut<Reachable>,
) {
    selected.0 = None;
    hovered.0 = None;
    *preview = PathPreviewState::default();
    *reachable = Reachable::default();
}

fn cursor_position(window: &Query<&Window, With<PrimaryWindow>>) -> Option<Vec2> {
    window.iter().next().and_then(|w| w.cursor_position())
}

/// Hover + mouse commands: left click selects, right click orders a march or an attack.
#[allow(clippy::too_many_arguments)]
pub fn hover_and_input(
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    markers: Query<(&ArmyMarker, &Transform)>,
    plates: Query<(&ArmyPlate, &ComputedNode, &UiGlobalTransform)>,
    views: Res<ArmyViews>,
    ground: Res<HeightSampler>,
    mouse: Res<ButtonInput<MouseButton>>,
    selected: Res<SelectedArmy>,
    mut hovered: ResMut<HoveredArmy>,
    mut commands: MessageWriter<ArmyCommand>,
) {
    let Some(cursor) = cursor_position(&window) else {
        return;
    };
    let Some((camera, camera_transform)) = camera.iter().next() else {
        return;
    };
    let items: Vec<Pickable> = markers
        .iter()
        .map(|(marker, transform)| Pickable {
            id: marker.id.as_str().to_owned(),
            rect: screen_rect(camera, camera_transform, transform),
            plate: plates
                .iter()
                .find(|(p, _, _)| p.id == marker.id)
                .map(|(_, node, transform)| {
                    let scale = node.inverse_scale_factor();
                    let center = transform.translation * scale;
                    Rect::from_center_size(center, node.size() * scale)
                }),
        })
        .collect();
    let under = pick_scored(&items, cursor, None).filter(|r| r.direct);
    let id = under.as_ref().and_then(|r| ArmyId::parse(&r.id));
    if hovered.0 != id {
        hovered.0 = id.clone();
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Some(id) = id.clone()
    {
        commands.write(ArmyCommand::Select(Some(id)));
    }
    if mouse.just_pressed(MouseButton::Right) {
        let Some(army) = selected.0.clone() else {
            return;
        };
        let target_army = pick_scored(&items, cursor, Some(army.as_str()))
            .filter(|r| r.direct)
            .and_then(|r| ArmyId::parse(&r.id))
            .filter(|t| views.get(t).is_some_and(|v| v.hostile));
        if let Some(target) = target_army {
            commands.write(ArmyCommand::Attack { army, target });
        } else if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor)
            && let Some(px) = ray_ground(ray, &ground)
        {
            commands.write(ArmyCommand::Move {
                army,
                target: [px.x, px.y],
            });
        }
    }
}

/// Script/test hook (no rule): `Tab` selects the next player army, `KeyM` orders the selected
/// army to march by the offset in `CENT_ANS_ARMY_MOVE` (`dx,dy` map pixels; default tries a few).
/// `CENT_ANS_ARMY_AUTOTEST=1` does both once, a few frames after the armies appear.
pub fn test_hook(
    keys: Res<ButtonInput<KeyCode>>,
    views: Res<ArmyViews>,
    selected: Res<SelectedArmy>,
    mut commands: MessageWriter<ArmyCommand>,
    mut frames: Local<u32>,
    mut auto_done: Local<bool>,
) {
    let mut cycle = keys.just_pressed(KeyCode::Tab);
    let mut march = keys.just_pressed(KeyCode::KeyM);
    if !*auto_done && std::env::var_os("CENT_ANS_ARMY_AUTOTEST").is_some() && !views.0.is_empty() {
        *frames += 1;
        if *frames == 5 {
            cycle = true;
        }
        if *frames == 10 {
            march = true;
            *auto_done = true;
        }
    }
    if cycle {
        let players: Vec<&ArmyId> = views.0.iter().filter(|v| v.player).map(|v| &v.id).collect();
        let next = match &selected.0 {
            Some(cur) => players
                .iter()
                .position(|id| *id == cur)
                .map_or(0, |i| (i + 1) % players.len().max(1)),
            None => 0,
        };
        if let Some(id) = players.get(next) {
            commands.write(ArmyCommand::Select(Some((*id).clone())));
        }
    }
    if march && let Some(view) = selected.0.as_ref().and_then(|id| views.get(id)) {
        for offset in test_offsets() {
            commands.write(ArmyCommand::Move {
                army: view.id.clone(),
                target: [view.point[0] + offset[0], view.point[1] + offset[1]],
            });
        }
    }
}

fn test_offsets() -> Vec<[f32; 2]> {
    if let Ok(text) = std::env::var("CENT_ANS_ARMY_MOVE")
        && let Some((dx, dy)) = text.split_once(',')
        && let (Ok(dx), Ok(dy)) = (dx.trim().parse(), dy.trim().parse())
    {
        return vec![[dx, dy]];
    }
    vec![[0.0, -12.0]]
}

pub fn execute_commands(
    mut messages: MessageReader<ArmyCommand>,
    mut sim: ResMut<Sim>,
    mut selected: ResMut<SelectedArmy>,
    mut log: ResMut<OrderLog>,
    mut ripples: ResMut<Ripples>,
    mut preview: ResMut<PathPreviewState>,
    ground: Res<HeightSampler>,
) {
    for command in messages.read() {
        match command {
            ArmyCommand::Select(id) => {
                if selected.0 != *id {
                    selected.0 = id.clone();
                    preview.view = None;
                    preview.target = None;
                }
            }
            ArmyCommand::Move { army, target } => {
                let order = Order::MoveArmy {
                    army: army.clone(),
                    target: MoveOrderTarget::Point {
                        x: target[0],
                        y: target[1],
                    },
                };
                run_order(&mut sim, &mut log, "move", army, order);
                if log.last.as_ref().is_some_and(|r| r.ok) {
                    ripples
                        .0
                        .push((px_to_world(Vec2::new(target[0], target[1]), &ground), 0.0));
                    preview.view = None;
                    preview.target = None;
                }
            }
            ArmyCommand::Attack { army, target } => {
                let order = Order::Attack {
                    army: army.clone(),
                    target_army: target.clone(),
                };
                run_order(&mut sim, &mut log, "attack", army, order);
            }
        }
    }
}

/// Mirrors `run_order_json` (godot-bridge): validate, apply, report the march.
fn run_order(sim: &mut Sim, log: &mut OrderLog, kind: &'static str, army: &ArmyId, order: Order) {
    log.issued += 1;
    let fail = |error: String| OrderResult {
        kind,
        army: army.as_str().to_owned(),
        ok: false,
        error,
        stop: String::new(),
        cost: 0,
        walked: 0,
        planned_cells: 0,
        position: [0.0; 2],
        movement_left: 0,
    };
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.as_mut()) else {
        log.last = Some(fail("aucune campagne en cours".into()));
        return;
    };
    let result = match state.submit_order_outcome(&data, order) {
        Err(error) => fail(error.to_string()),
        Ok(outcome) => {
            let entry = state.army(army);
            let position = entry.map_or([0.0; 2], |a| state.army_point(&data, a));
            let movement_left = entry.map_or(0, |a| a.movement_left);
            let mut result = OrderResult {
                ok: true,
                position,
                movement_left,
                ..fail(String::new())
            };
            if let OrderOutcome::Moved(report) = outcome {
                result.cost = report.cost;
                result.walked = report.walked.len();
                result.planned_cells = report.planned_path.len();
                result.stop = serde_json::to_value(&report.stop)
                    .ok()
                    .and_then(|v| v.get("kind").and_then(Value::as_str).map(str::to_owned))
                    .unwrap_or_default();
            }
            result
        }
    };
    if result.ok {
        log.accepted += 1;
    }
    log.last = Some(result);
}

/// Reachable area of the selected army, recomputed when the selection or the sim changes.
pub fn refresh_overlays(
    sim: Res<Sim>,
    selected: Res<SelectedArmy>,
    ground: Res<HeightSampler>,
    mut reachable: ResMut<Reachable>,
) {
    if !(selected.is_changed() || sim.is_changed()) {
        return;
    }
    let (Some(data), Some(state), Some(army)) = (&sim.data, &sim.state, &selected.0) else {
        if reachable.army.is_some() {
            *reachable = Reachable::default();
        }
        return;
    };
    if state.army(army).is_none() {
        *reachable = Reachable::default();
        return;
    }
    let area = state.reachable_cells(data, army);
    let grid = data.navgrid();
    reachable.army = Some(army.clone());
    reachable.count = area.this_turn.len();
    reachable.next_count = area.next_turn.len();
    reachable.budget = area.budget;
    let stride = (area.this_turn.len() / MAX_BUBBLE_CELLS).max(1);
    reachable.cells = area
        .this_turn
        .iter()
        .step_by(stride)
        .map(|(cell, _)| {
            let c = cell.center(grid);
            px_to_world(Vec2::new(c[0], c[1]), &ground)
        })
        .collect();
}

/// Path preview of the selected army towards the cursor ground point (throttled by cell).
pub fn preview_under_cursor(
    sim: Res<Sim>,
    selected: Res<SelectedArmy>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ground: Res<HeightSampler>,
    mut preview: ResMut<PathPreviewState>,
) {
    let (Some(data), Some(state), Some(army)) = (&sim.data, &sim.state, &selected.0) else {
        return;
    };
    let Some(cursor) = cursor_position(&window) else {
        return;
    };
    let Some((camera, camera_transform)) = camera.iter().next() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else {
        return;
    };
    let Some(px) = ray_ground(ray, &ground) else {
        return;
    };
    let cell = [(px.x / 4.0) as i32, (px.y / 4.0) as i32];
    if preview.target == Some(cell) {
        return;
    }
    preview.target = Some(cell);
    let distance = camera_distance(camera_transform, Vec3::new(px.x, 0.0, px.y));
    preview.view = state
        .plan_path(data, army, [px.x, px.y])
        .and_then(|plan| preview_from_plan(&plan, &ground, distance));
}

#[allow(clippy::too_many_arguments)]
pub fn draw_overlays(
    mut gizmos: Gizmos,
    time: Res<Time>,
    preview: Res<PathPreviewState>,
    reachable: Res<Reachable>,
    mut ripples: ResMut<Ripples>,
    views: Res<ArmyViews>,
    selected: Res<SelectedArmy>,
    ground: Res<HeightSampler>,
) {
    if let Some(view) = &preview.view {
        gizmos.linestrip(view.points.iter().copied(), PATH_COLOR);
        for &end in view.turn_ends.iter().filter_map(|i| view.corners.get(*i)) {
            gizmos.sphere(Isometry3d::from_translation(end), 0.8, PATH_COLOR);
        }
    }
    let bubble = Color::srgba(0.35, 0.6, 0.9, 0.5);
    for cell in &reachable.cells {
        gizmos.sphere(
            Isometry3d::from_translation(*cell + Vec3::Y * 0.1),
            0.35,
            bubble,
        );
    }
    if let Some(view) = selected.0.as_ref().and_then(|id| views.get(id)) {
        let at = px_to_world(Vec2::new(view.point[0], view.point[1]), &ground) + Vec3::Y * 0.2;
        gizmos.circle(
            Isometry3d::new(at, Quat::from_rotation_arc(Vec3::Z, Vec3::Y)),
            3.0,
            Color::srgb(1.0, 0.9, 0.4),
        );
    }
    let dt = time.delta_secs();
    ripples.0.retain_mut(|(at, age)| {
        *age += dt;
        let t = *age / RIPPLE_SECONDS;
        if t >= 1.0 {
            return false;
        }
        gizmos.circle(
            Isometry3d::new(
                *at + Vec3::Y * 0.2,
                Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
            ),
            1.0 + 5.0 * t,
            Color::srgba(1.0, 0.85, 0.4, 1.0 - t),
        );
        true
    });
}

/// `--dump-state` section `campaign_armies`.
pub fn dump_armies(world: &World) -> (String, Value) {
    let views = world.resource::<ArmyViews>();
    let selected = world.resource::<SelectedArmy>();
    let log = world.resource::<OrderLog>();
    let preview = world.resource::<PathPreviewState>();
    let reachable = world.resource::<Reachable>();
    let markers = world
        .try_query::<&ArmyMarker>()
        .map_or(0, |mut q| q.iter(world).count());
    let last = log.last.as_ref().map(|r| {
        json!({
            "kind": r.kind, "army": r.army, "ok": r.ok, "error": r.error, "stop": r.stop,
            "cost": r.cost, "walked_points": r.walked, "planned_cells": r.planned_cells,
            "position": r.position, "movement_left": r.movement_left,
        })
    });
    (
        "campaign_armies".into(),
        json!({
            "armies_shown": views.0.len(),
            "player_armies": views.0.iter().filter(|v| v.player).count(),
            "hostile_armies": views.0.iter().filter(|v| v.hostile).count(),
            "marker_entities": markers,
            "selected": selected.0.as_ref().map(ArmyId::as_str),
            "orders_issued": log.issued,
            "orders_accepted": log.accepted,
            "last_order": last,
            "path_length": preview.view.as_ref().map_or(0, |v| v.points.len()),
            "path_turns": preview.view.as_ref().map_or(0, |v| v.turns),
            "reachable_cells": reachable.count,
            "reachable_next_cells": reachable.next_count,
        }),
    )
}
