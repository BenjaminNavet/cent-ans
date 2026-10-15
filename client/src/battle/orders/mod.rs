//! Selection and orders on the battlefield (Total War style controls).
//!
//! Stub (V0). Will port: game/scripts/battle/battle_input.gd, battle_cursor.gd, battle_formation_menu.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.
//!
//! Shared interface (fixed before wave 3): [`BattleSelection`] (read by the battle HUD, written
//! here) and [`SelectBattleUnits`] (sent by the HUD unit cards, applied here).

use bevy::prelude::*;

/// Player's selected regiments, by `sim_battle::Unit::id`, in selection order.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct BattleSelection {
    pub units: Vec<u32>,
}

/// Request to change the selection (unit cards of the HUD, hotkeys).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SelectBattleUnits {
    pub units: Vec<u32>,
    /// Add to the current selection instead of replacing it (Shift/Ctrl).
    pub additive: bool,
}

pub struct BattleOrdersPlugin;

impl Plugin for BattleOrdersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BattleSelection>()
            .add_message::<SelectBattleUnits>();
    }
}
