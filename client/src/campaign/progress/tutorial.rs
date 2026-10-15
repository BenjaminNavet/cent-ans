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
//! The target of each step (`target`) gets a pulsing frame and an arrow (`target.rs`); « ☰ Étapes »
//! opens the table of contents, a click on a step jumps there (`jump_to`, buttons
//! `tutorial:toc`, `tutorial:step:<i>`); the camera moves to the royal army / the capital at the
//! steps « Votre armée » and « La province » (`target::focus_for`). Progress persists in
//! `Settings` (`tutorial_step`, `tutorial_done`, `tutorial_postponed`: `tutorial/step|done|
//! postponed` of Godot) and the guide resumes where it was left.
//!
//! Not ported: the encyclopedia (Codex lot).
//! Dev hooks: `CENT_ANS_TUTORIAL=<step index>` starts the guide at that step; `CENT_ANS_TUTORIAL=off`
//! disables the autostart.

use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use data_model::ProvinceId;
use game_api::progress::{
    TutorialSnapshot, TutorialStep, player_armies, royal_army, sim_objective_met,
    tutorial_snapshot, tutorial_steps,
};
use serde_json::{Value, json};

use super::StartTutorial;
use super::target::focus_for;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::armies::{ArmyViews, SelectedArmy};
use crate::campaign::camera::FocusCamera;
use crate::campaign::panels::chronicle::ChroniclePanel;
use crate::campaign::panels::diplomacy::DiplomacyUi;
use crate::campaign::panels::province::ProvincePanel;
use crate::campaign::provinces::data::ProvinceMap;
use crate::menu::pause::GamePaused;
use crate::paths;
use crate::settings::{Settings, SettingsFile, SettingsScreen};
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
    built_for: Option<(usize, bool, bool)>,
    /// Steps completed by their objective, in order (dump).
    pub completed: Vec<String>,
    pub skipped: u32,
    started_at: Option<usize>,
    /// Camera focuses sent (dump) and the last one.
    focuses: u32,
    last_focus: String,
    /// The table of contents is open.
    pub toc_open: bool,
    /// The royal army of the player (the arrow and the camera aim at it).
    royal: Option<String>,
    /// The camera must move to the focus of the current step.
    focus_pending: bool,
    /// Progress was read from the settings (and may now be written back).
    synced: bool,
    /// Progress last written to the settings: (step, done, postponed).
    persisted: Option<(u32, bool, bool)>,
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
                focus_camera,
                watch_tabs,
                handle_buttons,
                check_objective,
                rebuild,
                persist,
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
        self.toc_open = false;
        self.focus_pending = true;
        if let (Some(data), Some(state)) = (&sim.data, &sim.state) {
            self.snapshot = tutorial_snapshot(state);
            self.royal = royal_army(data, state);
        }
    }

    /// Sommaire: goes straight to step `index`.
    fn jump_to(&mut self, sim: &Sim, index: usize) {
        if self.active() && index < self.steps.len() {
            self.enter(sim, index);
        }
    }

    /// Target of the current step and whether its arrow shows (not while the objective is met).
    pub fn arrow_target(&self) -> (&str, bool) {
        let step = self.index.and_then(|i| self.steps.get(i));
        let name = step.map_or("", |s| s.target.as_str());
        let show = self.root.is_some() && self.done_timer.is_none() && !name.is_empty();
        (name, show)
    }

    /// The royal army the arrow aims at.
    pub fn royal_army(&self) -> Option<&str> {
        self.royal.as_deref()
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

impl Tutorial {
    /// Reads the progress saved in the settings (once): `tutorial/done`, `tutorial/postponed`.
    fn sync_settings(&mut self, settings: &Settings) {
        self.synced = true;
        self.finished = settings.tutorial_done;
        self.postponed = settings
            .tutorial_postponed
            .then_some(settings.tutorial_step as usize);
        self.persisted = Some((
            settings.tutorial_step,
            settings.tutorial_done,
            settings.tutorial_postponed,
        ));
    }

    /// What the settings should hold now: (step, done, postponed). Finished: step 0.
    fn progress(&self) -> (u32, bool, bool) {
        let step = match (self.index, self.postponed) {
            (Some(i), _) => i,
            (None, Some(p)) => p,
            _ => 0,
        };
        (step as u32, self.finished, self.postponed.is_some())
    }
}

/// Starts the guide on the first turns, once the briefing is closed; resumes it at the step saved
/// in the settings (`should_autostart`: early turns or a saved step above 0).
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
    if !tutorial.synced {
        match &settings {
            Some(s) => tutorial.sync_settings(s),
            None => tutorial.synced = true,
        }
        if tutorial.finished || tutorial.postponed.is_some() {
            tutorial.autostart_done = true;
            return;
        }
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
    let saved = settings.as_ref().map_or(0, |s| s.tutorial_step as usize);
    let turn = sim.state.as_ref().map_or(u32::MAX, |s| s.turn());
    let from = hook.as_deref().and_then(|h| h.parse::<usize>().ok());
    if let Some(from) = from {
        tutorial.start(&sim, from);
    } else if enabled && !tutorial.finished && (turn <= EARLY_TURNS || saved > 0) {
        tutorial.start(&sim, saved);
    }
}

/// Writes the progress back to the settings file when it changed (not under the dev hook).
fn persist(
    tutorial: Res<Tutorial>,
    settings: Option<ResMut<Settings>>,
    file: Option<ResMut<SettingsFile>>,
) {
    if !tutorial.synced || !tutorial.autostart_done {
        return;
    }
    if std::env::var("CENT_ANS_TUTORIAL").is_ok() {
        return;
    }
    let (Some(mut settings), Some(mut file)) = (settings, file) else {
        return;
    };
    let want = tutorial.progress();
    if tutorial.persisted == Some(want) {
        return;
    }
    // No change detection: the other settings systems have nothing to redo.
    let inner = settings.bypass_change_detection();
    inner.tutorial_step = want.0;
    inner.tutorial_done = want.1;
    inner.tutorial_postponed = want.2;
    file.save(inner);
}

/// Moves the camera at the start of the steps that ask for it.
fn focus_camera(
    mut tutorial: ResMut<Tutorial>,
    sim: Res<Sim>,
    views: Res<ArmyViews>,
    map: Option<Res<ProvinceMap>>,
    mut focus: MessageWriter<FocusCamera>,
) {
    if !tutorial.focus_pending || !tutorial.root.is_some() && tutorial.index.is_none() {
        return;
    }
    let Some(index) = tutorial.index else { return };
    let id = tutorial.steps.get(index).map_or("", |s| s.id.as_str());
    let army_point = tutorial.royal.as_deref().and_then(|royal| {
        views
            .0
            .iter()
            .find(|v| v.id.as_str() == royal)
            .map(|v| Vec2::new(v.point[0], v.point[1]))
    });
    let capital = sim.state.as_ref().and_then(|state| {
        state
            .factions
            .get(state.player_faction())
            .map(|f| f.capital.to_string())
    });
    let ready = map.is_some() && !views.0.is_empty();
    let request = focus_for(id, army_point, capital.as_deref());
    if request.is_none() && !ready && matches!(id, "select_army" | "open_province") {
        return; // the map is not built yet: try again next frame
    }
    tutorial.focus_pending = false;
    if let Some(request) = request {
        tutorial.last_focus = format!("{request:?}");
        tutorial.focuses += 1;
        focus.write(request);
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
        if name == "toc" {
            tutorial.toc_open = !tutorial.toc_open;
            continue;
        }
        if let Some(index) = name
            .strip_prefix("step:")
            .and_then(|i| i.parse::<usize>().ok())
        {
            tutorial.jump_to(&sim, index);
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
    let key = want.map(|i| (i, tutorial.done_timer.is_some(), tutorial.toc_open));
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
    let titles: Vec<String> = if tutorial.toc_open {
        tutorial.steps.iter().map(|s| s.title.clone()).collect()
    } else {
        Vec::new()
    };
    let total = tutorial.steps.len();
    tutorial.root = Some(spawn(
        &mut commands,
        &theme,
        &assets,
        &step,
        (index, total),
        done,
        &titles,
    ));
}

fn spawn(
    commands: &mut Commands,
    theme: &UiTheme,
    assets: &AssetServer,
    step: &TutorialStep,
    (index, total): (usize, usize),
    done: bool,
    toc: &[String],
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
                    page.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn(caption(
                            &theme,
                            &format!("Guide — étape {} sur {}", index + 1, total),
                        ));
                        header.spawn(button_small(&theme, "tutorial:toc", "☰ Étapes"));
                    });
                    if !toc.is_empty() {
                        page.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            ..default()
                        })
                        .with_children(|list| {
                            for (i, title) in toc.iter().enumerate() {
                                let mark = match i.cmp(&index) {
                                    std::cmp::Ordering::Less => "✔",
                                    std::cmp::Ordering::Equal => "▸",
                                    std::cmp::Ordering::Greater => " ",
                                };
                                list.spawn(button_small(
                                    &theme,
                                    &format!("tutorial:step:{i}"),
                                    &format!("{mark} {}. {title}", i + 1),
                                ));
                            }
                        });
                        page.spawn(separator(&theme));
                    }
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
            "toc_open": tutorial.toc_open,
            "royal": tutorial.royal,
            "focuses": tutorial.focuses,
            "last_focus": tutorial.last_focus,
            "persisted": tutorial.persisted,
            "progress": tutorial.progress(),
            "title": tutorial.index.and_then(|i| tutorial.steps.get(i)).map(|s| s.title.clone()),
        }),
    )
}
