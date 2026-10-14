//! Province borders, ownership colours, picking and hover.
//!
//! Stub (V0). Will port: game/scripts/map/province_picker.gd, province_snapshot.gd, province_mates_overlay.gd, map_mode_controller.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignProvincesPlugin;

impl Plugin for CampaignProvincesPlugin {
    fn build(&self, _app: &mut App) {}
}
