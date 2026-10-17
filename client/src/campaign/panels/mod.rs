//! Campaign panels: province, faction, character, diplomacy, tech, recruitment, chronicle.
//!
//! Each panel lives in its own subfolder, owned by one lot; this file only registers them. Will port: game/scripts/ui/province_panel.gd, faction_panel.gd, character_sheet.gd, chronicle_window.gd, council_chamber.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub mod army;
pub mod chronicle;
pub mod court;
pub mod diplomacy;
pub mod economy;
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
    Economy,
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
            economy::PanelEconomyPlugin,
            crate::campaign::council::CouncilRoomPlugin,
        ));
    }
}

#[cfg(test)]
mod switch_tests {
    use super::*;
    use crate::menu::KeyEdges;
    use crate::settings::KeyMap;
    use crate::ui::kit::text_field::TextInputFocus;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<tech::TechPanel>()
            .init_resource::<diplomacy::DiplomacyUi>()
            .init_resource::<KeyEdges>()
            .init_resource::<KeyMap>()
            .init_resource::<TextInputFocus>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<OpenPanel>()
            .add_systems(Update, (tech::open_close, diplomacy::open_close));
        app
    }

    fn send(app: &mut App, panel: OpenPanel) {
        app.world_mut().write_message(panel);
        app.update();
    }

    fn open(app: &App) -> (bool, bool) {
        (
            app.world().resource::<tech::TechPanel>().open,
            app.world().resource::<diplomacy::DiplomacyUi>().open,
        )
    }

    /// A launcher click on another window switches in one press; the same launcher toggles.
    #[test]
    fn launcher_switches_then_toggles() {
        let mut app = app();
        send(&mut app, OpenPanel::Tech);
        assert_eq!(open(&app), (true, false));
        send(&mut app, OpenPanel::Diplomacy);
        assert_eq!(open(&app), (false, true));
        send(&mut app, OpenPanel::Tech);
        assert_eq!(open(&app), (true, false));
        send(&mut app, OpenPanel::Tech);
        assert_eq!(open(&app), (false, false));
        send(&mut app, OpenPanel::Diplomacy);
        send(&mut app, OpenPanel::Diplomacy);
        assert_eq!(open(&app), (false, false));
    }
}
