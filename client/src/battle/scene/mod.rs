//! Battle scene: terrain, sky, camera and sim-battle stepping.
//!
//! Ports game/scripts/battle/battle_scene.gd (setup and fixed-tick stepping), battle_terrain*.gd
//! (height field mesh) and battle_camera.gd (TW-style camera). Owned by one lot of agent.

pub mod camera;
pub mod lighting;
pub mod replay;
pub mod replay_bar;
pub mod setup;
pub mod terrain;
pub mod water;

use bevy::platform::cell::SyncCell;
use bevy::prelude::*;
use data_model::GameData;
use sim_battle::custom::CustomBattle;
use sim_battle::{BattleSim, DT, ReplayStart, SideId};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::hud::aftermath::{HistoryInfo, ResultExtras};
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
    /// A replay is being shown (nobody commands, the replay drives the simulation).
    pub replaying: bool,
    /// Primary colour of each side (attacker, defender).
    pub colors: [[f32; 3]; 2],
    /// French title of the battle (« Bataille de Guyenne »), kept in its replay.
    pub title: String,
    /// What the dump and the bar read of the replay shown.
    pub replay_view: replay::ReplayView,
    /// Set while a replay is shown.
    playback: SyncCell<Option<replay::Playback>>,
    /// The replay of this battle was saved to the replays folder.
    pub replay_saved: Option<std::path::PathBuf>,
}

impl BattleSession {
    /// Access to the simulation (`BattleSim` is not `Sync`: only through `&mut`, so systems
    /// using it take `ResMut<BattleSession>`).
    pub fn sim_mut(&mut self) -> &mut BattleSim {
        self.sim.get()
    }

    /// The replay being shown, if any.
    pub fn playback(&mut self) -> Option<&mut replay::Playback> {
        self.playback.get().as_mut()
    }

    /// The simulation and the replay shown (disjoint borrows).
    pub fn sim_and_playback(&mut self) -> (&mut BattleSim, Option<&mut replay::Playback>) {
        (self.sim.get(), self.playback.get().as_mut())
    }

    /// The replay shown has reached the end of its recording.
    pub fn replay_at_end(&mut self) -> bool {
        let (sim, playback) = self.sim_and_playback();
        playback.is_some_and(|p| p.player.at_end(sim))
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
            .add_message::<replay::ReplayControl>()
            .add_message::<replay::WatchAgain>()
            .init_resource::<replay::ReplayDemo>()
            .add_plugins((
                terrain::BattleTerrainPlugin,
                camera::BattleCameraPlugin,
                lighting::BattleLightingPlugin,
                water::BattleWaterPlugin,
                replay_bar::ReplayBarPlugin,
            ))
            .add_systems(OnEnter(AppState::Battle), start_battle)
            .add_systems(OnExit(AppState::Battle), end_battle)
            .add_systems(
                Update,
                (
                    replay::apply_controls,
                    step_session,
                    save_replay,
                    replay::demo,
                )
                    .chain()
                    .in_set(StepSet)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            );
    }
}

/// A battle built and not yet wrapped into the session resource.
struct Opened {
    sim: BattleSim,
    start: ReplayStart,
    /// Starting hour applied after the recording starts.
    hour: Option<f64>,
    config: CustomBattle,
    seed: u64,
    title: String,
    /// The historical battle played (result screen card), scripted battles only.
    history: Option<HistoryInfo>,
}

/// Builds the session from the pending replay, the campaign battle, the historical battle, the
/// pending custom battle or the default custom battle.
#[allow(clippy::too_many_arguments)]
fn start_battle(
    mut commands: Commands,
    sim: Res<Sim>,
    pending: Option<Res<PendingBattle>>,
    campaign: Option<Res<crate::flow::CampaignBattle>>,
    historical: Option<Res<crate::menu::battles::PendingHistorical>>,
    replay_request: Option<Res<replay::PendingReplay>>,
    mut extras: ResMut<ResultExtras>,
    mut next: MessageWriter<crate::ui::fader::FadeToState>,
) {
    let Some(game) = sim.data.as_deref() else {
        error!("battle: game data not loaded");
        return;
    };
    replay::discard_recording();
    replay::set_replaying(replay_request.is_some());
    *extras = ResultExtras::default();
    if let Some(request) = replay_request {
        commands.remove_resource::<replay::PendingReplay>();
        commands.remove_resource::<PendingBattle>();
        match open_replay(game, &request) {
            Ok(mut session) => {
                let duration = session.playback().map_or(0.0, |p| p.duration);
                info!(
                    "battle: replay \"{}\" {duration} s, {} vs {}",
                    session.title, session.snapshot.strength[0], session.snapshot.strength[1]
                );
                commands.insert_resource(session);
            }
            Err(message) => {
                error!("battle: replay: {message}");
                commands.insert_resource(replay_bar::ReplayFailure(message));
                next.write(crate::ui::fader::FadeToState(AppState::MainMenu));
            }
        }
        return;
    }
    // A battle of the campaign (flow lot): built from the core's `BattleSetup`, not from a
    // custom composition, so regiment strengths, generals and sieges are those of the armies.
    let opened = if let Some(campaign) = campaign {
        commands.remove_resource::<PendingBattle>();
        crate::flow::launch::build(&campaign).map(|(battle, start, hour)| {
            let config = setup::default_custom_battle(game);
            let seed = campaign.launch.seed;
            let setup = battle.setup();
            let title = match (&campaign.site, setup.siege.is_some()) {
                (Some(map), _) => map.name.clone(),
                (None, true) => format!(
                    "Siège {}",
                    crate::ui::kit::fr::de_place(&setup.province_name)
                ),
                (None, false) => format!(
                    "Bataille {}",
                    crate::ui::kit::fr::de_place(&setup.province_name)
                ),
            };
            Opened {
                sim: battle,
                start,
                hour: Some(hour),
                config,
                seed,
                title,
                history: None,
            }
        })
    } else if let Some(historical) = historical {
        // A scripted historical battle (bsiege lot): orders of battle, site relief, waves.
        commands.remove_resource::<PendingBattle>();
        commands.remove_resource::<crate::menu::battles::PendingHistorical>();
        let seed = pending.as_ref().map_or(sim.seed, |p| p.seed);
        let config = pending
            .map(|p| p.config.clone())
            .unwrap_or_else(|| setup::default_custom_battle(game));
        open_historical(game, &historical.id, &historical.side, config, seed)
    } else {
        let (config, seed) = match pending {
            Some(pending) => (pending.config.clone(), pending.seed),
            None => (setup::default_custom_battle(game), sim.seed),
        };
        commands.remove_resource::<PendingBattle>();
        open_session(game, config, seed)
    };
    match opened.and_then(|opened| {
        let history = opened.history.clone();
        session_from_opened(game, opened).map(|session| (session, history))
    }) {
        Ok((session, history)) => {
            info!(
                "battle: {} units, {} vs {}",
                session.snapshot.regiments.iter().sum::<usize>(),
                session.snapshot.strength[0],
                session.snapshot.strength[1]
            );
            extras.history = history;
            commands.insert_resource(session);
        }
        Err(message) => error!("battle: {message}"),
    }
}

fn open_session(game: &GameData, config: CustomBattle, seed: u64) -> Result<Opened, String> {
    let root = paths::find_root()?;
    let data_dir = paths::data_dir(&root);
    setup::install_battle_rules(&data_dir);
    let (sim, start, hour) = setup::build_start(game, &data_dir, &config, seed)?;
    let province = sim.setup().province_name.clone();
    let title = if province.is_empty() {
        "Bataille personnalisée".to_owned()
    } else if province.starts_with("Bataille") {
        province
    } else {
        format!("Bataille {}", crate::ui::kit::fr::de_place(&province))
    };
    Ok(Opened {
        sim,
        start,
        hour,
        config,
        seed,
        title,
        history: None,
    })
}

/// The scripted battle of a historical map (`setup_historical`); falls back on the approximated
/// custom battle when the map cannot be built.
fn open_historical(
    game: &GameData,
    id: &str,
    side: &str,
    config: CustomBattle,
    seed: u64,
) -> Result<Opened, String> {
    let root = paths::find_root()?;
    let data_dir = paths::data_dir(&root);
    match historical_start(game, &data_dir, id, side, seed) {
        Ok((sim, start, map)) => {
            let history = HistoryInfo {
                name: map.name.clone(),
                date_fr: map.date_fr(),
                historical_winner: map.historical_winner,
                faction_names: [
                    sim.setup().attacker.faction_name.clone(),
                    sim.setup().defender.faction_name.clone(),
                ],
            };
            Ok(Opened {
                sim,
                start,
                hour: None,
                config,
                seed,
                title: map.name.clone(),
                history: Some(history),
            })
        }
        Err(error) => {
            warn!("historical battle {id}: {error}; custom approximation used");
            open_session(game, config, seed)
        }
    }
}

/// `siege::historical::build_sim` with the start it used (the replay needs it).
fn historical_start(
    game: &GameData,
    data_dir: &std::path::Path,
    id: &str,
    side: &str,
    seed: u64,
) -> Result<(BattleSim, ReplayStart, sim_battle::HistoricalMap), String> {
    setup::install_battle_rules(data_dir);
    let map = crate::battle::siege::historical::load_map(data_dir, id)?;
    let orders = game.battle_orders.values().cloned().collect();
    let abilities: Vec<_> = game.battle_abilities.values().cloned().collect();
    let standards = std::fs::read_to_string(data_dir.join("rules/battle_standards.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());
    let mut battle_setup =
        map.battle_setup(&game.unit_types, orders, standards, SideId::parse(side))?;
    battle_setup.abilities = abilities;
    let start = ReplayStart::new(battle_setup, seed).historical(map.clone());
    let sim = start.build()?;
    Ok((sim, start, map))
}

/// Playback of a replay: the session shows the re-simulation, nobody commands.
fn open_replay(game: &GameData, request: &replay::PendingReplay) -> Result<BattleSession, String> {
    let root = paths::find_root()?;
    setup::install_battle_rules(&paths::data_dir(&root));
    let (mut player, mut battle, title) = replay::load(&request.source)?;
    if request.start_at > 0.0 {
        player.seek(&mut battle, request.start_at);
        replay::drain_renderer_queues(&mut battle);
    }
    let duration = player.duration();
    let config = setup::default_custom_battle(game);
    let seed = player.replay().start.seed;
    let mut session = session_from(
        game,
        battle,
        config,
        seed,
        if title.is_empty() {
            "Rejeu".to_owned()
        } else {
            title.clone()
        },
    );
    session.player = None;
    session.replaying = true;
    session.snapshot.finished = false;
    if let Some(resume) = request.resume {
        session.speed = resume.speed;
        session.paused = resume.paused;
    }
    session.playback = SyncCell::new(Some(replay::Playback {
        player,
        title,
        duration,
    }));
    replay::refresh_view(&mut session);
    Ok(session)
}

/// Wraps an opened battle into the session resource and starts its recording.
fn session_from_opened(game: &GameData, opened: Opened) -> Result<BattleSession, String> {
    let Opened {
        mut sim,
        start,
        hour,
        config,
        seed,
        title,
        ..
    } = opened;
    // The recording starts right after the build; the hour is its first recorded input.
    replay::begin_recording(start, &sim);
    if let Some(hour) = hour {
        replay::drive(&mut sim, sim_battle::ReplayAction::StartHour { hour });
    }
    Ok(session_from(game, sim, config, seed, title))
}

/// Wraps a built simulation into the session resource.
fn session_from(
    game: &GameData,
    battle: BattleSim,
    config: CustomBattle,
    seed: u64,
    title: String,
) -> BattleSession {
    let colors = [SideId::Attacker, SideId::Defender]
        .map(|side| setup::side_color(game, &battle.setup().side(side).faction, side));
    let snapshot = BattleSnapshot::of(&battle);
    BattleSession {
        player: battle.setup().player_side,
        replaying: false,
        snapshot,
        sim: SyncCell::new(battle),
        config,
        seed,
        accumulator: 0.0,
        speed: 1.0,
        paused: false,
        colors,
        title,
        replay_view: replay::ReplayView::default(),
        playback: SyncCell::new(None),
        replay_saved: None,
    }
}

fn end_battle(mut commands: Commands, session: Option<ResMut<BattleSession>>) {
    // A battle left before its end is recorded too (« bataille inachevée »).
    if let Some(mut session) = session
        && !session.replaying
        && session.replay_saved.is_none()
        && session.snapshot.tick > 0
    {
        save_now(&mut session);
    }
    replay::discard_recording();
    replay::set_replaying(false);
    commands.remove_resource::<BattleSession>();
}

/// Writes the recording of the battle to the replays folder (once).
fn save_now(session: &mut BattleSession) {
    if session.replaying || session.replay_saved.is_some() || !replay::is_recording() {
        return;
    }
    if std::env::var("CENT_ANS_NO_REPLAY_SAVE").is_ok_and(|v| v != "0") {
        return;
    }
    let title = session.title.clone();
    session.replay_saved = replay::save_recording(session.sim_mut(), &title);
}

/// At the end of the battle: the recording goes to the replays folder.
fn save_replay(mut session: ResMut<BattleSession>) {
    if session.snapshot.finished {
        save_now(&mut session);
    }
}

/// Fixed-tick stepping of the simulation (`BattleSim::step`, 0.1 s; the AI runs inside it); in a
/// replay, the re-simulation advances instead.
pub fn step_session(
    time: Res<Time>,
    mut session: ResMut<BattleSession>,
    deployment: Option<Res<crate::battle::orders::deploy::Deployment>>,
) {
    if session.replaying {
        replay::step_playback(&time, &mut session);
        return;
    }
    if session.paused || session.snapshot.finished {
        return;
    }
    // The battle never advances before the deployment phase had its chance to open (whatever
    // the entry point: menu, `--state Battle`, campaign), only then may the first tick run.
    if let Some(deployment) = deployment
        && crate::battle::orders::deploy::awaiting_deployment(&mut session, &deployment)
    {
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
        replay::observe(session.sim_mut());
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
            "title": session.title,
            "replay": {
                "demo": world.get_resource::<replay::ReplayDemo>().map(|d| serde_json::json!({
                    "phase": format!("{:?}", d.phase), "live_tick": d.live_tick,
                    "live_digest": d.live_digest, "replay_digest": d.replay_digest,
                    "matches": d.matches, "jumped_back_to": d.jumped_back_to,
                })),
                "replaying": session.replaying,
                "duration": session.replay_view.duration,
                "at_end": session.replay_view.at_end,
                "divergence": session.replay_view.divergence,
                "keyframes": session.replay_view.keyframes,
                "recorded_actions": replay::recorded_actions(),
                "saved": session.replay_saved.as_ref().map(|p| p.to_string_lossy().into_owned()),
                "bar": world.get_resource::<replay_bar::BarStats>().map(|b| serde_json::json!({
                    "shown": b.shown, "time": b.time_text, "dragging": b.dragging,
                    "watch_button": b.watch_button, "seeks": b.seeks,
                })),
            },
            "camera": world.get_resource::<camera::BattleCameraState>().map(|c| serde_json::json!({
                "target": [c.target.x, c.target.y, c.target.z],
                "yaw": c.yaw,
                "distance": c.distance,
            })),
        }),
    )
}
