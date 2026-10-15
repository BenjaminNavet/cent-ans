//! Detailed towns at close zoom: town builder/plan/layers, outbuildings, landmarks, monuments, growth.
//!
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

pub mod data;
pub mod growth;
pub mod kit;
pub mod plan;
pub mod profile;
pub mod rng;

use bevy::prelude::*;

pub struct CampaignTownsPlugin;

impl Plugin for CampaignTownsPlugin {
    fn build(&self, _app: &mut App) {}
}
