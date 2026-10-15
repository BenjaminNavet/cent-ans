//! Building the 3D battle of a pending campaign battle.
//!
//! Godot's `battle_scene.gd` `begin()` asks the bridge for `get_battle_setup`, builds a
//! `BattleSim` from it with the campaign seed and sets the starting hour drawn by the campaign
//! (EP8). Here `game_api::flow::battle_launch` gives the same package and the battle scene
//! (`battle::scene`) reads [`CampaignBattle`] when it opens.

use bevy::prelude::*;
use game_api::flow::BattleLaunch;
use sim_battle::{BattleSim, ReplayStart};

use crate::battle::scene::setup::install_battle_rules;
use crate::paths;

/// Set by flow right before entering `AppState::Battle`; consumed (kept until the battle ends)
/// by the battle scene, which builds the simulation from the core's `BattleSetup`.
#[derive(Resource, Clone)]
pub struct CampaignBattle {
    pub launch: BattleLaunch,
}

/// The simulation of a campaign battle (same construction path as the Godot bridge's `setup`).
pub fn build_sim(launch: &BattleLaunch) -> Result<BattleSim, String> {
    let root = paths::find_root()?;
    install_battle_rules(&paths::data_dir(&root));
    let mut sim = ReplayStart::new(launch.setup.clone(), launch.seed).build()?;
    sim.set_start_hour(launch.hour);
    Ok(sim)
}
