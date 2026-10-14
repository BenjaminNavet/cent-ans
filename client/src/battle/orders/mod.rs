//! Selection and orders on the battlefield (Total War style controls).
//!
//! Stub (V0). Will port: game/scripts/battle/battle_input.gd, battle_cursor.gd, battle_formation_menu.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct BattleOrdersPlugin;

impl Plugin for BattleOrdersPlugin {
    fn build(&self, _app: &mut App) {}
}
