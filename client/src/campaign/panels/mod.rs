//! Campaign panels: province, faction, character, diplomacy, tech, recruitment, chronicle.
//!
//! Each panel lives in its own subfolder, owned by one lot; this file only registers them. Will port: game/scripts/ui/province_panel.gd, faction_panel.gd, character_sheet.gd, chronicle_window.gd, council_chamber.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub mod army;
pub mod diplomacy;
pub mod province;
pub mod tech;

pub struct CampaignPanelsPlugin;

impl Plugin for CampaignPanelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            province::PanelProvincePlugin,
            diplomacy::PanelDiplomacyPlugin,
            tech::PanelTechPlugin,
            army::PanelArmyPlugin,
        ));
    }
}
