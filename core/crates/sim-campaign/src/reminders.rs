//! Memory of the Audience's reminders (XP-SOBRE, lot D).
//!
//! A reminder (a free building slot, an idle army, the treasurer's advice...) that stays true
//! season after season is shown `learn_after` times, then goes quiet for `snooze_seasons`
//! seasons. The player may recall it by hand. The rules of which kinds learn live in
//! `data/rules/agenda.json`; this module only keeps the counters, so that they are saved with
//! the campaign. Nothing here changes the simulation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// What the Audience remembers of one reminder kind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderMemory {
    /// Seasons the reminder was shown since it last went quiet or was recalled.
    pub shown: u32,
    /// The reminder stays quiet while `turn < snoozed_until`.
    pub snoozed_until: u32,
}

/// Reminder counters by kind (`free_slot`, `advice`, `idle_army`...).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderLedger {
    pub kinds: BTreeMap<String, ReminderMemory>,
}

impl ReminderLedger {
    /// Nothing remembered (not serialised).
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    /// Whether `kind` is quiet at `turn`.
    pub fn is_snoozed(&self, kind: &str, turn: u32) -> bool {
        self.kinds.get(kind).is_some_and(|m| turn < m.snoozed_until)
    }

    /// The season ended with `kind` `present` (the matter is still there, whether shown or
    /// quiet). Shown `learn_after` times: quiet for `snooze_seasons` seasons.
    pub fn season_ended(
        &mut self,
        kind: &str,
        present: bool,
        turn: u32,
        learn_after: u32,
        snooze_seasons: u32,
    ) {
        // A matter that comes and goes (a new advice each season) keeps its count: only the
        // quiet period or a manual recall resets it.
        if !present {
            return;
        }
        let memory = self.kinds.entry(kind.to_owned()).or_default();
        if turn < memory.snoozed_until {
            return;
        }
        memory.shown += 1;
        if memory.shown >= learn_after {
            memory.shown = 0;
            // Quiet from the next season, for `snooze_seasons` seasons.
            memory.snoozed_until = turn + 1 + snooze_seasons;
        }
    }

    /// « Rappeler »: the reminder speaks again from now on.
    pub fn recall(&mut self, kind: &str) {
        self.kinds.remove(kind);
    }

    /// Kinds currently quiet at `turn`.
    pub fn snoozed(&self, turn: u32) -> Vec<String> {
        self.kinds
            .iter()
            .filter(|(_, m)| turn < m.snoozed_until)
            .map(|(k, _)| k.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shown_twice_then_quiet_four_seasons_then_back() {
        let mut ledger = ReminderLedger::default();
        let mut shown_turns = Vec::new();
        for turn in 0..12 {
            if !ledger.is_snoozed("free_slot", turn) {
                shown_turns.push(turn);
            }
            ledger.season_ended("free_slot", true, turn, 2, 4);
        }
        assert_eq!(shown_turns, [0, 1, 6, 7]);
    }

    #[test]
    fn an_intermittent_matter_keeps_its_count_and_recall_clears() {
        let mut ledger = ReminderLedger::default();
        ledger.season_ended("advice", true, 0, 2, 4);
        ledger.season_ended("advice", false, 1, 2, 4);
        ledger.season_ended("advice", true, 2, 2, 4);
        assert!(ledger.is_snoozed("advice", 4));
        ledger.recall("advice");
        assert!(!ledger.is_snoozed("advice", 4));
    }
}
