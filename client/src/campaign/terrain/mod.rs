//! Relief, ground colormap, vegetation and fields of the campaign map.
//!
//! Stub (V0). Will port: game/scripts/map/ terrain/relief/vegetation scripts (map_data_loader.gd, agri_seasons.gd, biome_parents.gd, cafv_tile.gd).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignTerrainPlugin;

impl Plugin for CampaignTerrainPlugin {
    fn build(&self, _app: &mut App) {}
}
