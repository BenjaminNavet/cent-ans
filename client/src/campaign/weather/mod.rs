//! Campaign weather, seasons overlay, mist and fog of war.
//!
//! Stub (V0). Will port: game/scripts/map/ (weather, mist, fog of war scripts).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignWeatherPlugin;

impl Plugin for CampaignWeatherPlugin {
    fn build(&self, _app: &mut App) {}
}
