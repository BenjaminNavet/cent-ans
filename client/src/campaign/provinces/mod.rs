//! Province layer of the campaign map: data, projection, picking, borders, map modes, labels.
//!
//! Skeleton. Ports game/scripts/map/{province_picker, faction_borders, map_mode_controller,
//! stance_fill, region_labels, label_placer}.gd. Owned by a single lot of agent (ADR 0357).

use bevy::prelude::*;

pub mod borders;
pub mod data;
pub mod labels;
pub mod modes;
pub mod picker;
pub mod projection;

pub struct CampaignProvincesPlugin;

impl Plugin for CampaignProvincesPlugin {
    fn build(&self, _app: &mut App) {}
}
