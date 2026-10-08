//! EP9b (ADR 0056, § EP9b): the attacker's archery duel and the order of
//! its assault. Rules in `data/rules/battle_duel.json` (schema
//! `data/schemas/battle_duel_rules.schema.json`).
//!
//! - The side that sought the battle trades volleys for
//!   [`DuelRules::duel_limit_seconds`] when it is not winning the duel, then
//!   closes in (it must attack, the defender may wait: EP9).
//! - While its shooters are winning the duel — over the last
//!   [`DuelRules::window_seconds`] the enemy lost at least
//!   [`DuelRules::winning_min_share`] of its soldiers and at least
//!   [`DuelRules::winning_ratio`] times the share it lost itself — it keeps
//!   shooting, up to [`DuelRules::winning_duel_max_seconds`]. A won duel
//!   costs the defender men, so the refused-battle clock keeps restarting
//!   (EP9): the upper bound, not the refusal, ends it.
//! - Closing in, the foot regiments of low base morale (militia) march in a
//!   second echelon [`DuelRules::second_echelon_depth_m`] behind the solid
//!   troops instead of leading the assault through the arrows; it closes up
//!   for the melee once the enemy is [`DuelRules::second_echelon_closes_m`]
//!   away.

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/battle_duel.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuelRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Duel of an attacker losing it or trading evenly (seconds).
    pub duel_limit_seconds: f64,
    /// Upper bound of the duel of an attacker winning it (seconds).
    pub winning_duel_max_seconds: f64,
    /// Sliding window over which inflicted and suffered losses compare.
    pub window_seconds: f64,
    /// The attacker wins the duel when the enemy's loss share over the
    /// window is at least this multiple of its own.
    pub winning_ratio: f64,
    /// ... and at least this share of the enemy's initial soldiers.
    pub winning_min_share: f64,
    /// Line regiments whose base morale is below this march in the second
    /// echelon when the line closes in.
    pub second_echelon_morale: f64,
    /// Distance of the second echelon behind the first line (metres).
    pub second_echelon_depth_m: f64,
    /// The second echelon closes up with the first line once the enemy is
    /// this close (metres): it follows the assault into the melee.
    pub second_echelon_closes_m: f64,
}

data_model::bundled_rules!(DuelRules, "rules/battle_duel.json");

impl DuelRules {
    /// Whether a side that lost `own` of its soldiers over the window while
    /// the enemy lost `enemy` (shares of the initial soldiers) is winning
    /// the duel.
    pub fn winning(&self, own: f64, enemy: f64) -> bool {
        enemy >= self.winning_min_share && enemy >= self.winning_ratio * own
    }

    /// How long the attacker keeps trading volleys at `elapsed` seconds,
    /// winning the duel or not.
    pub fn attacker_limit(&self, winning: bool) -> f64 {
        if winning {
            self.winning_duel_max_seconds.max(self.duel_limit_seconds)
        } else {
            self.duel_limit_seconds
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_duel_is_won_by_a_clear_margin_only() {
        let rules = DuelRules::bundled();
        let min = rules.winning_min_share;
        assert!(rules.winning(min / rules.winning_ratio, min));
        assert!(!rules.winning(min, min), "an even duel is not won");
        assert!(!rules.winning(0.0, min * 0.5), "a few stray arrows");
        assert!(rules.attacker_limit(true) >= rules.attacker_limit(false));
    }
}
