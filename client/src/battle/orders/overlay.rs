//! What the player sees of their orders on the field: selection outlines, the rectangle of a box
//! selection, the path preview while the right button is held, the paths and attack arrows of
//! the orders already given, the range arcs of selected or hovered shooters, and the context
//! cursor.
//!
//! Ports game/scripts/battle/battle_path_preview.gd (live preview and given orders),
//! battle_range_arc.gd and battle_cursor.gd as Bevy gizmos (draw-through ribbons) and a UI node.
//! Paths come from the core (`BattleSim::preview_group_width`, the same computation as the real
//! order); nothing here decides a rule.

use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use sim_battle::{Battlefield, HoverRules, SideId};

use super::BattleSelection;
use super::drag::{self, DRAG_PX};
use super::input::{BattleViews, Context, Gesture};
use super::picking::UnitView;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCamera, BattleCameraState};
use crate::ui::kit::style::UiTier;

/// Gizmo group of the orders layer (drawn over the terrain and the soldiers).
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct OrdersGizmos;

/// Lift above the ground (m).
const LIFT: f32 = 0.45;
/// Spacing of the samples of a line draped on the relief (m).
const SAMPLE_M: f32 = 4.0;
/// Camera closer than this (m): paths and arrows of given orders hidden (as in Godot).
const ORDERS_NEAR_HIDE_M: f32 = 30.0;
/// Arcs of range drawn at most for this many shooters.
const MAX_ARCS: usize = 8;
/// Live preview recomputed at most every this many seconds, and when the target moved this much.
const PREVIEW_INTERVAL_S: f32 = 0.1;
const PREVIEW_MOVE_M: f32 = 5.0;
/// Paths drawn for at most this many regiments (the core gives one path from the group centre
/// above its own threshold).
const RED: Color = Color::srgba(0.9, 0.12, 0.08, 0.95);

/// Result of the live preview of the right button.
#[derive(Resource, Debug, Default)]
pub struct LivePreview {
    /// One polyline per regiment (or one for the group), `None` when the core refused.
    pub paths: Vec<Option<Vec<Vec3>>>,
    pub reachable: bool,
    pub active: bool,
    last_point: Vec2,
    last_facing: Option<f32>,
    last_width: f32,
    last_time: f32,
    pub computed: u32,
}

/// Counters of the layer, for the dump.
#[derive(Resource, Debug, Default)]
pub struct OverlayStats {
    pub outlines: usize,
    pub order_paths: usize,
    pub arrows: usize,
    pub arcs: usize,
    pub arc_units: Vec<u32>,
}

#[derive(Component)]
pub struct DragRectNode;

pub fn spawn_drag_rect(mut commands: Commands) {
    commands.spawn((
        Name::new("BattleDragRect"),
        DragRectNode,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            border: UiRect::all(Val::Px(1.5)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.95, 0.85, 0.4, 0.10)),
        BorderColor::all(Color::srgba(0.95, 0.85, 0.4, 0.9)),
        // Under the HUD cards, over the world.
        GlobalZIndex(UiTier::Hud.z() - 1),
        Pickable::IGNORE,
        DespawnOnExit(crate::app::AppState::Battle),
    ));
}

pub fn update_drag_rect(gesture: Res<Gesture>, mut nodes: Query<&mut Node, With<DragRectNode>>) {
    let Ok(mut node) = nodes.single_mut() else {
        return;
    };
    match gesture.drag_rect {
        Some((min, max)) => {
            node.display = Display::Flex;
            node.left = Val::Px(min.x);
            node.top = Val::Px(min.y);
            node.width = Val::Px(max.x - min.x);
            node.height = Val::Px(max.y - min.y);
        }
        None => node.display = Display::None,
    }
}

/// Context cursor: crosshair over an enemy with a selection, pointer over an own regiment.
pub fn update_cursor(
    gesture: Res<Gesture>,
    mut commands: Commands,
    windows: Query<Entity, With<PrimaryWindow>>,
) {
    if !gesture.is_changed() {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let icon = match gesture.context {
        Context::Attack => SystemCursorIcon::Crosshair,
        Context::Own => SystemCursorIcon::Pointer,
        Context::Ground => SystemCursorIcon::Default,
    };
    commands.entity(window).insert(CursorIcon::System(icon));
}

pub fn reset_cursor(mut commands: Commands, windows: Query<Entity, With<PrimaryWindow>>) {
    if let Ok(window) = windows.single() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Default));
    }
}

// ----- drawing helpers ------------------------------------------------------------------------

/// Points from `a` to `b` (x, z) draped on the walkable height of the field.
pub fn ground_line(field: &Battlefield, a: Vec2, b: Vec2, lift: f32) -> Vec<Vec3> {
    let steps = ((a.distance(b) / SAMPLE_M).ceil() as usize).max(1);
    (0..=steps)
        .map(|i| {
            let p = a.lerp(b, i as f32 / steps as f32);
            Vec3::new(
                p.x,
                field.walk_height(f64::from(p.x), f64::from(p.y)) as f32 + lift,
                p.y,
            )
        })
        .collect()
}

fn side_color(session: &BattleSession, side: SideId) -> Color {
    let rgb = session.colors[usize::from(side == SideId::Defender)];
    let base = Color::srgb(rgb[0], rgb[1], rgb[2]);
    base.mix(&Color::WHITE, 0.3)
}

fn outline(gizmos: &mut Gizmos<OrdersGizmos>, field: &Battlefield, unit: &UnitView, color: Color) {
    let half_w = (unit.width * 0.5).max(2.0);
    let half_d = (unit.depth * 0.5).max(2.0);
    let right = Vec2::new(unit.facing.cos(), -unit.facing.sin());
    let forward = Vec2::new(unit.facing.sin(), unit.facing.cos());
    let centre = Vec2::new(unit.pos.x, unit.pos.z);
    let corner = |sr: f32, sf: f32| centre + right * (half_w * sr) + forward * (half_d * sf);
    let corners = [
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ];
    for i in 0..4 {
        gizmos.linestrip(
            ground_line(field, corners[i], corners[(i + 1) % 4], LIFT),
            color,
        );
    }
    // Front mark: a short tick forward from the middle of the front edge.
    let front = centre + forward * half_d;
    gizmos.linestrip(
        ground_line(field, front, front + forward * 3.0, LIFT),
        color,
    );
}

fn arrow_head(
    gizmos: &mut Gizmos<OrdersGizmos>,
    field: &Battlefield,
    from: Vec2,
    to: Vec2,
    color: Color,
) {
    let dir = (to - from).normalize_or_zero();
    if dir == Vec2::ZERO {
        return;
    }
    let side = Vec2::new(-dir.y, dir.x);
    for wing in [side, -side] {
        let tip = to - dir * 5.0 + wing * 3.0;
        gizmos.linestrip(ground_line(field, to, tip, LIFT), color);
    }
}

/// Draws the whole layer (one system so it shares one borrow of the simulation).
#[allow(clippy::too_many_arguments)]
pub fn draw_overlay(
    mut gizmos: Gizmos<OrdersGizmos>,
    mut session: ResMut<BattleSession>,
    selection: Res<BattleSelection>,
    views: Res<BattleViews>,
    gesture: Res<Gesture>,
    time: Res<Time<Real>>,
    mut live: ResMut<LivePreview>,
    mut stats: ResMut<OverlayStats>,
    camera_state: Option<Res<BattleCameraState>>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
) {
    *stats = OverlayStats::default();
    let Some(player) = session.player else { return };
    let color = side_color(&session, player);
    let enemy_color = side_color(&session, player.other());
    let near = camera_state
        .as_ref()
        .is_some_and(|c| c.distance < ORDERS_NEAR_HIDE_M);
    let eye = cameras
        .single()
        .map(|(_, t)| t.translation())
        .unwrap_or_default();
    let cursor_ground = cameras.single().ok().and_then(|(camera, transform)| {
        let ray = camera.viewport_to_world(transform, gesture.cursor).ok()?;
        Some(super::picking::ground_point(
            ray.origin,
            *ray.direction,
            session.sim_mut().field(),
        ))
    });
    let now = time.elapsed_secs();
    let sim = session.sim_mut();

    // Selection and hover outlines.
    for unit in views
        .0
        .iter()
        .filter(|u| selection.units.contains(&u.id) && u.present)
    {
        outline(&mut gizmos, sim.field(), unit, color);
        stats.outlines += 1;
    }
    for id in [gesture.hover_own, gesture.hover_enemy]
        .into_iter()
        .flatten()
    {
        if let Some(unit) = views
            .0
            .iter()
            .find(|u| u.id == id && u.present && !selection.units.contains(&id))
        {
            let hover = if unit.side == player {
                color.with_alpha(0.55)
            } else {
                enemy_color.with_alpha(0.7)
            };
            outline(&mut gizmos, sim.field(), unit, hover);
        }
    }

    // Orders already given: path to the destination, arrow to the target.
    if !near {
        for unit in sim
            .units()
            .iter()
            .filter(|u| selection.units.contains(&u.id) && u.present())
        {
            let from = Vec2::new(unit.x as f32, unit.z as f32);
            if let Some((x, z)) = unit.destination {
                let to = Vec2::new(x as f32, z as f32);
                gizmos.linestrip(
                    ground_line(sim.field(), from, to, LIFT),
                    color.with_alpha(0.8),
                );
                arrow_head(&mut gizmos, sim.field(), from, to, color.with_alpha(0.8));
                stats.order_paths += 1;
            }
            if let Some(target) = unit.target.and_then(|id| sim.units().get(id as usize))
                && target.present()
            {
                let to = Vec2::new(target.x as f32, target.z as f32);
                gizmos.linestrip(ground_line(sim.field(), from, to, LIFT), RED);
                arrow_head(&mut gizmos, sim.field(), from, to, RED);
                stats.arrows += 1;
            }
        }
    }

    // Range arcs: selected then hovered shooters.
    let half = HoverRules::bundled().range_arc.fire_half_angle() as f32;
    let mut shown: Vec<u32> = Vec::new();
    let hovered = [gesture.hover_own, gesture.hover_enemy]
        .into_iter()
        .flatten();
    if !near {
        for id in selection.units.iter().copied().chain(hovered) {
            if shown.len() >= MAX_ARCS || shown.contains(&id) {
                continue;
            }
            let Some(unit) = sim.units().get(id as usize).filter(|u| u.present()) else {
                continue;
            };
            let range = sim.ground_range(unit) as f32;
            if range <= 0.0 {
                continue;
            }
            shown.push(id);
            let selected = selection.units.contains(&id);
            let arc_color = if unit.side == player {
                color
            } else {
                enemy_color
            }
            .with_alpha(if selected { 0.85 } else { 0.5 });
            let centre = Vec2::new(unit.x as f32, unit.z as f32);
            let facing = unit.facing as f32;
            let half = half.clamp(0.01, std::f32::consts::PI);
            let segments = ((2.0 * half * range / 4.0).ceil() as usize).clamp(8, 160);
            let at = |angle: f32, r: f32| centre + Vec2::new(angle.sin(), angle.cos()) * r;
            let field = sim.field();
            let points: Vec<Vec3> = (0..=segments)
                .map(|s| {
                    let angle = facing - half + 2.0 * half * s as f32 / segments as f32;
                    let p = at(angle, range);
                    Vec3::new(
                        p.x,
                        field.walk_height(f64::from(p.x), f64::from(p.y)) as f32 + LIFT,
                        p.y,
                    )
                })
                .collect();
            gizmos.linestrip(points, arc_color);
            if half < std::f32::consts::PI - 0.01 {
                for sign in [-1.0, 1.0] {
                    let angle = facing + sign * half;
                    gizmos.linestrip(
                        ground_line(field, at(angle, 0.0), at(angle, range), LIFT),
                        arc_color.with_alpha(arc_color.alpha() * 0.45),
                    );
                }
            }
        }
    }
    stats.arcs = shown.len();
    stats.arc_units = shown;

    // Live preview of a right-drag / held right button.
    let enemy_hovered = gesture.hover_enemy.is_some();
    if let Some(press) = gesture.right_press
        && !selection.units.is_empty()
        && !enemy_hovered
        && !sim.is_deploying()
    {
        let ids: Vec<u32> = selection
            .units
            .iter()
            .copied()
            .filter(|id| views.0.iter().any(|u| u.id == *id && u.available()))
            .collect();
        let target = {
            cursor_ground.map(|ground| {
                if (press.pos - gesture.cursor).length() > DRAG_PX
                    && let Some(line) = drag::drag_line(press.ground, ground, eye)
                {
                    (
                        Vec2::new(line.mid.x, line.mid.z),
                        Some(line.facing),
                        line.width,
                    )
                } else {
                    (Vec2::new(ground.x, ground.z), None, 0.0)
                }
            })
        };
        if let Some((point, facing, width)) = target
            && !ids.is_empty()
        {
            let moved = point.distance(live.last_point) > PREVIEW_MOVE_M
                || facing.is_some() != live.last_facing.is_some()
                || (width - live.last_width).abs() > 1.0;
            if !live.active || (moved && now - live.last_time >= PREVIEW_INTERVAL_S) {
                let legs = sim.preview_group_width(
                    &ids,
                    f64::from(point.x),
                    f64::from(point.y),
                    facing.map(f64::from),
                    (width > 0.0).then_some(f64::from(width)),
                    gesture.shift,
                );
                live.paths = legs
                    .iter()
                    .map(|leg| {
                        leg.path.as_ref().ok().map(|path| {
                            let mut points = Vec::new();
                            let mut from = Vec2::new(leg.from.0 as f32, leg.from.1 as f32);
                            for &(x, z) in path {
                                let to = Vec2::new(x as f32, z as f32);
                                points.extend(ground_line(sim.field(), from, to, LIFT));
                                from = to;
                            }
                            points
                        })
                    })
                    .collect();
                live.reachable = live.paths.iter().any(Option::is_some);
                live.last_point = point;
                live.last_facing = facing;
                live.last_width = width;
                live.last_time = now;
                live.active = true;
                live.computed += 1;
            }
            let colour = if live.reachable { color } else { RED };
            for path in live.paths.iter().flatten() {
                gizmos.linestrip(path.iter().copied(), colour);
            }
            if !live.reachable {
                // Refused: a red cross where the order would end.
                let h = sim
                    .field()
                    .walk_height(f64::from(point.x), f64::from(point.y))
                    as f32
                    + LIFT;
                let c = Vec3::new(point.x, h, point.y);
                gizmos.line(
                    c + Vec3::new(-2.5, 0.0, -2.5),
                    c + Vec3::new(2.5, 0.0, 2.5),
                    RED,
                );
                gizmos.line(
                    c + Vec3::new(-2.5, 0.0, 2.5),
                    c + Vec3::new(2.5, 0.0, -2.5),
                    RED,
                );
            }
        }
    } else {
        live.active = false;
        live.paths.clear();
    }
}
