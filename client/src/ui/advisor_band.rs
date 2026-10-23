//! The single « Conseiller » band (ADR 0752). Replaces the four competing advice zones of the
//! first turn (journal, « Conseil » card, advisor bubble / subtitle toast, hint banners): every
//! source sends an [`BandSay`]; the band shows ONE message at a time, in priority order, folds
//! by itself once it has been read, keeps the folded message as a small chip when it is a
//! standing hint (still true, not yet done), and logs every shown message in
//! [`AdvisorHistory`] (read by the journal).
//!
//! The state machine ([`BandState`]) is pure and tested; the host entity ([`AdvisorBandHost`],
//! spawned by the HUD that owns the screen: campaign bottom left, battle journal slot) only
//! renders it. Texts come from data (`data/ui/next_hints.json`, `data/advisor/*.json`); this file
//! holds only the two button labels.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use serde_json::{Value, json};

use super::kit::style::{self, UiType};
use super::kit::widgets::{button_primary, button_small, panel_note};
use super::kit::{ButtonPressed, UiTheme};
use crate::agent::dump::DumpAppExt;

/// Messages kept for the journal.
const HISTORY_CAP: usize = 40;
/// Reading time bounds (seconds) and per-word cost.
const MIN_SECONDS: f32 = 4.0;
const MAX_SECONDS: f32 = 16.0;
const BASE_SECONDS: f32 = 2.5;
const PER_WORD: f32 = 0.33;

pub const GO: &str = "advisor:go";
pub const LATER: &str = "advisor:later";
pub const CHIP: &str = "advisor:chip";

/// Importance of a message; a higher one preempts the shown one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdvisorPriority {
    /// Voiced remark, flavour.
    Info,
    /// « What to do now » hint.
    Advice,
    /// Danger the player can still avoid.
    Warning,
    /// Needs an answer now.
    Urgent,
}

/// A message for the band.
#[derive(Message, Debug, Clone)]
pub struct BandSay {
    /// Same key = same message (refreshed, not duplicated).
    pub key: String,
    pub text: String,
    pub priority: AdvisorPriority,
    /// Display time; `None` = reading time of the text.
    pub seconds: Option<f32>,
    /// Caption of the primary button (`None` = no action, only « Compris »).
    pub action: Option<String>,
    /// Stays available as a chip after folding, until withdrawn.
    pub standing: bool,
}

impl BandSay {
    pub fn info(key: &str, text: &str, seconds: f32) -> Self {
        Self {
            key: key.to_owned(),
            text: text.to_owned(),
            priority: AdvisorPriority::Info,
            seconds: Some(seconds),
            action: None,
            standing: false,
        }
    }
}

/// The player pressed a button of the shown message.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct AdvisorAction {
    pub key: String,
    pub kind: AdvisorActionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisorActionKind {
    /// Primary button (« Y aller »).
    Go,
    /// « Plus tard » / « Compris »: the message is set aside.
    Later,
}

/// Withdraws a standing message whose reason is gone.
#[derive(Message, Debug, Clone)]
pub struct AdvisorWithdraw(pub String);

/// Reading time of `text` in seconds.
pub fn reading_seconds(text: &str) -> f32 {
    let words = text.split_whitespace().count() as f32;
    (BASE_SECONDS + words * PER_WORD).clamp(MIN_SECONDS, MAX_SECONDS)
}

#[derive(Debug, Clone)]
struct Active {
    say: BandSay,
    remaining: f32,
}

/// One logged message.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub key: String,
    pub text: String,
}

/// Pure state of the band.
#[derive(Debug, Default, Clone)]
pub struct BandState {
    queue: Vec<BandSay>,
    active: Option<Active>,
    /// Folded standing message, reopened by the chip.
    chip: Option<BandSay>,
    history: Vec<HistoryEntry>,
    pub shown: u32,
}

impl BandState {
    pub fn active(&self) -> Option<&BandSay> {
        self.active.as_ref().map(|a| &a.say)
    }

    pub fn chip(&self) -> Option<&BandSay> {
        self.chip.as_ref()
    }

    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    fn log(&mut self, say: &BandSay) {
        // The same message is not logged twice in a row.
        if self
            .history
            .last()
            .is_some_and(|h| h.key == say.key && h.text == say.text)
        {
            return;
        }
        self.history.push(HistoryEntry {
            key: say.key.clone(),
            text: say.text.clone(),
        });
        if self.history.len() > HISTORY_CAP {
            self.history.remove(0);
        }
    }

    fn activate(&mut self, say: BandSay) {
        let remaining = say.seconds.unwrap_or_else(|| reading_seconds(&say.text));
        if self.chip.as_ref().is_some_and(|c| c.key == say.key) {
            self.chip = None;
        }
        self.log(&say);
        self.shown += 1;
        self.active = Some(Active { say, remaining });
    }

    /// Folds the shown message: a standing one becomes the chip.
    fn fold_active(&mut self) {
        if let Some(active) = self.active.take()
            && active.say.standing
        {
            self.chip = Some(active.say);
        }
    }

    pub fn push(&mut self, say: BandSay) {
        if let Some(active) = self.active.as_mut()
            && active.say.key == say.key
        {
            // Same message: refresh the text, restart the reading time only when it changed.
            if active.say.text != say.text {
                active.remaining = say.seconds.unwrap_or_else(|| reading_seconds(&say.text));
            }
            active.say = say;
            return;
        }
        if let Some(chip) = self.chip.as_mut()
            && chip.key == say.key
        {
            // Folded and still true: stays folded unless the text changed.
            if chip.text == say.text {
                *chip = say;
                return;
            }
            self.chip = None;
        }
        self.queue.retain(|q| q.key != say.key);
        let preempts = self.active.as_ref().is_some_and(|a| {
            say.priority >= AdvisorPriority::Warning && say.priority > a.say.priority
        });
        self.queue.push(say);
        // Highest priority first, arrival order inside a priority (stable sort).
        self.queue.sort_by_key(|q| std::cmp::Reverse(q.priority));
        if preempts {
            self.fold_active();
        }
        if self.active.is_none() {
            self.next();
        }
    }

    fn next(&mut self) {
        if !self.queue.is_empty() {
            let say = self.queue.remove(0);
            self.activate(say);
        }
    }

    /// Advances the reading time (`paused` while the pointer or the focus is on the band).
    pub fn tick(&mut self, dt: f32, paused: bool) {
        if let Some(active) = self.active.as_mut() {
            if paused {
                return;
            }
            active.remaining -= dt;
            if active.remaining > 0.0 {
                return;
            }
            self.fold_active();
        }
        self.next();
    }

    /// « Plus tard » / « Compris »: sets the shown message aside (a standing one stays as chip
    /// unless `forget`).
    pub fn dismiss(&mut self, forget: bool) {
        if forget {
            self.active = None;
        } else {
            self.fold_active();
        }
        self.next();
    }

    /// The chip was pressed: shows the folded message again.
    pub fn reopen_chip(&mut self) {
        if let Some(chip) = self.chip.take() {
            self.queue.insert(0, chip);
            if let Some(active) = self.active.as_ref()
                && active.say.priority < AdvisorPriority::Warning
            {
                self.fold_active();
            }
            if self.active.is_none() {
                self.next();
            }
        }
    }

    pub fn withdraw(&mut self, key: &str) {
        self.queue.retain(|q| q.key != key);
        if self.chip.as_ref().is_some_and(|c| c.key == key) {
            self.chip = None;
        }
        if self.active.as_ref().is_some_and(|a| a.say.key == key) {
            self.active = None;
            self.next();
        }
    }
}

/// The band's state as a resource.
#[derive(Resource, Default)]
pub struct AdvisorBand {
    pub state: BandState,
    /// Another narrator (the tutorial guide) owns the zone: only urgent messages show.
    pub suppressed: bool,
    built: Option<BuiltKey>,
}

/// Read side for the journal.
pub type AdvisorHistory = AdvisorBand;

impl AdvisorBand {
    pub fn history(&self) -> &[HistoryEntry] {
        self.state.history()
    }
}

type BuiltKey = (Option<String>, Option<String>, String, Entity);

/// Marks the node the band is built under (one per screen).
#[derive(Component)]
pub struct AdvisorBandHost;

#[derive(Component)]
struct BandPanel;

pub struct AdvisorBandPlugin;

impl Plugin for AdvisorBandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AdvisorBand>()
            .add_message::<BandSay>()
            .add_message::<AdvisorAction>()
            .add_message::<AdvisorWithdraw>()
            .register_dump(dump)
            .add_systems(Update, (receive, buttons, tick, render).chain());
    }
}

fn receive(
    mut band: ResMut<AdvisorBand>,
    mut says: MessageReader<BandSay>,
    mut withdraws: MessageReader<AdvisorWithdraw>,
) {
    for say in says.read() {
        band.state.push(say.clone());
    }
    for key in withdraws.read() {
        band.state.withdraw(&key.0);
    }
}

fn buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut band: ResMut<AdvisorBand>,
    mut actions: MessageWriter<AdvisorAction>,
) {
    for press in pressed.read() {
        match press.name.as_str() {
            GO => {
                if let Some(say) = band.state.active().cloned() {
                    actions.write(AdvisorAction {
                        key: say.key,
                        kind: AdvisorActionKind::Go,
                    });
                    band.state.dismiss(false);
                }
            }
            LATER => {
                if let Some(say) = band.state.active().cloned() {
                    actions.write(AdvisorAction {
                        key: say.key,
                        kind: AdvisorActionKind::Later,
                    });
                    band.state.dismiss(false);
                }
            }
            CHIP => band.state.reopen_chip(),
            _ => {}
        }
    }
}

fn tick(time: Res<Time>, mut band: ResMut<AdvisorBand>, panel: Query<&Hovered, With<BandPanel>>) {
    let hovered = panel.iter().any(|h| h.get());
    band.state.tick(time.delta_secs(), hovered);
}

fn render(
    mut commands: Commands,
    mut band: ResMut<AdvisorBand>,
    theme: Res<UiTheme>,
    host: Query<Entity, With<AdvisorBandHost>>,
    existing: Query<Entity, With<BandPanel>>,
) {
    let Ok(host) = host.single() else { return };
    let suppressed = band.suppressed;
    let active = band
        .state
        .active()
        .filter(|s| !suppressed || s.priority == AdvisorPriority::Urgent)
        .cloned();
    let chip = if suppressed || active.is_some() {
        None
    } else {
        band.state.chip().cloned()
    };
    let key: BuiltKey = (
        active
            .as_ref()
            .map(|s| format!("{}|{}", s.key, s.action.is_some())),
        chip.as_ref().map(|s| s.key.clone()),
        active.as_ref().map(|s| s.text.clone()).unwrap_or_default(),
        host,
    );
    if band.built.as_ref() == Some(&key)
        && !existing.is_empty() == (key.0.is_some() || key.1.is_some())
    {
        return;
    }
    band.built = Some(key);
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    if let Some(say) = active {
        spawn_message(&mut commands, &theme, host, &say);
    } else if chip.is_some() {
        commands.spawn((
            Name::new("advisor:band"),
            BandPanel,
            Hovered::default(),
            ChildOf(host),
            Node {
                align_self: AlignSelf::FlexStart,
                ..default()
            },
            children![button_small(&theme, CHIP, "Conseiller ▸")],
        ));
    }
}

fn spawn_message(commands: &mut Commands, theme: &UiTheme, host: Entity, say: &BandSay) {
    let accent = match say.priority {
        AdvisorPriority::Info | AdvisorPriority::Advice => style::RUBRIC,
        AdvisorPriority::Warning | AdvisorPriority::Urgent => style::POOR,
    };
    let text = say.text.clone();
    let action = say.action.clone();
    let later = if say.standing { "Plus tard" } else { "Compris" };
    let theme = theme.clone();
    commands
        .spawn((
            Name::new("advisor:band"),
            BandPanel,
            Hovered::default(),
            ChildOf(host),
            panel_note(&theme),
        ))
        .insert(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(12.0)),
            ..default()
        })
        .with_children(|band| {
            band.spawn((
                Name::new("advisor:text"),
                Text::new(text),
                theme.text_font(false, false, UiType::Caption.size()),
                TextColor(accent),
            ));
            band.spawn(Node {
                column_gap: Val::Px(4.0),
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|row| {
                if let Some(label) = action {
                    row.spawn(button_primary(&theme, GO, &label));
                }
                row.spawn(button_small(&theme, LATER, later));
            });
        });
}

fn dump(world: &World) -> (String, Value) {
    let band = world.resource::<AdvisorBand>();
    (
        "advisor_band".to_owned(),
        json!({
            "active": band.state.active().map(|s| json!({
                "key": s.key, "text": s.text, "priority": format!("{:?}", s.priority),
                "action": s.action, "standing": s.standing,
            })),
            "chip": band.state.chip().map(|s| s.key.clone()),
            "queued": band.state.queued(),
            "shown": band.state.shown,
            "suppressed": band.suppressed,
            "history": band.history().iter().map(|h| h.key.clone()).collect::<Vec<_>>(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn say(key: &str, priority: AdvisorPriority, standing: bool) -> BandSay {
        BandSay {
            key: key.to_owned(),
            text: format!("texte {key}"),
            priority,
            seconds: Some(5.0),
            action: Some("Y aller".to_owned()),
            standing,
        }
    }

    #[test]
    fn one_message_at_a_time_in_priority_order() {
        let mut state = BandState::default();
        state.push(say("a", AdvisorPriority::Info, false));
        state.push(say("b", AdvisorPriority::Advice, false));
        state.push(say("c", AdvisorPriority::Advice, false));
        assert_eq!(state.active().map(|s| s.key.as_str()), Some("a"));
        assert_eq!(state.queued(), 2);
        state.tick(5.1, false);
        assert_eq!(state.active().map(|s| s.key.as_str()), Some("b"));
        state.tick(5.1, false);
        assert_eq!(state.active().map(|s| s.key.as_str()), Some("c"));
        state.tick(5.1, false);
        assert!(state.active().is_none());
        assert_eq!(state.history().len(), 3);
    }

    #[test]
    fn a_higher_priority_preempts_and_the_standing_one_folds_to_a_chip() {
        let mut state = BandState::default();
        state.push(say("hint", AdvisorPriority::Advice, true));
        state.push(say("siege", AdvisorPriority::Urgent, false));
        assert_eq!(state.active().map(|s| s.key.as_str()), Some("siege"));
        assert_eq!(state.chip().map(|s| s.key.as_str()), Some("hint"));
        state.reopen_chip();
        // Urgent stays; the hint waits in the queue.
        assert_eq!(state.active().map(|s| s.key.as_str()), Some("siege"));
        assert_eq!(state.queued(), 1);
    }

    #[test]
    fn reading_time_pauses_while_hovered_and_scales_with_the_text() {
        let mut state = BandState::default();
        state.push(say("a", AdvisorPriority::Info, false));
        state.tick(100.0, true);
        assert!(state.active().is_some());
        assert!(reading_seconds("un deux") >= MIN_SECONDS);
        let long = "mot ".repeat(80);
        assert!((reading_seconds(&long) - MAX_SECONDS).abs() < f32::EPSILON);
    }

    #[test]
    fn same_key_refreshes_and_a_folded_unchanged_hint_stays_folded() {
        let mut state = BandState::default();
        state.push(say("hint", AdvisorPriority::Advice, true));
        state.tick(6.0, false);
        assert!(state.active().is_none() && state.chip().is_some());
        state.push(say("hint", AdvisorPriority::Advice, true));
        assert!(state.active().is_none(), "same hint must not pop up again");
        let mut changed = say("hint", AdvisorPriority::Advice, true);
        changed.text = "autre".into();
        state.push(changed);
        assert_eq!(state.active().map(|s| s.text.as_str()), Some("autre"));
        assert_eq!(state.history().len(), 2);
    }

    #[test]
    fn withdraw_clears_active_queue_and_chip() {
        let mut state = BandState::default();
        state.push(say("a", AdvisorPriority::Advice, true));
        state.push(say("b", AdvisorPriority::Advice, false));
        state.withdraw("a");
        assert_eq!(state.active().map(|s| s.key.as_str()), Some("b"));
        state.dismiss(false);
        assert!(state.active().is_none());
    }
}
