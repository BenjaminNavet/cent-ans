//! Court panels: character sheet, council chamber.
//!
//! Stub (V0). Will port: game/scripts/ui/character_sheet.gd, council_chamber.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct PanelCourtPlugin;

impl Plugin for PanelCourtPlugin {
    fn build(&self, _app: &mut App) {}
}
