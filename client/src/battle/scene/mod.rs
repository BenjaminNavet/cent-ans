//! Battle scene: terrain, sky, camera and sim-battle stepping.
//!
//! Ports game/scripts/battle/battle_scene.gd (setup and fixed-tick stepping), battle_terrain*.gd
//! (height field mesh) and battle_camera.gd (TW-style camera). Owned by one lot of agent.

pub mod camera;
pub mod setup;
pub mod terrain;

use std::sync::{Mutex, MutexGuard};

use bevy::prelude::*;
use data_model::GameData;
use sim_battle::custom::CustomBattle;
use sim_battle::{BattleSim, DT, SideId};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::paths;
use crate::sim::Sim;

/// A battle asked by another screen (campaign hand-over, custom battle screen); consumed when
/// the battle screen opens. Without it a default custom battle starts.
#[derive(Resource, Clone)]
pub struct PendingBattle {
    pub config: CustomBattle,
    pub seed: u64,
}

/// The running battle: the simulation and its fixed-tick clock.
#[derive(Resource)]
pub struct BattleSession {
    sim: Mutex<BattleSim>,
    pub config: CustomBattle,
    pub seed: u64,
    /// Simulated time not yet consumed by a fixed step (seconds).
    pub accumulator: f64,
    /// Simulation speed factor (1 = real time).
    pub speed: f64,
    pub paused: bool,
    pub player: Option<SideId>,
    /// Primary colour of each side (attacker, defender).
    pub colors: [[f32; 3]; 2],
}

impl BattleSession {
    /// Read access to the simulation (`BattleSim` is not `Sync`, hence the lock; no contention:
    /// every system runs on the main schedule).
    pub fn sim(&self) -> MutexGuard<'_, BattleSim> {
        self.sim.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn sim_mut(&mut self) -> &mut BattleSim {
        self.sim.get_mut().unwrap_or_else(|e| e.into_inner())
    }

    /// Part of the next fixed step already elapsed (0..1), for pose blending.
    pub fn step_fraction(&self) -> f32 {
        (self.accumulator / DT).clamp(0.0, 1.0) as f32
    }
}

/// Most fixed steps run in one frame (a slow frame does not spiral).
const MAX_STEPS_PER_FRAME: u32 = 5;

/// System set of the fixed-tick stepping (renderers run after it).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StepSet;

pub struct BattleScenePlugin;

impl Plugin for BattleScenePlugin {
    fn build(&self, app: &mut App) {
        app.register_dump(dump_battle)
            .add_plugins((terrain::BattleTerrainPlugin, camera::BattleCameraPlugin))
            .add_systems(OnEnter(AppState::Battle), start_battle)
            .add_systems(OnExit(AppState::Battle), end_battle)
            .add_systems(
                Update,
                step_session
                    .in_set(StepSet)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

/// Builds the session from the pending battle or the default custom battle.
fn start_battle(mut commands: Commands, sim: Res<Sim>, pending: Option<Res<PendingBattle>>) {
    let Some(game) = sim.data.as_deref() else {
        error!("battle: game data not loaded");
        return;
    };
    let (config, seed) = match pending {
        Some(pending) => (pending.config.clone(), pending.seed),
        None => (setup::default_custom_battle(game), sim.seed),
    };
    commands.remove_resource::<PendingBattle>();
    match open_session(game, config, seed) {
        Ok(session) => {
            info!(
                "battle: {} units, {} vs {}",
                session.sim().units().len(),
                session.sim().strength(SideId::Attacker),
                session.sim().strength(SideId::Defender)
            );
            commands.insert_resource(session);
        }
        Err(message) => error!("battle: {message}"),
    }
}

fn open_session(game: &GameData, config: CustomBattle, seed: u64) -> Result<BattleSession, String> {
    let root = paths::find_root()?;
    let data_dir = paths::data_dir(&root);
    setup::install_battle_rules(&data_dir);
    let battle = setup::build_sim(game, &data_dir, &config, seed)?;
    let colors = [SideId::Attacker, SideId::Defender]
        .map(|side| setup::side_color(game, &battle.setup().side(side).faction, side));
    Ok(BattleSession {
        player: setup::player_side(&config),
        sim: Mutex::new(battle),
        config,
        seed,
        accumulator: 0.0,
        speed: 1.0,
        paused: false,
        colors,
    })
}

fn end_battle(mut commands: Commands) {
    commands.remove_resource::<BattleSession>();
}

/// Fixed-tick stepping of the simulation (`BattleSim::step`, 0.1 s; the AI runs inside it).
fn step_session(time: Res<Time>, mut session: ResMut<BattleSession>) {
    if session.paused || session.sim().is_finished() {
        return;
    }
    session.accumulator += f64::from(time.delta_secs()) * session.speed;
    let mut steps = 0;
    while session.accumulator >= DT && steps < MAX_STEPS_PER_FRAME {
        session.sim_mut().step();
        session.accumulator -= DT;
        steps += 1;
    }
    if steps == MAX_STEPS_PER_FRAME {
        session.accumulator = session.accumulator.min(DT);
    }
}

fn dump_battle(world: &World) -> (String, serde_json::Value) {
    let Some(session) = world.get_resource::<BattleSession>() else {
        return ("battle".into(), serde_json::json!({ "active": false }));
    };
    let sim = session.sim();
    let field = sim.field();
    let per_side = |side: SideId| {
        let units = sim
            .units()
            .iter()
            .filter(|u| u.side == side && !u.synthetic);
        let regiments = units.clone().count();
        let alive: u32 = units.map(|u| u.soldiers()).sum();
        serde_json::json!({ "units": regiments, "soldiers": alive, "strength": sim.strength(side) })
    };
    let drawn = world
        .get_resource::<crate::battle::units::SoldierStats>()
        .map_or(0, |stats| stats.drawn);
    (
        "battle".into(),
        serde_json::json!({
            "active": true,
            "tick": sim.ticks(),
            "elapsed_s": sim.elapsed(),
            "finished": sim.is_finished(),
            "attacker": per_side(SideId::Attacker),
            "defender": per_side(SideId::Defender),
            "soldiers_alive": per_side(SideId::Attacker)["soldiers"].as_u64().unwrap_or(0)
                + per_side(SideId::Defender)["soldiers"].as_u64().unwrap_or(0),
            "instanced_soldiers_drawn": drawn,
            "terrain": { "width": field.width, "depth": field.depth, "grid": [field.nx, field.nz] },
            "paused": session.paused,
            "speed": session.speed,
            "camera": world.get_resource::<camera::BattleCameraState>().map(|c| serde_json::json!({
                "target": [c.target.x, c.target.y, c.target.z],
                "yaw": c.yaw,
                "distance": c.distance,
            })),
        }),
    )
}
