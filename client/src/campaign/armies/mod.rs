//! Army figures, plates, picking, movement orders and paths on the campaign map.
//!
//! Ports (subset, see `docs/wip/bm-armies.md`): game/scripts/map/army_markers.gd, army_marker.gd,
//! army_plate.gd, army_scale.gd, army_picker.gd, army_movement_controller.gd,
//! army_movement_path.gd, army_movement_bubble.gd, path_preview.gd, reachable_markers.gd,
//! order_ripple.gd. Army state is read from the `Sim` resource (`CampaignState`); orders go
//! through `CampaignState::submit_order_outcome` exactly as `godot-bridge` does
//! (`campaign_sim_movement.rs`). World convention: `(x, ground, z) = (map px.x, h, map px.y)`.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

mod markers;
mod orders;
pub mod path;
pub mod pick;
pub mod projection;
pub mod scale;
mod views;

use bevy::prelude::*;
use sim_campaign::ArmyId;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;

pub use orders::{ArmyCommand, OrderLog};
pub use projection::HeightSampler;
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
        app.init_resource::<SelectedArmy>()
            .init_resource::<HoveredArmy>()
            .init_resource::<HeightSampler>()
            .init_resource::<ArmyViews>()
            .init_resource::<OrderLog>()
            .init_resource::<orders::PathPreviewState>()
            .init_resource::<orders::Reachable>()
            .init_resource::<orders::Ripples>()
            .add_message::<ArmyCommand>()
            .register_dump(orders::dump_armies)
            .add_systems(
                Update,
                (
                    views::refresh_views,
                    markers::sync_markers,
                    markers::place_markers,
                    markers::update_plates,
                    orders::hover_and_input,
                    orders::test_hook,
                    orders::execute_commands,
                    orders::refresh_overlays,
                    orders::preview_under_cursor,
                    orders::draw_overlays,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(OnExit(AppState::Campaign), orders::reset);
    }
}
