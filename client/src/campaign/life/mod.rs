//! Living map: folk, carts, convoys, mills, incidents.
//!
//! Stub (V0). Will port: game/scripts/map/ (folk, carts, convoys, map life scripts).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CampaignLifePlugin;

impl Plugin for CampaignLifePlugin {
    fn build(&self, _app: &mut App) {}
}
