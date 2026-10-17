//! Client-side history stored in the save file (ADR 0456).
//!
//! The core's `CampaignState` keeps only the last turn's events; the journal (« Journal » and
//! « Monde » lines), the sealed letters and the feudal events list are built by the client turn
//! after turn. They travel in the `client` field of the save wrapper (`game_api::saves`), absent
//! from older saves (they then load with an empty history, as before).
//!
//! `save_now` has no access to the ECS world, so a process-wide mirror ([`publish`]) holds the
//! last published history, like the played-battles log (`flow::history`). A load stashes the
//! history of the file ([`stash_loaded`]); the campaign screen's `OnEnter` systems pick it up
//! ([`restore_chronicle`], [`restore_feudal`]). Entering the campaign again without a load (back
//! from a battle) keeps the running history; only a new game ([`mark_new_game`]) empties it.
//!
//! The tutorial progress and the advisor's heard lines are not here: they are player
//! preferences, kept in the settings file and `advisor_seen.json`, not part of one campaign.

use std::sync::Mutex;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::campaign::panels::chronicle::log::ChronicleLog;
use crate::campaign::panels::feudal::{FeudalUi, RecentEvent};

/// What the save wrapper's `client` field holds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientExtras {
    #[serde(default)]
    pub chronicle: Option<ChronicleLog>,
    /// Feudal events of the latest turns, newest first: `(date, text)`.
    #[serde(default)]
    pub feudal_recent: Vec<(String, String)>,
}

struct Handoff {
    /// Last history published by the running campaign (what `save_now` writes).
    mirror: Value,
    /// History of a save just loaded; each consumer takes its part once.
    loaded: ClientExtras,
    /// What the next campaign screen does with the chronicle: restore, empty or keep.
    chronicle: Entry,
    feudal: Entry,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// A save was loaded: take its history.
    Restore,
    /// A new game (or the first campaign): start empty.
    Empty,
    /// Back from a battle: keep the running history.
    Keep,
}

static HANDOFF: Mutex<Handoff> = Mutex::new(Handoff {
    mirror: Value::Null,
    loaded: ClientExtras {
        chronicle: None,
        feudal_recent: Vec::new(),
    },
    chronicle: Entry::Empty,
    feudal: Entry::Empty,
});

fn handoff() -> std::sync::MutexGuard<'static, Handoff> {
    HANDOFF
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The history `save_now` writes (`Null` when none was published).
pub fn snapshot() -> Value {
    handoff().mirror.clone()
}

/// A save was loaded: its history replaces the mirror and waits for the campaign screen.
pub fn stash_loaded(client: &Value) {
    let extras: ClientExtras = if client.is_null() {
        ClientExtras::default()
    } else {
        serde_json::from_value(client.clone()).unwrap_or_else(|error| {
            warn!("save history unreadable, ignored: {error}");
            ClientExtras::default()
        })
    };
    let mut handoff = handoff();
    handoff.mirror = serde_json::to_value(&extras).unwrap_or(Value::Null);
    handoff.loaded = extras;
    handoff.chronicle = Entry::Restore;
    handoff.feudal = Entry::Restore;
}

/// A new game starts: the next campaign screen begins with an empty history.
pub fn mark_new_game() {
    let mut handoff = handoff();
    handoff.mirror = Value::Null;
    handoff.loaded = ClientExtras::default();
    handoff.chronicle = Entry::Empty;
    handoff.feudal = Entry::Empty;
}

/// `OnEnter(Campaign)` of the chronicle: restores the loaded history (empty for an older save),
/// empties it for a new game, keeps it otherwise.
pub fn restore_chronicle(log: &mut ChronicleLog) {
    let mut handoff = handoff();
    match std::mem::replace(&mut handoff.chronicle, Entry::Keep) {
        Entry::Restore => *log = handoff.loaded.chronicle.take().unwrap_or_default(),
        Entry::Empty => *log = ChronicleLog::default(),
        Entry::Keep => {}
    }
}

/// `OnEnter(Campaign)` of the feudal panel (same rule as [`restore_chronicle`]).
pub fn restore_feudal(ui: &mut FeudalUi) {
    let mut handoff = handoff();
    match std::mem::replace(&mut handoff.feudal, Entry::Keep) {
        Entry::Restore => {
            *ui = FeudalUi::default();
            ui.recent = std::mem::take(&mut handoff.loaded.feudal_recent)
                .into_iter()
                .map(|(date, text)| RecentEvent { date, text })
                .collect();
        }
        Entry::Empty => *ui = FeudalUi::default(),
        Entry::Keep => {}
    }
}

/// Keeps the mirror current: runs while the campaign is shown, cheap unless something changed.
fn publish(log: Res<ChronicleLog>, feudal: Res<FeudalUi>) {
    if !log.is_changed() && !feudal.is_changed() {
        return;
    }
    let extras = ClientExtras {
        chronicle: Some(log.clone()),
        feudal_recent: feudal
            .recent
            .iter()
            .map(|e| (e.date.clone(), e.text.clone()))
            .collect(),
    };
    handoff().mirror = serde_json::to_value(&extras).unwrap_or(Value::Null);
}

pub struct SaveExtrasPlugin;

impl Plugin for SaveExtrasPlugin {
    fn build(&self, app: &mut App) {
        use crate::app::AppState;
        app.add_systems(OnEnter(AppState::FactionSelect), mark_new_game)
            .add_systems(PostUpdate, publish.run_if(in_state(AppState::Campaign)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_history_survives_the_save_wrapper() {
        let mut log = ChronicleLog::default();
        log.lines.push(("war".into(), "[b]Bataille[/b]".into()));
        log.turns = 3;
        let extras = ClientExtras {
            chronicle: Some(log),
            feudal_recent: vec![("Printemps 1337".into(), "Hommage".into())],
        };
        let value = serde_json::to_value(&extras).unwrap();
        let back: ClientExtras = serde_json::from_value(value).unwrap();
        assert_eq!(back, extras);
        // Older saves: no field at all.
        let empty: ClientExtras = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(empty, ClientExtras::default());
    }
}
