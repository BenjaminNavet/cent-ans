//! Battle scene: terrain, sky, weather, camera and sim-battle stepping.
//!
//! Stub (V0). Will port: game/scripts/battle/battle_scene.gd, battle_camera.gd, battle_atmosphere.gd, battle_decor.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct BattleScenePlugin;

impl Plugin for BattleScenePlugin {
    fn build(&self, _app: &mut App) {}
}
