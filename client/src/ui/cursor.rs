//! Painted mouse cursors (`BattleCursor` / `CampaignCursor`).
//!
//! Godot swaps the system arrow for 32 px painted PNGs with `Input.set_custom_mouse_cursor`
//! (hotspot at the centre). Bevy 0.20 does the same with [`CustomCursor::Image`] on the window's
//! [`CursorIcon`]. This module owns the file names and hotspot ([`custom_cursor`]) and the battle
//! cursor system; the campaign map keeps its own mode logic in `campaign/armies/feedback.rs` and
//! uses [`custom_cursor`] too.
//!
//! Battle: the core says what a right click would do (`BattleSim::hover_context`: move, melee,
//! ranged, ranged_blocked, siege, forbidden, none) and the image is
//! `assets/ui/cursors/<context>.png`; `none` gives the system arrow. As in
//! `battle_scene.gd::_update_hover_cursor`, a live path preview that reaches nothing, or a queued
//! order refused because the queue is full (Shift held), shows `forbidden`. A replay, the
//! general's speech, and any visible piece of interface give the arrow.
//! Differences from Godot: the context is recomputed every frame the pointer or the selection
//! changes (cached by a 2 m cell, the target regiment and the selection, like Godot's `_hover_key`).

use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::window::{CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow, SystemCursorIcon};
use serde_json::{Value, json};
use sim_battle::{HoverKind, QueueRules};

use crate::battle::orders::BattleSelection;
use crate::battle::orders::input::Gesture;
use crate::battle::orders::overlay::LivePreview;
use crate::battle::orders::picking;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;

/// Folder of the six battle cursors, relative to `game/assets`.
pub const BATTLE_DIR: &str = "ui/cursors";
/// Hotspot of the 32 px cursors (`BattleCursor.HOTSPOT`, `CampaignCursor.HOTSPOT`).
pub const HOTSPOT: (u16, u16) = (16, 16);
/// Battle cursor contexts that have an image (`BattleCursor.CONTEXTS`).
pub const BATTLE_CONTEXTS: [&str; 6] = [
    "move",
    "melee",
    "ranged",
    "ranged_blocked",
    "siege",
    "forbidden",
];

/// Window cursor showing `dir/name.png` with the centre as hotspot.
pub fn custom_cursor(assets: &AssetServer, dir: &str, name: &str) -> CursorIcon {
    CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
        handle: assets.load(format!("{dir}/{name}.png")),
        hotspot: HOTSPOT,
        ..default()
    }))
}

/// Context to show: the core's `kind`, overridden by `forbidden` when the live preview reaches
/// nothing or the order queue is full. `None` is the system arrow.
pub fn battle_context(
    kind: HoverKind,
    preview_unreachable: bool,
    queue_full: bool,
) -> Option<&'static str> {
    let kind = if preview_unreachable || queue_full {
        HoverKind::Forbidden
    } else {
        kind
    };
    BATTLE_CONTEXTS.into_iter().find(|name| *name == kind.key())
}

/// Battle cursor in use, for the dump and to apply a change only once.
#[derive(Resource, Debug, Default)]
pub struct BattleCursorState {
    /// Context shown (`None`: system arrow).
    pub shown: Option<&'static str>,
    /// Number of cursor changes since the battle started.
    pub changes: u32,
    /// Calls of `hover_context` (the cache key avoids most).
    pub hover_calls: u32,
    key: String,
    last: Option<HoverKind>,
}

type UiNodes<'w, 's> =
    Query<'w, 's, (Option<&'static BackgroundColor>, Option<&'static ImageNode>), With<Node>>;

fn over_ui(hover: Option<&HoverMap>, nodes: &UiNodes) -> bool {
    let Some(hover) = hover else { return false };
    hover.values().flat_map(|hits| hits.keys()).any(|entity| {
        nodes.get(*entity).is_ok_and(|(background, image)| {
            image.is_some() || background.is_some_and(|b| b.0.alpha() > 0.05)
        })
    })
}

/// Sets the battle cursor from the core's hover context (`_update_hover_cursor`).
#[allow(clippy::too_many_arguments)]
pub fn update_battle_cursor(
    mut commands: Commands,
    assets: Res<AssetServer>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<BattleCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
    gesture: Res<Gesture>,
    selection: Res<BattleSelection>,
    live: Res<LivePreview>,
    hover_map: Option<Res<HoverMap>>,
    nodes: UiNodes,
    speech: Option<Res<crate::battle::orders::speech::Speech>>,
    mut session: ResMut<BattleSession>,
    mut state: ResMut<BattleCursorState>,
) {
    let Ok((window_entity, window)) = windows.single() else {
        return;
    };
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let pointer = window.cursor_position();
    let blocked = session.replaying
        || speech.is_some_and(|s| s.active())
        || over_ui(hover_map.as_deref(), &nodes);
    if let (Some(pointer), false, Some(side)) = (pointer, blocked, session.player)
        && let Ok((camera, transform)) = cameras.single()
        && let Ok(ray) = camera.viewport_to_world(transform, pointer)
    {
        let target = gesture.hover_enemy.or(gesture.hover_own);
        let sim = session.sim_mut();
        // Over a regiment the question is asked at its centre (Godot passes the unit position).
        let (x, z) = match target.and_then(|id| sim.unit(id)) {
            Some(unit) => (unit.x, unit.z),
            None => {
                let point = picking::ground_point(ray.origin, *ray.direction, sim.field());
                (f64::from(point.x), f64::from(point.z))
            }
        };
        let key = format!(
            "{},{},{:?},{:?}",
            (x / 2.0).floor(),
            (z / 2.0).floor(),
            target,
            selection.units
        );
        if key != state.key || state.last.is_none() {
            state.key = key;
            state.hover_calls += 1;
            state.last = Some(sim.hover_context(x, z, &selection.units, side).context);
        }
        let kind = state.last.unwrap_or(HoverKind::None);
        let max = QueueRules::bundled().max_queued_orders as usize;
        let queue_full = shift
            && !selection.units.is_empty()
            && selection
                .units
                .iter()
                .filter_map(|id| sim.unit(*id))
                .any(|u| u.order_queue.len() >= max);
        let wanted = battle_context(kind, live.active && !live.reachable, queue_full);
        apply(&mut commands, &assets, window_entity, &mut state, wanted);
        return;
    }
    state.key.clear();
    state.last = None;
    apply(&mut commands, &assets, window_entity, &mut state, None);
}

fn apply(
    commands: &mut Commands,
    assets: &AssetServer,
    window: Entity,
    state: &mut BattleCursorState,
    wanted: Option<&'static str>,
) {
    if state.shown == wanted && state.changes > 0 {
        return;
    }
    state.shown = wanted;
    state.changes += 1;
    let icon = match wanted {
        Some(name) => custom_cursor(assets, BATTLE_DIR, name),
        None => CursorIcon::System(SystemCursorIcon::Default),
    };
    commands.entity(window).insert(icon);
}

/// Back to the system arrow when the battle screen closes (`BattleCursor.reset`).
pub fn reset_battle_cursor(
    mut commands: Commands,
    mut state: ResMut<BattleCursorState>,
    windows: Query<Entity, With<PrimaryWindow>>,
) {
    *state = BattleCursorState::default();
    if let Ok(window) = windows.single() {
        commands
            .entity(window)
            .insert(CursorIcon::System(SystemCursorIcon::Default));
    }
}

/// Dump section `battle_cursor`.
pub fn dump(world: &World) -> (String, Value) {
    let Some(state) = world.get_resource::<BattleCursorState>() else {
        return ("battle_cursor".into(), json!({ "active": false }));
    };
    let images_present = crate::paths::find_root().map_or(0, |root| {
        let dir = crate::paths::asset_root(&root).join(BATTLE_DIR);
        BATTLE_CONTEXTS
            .iter()
            .filter(|name| dir.join(format!("{name}.png")).is_file())
            .count()
    });
    (
        "battle_cursor".into(),
        json!({
            "shown": state.shown,
            "changes": state.changes,
            "hover_calls": state.hover_calls,
            "images_present": images_present,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_context_has_an_image_and_none_is_the_arrow() {
        for kind in [
            HoverKind::Move,
            HoverKind::Melee,
            HoverKind::Ranged,
            HoverKind::RangedBlocked,
            HoverKind::Siege,
            HoverKind::Forbidden,
        ] {
            assert_eq!(battle_context(kind, false, false), Some(kind.key()));
        }
        assert_eq!(battle_context(HoverKind::None, false, false), None);
    }

    #[test]
    fn unreachable_or_full_queue_is_forbidden() {
        assert_eq!(
            battle_context(HoverKind::Move, true, false),
            Some("forbidden")
        );
        assert_eq!(
            battle_context(HoverKind::Melee, false, true),
            Some("forbidden")
        );
    }
}
