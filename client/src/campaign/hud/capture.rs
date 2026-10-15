//! Fate of a captured place (ports game/scripts/map/capture_controller.gd, TW2-T1, ADR 0101).
//!
//! When the player takes a place the core queues a [`PendingCapture`]; this window offers the four
//! outcomes (occupy, ransom, sack, raze) with their figures (`CampaignState::capture_decision_views`)
//! and answers with `Order::ChooseCaptureOutcome`. Razing is greyed out with its reason on a city.
//! « Plus tard » puts the window aside: it is not reopened for the same capture, and unanswered
//! captures are simply occupied at the end of the turn (core rule).
//!
//! Dev hook: `CENT_ANS_DEBUG_CAPTURE=<settlement id | auto>` takes that place once the campaign
//! starts (`auto`: the first foreign city), so the window can be checked without a siege.

use std::collections::BTreeSet;

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};
use sim_campaign::Order;
use sim_campaign::capture::{CaptureDecisionView, CaptureOutcome};

use super::HudSet;
use super::flow::orders_open;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::sim::Sim;
use crate::ui::kit::modal::ToastRequest;
use crate::ui::kit::style::{UiTier, UiType};
use crate::ui::kit::widgets::{button_small, caption, panel_sized};
use crate::ui::kit::{ButtonPressed, UiTheme, typed};

const ENV_DEBUG: &str = "CENT_ANS_DEBUG_CAPTURE";
const LATER: &str = "hud:capture_later";
const OPTION_PREFIX: &str = "hud:capture_option:";

#[derive(Component)]
struct CaptureWindow {
    id: u32,
}

/// Captures already offered (not reopened after « Plus tard ») and counters for the dump.
#[derive(Resource, Default)]
pub struct CaptureOffers {
    offered: BTreeSet<u32>,
    pub opened: u32,
    pub chosen: u32,
    pub last_outcome: Option<String>,
    debug_done: bool,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<CaptureOffers>()
        .register_dump(dump_capture)
        .add_systems(OnEnter(AppState::Campaign), reset_offers)
        .add_systems(
            Update,
            (debug_capture, offer_pending, handle_buttons)
                .chain()
                .in_set(HudSet::Flow)
                .run_if(in_state(AppState::Campaign))
                .run_if(orders_open),
        );
}

fn reset_offers(mut offers: ResMut<CaptureOffers>) {
    *offers = CaptureOffers::default();
}

/// Captures waiting for the player's choice.
pub fn pending(sim: &Sim) -> Vec<CaptureDecisionView> {
    match (&sim.data, &sim.state) {
        (Some(data), Some(state)) => state.capture_decision_views(data, &sim.player),
        _ => Vec::new(),
    }
}

/// Outcome of a button name (`hud:capture_option:<snake_case outcome>`).
pub fn outcome_of(button: &str) -> Option<CaptureOutcome> {
    let key = button.strip_prefix(OPTION_PREFIX)?;
    CaptureOutcome::ALL.into_iter().find(|o| o.key() == key)
}

fn debug_capture(mut sim: ResMut<Sim>, mut offers: ResMut<CaptureOffers>) {
    if offers.debug_done {
        return;
    }
    let Ok(wanted) = std::env::var(ENV_DEBUG) else {
        offers.debug_done = true;
        return;
    };
    let Some(data) = sim.data.clone() else { return };
    let player = sim.player.clone();
    let Some(state) = sim.state.as_mut() else {
        return;
    };
    offers.debug_done = true;
    let target = if wanted == "auto" {
        state
            .settlements
            .iter()
            .find(|(id, s)| {
                s.controller != player && state.province_city_id(&s.province) == Some(*id)
            })
            .map(|(id, _)| id.clone())
    } else {
        state
            .settlements
            .keys()
            .find(|id| id.as_str() == wanted)
            .cloned()
    };
    match target {
        Some(id) => {
            let taken = sim_campaign::capture::debug_capture(state, &data, &id, &player);
            info!("{ENV_DEBUG}: {id} taken = {taken}");
        }
        None => warn!("{ENV_DEBUG}: no settlement `{wanted}`"),
    }
}

/// Opens the window on the first capture not yet offered, when no window is up.
fn offer_pending(
    mut commands: Commands,
    sim: Res<Sim>,
    mut offers: ResMut<CaptureOffers>,
    theme: Res<UiTheme>,
    windows: Query<(Entity, &CaptureWindow)>,
) {
    let views = pending(&sim);
    // A window whose capture is gone (end of turn, answered elsewhere) closes.
    for (entity, window) in &windows {
        if !views.iter().any(|v| v.id == window.id) {
            commands.entity(entity).despawn();
        }
    }
    if !windows.is_empty() {
        return;
    }
    let Some(view) = views.iter().find(|v| !offers.offered.contains(&v.id)) else {
        return;
    };
    offers.offered.insert(view.id);
    offers.opened += 1;
    spawn_window(&mut commands, &theme, view, views.len());
}

fn spawn_window(
    commands: &mut Commands,
    theme: &UiTheme,
    view: &CaptureDecisionView,
    queued: usize,
) {
    commands
        .spawn((
            Name::new("hud:capture_window"),
            CaptureWindow { id: view.id },
            DespawnOnExit(AppState::Campaign),
            UiTier::Modal.index(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::default(),
        ))
        .with_children(|root| {
            root.spawn((panel_sized(theme, 620.0), Pickable::default()))
                .with_children(|card| {
                    card.spawn(typed(theme, &view.title, UiType::Heading));
                    let sub = if queued > 1 {
                        format!("{} ({} places en attente)", view.province_name, queued)
                    } else {
                        view.province_name.clone()
                    };
                    card.spawn(caption(theme, &sub));
                    card.spawn(typed(theme, &view.text, UiType::Body));
                    for option in &view.options {
                        let name = format!("{OPTION_PREFIX}{}", option.outcome.key());
                        card.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            align_items: AlignItems::FlexStart,
                            ..default()
                        })
                        .with_children(|entry| {
                            let mut button = entry.spawn(button_small(theme, &name, &option.label));
                            if !option.allowed {
                                button.insert(bevy::ui::InteractionDisabled);
                            }
                            if !option.effects_text.is_empty() {
                                entry.spawn(caption(theme, &option.effects_text));
                            }
                            if let Some(reason) = &option.reason {
                                entry.spawn(caption(theme, reason));
                            }
                        });
                    }
                    card.spawn(button_small(theme, LATER, "Plus tard"));
                });
        });
}

fn handle_buttons(
    mut pressed: MessageReader<ButtonPressed>,
    mut commands: Commands,
    mut sim: ResMut<Sim>,
    mut offers: ResMut<CaptureOffers>,
    windows: Query<(Entity, &CaptureWindow)>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    for press in pressed.read() {
        let Ok((window, capture)) = windows.single() else {
            continue;
        };
        if press.name == LATER {
            commands.entity(window).despawn();
            continue;
        }
        let Some(outcome) = outcome_of(&press.name) else {
            continue;
        };
        let Some(data) = sim.data.clone() else {
            continue;
        };
        let Some(state) = sim.state.as_mut() else {
            continue;
        };
        match state.submit_order_outcome(
            &data,
            Order::ChooseCaptureOutcome {
                decision: capture.id,
                outcome,
            },
        ) {
            Ok(_) => {
                offers.chosen += 1;
                offers.last_outcome = Some(outcome.key().to_owned());
                toasts.write(ToastRequest::new(&format!(
                    "Ordre donné : {}.",
                    outcome.label_fr().to_lowercase()
                )));
                commands.entity(window).despawn();
            }
            Err(error) => {
                toasts.write(ToastRequest::new(&error.to_string()));
            }
        }
    }
}

fn dump_capture(world: &World) -> (String, Value) {
    let offers = world.resource::<CaptureOffers>();
    let sim = world.resource::<Sim>();
    let views = pending(sim);
    let windows = world
        .try_query::<&CaptureWindow>()
        .map(|mut q| q.iter(world).map(|w| w.id).collect::<Vec<_>>())
        .unwrap_or_default();
    (
        "hud_capture".into(),
        json!({
            "pending": views.iter().map(|v| json!({
                "id": v.id,
                "place": v.settlement_name,
                "province": v.province_name,
                "options": v.options.iter().map(|o| json!({
                    "outcome": o.outcome.key(),
                    "allowed": o.allowed,
                    "gold": o.effects.gold,
                    "unrest": o.effects.unrest,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "windows_open": windows,
            "opened": offers.opened,
            "chosen": offers.chosen,
            "last_outcome": offers.last_outcome,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_names_map_to_outcomes() {
        for outcome in CaptureOutcome::ALL {
            let name = format!("{OPTION_PREFIX}{}", outcome.key());
            assert_eq!(outcome_of(&name), Some(outcome));
        }
        assert_eq!(outcome_of(LATER), None);
        assert_eq!(outcome_of("hud:capture_option:nope"), None);
    }
}
