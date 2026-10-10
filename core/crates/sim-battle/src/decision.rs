//! How a field battle ends (lot EP9, ADR 0056): the broken army and the
//! refused battle (`data/rules/battle_decision.json`, schema
//! `data/schemas/battle_decision_rules.schema.json`).
//!
//! - **Broken army**: a side whose fighting strength (soldiers of the
//!   regiments present and in good order, plus the reserves still to come)
//!   falls below [`DecisionRules::break_share`] of its initial soldiers
//!   breaks: its remaining regiments rout and the battle ends. Without its
//!   general (killed or taken) the army lets go sooner
//!   ([`DecisionRules::break_share_without_general`]).
//! - **Refused battle**: when nobody engages for
//!   [`DecisionRules::refusal_seconds`] (no melee, losses under
//!   [`DecisionRules::engagement_loss_share`], and the armies no longer
//!   draw closer by [`DecisionRules::approach_meters`]), the attacker
//!   gives up and withdraws, the defender keeps the field (Buironfosse,
//!   1339). After a melee, a shorter lull ([`DecisionRules::lull_seconds`])
//!   is enough: the side that suffered less keeps the field (the defender unless the attacker lost clearly
//!   less, [`DecisionRules::lull_loss_margin`]).
//!
//! Siege battles keep their own end (the square held, the garrison beaten).

use data_model::key_enum;
use serde::{Deserialize, Serialize};

/// Campaign morale after a refused battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefusedMorale {
    pub attacker: i32,
    pub defender: i32,
}

/// Campaign morale after an ordered general retreat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WithdrawalMorale {
    pub withdrawer: i32,
    pub other: i32,
}

/// Contents of `data/rules/battle_decision.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    /// Share of its initial soldiers below which an army breaks.
    pub break_share: f64,
    /// The same once its general is killed or taken.
    pub break_share_without_general: f64,
    /// The army breaks once it has stayed below its share this long
    /// (regiments that rally at once do not break it).
    pub(crate) break_hold_seconds: f64,
    /// Seconds without engagement before the battle ends.
    pub refusal_seconds: f64,
    /// The same once a melee has taken place (the fight died down).
    pub(crate) lull_seconds: f64,
    /// Window over which an approach is measured.
    pub(crate) approach_window_seconds: f64,
    /// Drawing this much closer to the enemy restarts the clock.
    pub(crate) approach_meters: f64,
    /// Missiles restart the clock once either side has lost this share of
    /// its initial soldiers since the last engagement (a few stray arrows
    /// across a river do not keep a battle going).
    pub(crate) engagement_loss_share: f64,
    /// After a lull, the attacker keeps the field only when its share of
    /// losses is lower than the defender's by this margin.
    pub(crate) lull_loss_margin: f64,
    pub refused_morale: RefusedMorale,
    /// TW retreat: campaign morale after an ordered general retreat.
    pub withdrawal_morale: WithdrawalMorale,
}

data_model::bundled_rules!(DecisionRules, "rules/battle_decision.json");

key_enum! {
/// How a battle ended (result screen, campaign chronicle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleEnd {
    /// Every regiment of the loser fled, fell or left the field.
    #[default]
    Rout => "rout",
    /// The loser's army broke (below its break share).
    Broken => "broken",
    /// TW retreat: an ordered general retreat of an army still in good
    /// order (no regiment routed): a defeat without a rout.
    Withdrawal => "withdrawal",
    /// Nobody engaged: the attacker gave up, the defender keeps the field.
    Refused => "refused",
    /// The fight died down: the side that suffered less keeps the field.
    Lull => "lull",
    /// Nightfall (the hour limit).
    Nightfall => "nightfall",
    /// Siege: the attacker holds the central square.
    SquareHeld => "square_held",
}
}

/// Engagement clock of a field battle (derived from the state each step).
#[derive(Debug, Clone, Default)]
pub(crate) struct EngagementClock {
    /// Time of the last loss, melee or approach.
    pub(crate) last: f64,
    /// Gap between the armies at the start of the approach window, and
    /// when that window began.
    pub(crate) gap_mark: Option<(f64, f64)>,
    /// A loss or a melee happened at some point.
    pub(crate) fought: bool,
    /// A melee happened at some point.
    pub(crate) melee_seen: bool,
    /// Loss share of each side when the clock last restarted.
    pub(crate) losses_mark: [f64; 2],
    /// Since when each side has been below its break share.
    pub(crate) below_since: [Option<f64>; 2],
    /// EP9b: soldiers of each side struck down by missiles so far.
    pub(crate) missile_losses: [f64; 2],
    /// EP9b: share of its initial soldiers each side lost to missiles,
    /// sampled every AI period (oldest first), for the attacker's archery
    /// duel (`duel.rs`).
    pub(crate) missile_log: std::collections::VecDeque<(f64, [f64; 2])>,
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<BattleEnd>();
    }
}
