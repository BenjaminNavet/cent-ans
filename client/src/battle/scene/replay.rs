//! Battle replays (EP13, ADR 0072): recording of the live battle, files in the user folder, and
//! playback on a [`BattleSession`].
//!
//! Ports `battle_replay_controller.gd` and the replay half of the bridge
//! (`godot-bridge/src/battle_replay.rs`). The simulation is deterministic: a replay is the start
//! of the battle ([`ReplayStart`]) plus the inputs stamped with the step they were issued at
//! ([`ReplayAction`]); [`sim_battle::ReplayPlayer`] re-simulates it.
//!
//! Recording: the battle scene calls [`begin_recording`] once the simulation is built, and every
//! input that changes the battle goes through [`drive`] / [`issue`] / [`deploy_unit`] /
//! [`begin_deployment`] / [`start_battle`] (the orders module calls these instead of the
//! simulation directly). The recorder is process-wide because those callers only hold a
//! `&mut BattleSim`; it is cleared when a battle is built or left.
//!
//! Playback: [`PendingReplay`] (set by the replays menu or by « Revoir la bataille ») makes the
//! scene build the session from the recording; the session then has a [`Playback`], `player` is
//! `None` (nobody commands: deployment, orders and the result screen stay off) and the replay bar
//! (`replay_bar`) drives it through [`ReplayControl`] messages.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use sim_battle::{
    BattleReplay, BattleSim, Command, DT, ReplayAction, ReplayPlayer, ReplayRecorder, ReplayRules,
    ReplayStart,
};

use super::BattleSession;
use crate::app::AppState;

// ----- recording ---------------------------------------------------------------------------------

static RECORDER: Mutex<Option<ReplayRecorder>> = Mutex::new(None);

fn with_recorder<R>(f: impl FnOnce(&mut ReplayRecorder) -> R) -> Option<R> {
    RECORDER.lock().ok()?.as_mut().map(f)
}

/// Starts recording `sim`, just built from `start` (before any input, the starting hour included).
pub fn begin_recording(start: ReplayStart, sim: &BattleSim) {
    if let Ok(mut guard) = RECORDER.lock() {
        *guard = Some(ReplayRecorder::new(start, sim));
    }
}

/// Forgets the recording (a battle is built, left, or played back).
pub fn discard_recording() {
    if let Ok(mut guard) = RECORDER.lock() {
        *guard = None;
    }
}

/// True while a battle is being recorded.
pub fn is_recording() -> bool {
    RECORDER.lock().is_ok_and(|guard| guard.is_some())
}

/// Notes `action` (about to be applied to `sim`).
pub fn record(sim: &BattleSim, action: ReplayAction) {
    with_recorder(|recorder| recorder.record(sim, action));
}

/// Records `action` and applies it (`ReplayAction::apply`: results are those of the recording).
pub fn drive(sim: &mut BattleSim, action: ReplayAction) {
    record(sim, action.clone());
    action.apply(sim);
}

/// `issue_command`, recorded.
pub fn issue(sim: &mut BattleSim, command: Command) -> Result<(), sim_battle::CommandError> {
    record(
        sim,
        ReplayAction::Command {
            command: command.clone(),
        },
    );
    sim.issue_command(command)
}

/// `deploy_unit_width`, recorded.
pub fn deploy_unit(
    sim: &mut BattleSim,
    unit: u32,
    x: f64,
    z: f64,
    facing: Option<f64>,
    width: Option<f64>,
) -> Result<(), sim_battle::CommandError> {
    record(
        sim,
        ReplayAction::DeployUnit {
            unit,
            x,
            z,
            facing,
            width,
        },
    );
    sim.deploy_unit_width(unit, x, z, facing, width)
}

/// `begin_deployment`, recorded.
pub fn begin_deployment(sim: &mut BattleSim) -> bool {
    record(sim, ReplayAction::BeginDeployment);
    sim.begin_deployment()
}

/// `start_battle`, recorded.
pub fn start_battle(sim: &mut BattleSim) -> Result<(), sim_battle::CommandError> {
    record(sim, ReplayAction::StartBattle);
    sim.start_battle()
}

/// Fingerprints and end of the battle, after time moved on.
pub fn observe(sim: &BattleSim) {
    with_recorder(|recorder| recorder.observe(sim));
}

/// The recording so far, titled and dated (`None` when nothing is recorded).
pub fn recording(sim: &BattleSim, title: &str) -> Option<BattleReplay> {
    with_recorder(|recorder| {
        recorder.observe(sim);
        recorder.finish(title, now())
    })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

// ----- files -------------------------------------------------------------------------------------

/// File names of the replays: `rejeu-<unix time>[-n].json`.
const PREFIX: &str = "rejeu-";

/// Folder of the replays: `CENT_ANS_REPLAYS`, else `replays` next to the save folder.
pub fn replays_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("CENT_ANS_REPLAYS") {
        return PathBuf::from(dir);
    }
    crate::menu::saves::default_save_dir().with_file_name("replays")
}

/// Replay files of `dir`, oldest first.
pub fn replay_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().is_some_and(|e| e == "json")
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with(PREFIX))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|p| order_key(p));
    files
}

/// `(time, n)` of `rejeu-<time>[-n].json`: files of the same second keep their order of writing.
fn order_key(path: &Path) -> (u64, u64, String) {
    let name = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .trim_start_matches(PREFIX);
    let mut parts = name.split('-');
    let stamp = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let n = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    (stamp, n, name.to_owned())
}

/// Writes `replay` in `dir` and removes the oldest beyond `keep`.
pub fn store(dir: &Path, replay: &BattleReplay, keep: usize) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stamp = replay.header.recorded_at;
    let last = replay_files(dir)
        .iter()
        .map(|p| order_key(p))
        .filter(|(s, _, _)| *s == stamp)
        .map(|(_, n, _)| n)
        .max();
    let path = match last {
        None => dir.join(format!("{PREFIX}{stamp:010}.json")),
        Some(n) => dir.join(format!("{PREFIX}{stamp:010}-{}.json", n + 1)),
    };
    std::fs::write(&path, replay.to_json()).map_err(|e| e.to_string())?;
    let files = replay_files(dir);
    let excess = files.len().saturating_sub(keep.max(1));
    for old in &files[..excess] {
        let _ = std::fs::remove_file(old);
    }
    Ok(path)
}

/// Saves the recording of the battle in the replays folder (the `keep_count` latest are kept).
pub fn save_recording(sim: &BattleSim, title: &str) -> Option<PathBuf> {
    let replay = recording(sim, title)?;
    let keep = ReplayRules::bundled().keep_count;
    match store(&replays_dir(), &replay, keep) {
        Ok(path) => Some(path),
        Err(error) => {
            warn!("replay: not saved: {error}");
            None
        }
    }
}

/// One line of the replays menu.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayEntry {
    pub path: PathBuf,
    pub title: String,
    pub attacker: String,
    pub defender: String,
    pub winner: Option<sim_battle::SideId>,
    pub duration: f64,
    pub recorded_at: u64,
    pub format: u32,
    pub readable: bool,
}

/// Replays of `dir`, newest first (a file of another format is listed but not `readable`).
pub fn list_replays(dir: &Path) -> Vec<ReplayEntry> {
    let mut files = replay_files(dir);
    files.reverse();
    files
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let header = BattleReplay::header_from_json(&text).ok()?;
            Some(ReplayEntry {
                title: header.title,
                attacker: header.attacker,
                defender: header.defender,
                winner: header.winner,
                duration: header.duration,
                recorded_at: header.recorded_at,
                format: header.format,
                readable: header.format == sim_battle::replay::REPLAY_FORMAT,
                path,
            })
        })
        .collect()
}

// ----- playback ----------------------------------------------------------------------------------

/// Where the replay to show comes from.
#[derive(Debug, Clone)]
pub enum ReplaySource {
    File(PathBuf),
    Memory(Box<BattleReplay>),
}

/// What the player had set before a jump back reloaded the replay.
#[derive(Debug, Clone, Copy)]
pub struct Resume {
    pub speed: f64,
    pub paused: bool,
}

/// A replay asked for by the replays menu or by « Revoir la bataille »; consumed when the battle
/// screen opens.
#[derive(Resource, Debug, Clone)]
pub struct PendingReplay {
    pub source: ReplaySource,
    /// Battle seconds to jump to once loaded (0: the start).
    pub start_at: f64,
    pub resume: Option<Resume>,
}

impl PendingReplay {
    pub fn file(path: PathBuf) -> Self {
        PendingReplay {
            source: ReplaySource::File(path),
            start_at: 0.0,
            resume: None,
        }
    }
}

/// Playback state of a session showing a replay.
pub struct Playback {
    pub player: ReplayPlayer,
    pub title: String,
    pub duration: f64,
}

/// What the dump and the bar read of a replay without mutable access.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReplayView {
    pub duration: f64,
    pub at_end: bool,
    pub divergence: Option<String>,
    pub keyframes: usize,
    pub actions: usize,
}

impl Playback {
    /// The current view of the playback.
    pub fn view(&self, sim: &BattleSim) -> ReplayView {
        ReplayView {
            duration: self.duration,
            at_end: self.player.at_end(sim),
            divergence: self.player.divergence().map(|d| d.message.clone()),
            keyframes: self.player.keyframe_count(),
            actions: self.player.replay().actions.len(),
        }
    }
}

/// Number of inputs recorded so far (dump), `None` when nothing is recorded.
pub fn recorded_actions() -> Option<usize> {
    with_recorder(|recorder| recorder.replay().actions.len())
}

/// Messages of the replay bar.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub enum ReplayControl {
    TogglePlay,
    Speed(f64),
    /// Jump to battle seconds.
    Seek(f64),
    /// Leave the replay.
    Quit,
}

/// Asks to watch the battle just fought again (sent by the result screen button).
#[derive(Message, Debug, Clone, Copy)]
pub struct WatchAgain;

/// Loads a replay and rebuilds the simulation to show (without the jump to `start_at`).
pub fn load(source: &ReplaySource) -> Result<(ReplayPlayer, BattleSim, String), String> {
    let replay = match source {
        ReplaySource::File(path) => std::fs::read_to_string(path)
            .map_err(|e| format!("rejeu introuvable : {e}"))
            .and_then(|text| BattleReplay::from_json(&text).map_err(|e| e.to_string()))?,
        ReplaySource::Memory(replay) => (**replay).clone(),
    };
    let title = replay.header.title.clone();
    let (player, sim) = ReplayPlayer::load(replay).map_err(|e| e.to_string())?;
    Ok((player, sim, title))
}

/// Empties what the renderers have not read yet (after a jump), as `drain_renderer_queues`.
pub fn drain_renderer_queues(sim: &mut BattleSim) {
    sim.take_shots();
    sim.take_impacts();
    sim.take_new_siege_fx();
    sim.take_new_events();
}

/// « 3:05 ».
pub fn clock(seconds: f64) -> String {
    let total = seconds.max(0.0).floor() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

/// « 12 min 05 s ».
pub fn duration_fr(seconds: f64) -> String {
    let total = seconds.round().max(0.0) as u64;
    if total < 60 {
        format!("{total} s")
    } else {
        format!("{} min {:02} s", total / 60, total % 60)
    }
}

/// Applies the bar's messages to the session; a jump back reloads the replay (every renderer
/// starts afresh, nothing of the future stays on screen).
pub fn apply_controls(
    mut controls: MessageReader<ReplayControl>,
    mut watch: MessageReader<WatchAgain>,
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut leave: MessageWriter<crate::flow::LeaveBattle>,
    mut next: ResMut<NextState<AppState>>,
) {
    // « Revoir la bataille »: from the recording in memory, the result stays applied.
    if watch.read().last().is_some() && session.playback().is_none() {
        let title = session.title.clone();
        if let Some(replay) = recording(session.sim_mut(), &title) {
            commands.insert_resource(PendingReplay {
                source: ReplaySource::Memory(Box::new(replay)),
                start_at: 0.0,
                resume: None,
            });
            next.set(AppState::Battle);
        }
        return;
    }
    for control in controls.read() {
        if session.playback().is_none() {
            continue;
        }
        match *control {
            ReplayControl::TogglePlay => {
                let at_end = session.replay_at_end();
                if session.paused && at_end {
                    reload(&mut commands, &mut session, &mut next, 0.0, false);
                    return;
                }
                session.paused = !session.paused;
            }
            ReplayControl::Speed(value) => {
                session.speed = value;
                session.paused = false;
            }
            ReplayControl::Seek(seconds) => {
                let (sim, playback) = session.sim_and_playback();
                let Some(playback) = playback else { continue };
                if seconds + 0.05 < sim.elapsed() {
                    reload(&mut commands, &mut session, &mut next, seconds, true);
                    return;
                }
                playback.player.seek(sim, seconds);
                drain_renderer_queues(sim);
                let snapshot = super::BattleSnapshot::of(sim);
                session.snapshot = snapshot;
                session.snapshot.finished = false;
                refresh_view(&mut session);
            }
            ReplayControl::Quit => {
                leave.write(crate::flow::LeaveBattle { abandoned: false });
            }
        }
    }
}

fn reload(
    commands: &mut Commands,
    session: &mut BattleSession,
    next: &mut NextState<AppState>,
    seconds: f64,
    keep_state: bool,
) {
    let Some(playback) = session.playback() else {
        return;
    };
    let replay = playback.player.replay().clone();
    let resume = keep_state.then_some(Resume {
        speed: session.speed,
        paused: session.paused,
    });
    commands.insert_resource(PendingReplay {
        source: ReplaySource::Memory(Box::new(replay)),
        start_at: seconds,
        resume,
    });
    next.set(AppState::Battle);
}

/// One frame of playback: advances the re-simulation (no stepping by the live clock).
pub fn step_playback(time: &Time, session: &mut BattleSession) {
    if session.paused {
        return;
    }
    let dt = f64::from(time.delta_secs()) * session.speed;
    let (sim, playback) = session.sim_and_playback();
    let Some(playback) = playback else { return };
    let steps = playback.player.advance(sim, dt);
    let at_end = playback.player.at_end(sim);
    let fraction = playback.player.step_fraction(sim);
    session.accumulator = fraction * DT;
    if steps > 0 {
        let snapshot = super::BattleSnapshot::of(session.sim_mut());
        session.snapshot = snapshot;
        session.snapshot.finished = false;
    }
    refresh_view(session);
    if at_end {
        session.paused = true;
    }
}

/// Copies the playback state into `session.replay_view`.
pub fn refresh_view(session: &mut BattleSession) {
    let (sim, playback) = session.sim_and_playback();
    let view = playback.map(|p| p.view(sim));
    if let Some(view) = view {
        session.replay_view = view;
    }
}

// ----- dev hooks (scripts and agents only) -----------------------------------------------------

/// Progress of the replay proof (`CENT_ANS_REPLAY_DEMO=<steps>`).
#[derive(Resource, Debug, Default, Clone)]
pub struct ReplayDemo {
    pub phase: DemoPhase,
    pub frames: u32,
    pub live_tick: u64,
    pub live_digest: String,
    pub replay_digest: String,
    pub matches: Option<bool>,
    pub jumped_back_to: Option<f64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DemoPhase {
    #[default]
    Idle,
    Recorded,
    Watching,
    Compared,
    JumpedBack,
}

/// `CENT_ANS_REPLAY_DEMO=<steps>`: both sides on AI, the battle runs `<steps>` steps at once
/// (recorded), then « Revoir la bataille » starts; the replay jumps to the end and compares its
/// state fingerprint with the live one, then jumps back (reload). Proof of determinism.
pub fn demo(
    mut session: ResMut<BattleSession>,
    mut demo: ResMut<ReplayDemo>,
    mut watch: MessageWriter<WatchAgain>,
    mut controls: MessageWriter<ReplayControl>,
) {
    let Some(steps) = std::env::var("CENT_ANS_REPLAY_DEMO")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
    else {
        return;
    };
    demo.frames += 1;
    match demo.phase {
        DemoPhase::Idle if !session.replaying && demo.frames >= 15 => {
            let sim = session.sim_mut();
            drive(
                sim,
                ReplayAction::SetAi {
                    side: sim_battle::SideId::Attacker,
                    enabled: true,
                },
            );
            drive(
                sim,
                ReplayAction::SetAi {
                    side: sim_battle::SideId::Defender,
                    enabled: true,
                },
            );
            for _ in 0..steps {
                sim.step();
                observe(sim);
            }
            demo.live_tick = sim.ticks();
            demo.live_digest = format!("{:016x}", sim_battle::replay::state_digest(sim));
            let snapshot = super::BattleSnapshot::of(sim);
            session.snapshot = snapshot;
            session.paused = true;
            demo.phase = DemoPhase::Recorded;
            demo.frames = 0;
        }
        DemoPhase::Recorded if demo.frames >= 10 => {
            watch.write(WatchAgain);
            demo.phase = DemoPhase::Watching;
            demo.frames = 0;
        }
        DemoPhase::Watching if session.replaying && demo.frames == 20 => {
            let duration = session.replay_view.duration;
            controls.write(ReplayControl::Seek(duration));
        }
        DemoPhase::Watching if session.replaying && demo.frames == 30 => {
            let tick = session.sim_mut().ticks();
            demo.replay_digest = format!(
                "{:016x}",
                sim_battle::replay::state_digest(session.sim_mut())
            );
            demo.matches = Some(tick == demo.live_tick && demo.replay_digest == demo.live_digest);
            demo.phase = DemoPhase::Compared;
            demo.frames = 0;
        }
        DemoPhase::Compared if demo.frames == 10 => {
            controls.write(ReplayControl::Seek(20.0));
            demo.jumped_back_to = Some(20.0);
            demo.phase = DemoPhase::JumpedBack;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cent-ans-replays-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn clocks_are_french() {
        assert_eq!(clock(185.0), "3:05");
        assert_eq!(duration_fr(725.0), "12 min 05 s");
        assert_eq!(duration_fr(42.0), "42 s");
    }

    #[test]
    fn store_keeps_the_latest_and_lists_newest_first() {
        let root = crate::paths::find_root().expect("root");
        let dir = crate::paths::data_dir(&root);
        let (game, _) = data_model::GameData::load(&dir).expect("game data");
        let custom = super::super::setup::default_custom_battle(&game);
        super::super::setup::install_battle_rules(&dir);
        let (sim, start, _) = super::super::setup::build_start(&game, &dir, &custom, 5).unwrap();
        let recorder = ReplayRecorder::new(start, &sim);
        let out = temp_dir("store");
        for stamp in [100, 300, 200, 300] {
            store(&out, &recorder.finish(&format!("t{stamp}"), stamp), 3).unwrap();
        }
        let listed = list_replays(&out);
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].recorded_at, 300);
        assert!(listed.iter().all(|e| e.readable));
        assert_eq!(listed.last().unwrap().recorded_at, 200);
        let _ = std::fs::remove_dir_all(&out);
    }

    #[test]
    fn a_campaign_battle_on_a_historical_site_replays_identically() {
        let root = crate::paths::find_root().expect("root");
        let dir = crate::paths::data_dir(&root);
        let (game, _) = data_model::GameData::load(&dir).expect("game data");
        let custom = super::super::setup::default_custom_battle(&game);
        super::super::setup::install_battle_rules(&dir);
        let (plain, _, _) = super::super::setup::build_start(&game, &dir, &custom, 3).unwrap();
        let setup = plain.setup().clone();
        let map = crate::flow::launch::maps_in(&dir)
            .into_iter()
            .find(|m| m.id == "crecy")
            .expect("crecy map");
        let start = ReplayStart::new(setup, 3).on_site(map.clone());
        let mut sim = start.build().expect("site battle builds");
        assert!(
            sim.field().width > plain.field().width,
            "the site's scale replaces the tier's"
        );
        let mut recorder = ReplayRecorder::new(start, &sim);
        for _ in 0..40 {
            sim.step();
            recorder.observe(&sim);
        }
        let digest = sim_battle::replay::state_digest(&sim);
        let (mut player, mut copy) =
            ReplayPlayer::load(recorder.finish("Crécy", 2)).expect("loads");
        assert!(player.divergence().is_none());
        player.seek(&mut copy, 4.0);
        assert_eq!(sim_battle::replay::state_digest(&copy), digest);
    }

    #[test]
    fn a_recorded_battle_plays_back_identically() {
        let root = crate::paths::find_root().expect("root");
        let dir = crate::paths::data_dir(&root);
        let (game, _) = data_model::GameData::load(&dir).expect("game data");
        let custom = super::super::setup::default_custom_battle(&game);
        super::super::setup::install_battle_rules(&dir);
        let (mut sim, start, _) = super::super::setup::build_start(&game, &dir, &custom, 11).unwrap();
        let mut recorder = ReplayRecorder::new(start, &sim);
        recorder.record(&sim, ReplayAction::StartHour { hour: 9.0 });
        sim.set_start_hour(9.0);
        for _ in 0..80 {
            sim.step();
            recorder.observe(&sim);
        }
        let digest = sim_battle::replay::state_digest(&sim);
        let replay = recorder.finish("essai", 1);
        let (mut player, mut copy) = ReplayPlayer::load(replay).expect("loads");
        assert!(player.divergence().is_none());
        player.seek(&mut copy, 8.0);
        assert_eq!(sim_battle::replay::state_digest(&copy), digest);
        // A jump back restarts from an earlier copy and lands on the same state.
        player.seek(&mut copy, 2.0);
        assert_eq!(copy.ticks(), 20);
        player.seek(&mut copy, 8.0);
        assert_eq!(sim_battle::replay::state_digest(&copy), digest);
    }
}
