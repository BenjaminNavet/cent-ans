//! Selection and orders on the battlefield (Total War style controls).
//!
//! Ports game/scripts/battle/battle_input.gd (clicks, box, drag, hotkeys, groups),
//! battle_cursor.gd, formation_drag.gd, battle_path_preview.gd, battle_range_arc.gd,
//! battle_groups.gd and battle_hotkeys.gd. Owned by a single lot of agent (ADR 0357); other lots
//! do not edit this folder.
//!
//! Shared interface (fixed before wave 3): [`BattleSelection`] (read by the battle HUD, written
//! here) and [`SelectBattleUnits`] (sent by the HUD unit cards, applied here). Added: the
//! messages [`input::IssueBattleCommand`] (any `sim_battle::Command` from the HUD: ability bar,
//! formation menu, withdraw) and [`input::MoveSelectionTo`] (right click on the minimap), the
//! resource [`groups::BattleGroups`] (group numbers for the card badges, locked groups) and
//! [`input::OrderLog`] (orders sent and the core's answers).
//!
//! Picking is CPU-only on the positions of the simulation ([`picking`]); orders are built in
//! [`drag`] and sent to `BattleSim::issue_command` as `godot-bridge` does.
//!
//! Dev hook (agents): `CENT_ANS_ORDERS_DEMO=1` plays a scripted sequence through the real pointer
//! code (box select, right click move, right drag line, attack) once the battle runs; dump
//! section `battle_orders`.

pub mod cinematic;
pub mod deploy;
pub mod drag;
pub mod formations;
pub mod groups;
pub mod hotkeys;
pub mod input;
pub mod marks;
pub mod overlay;
pub mod picking;
pub mod proof;
pub mod select;
pub mod speech;

use bevy::gizmos::config::{GizmoConfig, GizmoLineConfig};
use bevy::prelude::*;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::{BattleSession, StepSet};

/// Player's selected regiments, by `sim_battle::Unit::id`, in selection order.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct BattleSelection {
    pub units: Vec<u32>,
}

/// Request to change the selection (unit cards of the HUD, hotkeys).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SelectBattleUnits {
    pub units: Vec<u32>,
    /// Add to the current selection instead of replacing it (Shift/Ctrl).
    pub additive: bool,
}

/// System set of the input handling (the HUD may order itself relative to it).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrdersSet;

pub struct BattleOrdersPlugin;

impl Plugin for BattleOrdersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BattleSelection>()
            .add_message::<SelectBattleUnits>()
            .add_message::<input::PointerEvent>()
            .add_message::<input::IssueBattleCommand>()
            .add_message::<input::MoveSelectionTo>()
            .init_resource::<groups::BattleGroups>()
            .init_resource::<input::Gesture>()
            .init_resource::<input::OrderLog>()
            .init_resource::<input::BattleViews>()
            .init_resource::<input::SimOrders>()
            .init_resource::<demo::Demo>()
            .init_resource::<overlay::LivePreview>()
            .init_resource::<overlay::OverlayStats>()
            .init_resource::<deploy::Deployment>()
            .init_resource::<formations::GroupFormations>()
            .init_resource::<marks::Waypoints>()
            .init_resource::<marks::CameraBookmarks>()
            .init_resource::<cinematic::Cinematic>()
            .init_resource::<speech::Speech>()
            .init_resource::<proof::BdeployDemo>()
            .insert_gizmo_config(
                overlay::OrdersGizmos,
                GizmoConfig {
                    // Over the grass and the figures, like the ribbons of the Godot preview.
                    depth_bias: -1.0,
                    line: GizmoLineConfig {
                        width: 2.5,
                        ..default()
                    },
                    ..default()
                },
            )
            .add_systems(
                PostUpdate,
                cinematic::drive_camera
                    .before(bevy::transform::TransformSystems::Propagate)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .register_dump(dump_orders)
            .register_dump(speech::dump)
            .add_systems(
                OnEnter(AppState::Battle),
                (
                    reset_state,
                    overlay::spawn_drag_rect,
                    formations::spawn_picker,
                ),
            )
            .add_systems(
                OnExit(AppState::Battle),
                (reset_state, overlay::reset_cursor),
            )
            .add_systems(
                Update,
                (
                    input::refresh_views,
                    input::read_pointer,
                    input::apply_inputs,
                    input::snapshot_sim_orders,
                    demo::run_demo,
                )
                    .chain()
                    .in_set(OrdersSet)
                    .after(StepSet)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(
                Update,
                (
                    deploy::refresh_deployment,
                    proof::run_bdeploy_demo,
                    deploy::sync_banner,
                    deploy::banner_buttons,
                    formations::picker_buttons,
                    formations::sync_picker,
                    marks::bookmark_keys,
                    cinematic::run_cinematic.before(crate::menu::pause::pause_escape),
                    speech::run_speech.before(crate::menu::pause::pause_escape),
                )
                    .chain()
                    .before(OrdersSet)
                    .after(StepSet)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(
                Update,
                (
                    overlay::draw_overlay,
                    overlay::update_drag_rect,
                    overlay::update_cursor,
                    deploy::draw_deployment,
                    formations::draw_ghosts,
                    marks::draw_marks,
                    marks::update_labels,
                    speech::sync_subtitles,
                )
                    .after(OrdersSet)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn reset_state(
    mut selection: ResMut<BattleSelection>,
    mut groups: ResMut<groups::BattleGroups>,
    mut gesture: ResMut<input::Gesture>,
    mut log: ResMut<input::OrderLog>,
    mut live: ResMut<overlay::LivePreview>,
    mut demo: ResMut<demo::Demo>,
    mut deployment: ResMut<deploy::Deployment>,
    mut formations: ResMut<formations::GroupFormations>,
    mut bookmarks: ResMut<marks::CameraBookmarks>,
    mut cinematic: ResMut<cinematic::Cinematic>,
    mut speech: ResMut<speech::Speech>,
    mut waypoints: ResMut<marks::Waypoints>,
) {
    waypoints.routes.clear();
    *deployment = deploy::Deployment::default();
    *formations = formations::GroupFormations::default();
    *bookmarks = marks::CameraBookmarks::default();
    *cinematic = cinematic::Cinematic::default();
    *speech = speech::Speech::default();
    *selection = BattleSelection::default();
    *groups = groups::BattleGroups::default();
    *gesture = input::Gesture::default();
    *log = input::OrderLog::default();
    *live = overlay::LivePreview::default();
    *demo = demo::Demo::default();
}

mod demo;

fn dump_orders(world: &World) -> (String, serde_json::Value) {
    use serde_json::json;
    let (Some(selection), Some(log)) = (
        world.get_resource::<BattleSelection>(),
        world.get_resource::<input::OrderLog>(),
    ) else {
        return ("battle_orders".into(), json!({ "active": false }));
    };
    if world.get_resource::<BattleSession>().is_none() {
        return ("battle_orders".into(), json!({ "active": false }));
    }
    let groups = world.resource::<groups::BattleGroups>();
    let gesture = world.resource::<input::Gesture>();
    let stats = world.resource::<overlay::OverlayStats>();
    let live = world.resource::<overlay::LivePreview>();
    let views = world.resource::<input::BattleViews>();
    let unit_state = |id: &u32| {
        views.0.iter().find(|u| u.id == *id).map(|u| {
            json!({ "id": u.id, "type": u.unit_type, "x": u.pos.x, "z": u.pos.z, "facing": u.facing })
        })
    };
    (
        "battle_orders".into(),
        json!({
            "active": true,
            "selection": selection.units,
            "selected": selection.units.iter().filter_map(unit_state).collect::<Vec<_>>(),
            "groups": groups.groups,
            "locks": groups.locks.iter().map(|(tag, g)| json!({ "tag": tag, "ids": g.ids })).collect::<Vec<_>>(),
            "orders": { "accepted": log.accepted, "refused": log.refused },
            "recent": log.recent.iter().map(|o| json!({ "tick": o.tick, "command": o.command, "result": o.result })).collect::<Vec<_>>(),
            "sim_orders": world.get_resource::<input::SimOrders>().map(|s| s.0.clone()),
            "hover": { "own": gesture.hover_own, "enemy": gesture.hover_enemy, "context": format!("{:?}", gesture.context) },
            "overlay": {
                "outlines": stats.outlines, "order_paths": stats.order_paths, "arrows": stats.arrows,
                "arcs": stats.arcs, "arc_units": stats.arc_units,
                "live_active": live.active, "live_computed": live.computed,
            },
            "demo": world.resource::<demo::Demo>().report(),
            "deployment": {
                "opened": world.resource::<deploy::Deployment>().opened,
                "active": world.resource::<deploy::Deployment>().active,
                "zones": world.resource::<deploy::Deployment>().zones,
            },
            "formations": {
                "presets": sim_battle::group_formation::presets().iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
                "active": world.resource::<formations::GroupFormations>().active,
                "pending": world.resource::<formations::GroupFormations>().pending.len(),
                "orders": world.resource::<formations::GroupFormations>().orders,
                "collapsed": world.resource::<formations::GroupFormations>().collapsed,
            },
            "bdeploy_demo": world.resource::<proof::BdeployDemo>().report(),
            "cinematic": {
                "active": world.resource::<cinematic::Cinematic>().active,
                "done": world.resource::<cinematic::Cinematic>().done,
            },
            "marks": {
                "labels": world.resource::<marks::Waypoints>().labels.iter().map(|(_, n)| *n).collect::<Vec<_>>(),
                "legs": world.resource::<marks::Waypoints>().legs,
                "routed": world.resource::<marks::Waypoints>().routed,
                "route_cache": world.resource::<marks::Waypoints>().routes.len(),
                "ghosts": world.resource::<marks::Waypoints>().ghosts,
                "bookmarks": world.resource::<marks::CameraBookmarks>().marks.iter().map(Option::is_some).collect::<Vec<_>>(),
            },
        }),
    )
}
