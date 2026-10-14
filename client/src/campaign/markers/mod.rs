//! Town, castle and landmark markers, labels and declutter.
//!
//! Stub (V0). Will port: game/scripts/map/marker_declutter.gd, map_hover_text.gd, map_legend.gd, bridge_meshes.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignMarkersPlugin;

impl Plugin for CampaignMarkersPlugin {
    fn build(&self, _app: &mut App) {}
}
