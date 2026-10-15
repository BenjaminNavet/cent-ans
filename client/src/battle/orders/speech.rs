//! The general's speech before the battle (screen and text part).
//!
//! Ports game/scripts/battle/battle_speech.gd. The text is composed from
//! `data/speeches/battle_speeches.json` (faction, general, balance of forces, terrain, weather;
//! deterministic choices from the seed of the battle), then the faction's war cry
//! (`data/battle_orders/order_war_cry.json`). It is shown as a subtitle band (top centre, under
//! the balance bar) while the camera travels along the player's lines; the war cry pulls the
//! camera back and is shown large. Each sentence and the cry send an `audio::speech::PlaySpeech`
//! (voice, ducking and clip path are the audio director's); the cry also plays the `war_cry`
//! effect. Escape, Enter, Space or a click skip it (the enemy general's speech is then skipped
//! too); the camera is given back as it was. Played when the deployment opens (the simulation is
//! frozen); without a deployment phase the battle is paused for the length of the speech.
//!
//! Not played: AI against AI, replays, `CENT_ANS_NO_SPEECH=1`, nor (to keep the scripted proofs
//! of the other lots running) with `CENT_ANS_NO_DEPLOY` / `CENT_ANS_ORDERS_DEMO` unless
//! `CENT_ANS_SPEECH=1` forces it. Dump section `battle_speech`.
//!
//! Deviations: the sentence is picked with an FNV-1a hash of `seed/side/salt` (GDScript's `hash`
//! is not reproducible), so the choices differ from Godot's; the length of a sentence is the
//! written one or the estimate of the voice (`speech::spoken_seconds`) if the faction has a
//! voice (Godot probes the clip).

use bevy::prelude::*;
use serde_json::Value;
use sim_battle::SideId;

use super::deploy::Deployment;
use crate::audio::battle::PlaySfxAt;
use crate::audio::speech::{PlaySpeech, spoken_seconds};
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCameraState;
use crate::ui::kit::UiTheme;
use crate::ui::kit::style::UiTier;

const CAMERA_DISTANCE: f32 = 34.0;
const CRY_DISTANCE: f32 = 95.0;
/// Top offset (px) of the band, under the balance bar.
const SUBTITLE_TOP: f32 = 76.0;

/// A composed speech (text only).
#[derive(Debug, Clone, PartialEq)]
pub struct Composed {
    pub lines: Vec<String>,
    pub cry: String,
    pub speaker: String,
    pub line_seconds: f32,
    pub cry_seconds: f32,
    pub faction: String,
    pub general_id: String,
}

/// FNV-1a 64 of `text`.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// `section[key]`, else `section.default` (empty if neither).
fn by_key(data: &Value, section: &str, key: &str) -> Vec<String> {
    let table = &data[section];
    let found = strings(&table[key]);
    if found.is_empty() {
        strings(&table["default"])
    } else {
        found
    }
}

/// Speech of a side. `ratio` = our men / theirs. `None` without data.
#[allow(clippy::too_many_arguments)]
pub fn compose(
    data: &Value,
    war_cries: &Value,
    faction: &str,
    general: Option<(&str, &str)>,
    ratio: f32,
    terrain_key: &str,
    weather_key: &str,
    seed: u64,
    side_tag: &str,
) -> Option<Composed> {
    if !data.is_object() {
        return None;
    }
    let pick = |list: &[String], salt: &str| -> String {
        if list.is_empty() {
            return String::new();
        }
        let hash = fnv(&format!("{seed}/{side_tag}/{salt}"));
        list[(hash % list.len() as u64) as usize].clone()
    };
    let (general_character, general_name) = general.unwrap_or(("", ""));
    let mut out = vec![pick(&by_key(data, "openings", faction), "open")];
    if !general_name.is_empty() {
        out.push(
            pick(&strings(&data["general_lines"]), "general").replace("{general}", general_name),
        );
    }
    let strong = data["odds_thresholds"]["strong"].as_f64().unwrap_or(1.3) as f32;
    let weak = data["odds_thresholds"]["weak"].as_f64().unwrap_or(0.77) as f32;
    let odds = if ratio >= strong {
        "strong"
    } else if ratio <= weak {
        "weak"
    } else {
        "even"
    };
    out.push(pick(&strings(&data["odds"][odds]), "odds"));
    out.push(pick(&by_key(data, "terrain", terrain_key), "terrain"));
    let weather = strings(&data["weather"][weather_key]);
    if !weather.is_empty() {
        out.push(pick(&weather, "weather"));
    }
    out.push(pick(&by_key(data, "closings", faction), "close"));
    let lines: Vec<String> = out.into_iter().filter(|l| !l.is_empty()).collect();
    let cry = war_cries["labels_by_faction"][faction]
        .as_str()
        .or_else(|| data["default_cry"].as_str())
        .unwrap_or("En avant !")
        .to_owned();
    Some(Composed {
        lines,
        cry,
        speaker: if general_name.is_empty() {
            data["speaker_without_general"]
                .as_str()
                .unwrap_or("le capitaine")
                .to_owned()
        } else {
            general_name.to_owned()
        },
        line_seconds: data["line_seconds"].as_f64().unwrap_or(4.6) as f32,
        cry_seconds: data["cry_seconds"].as_f64().unwrap_or(3.0) as f32,
        faction: faction.to_owned(),
        general_id: if general_character.is_empty() {
            general_name.to_owned()
        } else {
            general_character.to_owned()
        },
    })
}

/// Start time and length of each sentence: `max(written length, voice estimate)` when the
/// faction has a voice.
pub fn schedule(lines: &[String], line_seconds: f32, voiced: bool) -> (Vec<f32>, Vec<f32>) {
    let mut starts = Vec::new();
    let mut durations = Vec::new();
    let mut t = 0.0;
    for line in lines {
        let length = if voiced {
            line_seconds.max(spoken_seconds(line))
        } else {
            line_seconds
        };
        starts.push(t);
        durations.push(length);
        t += length;
    }
    (starts, durations)
}

/// Point of the travelling path at `u` in 0..=1 (linear between the regiments).
pub fn point_at(path: &[Vec3], u: f32) -> Vec3 {
    match path {
        [] => Vec3::ZERO,
        [only] => *only,
        _ => {
            let x = u.clamp(0.0, 1.0) * (path.len() - 1) as f32;
            let i = (x as usize).min(path.len() - 2);
            path[i].lerp(path[i + 1], x - i as f32)
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The speech being played.
#[derive(Debug)]
struct Playing {
    speech: Composed,
    side: SideId,
    path: Vec<Vec3>,
    yaw: f32,
    t: f32,
    starts: Vec<f32>,
    durations: Vec<f32>,
    cry_s: f32,
    spoken: Option<usize>,
    cried: bool,
    saved: (Vec3, f32, f32),
}

impl Playing {
    fn speaking_seconds(&self) -> f32 {
        match (self.starts.last(), self.durations.last()) {
            (Some(start), Some(length)) => start + length,
            _ => 0.0,
        }
    }

    fn total_seconds(&self) -> f32 {
        self.speaking_seconds() + self.cry_s
    }

    fn line_index(&self, t: f32) -> usize {
        self.starts
            .iter()
            .rposition(|start| t >= *start)
            .unwrap_or(0)
            .min(self.speech.lines.len().saturating_sub(1))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Stage {
    #[default]
    Waiting,
    Player,
    Enemy,
    Done,
}

/// State of the speeches of this battle.
#[derive(Resource, Debug, Default)]
pub struct Speech {
    stage: Stage,
    playing: Option<Playing>,
    /// The player skipped his speech (the enemy's is then not played).
    pub skipped: bool,
    /// Text on screen: (speaker, line, cry visible).
    pub shown: Option<(String, String, bool)>,
    /// Every sentence and cry said, in order (`PlaySpeech` sent), for the dump.
    pub said: Vec<String>,
    /// Speeches played to the end or skipped.
    pub finished: u32,
    /// The battle was paused for the speeches; the value to give back to `paused`.
    pause_restore: Option<bool>,
}

impl Speech {
    /// A speech is on screen.
    pub fn active(&self) -> bool {
        self.playing.is_some()
    }
}

#[derive(Component)]
pub struct SpeechRoot;
#[derive(Component)]
pub struct SpeechBand;
#[derive(Component)]
pub struct SpeechSpeaker;
#[derive(Component)]
pub struct SpeechLine;
#[derive(Component)]
pub struct SpeechCry;

fn env_set(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v != "0")
}

fn disabled() -> bool {
    if env_set("CENT_ANS_NO_SPEECH") {
        return true;
    }
    (env_set("CENT_ANS_NO_DEPLOY") || env_set("CENT_ANS_ORDERS_DEMO"))
        && !env_set("CENT_ANS_SPEECH")
}

fn read_data(name: &str) -> Value {
    crate::paths::find_root()
        .ok()
        .and_then(|root| std::fs::read_to_string(crate::paths::data_dir(&root).join(name)).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

fn voice_cast(faction: &str, general_id: &str) -> bool {
    let casting = read_data("voice/speech_voices.json");
    !crate::audio::speech::speech_voice(&casting, faction, general_id).is_empty()
}

/// Composes and opens the speech of `side`.
fn open(
    session: &mut BattleSession,
    state: &mut Speech,
    camera: &BattleCameraState,
    side: SideId,
    deploying: bool,
) -> bool {
    let seed = session.seed;
    let sim = session.sim_mut();
    let own = sim.strength(side) as f32;
    let theirs = (sim.strength(side.other()) as f32).max(1.0);
    let setup = sim.setup();
    let side_setup = setup.side(side);
    let general = side_setup
        .general
        .as_ref()
        .map(|g| (g.character.as_str(), g.name.as_str()));
    let Some(speech) = compose(
        &read_data("speeches/battle_speeches.json"),
        &read_data("battle_orders/order_war_cry.json"),
        &side_setup.faction,
        general,
        own / theirs,
        setup.terrain.key(),
        sim.weather().key(),
        seed,
        if side == SideId::Attacker {
            "attacker"
        } else {
            "defender"
        },
    ) else {
        return false;
    };
    if speech.lines.is_empty() {
        return false;
    }
    // Camera path: the centres of the regiments, along the front.
    let mut points: Vec<Vec3> = Vec::new();
    let mut facing = Vec2::ZERO;
    for unit in sim.units().iter().filter(|u| {
        u.side == side
            && u.present()
            && !u.synthetic
            && u.category != data_model::UnitCategory::Siege
    }) {
        let y = sim.field().walk_height(unit.x, unit.z) as f32;
        points.push(Vec3::new(unit.x as f32, y, unit.z as f32));
        facing += Vec2::new(unit.facing.sin() as f32, unit.facing.cos() as f32);
    }
    if points.is_empty() {
        return false;
    }
    let yaw = facing.x.atan2(facing.y);
    let lateral = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    points.sort_by(|a, b| a.dot(lateral).total_cmp(&b.dot(lateral)));
    let voiced = voice_cast(&speech.faction, &speech.general_id);
    let (starts, durations) = schedule(&speech.lines, speech.line_seconds, voiced);
    let cry_s = if voiced {
        speech
            .cry_seconds
            .max(spoken_seconds(&speech.cry) + 0.3 - 0.6)
    } else {
        speech.cry_seconds
    };
    if !deploying && state.pause_restore.is_none() {
        state.pause_restore = Some(session.paused);
        session.paused = true;
    }
    state.shown = Some((speech.speaker.to_uppercase(), String::new(), false));
    state.playing = Some(Playing {
        speech,
        side,
        path: points,
        yaw,
        t: 0.0,
        starts,
        durations,
        cry_s,
        spoken: None,
        cried: false,
        saved: (camera.target, camera.distance, camera.yaw),
    });
    true
}

/// Starts, runs and ends the two speeches (player first, then the enemy general).
#[allow(clippy::too_many_arguments)]
pub fn run_speech(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut edges: ResMut<crate::menu::KeyEdges>,
    deployment: Res<Deployment>,
    mut state: ResMut<Speech>,
    mut session: ResMut<BattleSession>,
    camera: Option<ResMut<BattleCameraState>>,
    mut says: MessageWriter<PlaySpeech>,
    mut sfx: MessageWriter<PlaySfxAt>,
) {
    let Some(mut camera) = camera else { return };
    if state.stage == Stage::Done {
        return;
    }
    let Some(player) = session.player else {
        state.stage = Stage::Done;
        return;
    };
    if state.playing.is_none() {
        // Next speech to open.
        let side = match state.stage {
            Stage::Waiting => {
                if !deployment.tried {
                    return;
                }
                if disabled() || session.replaying || session.snapshot.finished {
                    state.stage = Stage::Done;
                    return;
                }
                player
            }
            Stage::Player if !state.skipped => player.other(),
            _ => {
                state.stage = Stage::Done;
                return;
            }
        };
        let next = if state.stage == Stage::Waiting {
            Stage::Player
        } else {
            Stage::Enemy
        };
        if open(&mut session, &mut state, &camera, side, deployment.active) {
            state.stage = next;
        } else {
            state.stage = Stage::Done;
        }
        if state.playing.is_none() {
            return;
        }
    }
    let skip = edges.escape
        || edges.enter
        || keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || buttons.just_pressed(MouseButton::Left);
    let Some(playing) = state.playing.as_mut() else {
        return;
    };
    if edges.escape {
        edges.escape = false; // the pause menu must not open for the skip
    }
    playing.t += time.delta_secs();
    let speaking = playing.speaking_seconds();
    let mut say: Option<String> = None;
    let mut cry: Option<(String, Vec3)> = None;
    let shown: (String, String, bool);
    if playing.t < speaking {
        let u = smoothstep(0.0, 1.0, playing.t / speaking.max(0.01));
        let focus = point_at(&playing.path, u);
        let angle = playing.yaw + 0.75 + (-0.75 - 0.75) * u;
        aim(&mut camera, focus, CAMERA_DISTANCE, angle);
        let index = playing.line_index(playing.t);
        if playing.spoken != Some(index) {
            playing.spoken = Some(index);
            say = Some(playing.speech.lines[index].clone());
        }
        let last = playing.speech.lines.len() - 1;
        let mut text = playing.speech.lines[index].clone();
        if index == 0 {
            text = format!("« {text}");
        }
        if index == last {
            text.push_str(" »");
        }
        shown = (playing.speech.speaker.to_uppercase(), text, false);
    } else {
        let k = smoothstep(0.0, 1.0, (playing.t - speaking) / playing.cry_s.max(0.01));
        let centre = point_at(&playing.path, 0.5);
        aim(
            &mut camera,
            centre,
            CAMERA_DISTANCE + (CRY_DISTANCE - CAMERA_DISTANCE) * k,
            playing.yaw,
        );
        shown = (String::new(), playing.speech.cry.clone(), true);
        if !playing.cried {
            playing.cried = true;
            say = Some(playing.speech.cry.clone());
            cry = Some((playing.speech.cry.clone(), centre));
        }
    }
    if let Some(text) = say {
        let general_id = playing.speech.general_id.clone();
        let faction = playing.speech.faction.clone();
        says.write(PlaySpeech {
            faction,
            general_id,
            text: text.clone(),
        });
        state.said.push(text);
    }
    if let Some((_, centre)) = cry {
        sfx.write(PlaySfxAt::new("war_cry", centre));
    }
    state.shown = Some(shown);
    let Some(playing) = state.playing.as_ref() else {
        return;
    };
    if skip || playing.t >= playing.total_seconds() {
        let was_skip = skip;
        let playing = state.playing.take().expect("playing");
        camera.target = playing.saved.0;
        camera.distance = playing.saved.1;
        camera.target_distance = playing.saved.1;
        camera.yaw = playing.saved.2;
        state.finished += 1;
        state.shown = None;
        if was_skip {
            state.skipped = true;
        }
        if playing.side != player || was_skip {
            state.stage = Stage::Done;
            if let Some(paused) = state.pause_restore.take() {
                session.paused = paused;
            }
        }
    }
}

fn aim(camera: &mut BattleCameraState, focus: Vec3, distance: f32, yaw: f32) {
    camera.target = focus;
    camera.distance = distance;
    camera.target_distance = distance;
    camera.yaw = yaw;
}

/// The subtitle band and the cry, built when a speech starts and removed at its end.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_subtitles(
    mut commands: Commands,
    state: Res<Speech>,
    theme: Res<UiTheme>,
    roots: Query<Entity, With<SpeechRoot>>,
    mut speaker: Query<&mut Text, (With<SpeechSpeaker>, Without<SpeechLine>, Without<SpeechCry>)>,
    mut line: Query<&mut Text, (With<SpeechLine>, Without<SpeechSpeaker>, Without<SpeechCry>)>,
    mut cry: Query<
        (&mut Text, &mut Visibility),
        (With<SpeechCry>, Without<SpeechSpeaker>, Without<SpeechLine>),
    >,
    mut band: Query<&mut Visibility, (With<SpeechBand>, Without<SpeechCry>)>,
) {
    let Some((who, text, is_cry)) = state.shown.clone() else {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    };
    if roots.is_empty() {
        spawn_subtitles(&mut commands, &theme);
        return;
    }
    let set = |target: &mut Text, value: &str| {
        if target.0 != value {
            target.0 = value.to_owned();
        }
    };
    if !who.is_empty() {
        for mut t in &mut speaker {
            set(&mut t, &who);
        }
    }
    for mut t in &mut line {
        set(&mut t, if is_cry { "" } else { &text });
    }
    for (mut t, mut visibility) in &mut cry {
        set(&mut t, if is_cry { &text } else { "" });
        *visibility = if is_cry {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for mut visibility in &mut band {
        *visibility = if is_cry {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

fn spawn_subtitles(commands: &mut Commands, theme: &UiTheme) {
    let root = commands
        .spawn((
            Name::new("SpeechRoot"),
            SpeechRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(UiTier::Banner.z()),
            Pickable::IGNORE,
            DespawnOnExit(crate::app::AppState::Battle),
        ))
        .id();
    let band = commands
        .spawn((
            Name::new("SpeechBand"),
            SpeechBand,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(20.0),
                width: Val::Percent(60.0),
                top: Val::Px(SUBTITLE_TOP),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                padding: UiRect {
                    left: Val::Px(28.0),
                    right: Val::Px(28.0),
                    top: Val::Px(10.0),
                    bottom: Val::Px(12.0),
                },
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.05, 0.04, 0.72)),
            Pickable::IGNORE,
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        Name::new("SpeechSpeaker"),
        SpeechSpeaker,
        Text::new(""),
        theme.text_font(true, false, 15.0),
        TextColor(Color::srgb(0.93, 0.8, 0.5)),
        Pickable::IGNORE,
        ChildOf(band),
    ));
    commands.spawn((
        Name::new("SpeechLine"),
        SpeechLine,
        Text::new(""),
        theme.text_font(false, false, 23.0),
        TextColor(Color::srgb(0.97, 0.95, 0.9)),
        TextLayout::justify(Justify::Center),
        Pickable::IGNORE,
        ChildOf(band),
    ));
    commands.spawn((
        Name::new("SpeechHint"),
        Text::new("Échap ou clic : passer"),
        theme.text_font(false, false, 12.0),
        TextColor(Color::srgba(0.8, 0.76, 0.66, 0.8)),
        Node {
            align_self: AlignSelf::FlexEnd,
            ..default()
        },
        Pickable::IGNORE,
        ChildOf(band),
    ));
    commands.spawn((
        Name::new("SpeechCry"),
        SpeechCry,
        Text::new(""),
        theme.text_font(true, false, 54.0),
        TextColor(Color::srgb(0.98, 0.9, 0.62)),
        TextShadow {
            offset: Vec2::splat(3.0),
            color: Color::srgba(0.0, 0.0, 0.0, 0.8),
        },
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            top: Val::Percent(36.0),
            ..default()
        },
        Visibility::Hidden,
        Pickable::IGNORE,
        ChildOf(root),
    ));
}

pub fn reset(mut state: ResMut<Speech>) {
    *state = Speech::default();
}

pub fn dump(world: &World) -> (String, Value) {
    let Some(state) = world.get_resource::<Speech>() else {
        return ("battle_speech".into(), Value::Null);
    };
    (
        "battle_speech".into(),
        serde_json::json!({
        "stage": format!("{:?}", state.stage),
        "active": state.active(),
        "skipped": state.skipped,
        "finished": state.finished,
        "shown": state.shown.as_ref().map(|(who, text, cry)| serde_json::json!({
            "speaker": who, "text": text, "cry": cry,
        })),
        "said": state.said,
        "ui": world.iter_entities().filter(|e| e.contains::<SpeechRoot>()).count(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../data/speeches/battle_speeches.json");
        serde_json::from_str(&std::fs::read_to_string(path).expect("data")).expect("json")
    }

    fn cries() -> Value {
        serde_json::json!({"labels_by_faction": {"fac_france": "Montjoie !"}})
    }

    #[test]
    fn compose_is_deterministic_and_has_the_parts() {
        let args = |ratio, seed| {
            compose(
                &data(),
                &cries(),
                "fac_france",
                Some(("chr_philippe", "Philippe VI")),
                ratio,
                "plains",
                "rain",
                seed,
                "attacker",
            )
            .expect("speech")
        };
        let a = args(1.0, 7);
        assert_eq!(a, args(1.0, 7));
        // opening, general, odds, terrain, weather, closing
        assert_eq!(a.lines.len(), 6);
        assert!(a.lines[1].contains("Philippe VI"));
        assert_eq!(a.cry, "Montjoie !");
        assert_eq!(a.speaker, "Philippe VI");
        assert_eq!(a.general_id, "chr_philippe");
        // Different seeds eventually choose other sentences.
        assert!((0..20).any(|s| args(1.0, s).lines != a.lines));
        // No weather line when clear; strong odds pick from the strong list.
        let clear = compose(
            &data(),
            &cries(),
            "fac_unknown",
            None,
            2.0,
            "nowhere",
            "clear",
            1,
            "defender",
        )
        .expect("speech");
        assert_eq!(clear.lines.len(), 4);
        assert_eq!(clear.speaker, "le capitaine");
        assert_eq!(clear.cry, "À moi ! En avant !");
        let strong = strings(&data()["odds"]["strong"]);
        assert!(strong.contains(&clear.lines[1]));
    }

    #[test]
    fn schedule_and_path() {
        let lines = vec!["abc".to_owned(), "x".repeat(130)];
        let (starts, durations) = schedule(&lines, 4.6, false);
        assert_eq!(starts, vec![0.0, 4.6]);
        assert_eq!(durations, vec![4.6, 4.6]);
        let (_, voiced) = schedule(&lines, 4.6, true);
        assert!(voiced[1] > 10.0 && voiced[0] == 4.6);
        let path = [
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 20.0),
        ];
        assert_eq!(point_at(&path, 0.0), Vec3::ZERO);
        assert_eq!(point_at(&path, 0.5), Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(point_at(&path, 1.0), Vec3::new(10.0, 0.0, 20.0));
        assert_eq!(point_at(&path[..1], 0.7), Vec3::ZERO);
    }
}
