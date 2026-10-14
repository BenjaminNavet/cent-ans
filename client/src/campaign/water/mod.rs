//! Sea, lakes, rivers and sea lanes.
//!
//! Stub (V0). Will port: game/scripts/map/sea.gd, sea_basins.gd, sea_lane_layer.gd, rivers_renderer.gd, river_labels.gd, river_crossings.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignWaterPlugin;

impl Plugin for CampaignWaterPlugin {
    fn build(&self, _app: &mut App) {}
}
