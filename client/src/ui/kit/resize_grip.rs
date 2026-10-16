//! Window resize grip (port of `game/scripts/ui/window_resize_grip.gd`, Godot lot RG1).
//!
//! Every [`StackedPanel`] of tier `Central` or `Companion` gets a small triangle in its bottom
//! right corner. Dragging it grows or shrinks the panel (`width` / `height` of its `Node`),
//! bounded between the size it had when first shown (the "floor") and the screen edge, minus
//! [`SCREEN_MARGIN`]. The size chosen is remembered for the session ([`RememberedSizes`], by panel
//! id, nothing on disk) and flagged by [`ResizableWindow::user_sized`], so that layouts
//! ([`super::layout::FitZone`]) stop imposing their own `max_height`.
//!
//! The grip is re-created on its own when a panel rebuilds its children. Scripts and tests can
//! drive it without a mouse with the [`ResizeDrag`] message (`drag_by` in Godot). No game rule
//! here: layout only.

use std::collections::HashMap;
use std::f32::consts::{FRAC_1_SQRT_2, SQRT_2};

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, ComputedUiRenderTargetInfo, FocusPolicy, UiGlobalTransform};
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};

use super::panel_stack::{PanelTier, StackedPanel};
use super::style;
use crate::agent::dump::DumpAppExt;

/// Side of the grip (`GRIP_SIZE`).
pub const GRIP_SIZE: f32 = 18.0;
/// Margin left between the window and the screen edge (`SCREEN_MARGIN`).
pub const SCREEN_MARGIN: f32 = 4.0;

/// Resize state of a stacked panel (put on its root by [`attach_grips`]).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ResizableWindow {
    /// Smallest size (logical px): the panel's size when first laid out.
    pub floor: Option<Vec2>,
    /// The player chose a size.
    pub user_sized: bool,
}

/// Marker of the grip node (child of the panel root).
#[derive(Component)]
pub struct ResizeGrip;

/// Last size chosen for each panel id, for the session (`_remembered`).
#[derive(Resource, Default, Debug)]
pub struct RememberedSizes(pub HashMap<String, Vec2>);

impl RememberedSizes {
    /// `WindowResizeGrip.forget_all`.
    pub fn forget_all(&mut self) {
        self.0.clear();
    }
}

/// Grows (or shrinks, with negative values) the panel `id` by `delta` logical px, bounded.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct ResizeDrag {
    pub id: String,
    pub delta: Vec2,
}

/// Size the panel may take: `wanted` bounded between `floor` and the screen edge. `origin` is the
/// panel's top-left corner and `view` the window size (logical px). Pure, tested.
pub fn clamp_size(wanted: Vec2, floor: Vec2, origin: Vec2, view: Vec2) -> Vec2 {
    let ceiling = view - origin - Vec2::splat(SCREEN_MARGIN);
    Vec2::new(
        wanted.x.clamp(floor.x, floor.x.max(ceiling.x)),
        wanted.y.clamp(floor.y, floor.y.max(ceiling.y)),
    )
}

fn is_resizable(tier: PanelTier) -> bool {
    matches!(tier, PanelTier::Central | PanelTier::Companion)
}

/// Adds [`ResizableWindow`] and a grip to the central / companion panels, and again when a rebuild
/// of the panel's children removed the grip.
fn attach_grips(
    mut commands: Commands,
    panels: Query<(Entity, &StackedPanel, Option<&Children>), With<Node>>,
    grips: Query<(), With<ResizeGrip>>,
) {
    for (entity, panel, children) in &panels {
        if !is_resizable(panel.tier) {
            continue;
        }
        let has_grip = children.is_some_and(|kids| kids.iter().any(|kid| grips.contains(kid)));
        if has_grip {
            continue;
        }
        commands
            .entity(entity)
            .try_insert_if_new(ResizableWindow::default());
        commands.entity(entity).with_child(grip_bundle());
    }
}

fn grip_bundle() -> impl Bundle {
    (
        ResizeGrip,
        Name::new("ResizeGrip"),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            width: Val::Px(GRIP_SIZE),
            height: Val::Px(GRIP_SIZE),
            overflow: Overflow::clip(),
            ..default()
        },
        Hovered::default(),
        FocusPolicy::Block,
        GlobalZIndex(10),
        children![
            // A square turned 45 degrees and centred on the corner, clipped to the grip box:
            // the bottom-right triangle.
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(GRIP_SIZE * (1.0 - FRAC_1_SQRT_2)),
                    top: Val::Px(GRIP_SIZE * (1.0 - FRAC_1_SQRT_2)),
                    width: Val::Px(GRIP_SIZE * SQRT_2),
                    height: Val::Px(GRIP_SIZE * SQRT_2),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::degrees(45.0)),
                BackgroundColor(style::INK_SOFT.with_alpha(0.75)),
                FocusPolicy::Pass,
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(GRIP_SIZE * 0.45),
                    top: Val::Px(GRIP_SIZE * 0.45),
                    width: Val::Px(GRIP_SIZE * 0.78),
                    height: Val::Px(1.0),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::degrees(-45.0)),
                BackgroundColor(style::PARCHMENT_LIGHT.with_alpha(0.8)),
                FocusPolicy::Pass,
            ),
        ],
    )
}

/// Mouse drag on a grip: turns the cursor movement into [`ResizeDrag`]s.
fn drag_with_mouse(
    grips: Query<(&Hovered, &ChildOf), With<ResizeGrip>>,
    panels: Query<&StackedPanel>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut dragging: Local<Option<(String, Vec2)>>,
    mut drags: MessageWriter<ResizeDrag>,
) {
    let cursor = window.single().ok().and_then(Window::cursor_position);
    if !mouse.pressed(MouseButton::Left) {
        *dragging = None;
        return;
    }
    let Some(cursor) = cursor else {
        return;
    };
    if dragging.is_none() && mouse.just_pressed(MouseButton::Left) {
        for (hovered, parent) in &grips {
            if hovered.get()
                && let Ok(panel) = panels.get(parent.parent())
            {
                *dragging = Some((panel.id.clone(), cursor));
                break;
            }
        }
    }
    if let Some((id, last)) = dragging.as_mut() {
        let delta = (cursor - *last) / ui_scale.0.max(0.01);
        *last = cursor;
        if delta != Vec2::ZERO {
            drags.write(ResizeDrag {
                id: id.clone(),
                delta,
            });
        }
    }
}

/// Learns each panel's floor, restores the remembered size and applies [`ResizeDrag`]s.
fn apply_sizes(
    mut panels: Query<(
        &StackedPanel,
        &mut ResizableWindow,
        &mut Node,
        &ComputedNode,
        &UiGlobalTransform,
        &ComputedUiRenderTargetInfo,
    )>,
    mut remembered: ResMut<RememberedSizes>,
    mut drags: MessageReader<ResizeDrag>,
) {
    let mut pending: Vec<ResizeDrag> = drags.read().cloned().collect();
    for (panel, mut state, mut node, computed, transform, target) in &mut panels {
        let size = computed.size() * computed.inverse_scale_factor();
        if size.x < 1.0 || size.y < 1.0 {
            continue;
        }
        let view = target.logical_size();
        let origin = transform.translation * computed.inverse_scale_factor() - size * 0.5;
        if state.floor.is_none() {
            state.floor = Some(size);
            if let Some(&wanted) = remembered.0.get(&panel.id) {
                let floor = size;
                set_size(
                    &mut state,
                    &mut node,
                    clamp_size(wanted, floor, origin, view),
                );
            }
        }
        let floor = state.floor.unwrap_or(size);
        for drag in pending.iter().filter(|drag| drag.id == panel.id) {
            let current = match (node.width, node.height) {
                (Val::Px(w), Val::Px(h)) if state.user_sized => Vec2::new(w, h),
                _ => size,
            };
            let clamped = clamp_size(current + drag.delta, floor, origin, view);
            set_size(&mut state, &mut node, clamped);
            remembered.0.insert(panel.id.clone(), clamped);
        }
    }
    pending.clear();
}

fn set_size(state: &mut ResizableWindow, node: &mut Node, size: Vec2) {
    state.user_sized = true;
    node.width = Val::Px(size.x);
    node.height = Val::Px(size.y);
    node.max_height = Val::Auto;
}

fn dump_grips(world: &World) -> (String, Value) {
    let mut query = world.try_query::<(&StackedPanel, &ResizableWindow, &Node)>();
    let panels: Vec<Value> = query
        .as_mut()
        .map(|q| {
            let mut rows: Vec<_> = q
                .iter(world)
                .map(|(panel, state, node)| {
                    let px = |val: Val| match val {
                        Val::Px(v) => json!(v),
                        _ => Value::Null,
                    };
                    json!({
                        "id": panel.id,
                        "user_sized": state.user_sized,
                        "floor": state.floor.map(|f| [f.x, f.y]),
                        "width": px(node.width),
                        "height": px(node.height),
                    })
                })
                .collect();
            rows.sort_by_key(|row| row["id"].as_str().unwrap_or_default().to_owned());
            rows
        })
        .unwrap_or_default();
    let remembered = world
        .get_resource::<RememberedSizes>()
        .map_or(0, |r| r.0.len());
    (
        "resize_grips".to_owned(),
        json!({"remembered": remembered, "panels": panels}),
    )
}

pub fn plugin(app: &mut App) {
    app.init_resource::<RememberedSizes>()
        .add_message::<ResizeDrag>()
        .register_dump(dump_grips)
        .add_systems(Update, (attach_grips, drag_with_mouse, apply_sizes).chain());
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Vec2 = Vec2::new(1280.0, 720.0);

    #[test]
    fn clamp_keeps_floor_and_screen_margin() {
        let floor = Vec2::new(400.0, 300.0);
        let origin = Vec2::new(100.0, 50.0);
        assert_eq!(
            clamp_size(Vec2::new(10.0, 10.0), floor, origin, VIEW),
            floor
        );
        let big = clamp_size(Vec2::new(5000.0, 5000.0), floor, origin, VIEW);
        assert_eq!(big, Vec2::new(1280.0 - 100.0 - 4.0, 720.0 - 50.0 - 4.0));
        assert_eq!(
            clamp_size(Vec2::new(600.0, 400.0), floor, origin, VIEW),
            Vec2::new(600.0, 400.0)
        );
    }

    #[test]
    fn floor_wins_when_the_panel_already_overflows_the_screen() {
        let floor = Vec2::new(900.0, 800.0);
        let size = clamp_size(
            Vec2::new(1000.0, 1000.0),
            floor,
            Vec2::new(900.0, 100.0),
            VIEW,
        );
        assert_eq!(size, floor);
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<UiScale>()
            .init_resource::<RememberedSizes>()
            .add_message::<ResizeDrag>()
            .add_systems(Update, attach_grips);
        app
    }

    #[test]
    fn grip_is_attached_to_central_panels_only_and_rebuilt() {
        let mut app = app();
        let central = app
            .world_mut()
            .spawn((Node::default(), StackedPanel::new(PanelTier::Central, "a")))
            .id();
        let docked = app
            .world_mut()
            .spawn((Node::default(), StackedPanel::new(PanelTier::Docked, "b")))
            .id();
        app.update();
        let count = |app: &mut App, entity: Entity| {
            app.world()
                .get::<Children>(entity)
                .map_or(0, |kids| kids.len())
        };
        assert_eq!(count(&mut app, central), 1);
        assert_eq!(count(&mut app, docked), 0);
        assert!(app.world().get::<ResizableWindow>(central).is_some());
        // A rebuild removes the children: the grip comes back.
        app.world_mut().entity_mut(central).despawn_children();
        app.update();
        assert_eq!(count(&mut app, central), 1);
    }
}
