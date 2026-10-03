//! Battle replays (lot EP13, ADR 0072).
//!
//! The simulation is deterministic: the same start (setup, seed, scale,
//! historical site), the same inputs issued at the same step always give the
//! same battle. A [`BattleReplay`] therefore stores only the start and the
//! inputs stamped with the step they were issued at ([`ReplayEntry`]); the
//! battle is re-simulated to watch it again ([`ReplayPlayer`]).
//!
//! To notice a rule change between recording and viewing, the recording also
//! keeps fingerprints of the state ([`state_digest`]): right after the start,
//! every [`ReplayRules::checkpoint_seconds`] and at the end (with the
//! outcome). The player compares them as it goes and reports the first
//! mismatch ([`Divergence`]) instead of silently showing another battle.
//!
//! Jumps along the time bar: the player keeps in-memory copies of the
//! simulation ([`BattleSim`] is `Clone`) every
//! [`ReplayRules::keyframe_seconds`]; a jump back restarts from the nearest
//! earlier copy, a jump forward simulates ahead.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::historical::HistoricalMap;
use crate::outcome::BattleOutcome;
use crate::queue::QueuedOrder;
use crate::scale::BattleScale;
use crate::setup::{BattleSetup, SideId};
use crate::sim::{BattleSim, DT};

/// Version of the replay file format. A file of another version is refused
/// with a message (never read by guesswork).
pub const REPLAY_FORMAT: u32 = 1;

/// Contents of `data/rules/battle_replay.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Replays kept in the user folder.
    pub keep_count: usize,
    /// Period of the state fingerprints (battle seconds).
    pub checkpoint_seconds: f64,
    /// Period of the in-memory copies kept by the player (battle seconds).
    pub keyframe_seconds: f64,
    /// Copies at most (the period grows for long battles).
    pub max_keyframes: usize,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_replay.json");

impl ReplayRules {
    /// `data/rules/battle_replay.json` as compiled into the crate.
    pub fn bundled() -> &'static ReplayRules {
        static RULES: OnceLock<ReplayRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_replay.json is valid")
        })
    }

    fn period_ticks(seconds: f64) -> u64 {
        ((seconds / DT).round() as u64).max(1)
    }
}

/// How the battle was built: everything [`ReplayStart::build`] needs to
/// rebuild the very same simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayStart {
    pub setup: BattleSetup,
    pub seed: u64,
    /// Scale actually used (tier by head count, forced tier or map scale).
    pub scale: BattleScale,
    /// Historical site of the battle (EP7), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<HistoricalMap>,
    /// `true`: the historical battle itself (orders of battle, weather,
    /// hour, scenario: [`HistoricalMap::start`]); `false`: a campaign
    /// battle fought on the site only ([`HistoricalMap::apply_site`]).
    #[serde(default)]
    pub scripted: bool,
    /// NT2: weather forced by a custom battle (`None`: drawn from the
    /// season). Ignored on a historical site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weather: Option<crate::field::Weather>,
}

impl ReplayStart {
    /// A battle at the scale of its head count (`BattleSim::new`).
    pub fn plain(setup: BattleSetup, seed: u64) -> Self {
        let scale = BattleScale::for_setup(&setup);
        Self::scaled(setup, seed, scale)
    }

    /// A battle at a forced scale (`BattleSim::new_scaled`).
    pub fn scaled(setup: BattleSetup, seed: u64, scale: BattleScale) -> Self {
        ReplayStart {
            setup,
            seed,
            scale,
            site: None,
            scripted: false,
            weather: None,
        }
    }

    /// A campaign battle fought on a historical site.
    pub fn on_site(setup: BattleSetup, seed: u64, map: HistoricalMap) -> Self {
        ReplayStart {
            setup,
            seed,
            scale: map.scale(),
            site: Some(map),
            scripted: false,
            weather: None,
        }
    }

    /// The historical battle of `map`.
    pub fn historical(setup: BattleSetup, seed: u64, map: HistoricalMap) -> Self {
        ReplayStart {
            setup,
            seed,
            scale: map.scale(),
            site: Some(map),
            scripted: true,
            weather: None,
        }
    }

    /// NT2: the same start with the weather forced (custom battle).
    pub fn with_weather(mut self, weather: Option<crate::field::Weather>) -> Self {
        self.weather = weather;
        self
    }

    /// Builds the simulation (the single construction path of the bridge,
    /// so a live battle and its replay start alike).
    pub fn build(&self) -> Result<BattleSim, String> {
        match (&self.site, self.scripted) {
            (Some(map), true) => map.start(self.setup.clone(), self.seed),
            (Some(map), false) => {
                let mut sim =
                    BattleSim::new_scaled(self.setup.clone(), self.seed, self.scale.clone())
                        .map_err(|e| e.to_string())?;
                map.apply_site(sim.field_mut());
                Ok(sim)
            }
            (None, _) => BattleSim::new_scaled_weather(
                self.setup.clone(),
                self.seed,
                self.scale.clone(),
                self.weather,
            )
            .map_err(|e| e.to_string()),
        }
    }
}

/// An input that changes the battle, as the bridge received it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ReplayAction {
    /// `set_start_hour` (EP8).
    StartHour { hour: f64 },
    /// `set_ai` (autoplay).
    SetAi { side: SideId, enabled: bool },
    /// `set_hold` (NT11, "hold ground").
    SetHold { side: SideId, enabled: bool },
    /// `begin_deployment` (F5a).
    BeginDeployment,
    /// `deploy_unit` (F5a).
    DeployUnit {
        unit: u32,
        x: f64,
        z: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        facing: Option<f64>,
        /// CB1: frontage set by a right-drag in deployment.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
    },
    /// `start_battle` (F5a).
    StartBattle,
    /// A player command (`issue_command`).
    Command { command: Command },
    /// `debug_ignite` (S2): a house, or the gate when `house` is `None`.
    Ignite {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        house: Option<usize>,
    },
    /// `debug_set_piece_hp` (SG1).
    PieceHp { piece: usize, hp: f64 },
}

impl ReplayAction {
    /// Applies the input to `sim` (results are those of the recording: a
    /// refused command is refused again).
    pub fn apply(&self, sim: &mut BattleSim) {
        match self {
            ReplayAction::StartHour { hour } => sim.set_start_hour(*hour),
            ReplayAction::SetAi { side, enabled } => sim.set_ai(*side, *enabled),
            ReplayAction::SetHold { side, enabled } => sim.set_hold(*side, *enabled),
            ReplayAction::BeginDeployment => {
                sim.begin_deployment();
            }
            ReplayAction::DeployUnit {
                unit,
                x,
                z,
                facing,
                width,
            } => {
                let _ = sim.deploy_unit_width(*unit, *x, *z, *facing, *width);
            }
            ReplayAction::StartBattle => {
                let _ = sim.start_battle();
            }
            ReplayAction::Command { command } => {
                let _ = sim.issue_command(command.clone());
            }
            ReplayAction::Ignite { house } => {
                match house {
                    Some(index) => sim.ignite_house(*index),
                    None => sim.ignite_gate(),
                };
            }
            ReplayAction::PieceHp { piece, hp } => {
                if let Some(p) = sim
                    .siege_mut()
                    .and_then(|works| works.pieces.get_mut(*piece))
                {
                    p.hp = hp.clamp(0.0, p.max_hp);
                }
            }
        }
    }
}

/// An input and the step it was issued at (after `tick` steps had run).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayEntry {
    pub tick: u64,
    #[serde(flatten)]
    pub action: ReplayAction,
}

/// Fingerprint of the state after `tick` steps (hexadecimal [`state_digest`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub tick: u64,
    pub digest: String,
}

/// End of the recorded battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayEnd {
    pub tick: u64,
    pub digest: String,
    pub outcome: BattleOutcome,
}

/// What the list of replays shows without reading the battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayHeader {
    pub format: u32,
    /// Version of the game that recorded it (information only: rule
    /// changes are caught by the fingerprints).
    pub game_version: String,
    /// Unix time of the recording (seconds), given by the caller.
    #[serde(default)]
    pub recorded_at: u64,
    /// French title (« Bataille de Guyenne »), given by the caller.
    #[serde(default)]
    pub title: String,
    pub attacker: String,
    pub defender: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_side: Option<SideId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner: Option<SideId>,
    /// Battle seconds recorded.
    pub duration: f64,
}

/// A recorded battle (file `data/schemas/battle_replay.schema.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleReplay {
    pub header: ReplayHeader,
    pub start: ReplayStart,
    /// Fingerprint right after the start (before any input).
    pub initial_digest: String,
    pub actions: Vec<ReplayEntry>,
    pub checkpoints: Vec<Checkpoint>,
    /// Last step recorded (the battle may have been left unfinished).
    pub last_tick: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<ReplayEnd>,
}

/// Why a replay cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    /// Not a replay file (French detail).
    Invalid(String),
    /// A format this build does not read.
    Format(u32),
    /// The battle cannot be rebuilt (unknown data, empty side...).
    Build(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::Invalid(detail) => write!(f, "fichier de rejeu illisible : {detail}"),
            ReplayError::Format(found) if *found > REPLAY_FORMAT => write!(
                f,
                "ce rejeu vient d'une version plus récente du jeu (format {found}, \
                 celui-ci lit le format {REPLAY_FORMAT})"
            ),
            ReplayError::Format(found) => write!(
                f,
                "ce rejeu vient d'une version trop ancienne du jeu (format {found}, \
                 celui-ci lit le format {REPLAY_FORMAT})"
            ),
            ReplayError::Build(detail) => {
                write!(
                    f,
                    "la bataille de ce rejeu ne peut être reconstruite : {detail}"
                )
            }
        }
    }
}

impl std::error::Error for ReplayError {}

impl BattleReplay {
    /// JSON text of the file.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a replay always serialises")
    }

    /// Reads a replay file, refusing other formats.
    pub fn from_json(text: &str) -> Result<BattleReplay, ReplayError> {
        let raw: serde_json::Value =
            serde_json::from_str(text).map_err(|e| ReplayError::Invalid(e.to_string()))?;
        let format = raw
            .get("header")
            .and_then(|h| h.get("format"))
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| ReplayError::Invalid("pas d'en-tête de format".to_owned()))?;
        if format != u64::from(REPLAY_FORMAT) {
            return Err(ReplayError::Format(format as u32));
        }
        serde_json::from_value(raw).map_err(|e| ReplayError::Invalid(e.to_string()))
    }

    /// Only the header (list of replays), whatever the format.
    pub fn header_from_json(text: &str) -> Result<ReplayHeader, ReplayError> {
        #[derive(Deserialize)]
        struct HeaderOnly {
            header: ReplayHeader,
        }
        serde_json::from_str::<HeaderOnly>(text)
            .map(|h| h.header)
            .map_err(|e| ReplayError::Invalid(e.to_string()))
    }

    /// Battle seconds recorded.
    pub fn duration(&self) -> f64 {
        self.last_tick as f64 * DT
    }
}

/// Fingerprint of everything the rules act on: time, random stream, and
/// each regiment's position, strength, morale, fatigue, ammunition and
/// state. Stable for a given build; any rule change shows up in it sooner
/// or later.
pub fn state_digest(sim: &BattleSim) -> u64 {
    let mut hash = Fnv::new();
    hash.u64(sim.ticks());
    hash.u64(sim.rng.clone().next_u64());
    hash.u64(u64::from(sim.is_finished()));
    hash.u64(match sim.winner() {
        None => 0,
        Some(SideId::Attacker) => 1,
        Some(SideId::Defender) => 2,
    });
    for unit in sim.units() {
        hash.u64(u64::from(unit.id));
        for value in [
            unit.hp,
            unit.x,
            unit.z,
            unit.facing,
            unit.morale,
            unit.fatigue,
        ] {
            hash.u64(value.to_bits());
        }
        hash.u64(u64::from(unit.ammo));
        hash.u64(u64::from(unit.left_field));
        hash.bytes(format!("{:?}", unit.state).as_bytes());
        // CB1: a dragged width and a grouped pace, only when set (the
        // digests of replays recorded before CB1 are unchanged).
        if let Some(files) = unit.line_files {
            hash.u64(0x4c46);
            hash.u64(u64::from(files));
        }
        if unit.match_speed || unit.group_tag.is_some() {
            hash.u64(0x4754);
            hash.u64(u64::from(unit.match_speed));
            hash.u64(unit.group_tag.map_or(u64::MAX, u64::from));
        }
        // CB2: the mode flags are not hashed. They follow from recorded
        // commands and the deterministic AI, and what they change (pace,
        // pursuit, step back, shots) shows in the positions, soldiers and
        // states above; hashing the flags would report replays recorded
        // before CB2 as diverging (the defensive AI now puts its line on
        // guard) while the battle they show has not changed.
        // CB-M3: queued orders, only when there are some (the digests of
        // replays recorded before CB are unchanged).
        if !unit.order_queue.is_empty() {
            hash.u64(unit.order_queue.len() as u64);
            for order in &unit.order_queue {
                match order {
                    QueuedOrder::Move {
                        x,
                        z,
                        facing,
                        run,
                        width,
                        match_speed,
                        group_tag,
                    } => {
                        hash.u64(1);
                        hash.u64(x.to_bits());
                        hash.u64(z.to_bits());
                        hash.u64(facing.map_or(u64::MAX, f64::to_bits));
                        hash.u64(u64::from(*run));
                        // CB1: only when set (digests of CB-M3 replays kept).
                        if width.is_some() || *match_speed || group_tag.is_some() {
                            hash.u64(width.map_or(u64::MAX, f64::to_bits));
                            hash.u64(u64::from(*match_speed));
                            hash.u64(group_tag.map_or(u64::MAX, u64::from));
                        }
                    }
                    QueuedOrder::Attack { target, run } => {
                        hash.u64(2);
                        hash.u64(u64::from(*target));
                        hash.u64(u64::from(*run));
                    }
                }
            }
        }
    }
    hash.0
}

fn digest_hex(sim: &BattleSim) -> String {
    format!("{:016x}", state_digest(sim))
}

/// FNV-1a, 64 bits.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }
}

/// Records a live battle: the bridge reports every input and calls
/// [`ReplayRecorder::observe`] after advancing time.
#[derive(Debug, Clone)]
pub struct ReplayRecorder {
    replay: BattleReplay,
    checkpoint_ticks: u64,
    next_checkpoint: u64,
}

impl ReplayRecorder {
    /// Starts recording `sim`, just built from `start`.
    pub fn new(start: ReplayStart, sim: &BattleSim) -> Self {
        let rules = ReplayRules::bundled();
        let checkpoint_ticks = ReplayRules::period_ticks(rules.checkpoint_seconds);
        let header = ReplayHeader {
            format: REPLAY_FORMAT,
            game_version: env!("CARGO_PKG_VERSION").to_owned(),
            recorded_at: 0,
            title: String::new(),
            attacker: start.setup.attacker.faction_name.clone(),
            defender: start.setup.defender.faction_name.clone(),
            player_side: start.setup.player_side,
            winner: None,
            duration: 0.0,
        };
        ReplayRecorder {
            replay: BattleReplay {
                header,
                initial_digest: digest_hex(sim),
                start,
                actions: Vec::new(),
                checkpoints: Vec::new(),
                last_tick: sim.ticks(),
                end: None,
            },
            checkpoint_ticks,
            next_checkpoint: sim.ticks() + checkpoint_ticks,
        }
    }

    /// Notes an input about to be applied to `sim` (after `sim.ticks()`
    /// steps). Inputs after the end are not kept.
    pub fn record(&mut self, sim: &BattleSim, action: ReplayAction) {
        if self.replay.end.is_some() {
            return;
        }
        self.replay.actions.push(ReplayEntry {
            tick: sim.ticks(),
            action,
        });
    }

    /// Called after time moved on: fingerprints and end of the battle.
    pub fn observe(&mut self, sim: &BattleSim) {
        if self.replay.end.is_some() {
            return;
        }
        let tick = sim.ticks();
        self.replay.last_tick = tick;
        if sim.is_finished() {
            if let Some(outcome) = sim.outcome() {
                self.replay.header.winner = Some(outcome.winner);
                self.replay.end = Some(ReplayEnd {
                    tick,
                    digest: digest_hex(sim),
                    outcome,
                });
            }
        } else if tick >= self.next_checkpoint {
            self.replay.checkpoints.push(Checkpoint {
                tick,
                digest: digest_hex(sim),
            });
            self.next_checkpoint = tick + self.checkpoint_ticks;
        }
        self.replay.header.duration = self.replay.duration();
    }

    /// The recording so far.
    pub fn replay(&self) -> &BattleReplay {
        &self.replay
    }

    /// The recording, titled and dated by the caller.
    pub fn finish(&self, title: &str, recorded_at: u64) -> BattleReplay {
        let mut replay = self.replay.clone();
        replay.header.title = title.to_owned();
        replay.header.recorded_at = recorded_at;
        replay.header.duration = replay.duration();
        replay
    }
}

/// The re-simulation no longer matches the recording.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Divergence {
    /// Step where the mismatch was seen (0: from the start).
    pub tick: u64,
    /// Battle seconds.
    pub time: f64,
    /// French explanation for the player.
    pub message: String,
}

#[derive(Debug, Clone)]
struct Keyframe {
    tick: u64,
    cursor: usize,
    sim: BattleSim,
}

/// Plays a [`BattleReplay`] back on a [`BattleSim`] owned by the caller
/// (the bridge keeps reading the same simulation object as in battle).
#[derive(Debug, Clone)]
pub struct ReplayPlayer {
    replay: BattleReplay,
    /// Next input to apply.
    cursor: usize,
    /// Next fingerprint to check.
    checkpoint: usize,
    accumulator: f64,
    keyframes: Vec<Keyframe>,
    keyframe_ticks: u64,
    divergence: Option<Divergence>,
}

/// Steps at most per [`ReplayPlayer::advance`] call (×8 at a low frame
/// rate stays well below).
const MAX_STEPS_PER_ADVANCE: u32 = 600;

impl ReplayPlayer {
    /// Rebuilds the battle of `replay` and applies its opening inputs
    /// (deployment). Returns the player and the simulation to show.
    pub fn load(replay: BattleReplay) -> Result<(ReplayPlayer, BattleSim), ReplayError> {
        if replay.header.format != REPLAY_FORMAT {
            return Err(ReplayError::Format(replay.header.format));
        }
        let mut sim = replay.start.build().map_err(ReplayError::Build)?;
        let rules = ReplayRules::bundled();
        let period = ReplayRules::period_ticks(rules.keyframe_seconds);
        let spread = replay
            .last_tick
            .div_ceil(rules.max_keyframes.max(2) as u64 - 1);
        let mut player = ReplayPlayer {
            replay,
            cursor: 0,
            checkpoint: 0,
            accumulator: 0.0,
            keyframes: Vec::new(),
            keyframe_ticks: period.max(spread).max(1),
            divergence: None,
        };
        if digest_hex(&sim) != player.replay.initial_digest {
            player.divergence = Some(Divergence {
                tick: 0,
                time: 0.0,
                message: "Ce rejeu a été enregistré avec d'autres règles du jeu : dès le \
                          début, la bataille ne se rejoue plus comme elle a été livrée."
                    .to_owned(),
            });
        }
        player.apply_pending(&mut sim);
        player.keyframes.push(Keyframe {
            tick: sim.ticks(),
            cursor: player.cursor,
            sim: sim.clone(),
        });
        Ok((player, sim))
    }

    pub fn replay(&self) -> &BattleReplay {
        &self.replay
    }

    /// Battle seconds recorded.
    pub fn duration(&self) -> f64 {
        self.replay.duration()
    }

    /// First mismatch with the recording, if any.
    pub fn divergence(&self) -> Option<&Divergence> {
        self.divergence.as_ref()
    }

    /// `true` once the recording is played to its last step.
    pub fn at_end(&self, sim: &BattleSim) -> bool {
        sim.ticks() >= self.replay.last_tick || sim.is_finished()
    }

    /// Applies the inputs issued at the current step.
    fn apply_pending(&mut self, sim: &mut BattleSim) {
        while let Some(entry) = self.replay.actions.get(self.cursor) {
            if entry.tick > sim.ticks() {
                break;
            }
            entry.action.apply(sim);
            self.cursor += 1;
        }
    }

    /// Runs one step: the fingerprint of this step is checked before the
    /// inputs issued at it are applied (the recorder took it the same way).
    pub fn step(&mut self, sim: &mut BattleSim) -> bool {
        if self.at_end(sim) {
            return false;
        }
        let before = sim.ticks();
        sim.step();
        let tick = sim.ticks();
        if tick == before {
            // Still deploying (a recording left before the battle began).
            return false;
        }
        self.check(sim, tick);
        self.apply_pending(sim);
        if tick.is_multiple_of(self.keyframe_ticks)
            && self.keyframes.last().is_none_or(|k| k.tick < tick)
        {
            self.keyframes.push(Keyframe {
                tick,
                cursor: self.cursor,
                sim: sim.clone(),
            });
        }
        true
    }

    fn check(&mut self, sim: &BattleSim, tick: u64) {
        while let Some(point) = self.replay.checkpoints.get(self.checkpoint) {
            if point.tick > tick {
                break;
            }
            if point.tick == tick && point.digest != digest_hex(sim) {
                self.diverge(tick);
            }
            self.checkpoint += 1;
        }
        if self.replay.end.is_none() && sim.is_finished() && tick < self.replay.last_tick {
            self.diverge(tick);
        }
        if let Some(end) = &self.replay.end {
            let early_end = sim.is_finished() && tick < end.tick;
            let reached = tick == end.tick
                && (!sim.is_finished()
                    || end.digest != digest_hex(sim)
                    || sim.outcome().as_ref() != Some(&end.outcome));
            if early_end || reached {
                self.diverge(tick);
            }
        }
    }

    fn diverge(&mut self, tick: u64) {
        if self.divergence.is_some() {
            return;
        }
        let time = tick as f64 * DT;
        self.divergence = Some(Divergence {
            tick,
            time,
            message: format!(
                "Les règles du jeu ont changé depuis l'enregistrement : à partir de {}, \
                 ce rejeu ne montre plus la bataille telle qu'elle a été livrée.",
                clock_fr(time)
            ),
        });
    }

    /// RJ-b: the part of the next recorded step already elapsed (see
    /// [`BattleSim::step_fraction`]); 1 at the end of the recording.
    pub fn step_fraction(&self, sim: &BattleSim) -> f64 {
        if self.at_end(sim) || !sim.can_step() {
            return 1.0;
        }
        (self.accumulator / DT).clamp(0.0, 1.0)
    }

    /// Advances playback by `dt` battle seconds; returns the steps run.
    pub fn advance(&mut self, sim: &mut BattleSim, dt: f64) -> u32 {
        if !dt.is_finite() || dt <= 0.0 {
            return 0;
        }
        self.accumulator += dt;
        let mut steps = 0;
        while self.accumulator >= DT - 1e-9 && steps < MAX_STEPS_PER_ADVANCE {
            self.accumulator -= DT;
            if !self.step(sim) {
                self.accumulator = 0.0;
                break;
            }
            steps += 1;
        }
        if steps == MAX_STEPS_PER_ADVANCE {
            self.accumulator = 0.0;
        }
        steps
    }

    /// Jumps to `seconds` (clamped to the recording): back from the nearest
    /// earlier copy, forward by simulating. Returns the steps simulated.
    pub fn seek(&mut self, sim: &mut BattleSim, seconds: f64) -> u64 {
        let target = ((seconds.max(0.0) / DT).round() as u64).min(self.replay.last_tick);
        if target < sim.ticks() {
            let keyframe = self
                .keyframes
                .iter()
                .rev()
                .find(|k| k.tick <= target)
                .unwrap_or(&self.keyframes[0]);
            *sim = keyframe.sim.clone();
            self.cursor = keyframe.cursor;
            self.checkpoint = self
                .replay
                .checkpoints
                .iter()
                .position(|c| c.tick > keyframe.tick)
                .unwrap_or(self.replay.checkpoints.len());
        }
        self.accumulator = 0.0;
        let mut steps = 0;
        while sim.ticks() < target && self.step(sim) {
            steps += 1;
        }
        steps
    }

    /// In-memory copies kept so far.
    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }
}

/// « 3 min 20 s ».
pub fn clock_fr(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    let (minutes, secs) = (total / 60, total % 60);
    if minutes == 0 {
        format!("{secs} s")
    } else {
        format!("{minutes} min {secs:02} s")
    }
}
