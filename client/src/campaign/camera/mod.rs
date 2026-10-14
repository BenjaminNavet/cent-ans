//! Campaign 3D camera (zoom levels, pan, orbit) and its input.
//!
//! Stub (V0). Will port: game/scripts/map/campaign_camera.gd, map_projection.gd, ui/camera_feel.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignCameraPlugin;

impl Plugin for CampaignCameraPlugin {
    fn build(&self, _app: &mut App) {}
}
