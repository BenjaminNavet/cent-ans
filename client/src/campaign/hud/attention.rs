//! Probe of the attention channels (XP-AUDIENCE, R1): counts what opens by itself in front of
//! the player (toasts, advisor band messages, modals other than the end-of-season confirmation,
//! the decision and mission-offer windows, the Audience card), so the before/after of the
//! attention budget is measured by a dump and not by eye. It only reads; it never opens or
//! closes anything. A window rebuilt in the same season counts once.

use std::collections::BTreeSet;

use bevy::prelude::*;
use serde_json::{Value, json};

use super::HudData;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::ui::advisor_band::BandSay;
use crate::ui::kit::modal::{ModalQueue, ToastRequest};

/// Names of the windows that open on their own.
pub const DECISION_WINDOW: &str = "hud:decision_window";
pub const OFFER_WINDOW: &str = "hud:mission_offer";
/// The player asked for this one (end-of-season confirmation): not an unsolicited opening.
const ASKED_MODAL: &str = "hud:confirm_end_turn";

/// What opened by itself, in total and by season.
#[derive(Resource, Default, Debug, Clone)]
pub struct AttentionLog {
    pub toasts: u32,
    pub band_says: u32,
    pub modals: u32,
    pub decision_windows: u32,
    pub offer_windows: u32,
    pub audience_cards: u32,
    /// `(turn, pushes)` for each season seen, in order.
    pub per_turn: Vec<(u32, u32)>,
    pub modal_keys: Vec<String>,
    /// First words of each toast, to see which channel they come from.
    pub toast_texts: Vec<String>,
    last_total: u32,
    windows_seen: BTreeSet<(String, u32)>,
}

impl AttentionLog {
    /// Everything that opened in front of the player.
    pub fn total(&self) -> u32 {
        self.toasts
            + self.band_says
            + self.modals
            + self.decision_windows
            + self.offer_windows
            + self.audience_cards
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<AttentionLog>()
        .register_dump(dump)
        .add_systems(
            Update,
            probe
                .after(super::HudSet::Ui)
                .run_if(in_state(AppState::Campaign)),
        );
}

#[allow(clippy::too_many_arguments)]
fn probe(
    mut toasts: MessageReader<ToastRequest>,
    mut says: MessageReader<BandSay>,
    queue: Option<Res<ModalQueue>>,
    names: Query<&Name, Added<Name>>,
    hud: Res<HudData>,
    mut log: ResMut<AttentionLog>,
    mut last_modal: Local<String>,
) {
    let turn = hud.top.as_ref().map_or(0, |t| t.turn);
    for toast in toasts.read() {
        log.toasts += 1;
        log.toast_texts.push(toast.text.chars().take(60).collect());
    }
    log.band_says += says.read().count() as u32;
    if let Some(queue) = queue {
        let key = queue.active_key().to_owned();
        if key != *last_modal && !key.is_empty() && key != ASKED_MODAL {
            log.modals += 1;
            log.modal_keys.push(key.clone());
        }
        *last_modal = key;
    }
    for name in &names {
        let counter = match name.as_str() {
            DECISION_WINDOW => 0,
            OFFER_WINDOW => 1,
            _ => continue,
        };
        if log.windows_seen.insert((name.to_string(), turn)) {
            if counter == 0 {
                log.decision_windows += 1;
            } else {
                log.offer_windows += 1;
            }
        }
    }
    if hud.top.is_none() {
        return;
    }
    let total = log.total();
    let base = log.last_total;
    if let Some((_, pushes)) = log.per_turn.last_mut().filter(|(seen, _)| *seen == turn) {
        *pushes = total - base;
    } else {
        log.last_total = total;
        log.per_turn.push((turn, 0));
    }
}

fn dump(world: &World) -> (String, Value) {
    let log = world.resource::<AttentionLog>();
    (
        "attention".to_owned(),
        json!({
            "total": log.total(),
            "toasts": log.toasts,
            "band_says": log.band_says,
            "modals": log.modals,
            "modal_keys": log.modal_keys,
            "toast_texts": log.toast_texts,
            "decision_windows": log.decision_windows,
            "offer_windows": log.offer_windows,
            "audience_cards": log.audience_cards,
            "per_turn": log.per_turn,
        }),
    )
}
