//! Battle audio: spatialised one-shots, looping beds, melee fronts, shouted lines, war cry in
//! chorus, alerts, weather. Ports game/scripts/audio/battle_audio.gd and battle_voices.gd; what to
//! play is decided by the pure [`super::cues`] module, this file spawns the sounds.
//!
//! - [`PlaySfxAt`]: a bank event at a world position (other lots may send it: siege renderer,
//!   volleys). Distance culling, cooldown, instance limits and priority use the bank (`unit_size_m`,
//!   `max_distance_m`, `priority`, `max_instances`, `cooldown_s`, `layers`); panning is Bevy's
//!   spatial audio (listener on the battle camera, [`SPATIAL_SCALE`]), the distance fade is
//!   [`cues::attenuation`] (rodio's own falloff is neutralised by the scale).
//! - Beds (`beds`): melee x2, clamor, march, cavalry, fire loops at the barycentre of the nearby
//!   regiments; each melee front (`fronts`) has an emitter with close shocks; a 2D distant-battle
//!   loop from a high camera; wind / rain / thunder from the weather.
//! - Shouted lines: bark corpus (`data/voice/barks.json`) for charge, rout, rally, general down,
//!   flanked and for the player's selections and orders (`voice/barks` setting); the first charge
//!   of a side shouts its war cry in chorus (`speech/<voice>/<sha1>`, ADR 0145): up to three voices
//!   of the casting, a few tenths of a second apart, from neighbouring regiments.
//! - [`BattleAlertRaised`] (HUD): rout and fallen general cry at the place (shared dedup window
//!   with the state transitions), other alerts ping `ui_alert`.
//!
//! Buses: no bus graph in Bevy; each sound carries its [`Channel`] (sfx / voice) or the ambience
//! channel, scaled by the `Settings` volumes through [`AudioSettings`]. The Godot low-pass /
//! reverb of the far battle bus are real ([`super::farbus`]: biquad + Schroeder reverb in a wrapped
//! decoder, driven by the camera height).
//! `CENT_ANS_MUTE=1`: every decision is made and counted, nothing is spawned.

use std::collections::{BTreeMap, HashMap, VecDeque};

use bevy::audio::{AudioSinkPlayback, SpatialAudioSink, SpatialListener, SpatialScale, Volume};
use bevy::prelude::*;
use serde_json::{Value, json};
use sim_battle::{Command, FireState, PieceKind, SideId, UnitState, Weather};

use super::bank::{Claim, PoolVoice, claim};
use super::cues::{
    BarkGate, BarkRequest, Cue, Detector, Phase, PieceView, ShotView, Tuning, UnitView,
    attenuation, far_bus, far01,
};
use super::farbus::{FarBusParams, FarSource};
use super::{
    AudioDirector, AudioSettings, Channel, DuckMusic, PlaySfx, PlayVoice, SfxVoice, channel_of_bus,
    db_to_linear, read_json, speech,
};
use crate::app::AppState;
use crate::battle::hud::BattleAlertRaised;
use crate::battle::orders::{BattleSelection, input::IssueBattleCommand};
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCamera, BattleCameraState};
use crate::battle::units::volleys::ShotFeed;
use crate::settings::Settings;

/// World metres to rodio units: 200 m = 1. rodio attenuates by `1 / d^2` beyond 1 unit, so only
/// sounds far past their own range feel it; panning stays readable close up.
pub const SPATIAL_SCALE: f32 = 1.0 / 200.0;
const SPATIAL_EAR_GAP: f32 = 1.0;
const HISTORY: usize = 32;
/// Reach and size of a spatialised bark / of an army's cry (battle_voices.gd).
const BARK_MAX_DISTANCE: f32 = 240.0;
const BARK_UNIT_SIZE: f32 = 22.0;
const CRY_MAX_DISTANCE: f32 = 700.0;
const CRY_UNIT_SIZE: f32 = 60.0;
const CRY_PRIORITY: i32 = 8;
/// Length assumed for a line whose clip is not probed (s).
const BARK_LENGTH: f32 = 1.4;
const CHORUS_VOICES: usize = 3;
const CHORUS_STEP: f32 = 0.18;

/// Plays a bank event at a world position (`BattleAudio.play_at`).
#[derive(Message, Debug, Clone)]
pub struct PlaySfxAt {
    pub event: String,
    pub position: Vec3,
    pub gain_db: f32,
}

impl PlaySfxAt {
    pub fn new(event: impl Into<String>, position: Vec3) -> Self {
        Self {
            event: event.into(),
            position,
            gain_db: 0.0,
        }
    }
}

/// A spatial clip outside the bank (`audio/<clip>`): shouted lines, the chorus.
#[derive(Debug, Clone)]
struct ClipCue {
    clip: String,
    position: Vec3,
    gain_db: f32,
    unit_size: f32,
    max_distance: f32,
    priority: i32,
    pitch: f32,
    label: String,
}

#[derive(Debug, Clone)]
enum Item {
    Event(Cue),
    Clip(ClipCue),
}

/// Spatial sound with distance fade, refreshed every frame.
#[derive(Component)]
pub(super) struct Spatial3d {
    gain: f32,
    unit_size: f32,
    max_distance: f32,
    channel: Channel,
    /// Routed through the far bus (low-pass, reverb, `dry` gain): only `Bataille` events
    /// started beyond `near_distance_m`, like `battle_audio.gd` (decided once, at the start).
    far: bool,
}

impl Spatial3d {
    pub(super) fn channel(&self) -> Channel {
        self.channel
    }
}

/// Looping spatial bed (`beds`) or melee front emitter.
#[derive(Component)]
struct BedSound;

/// 2D loop of the battle weather (`ambience`).
#[derive(Component)]
pub(super) struct BattleAmbience {
    db: f32,
    target_db: f32,
}

/// State of the battle soundscape (one battle at a time).
#[derive(Resource)]
pub struct BattleAudio {
    detector: Detector,
    raw_bank: Value,
    scheduled: Vec<(f32, Item)>,
    pending_barks: Vec<BarkRequest>,
    gate: BarkGate,
    war_cries: Value,
    speeches: Value,
    speech_voices: Value,
    side_factions: [String; 2],
    player_side: Option<u8>,
    weather: Weather,
    /// Real seconds since the battle started.
    time: f32,
    elapsed: f32,
    far01: f32,
    pool: Vec<PoolVoice<Entity>>,
    last_played: HashMap<String, f32>,
    last_line: HashMap<String, String>,
    beds: HashMap<String, Entity>,
    bed_levels: BTreeMap<String, f32>,
    fronts: HashMap<String, Entity>,
    ambience: HashMap<String, Entity>,
    next_thunder: f32,
    distant_level: f32,
    selection: Vec<u32>,
    requested: BTreeMap<String, u32>,
    played: BTreeMap<String, u32>,
    alerts: BTreeMap<String, u32>,
    barks: VecDeque<Value>,
    chorus: Vec<Value>,
    recent: VecDeque<Value>,
    unknown: Vec<String>,
    finished_done: bool,
    /// Volleys read from `ShotFeed` (dump).
    shots_seen: usize,
}

impl BattleAudio {
    fn new(director: &AudioDirector, session: &BattleSession, weather: Weather) -> Self {
        let raw_bank =
            read_json(&director.data_dir.join("audio/sound_bank.json")).unwrap_or(Value::Null);
        let load = |name: &str| read_json(&director.data_dir.join(name)).unwrap_or(Value::Null);
        let tuning = Tuning::from_json(&raw_bank);
        let seed = session.seed ^ 0x4242;
        let mut detector = Detector::new(tuning, seed);
        let next_thunder = 20.0 + detector.unit_random() * 10.0;
        Self {
            detector,
            raw_bank,
            scheduled: Vec::new(),
            pending_barks: Vec::new(),
            gate: BarkGate::default(),
            war_cries: load("battle_orders/order_war_cry.json"),
            speeches: load("speeches/battle_speeches.json"),
            speech_voices: load("voice/speech_voices.json"),
            side_factions: [
                session.config.attacker.faction.clone(),
                session.config.defender.faction.clone(),
            ],
            player_side: session
                .player
                .and_then(|side| SideId::BOTH.iter().position(|other| *other == side))
                .map(|index| index as u8),
            weather,
            time: 0.0,
            elapsed: 0.0,
            far01: 0.0,
            pool: Vec::new(),
            last_played: HashMap::new(),
            last_line: HashMap::new(),
            beds: HashMap::new(),
            bed_levels: BTreeMap::new(),
            fronts: HashMap::new(),
            ambience: HashMap::new(),
            next_thunder,
            distant_level: 0.0,
            selection: Vec::new(),
            requested: BTreeMap::new(),
            played: BTreeMap::new(),
            alerts: BTreeMap::new(),
            barks: VecDeque::new(),
            chorus: Vec::new(),
            recent: VecDeque::new(),
            unknown: Vec::new(),
            finished_done: false,
            shots_seen: 0,
        }
    }

    fn bed_param(&self, name: &str, key: &str, default: f32) -> f32 {
        self.raw_bank
            .pointer(&format!("/beds/{name}/{key}"))
            .and_then(Value::as_f64)
            .map_or(default, |value| value as f32)
    }

    fn note(&mut self, event: &str, position: Vec3) {
        *self.played.entry(category(event).to_string()).or_default() += 1;
        self.recent.push_back(json!({
            "event": event,
            "category": category(event),
            "position": [position.x.round(), position.y.round(), position.z.round()],
            "t": (self.elapsed * 10.0).round() / 10.0,
        }));
        while self.recent.len() > HISTORY {
            self.recent.pop_front();
        }
    }

    fn schedule(&mut self, delay: f32, item: Item) {
        self.scheduled.push((self.elapsed + delay.max(0.0), item));
    }
}

/// Dump category of a bank event.
pub fn category(event: &str) -> &'static str {
    match event {
        "sword_clash" | "shield_bash" | "armor_hit" | "body_fall" | "contact" => "melee",
        "effort_cry" | "death_groan" | "charge_cry" | "rout_cry" | "war_cry" | "general_death" => {
            "cries"
        }
        "arrow_whistle" | "arrow_impact" | "arrow_flyby" | "bow_release" | "crossbow_release"
        | "bombard" | "stone_impact" | "trebuchet_release" => "missiles",
        "horse_neigh" | "cavalry_charge_impact" => "cavalry",
        "horn" | "drum" | "bell_toll" => "signals",
        "ram_hit" | "wall_collapse" | "boiling_oil" => "siege",
        "thunder" => "weather",
        e if e.starts_with("ui_") => "ui",
        _ => "other",
    }
}

pub struct BattleAudioPlugin;

impl Plugin for BattleAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaySfxAt>()
            .add_message::<BattleAlertRaised>()
            .add_message::<IssueBattleCommand>()
            .add_systems(OnExit(AppState::Battle), end_battle_audio)
            .add_systems(
                Update,
                (
                    begin_battle_audio,
                    attach_listener,
                    queue_alerts,
                    queue_orders,
                    battle_update,
                    handle_sfx_at,
                    attenuate_spatial,
                    drive_far_bus,
                    drive_ambience,
                )
                    .chain()
                    .after(super::handle_sfx)
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

fn end_battle_audio(mut commands: Commands) {
    commands.remove_resource::<BattleAudio>();
}

fn begin_battle_audio(
    mut commands: Commands,
    director: Res<AudioDirector>,
    session: Option<ResMut<BattleSession>>,
    audio: Option<Res<BattleAudio>>,
) {
    if audio.is_some() {
        return;
    }
    let Some(mut session) = session else { return };
    let weather = session.sim_mut().weather();
    commands.insert_resource(BattleAudio::new(&director, &session, weather));
}

/// The battle camera is the ears.
fn attach_listener(
    mut commands: Commands,
    cameras: Query<Entity, (With<BattleCamera>, Without<SpatialListener>)>,
) {
    for camera in &cameras {
        commands
            .entity(camera)
            .insert(SpatialListener::new(SPATIAL_EAR_GAP));
    }
}

/// Typed alerts of the HUD to sounds (`battle_alerts_column.gd` `CRIES`): the cry of a rout or of
/// a fallen general at the place (shared dedup with the state transitions), otherwise a ping.
fn queue_alerts(
    mut raised: MessageReader<BattleAlertRaised>,
    audio: Option<ResMut<BattleAudio>>,
    mut sfx: MessageWriter<PlaySfx>,
) {
    let Some(mut audio) = audio else {
        raised.clear();
        return;
    };
    for BattleAlertRaised(alert) in raised.read() {
        let kind = alert.kind.key();
        *audio.alerts.entry(kind.to_string()).or_default() += 1;
        let position = Vec3::new(alert.x as f32, 1.5, alert.z as f32);
        match kind {
            "rout" => audio.schedule(0.0, Item::Event(cue("rout_cry", position))),
            "general_down" => audio.schedule(0.0, Item::Event(cue("general_death", position))),
            "flanked" => {
                let own = alert
                    .side
                    .and_then(|side| SideId::BOTH.iter().position(|s| *s == side));
                if let (Some(unit), true) = (alert.unit, own.map(|o| o as u8) == audio.player_side)
                {
                    audio.pending_barks.push(BarkRequest {
                        situation: "flanked",
                        unit,
                        position,
                    });
                }
                sfx.write(PlaySfx::new("ui_alert"));
            }
            "rallied" => {}
            _ => {
                sfx.write(PlaySfx::new("ui_alert"));
            }
        }
    }
}

fn cue(event: &str, position: Vec3) -> Cue {
    Cue {
        event: event.to_string(),
        position,
        gain_db: 0.0,
        delay_s: 0.0,
    }
}

/// Order barks (`BattleVoices.on_order`).
fn queue_orders(
    mut commands: MessageReader<IssueBattleCommand>,
    audio: Option<ResMut<BattleAudio>>,
) {
    let Some(mut audio) = audio else {
        commands.clear();
        return;
    };
    for IssueBattleCommand(command) in commands.read() {
        let (situation, units) = match command {
            Command::Attack { units, .. } | Command::Pursue { units, .. } => ("attack", units),
            Command::Move { units, .. } => ("move", units),
            Command::Halt { units } => ("hold", units),
            Command::Formation { units, .. } => ("formation", units),
            Command::Withdraw { units, .. } => ("retreat", units),
            _ => continue,
        };
        // The order click (`ui_order` / `ui_order_refused`) follows the order log: `uisound.rs`.
        if let Some(unit) = units.first() {
            audio.pending_barks.push(BarkRequest {
                situation,
                unit: *unit,
                position: Vec3::ZERO,
            });
        }
    }
}

fn unit_views(session: &mut BattleSession) -> (Vec<UnitView>, f32, bool) {
    let sim = session.sim_mut();
    let views = sim
        .units()
        .iter()
        .filter(|unit| !unit.synthetic)
        .map(|unit| {
            let facing = unit.facing as f32;
            UnitView {
                id: unit.id,
                side: SideId::BOTH
                    .iter()
                    .position(|side| *side == unit.side)
                    .unwrap_or(0) as u8,
                phase: match unit.state {
                    UnitState::Marching => Phase::Marching,
                    UnitState::Charging => Phase::Charging,
                    UnitState::Melee => Phase::Melee,
                    UnitState::Shooting => Phase::Shooting,
                    UnitState::Routing => Phase::Routing,
                    UnitState::Rallied => Phase::Rallied,
                    _ => Phase::Idle,
                },
                present: unit.present(),
                soldiers: unit.soldiers(),
                ammo: unit.ammo,
                target: unit.target,
                position: Vec3::new(unit.x as f32, 1.5, unit.z as f32),
                forward: Vec3::new(facing.sin(), 0.0, facing.cos()),
                depth: unit.extent().1 as f32,
                mounted: unit.mounted,
                is_general: unit.is_general,
                siege: unit.category == data_model::UnitCategory::Siege,
                unit_type: unit.unit_type.to_string(),
                category: unit.category.key().to_string(),
            }
        })
        .collect();
    (views, sim.elapsed() as f32, sim.is_finished())
}

/// A simulation volley in world space (ground assumed flat at 1.5 m like the regiment views).
fn shot_view(shot: &sim_battle::ShotEvent) -> ShotView {
    use sim_battle::MissileKind;
    ShotView {
        kind: match shot.kind {
            MissileKind::Arrow | MissileKind::Javelin => "arrow",
            MissileKind::Bolt => "bolt",
            MissileKind::Ball => "ball",
            MissileKind::Stone => "stone",
            MissileKind::Bullet => "bullet",
        },
        from: Vec3::new(shot.from.0 as f32, 1.5, shot.from.1 as f32),
        aim: Vec3::new(shot.aim.0 as f32, 1.5, shot.aim.1 as f32),
        indirect: shot.indirect,
    }
}

/// Siege pieces, burning places and centre for the siege sounds and the fire bed.
type SiegeViews = (Vec3, Vec<PieceView>, Vec<(Vec3, f32)>);

fn siege_views(session: &mut BattleSession) -> Option<SiegeViews> {
    let works = session.sim_mut().siege()?;
    let center = Vec3::new(works.center.0 as f32, 0.0, works.center.1 as f32);
    let pieces = works
        .pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| PieceView {
            index,
            hp: piece.hp as f32,
            gate: piece.kind == PieceKind::Gate,
            middle: Vec3::new(
                ((piece.a.0 + piece.b.0) * 0.5) as f32,
                4.0,
                ((piece.a.1 + piece.b.1) * 0.5) as f32,
            ),
        })
        .collect();
    let mut fires: Vec<(Vec3, f32)> = works
        .houses
        .iter()
        .filter(|house| house.fire.state == FireState::Burning)
        .map(|house| {
            (
                Vec3::new(house.x as f32, 3.0, house.z as f32),
                house.fire.intensity as f32,
            )
        })
        .collect();
    if works.gate_fire.state == FireState::Burning {
        fires.push((center, works.gate_fire.intensity as f32));
    }
    Some((center, pieces, fires))
}

#[allow(clippy::too_many_arguments)]
fn battle_update(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut director: ResMut<AudioDirector>,
    audio: Option<ResMut<BattleAudio>>,
    session: Option<ResMut<BattleSession>>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    camera_state: Option<Res<BattleCameraState>>,
    selection: Option<Res<BattleSelection>>,
    shot_feed: Option<Res<ShotFeed>>,
    settings: Res<AudioSettings>,
    player_settings: Option<Res<Settings>>,
    asset_server: Res<AssetServer>,
    mut sfx_at: MessageWriter<PlaySfxAt>,
    mut sfx: MessageWriter<PlaySfx>,
    mut voices: MessageWriter<PlayVoice>,
    mut beds: Query<(&mut Spatial3d, &mut Transform), With<BedSound>>,
) {
    let (Some(mut audio), Some(mut session)) = (audio, session) else {
        return;
    };
    let real_dt = time.delta_secs();
    audio.time += real_dt;
    let listener = camera.iter().next().map(GlobalTransform::translation);
    let focus = camera_state
        .as_ref()
        .map_or(Vec3::ZERO, |state| state.target);
    let height = listener.map_or(120.0, |ear| ear.y);
    audio.far01 = far01(height);
    let far = audio.far01;

    let (units, elapsed, finished) = unit_views(&mut session);
    let battle_dt = (elapsed - audio.elapsed).max(0.0);
    audio.elapsed = elapsed;
    let by_id: HashMap<u32, &UnitView> = units.iter().map(|unit| (unit.id, unit)).collect();

    // Volleys reported by the simulation (`ShotFeed`, filled once per frame by the volley renderer).
    if let Some(feed) = shot_feed.as_ref()
        && feed.is_changed()
        && !finished
        && !feed.shots.is_empty()
    {
        let views: Vec<ShotView> = feed.shots.iter().map(shot_view).collect();
        audio.shots_seen += views.len();
        for decided in audio.detector.shots(&views, listener) {
            let delay = decided.delay_s;
            audio.schedule(delay, Item::Event(decided));
        }
    }

    // Transitions -> cues and barks (battle clock only).
    if battle_dt > 0.0 && !finished {
        let (cues, barks) = audio.detector.detect(&units, elapsed);
        for decided in cues {
            let delay = decided.delay_s;
            audio.schedule(delay, Item::Event(decided));
        }
        audio.pending_barks.extend(barks);
        if let Some((center, pieces, _)) = siege_views(&mut session) {
            for decided in audio.detector.siege(center, elapsed, &pieces, true) {
                audio.schedule(0.0, Item::Event(decided));
            }
        }
    }

    // Selection barks.
    if let Some(selection) = selection
        && selection.units != audio.selection
    {
        {
            let added: Vec<u32> = selection
                .units
                .iter()
                .copied()
                .filter(|id| !audio.selection.contains(id))
                .collect();
            audio.selection = selection.units.clone();
            if let Some(unit) = added.first() {
                audio.pending_barks.push(BarkRequest {
                    situation: "select",
                    unit: *unit,
                    position: Vec3::ZERO,
                });
            }
        }
    }

    // Victory shout and stingers.
    if finished && !audio.finished_done {
        audio.finished_done = true;
        let outcome = session.sim_mut().outcome();
        let player_won = outcome
            .as_ref()
            .and_then(|outcome| {
                SideId::BOTH
                    .iter()
                    .position(|side| *side == outcome.winner)
                    .map(|winner| Some(winner as u8) == audio.player_side)
            })
            .unwrap_or(false);
        sfx.write(PlaySfx::new(if player_won { "fanfare" } else { "choir" }));
        if player_won
            && audio.detector.victory_once()
            && let Some(speaker) = units
                .iter()
                .find(|unit| Some(unit.side) == audio.player_side && unit.present && !unit.siege)
        {
            audio.pending_barks.push(BarkRequest {
                situation: "victory",
                unit: speaker.id,
                position: speaker.position,
            });
        }
    }

    // Shouted lines.
    let barks_enabled = unit_barks_on(player_settings.as_deref());
    let pending: Vec<BarkRequest> = std::mem::take(&mut audio.pending_barks);
    for request in pending {
        let Some(unit) = by_id.get(&request.unit) else {
            continue;
        };
        if request.situation == "war_cry" {
            chorus(&mut audio, &director, &units, unit, barks_enabled);
            continue;
        }
        play_bark(
            &mut audio,
            &mut director,
            unit,
            &request,
            barks_enabled,
            listener,
            &mut voices,
        );
    }

    // Weather: thunder.
    if matches!(audio.weather, Weather::Rain) && elapsed >= audio.next_thunder {
        audio.next_thunder = elapsed + 35.0 + audio.detector.unit_random() * 45.0;
        let angle = audio.detector.unit_random() * std::f32::consts::TAU;
        let at = listener.unwrap_or(Vec3::ZERO)
            + Vec3::new(angle.cos() * 250.0, 120.0, angle.sin() * 250.0);
        audio.schedule(0.0, Item::Event(cue("thunder", at)));
    }

    // Due cues.
    let due: Vec<Item> = {
        let now = audio.elapsed;
        let mut due = Vec::new();
        audio.scheduled.retain(|(at, item)| {
            if *at <= now {
                due.push(item.clone());
                false
            } else {
                true
            }
        });
        due
    };
    for item in due {
        match item {
            Item::Event(cue) => {
                sfx_at.write(PlaySfxAt {
                    event: cue.event,
                    position: cue.position,
                    gain_db: cue.gain_db,
                });
            }
            Item::Clip(clip) => play_clip(
                &mut commands,
                &mut audio,
                &mut director,
                &settings,
                &asset_server,
                listener,
                clip,
            ),
        }
    }

    // Beds and fronts.
    let fires = siege_views(&mut session).map_or_else(Vec::new, |(_, _, fires)| fires);
    let (targets, distant) = audio.detector.beds(&units, focus, far, &fires);
    audio.distant_level = distant;
    let smoothing = (real_dt * audio.detector.tuning.bed_smoothing).clamp(0.0, 1.0);
    for target in targets {
        let level = {
            let slot = audio.bed_levels.entry(target.name.clone()).or_insert(0.0);
            *slot += (target.level - *slot) * smoothing;
            *slot
        };
        let position = target.center.unwrap_or(focus);
        drive_bed(
            &mut commands,
            &mut audio,
            &director,
            &settings,
            &asset_server,
            &mut beds,
            BedKey::Bed(target.name),
            level,
            position,
            real_dt,
            listener,
        );
    }
    let fronts = audio.detector.fronts(&units, listener.unwrap_or(focus));
    let (drives, shocks, dropped) = audio.detector.drive_fronts(&fronts, battle_dt, real_dt);
    for shock in shocks {
        audio.schedule(0.0, Item::Event(shock));
    }
    for drive in drives {
        drive_bed(
            &mut commands,
            &mut audio,
            &director,
            &settings,
            &asset_server,
            &mut beds,
            BedKey::Front(drive.key, drive.bed_name),
            drive.bed_level,
            drive.position,
            real_dt,
            listener,
        );
    }
    for key in dropped {
        if let Some(entity) = audio.fronts.remove(&key) {
            commands.entity(entity).despawn();
        }
    }
}

enum BedKey {
    Bed(String),
    Front(String, String),
}

/// Keeps a looping spatial bed at `level` (0..1): spawned when it becomes audible, gain and
/// position updated every frame, despawned once silent.
#[allow(clippy::too_many_arguments)]
fn drive_bed(
    commands: &mut Commands,
    audio: &mut BattleAudio,
    director: &AudioDirector,
    settings: &AudioSettings,
    asset_server: &AssetServer,
    beds: &mut Query<(&mut Spatial3d, &mut Transform), With<BedSound>>,
    key: BedKey,
    level: f32,
    position: Vec3,
    real_dt: f32,
    listener: Option<Vec3>,
) {
    let (bed_name, slot_key, is_front) = match &key {
        BedKey::Bed(name) => (name.clone(), name.clone(), false),
        BedKey::Front(front, name) => (name.clone(), front.clone(), true),
    };
    let Some(entry) = director.bank.beds.get(&bed_name) else {
        return;
    };
    let gain = db_to_linear(entry.volume_db) * level;
    let existing = if is_front {
        audio.fronts.get(&slot_key).copied()
    } else {
        audio.beds.get(&slot_key).copied()
    };
    match existing {
        Some(entity) => {
            if level <= 0.01 {
                commands.entity(entity).despawn();
                if is_front {
                    audio.fronts.remove(&slot_key);
                } else {
                    audio.beds.remove(&slot_key);
                }
            } else if let Ok((mut spatial, mut transform)) = beds.get_mut(entity) {
                spatial.gain = gain;
                // The bed glides towards its barycentre (no jump when a regiment enters or leaves).
                let step = (real_dt * 2.0).clamp(0.0, 1.0);
                transform.translation = transform.translation.lerp(position, step);
            }
        }
        None if level > 0.02 => {
            let Some(path) = director.resolve_clip("audio/", &entry.file) else {
                return;
            };
            let unit_size = audio.bed_param(&bed_name, "unit_size_m", 40.0);
            let max_distance = audio.bed_param(&bed_name, "max_distance_m", 600.0);
            let volume = director.channel_volume(settings, Channel::Sfx)
                * gain
                * listener.map_or(1.0, |ear| {
                    attenuation(ear.distance(position), unit_size, max_distance)
                });
            if director.muted {
                // Counted as audible: the dump shows which beds would play.
                let entity = commands
                    .spawn((Name::new(format!("Bed {bed_name} (muted)")),))
                    .id();
                if is_front {
                    audio.fronts.insert(slot_key, entity);
                } else {
                    audio.beds.insert(slot_key, entity);
                }
                return;
            }
            let entity = commands
                .spawn((
                    Name::new(format!("Bed {bed_name}")),
                    AudioPlayer::new(asset_server.load::<AudioSource>(path)),
                    PlaybackSettings::LOOP
                        .with_volume(Volume::Linear(volume))
                        .with_spatial(true)
                        .with_spatial_scale(SpatialScale::new(SPATIAL_SCALE)),
                    Transform::from_translation(position),
                    Spatial3d {
                        gain,
                        unit_size,
                        max_distance,
                        channel: Channel::Sfx,
                        far: false,
                    },
                    BedSound,
                    DespawnOnExit(AppState::Battle),
                ))
                .id();
            if is_front {
                audio.fronts.insert(slot_key, entity);
            } else {
                audio.beds.insert(slot_key, entity);
            }
        }
        None => {}
    }
}

/// Plays a shouted line of a regiment: spatial for the situations that ask for it, else 2D.
fn play_bark(
    audio: &mut BattleAudio,
    director: &mut AudioDirector,
    unit: &UnitView,
    request: &BarkRequest,
    enabled: bool,
    listener: Option<Vec3>,
    voices: &mut MessageWriter<PlayVoice>,
) {
    if !enabled {
        return;
    }
    let Some(situation) = director.voices.situation(request.situation) else {
        return;
    };
    let global_gap = director
        .voices
        .barks
        .get("global_gap_s")
        .and_then(Value::as_f64)
        .unwrap_or(1.0) as f32;
    let position = unit.position + Vec3::new(0.0, 0.2, 0.0);
    if situation.spatial && listener.is_some_and(|ear| ear.distance(position) > BARK_MAX_DISTANCE) {
        return;
    }
    if (audio.detector.unit_random()) > situation.chance {
        return;
    }
    let faction = audio.side_factions[usize::from(unit.side).min(1)].clone();
    let language = director
        .voices
        .language_for(&unit.unit_type, &faction, unit.is_general);
    let candidates = director
        .voices
        .bark_candidates(&language, request.situation, &unit.category);
    if candidates.is_empty() {
        return;
    }
    if !audio.gate.admit(
        request.situation,
        situation.priority,
        situation.cooldown_s,
        global_gap,
        audio.time,
        BARK_LENGTH,
    ) {
        return;
    }
    let key = format!("{language}/{}", request.situation);
    let mut index = ((audio.detector.unit_random() * candidates.len() as f32) as usize)
        .min(candidates.len() - 1);
    if candidates.len() > 1 && audio.last_line.get(&key) == Some(&candidates[index].id) {
        index = (index + 1) % candidates.len();
    }
    let line = &candidates[index];
    audio.last_line.insert(key, line.id.clone());
    audio.barks.push_back(json!({
        "id": line.id, "situation": request.situation, "unit": unit.id,
        "spatial": situation.spatial, "language": language,
    }));
    while audio.barks.len() > 24 {
        audio.barks.pop_front();
    }
    *audio.played.entry("voices".into()).or_default() += 1;
    let relative = format!("barks/{}", line.id);
    if situation.spatial {
        let clip = ClipCue {
            clip: format!("voice/{relative}"),
            position,
            gain_db: 0.0,
            unit_size: BARK_UNIT_SIZE,
            max_distance: BARK_MAX_DISTANCE,
            priority: 9,
            pitch: 1.0,
            label: line.id.clone(),
        };
        audio.schedule(0.0, Item::Clip(clip));
    } else {
        voices.write(PlayVoice {
            clip: relative,
            priority: situation.priority,
        });
    }
}

/// The first charge of a side: its war cry, repeated by up to three voices of the casting from
/// neighbouring regiments (`speech/<voice>/<sha1(text)[..12]>`).
fn chorus(
    audio: &mut BattleAudio,
    director: &AudioDirector,
    units: &[UnitView],
    crier: &UnitView,
    enabled: bool,
) {
    if !enabled {
        return;
    }
    let faction = audio.side_factions[usize::from(crier.side).min(1)].clone();
    let text = speech::war_cry_text(&audio.war_cries, &audio.speeches, &faction);
    if text.is_empty() {
        return;
    }
    let available: Vec<(String, String)> = speech::casting_voices(&audio.speech_voices, &faction)
        .into_iter()
        .map(|voice| {
            let relative = speech::speech_path(&voice, &text);
            (voice, relative)
        })
        .filter(|(_, relative)| director.resolve_clip("audio/voice/", relative).is_some())
        .collect();
    let mut allies: Vec<&UnitView> = units
        .iter()
        .filter(|unit| {
            unit.side == crier.side && unit.present && !unit.siege && unit.id != crier.id
        })
        .collect();
    allies.sort_by(|a, b| {
        a.position
            .distance_squared(crier.position)
            .total_cmp(&b.position.distance_squared(crier.position))
    });
    let mut used = Vec::new();
    let offset = usize::from(crier.side);
    for index in 0..available.len().min(CHORUS_VOICES) {
        let (voice, relative) = &available[(offset + index) % available.len()];
        let position = if index == 0 {
            crier.position
        } else {
            allies
                .get(index - 1)
                .map_or(crier.position, |ally| ally.position)
        } + Vec3::new(0.0, 0.2, 0.0);
        let pitch = [1.0, 0.97, 1.04][index % 3];
        let clip = ClipCue {
            clip: format!("voice/{relative}"),
            position,
            gain_db: -(index as f32) * 2.0,
            unit_size: CRY_UNIT_SIZE,
            max_distance: CRY_MAX_DISTANCE,
            priority: CRY_PRIORITY,
            pitch,
            label: format!("cry_{voice}"),
        };
        used.push(json!({"voice": voice, "clip": relative, "delay_s": index as f32 * CHORUS_STEP}));
        audio.schedule(index as f32 * CHORUS_STEP, Item::Clip(clip));
    }
    audio.chorus.push(json!({
        "side": crier.side, "faction": faction, "text": text,
        "voices_available": available.len(), "chorus": used,
    }));
    *audio.played.entry("voices".into()).or_default() += 1;
}

fn play_clip(
    commands: &mut Commands,
    audio: &mut BattleAudio,
    director: &mut AudioDirector,
    settings: &AudioSettings,
    asset_server: &AssetServer,
    listener: Option<Vec3>,
    clip: ClipCue,
) {
    let distance = listener.map_or(0.0, |ear| ear.distance(clip.position));
    if distance > clip.max_distance {
        return;
    }
    let Some(path) = director.resolve_clip("audio/", &clip.clip) else {
        return;
    };
    audio.note(&clip.label, clip.position);
    if director.muted {
        return;
    }
    let capacity = director.bank.max_voices.max(8);
    match claim(&audio.pool, capacity, &clip.label, clip.priority, 2) {
        Claim::Refuse => return,
        Claim::Replace(index) => {
            let victim = audio.pool.remove(index);
            commands.entity(victim.handle).despawn();
        }
        Claim::Free => {}
    }
    let gain = db_to_linear(clip.gain_db);
    let volume = director.channel_volume(settings, Channel::Voice)
        * gain
        * attenuation(distance, clip.unit_size, clip.max_distance);
    let entity = commands
        .spawn((
            Name::new(format!("Shout {}", clip.label)),
            AudioPlayer::new(asset_server.load::<AudioSource>(path)),
            PlaybackSettings::DESPAWN
                .with_volume(Volume::Linear(volume))
                .with_speed(clip.pitch)
                .with_spatial(true)
                .with_spatial_scale(SpatialScale::new(SPATIAL_SCALE)),
            Transform::from_translation(clip.position),
            Spatial3d {
                gain,
                unit_size: clip.unit_size,
                max_distance: clip.max_distance,
                channel: Channel::Voice,
                far: false,
            },
            DespawnOnExit(AppState::Battle),
        ))
        .id();
    audio.pool.push(PoolVoice {
        handle: entity,
        event: clip.label,
        priority: clip.priority,
        started: audio.time,
    });
}

/// Plays the bank events asked at a position: `layers` first, then the event's own files.
#[allow(clippy::too_many_arguments)]
fn handle_sfx_at(
    mut commands: Commands,
    mut requests: MessageReader<PlaySfxAt>,
    mut director: ResMut<AudioDirector>,
    audio: Option<ResMut<BattleAudio>>,
    settings: Res<AudioSettings>,
    asset_server: Res<AssetServer>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    alive: Query<(), With<Spatial3d>>,
    mut ducks: MessageWriter<DuckMusic>,
) {
    let Some(mut audio) = audio else {
        requests.clear();
        return;
    };
    audio.pool.retain(|voice| alive.contains(voice.handle));
    let listener = camera.iter().next().map(GlobalTransform::translation);
    for request in requests.read() {
        // Expand layers (depth-first, at most three levels).
        let mut queue = vec![(request.event.clone(), request.gain_db, 0u8)];
        while let Some((name, gain_db, depth)) = queue.pop() {
            let Some(entry) = director.bank.events.get(&name).cloned() else {
                if !audio.unknown.contains(&name) {
                    audio.unknown.push(name);
                }
                continue;
            };
            let now = audio.time;
            if audio.detector.is_duplicate(&name, request.position, now) {
                continue;
            }
            if depth < 3 {
                for layer in entry.layers.iter().rev() {
                    queue.push((layer.clone(), gain_db, depth + 1));
                }
            }
            if entry.files.is_empty() {
                continue;
            }
            let distance = listener.map_or(0.0, |ear| ear.distance(request.position));
            if distance > entry.max_distance_m {
                continue;
            }
            if entry.cooldown_s > 0.0
                && now - audio.last_played.get(&name).copied().unwrap_or(-1000.0) < entry.cooldown_s
            {
                continue;
            }
            *audio
                .requested
                .entry(category(&name).to_string())
                .or_default() += 1;
            let last = director.last_variant.get(&name).copied();
            let AudioDirector { bank, rng, .. } = &mut *director;
            let Some((index, file)) = bank.pick_variant(&name, last, rng) else {
                continue;
            };
            let file = file.to_string();
            director.last_variant.insert(name.clone(), index);
            let Some(path) = director.resolve_clip("audio/", &file) else {
                continue;
            };
            audio.last_played.insert(name.clone(), now);
            audio.note(&name, request.position);
            if entry.duck_music_db < 0.0 {
                ducks.write(DuckMusic {
                    db: entry.duck_music_db,
                    hold: entry.duck_seconds,
                });
            }
            if director.muted {
                continue;
            }
            match claim(
                &audio.pool,
                director.bank.max_voices.max(8),
                &name,
                entry.priority,
                entry.max_instances,
            ) {
                Claim::Refuse => continue,
                Claim::Replace(victim_index) => {
                    let victim = audio.pool.remove(victim_index);
                    commands.entity(victim.handle).despawn();
                }
                Claim::Free => {}
            }
            let channel = channel_of_bus(&entry.bus);
            let gain = db_to_linear(
                entry.volume_db + gain_db + entry.file_gain_db.get(&file).copied().unwrap_or(0.0),
            );
            let pitch =
                entry.pitch[0] + audio.detector.unit_random() * (entry.pitch[1] - entry.pitch[0]);
            let volume = director.channel_volume(&settings, channel)
                * gain
                * attenuation(distance, entry.unit_size_m, entry.max_distance_m);
            let far = routes_to_far_bus(&entry.bus, distance, audio.detector.tuning.near_distance);
            let mut sound = commands.spawn((
                Name::new(format!("Sfx {name}")),
                PlaybackSettings::DESPAWN
                    .with_volume(Volume::Linear(volume))
                    .with_speed(pitch)
                    .with_spatial(true)
                    .with_spatial_scale(SpatialScale::new(SPATIAL_SCALE)),
                Transform::from_translation(request.position),
                Spatial3d {
                    gain,
                    unit_size: entry.unit_size_m,
                    max_distance: entry.max_distance_m,
                    channel,
                    far,
                },
                SfxVoice { gain, channel },
                DespawnOnExit(AppState::Battle),
            ));
            if far {
                sound.insert(AudioPlayer(asset_server.load::<FarSource>(path)));
            } else {
                sound.insert(AudioPlayer::new(asset_server.load::<AudioSource>(path)));
            }
            let entity = sound.id();
            audio.pool.push(PoolVoice {
                handle: entity,
                event: name,
                priority: entry.priority,
                started: now,
            });
        }
    }
}

/// `battle_audio.gd`: a `Bataille` event heard from beyond `near_distance_m` goes through the
/// far bus (`BatailleLointain`); every other sound, and every near one, stays dry.
pub fn routes_to_far_bus(bus: &str, distance: f32, near_distance: f32) -> bool {
    bus == "Bataille" && distance > near_distance
}

/// Camera height to the shared low-pass / reverb settings of the far bus.
fn drive_far_bus(params: Res<FarBusParams>, camera: Query<&GlobalTransform, With<BattleCamera>>) {
    if let Some(ear) = camera.iter().next() {
        let bus = far_bus(far01(ear.translation().y));
        params.0.set(bus.cutoff_hz, bus.wet, bus.room_size);
    }
}

/// Distance fade and live volume of every spatial sound (sliders, camera moves).
fn attenuate_spatial(
    director: Res<AudioDirector>,
    settings: Res<AudioSettings>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    mut sounds: Query<(&Spatial3d, &GlobalTransform, &mut SpatialAudioSink)>,
) {
    let listener = camera.iter().next().map(GlobalTransform::translation);
    let dry = listener.map_or(1.0, |ear| {
        let bus = far_bus(far01(ear.y));
        bus.dry
    });
    for (spatial, transform, mut sink) in &mut sounds {
        let distance = listener.map_or(0.0, |ear| ear.distance(transform.translation()));
        let volume = director.channel_volume(&settings, spatial.channel)
            * spatial.gain
            * if spatial.far { dry } else { 1.0 }
            * attenuation(distance, spatial.unit_size, spatial.max_distance);
        sink.set_volume(Volume::Linear(volume));
    }
}

/// Weather loops (wind, strong wind, rain) and the 2D far battle heard from a high camera.
#[allow(clippy::too_many_arguments)]
fn drive_ambience(
    mut commands: Commands,
    time: Res<Time<Real>>,
    director: Res<AudioDirector>,
    settings: Res<AudioSettings>,
    asset_server: Res<AssetServer>,
    audio: Option<ResMut<BattleAudio>>,
    mut layers: Query<(&mut BattleAmbience, &mut AudioSink)>,
) {
    let Some(mut audio) = audio else { return };
    let weather = audio.weather;
    let far = audio.far01;
    let wanted = |layer: &str, offset: f32| {
        director
            .bank
            .ambience
            .get(layer)
            .map_or(offset, |entry| entry.volume_db + offset)
    };
    let mut targets: Vec<(&str, f32)> = vec![
        (
            "wind",
            wanted(
                "wind",
                (if matches!(weather, Weather::Clear | Weather::Fog) {
                    -6.0
                } else {
                    0.0
                }) + (-6.0 + 8.0 * far),
            ),
        ),
        (
            "battle_distant",
            if audio.distant_level > 0.01 {
                wanted("battle_distant", 0.0) + 20.0 * (audio.distant_level).max(0.0001).log10()
            } else {
                -80.0
            },
        ),
    ];
    if matches!(weather, Weather::Snow) {
        targets.push(("wind_strong", wanted("wind_strong", -3.0)));
    }
    if matches!(weather, Weather::Rain) {
        targets.push(("rain", wanted("rain", 0.0)));
    }
    let step = time.delta_secs() * 12.0;
    for (layer, target) in targets {
        let Some(entry) = director.bank.ambience.get(layer) else {
            continue;
        };
        if let Some(entity) = audio.ambience.get(layer).copied()
            && let Ok((mut state, mut sink)) = layers.get_mut(entity)
        {
            state.target_db = target;
            state.db += (state.target_db - state.db).clamp(-step, step);
            sink.set_volume(Volume::Linear(
                settings.master
                    * settings.ambience
                    * db_to_linear(state.db)
                    * director.dynamics.ambience_gain(),
            ));
            continue;
        }
        if audio.ambience.contains_key(layer) {
            continue;
        }
        if target <= -60.0 {
            continue;
        }
        if director.muted {
            let entity = commands
                .spawn(Name::new(format!("Ambience {layer} (muted)")))
                .id();
            audio.ambience.insert(layer.to_string(), entity);
            continue;
        }
        let Some(path) = director.resolve_clip("audio/", &entry.file) else {
            continue;
        };
        let name = Name::new(format!("Battle ambience {layer}"));
        let player = PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0));
        let state = BattleAmbience {
            db: -80.0,
            target_db: target,
        };
        // The far battle is heard through the far bus; the weather beds stay dry.
        let entity = if layer == "battle_distant" {
            commands
                .spawn((
                    name,
                    AudioPlayer(asset_server.load::<FarSource>(path)),
                    player,
                    state,
                    DespawnOnExit(AppState::Battle),
                ))
                .id()
        } else {
            commands
                .spawn((
                    name,
                    AudioPlayer::new(asset_server.load::<AudioSource>(path)),
                    player,
                    state,
                    DespawnOnExit(AppState::Battle),
                ))
                .id()
        };
        audio.ambience.insert(layer.to_string(), entity);
    }
}

/// `battle_audio` dump section: what the battle soundscape decided, by category.
pub fn dump_battle_audio(world: &World) -> (String, Value) {
    let Some(audio) = world.get_resource::<BattleAudio>() else {
        return ("battle_audio".into(), Value::Null);
    };
    let settings = world
        .get_resource::<AudioSettings>()
        .cloned()
        .unwrap_or_default();
    let muted = world
        .get_resource::<AudioDirector>()
        .is_some_and(|director| director.muted);
    let beds: BTreeMap<&String, f32> = audio
        .bed_levels
        .iter()
        .map(|(name, level)| (name, (level * 1000.0).round() / 1000.0))
        .collect();
    (
        "battle_audio".into(),
        json!({
            "muted": muted,
            "weather": format!("{:?}", audio.weather),
            "elapsed_s": audio.elapsed,
            "far01": audio.far01,
            "shots_seen": audio.shots_seen,
            "far_bus": {"cutoff_hz": far_bus(audio.far01).cutoff_hz, "wet": far_bus(audio.far01).wet, "dry_applied": far_bus(audio.far01).dry, "room_size": far_bus(audio.far01).room_size},
            "listener": world.get_resource::<BattleCameraState>().is_some(),
            "sounds_played_by_category": audio.played,
            "sounds_requested_by_category": audio.requested,
            "alerts": audio.alerts,
            "unknown_events": audio.unknown,
            "voices_busy": audio.pool.len(),
            "scheduled": audio.scheduled.len(),
            "beds": beds,
            "beds_audible": audio.beds.keys().collect::<Vec<_>>(),
            "melee_fronts": audio.fronts.len(),
            "ambience_layers": audio.ambience.keys().collect::<Vec<_>>(),
            "distant_level": audio.distant_level,
            "barks": audio.barks,
            "war_cry_chorus": audio.chorus,
            "recent": audio.recent,
            "buses": {
                "music": settings.master * settings.music,
                "effects": settings.master * settings.sfx,
                "battle_effects": settings.master * settings.battle_sfx,
                "battle_music": settings.battle_music_gain(),
                "voice": settings.master * settings.voice,
                "ambience": settings.master * settings.ambience,
            },
        }),
    )
}

/// `voice_barks` (Godot `voice/barks`): units answer, shout charges and routs; on without settings.
pub fn unit_barks_on(settings: Option<&crate::settings::Settings>) -> bool {
    settings.is_none_or(|settings| settings.voice_barks)
}

#[cfg(test)]
mod voice_barks_tests {
    use super::*;

    #[test]
    fn far_bus_only_for_distant_battle_events() {
        assert!(!routes_to_far_bus("Bataille", 30.0, 70.0));
        assert!(!routes_to_far_bus("Bataille", 70.0, 70.0));
        assert!(routes_to_far_bus("Bataille", 71.0, 70.0));
        assert!(!routes_to_far_bus("Voix", 500.0, 70.0));
        assert!(!routes_to_far_bus("Interface", 500.0, 70.0));
    }

    #[test]
    fn voice_barks_setting_gates_the_unit_lines() {
        let mut settings = crate::settings::Settings::default();
        assert!(unit_barks_on(Some(&settings)));
        assert!(unit_barks_on(None));
        settings.voice_barks = false;
        assert!(!unit_barks_on(Some(&settings)));
    }
}
