//! End-of-turn flow: readiness check, confirmation, the turn resolved on a worker thread (AI
//! included, exactly `godot-bridge` `turn_job.rs::resolve_turn`), then the season report.

use std::sync::Arc;
use std::thread::JoinHandle;

use bevy::prelude::*;
use data_model::GameData;
use sim_campaign::{AiMoveRecord, CampaignState, GameEvent};

use super::decision::OpenDecision;
use super::replay::{self, AiReplay, HeldReport};
use super::report;
use super::{HudData, HudSet};
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::settings::{KeyMap, Settings};
use crate::sim::Sim;
use crate::ui::kit::ButtonPressed;
use crate::ui::kit::modal::{
    ModalQueue, ModalResult, ModalSpec, PRIORITY_DECISION, PRIORITY_REPORT, ToastRequest,
};
use crate::ui::kit::style::UiTier;
use crate::ui::kit::text_field::{TextInputFocus, keyboard_free};

/// The AI planner recurses on route tables: the worker needs a big stack (8 MiB on main).
const WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;
const CONFIRM_KEY: &str = "hud:confirm_end_turn";
const REPORT_KEY: &str = "hud:season_report";

type Outcome = (CampaignState, Vec<GameEvent>, Vec<AiMoveRecord>);

/// The turn resolved on its own thread, on a clone of the state.
pub struct TurnJob {
    handle: JoinHandle<Outcome>,
}

impl TurnJob {
    /// `record_ai`: the core also records the AI armies' moves (`Some(notable radius in km)`).
    pub fn spawn(
        state: CampaignState,
        data: Arc<GameData>,
        record_ai: Option<f64>,
    ) -> std::io::Result<Self> {
        let mut state = state;
        if let Some(radius_km) = record_ai {
            state.set_ai_replay_recording(true, radius_km);
        }
        let handle = std::thread::Builder::new()
            .name("cent-ans-end-turn".to_owned())
            .stack_size(WORKER_STACK_BYTES)
            .spawn(move || {
                let events = state.end_turn_with(&data, ai::plan_turn);
                let moves = state.ai_turn_moves().to_vec();
                (state, events, moves)
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

/// Asks for the end of the season (button, Enter, scripts). Runs the readiness check; `fast`
/// (Shift+Enter, action `campaign_end_turn_fast`) skips the confirmation of pending decisions.
#[derive(Message, Debug, Clone, Copy, Default)]
pub struct EndTurnRequested {
    pub fast: bool,
}

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
    /// Frames during which `Sim` was changed by someone else while the turn was running: those
    /// orders would be overwritten by the result, the overlay and the keyboard lock avoid it.
    pub orders_dropped: u32,
    /// Orders (or other writes to `Sim`) seen during the last turn.
    pub last_dropped: u32,
}

impl TurnFlow {
    pub fn running(&self) -> bool {
        self.job.is_some()
    }
}

/// Run condition for systems that issue orders: no order while the end of turn is resolved on
/// its thread (the worker owns a clone of the state, its result replaces `Sim::state`).
pub fn orders_open(flow: Res<TurnFlow>) -> bool {
    !flow.running()
}

/// Full-screen veil that takes the clicks during the turn.
#[derive(Component)]
struct OrderLock;

pub fn plugin(app: &mut App) {
    app.init_resource::<TurnFlow>()
        .add_message::<EndTurnRequested>()
        .add_message::<TurnAdvanced>()
        .add_systems(OnEnter(AppState::Campaign), spawn_lock)
        .add_systems(
            Update,
            (
                watch_orders,
                update_lock,
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

fn spawn_lock(mut commands: Commands) {
    commands.spawn((
        Name::new("hud:order_lock"),
        DespawnOnExit(AppState::Campaign),
        OrderLock,
        GlobalZIndex(UiTier::Banner.z() + 5),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.08, 0.05, 0.02, 0.10)),
    ));
}

fn update_lock(flow: Res<TurnFlow>, mut lock: Query<&mut Node, With<OrderLock>>) {
    if let Ok(mut node) = lock.single_mut() {
        let want = if flow.running() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != want {
            node.display = want;
        }
    }
}

/// Counts the writes to `Sim` made by others while the turn runs (the open point of the first
/// HUD lot: an order given then was silently overwritten by the worker's result).
fn watch_orders(sim: Res<Sim>, mut flow: ResMut<TurnFlow>, mut last_running: Local<bool>) {
    let running = flow.running();
    if running && *last_running && sim.is_changed() {
        flow.orders_dropped += 1;
    }
    *last_running = running;
}

fn request_from_inputs(
    mut pressed: MessageReader<ButtonPressed>,
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Option<Res<KeyMap>>,
    focus: Option<Res<TextInputFocus>>,
    mut requests: MessageWriter<EndTurnRequested>,
) {
    let typing = !keyboard_free(focus);
    let clicked = pressed.read().any(|p| p.name == "hud:end_turn");
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let (key, fast_key) = match keymap.as_deref().filter(|_| !typing) {
        Some(map) => (
            map.just_pressed("campaign_end_turn", &keys),
            map.just_pressed("campaign_end_turn_fast", &keys),
        ),
        None => {
            let enter = !typing && keys.just_pressed(KeyCode::Enter);
            (enter && !shift, enter && shift)
        }
    };
    if fast_key {
        requests.write(EndTurnRequested { fast: true });
    } else if clicked || key {
        requests.write(EndTurnRequested::default());
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_request(
    mut requests: MessageReader<EndTurnRequested>,
    sim: Res<Sim>,
    data_view: Res<HudData>,
    mut flow: ResMut<TurnFlow>,
    mut queue: ResMut<ModalQueue>,
    replay: Res<AiReplay>,
    settings: Option<Res<Settings>>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    let Some(request) = requests.read().next().copied() else {
        return;
    };
    requests.clear();
    // A modal in front (or a turn running) swallows the request, as the Godot bell does.
    if flow.running() || queue.is_busy() {
        return;
    }
    let Some(readiness) = data_view.fresh_readiness(&sim) else {
        return;
    };
    flow.confirm_shown = false;
    if readiness.blocked && !request.fast {
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
    start_turn(&sim, &mut flow, &replay, settings.as_deref(), &mut toasts);
}

fn handle_confirmation(
    mut results: MessageReader<ModalResult>,
    sim: Res<Sim>,
    mut flow: ResMut<TurnFlow>,
    replay: Res<AiReplay>,
    settings: Option<Res<Settings>>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for result in results.read() {
        if result.key == CONFIRM_KEY && result.confirmed && !flow.running() {
            start_turn(&sim, &mut flow, &replay, settings.as_deref(), &mut toasts);
        }
    }
}

fn start_turn(
    sim: &Sim,
    flow: &mut TurnFlow,
    replay: &AiReplay,
    settings: Option<&Settings>,
    toasts: &mut MessageWriter<ToastRequest>,
) {
    let (Some(data), Some(state)) = (sim.data.clone(), sim.state.clone()) else {
        return;
    };
    let record = replay::recording_wanted(settings, sim).then_some(replay.tuning.notable_radius_km);
    flow.orders_dropped = 0;
    match TurnJob::spawn(state, data, record) {
        Ok(job) => flow.job = Some(job),
        Err(error) => {
            flow.error = error.to_string();
            toasts.write(ToastRequest::new("Impossible de lancer la fin de saison."));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn poll_turn(
    mut flow: ResMut<TurnFlow>,
    mut sim: ResMut<Sim>,
    mut queue: ResMut<ModalQueue>,
    mut advanced: MessageWriter<TurnAdvanced>,
    mut decisions: MessageWriter<OpenDecision>,
    mut toasts: MessageWriter<ToastRequest>,
    mut data_view: ResMut<HudData>,
    mut replay: ResMut<AiReplay>,
    settings: Option<Res<Settings>>,
    rig: Option<Res<CampaignRig>>,
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
        Ok((state, events, moves)) => {
            flow.last_dropped = flow.orders_dropped;
            if flow.orders_dropped > 0 {
                toasts.write(ToastRequest::new(
                    "Des ordres donnés pendant la résolution de la saison ont été annulés.",
                ));
            }
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
            let held = HeldReport {
                key: REPORT_KEY.to_owned(),
                spec: ModalSpec::info(
                    &format!("Rapport de saison — {date}"),
                    &report::bbcode(&rep),
                ),
            };
            let speed = settings.as_deref().map_or(1.0, |s| s.ai_moves_speed);
            let replayed = match sim.data.clone() {
                Some(data) => {
                    replay.begin(&moves, &data, replay::current_focus(rig.as_deref()), speed)
                }
                None => false,
            };
            if replayed {
                // The season report comes after the replay of the AI turn.
                replay.hold_report(held);
            } else {
                queue.request(&held.key, PRIORITY_REPORT, held.spec);
            }
            flow.last_events = events.clone();
            data_view.force_refresh();
            advanced.write(TurnAdvanced { turn, events });
            decisions.write(OpenDecision::FirstBlocking);
        }
    }
}
