//! First-turns tutorial (port of `tutorial_controller.gd` and `ui/tutorial_steps.gd`): 14 steps
//! (`data/tutorial/steps.json`), each with a text, an objective, the faction's historical advice,
//! and a condition the controller watches; the next step opens as soon as it is met.
//!
//! Steps and texts come from `game_api::progress::tutorial_steps`; the conditions that read the
//! simulation (march, building, research, turn, tax, governor) from
//! `game_api::progress::sim_objective_met`; the ones that read the interface (selected army,
//! open province, Buildings tab, diplomacy, season report, chronicle) are decided here from the
//! panels' public state.
//!
//! The card shows at the left of the screen (tier `Tutorial`): « Continuer » (manual steps),
//! « Passer l’étape », « Plus tard » (puts the guide aside, resumes on [`StartTutorial`]),
//! « Passer le tutoriel ». It hides under the pause menu and the settings. Starts by itself on the
//! first turns when `Settings::tutorial_enabled` and the guide was never finished in this session
//! (once the briefing is closed). Widget names: `tutorial:next`, `tutorial:skip`, `tutorial:later`,
//! `tutorial:quit`.
//!
//! Not ported: the arrow and halo on the target (`resolve_target`), the table of contents (jump to
//! a step), the camera focus of each step, persistence of the step in `Settings` (the keys
//! `tutorial/step|done|postponed` do not exist in `Settings`: progress lives for the session),
//! the encyclopedia (Codex lot).
//! Dev hooks: `CENT_ANS_TUTORIAL=<step index>` starts the guide at that step; `CENT_ANS_TUTORIAL=off`
//! disables the autostart.

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use data_model::ProvinceId;
use game_api::progress::{
    TutorialSnapshot, TutorialStep, player_armies, sim_objective_met, tutorial_snapshot,
    tutorial_steps,
};
use serde_json::{Value, json};

use super::StartTutorial;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::SelectedArmy;
use crate::campaign::panels::chronicle::ChroniclePanel;
use crate::campaign::panels::diplomacy::DiplomacyUi;
use crate::campaign::panels::province::ProvincePanel;
use crate::menu::pause::GamePaused;
use crate::paths;
use crate::settings::{Settings, SettingsScreen};
use crate::sim::Sim;
use crate::ui::kit::modal::ModalQueue;
use crate::ui::kit::richtext::{RichStyle, rich_text};
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::{
    TabSelected, button_primary, button_small, caption, panel_sized, separator,
};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

const CARD_WIDTH: f32 = 380.0;
/// Seconds between two checks of the objective.
const CHECK_INTERVAL: f32 = 0.25;
/// Seconds a met objective stays on screen before the next step.
const DONE_PAUSE: f32 = 0.8;
/// Turns after which the guide no longer starts by itself.
const EARLY_TURNS: u32 = 2;
/// Key of the season report in `ModalQueue` (`hud/flow.rs`).
const REPORT_KEY: &str = "hud:season_report";

#[derive(Resource, Default)]
pub struct Tutorial {
    raw: Option<Value>,
    pub steps: Vec<TutorialStep>,
    /// Current step; `None` while the guide is not running.
    pub index: Option<usize>,
    pub snapshot: TutorialSnapshot,
    /// Step the guide was put aside at (« Plus tard »).
    pub postponed: Option<usize>,
    pub finished: bool,
    /// Autostart waits for the briefing; counts seconds in the campaign.
    waited: f32,
    autostart_done: bool,
    check_timer: f32,
    done_timer: Option<f32>,
    /// The Buildings tab of a province was opened since the step began.
    tab_buildings_seen: bool,
    root: Option<Entity>,
    built_for: Option<(usize, bool)>,
    /// Steps completed by their objective, in order (dump).
    pub completed: Vec<String>,
    pub skipped: u32,
    started_at: Option<usize>,
}

#[derive(Component)]
struct TutorialCard;

pub fn plugin(app: &mut App) {
    app.init_resource::<Tutorial>()
        .register_dump(dump)
        .add_systems(Startup, load_steps)
        .add_systems(OnEnter(AppState::Campaign), reset_on_enter)
        .add_systems(
            Update,
            (
                autostart,
                handle_start_message,
                watch_tabs,
                handle_buttons,
                check_objective,
                rebuild,
            )
                .chain()
                .run_if(in_state(AppState::Campaign)),
        )
        .add_systems(OnExit(AppState::Campaign), close_on_exit);
}

fn load_steps(mut tutorial: ResMut<Tutorial>) {
    let loaded = paths::find_root().and_then(|root| {
        let bytes = std::fs::read(paths::data_dir(&root).join("tutorial/steps.json"))
            .map_err(|e| e.to_string())?;
        serde_json::from_slice::<Value>(&bytes).map_err(|e| e.to_string())
    });
    match loaded {
        Ok(raw) => tutorial.raw = Some(raw),
        Err(message) => warn!("tutorial: steps.json not loaded: {message}"),
    }
}

/// Entering the campaign (new game, load, back from a battle): the guide keeps its progress, as
/// the Godot one persisted `tutorial/step|done|postponed`; a guide never started autostarts again.
fn reset_on_enter(mut tutorial: ResMut<Tutorial>, sim: Res<Sim>, mut commands: Commands) {
    if let Some(root) = tutorial.root.take() {
        commands.entity(root).despawn();
    }
    tutorial.built_for = None;
    tutorial.waited = 0.0;
    tutorial.done_timer = None;
    tutorial.autostart_done =
        tutorial.active() || tutorial.finished || tutorial.postponed.is_some();
    if let Some(index) = tutorial.index {
        tutorial.enter(&sim, index);
    }
}

fn close_on_exit(mut commands: Commands, mut tutorial: ResMut<Tutorial>) {
    if let Some(root) = tutorial.root.take() {
        commands.entity(root).despawn();
    }
    tutorial.built_for = None;
}

impl Tutorial {
    pub fn active(&self) -> bool {
        self.index.is_some()
    }

    pub fn step_id(&self) -> &str {
        self.index
            .and_then(|i| self.steps.get(i))
            .map_or("", |s| s.id.as_str())
    }

    fn start(&mut self, sim: &Sim, from: usize) {
        let (Some(data), Some(state), Some(raw)) = (&sim.data, &sim.state, &self.raw) else {
            return;
        };
        self.steps = tutorial_steps(raw, data, state);
        if self.steps.is_empty() {
            return;
        }
        self.postponed = None;
        self.finished = false;
        self.started_at = Some(from);
        self.enter(sim, from.min(self.steps.len() - 1));
    }

    fn enter(&mut self, sim: &Sim, index: usize) {
        self.index = Some(index);
        self.done_timer = None;
        self.check_timer = 0.0;
        self.tab_buildings_seen = false;
        if let Some(state) = &sim.state {
            self.snapshot = tutorial_snapshot(state);
        }
    }

    fn advance(&mut self, sim: &Sim) {
        let Some(index) = self.index else { return };
        if index + 1 >= self.steps.len() {
            self.finish();
        } else {
            self.enter(sim, index + 1);
        }
    }

    fn finish(&mut self) {
        self.index = None;
        self.finished = true;
        self.postponed = None;
    }

    fn postpone(&mut self) {
        if let Some(index) = self.index.take() {
            self.postponed = Some(index);
        }
    }
}

/// Starts the guide on the first turns, once the briefing is closed.
fn autostart(
    mut tutorial: ResMut<Tutorial>,
    sim: Res<Sim>,
    settings: Option<Res<Settings>>,
    queue: Res<ModalQueue>,
    time: Res<Time<Real>>,
) {
    if tutorial.autostart_done || tutorial.raw.is_none() {
        return;
    }
    tutorial.waited += time.delta_secs();
    let hook = std::env::var("CENT_ANS_TUTORIAL").ok();
    if hook.as_deref() == Some("off") {
        tutorial.autostart_done = true;
        return;
    }
    if tutorial.waited < 1.0 || queue.is_busy() {
        return;
    }
    tutorial.autostart_done = true;
    let enabled = settings.as_ref().is_none_or(|s| s.tutorial_enabled);
    let turn = sim.state.as_ref().map_or(u32::MAX, |s| s.turn());
    let from = hook.as_deref().and_then(|h| h.parse::<usize>().ok());
    if from.is_some() || (enabled && !tutorial.finished && turn <= EARLY_TURNS) {
        tutorial.start(&sim, from.unwrap_or(0));
    }
}

fn handle_start_message(
    mut messages: MessageReader<StartTutorial>,
    mut tutorial: ResMut<Tutorial>,
    sim: Res<Sim>,
) {
    for _ in messages.read() {
        if tutorial.active() {
            continue;
        }
        let from = tutorial.postponed.unwrap_or(0);
        tutorial.autostart_done = true;
        tutorial.start(&sim, from);
    }
}

fn watch_tabs(mut tabs: MessageReader<TabSelected>, mut tutorial: ResMut<Tutorial>) {
    for tab in tabs.read() {
        if tab.group == "prov" && tab.name == "prov:tab_buildings" {
            tutorial.tab_buildings_seen = true;
        }
    }
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut tutorial: ResMut<Tutorial>,
    sim: Res<Sim>,
    mut toasts: MessageWriter<crate::ui::kit::modal::ToastRequest>,
) {
    for press in pressed.read() {
        let Some(name) = press.name.strip_prefix("tutorial:") else {
            continue;
        };
        if !tutorial.active() {
            continue;
        }
        match name {
            "next" | "skip" => {
                if name == "skip" {
                    tutorial.skipped += 1;
                }
                tutorial.advance(&sim);
            }
            "later" => {
                let step = tutorial.index.unwrap_or(0) + 1;
                tutorial.postpone();
                toasts.write(crate::ui::kit::modal::ToastRequest::new(&format!(
                    "Guide mis de côté à l’étape {step} : reprenez-le par le conseil, l’aide (F1) ou Menu → Tutoriel."
                )));
            }
            "quit" => {
                tutorial.finish();
                toasts.write(crate::ui::kit::modal::ToastRequest::new(
                    "Tutoriel passé : Menu → Tutoriel ou l’aide (F1) pour le relancer.",
                ));
            }
            _ => {}
        }
    }
}

/// Everything the interface-driven steps read.
#[derive(bevy::ecs::system::SystemParam)]
struct UiSignals<'w> {
    selected: Res<'w, SelectedArmy>,
    province: Res<'w, ProvincePanel>,
    diplomacy: Option<Res<'w, DiplomacyUi>>,
    chronicle: Res<'w, ChroniclePanel>,
    queue: Res<'w, ModalQueue>,
}

fn objective_met(tutorial: &Tutorial, sim: &Sim, ui: &UiSignals) -> bool {
    let (Some(data), Some(state)) = (&sim.data, &sim.state) else {
        return false;
    };
    let id = tutorial.step_id();
    if let Some(met) = sim_objective_met(id, &tutorial.snapshot, state) {
        return met;
    }
    match id {
        "select_army" => ui
            .selected
            .0
            .as_ref()
            .is_some_and(|army| player_armies(state).iter().any(|a| a == army.as_str())),
        "open_province" => ui
            .province
            .open
            .as_deref()
            .and_then(|p| ProvinceId::new(p).ok())
            .is_some_and(|p| state.province_owner(&p) == Some(state.player_faction())),
        "city_tab" => tutorial.tab_buildings_seen,
        "diplomacy" => ui.diplomacy.as_ref().is_some_and(|d| d.open),
        "season_report" => {
            ui.queue.active_key() != REPORT_KEY && !ui.queue.pending_keys().contains(&REPORT_KEY)
        }
        "chronicle" => ui.chronicle.open,
        _ => {
            let _ = data;
            false
        }
    }
}

fn check_objective(
    mut tutorial: ResMut<Tutorial>,
    sim: Res<Sim>,
    ui: UiSignals,
    time: Res<Time<Real>>,
) {
    let Some(index) = tutorial.index else { return };
    let manual = tutorial.steps.get(index).is_some_and(|s| s.manual);
    if let Some(left) = tutorial.done_timer {
        let left = left - time.delta_secs();
        if left <= 0.0 {
            tutorial.done_timer = None;
            tutorial.advance(&sim);
        } else {
            tutorial.done_timer = Some(left);
        }
        return;
    }
    tutorial.check_timer -= time.delta_secs();
    if tutorial.check_timer > 0.0 || manual {
        return;
    }
    tutorial.check_timer = CHECK_INTERVAL;
    if objective_met(&tutorial, &sim, &ui) {
        let id = tutorial.step_id().to_owned();
        tutorial.completed.push(id);
        tutorial.done_timer = Some(DONE_PAUSE);
    }
}

fn rebuild(
    mut commands: Commands,
    mut tutorial: ResMut<Tutorial>,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    paused: Option<Res<GamePaused>>,
    settings_screen: Option<Res<SettingsScreen>>,
) {
    let covered = paused.is_some_and(|p| p.0) || settings_screen.is_some_and(|s| s.root.is_some());
    let want = tutorial.index.filter(|_| !covered);
    let key = want.map(|i| (i, tutorial.done_timer.is_some()));
    if key == tutorial.built_for && (tutorial.root.is_some() == key.is_some()) {
        return;
    }
    if let Some(root) = tutorial.root.take() {
        commands.entity(root).despawn();
    }
    tutorial.built_for = key;
    let Some(index) = want else { return };
    let Some(step) = tutorial.steps.get(index).cloned() else {
        return;
    };
    let done = tutorial.done_timer.is_some();
    let total = tutorial.steps.len();
    tutorial.root = Some(spawn(
        &mut commands,
        &theme,
        &assets,
        &step,
        index,
        total,
        done,
    ));
}

fn spawn(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    step: &TutorialStep,
    index: usize,
    total: usize,
    done: bool,
) -> Entity {
    let theme = theme.clone();
    let body = RichStyle::body(&theme);
    commands
        .spawn((
            Name::new("TutorialCard"),
            TutorialCard,
            DespawnOnExit(AppState::Campaign),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(120.0),
                ..default()
            },
            UiTier::Tutorial.index(),
        ))
        .with_children(|slot| {
            slot.spawn(panel_sized(&theme, CARD_WIDTH))
                .with_children(|page| {
                    page.spawn(caption(
                        &theme,
                        &format!("Guide — étape {} sur {}", index + 1, total),
                    ));
                    page.spawn((
                        Name::new("TutorialTitle"),
                        typed(&theme, &step.title, UiType::Heading),
                    ));
                    page.spawn(rich_text(&theme, assets, &step.text, &body));
                    page.spawn(separator(&theme));
                    page.spawn((
                        Name::new("TutorialObjective"),
                        Text::new(if done {
                            format!("✔ {}", step.objective)
                        } else {
                            format!("Objectif : {}", step.objective)
                        }),
                        theme.text_font(true, false, UiType::Body.size()),
                        TextColor(if done { style::GOOD } else { style::RUBRIC }),
                    ));
                    if !step.advice.is_empty() {
                        page.spawn(caption(
                            &theme,
                            &format!("Conseil historique : {}", step.advice),
                        ));
                    }
                    page.spawn(Node {
                        column_gap: Val::Px(6.0),
                        flex_wrap: FlexWrap::Wrap,
                        row_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|row| {
                        if step.manual {
                            let label = if index + 1 >= total {
                                "Terminer"
                            } else {
                                "Continuer"
                            };
                            row.spawn(button_primary(&theme, "tutorial:next", label));
                        } else {
                            let mut skip =
                                row.spawn(button_small(&theme, "tutorial:skip", "Passer l’étape"));
                            if done {
                                skip.insert(InteractionDisabled);
                            }
                        }
                        row.spawn(button_small(&theme, "tutorial:later", "Plus tard"));
                        row.spawn(button_small(&theme, "tutorial:quit", "Passer le tutoriel"));
                    });
                });
        })
        .id()
}

fn dump(world: &World) -> (String, Value) {
    let tutorial = world.resource::<Tutorial>();
    (
        "progress_tutorial".to_owned(),
        json!({
            "loaded": tutorial.raw.is_some(),
            "steps": tutorial.steps.len(),
            "active": tutorial.active(),
            "step": tutorial.index,
            "step_id": tutorial.step_id(),
            "postponed": tutorial.postponed,
            "finished": tutorial.finished,
            "visible": tutorial.root.is_some(),
            "completed": tutorial.completed,
            "skipped": tutorial.skipped,
            "started_at": tutorial.started_at,
            "title": tutorial.index.and_then(|i| tutorial.steps.get(i)).map(|s| s.title.clone()),
        }),
    )
}
