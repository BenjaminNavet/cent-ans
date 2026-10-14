//! Campaign panels: province, faction, character, diplomacy, tech, recruitment, chronicle.
//!
//! Stub (V0). Will port: game/scripts/ui/province_panel.gd, faction_panel.gd, character_sheet.gd, chronicle_window.gd, council_chamber.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignPanelsPlugin;

impl Plugin for CampaignPanelsPlugin {
    fn build(&self, _app: &mut App) {}
}
