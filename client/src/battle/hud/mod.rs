//! Battle HUD: unit cards, alerts, minimap, aftermath.
//!
//! Stub (V0). Will port: game/scripts/battle/battle_hud.gd, battle_alerts_column.gd, battle_aftermath.gd, battle_compare_panel.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct BattleHudPlugin;

impl Plugin for BattleHudPlugin {
    fn build(&self, _app: &mut App) {}
}
