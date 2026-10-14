//! Soldier and unit rendering, formations, animations and effects.
//!
//! Stub (V0). Will port: game/scripts/battle/battle_cavalry_gaits.gd, battle_formation_outline.gd, battle_blood.gd, battle_effects.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct BattleUnitsPlugin;

impl Plugin for BattleUnitsPlugin {
    fn build(&self, _app: &mut App) {}
}
