//! Siege panel: siege status, assault, starve, sally.
//!
//! Stub (V0). Will port: game/scripts/ui/ (siege panel scripts).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct PanelSiegePlugin;

impl Plugin for PanelSiegePlugin {
    fn build(&self, _app: &mut App) {}
}
