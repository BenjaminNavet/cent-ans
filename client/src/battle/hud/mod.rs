//! Battle HUD: unit cards, alerts, speed, comparison, result screen.
//!
//! Skeleton (bhud lot). Ports game/scripts/battle/battle_hud.gd, unit_card.gd, roster_card.gd,
//! battle_alerts_column.gd, battle_result_screen.gd, battle_compare_panel.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

pub mod alerts;
pub mod cards;
pub mod compare;
pub mod result;
pub mod speed;
pub mod tips;
pub mod view;

use bevy::prelude::*;

pub struct BattleHudPlugin;

impl Plugin for BattleHudPlugin {
    fn build(&self, _app: &mut App) {}
}
