//! The advisor (Jean le Bel): brief spoken interventions with a subtitle, port of
//! game/scripts/audio/advisor.gd.
//!
//! Triggers: campaign start, first battle / assault / siege / victory / defeat (said once, kept in
//! `advisor_seen.json` beside the settings file) and important turn alerts (war, plague, famine,
//! revolt, bankruptcy...; one per turn, a kind at most once per `alert_cooldown_turns`). Texts:
//! `data/voice/advisor.json`; audio `audio/voice/advisor/<id>.ogg` (absent: subtitle only).
//! The music is ducked while he speaks. A line waits while a blocking window is open (queue of
//! 2, forgotten after 120 s). Disabled by the `voice/advisor` setting.
//!
//! Subtitle: a kit toast `« text »` for the duration of the line (Godot draws a dedicated bubble
//! with a dismiss button, not ported). Other lots send [`AdvisorSay`].

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde_json::{Value, json};

use super::voice::{BarkLine, VoiceData};
use super::{AudioDirector, DuckMusic, PlayVoice, read_json};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::hud::flow::TurnAdvanced;
use crate::settings::Settings;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::panel_stack::PanelStack;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::widgets::button_small;
use crate::ui::kit::widgets::panel_note;
use crate::ui::kit::{ButtonPressed, UiTheme};

const DUCK_DB: f32 = -9.0;
const READ_CHARS_PER_SECOND: f32 = 14.0;
const MAX_QUEUE: usize = 2;
const MAX_WAIT_S: f32 = 120.0;
const VOICE_PRIORITY: i32 = 6;

/// A trigger from another lot (`first_victory`, `first_assault`...).
#[derive(Message, Debug, Clone)]
pub struct AdvisorSay {
    pub trigger: String,
    pub faction: String,
}

/// Tunables of `advisor.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdvisorTuning {
    pub alert_cooldown_turns: u32,
    pub min_gap_s: f32,
    pub linger_s: f32,
}

impl AdvisorTuning {
    pub fn from_json(advisor: &Value) -> Self {
        let number =
            |key: &str, default: f64| advisor.get(key).and_then(Value::as_f64).unwrap_or(default);
        Self {
            alert_cooldown_turns: number("alert_cooldown_turns", 8.0) as u32,
            min_gap_s: number("min_gap_s", 1.5) as f32,
            linger_s: number("linger_s", 2.5) as f32,
        }
    }
}

/// What the advisor decided to say now.
#[derive(Debug, Clone, PartialEq)]
pub struct Intervention {
    pub line: BarkLine,
    /// Seconds the voice lasts (estimated from the text length: no probe of the clip).
    pub spoken_s: f32,
}

/// Pure state machine (queue, cooldowns, once-only triggers).
#[derive(Debug, Default)]
pub struct Advisor {
    pub seen: HashSet<String>,
    pub said: Vec<String>,
    time: f32,
    until: f32,
    last_end: f32,
    queue: Vec<(BarkLine, f32)>,
    alert_turns: HashMap<String, u32>,
}

impl Advisor {
    pub fn new() -> Self {
        Self {
            last_end: -1000.0,
            ..Self::default()
        }
    }

    pub fn speaking(&self) -> bool {
        self.time < self.until
    }

    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    /// Line of a trigger (`first_*` only once). `None`: nothing to say.
    pub fn trigger(&mut self, data: &VoiceData, name: &str, faction: &str) -> Option<BarkLine> {
        let once = name.starts_with("first_");
        if once && self.seen.contains(name) {
            return None;
        }
        let line = data.advisor_line(name, faction, "")?;
        if once {
            self.seen.insert(name.to_string());
        }
        Some(line)
    }

    /// Turn alerts of the player's faction: `(faction, kind)` pairs. Returns the lines to say
    /// (the first siege, else one alert per turn at most).
    pub fn turn_events(
        &mut self,
        data: &VoiceData,
        tuning: &AdvisorTuning,
        events: &[(String, String)],
        player: &str,
        turn: u32,
    ) -> Option<BarkLine> {
        for (faction, kind) in events {
            if faction != player {
                continue;
            }
            if kind == "siege_started"
                && let Some(line) = self.trigger(data, "first_siege", player)
            {
                return Some(line);
            }
            let Some(line) = data.advisor_line("alert", player, kind) else {
                continue;
            };
            if let Some(last) = self.alert_turns.get(kind)
                && turn.saturating_sub(*last) < tuning.alert_cooldown_turns
            {
                continue;
            }
            self.alert_turns.insert(kind.clone(), turn);
            return Some(line);
        }
        None
    }

    /// Says `line` now (`Some`) or queues it (a voice is speaking or a window blocks).
    pub fn say(
        &mut self,
        line: BarkLine,
        blocked: bool,
        tuning: &AdvisorTuning,
    ) -> Option<Intervention> {
        if self.speaking() || blocked {
            if self.queue.len() < MAX_QUEUE {
                self.queue.push((line, self.time));
            }
            return None;
        }
        Some(self.start(line, tuning))
    }

    fn start(&mut self, line: BarkLine, tuning: &AdvisorTuning) -> Intervention {
        self.said.push(line.id.clone());
        if self.said.len() > 16 {
            self.said.remove(0);
        }
        let spoken_s = line.text.chars().count() as f32 / READ_CHARS_PER_SECOND;
        self.until = self.time + spoken_s + tuning.linger_s;
        Intervention { line, spoken_s }
    }

    /// Advances the clock; starts the next queued line once the voice is free and no window
    /// blocks (`min_gap_s` after the last one).
    pub fn tick(&mut self, dt: f32, blocked: bool, tuning: &AdvisorTuning) -> Option<Intervention> {
        let was_speaking = self.speaking();
        self.time += dt;
        if was_speaking && !self.speaking() {
            self.last_end = self.time;
        }
        let time = self.time;
        self.queue.retain(|(_, at)| time - at <= MAX_WAIT_S);
        if !self.speaking()
            && !blocked
            && !self.queue.is_empty()
            && self.time - self.last_end >= tuning.min_gap_s
        {
            let (line, _) = self.queue.remove(0);
            return Some(self.start(line, tuning));
        }
        None
    }
}

#[derive(Resource)]
pub struct AdvisorState {
    pub advisor: Advisor,
    tuning: AdvisorTuning,
    seen_path: Option<std::path::PathBuf>,
    /// Interventions started (dump).
    spoken: Vec<String>,
    /// A modal, the pause menu or the tutorial card is up: the advisor keeps quiet (the line
    /// waits in its queue) so that one narrator speaks at a time and no subtitle covers a window.
    covered: bool,
}

fn seen_file() -> Option<std::path::PathBuf> {
    Settings::file_path()
        .parent()
        .map(|directory| directory.join("advisor_seen.json"))
}

impl AdvisorState {
    fn load(director: &AudioDirector) -> Self {
        let mut advisor = Advisor::new();
        let seen_path = if director.muted { None } else { seen_file() };
        if let Some(path) = &seen_path
            && let Some(Value::Array(list)) = read_json(path)
        {
            advisor
                .seen
                .extend(list.iter().filter_map(Value::as_str).map(String::from));
        }
        Self {
            advisor,
            tuning: AdvisorTuning::from_json(&director.voices.advisor),
            seen_path,
            spoken: Vec::new(),
            covered: false,
        }
    }

    fn save_seen(&self) {
        let Some(path) = &self.seen_path else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut seen: Vec<&String> = self.advisor.seen.iter().collect();
        seen.sort();
        let _ = std::fs::write(path, json!(seen).to_string());
    }
}

pub struct AdvisorPlugin;

impl Plugin for AdvisorPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AdvisorSay>()
            .add_message::<TurnAdvanced>()
            .add_message::<ToastRequest>()
            .add_message::<AdvisorBubble>()
            .add_systems(
                Update,
                (show_bubble, expire_bubble, close_bubble).run_if(in_state(AppState::Battle)),
            )
            .add_systems(
                Startup,
                |mut commands: Commands, director: Res<AudioDirector>| {
                    commands.insert_resource(AdvisorState::load(&director));
                },
            )
            .add_systems(OnEnter(AppState::Campaign), campaign_start)
            .add_systems(
                Update,
                (
                    sync_cover,
                    turn_alerts,
                    battle_start,
                    battle_end,
                    external_triggers,
                    advisor_tick,
                )
                    .chain(),
            )
            .register_dump(dump_advisor);
    }
}

/// Everything that makes the advisor speak goes through here.
#[allow(clippy::too_many_arguments)]
fn speak(
    state: &mut AdvisorState,
    line: BarkLine,
    stack: Option<&PanelStack>,
    voices: &mut MessageWriter<PlayVoice>,
    ducks: &mut MessageWriter<DuckMusic>,
    toasts: &mut Subtitles,
) {
    let tuning = state.tuning;
    let blocked = blocked(stack) || state.covered;
    if let Some(intervention) = state.advisor.say(line, blocked, &tuning) {
        emit(state, intervention, voices, ducks, toasts);
    }
}

fn emit(
    state: &mut AdvisorState,
    intervention: Intervention,
    voices: &mut MessageWriter<PlayVoice>,
    ducks: &mut MessageWriter<DuckMusic>,
    toasts: &mut Subtitles,
) {
    state.spoken.push(intervention.line.id.clone());
    state.save_seen();
    voices.write(PlayVoice {
        clip: format!("advisor/{}", intervention.line.id),
        priority: VOICE_PRIORITY,
    });
    ducks.write(DuckMusic {
        db: DUCK_DB,
        hold: intervention.spoken_s + 0.5,
    });
    toasts.say(
        format!("Jean le Bel : « {} »", intervention.line.text),
        intervention.spoken_s + state.tuning.linger_s,
    );
}

/// Where a subtitle goes: a kit toast on the campaign, a bubble under the journal (left column)
/// in battle, as Godot's `advisor.gd` places it.
#[derive(SystemParam)]
pub struct Subtitles<'w> {
    toasts: MessageWriter<'w, ToastRequest>,
    bubbles: MessageWriter<'w, AdvisorBubble>,
    state: Res<'w, State<AppState>>,
}

impl Subtitles<'_> {
    fn say(&mut self, text: String, seconds: f32) {
        if *self.state.get() == AppState::Battle {
            self.bubbles.write(AdvisorBubble { text, seconds });
        } else {
            self.toasts.write(ToastRequest { text, seconds });
        }
    }
}

/// A subtitle for the battle bubble.
#[derive(Message, Debug, Clone)]
pub struct AdvisorBubble {
    pub text: String,
    pub seconds: f32,
}

#[derive(Component)]
struct BubbleLife(Timer);

/// Shows the bubble at the top of the `Journal` slot, ABOVE the journal panel (never under it:
/// the journal grows downwards and would push the bubble out of sight). One bubble at a time;
/// the « × » button dismisses it (Godot `dismiss`; the voice line itself is not interrupted,
/// the audio director has no stop message).
fn show_bubble(
    mut commands: Commands,
    mut requests: MessageReader<AdvisorBubble>,
    slots: Query<(Entity, &crate::battle::hud::geometry::HudSlot)>,
    old: Query<Entity, With<BubbleLife>>,
    theme: Option<Res<UiTheme>>,
) {
    // Headless audio tests run the advisor without the UI kit: no theme, no bubble.
    let Some(theme) = theme else {
        return;
    };
    let Some(request) = requests.read().last().cloned() else {
        return;
    };
    let Some((slot, _)) = slots
        .iter()
        .find(|(_, s)| **s == crate::battle::hud::geometry::HudSlot::Journal)
    else {
        return;
    };
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let bubble = commands
        .spawn((
            Name::new("AdvisorBubble"),
            BubbleLife(Timer::from_seconds(
                request.seconds.max(1.0),
                TimerMode::Once,
            )),
            panel_note(&theme),
            Pickable::IGNORE,
        ))
        .insert(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::top(Val::Px(4.0)),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        })
        .with_children(|bubble| {
            bubble
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::FlexStart,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Text::new(request.text),
                        theme.text_font(false, true, UiType::Caption.size()),
                        TextColor(style::INK),
                        Node {
                            flex_shrink: 1.0,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                    row.spawn(button_small(&theme, BUBBLE_CLOSE, "×"));
                });
        })
        .id();
    // First child of the slot: above the journal.
    commands.entity(slot).insert_children(0, &[bubble]);
}

const BUBBLE_CLOSE: &str = "advisor:close";

/// « × » of the bubble: dismisses it.
fn close_bubble(
    mut commands: Commands,
    mut pressed: MessageReader<ButtonPressed>,
    bubbles: Query<Entity, With<BubbleLife>>,
) {
    if pressed.read().any(|press| press.name == BUBBLE_CLOSE) {
        for entity in &bubbles {
            commands.entity(entity).despawn();
        }
    }
}

fn expire_bubble(
    mut commands: Commands,
    time: Res<Time>,
    mut bubbles: Query<(Entity, &mut BubbleLife)>,
) {
    for (entity, mut life) in &mut bubbles {
        if life.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn blocked(stack: Option<&PanelStack>) -> bool {
    stack.is_some_and(|stack| stack.has_modal_open() || stack.has_central_open())
}

fn enabled(settings: &Option<Res<Settings>>) -> bool {
    advisor_voice_on(settings.as_deref())
}

/// `voice_advisor` (Godot `voice/advisor`): the advisor speaks; on when no settings exist.
pub fn advisor_voice_on(settings: Option<&Settings>) -> bool {
    settings.is_none_or(|settings| settings.voice_advisor)
}

#[allow(clippy::too_many_arguments)]
fn campaign_start(
    state: Option<ResMut<AdvisorState>>,
    director: Res<AudioDirector>,
    sim: Option<Res<crate::sim::Sim>>,
    settings: Option<Res<Settings>>,
    stack: Option<Res<PanelStack>>,
    mut voices: MessageWriter<PlayVoice>,
    mut ducks: MessageWriter<DuckMusic>,
    mut toasts: Subtitles,
) {
    let (Some(mut state), Some(sim)) = (state, sim) else {
        return;
    };
    if !enabled(&settings) {
        return;
    }
    // The opening speech belongs to the first turn: a loaded game or a return from a battle
    // re-enters the campaign screen without hearing it again.
    if sim.state.as_ref().is_some_and(|state| state.turn() > 0) {
        return;
    }
    let faction = sim.player.as_str().to_string();
    if let Some(line) = state
        .advisor
        .trigger(&director.voices, "campaign_start", &faction)
    {
        speak(
            &mut state,
            line,
            stack.as_deref(),
            &mut voices,
            &mut ducks,
            &mut toasts,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn battle_start(
    state: Option<ResMut<AdvisorState>>,
    director: Res<AudioDirector>,
    session: Option<ResMut<crate::battle::scene::BattleSession>>,
    settings: Option<Res<Settings>>,
    stack: Option<Res<PanelStack>>,
    mut announced: Local<bool>,
    mut voices: MessageWriter<PlayVoice>,
    mut ducks: MessageWriter<DuckMusic>,
    mut toasts: Subtitles,
) {
    // Once per battle, as soon as the session exists (it is built after the state change).
    let (Some(mut state), Some(mut session)) = (state, session) else {
        *announced = false;
        return;
    };
    if *announced || !enabled(&settings) {
        return;
    }
    *announced = true;
    let name = if session.sim_mut().siege().is_some() {
        "first_assault"
    } else {
        "first_battle"
    };
    if let Some(line) = state.advisor.trigger(&director.voices, name, "") {
        speak(
            &mut state,
            line,
            stack.as_deref(),
            &mut voices,
            &mut ducks,
            &mut toasts,
        );
    }
}

/// First victory / defeat when the battle ends.
#[allow(clippy::too_many_arguments)]
fn battle_end(
    state: Option<ResMut<AdvisorState>>,
    director: Res<AudioDirector>,
    session: Option<ResMut<crate::battle::scene::BattleSession>>,
    settings: Option<Res<Settings>>,
    stack: Option<Res<PanelStack>>,
    mut announced: Local<bool>,
    mut voices: MessageWriter<PlayVoice>,
    mut ducks: MessageWriter<DuckMusic>,
    mut toasts: Subtitles,
) {
    let (Some(mut state), Some(mut session)) = (state, session) else {
        *announced = false;
        return;
    };
    let sim = session.sim_mut();
    if !sim.is_finished() {
        *announced = false;
        return;
    }
    if *announced || !enabled(&settings) {
        return;
    }
    *announced = true;
    let winner = sim.outcome().map(|outcome| outcome.winner);
    let won = matches!((winner, session.player), (Some(winner), Some(player)) if winner == player);
    let name = if won { "first_victory" } else { "first_defeat" };
    if let Some(line) = state.advisor.trigger(&director.voices, name, "") {
        speak(
            &mut state,
            line,
            stack.as_deref(),
            &mut voices,
            &mut ducks,
            &mut toasts,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn turn_alerts(
    mut turns: MessageReader<TurnAdvanced>,
    state: Option<ResMut<AdvisorState>>,
    director: Res<AudioDirector>,
    sim: Option<Res<crate::sim::Sim>>,
    settings: Option<Res<Settings>>,
    stack: Option<Res<PanelStack>>,
    mut voices: MessageWriter<PlayVoice>,
    mut ducks: MessageWriter<DuckMusic>,
    mut toasts: Subtitles,
) {
    let (Some(mut state), Some(sim)) = (state, sim) else {
        turns.clear();
        return;
    };
    for turn in turns.read() {
        if !enabled(&settings) {
            continue;
        }
        let events: Vec<(String, String)> = turn
            .events
            .iter()
            .filter_map(|event| {
                let faction = event.faction.as_ref()?.as_str().to_string();
                let kind = serde_json::to_value(event.kind).ok()?.as_str()?.to_string();
                Some((faction, kind))
            })
            .collect();
        let tuning = state.tuning;
        let player = sim.player.as_str().to_string();
        if let Some(line) =
            state
                .advisor
                .turn_events(&director.voices, &tuning, &events, &player, turn.turn)
        {
            speak(
                &mut state,
                line,
                stack.as_deref(),
                &mut voices,
                &mut ducks,
                &mut toasts,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn external_triggers(
    mut requests: MessageReader<AdvisorSay>,
    state: Option<ResMut<AdvisorState>>,
    director: Res<AudioDirector>,
    settings: Option<Res<Settings>>,
    stack: Option<Res<PanelStack>>,
    mut voices: MessageWriter<PlayVoice>,
    mut ducks: MessageWriter<DuckMusic>,
    mut toasts: Subtitles,
) {
    let Some(mut state) = state else {
        requests.clear();
        return;
    };
    for request in requests.read() {
        if !enabled(&settings) {
            continue;
        }
        if let Some(line) =
            state
                .advisor
                .trigger(&director.voices, &request.trigger, &request.faction)
        {
            speak(
                &mut state,
                line,
                stack.as_deref(),
                &mut voices,
                &mut ducks,
                &mut toasts,
            );
        }
    }
}

/// What keeps the advisor quiet: an open modal, the pause menu, the tutorial card.
fn sync_cover(
    state: Option<ResMut<AdvisorState>>,
    modals: Option<Res<crate::ui::kit::modal::ModalQueue>>,
    paused: Option<Res<crate::menu::pause::GamePaused>>,
    tutorial: Option<Res<crate::campaign::progress::tutorial::Tutorial>>,
) {
    let Some(mut state) = state else { return };
    let covered = modals.is_some_and(|m| m.is_busy())
        || paused.is_some_and(|p| p.0)
        || tutorial.is_some_and(|t| t.index.is_some());
    if state.covered != covered {
        state.covered = covered;
    }
}

fn advisor_tick(
    time: Res<Time<Real>>,
    state: Option<ResMut<AdvisorState>>,
    stack: Option<Res<PanelStack>>,
    mut voices: MessageWriter<PlayVoice>,
    mut ducks: MessageWriter<DuckMusic>,
    mut toasts: Subtitles,
) {
    let Some(mut state) = state else { return };
    let tuning = state.tuning;
    let blocked = blocked(stack.as_deref()) || state.covered;
    if let Some(intervention) = state.advisor.tick(time.delta_secs(), blocked, &tuning) {
        emit(
            &mut state,
            intervention,
            &mut voices,
            &mut ducks,
            &mut toasts,
        );
    }
}

fn dump_advisor(world: &World) -> (String, Value) {
    let Some(state) = world.get_resource::<AdvisorState>() else {
        return ("advisor".into(), Value::Null);
    };
    let mut seen: Vec<&String> = state.advisor.seen.iter().collect();
    seen.sort();
    (
        "advisor".into(),
        json!({
            "spoken": state.spoken,
            "said": state.advisor.said,
            "seen": seen,
            "queued": state.advisor.queued(),
            "speaking": state.advisor.speaking(),
            "seen_file": state.seen_path.as_ref().map(|path| path.display().to_string()),
            "tuning": {"alert_cooldown_turns": state.tuning.alert_cooldown_turns, "min_gap_s": state.tuning.min_gap_s, "linger_s": state.tuning.linger_s},
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_advisor_setting_gates_the_advisor() {
        let mut settings = Settings::default();
        assert!(advisor_voice_on(Some(&settings)));
        assert!(advisor_voice_on(None));
        settings.voice_advisor = false;
        assert!(!advisor_voice_on(Some(&settings)));
    }

    fn data() -> VoiceData {
        VoiceData {
            barks: Value::Null,
            advisor: json!({"lines": [
                {"id": "adv_start_default", "trigger": "campaign_start", "text": "Monseigneur."},
                {"id": "adv_start_france", "trigger": "campaign_start", "faction": "fac_france", "text": "Sire."},
                {"id": "adv_first_battle", "trigger": "first_battle", "text": "Voici votre première journée."},
                {"id": "adv_first_siege", "trigger": "first_siege", "text": "Un siège."},
                {"id": "adv_plague", "trigger": "alert", "event": "plague", "text": "La mortalité."},
                {"id": "adv_famine", "trigger": "alert", "event": "famine", "text": "La disette."},
            ]}),
        }
    }

    fn tuning() -> AdvisorTuning {
        AdvisorTuning {
            alert_cooldown_turns: 8,
            min_gap_s: 1.5,
            linger_s: 2.5,
        }
    }

    #[test]
    fn faction_line_first_and_first_times_once() {
        let mut advisor = Advisor::new();
        let data = data();
        assert_eq!(
            advisor
                .trigger(&data, "campaign_start", "fac_france")
                .unwrap()
                .id,
            "adv_start_france"
        );
        assert_eq!(
            advisor
                .trigger(&data, "campaign_start", "fac_x")
                .unwrap()
                .id,
            "adv_start_default"
        );
        assert!(advisor.trigger(&data, "first_battle", "").is_some());
        assert!(advisor.trigger(&data, "first_battle", "").is_none());
    }

    #[test]
    fn alerts_one_per_turn_with_cooldown() {
        let mut advisor = Advisor::new();
        let data = data();
        let events = vec![
            ("fac_other".to_string(), "plague".to_string()),
            ("fac_france".to_string(), "plague".to_string()),
            ("fac_france".to_string(), "famine".to_string()),
        ];
        let first = advisor
            .turn_events(&data, &tuning(), &events, "fac_france", 3)
            .unwrap();
        assert_eq!(first.id, "adv_plague");
        // Same kind within the cooldown: skipped, the next kind speaks.
        let second = advisor
            .turn_events(&data, &tuning(), &events, "fac_france", 5)
            .unwrap();
        assert_eq!(second.id, "adv_famine");
        let third = advisor
            .turn_events(&data, &tuning(), &events, "fac_france", 12)
            .unwrap();
        assert_eq!(third.id, "adv_plague");
        // A siege start is the "first siege" the first time.
        let siege = vec![("fac_france".to_string(), "siege_started".to_string())];
        assert_eq!(
            advisor
                .turn_events(&data, &tuning(), &siege, "fac_france", 13)
                .unwrap()
                .id,
            "adv_first_siege"
        );
    }

    #[test]
    fn queue_waits_for_voice_and_windows() {
        let mut advisor = Advisor::new();
        let data = data();
        let line = |id: &str| BarkLine {
            id: id.into(),
            text: "x".repeat(28),
        };
        let tuning = tuning();
        assert!(advisor.say(line("a"), false, &tuning).is_some());
        assert!(advisor.say(line("b"), false, &tuning).is_none());
        assert!(advisor.say(line("c"), false, &tuning).is_none());
        assert!(advisor.say(line("d"), false, &tuning).is_none()); // queue of 2
        assert_eq!(advisor.queued(), 2);
        // Voice lasts 2 s + 2.5 s of linger.
        assert!(advisor.tick(4.0, false, &tuning).is_none());
        assert!(advisor.tick(1.0, true, &tuning).is_none()); // blocked by a window
        assert!(advisor.tick(0.1, false, &tuning).is_none()); // min gap not reached
        let next = advisor.tick(1.5, false, &tuning).unwrap();
        assert_eq!(next.line.id, "b");
        let _ = data;
    }
}
