//! Army figures, plates, picking, movement orders and paths.
//!
//! Stub (V0). Will port: game/scripts/map/army_markers.gd, army_figures.gd, army_plate.gd, army_picker.gd, army_movement_controller.gd, army_movement_path.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignArmiesPlugin;

impl Plugin for CampaignArmiesPlugin {
    fn build(&self, _app: &mut App) {}
}
