//! Music, ambience, UI/battle sounds and voices (OGG/MP3/WAV from game/assets/audio and
//! game/assets/third_party/music), driven by `data/audio` and `data/voice`.
//!
//! Ports game/scripts/audio/: `audio_director.gd` (playlists per context, shuffled bags,
//! crossfades, ducking), `sound_bank.gd` + `voice_pool.gd` + `ui_sounds.gd` (one-shot events with
//! priority / instance limits / random pitch), `campaign_ambience.gd` (layer levels, see
//! [`ambience`]) and `voice_lines.gd` (see [`voice`]). Built on `bevy_audio`: it streams the
//! decoded sources lazily and exposes a per-sink volume, which is all a crossfade needs, so
//! `bevy_kira_audio` is not used. Pure logic lives in [`playlist`], [`bank`], [`ambience`],
//! [`voice`] (unit-tested).
//!
//! Interface for other lots:
//! - [`PlaySfx`] message (bank event such as `ui_click`, `ui_order`, `sword_clash`, or a raw
//!   clip of `audio/sfx/`), [`PlayVoice`] (`barks/<id>`, `advisor/<id>`), [`PlayTurnSfx`],
//!   [`DuckMusic`].
//! - [`AudioSettings`] resource (volumes 0..1), edited by the settings lot.
//! - [`MusicHints`] (campaign lot: at war, court open, player faction) and [`AmbienceInput`]
//!   (campaign lot: terrain/zoom/season/weather under the camera).
//!
//! `CENT_ANS_MUTE=1` keeps all the logic (contexts, tracks, dump) but plays nothing.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

pub mod advisor;
pub mod ambience;
pub mod bank;
pub mod battle;
pub mod battle_music;
pub mod campaign;
pub mod clip;
pub mod cues;
pub mod dynamics;
pub mod farbus;
pub mod playlist;
pub mod speech;
pub mod uisound;
pub mod voice;

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::audio::{AudioSinkPlayback, Decodable, Volume};
use bevy::prelude::*;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::ui::kit::ButtonPressed;
use ambience::{AmbienceEnv, LAYERS};
use bank::{Claim, PoolVoice, SoundBank};
use playlist::{MusicData, Rotation, XorShift, should_crossfade};
use voice::VoiceData;

/// Environment variable: `1` = no sound at all (background runs).
pub const MUTE_ENV: &str = "CENT_ANS_MUTE";
const FADE_SECONDS: f32 = 1.5;
const SFX_CAPACITY: usize = 16;
const VOICE_CAPACITY: usize = 6;
const DUCK_ATTACK: f32 = 0.25;
const DUCK_RELEASE: f32 = 1.5;
const AMBIENCE_FADE_DB_PER_S: f32 = 10.0;
const SILENT_DB: f32 = -80.0;
const TURN_EVENT_DELAY: f32 = 0.7;

/// Player volumes, linear 0..1 (Godot defaults: `settings.gd`). Edited by the settings lot.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct AudioSettings {
    pub master: f32,
    pub music: f32,
    pub sfx: f32,
    pub voice: f32,
    pub ambience: f32,
    /// `audio/bus_BatailleMusique`: child of the music bus (multiplies `music`).
    pub battle_music: f32,
    /// `audio/bus_Bataille`: battle effects.
    pub battle_sfx: f32,
}

impl AudioSettings {
    /// Gain of the battle music layers: Master x Musique x BatailleMusique.
    pub fn battle_music_gain(&self) -> f32 {
        self.master * self.music * self.battle_music
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master: 1.0,
            music: 0.6,
            sfx: 0.8,
            voice: 0.9,
            ambience: 0.8,
            battle_music: 1.0,
            battle_sfx: 0.9,
        }
    }
}

/// Campaign facts that change the music context (set by the campaign lot).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct MusicHints {
    /// The player is at war with someone (`war` list, blended with the regional list).
    pub at_war: bool,
    /// The court panel is open (`court` list).
    pub court_open: bool,
    /// Player faction id (`fac_france`): its culture picks the regional playlist.
    /// Empty = the faction of [`crate::sim::Sim`].
    pub faction: String,
}

/// Terrain / zoom / season / weather under the campaign camera (set by the campaign lot);
/// `active = false` fades every ambience layer out.
#[derive(Resource, Debug, Clone, Default)]
pub struct AmbienceInput {
    pub active: bool,
    pub env: AmbienceEnv,
}

/// Plays a bank event (`ui_click`) or, failing that, the clip `audio/sfx/<event>`.
#[derive(Message, Debug, Clone)]
pub struct PlaySfx {
    pub event: String,
}

impl PlaySfx {
    pub fn new(event: impl Into<String>) -> Self {
        Self {
            event: event.into(),
        }
    }
}

/// Plays a voice clip: `clip` is relative to `audio/voice/` without extension
/// (`barks/fr_select_01`, `advisor/adv_peace_signed`).
#[derive(Message, Debug, Clone)]
pub struct PlayVoice {
    pub clip: String,
    pub priority: i32,
}

/// End of campaign turn: bell, then the clip of the most notable event kind (`event_sfx`).
#[derive(Message, Debug, Clone)]
pub struct PlayTurnSfx {
    pub event_kinds: Vec<String>,
}

/// Lowers the music by `db` (<= 0) for `hold` seconds (war cry, general's death, wall collapse).
#[derive(Message, Debug, Clone)]
pub struct DuckMusic {
    pub db: f32,
    pub hold: f32,
}

/// One music player (current or fading out).
#[derive(Component)]
struct MusicVoice {
    handle: Handle<AudioSource>,
    gain: f32,
    age: f32,
    fade_in: f32,
    fade_out: Option<(f32, f32)>,
    length: Option<f32>,
    probed: bool,
}

#[derive(Component)]
struct SfxVoice {
    gain: f32,
    channel: Channel,
}

#[derive(Component)]
struct AmbienceLayer {
    name: &'static str,
    db: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Channel {
    Sfx,
    Voice,
    /// The `Bataille` bus (battle effects).
    Battle,
}

/// Channel of a cue by its Godot bus name (`Voix`, `Bataille`; anything else = Interface).
fn channel_of_bus(bus: &str) -> Channel {
    match bus {
        "Voix" => Channel::Voice,
        "Bataille" => Channel::Battle,
        _ => Channel::Sfx,
    }
}

#[cfg(test)]
mod battle_bus_tests {
    use super::*;

    #[test]
    fn battle_buses_follow_their_own_sliders() {
        let settings = AudioSettings {
            master: 0.5,
            music: 0.4,
            sfx: 0.8,
            battle_music: 0.5,
            battle_sfx: 0.2,
            ..AudioSettings::default()
        };
        assert!((settings.battle_music_gain() - 0.1).abs() < 1e-6);
        let director = AudioDirector::load(true);
        assert!((director.channel_volume(&settings, Channel::Battle) - 0.1).abs() < 1e-6);
        assert!((director.channel_volume(&settings, Channel::Sfx) - 0.4).abs() < 1e-6);
        assert_eq!(channel_of_bus("Bataille"), Channel::Battle);
        assert_eq!(channel_of_bus("Voix"), Channel::Voice);
        assert_eq!(channel_of_bus("Interface"), Channel::Sfx);
    }

    #[test]
    fn muted_pools_respect_instance_caps_and_expire() {
        let mut director = AudioDirector::load(true);
        director.clock = 0.0;
        // max_instances 2: the third copy replaces the oldest instead of stacking.
        assert!(director.claim_virtual(false, "clash", "audio/none.ogg", 3, 2));
        assert!(director.claim_virtual(false, "clash", "audio/none.ogg", 3, 2));
        assert_eq!(director.virtual_sfx.len(), 2);
        assert!(director.claim_virtual(false, "clash", "audio/none.ogg", 3, 2));
        assert_eq!(director.virtual_sfx.len(), 2);
        assert_eq!(director.virtual_replaced, 1);
        // Fill the capacity with high priority sounds: a low priority one is refused.
        for index in 0..SFX_CAPACITY {
            director.claim_virtual(false, &format!("e{index}"), "audio/none.ogg", 9, 4);
        }
        assert_eq!(director.virtual_sfx.len(), SFX_CAPACITY);
        assert!(!director.claim_virtual(false, "weak", "audio/none.ogg", 1, 4));
        assert_eq!(director.virtual_refused, 1);
        // Unknown length = 1 s: after that the voices are gone.
        director.clock = 5.0;
        assert!(director.claim_virtual(false, "weak", "audio/none.ogg", 1, 4));
        assert_eq!(director.virtual_sfx.len(), 1);
    }

    #[test]
    fn dynamics_scale_the_channel_volume() {
        let settings = AudioSettings::default();
        let mut director = AudioDirector::load(true);
        let base = director.channel_volume(&settings, Channel::Battle);
        director.dynamics.update(
            dynamics::BusDemand {
                battle: 3.0,
                ..Default::default()
            },
            0.5,
        );
        assert!(director.channel_volume(&settings, Channel::Battle) < base);
        assert!(director.music_volume(&settings) <= base.max(1.0));
    }
}

#[derive(Debug, Default)]
struct Duck {
    depth_db: f32,
    level_db: f32,
    hold_left: f32,
    release_rate: f32,
}

struct ActiveTrack {
    entity: Option<Entity>,
    path: String,
}

/// Director state: data, rotation, current context/track, voice pools, logs.
#[derive(Resource)]
pub struct AudioDirector {
    music: MusicData,
    bank: SoundBank,
    voices: VoiceData,
    assets_dir: PathBuf,
    data_dir: PathBuf,
    rotation: Rotation,
    /// `music_rotation.json` beside the settings file (`None` when muted: tests and background
    /// runs must not touch the player's rotation).
    rotation_path: Option<PathBuf>,
    rng: XorShift,
    muted: bool,
    context: String,
    war_blend: String,
    active: Option<ActiveTrack>,
    cultures: HashMap<String, String>,
    last_variant: HashMap<String, usize>,
    last_play: HashMap<String, f32>,
    /// Requests per bank event / clip (dump `sfx_requested`) and those past the cooldown
    /// (`sfx_triggered`); counted while muted too.
    sfx_requested: std::collections::BTreeMap<String, u32>,
    sfx_triggered: std::collections::BTreeMap<String, u32>,
    sfx_pool: Vec<PoolVoice<Entity>>,
    voice_pool: Vec<PoolVoice<Entity>>,
    duck: Duck,
    pending: Vec<(f32, String)>,
    clock: f32,
    played: Vec<String>,
    tracks_indexed: usize,
    tracks_missing: Vec<String>,
    sfx_missing: Vec<String>,
    last_hints: MusicHints,
    fade_requests: Vec<(Entity, f32)>,
    /// Level (dB) and speed of the music set by the battle music layers (0 / 1 otherwise).
    music_trim_db: f32,
    music_speed: f32,
    /// Speech clips requested (dump).
    speech_log: Vec<String>,
    /// Gains of the bus limiter / compressors (`dynamics`), read by the volume setters.
    dynamics: dynamics::BusDynamics,
    /// Muted mode: the pools are simulated (no sound, no entity) so the instance caps and the
    /// priority replacement behave as when playing; `handle` = end time of the virtual voice.
    virtual_sfx: Vec<PoolVoice<f32>>,
    virtual_voice: Vec<PoolVoice<f32>>,
    virtual_replaced: u32,
    virtual_refused: u32,
    /// Clip durations probed from the file headers (`clip::clip_seconds`), by asset path.
    clip_lengths: HashMap<String, Option<f32>>,
}

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Beside the settings file (so `CENT_ANS_SETTINGS_PATH` redirects both).
fn rotation_file() -> Option<PathBuf> {
    crate::settings::Settings::file_path()
        .parent()
        .map(|directory| directory.join("music_rotation.json"))
}

fn read_json(path: &std::path::Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

impl AudioDirector {
    fn load(muted: bool) -> Self {
        let (assets_dir, data_dir) = match crate::paths::find_root() {
            Ok(root) => (
                crate::paths::asset_root(&root),
                crate::paths::data_dir(&root),
            ),
            Err(error) => {
                warn!("audio: {error}; audio data empty");
                (PathBuf::new(), PathBuf::new())
            }
        };
        let music = read_json(&data_dir.join("audio/music.json"))
            .map(|json| MusicData::from_json(&json))
            .unwrap_or_default();
        let bank = read_json(&data_dir.join("audio/sound_bank.json"))
            .map(|json| SoundBank::from_json(&json))
            .unwrap_or_default();
        let voices = VoiceData {
            barks: read_json(&data_dir.join("voice/barks.json")).unwrap_or(Value::Null),
            advisor: read_json(&data_dir.join("voice/advisor.json")).unwrap_or(Value::Null),
        };
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1337, |elapsed| elapsed.as_nanos() as u64);
        let mut director = Self {
            music,
            bank,
            voices,
            assets_dir,
            data_dir,
            rotation: Rotation::new(seed),
            rotation_path: None,
            rng: XorShift::new(seed ^ 0x9E37_79B9),
            muted,
            context: String::new(),
            war_blend: String::new(),
            active: None,
            cultures: HashMap::new(),
            last_variant: HashMap::new(),
            last_play: HashMap::new(),
            sfx_requested: Default::default(),
            sfx_triggered: Default::default(),
            sfx_pool: Vec::new(),
            voice_pool: Vec::new(),
            duck: Duck::default(),
            pending: Vec::new(),
            clock: 0.0,
            played: Vec::new(),
            tracks_indexed: 0,
            tracks_missing: Vec::new(),
            sfx_missing: Vec::new(),
            last_hints: MusicHints::default(),
            fade_requests: Vec::new(),
            music_trim_db: 0.0,
            music_speed: 1.0,
            speech_log: Vec::new(),
            dynamics: dynamics::BusDynamics::default(),
            virtual_sfx: Vec::new(),
            virtual_voice: Vec::new(),
            virtual_replaced: 0,
            virtual_refused: 0,
            clip_lengths: HashMap::new(),
        };
        director.index_files();
        if !muted {
            director.rotation_path = rotation_file();
            director.load_rotation();
        }
        director
    }

    fn load_rotation(&mut self) {
        let Some(path) = &self.rotation_path else {
            return;
        };
        if let Some(state) = read_json(path) {
            self.rotation.load_json(&state);
        }
    }

    /// Written after each draw (Godot: `save_rotation`).
    fn save_rotation(&self) {
        let Some(path) = &self.rotation_path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) = std::fs::write(path, self.rotation.to_json().to_string()) {
            warn!("audio: cannot save the music rotation: {error}");
        }
    }

    /// Bark / advisor lookups (language of a regiment, candidate lines) for the battle lot.
    pub fn voice_data(&self) -> &VoiceData {
        &self.voices
    }

    fn file_exists(&self, relative: &str) -> bool {
        self.assets_dir.join(relative).is_file()
    }

    /// `audio/<relative>.ogg` or `.wav` if present.
    fn resolve_clip(&self, directory: &str, relative: &str) -> Option<String> {
        ["ogg", "wav"]
            .into_iter()
            .map(|extension| format!("{directory}{relative}.{extension}"))
            .find(|path| self.file_exists(path))
    }

    /// Counts the tracks of the playlists and lists the missing files.
    fn index_files(&mut self) {
        let mut tracks: Vec<&String> = self
            .music
            .playlists
            .values()
            .flat_map(|playlist| playlist.primary.iter().chain(&playlist.fallback))
            .collect();
        tracks.sort();
        tracks.dedup();
        self.tracks_indexed = tracks.len();
        self.tracks_missing = tracks
            .into_iter()
            .filter(|track| !self.file_exists(track))
            .cloned()
            .collect();
        let mut missing: Vec<String> = self
            .bank
            .all_files()
            .into_iter()
            .filter(|file| self.resolve_clip("audio/", file).is_none())
            .map(String::from)
            .collect();
        missing.sort();
        missing.dedup();
        self.sfx_missing = missing;
    }

    fn culture_of(&mut self, faction: &str) -> String {
        if faction.is_empty() {
            return String::new();
        }
        if !self.cultures.contains_key(faction) {
            let culture = read_json(&self.data_dir.join(format!("factions/{faction}.json")))
                .and_then(|json| {
                    json.get("culture")
                        .and_then(Value::as_str)
                        .map(String::from)
                })
                .unwrap_or_default();
            self.cultures.insert(faction.to_string(), culture);
        }
        self.cultures[faction].clone()
    }

    /// Music context of a screen (`None` = leave the music as it is).
    fn context_for(
        &mut self,
        state: AppState,
        hints: &MusicHints,
        fallback_faction: &str,
    ) -> Option<String> {
        match state {
            AppState::Boot | AppState::Codex => None,
            AppState::MainMenu | AppState::FactionSelect => Some("menu".into()),
            AppState::Battle => Some("battle".into()),
            AppState::Campaign => {
                let faction = if hints.faction.is_empty() {
                    fallback_faction
                } else {
                    &hints.faction
                };
                let culture = self.culture_of(faction);
                let peace = self.music.campaign_context(&culture);
                self.war_blend = peace.clone();
                Some(if hints.court_open {
                    "court".into()
                } else if hints.at_war {
                    "war".into()
                } else {
                    peace
                })
            }
        }
    }

    fn log(&mut self, event: &str) {
        self.played.push(event.to_string());
        if self.played.len() > 16 {
            self.played.remove(0);
        }
    }

    /// Player volume of a channel times the bus dynamics (compressor, limiter).
    fn channel_volume(&self, settings: &AudioSettings, channel: Channel) -> f32 {
        settings.master
            * match channel {
                Channel::Sfx => settings.sfx,
                Channel::Voice => settings.voice,
                Channel::Battle => settings.battle_sfx,
            }
            * self.dynamics.channel_gain(channel)
    }

    fn music_volume(&self, settings: &AudioSettings) -> f32 {
        settings.master
            * settings.music
            * db_to_linear(self.duck.level_db + self.music_trim_db)
            * self.dynamics.music_gain()
    }

    /// Real duration (s) of a clip of the assets folder, probed once from its header.
    fn clip_length(&mut self, path: &str) -> Option<f32> {
        if let Some(known) = self.clip_lengths.get(path) {
            return *known;
        }
        let length = clip::clip_seconds(&self.assets_dir.join(path));
        self.clip_lengths.insert(path.to_string(), length);
        length
    }

    /// Muted mode: runs the pool policy on a virtual voice and returns whether the sound would
    /// have played. Voices end after the real clip length (1 s if unknown).
    fn claim_virtual(
        &mut self,
        voice_pool: bool,
        event: &str,
        path: &str,
        priority: i32,
        max_instances: usize,
    ) -> bool {
        let now = self.clock;
        let length = self.clip_length(path).unwrap_or(1.0);
        let (pool, capacity) = if voice_pool {
            (&mut self.virtual_voice, VOICE_CAPACITY)
        } else {
            (&mut self.virtual_sfx, SFX_CAPACITY)
        };
        pool.retain(|voice| voice.handle > now);
        match bank::claim(pool, capacity, event, priority, max_instances) {
            Claim::Refuse => {
                self.virtual_refused += 1;
                return false;
            }
            Claim::Replace(index) => {
                pool.remove(index);
                self.virtual_replaced += 1;
            }
            Claim::Free => {}
        }
        pool.push(PoolVoice {
            handle: now + length,
            event: event.to_string(),
            priority,
            started: now,
        });
        true
    }
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        // Agent runs (offscreen, shot, dump, script, bench) are always silent, env var or not.
        let agent_run =
            crate::agent::cli::Cli::parse(&std::env::args().skip(1).collect::<Vec<_>>())
                .is_ok_and(|cli| cli.is_agent_run());
        let muted = agent_run
            || std::env::var(MUTE_ENV).is_ok_and(|value| !value.is_empty() && value != "0");
        app.insert_resource(AudioDirector::load(muted))
            .init_resource::<AudioSettings>()
            .init_resource::<MusicHints>()
            .init_resource::<AmbienceInput>()
            .add_message::<PlaySfx>()
            .add_message::<PlayVoice>()
            .add_message::<PlayTurnSfx>()
            .add_message::<DuckMusic>()
            .add_message::<speech::PlaySpeech>()
            .add_message::<ButtonPressed>()
            .add_systems(
                Update,
                (
                    button_clicks,
                    follow_context,
                    handle_ducking,
                    handle_turn_sfx,
                    speech::handle_speech,
                    handle_sfx,
                    handle_voice,
                    update_bus_dynamics,
                    tick_music,
                    sync_voice_volumes,
                    tick_ambience,
                )
                    .chain(),
            )
            .add_plugins((
                battle::BattleAudioPlugin,
                battle_music::BattleMusicPlugin,
                advisor::AdvisorPlugin,
                campaign::CampaignAudioPlugin,
                uisound::UiSoundPlugin,
                farbus::FarBusPlugin,
            ))
            .register_dump(dump_audio)
            .register_dump(battle::dump_battle_audio)
            .register_dump(battle_music::dump_battle_music);
    }
}

/// Bank event of a kit button: unit cards and ability buttons of the battle bar play `ui_card`
/// (Godot `ui_feedback("card")`), every other button `ui_click`.
pub fn button_event(name: &str) -> &'static str {
    if name.starts_with("unitcard:") || name.starts_with("ability:") {
        "ui_card"
    } else {
        "ui_click"
    }
}

/// Every kit button plays `ui_click` (Godot: `AudioDirector._on_button_pressed`).
fn button_clicks(mut pressed: MessageReader<ButtonPressed>, mut sfx: MessageWriter<PlaySfx>) {
    for press in pressed.read() {
        sfx.write(PlaySfx::new(button_event(&press.name)));
    }
}

/// Picks the music context from the screen and the hints; switches playlist on change.
fn follow_context(
    mut commands: Commands,
    mut director: ResMut<AudioDirector>,
    state: Option<Res<State<AppState>>>,
    hints: Res<MusicHints>,
    sim: Option<Res<crate::sim::Sim>>,
    asset_server: Res<AssetServer>,
) {
    let Some(state) = state else { return };
    if !(state.is_changed() || hints.is_changed() || director.context.is_empty()) {
        return;
    }
    let fallback_faction = sim
        .as_ref()
        .map(|sim| sim.player.as_str().to_string())
        .unwrap_or_default();
    let Some(context) = director.context_for(*state.get(), &hints, &fallback_faction) else {
        return;
    };
    director.last_hints = hints.clone();
    if context == director.context {
        return;
    }
    director.context = context;
    director.begin_crossfade(&mut commands, &asset_server, FADE_SECONDS);
}

impl AudioDirector {
    /// Draws the next track of the current context and crossfades to it over `seconds`: the
    /// current voice is asked to fade out (applied by `tick_music`), the new one fades in.
    fn begin_crossfade(
        &mut self,
        commands: &mut Commands,
        asset_server: &AssetServer,
        seconds: f32,
    ) {
        if let Some(entity) = self.active.take().and_then(|active| active.entity) {
            self.fade_requests.push((entity, seconds));
        }
        if self.context.is_empty() {
            return;
        }
        let assets_dir = self.assets_dir.clone();
        let exists = move |path: &str| assets_dir.join(path).is_file();
        let context = self.context.clone();
        let war_blend = self.war_blend.clone();
        let next = {
            let AudioDirector {
                music, rotation, ..
            } = &mut *self;
            rotation.next_track(music, &context, &war_blend, &exists)
        };
        self.save_rotation();
        let Some(path) = next else { return };
        let entity = if self.muted {
            None
        } else {
            let handle: Handle<AudioSource> = asset_server.load(path.clone());
            Some(
                commands
                    .spawn((
                        Name::new(format!("Music {path}")),
                        AudioPlayer::new(handle.clone()),
                        PlaybackSettings::DESPAWN.with_volume(Volume::SILENT),
                        MusicVoice {
                            handle,
                            gain: db_to_linear(self.music.gain_db(&path)),
                            age: 0.0,
                            fade_in: seconds,
                            fade_out: None,
                            length: None,
                            probed: false,
                        },
                    ))
                    .id(),
            )
        };
        self.active = Some(ActiveTrack { entity, path });
    }
}

fn probe_length(source: &AudioSource) -> Option<f32> {
    use bevy::audio::Source;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        source.decoder().total_duration().map(|d| d.as_secs_f32())
    }))
    .ok()
    .flatten()
}

/// Fades the music voices, probes the track length, chains the next track (overlapping the
/// last seconds when the length is known, else once the track has ended).
fn tick_music(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut director: ResMut<AudioDirector>,
    settings: Res<AudioSettings>,
    asset_server: Res<AssetServer>,
    sources: Res<Assets<AudioSource>>,
    mut voices: Query<(Entity, &mut MusicVoice, Option<&mut AudioSink>)>,
) {
    let dt = time.delta_secs();
    for (entity, seconds) in std::mem::take(&mut director.fade_requests) {
        if let Ok((_, mut voice, _)) = voices.get_mut(entity)
            && voice.fade_out.is_none()
        {
            voice.fade_out = Some((0.0, seconds));
        }
    }
    let base = director.music_volume(&settings);
    let speed = director.music_speed;
    let crossfade = director.music.crossfade_seconds;
    let active_entity = director.active.as_ref().and_then(|active| active.entity);
    let mut chain: Option<f32> = None;
    let mut active_seen = false;
    for (entity, mut voice, sink) in &mut voices {
        voice.age += dt;
        let fade_in = if voice.fade_in > 0.0 {
            (voice.age / voice.fade_in).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let mut fade_out = 1.0;
        if let Some((elapsed, duration)) = voice.fade_out.as_mut() {
            *elapsed += dt;
            fade_out = (1.0 - *elapsed / duration.max(0.001)).clamp(0.0, 1.0);
            if fade_out <= 0.0 {
                commands.entity(entity).despawn();
                continue;
            }
        }
        if !voice.probed
            && let Some(source) = sources.get(&voice.handle)
        {
            voice.probed = true;
            voice.length = probe_length(source);
        }
        let is_active = Some(entity) == active_entity;
        active_seen |= is_active;
        if let Some(mut sink) = sink {
            sink.set_volume(Volume::Linear(base * voice.gain * fade_in * fade_out));
            if (sink.speed() - speed).abs() > 0.001 {
                sink.set_speed(speed);
            }
            if is_active
                && let Some(length) = voice.length
                && should_crossfade(sink.position().as_secs_f32(), length, crossfade)
            {
                chain = Some(crossfade);
            }
        }
    }
    // The active track is gone (ended, no overlap possible): next one with a short fade-in.
    if active_entity.is_some()
        && !active_seen
        && voices.iter().all(|(e, ..)| Some(e) != active_entity)
    {
        chain = Some(0.5);
    }
    if let Some(seconds) = chain {
        director.begin_crossfade(&mut commands, &asset_server, seconds);
    }
}

fn handle_ducking(
    mut requests: MessageReader<DuckMusic>,
    mut director: ResMut<AudioDirector>,
    time: Res<Time<Real>>,
) {
    let dt = time.delta_secs();
    let duck = &mut director.duck;
    for request in requests.read() {
        let depth = request.db.min(0.0);
        // A deeper or longer duck already running is not shortened.
        if duck.hold_left > 0.0 && depth >= duck.depth_db && request.hold <= duck.hold_left {
            continue;
        }
        duck.depth_db = duck.depth_db.min(depth).min(0.0);
        duck.hold_left = duck.hold_left.max(request.hold);
    }
    let target = if duck.hold_left > 0.0 {
        duck.depth_db
    } else {
        0.0
    };
    duck.hold_left = (duck.hold_left - dt).max(0.0);
    if target < duck.level_db {
        let rate = duck.depth_db.abs().max(1.0) / DUCK_ATTACK;
        duck.level_db = (duck.level_db - rate * dt).max(target);
        duck.release_rate = duck.depth_db.abs().max(1.0) / DUCK_RELEASE;
    } else if target > duck.level_db {
        duck.level_db = (duck.level_db + duck.release_rate.max(1.0) * dt).min(target);
        if duck.level_db >= 0.0 {
            duck.depth_db = 0.0;
        }
    }
}

fn handle_turn_sfx(
    mut turns: MessageReader<PlayTurnSfx>,
    mut sfx: MessageWriter<PlaySfx>,
    mut director: ResMut<AudioDirector>,
) {
    for turn in turns.read() {
        sfx.write(PlaySfx::new("turn_bell"));
        let kinds: Vec<&str> = turn.event_kinds.iter().map(String::as_str).collect();
        if let Some(clip) = director.bank.event_sfx(&kinds).map(String::from) {
            let at = director.clock + TURN_EVENT_DELAY;
            director.pending.push((at, clip));
        }
    }
    let now = director.clock;
    let due: Vec<String> = director
        .pending
        .iter()
        .filter(|(at, _)| *at <= now)
        .map(|(_, clip)| clip.clone())
        .collect();
    director.pending.retain(|(at, _)| *at > now);
    for clip in due {
        sfx.write(PlaySfx::new(clip));
    }
}

/// Plays one-shot events with the bank rules (cooldown, instances, priority, pitch).
#[allow(clippy::too_many_arguments)]
fn handle_sfx(
    mut commands: Commands,
    mut requests: MessageReader<PlaySfx>,
    mut director: ResMut<AudioDirector>,
    settings: Res<AudioSettings>,
    asset_server: Res<AssetServer>,
    time: Res<Time<Real>>,
    alive: Query<(), With<SfxVoice>>,
    mut ducks: MessageWriter<DuckMusic>,
) {
    director.clock += time.delta_secs();
    let now = director.clock;
    director
        .sfx_pool
        .retain(|voice| alive.contains(voice.handle));
    for request in requests.read() {
        let name = request.event.clone();
        *director.sfx_requested.entry(name.clone()).or_default() += 1;
        let entry = director.bank.events.get(&name).cloned();
        if let Some(entry) = &entry {
            let last = director.last_play.get(&name).copied().unwrap_or(-1000.0);
            if now - last < entry.cooldown_s {
                continue;
            }
        }
        director.last_play.insert(name.clone(), now);
        *director.sfx_triggered.entry(name.clone()).or_default() += 1;
        director.log(&name);
        // Resolve the clip: a bank variant, else a raw clip of audio/sfx/.
        let (path, db, pitch_span, priority, max_instances, channel, duck) = match &entry {
            Some(entry) => {
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
                let gain = entry.file_gain_db.get(&file).copied().unwrap_or(0.0);
                let channel = if entry.bus == "Voix" {
                    Channel::Voice
                } else {
                    Channel::Sfx
                };
                (
                    path,
                    entry.volume_db + gain,
                    entry.pitch,
                    entry.priority,
                    entry.max_instances,
                    channel,
                    (entry.duck_music_db, entry.duck_seconds),
                )
            }
            None => {
                let Some(path) = director.resolve_clip("audio/sfx/", &name) else {
                    continue;
                };
                (path, 0.0, [1.0, 1.0], 3, 2, Channel::Sfx, (0.0, 0.0))
            }
        };
        if director.muted {
            director.claim_virtual(false, &name, &path, priority, max_instances);
            continue;
        }
        match bank::claim(
            &director.sfx_pool,
            SFX_CAPACITY,
            &name,
            priority,
            max_instances,
        ) {
            Claim::Refuse => continue,
            Claim::Replace(index) => {
                let victim = director.sfx_pool.remove(index);
                commands.entity(victim.handle).despawn();
            }
            Claim::Free => {}
        }
        let pitch = pitch_span[0] + director.rng.unit() * (pitch_span[1] - pitch_span[0]);
        let gain = db_to_linear(db);
        let volume = director.channel_volume(&settings, channel) * gain;
        let entity = commands
            .spawn((
                Name::new(format!("Sfx {name}")),
                AudioPlayer::new(asset_server.load::<AudioSource>(path)),
                PlaybackSettings::DESPAWN
                    .with_volume(Volume::Linear(volume))
                    .with_speed(pitch),
                SfxVoice { gain, channel },
            ))
            .id();
        director.sfx_pool.push(PoolVoice {
            handle: entity,
            event: name,
            priority,
            started: now,
        });
        if duck.0 < 0.0 {
            ducks.write(DuckMusic {
                db: duck.0,
                hold: duck.1,
            });
        }
    }
}

/// Plays voice clips (`audio/voice/<clip>`) in a small priority pool.
fn handle_voice(
    mut commands: Commands,
    mut requests: MessageReader<PlayVoice>,
    mut director: ResMut<AudioDirector>,
    settings: Res<AudioSettings>,
    asset_server: Res<AssetServer>,
    alive: Query<(), With<SfxVoice>>,
) {
    let now = director.clock;
    director
        .voice_pool
        .retain(|voice| alive.contains(voice.handle));
    for request in requests.read() {
        let Some(path) = director.resolve_clip("audio/voice/", &request.clip) else {
            continue;
        };
        director.log(&format!("voice:{}", request.clip));
        if director.muted {
            director.claim_virtual(true, &request.clip, &path, request.priority, 1);
            continue;
        }
        match bank::claim(
            &director.voice_pool,
            VOICE_CAPACITY,
            &request.clip,
            request.priority,
            1,
        ) {
            Claim::Refuse => continue,
            Claim::Replace(index) => {
                let victim = director.voice_pool.remove(index);
                commands.entity(victim.handle).despawn();
            }
            Claim::Free => {}
        }
        let volume = director.channel_volume(&settings, Channel::Voice);
        let entity = commands
            .spawn((
                Name::new(format!("Voice {}", request.clip)),
                AudioPlayer::new(asset_server.load::<AudioSource>(path)),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
                SfxVoice {
                    gain: 1.0,
                    channel: Channel::Voice,
                },
            ))
            .id();
        director.voice_pool.push(PoolVoice {
            handle: entity,
            event: request.clip.clone(),
            priority: request.priority,
            started: now,
        });
    }
}

/// Measures what the live sinks ask for on each bus and steps the limiter / compressors
/// (`dynamics`). The demand is read back from the sink volumes divided by the gain applied.
#[allow(clippy::too_many_arguments)]
fn update_bus_dynamics(
    time: Res<Time<Real>>,
    mut director: ResMut<AudioDirector>,
    sfx: Query<(&SfxVoice, &AudioSink)>,
    spatial: Query<(&battle::Spatial3d, &AudioSink)>,
    music: Query<&AudioSink, With<MusicVoice>>,
    ambience: Query<&AudioSink, With<AmbienceLayer>>,
    battle_ambience: Query<&AudioSink, With<battle::BattleAmbience>>,
) {
    let floor = 0.05;
    let dynamics = &director.dynamics;
    let mut demand = dynamics::BusDemand::default();
    for (voice, sink) in &sfx {
        let gain = dynamics.channel_gain(voice.channel).max(floor);
        demand.add(voice.channel, sink.volume().to_linear() / gain);
    }
    for (voice, sink) in &spatial {
        let gain = dynamics.channel_gain(voice.channel()).max(floor);
        demand.add(voice.channel(), sink.volume().to_linear() / gain);
    }
    let gain = dynamics.music_gain().max(floor);
    demand.music = music
        .iter()
        .map(|sink| sink.volume().to_linear() / gain)
        .sum();
    let gain = dynamics.ambience_gain().max(floor);
    demand.ambience = ambience
        .iter()
        .chain(battle_ambience.iter())
        .map(|sink| sink.volume().to_linear() / gain)
        .sum();
    director.dynamics.update(demand, time.delta_secs());
}

/// Live volume changes (settings sliders) reach the sounds already playing.
fn sync_voice_volumes(
    settings: Res<AudioSettings>,
    director: Res<AudioDirector>,
    mut voices: Query<(&SfxVoice, &mut AudioSink)>,
) {
    // Every frame: the bus dynamics change the gain of the sounds already playing.
    for (voice, mut sink) in &mut voices {
        sink.set_volume(Volume::Linear(
            director.channel_volume(&settings, voice.channel) * voice.gain,
        ));
    }
}

/// Ambience layers: spawned lazily, faded in dB toward the level computed from the
/// environment (campaign lot), stopped once silent.
#[allow(clippy::too_many_arguments)]
fn tick_ambience(
    mut commands: Commands,
    time: Res<Time<Real>>,
    director: Res<AudioDirector>,
    input: Res<AmbienceInput>,
    settings: Res<AudioSettings>,
    asset_server: Res<AssetServer>,
    mut layers: Query<(Entity, &mut AmbienceLayer, Option<&mut AudioSink>)>,
) {
    if director.muted {
        return;
    }
    let dt = time.delta_secs();
    let levels = if input.active {
        ambience::layer_levels(&input.env)
    } else {
        [0.0; 8]
    };
    for (index, name) in LAYERS.iter().enumerate() {
        let level = levels[index];
        let entry = director.bank.ambience.get(*name);
        let target_db = match entry {
            Some(entry) if level > 0.005 => entry.volume_db + 20.0 * level.log10(),
            _ => SILENT_DB,
        };
        let existing = layers.iter_mut().find(|(_, layer, _)| layer.name == *name);
        match existing {
            Some((entity, mut layer, sink)) => {
                let step = AMBIENCE_FADE_DB_PER_S * dt;
                layer.db += (target_db - layer.db).clamp(-step, step);
                if layer.db <= SILENT_DB + 1.0 && target_db <= SILENT_DB {
                    commands.entity(entity).despawn();
                } else if let Some(mut sink) = sink {
                    let volume = settings.master
                        * settings.ambience
                        * db_to_linear(layer.db)
                        * director.dynamics.ambience_gain();
                    sink.set_volume(Volume::Linear(volume));
                }
            }
            None => {
                if target_db <= SILENT_DB {
                    continue;
                }
                let Some(entry) = entry else { continue };
                let Some(path) = director.resolve_clip("audio/", &entry.file) else {
                    continue;
                };
                commands.spawn((
                    Name::new(format!("Ambience {name}")),
                    AudioPlayer::new(asset_server.load::<AudioSource>(path)),
                    PlaybackSettings::LOOP.with_volume(Volume::SILENT),
                    AmbienceLayer {
                        name,
                        db: SILENT_DB,
                    },
                ));
            }
        }
    }
}

/// The `audio` dump section's JSON (integration tests).
pub fn dump_for_tests(world: &World) -> Value {
    dump_audio(world).1
}

fn dump_audio(world: &World) -> (String, Value) {
    let Some(director) = world.get_resource::<AudioDirector>() else {
        return ("audio".into(), Value::Null);
    };
    let settings = world
        .get_resource::<AudioSettings>()
        .cloned()
        .unwrap_or_default();
    let missing = |list: &Vec<String>| json!({"count": list.len(), "first": list.iter().take(20).collect::<Vec<_>>()});
    let mut contexts: Vec<(&String, usize, usize)> = director
        .music
        .playlists
        .iter()
        .map(|(name, list)| (name, list.primary.len(), list.fallback.len()))
        .collect();
    contexts.sort();
    (
        "audio".into(),
        json!({
            "muted": director.muted,
            "playlist": director.context,
            "track": director.active.as_ref().map(|active| active.path.clone()),
            "track_gain_db": director.active.as_ref().map(|active| director.music.gain_db(&active.path)),
            "volumes": {
                "master": settings.master, "music": settings.music, "sfx": settings.sfx,
                "voice": settings.voice, "ambience": settings.ambience,
            },
            "duck_db": director.duck.level_db,
            "tracks_indexed": director.tracks_indexed,
            "tracks_missing": missing(&director.tracks_missing),
            "sfx_files_missing": missing(&director.sfx_missing),
            "bank_events": director.bank.events.len(),
            "playlists": contexts.iter().map(|(name, primary, fallback)| json!({"context": name, "primary": primary, "fallback": fallback})).collect::<Vec<_>>(),
            "sfx_voices": director.sfx_pool.len(),
            "virtual_pools": {
                "sfx": director.virtual_sfx.len(), "voice": director.virtual_voice.len(),
                "replaced": director.virtual_replaced, "refused": director.virtual_refused,
            },
            "dynamics": {
                "limiter_gain": director.dynamics.limiter_gain(),
                "voice": director.dynamics.channel_gain(Channel::Voice),
                "battle": director.dynamics.channel_gain(Channel::Battle),
                "ambience": director.dynamics.ambience_gain(),
                "music": director.dynamics.music_gain(),
            },
            "voice_voices": director.voice_pool.len(),
            "recent_events": director.played,
            "sfx_requested": director.sfx_requested,
            "sfx_triggered": director.sfx_triggered,
            "crossfade_seconds": director.music.crossfade_seconds,
            "rotation_file": director.rotation_path.as_ref().map(|path| path.display().to_string()),
            "speech_requested": director.speech_log,
            "rotation_last_track": director.rotation.current,
        }),
    )
}
