//! M6 (ADR 0721): perjury and other windowed diplomatic events, read from
//! the `events` table of `data/rules/diplomacy.json`.
//!
//! No new score: an event writes the existing opinion modifier on every
//! other lord, and costs the author's ruler prestige (more when the
//! author's chivalric order stands: the knight forsworn).

use crate::diplomacy::TRUCE_TURNS;
use crate::state::CampaignState;
use data_model::{DiplomacyEvent, FactionId, GameData};

/// Event ids of the table (`data/rules/diplomacy.json`).
pub const TRUCE_BROKEN_RECENT: &str = "truce_broken_recent";
pub const TRUCE_BROKEN: &str = "truce_broken";
pub const PACT_BROKEN: &str = "pact_broken";
pub const ALLY_ATTACKED: &str = "ally_attacked";
pub const CAPTIVE_EXECUTED_ON_PAROLE: &str = "captive_executed_on_parole";

/// Prefix shared by the reasons of every perjury event.
const PERJURY_PREFIX: &str = "Parjure";

/// `true` for the reason of any perjury event (a reputation every court
/// holds, no personal grudge).
pub fn is_perjury_reason(reason: &str) -> bool {
    reason.starts_with(PERJURY_PREFIX)
}

/// Cost an event would inflict on its author, shown before acting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventPreview {
    /// Opinion change every other lord holds about the author (negative).
    pub opinion: i32,
    /// Seasons it lasts.
    pub duration: u32,
    /// Prestige the author's ruler loses (negative or zero).
    pub prestige: i32,
}

/// Seasons since `author` and `other` signed the truce running between
/// them (the standard length is assumed: the state keeps only the end).
pub fn truce_age(state: &CampaignState, author: &FactionId, other: &FactionId) -> Option<u32> {
    let until = *state.factions.get(author)?.truces.get(other)?;
    let left = until.checked_sub(state.turn).filter(|l| *l > 0)?;
    Some(TRUCE_TURNS.saturating_sub(left))
}

/// Id of the event breaking the truce between `author` and `other`: the
/// major recent one inside `truce_window` seasons of its signing.
pub fn truce_break_event(
    state: &CampaignState,
    data: &GameData,
    author: &FactionId,
    other: &FactionId,
) -> &'static str {
    match truce_age(state, author, other) {
        Some(age) if age < data.diplomacy_rules.truce_window => TRUCE_BROKEN_RECENT,
        _ => TRUCE_BROKEN,
    }
}

fn standing_order(state: &CampaignState, author: &FactionId) -> bool {
    state
        .factions
        .get(author)
        .and_then(|f| f.chivalric_order.as_ref())
        .is_some_and(|o| !o.collapsed)
}

fn prestige_of(state: &CampaignState, author: &FactionId, event: &DiplomacyEvent) -> i32 {
    event.prestige
        + if standing_order(state, author) {
            event.chivalry_prestige
        } else {
            0
        }
}

/// Cost of the event `event_id` for `author`, without applying it (`None`
/// when the data does not define it).
pub fn preview(
    state: &CampaignState,
    data: &GameData,
    event_id: &str,
    author: &FactionId,
) -> Option<EventPreview> {
    let event = data.diplomacy_rules.event(event_id)?;
    Some(EventPreview {
        opinion: event.opinion,
        duration: event.duration,
        prestige: prestige_of(state, author, event),
    })
}

/// Applies the event `event_id` committed by `author`: every other lord
/// (but the author) holds the opinion modifier for the event's duration and
/// the author's ruler loses the prestige. No-op when the data lacks it.
pub fn record_event(
    state: &mut CampaignState,
    data: &GameData,
    event_id: &str,
    author: &FactionId,
) {
    let Some(event) = data.diplomacy_rules.event(event_id) else {
        return;
    };
    let prestige = prestige_of(state, author, event);
    let witnesses: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(id, f)| f.alive && *id != author && !id.is_rebels())
        .map(|(id, _)| id.clone())
        .collect();
    for witness in &witnesses {
        state.add_modifier(
            witness,
            author,
            event.opinion,
            &event.reason_fr,
            event.duration,
        );
    }
    state.change_ruler_prestige(author, prestige);
}
