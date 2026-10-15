//! Campaign panels: province, faction, character, diplomacy, tech, recruitment, chronicle.
//!
//! Each panel lives in its own subfolder, owned by one lot; this file only registers them. Will port: game/scripts/ui/province_panel.gd, faction_panel.gd, character_sheet.gd, chronicle_window.gd, council_chamber.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub mod army;
pub mod chronicle;
pub mod court;
pub mod diplomacy;
pub mod faction;
pub mod feudal;
pub mod province;
pub mod siege;
pub mod tech;

/// Shared request to open or close a campaign panel (sent by the HUD, the map pickers or a
/// script). Each panel subfolder reads it and opens itself; a panel that opens closes the others
/// of the same tier (docked panels: province, army; central screens: diplomacy, tech).
#[derive(Message, Debug, Clone, PartialEq)]
pub enum OpenPanel {
    Province(String),
    Army(String),
    Recruitment(String),
    Diplomacy,
    Tech,
    Siege(String),
    Character(String),
    Council,
    Faction(String),
    Feudal,
    Chronicle,
    CloseAll,
}

pub struct CampaignPanelsPlugin;

impl Plugin for CampaignPanelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<OpenPanel>().add_plugins((
            province::PanelProvincePlugin,
            diplomacy::PanelDiplomacyPlugin,
            tech::PanelTechPlugin,
            army::PanelArmyPlugin,
            siege::PanelSiegePlugin,
            court::PanelCourtPlugin,
            feudal::PanelFeudalPlugin,
            chronicle::PanelChroniclePlugin,
            faction::PanelFactionPlugin,
        ));
    }
}
