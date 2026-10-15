//! Campaign ↔ battle flow: encounter → pre-battle → battle or auto-resolve → result back to the campaign.
//!
//! Stub (V0). Will port: game/scripts/ (battle launch / result return glue).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct FlowPlugin;

impl Plugin for FlowPlugin {
    fn build(&self, _app: &mut App) {}
}
