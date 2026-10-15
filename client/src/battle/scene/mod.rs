//! Battle scene: terrain, sky, camera and sim-battle stepping.
//!
//! Ports game/scripts/battle/battle_scene.gd (setup and fixed-tick stepping), battle_terrain*.gd
//! (height field mesh) and battle_camera.gd (TW-style camera). Owned by one lot of agent.

pub mod camera;
pub mod setup;
pub mod terrain;

use bevy::platform::cell::SyncCell;
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
    sim: SyncCell<BattleSim>,
    /// Counters of the latest step, readable without access to the simulation (dump, HUD).
    pub snapshot: BattleSnapshot,
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
    /// Access to the simulation (`BattleSim` is not `Sync`: only through `&mut`, so systems
    /// using it take `ResMut<BattleSession>`).
    pub fn sim_mut(&mut self) -> &mut BattleSim {
        self.sim.get()
    }

    /// Part of the next fixed step already elapsed (0..1), for pose blending.
    pub fn step_fraction(&self) -> f32 {
        (self.accumulator / DT).clamp(0.0, 1.0) as f32
    }
}

/// What the dump and the HUD need from the simulation, refreshed after every step.
#[derive(Debug, Clone, Default)]
pub struct BattleSnapshot {
    pub tick: u64,
    pub elapsed_s: f64,
    pub finished: bool,
    /// `[attacker, defender]`: regiments, soldiers alive, strength, as in the HUD balance bar.
    pub regiments: [usize; 2],
    pub soldiers: [u32; 2],
    pub strength: [u32; 2],
    pub field: (f64, f64, usize, usize),
}

impl BattleSnapshot {
    pub fn of(sim: &BattleSim) -> Self {
        let mut snapshot = BattleSnapshot {
            tick: sim.ticks(),
            elapsed_s: sim.elapsed(),
            finished: sim.is_finished(),
            field: (
                sim.field().width,
                sim.field().depth,
                sim.field().nx,
                sim.field().nz,
            ),
            ..default()
        };
        for (index, side) in SideId::BOTH.into_iter().enumerate() {
            for unit in sim
                .units()
                .iter()
                .filter(|u| u.side == side && !u.synthetic)
            {
                snapshot.regiments[index] += 1;
                snapshot.soldiers[index] += unit.soldiers();
            }
            snapshot.strength[index] = sim.strength(side);
        }
        snapshot
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
fn start_battle(
    mut commands: Commands,
    sim: Res<Sim>,
    pending: Option<Res<PendingBattle>>,
    campaign: Option<Res<crate::flow::CampaignBattle>>,
) {
    let Some(game) = sim.data.as_deref() else {
        error!("battle: game data not loaded");
        return;
    };
    // A battle of the campaign (flow lot): built from the core's `BattleSetup`, not from a
    // custom composition, so regiment strengths, generals and sieges are those of the armies.
    let opened = if let Some(campaign) = campaign {
        commands.remove_resource::<PendingBattle>();
        crate::flow::launch::build_sim(&campaign.launch).and_then(|battle| {
            let config = setup::default_custom_battle(game);
            let seed = campaign.launch.seed;
            session_from_sim(game, battle, config, seed)
        })
    } else {
        let (config, seed) = match pending {
            Some(pending) => (pending.config.clone(), pending.seed),
            None => (setup::default_custom_battle(game), sim.seed),
        };
        commands.remove_resource::<PendingBattle>();
        open_session(game, config, seed)
    };
    match opened {
        Ok(session) => {
            info!(
                "battle: {} units, {} vs {}",
                session.snapshot.regiments.iter().sum::<usize>(),
                session.snapshot.strength[0],
                session.snapshot.strength[1]
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
    session_from_sim(game, battle, config, seed)
}

/// Wraps a built simulation into the session resource.
fn session_from_sim(
    game: &GameData,
    battle: BattleSim,
    config: CustomBattle,
    seed: u64,
) -> Result<BattleSession, String> {
    let colors = [SideId::Attacker, SideId::Defender]
        .map(|side| setup::side_color(game, &battle.setup().side(side).faction, side));
    let snapshot = BattleSnapshot::of(&battle);
    Ok(BattleSession {
        player: battle.setup().player_side,
        snapshot,
        sim: SyncCell::new(battle),
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
    if session.paused || session.snapshot.finished {
        return;
    }
    session.accumulator += f64::from(time.delta_secs()) * session.speed;
    let mut steps = 0;
    while session.accumulator >= DT && steps < MAX_STEPS_PER_FRAME {
        session.sim_mut().step();
        session.accumulator -= DT;
        steps += 1;
    }
    if steps > 0 {
        let snapshot = BattleSnapshot::of(session.sim_mut());
        session.snapshot = snapshot;
    }
    if steps == MAX_STEPS_PER_FRAME {
        session.accumulator = session.accumulator.min(DT);
    }
}

fn dump_battle(world: &World) -> (String, serde_json::Value) {
    let Some(session) = world.get_resource::<BattleSession>() else {
        return ("battle".into(), serde_json::json!({ "active": false }));
    };
    let snap = &session.snapshot;
    let side = |i: usize| serde_json::json!({ "units": snap.regiments[i], "soldiers": snap.soldiers[i], "strength": snap.strength[i] });
    let drawn = world
        .get_resource::<crate::battle::units::SoldierStats>()
        .map_or(0, |stats| stats.drawn);
    (
        "battle".into(),
        serde_json::json!({
            "active": true,
            "tick": snap.tick,
            "elapsed_s": snap.elapsed_s,
            "finished": snap.finished,
            "attacker": side(0),
            "defender": side(1),
            "soldiers_alive": snap.soldiers[0] + snap.soldiers[1],
            "instanced_soldiers_drawn": drawn,
            "terrain": { "width": snap.field.0, "depth": snap.field.1, "grid": [snap.field.2, snap.field.3] },
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
