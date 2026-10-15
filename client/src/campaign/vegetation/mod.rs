//! Campaign forests, trees and wetlands dressing.
//!
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

pub mod data;
pub mod mask;
pub mod render;
pub mod tiles;

use bevy::prelude::*;

pub struct CampaignVegetationPlugin;

impl Plugin for CampaignVegetationPlugin {
    fn build(&self, _app: &mut App) {}
}
