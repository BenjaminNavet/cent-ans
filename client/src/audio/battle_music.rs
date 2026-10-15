//! Layered battle music: port of game/scripts/battle/battle_music.gd.
//!
//! The base track is the director's `battle` playlist (see `mod.rs`); this module reads the
//! intensity of the fight ([`compute_state`]: approach, engagement, critical, victory, defeat),
//! applies the preset of `data/audio/battle_layers.json` to it (base level in dB and playback
//! speed through the director's music trim) and fades the period-instrument layers (drums,
//! straight trumpet, bagpipe drone, shawm) in and out. Rises are immediate, descents need the
//! new state to hold for `hysteresis_seconds`. The low-pass of the base track (`base_cutoff_hz`)
//! has no equivalent in `bevy_audio`: it is computed and dumped, not applied. Stingers are left to
//! `battle.rs` (`fanfare` / `choir` at the end of the battle). The `melee_din` layer is muted when
//! the spatial battle audio is active, which is always the case here.

use std::collections::BTreeMap;

use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;
use serde_json::{Value, json};

use super::{AudioDirector, AudioSettings, SILENT_DB, db_to_linear, read_json};
use crate::app::AppState;
use crate::battle::scene::BattleSession;

const CRITICAL_MORALE: f32 = 0.32;
const CRITICAL_ROUT_FRACTION: f32 = 0.3;
const DEFAULT_FADE_SECONDS: f32 = 2.0;
const DEFAULT_HYSTERESIS_SECONDS: f32 = 2.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MusicState {
    Approach,
    Engagement,
    Critical,
    Victory,
    Defeat,
}

impl MusicState {
    pub fn key(self) -> &'static str {
        match self {
            Self::Approach => "approach",
            Self::Engagement => "engagement",
            Self::Critical => "critical",
            Self::Victory => "victory",
            Self::Defeat => "defeat",
        }
    }

    /// `URGENCY` of the Godot script: a higher value rises immediately.
    fn urgency(self) -> u8 {
        match self {
            Self::Approach => 0,
            Self::Engagement => 1,
            Self::Critical => 2,
            Self::Victory | Self::Defeat => 3,
        }
    }
}

/// What the intensity needs of a regiment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MusicUnit {
    pub side: u8,
    pub engaged: bool,
    pub routing: bool,
    pub present: bool,
    pub morale: f32,
}

/// Intensity of the fight (`compute_state`): pure, testable without audio.
pub fn compute_state(units: &[MusicUnit], resolved: bool, victory: bool) -> MusicState {
    if resolved {
        return if victory {
            MusicState::Victory
        } else {
            MusicState::Defeat
        };
    }
    if !units.iter().any(|unit| unit.engaged) {
        return MusicState::Approach;
    }
    for side in 0..2u8 {
        let own: Vec<&MusicUnit> = units.iter().filter(|unit| unit.side == side).collect();
        let present: Vec<&&MusicUnit> = own.iter().filter(|unit| unit.present).collect();
        if present.is_empty() {
            continue;
        }
        let morale =
            present.iter().map(|unit| unit.morale).sum::<f32>() / present.len().max(1) as f32;
        let routing = own.iter().filter(|unit| unit.routing).count() as f32 / own.len() as f32;
        if morale < CRITICAL_MORALE || routing > CRITICAL_ROUT_FRACTION {
            return MusicState::Critical;
        }
    }
    MusicState::Engagement
}

/// State machine with hysteresis (`_advance` / `_commit`).
#[derive(Debug, Clone)]
pub struct Intensity {
    pub current: MusicState,
    pending: MusicState,
    elapsed: f32,
}

impl Default for Intensity {
    fn default() -> Self {
        Self {
            current: MusicState::Approach,
            pending: MusicState::Approach,
            elapsed: 0.0,
        }
    }
}

impl Intensity {
    /// Feeds the wanted state; returns the new state when it is committed.
    pub fn advance(&mut self, wanted: MusicState, dt: f32, hysteresis: f32) -> Option<MusicState> {
        if wanted == self.current {
            self.pending = wanted;
            self.elapsed = 0.0;
            return None;
        }
        if wanted.urgency() > self.current.urgency() {
            return Some(self.commit(wanted));
        }
        if wanted != self.pending {
            self.pending = wanted;
            self.elapsed = 0.0;
            return None;
        }
        self.elapsed += dt;
        (self.elapsed >= hysteresis).then(|| self.commit(wanted))
    }

    fn commit(&mut self, state: MusicState) -> MusicState {
        self.current = state;
        self.pending = state;
        self.elapsed = 0.0;
        state
    }
}

/// Preset of one state.
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub base_db: f32,
    pub base_cutoff_hz: f32,
    pub base_pitch: f32,
    pub layers: BTreeMap<String, f32>,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            base_db: -40.0,
            base_cutoff_hz: 20000.0,
            base_pitch: 1.0,
            layers: BTreeMap::new(),
        }
    }
}

/// `data/audio/battle_layers.json`.
#[derive(Debug, Clone, Default)]
pub struct LayerConfig {
    pub fade_seconds: f32,
    pub hysteresis_seconds: f32,
    /// Layer name -> clip relative to the asset root.
    pub layers: BTreeMap<String, String>,
    pub muted_with_battle_audio: Vec<String>,
    pub states: BTreeMap<MusicState, Preset>,
}

impl LayerConfig {
    pub fn from_json(root: &Value) -> Self {
        let number = |value: Option<&Value>, default: f32| {
            value.and_then(Value::as_f64).map_or(default, |n| n as f32)
        };
        let mut config = Self {
            fade_seconds: number(root.get("fade_seconds"), DEFAULT_FADE_SECONDS),
            hysteresis_seconds: number(root.get("hysteresis_seconds"), DEFAULT_HYSTERESIS_SECONDS),
            ..Self::default()
        };
        if let Some(layers) = root.get("layers").and_then(Value::as_object) {
            for (name, path) in layers {
                if let Some(path) = path.as_str() {
                    config
                        .layers
                        .insert(name.clone(), super::playlist::asset_relative(path));
                }
            }
        }
        config.muted_with_battle_audio = root
            .get("mute_when_battle_audio_active")
            .and_then(Value::as_array)
            .map(|names| {
                names
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        for state in [
            MusicState::Approach,
            MusicState::Engagement,
            MusicState::Critical,
            MusicState::Victory,
            MusicState::Defeat,
        ] {
            let Some(entry) = root.pointer(&format!("/states/{}", state.key())) else {
                continue;
            };
            let defaults = Preset::default();
            let mut preset = Preset {
                base_db: number(entry.get("base_db"), defaults.base_db),
                base_cutoff_hz: number(entry.get("base_cutoff_hz"), defaults.base_cutoff_hz),
                base_pitch: number(entry.get("base_pitch"), defaults.base_pitch),
                layers: BTreeMap::new(),
            };
            if let Some(levels) = entry.get("layers").and_then(Value::as_object) {
                for (name, db) in levels {
                    if let Some(db) = db.as_f64() {
                        preset.layers.insert(name.clone(), db as f32);
                    }
                }
            }
            config.states.insert(state, preset);
        }
        config
    }

    /// Target level (dB) of a layer in a state; `-80` when the preset does not list it or when
    /// the spatial battle audio replaces it.
    pub fn layer_db(&self, state: MusicState, layer: &str, battle_audio_active: bool) -> f32 {
        if battle_audio_active
            && self
                .muted_with_battle_audio
                .iter()
                .any(|name| name == layer)
        {
            return SILENT_DB;
        }
        self.states
            .get(&state)
            .and_then(|preset| preset.layers.get(layer))
            .copied()
            .unwrap_or(SILENT_DB)
    }

    pub fn preset(&self, state: MusicState) -> Preset {
        self.states.get(&state).cloned().unwrap_or_default()
    }
}

#[derive(Component)]
struct MusicLayer {
    name: String,
    db: f32,
}

/// Battle music state (resource while a battle runs).
#[derive(Resource)]
pub struct BattleMusic {
    config: LayerConfig,
    intensity: Intensity,
    transitions: Vec<&'static str>,
    layers_spawned: usize,
}

impl BattleMusic {
    fn new(config: LayerConfig) -> Self {
        Self {
            config,
            intensity: Intensity::default(),
            transitions: vec!["approach"],
            layers_spawned: 0,
        }
    }
}

pub struct BattleMusicPlugin;

impl Plugin for BattleMusicPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(AppState::Battle), end_battle_music)
            .add_systems(
                Update,
                battle_music_update
                    .after(super::tick_music)
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

fn end_battle_music(mut commands: Commands, mut director: ResMut<AudioDirector>) {
    commands.remove_resource::<BattleMusic>();
    director.music_trim_db = 0.0;
    director.music_speed = 1.0;
}

fn music_units(session: &mut BattleSession) -> (Vec<MusicUnit>, bool, bool) {
    use sim_battle::{SideId, UnitState};
    let sim = session.sim_mut();
    let units = sim
        .units()
        .iter()
        .filter(|unit| !unit.synthetic)
        .map(|unit| MusicUnit {
            side: SideId::BOTH
                .iter()
                .position(|side| *side == unit.side)
                .unwrap_or(0) as u8,
            engaged: matches!(
                unit.state,
                UnitState::Melee | UnitState::Shooting | UnitState::Charging
            ),
            routing: unit.state == UnitState::Routing,
            present: unit.present(),
            morale: unit.morale as f32,
        })
        .collect();
    let finished = sim.is_finished();
    let winner = sim.outcome().map(|outcome| outcome.winner);
    let player_side = session.player;
    let victory = match (winner, player_side) {
        (Some(winner), Some(player)) => winner == player,
        _ => false,
    };
    (units, finished, victory)
}

#[allow(clippy::too_many_arguments)]
fn battle_music_update(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut director: ResMut<AudioDirector>,
    music: Option<ResMut<BattleMusic>>,
    session: Option<ResMut<BattleSession>>,
    settings: Res<AudioSettings>,
    asset_server: Res<AssetServer>,
    mut layers: Query<(Entity, &mut MusicLayer, Option<&mut bevy::audio::AudioSink>)>,
) {
    let Some(mut session) = session else { return };
    let Some(mut music) = music else {
        let config = read_json(&director.data_dir.join("audio/battle_layers.json"))
            .map(|json| LayerConfig::from_json(&json))
            .unwrap_or_default();
        commands.insert_resource(BattleMusic::new(config));
        return;
    };
    let dt = time.delta_secs();
    let (units, finished, victory) = music_units(&mut session);
    let wanted = compute_state(&units, finished, victory);
    let hysteresis = music.config.hysteresis_seconds;
    if let Some(state) = music.intensity.advance(wanted, dt, hysteresis) {
        music.transitions.push(state.key());
    }
    let state = music.intensity.current;
    let preset = music.config.preset(state);
    // Base track: level and speed through the director (the track itself is its playlist's).
    let fade = music.config.fade_seconds.max(0.05);
    let rate = 60.0 / fade;
    let step = rate * dt;
    director.music_trim_db += (preset.base_db - director.music_trim_db).clamp(-step, step);
    director.music_speed += (preset.base_pitch - director.music_speed).clamp(-dt / fade, dt / fade);
    // Layers.
    let names: Vec<String> = music.config.layers.keys().cloned().collect();
    for name in names {
        let target = music.config.layer_db(state, &name, true);
        let existing = layers.iter_mut().find(|(_, layer, _)| layer.name == name);
        match existing {
            Some((entity, mut layer, sink)) => {
                layer.db += (target - layer.db).clamp(-step, step);
                if layer.db <= SILENT_DB + 1.0 && target <= SILENT_DB {
                    commands.entity(entity).despawn();
                } else if let Some(mut sink) = sink {
                    let volume = settings.master * settings.music * db_to_linear(layer.db);
                    sink.set_volume(Volume::Linear(volume));
                }
            }
            None => {
                if target <= SILENT_DB + 1.0 || director.muted {
                    continue;
                }
                let Some(path) = music.config.layers.get(&name) else {
                    continue;
                };
                if !director.file_exists(path) {
                    continue;
                }
                music.layers_spawned += 1;
                commands.spawn((
                    Name::new(format!("BattleMusicLayer {name}")),
                    bevy::audio::AudioPlayer::new(
                        asset_server.load::<bevy::audio::AudioSource>(path.clone()),
                    ),
                    bevy::audio::PlaybackSettings::LOOP.with_volume(Volume::SILENT),
                    MusicLayer {
                        name: name.clone(),
                        db: SILENT_DB,
                    },
                    DespawnOnExit(AppState::Battle),
                ));
            }
        }
    }
}

pub fn dump_battle_music(world: &World) -> (String, Value) {
    let (Some(music), Some(director)) = (
        world.get_resource::<BattleMusic>(),
        world.get_resource::<AudioDirector>(),
    ) else {
        return ("battle_music".into(), Value::Null);
    };
    let preset = music.config.preset(music.intensity.current);
    let mut layers = world.try_query::<&MusicLayer>();
    let playing: Vec<Value> = layers
        .as_mut()
        .map(|query| {
            query
                .iter(world)
                .map(|layer| json!({"name": layer.name, "db": (layer.db * 10.0).round() / 10.0}))
                .collect()
        })
        .unwrap_or_default();
    (
        "battle_music".into(),
        json!({
            "state": music.intensity.current.key(),
            "transitions": music.transitions,
            "base_db_target": preset.base_db,
            "base_cutoff_hz_not_applied": preset.base_cutoff_hz,
            "base_trim_db": director.music_trim_db,
            "base_speed": director.music_speed,
            "layers_configured": music.config.layers.keys().collect::<Vec<_>>(),
            "layers_spawned": music.layers_spawned,
            "layers_playing": playing,
            "layer_targets_db": music.config.layers.keys().map(|name| (name.clone(), music.config.layer_db(music.intensity.current, name, true))).collect::<BTreeMap<_, _>>(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(side: u8, engaged: bool, morale: f32) -> MusicUnit {
        MusicUnit {
            side,
            engaged,
            routing: false,
            present: true,
            morale,
        }
    }

    #[test]
    fn state_follows_contact_and_morale() {
        let calm = [unit(0, false, 1.0), unit(1, false, 1.0)];
        assert_eq!(compute_state(&calm, false, false), MusicState::Approach);
        let fight = [unit(0, true, 0.9), unit(1, false, 0.8)];
        assert_eq!(compute_state(&fight, false, false), MusicState::Engagement);
        let shaky = [unit(0, true, 0.9), unit(1, false, 0.2)];
        assert_eq!(compute_state(&shaky, false, false), MusicState::Critical);
        let mut routed = [unit(0, true, 0.9), unit(1, true, 0.9), unit(1, true, 0.9)];
        routed[1].routing = true;
        routed[2].routing = true;
        assert_eq!(compute_state(&routed, false, false), MusicState::Critical);
        assert_eq!(compute_state(&shaky, true, true), MusicState::Victory);
        assert_eq!(compute_state(&shaky, true, false), MusicState::Defeat);
    }

    #[test]
    fn rises_are_immediate_descents_wait() {
        let mut intensity = Intensity::default();
        assert_eq!(
            intensity.advance(MusicState::Critical, 0.1, 2.5),
            Some(MusicState::Critical)
        );
        assert_eq!(intensity.advance(MusicState::Engagement, 1.0, 2.5), None);
        assert_eq!(intensity.advance(MusicState::Engagement, 1.0, 2.5), None);
        assert_eq!(
            intensity.advance(MusicState::Engagement, 1.0, 2.5),
            Some(MusicState::Engagement)
        );
        // A flicker resets the wait.
        assert_eq!(intensity.advance(MusicState::Approach, 2.0, 2.5), None);
        assert_eq!(intensity.advance(MusicState::Engagement, 0.1, 2.5), None);
        assert_eq!(intensity.advance(MusicState::Approach, 2.0, 2.5), None);
    }

    #[test]
    fn config_from_the_shipped_data() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../data/audio/battle_layers.json");
        let json = read_json(&root).expect("battle_layers.json");
        let config = LayerConfig::from_json(&json);
        assert_eq!(config.layers.len(), 5);
        assert_eq!(config.states.len(), 5);
        assert_eq!(
            config.layers["drums"],
            "audio/music/battle_layers/battle_drum.ogg"
        );
        assert_eq!(
            config.layer_db(MusicState::Engagement, "drums", true),
            -10.0
        );
        assert_eq!(
            config.layer_db(MusicState::Engagement, "melee_din", true),
            SILENT_DB
        );
        assert_eq!(
            config.layer_db(MusicState::Engagement, "melee_din", false),
            -14.0
        );
        assert_eq!(config.preset(MusicState::Critical).base_pitch, 1.05);
    }
}
