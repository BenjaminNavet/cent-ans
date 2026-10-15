//! End-of-turn flow: readiness check, confirmation, the turn resolved on a worker thread (AI
//! included, exactly `godot-bridge` `turn_job.rs::resolve_turn`), then the season report.

use std::sync::Arc;
use std::thread::JoinHandle;

use bevy::prelude::*;
use data_model::GameData;
use sim_campaign::{CampaignState, GameEvent};

use super::decision::OpenDecision;
use super::report;
use super::{HudData, HudSet};
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::modal::{
    ModalQueue, ModalResult, ModalSpec, PRIORITY_DECISION, PRIORITY_REPORT, ToastRequest,
};

/// The AI planner recurses on route tables: the worker needs a big stack (8 MiB on main).
const WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;
const CONFIRM_KEY: &str = "hud:confirm_end_turn";
const REPORT_KEY: &str = "hud:season_report";

type Outcome = (CampaignState, Vec<GameEvent>);

/// The turn resolved on its own thread, on a clone of the state.
pub struct TurnJob {
    handle: JoinHandle<Outcome>,
}

impl TurnJob {
    pub fn spawn(state: CampaignState, data: Arc<GameData>) -> std::io::Result<Self> {
        let mut state = state;
        let handle = std::thread::Builder::new()
            .name("cent-ans-end-turn".to_owned())
            .stack_size(WORKER_STACK_BYTES)
            .spawn(move || {
                let events = state.end_turn_with(&data, ai::plan_turn);
                (state, events)
            })?;
        Ok(Self { handle })
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub fn join(self) -> Result<Outcome, String> {
        self.handle.join().map_err(|panic| {
            panic
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panique sans message".to_owned())
        })
    }
}

/// Asks for the end of the season (button, Enter, scripts). Runs the readiness check.
#[derive(Message, Debug, Clone, Copy)]
pub struct EndTurnRequested;

/// A turn was resolved and installed in `Sim`.
#[derive(Message, Debug, Clone)]
pub struct TurnAdvanced {
    pub turn: u32,
    pub events: Vec<GameEvent>,
}

#[derive(Resource, Default)]
pub struct TurnFlow {
    job: Option<TurnJob>,
    /// Number of turns played by this client.
    pub turns_played: u32,
    /// Events of the last resolved turn.
    pub last_events: Vec<GameEvent>,
    /// Entries of the last season report, per rubric.
    pub last_report_counts: [usize; 5],
    /// Last failure message (worker panic).
    pub error: String,
    /// The confirmation modal was shown for the last request.
    pub confirm_shown: bool,
}

impl TurnFlow {
    pub fn running(&self) -> bool {
        self.job.is_some()
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<TurnFlow>()
        .add_message::<EndTurnRequested>()
        .add_message::<TurnAdvanced>()
        .add_systems(
            Update,
            (
                request_from_inputs,
                handle_request,
                handle_confirmation,
                poll_turn,
            )
                .chain()
                .in_set(HudSet::Flow)
                .run_if(in_state(AppState::Campaign)),
        );
}

fn request_from_inputs(
    mut pressed: MessageReader<ButtonPressed>,
    keys: Res<ButtonInput<KeyCode>>,
    mut requests: MessageWriter<EndTurnRequested>,
) {
    let clicked = pressed.read().any(|p| p.name == "hud:end_turn");
    if clicked || keys.just_pressed(KeyCode::Enter) {
        requests.write(EndTurnRequested);
    }
}

fn handle_request(
    mut requests: MessageReader<EndTurnRequested>,
    sim: Res<Sim>,
    data_view: Res<HudData>,
    mut flow: ResMut<TurnFlow>,
    mut queue: ResMut<ModalQueue>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    if requests.read().next().is_none() {
        return;
    }
    requests.clear();
    // A modal in front (or a turn running) swallows the request, as the Godot bell does.
    if flow.running() || queue.is_busy() {
        return;
    }
    let Some(readiness) = data_view.fresh_readiness(&sim) else {
        return;
    };
    flow.confirm_shown = false;
    if readiness.blocked {
        let waiting: Vec<String> = readiness
            .alerts
            .iter()
            .filter(|a| a.blocking)
            .map(|a| format!("• {}", a.text))
            .collect();
        queue.request(
            CONFIRM_KEY,
            PRIORITY_DECISION,
            ModalSpec::confirm(
                "Terminer la saison ?",
                &format!(
                    "Des décisions attendent encore :\n{}\n\nSi vous terminez la saison, le conseil tranchera à votre place.",
                    waiting.join("\n")
                ),
            ),
        );
        flow.confirm_shown = true;
        return;
    }
    start_turn(&sim, &mut flow, &mut toasts);
}

fn handle_confirmation(
    mut results: MessageReader<ModalResult>,
    sim: Res<Sim>,
    mut flow: ResMut<TurnFlow>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for result in results.read() {
        if result.key == CONFIRM_KEY && result.confirmed && !flow.running() {
            start_turn(&sim, &mut flow, &mut toasts);
        }
    }
}

fn start_turn(sim: &Sim, flow: &mut TurnFlow, toasts: &mut MessageWriter<ToastRequest>) {
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.clone()) else {
        return;
    };
    match TurnJob::spawn(state, data) {
        Ok(job) => flow.job = Some(job),
        Err(error) => {
            flow.error = error.to_string();
            toasts.write(ToastRequest::new("Impossible de lancer la fin de saison."));
        }
    }
}

fn poll_turn(
    mut flow: ResMut<TurnFlow>,
    mut sim: ResMut<Sim>,
    mut queue: ResMut<ModalQueue>,
    mut advanced: MessageWriter<TurnAdvanced>,
    mut decisions: MessageWriter<OpenDecision>,
    mut toasts: MessageWriter<ToastRequest>,
    mut data_view: ResMut<HudData>,
) {
    if !flow.job.as_ref().is_some_and(TurnJob::is_finished) {
        return;
    }
    let Some(job) = flow.job.take() else { return };
    match job.join() {
        Err(message) => {
            // The state of before stays: the turn is simply not played.
            error!("end of turn failed: {message}");
            flow.error = message;
            toasts.write(ToastRequest::new("La fin de saison a échoué."));
        }
        Ok((state, events)) => {
            let player = sim.player.as_str().to_owned();
            let interest = data_view.interest_factions();
            let turn = state.turn();
            let date = state.date_label();
            sim.state = Some(state);
            flow.turns_played += 1;
            let rep = report::build(&events, &player, &interest);
            for (slot, group) in flow.last_report_counts.iter_mut().zip(&rep.groups) {
                *slot = group.len();
            }
            queue.request(
                REPORT_KEY,
                PRIORITY_REPORT,
                ModalSpec::info(
                    &format!("Rapport de saison — {date}"),
                    &report::bbcode(&rep),
                ),
            );
            flow.last_events = events.clone();
            data_view.force_refresh();
            advanced.write(TurnAdvanced { turn, events });
            decisions.write(OpenDecision::FirstBlocking);
        }
    }
}
