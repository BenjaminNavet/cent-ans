//! Nuanced battle outcomes (lot CV3-1, spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 3): heroic, decisive,
//! Pyrrhic victories; honourable defeat, disaster. Thresholds and
//! consequences in `data/rules/battle_outcome.json`
//! ([`data_model::BattleOutcomeRules`]).

use data_model::{BattleOutcomeClass, BattleOutcomeRules};
use serde::{Deserialize, Serialize};

/// What the classification needs to know of one side.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct SideTally {
    /// Men engaged.
    pub strength: u32,
    /// Men lost.
    pub losses: u32,
    /// The commanding general was killed or taken.
    pub general_lost: bool,
}

/// Class of the battle for a side that `won` with `own` against `enemy`.
pub fn classify(
    _rules: &BattleOutcomeRules,
    won: bool,
    _own: &SideTally,
    _enemy: &SideTally,
) -> BattleOutcomeClass {
    if won {
        BattleOutcomeClass::Victory
    } else {
        BattleOutcomeClass::Defeat
    }
}
