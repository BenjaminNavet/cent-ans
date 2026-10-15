//! Battle decor: trees, vegetation, ground textures, props.
//!
//! Stub (V0). Will port: game/scripts/battle/battle_vegetation.gd and decor scripts.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct BattleDecorPlugin;

impl Plugin for BattleDecorPlugin {
    fn build(&self, _app: &mut App) {}
}
