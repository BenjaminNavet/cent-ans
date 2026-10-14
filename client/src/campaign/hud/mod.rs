//! Campaign HUD: top bar, alerts, end-turn button, army strip.
//!
//! Stub (V0). Will port: game/scripts/map/hud_controller.gd, map_ui.gd, game/scripts/ui/top_bar_fit.gd, alerts.gd, army_strip.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignHudPlugin;

impl Plugin for CampaignHudPlugin {
    fn build(&self, _app: &mut App) {}
}
