//! Faction panel: overview of a faction (lord, titles, treasury, armies, relations).
//!
//! Stub (V0). Will port: game/scripts/ui/faction_panel.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct PanelFactionPlugin;

impl Plugin for PanelFactionPlugin {
    fn build(&self, _app: &mut App) {}
}
