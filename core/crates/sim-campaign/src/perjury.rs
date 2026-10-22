//! M6 (ADR 0721): perjury and other windowed diplomatic events, read from
//! the `events` table of `data/rules/diplomacy.json`.

use crate::state::CampaignState;
use data_model::{FactionId, GameData};

/// Cost an event would inflict on its author, shown before acting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventPreview {
    /// Opinion change every lord holds about the author (negative).
    pub opinion: i32,
    /// Seasons it lasts.
    pub duration: u32,
    /// Prestige the author's ruler loses (negative).
    pub prestige: i32,
}

/// Cost of the event `event_id` for `author`, without applying it.
pub fn preview(_data: &GameData, _event_id: &str) -> Option<EventPreview> {
    None
}

/// Applies the event `event_id` committed by `author` against `victim`.
pub fn record_event(
    _state: &mut CampaignState,
    _data: &GameData,
    _event_id: &str,
    _author: &FactionId,
    _victim: &FactionId,
) {
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "M6 skeleton"]
    fn tx_m6_truce_break_inside_window_is_major() {}

    #[test]
    #[ignore = "M6 skeleton"]
    fn tx_m6_opinion_decays_and_expires() {}
}
