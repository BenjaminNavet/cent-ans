//! Army figures, plates, picking, movement orders and paths on the campaign map.
//!
//! Ports (subset, see `docs/wip/bm-armies.md`): game/scripts/map/army_markers.gd, army_marker.gd,
//! army_plate.gd, army_scale.gd, army_picker.gd, army_movement_controller.gd,
//! army_movement_path.gd, army_movement_bubble.gd, path_preview.gd, reachable_markers.gd,
//! order_ripple.gd. Army state is read from the `Sim` resource (`CampaignState`); orders go
//! through `CampaignState::submit_order_outcome` exactly as `godot-bridge` does
//! (`campaign_sim_movement.rs`). World convention: `(x, ground, z) = (map px.x, h, map px.y)`.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

mod bubble;
pub mod bubbles;
pub mod cloth;
mod feedback;
pub mod figures;
pub mod hover_text;
pub mod landmask;
mod markers;
mod orders;
pub mod path;
pub mod pick;
pub mod plate_layout;
pub mod pointer;
pub mod projection;
pub mod roster;
pub mod scale;
pub mod sea;
pub mod skinned;
pub mod standard;
mod views;

use bevy::prelude::*;
use sim_campaign::ArmyId;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::flow::orders_open;

pub use feedback::{CursorMode, HoverHint};
pub use orders::{ArmyCommand, OrderLog};
pub use projection::Ground;
pub use sea::SeaLanes;
pub use views::{ArmyView, ArmyViews};

/// The army the player has selected.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct SelectedArmy(pub Option<ArmyId>);

/// Army under the cursor.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct HoveredArmy(pub Option<ArmyId>);

pub struct CampaignArmiesPlugin;

impl Plugin for CampaignArmiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((roster::RosterPlugin, standard::StandardPlugin))
            .init_resource::<SelectedArmy>()
            .init_resource::<HoveredArmy>()
            .init_resource::<ArmyViews>()
            .init_resource::<views::FogHidden>()
            .init_resource::<markers::PlateStats>()
            .init_resource::<landmask::LandMask>()
            .init_resource::<OrderLog>()
            .init_resource::<orders::PathPreviewState>()
            .init_resource::<orders::Reachable>()
            .init_resource::<orders::Ripples>()
            .init_resource::<orders::ReliefTick>()
            .init_resource::<bubble::BubbleHandles>()
            .init_resource::<figures::LiveryCache>()
            .init_resource::<pointer::PointerOverUi>()
            .init_resource::<skinned::SkinnedArmies>()
            .init_resource::<sea::Glides>()
            .init_resource::<sea::SeaLanes>()
            .init_resource::<feedback::CursorMode>()
            .init_resource::<feedback::HoverHint>()
            .init_resource::<bubbles::RichHover>()
            .add_message::<ArmyCommand>()
            .register_dump(orders::dump_armies)
            .add_systems(
                OnEnter(AppState::Campaign),
                (
                    sea::load_sea_lanes,
                    landmask::load_land_mask,
                    skinned::load_skinned,
                    feedback::spawn_hint,
                ),
            )
            .add_systems(
                Update,
                (
                    views::refresh_views,
                    orders::auto_resolve_naval,
                    orders::advance_glides,
                    markers::sync_markers,
                    figures::tint_liveries,
                    figures::hide_far_bivouacs,
                    markers::refresh_plate_text,
                    markers::place_markers,
                    (skinned::update_skinned_armies, standard::update_standards).chain(),
                    pointer::track_pointer_over_ui,
                    markers::update_plates,
                    orders::track_relief,
                    orders::hover_and_input.run_if(orders_open),
                    orders::test_hook
                        .run_if(orders_open)
                        .run_if(crate::ui::kit::text_field::keyboard_free),
                    orders::execute_commands,
                    orders::refresh_overlays,
                    orders::update_bubble,
                    orders::preview_under_cursor,
                    orders::draw_overlays,
                    (
                        feedback::update_cursor,
                        bubbles::update_rich_hover,
                        feedback::update_hint,
                    )
                        .chain(),
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(
                OnExit(AppState::Campaign),
                (
                    orders::reset,
                    orders::forget_bubble,
                    feedback::reset_cursor,
                    skinned::reset_skinned,
                ),
            );
    }
}
