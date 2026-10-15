//! Chronicle window: history of the campaign and battles.
//!
//! Stub (V0). Will port: game/scripts/ui/chronicle_window.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct PanelChroniclePlugin;

impl Plugin for PanelChroniclePlugin {
    fn build(&self, _app: &mut App) {}
}
