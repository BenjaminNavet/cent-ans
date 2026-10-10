//! Nuanced battle outcomes, mirroring
//! `data/schemas/battle_outcome_rules.schema.json`
//! (`data/rules/battle_outcome.json`). Spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 3.

use crate::key_enum;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

key_enum! {
/// Class of a battle result for one side, tested in declaration order
/// within victories then within defeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleOutcomeClass {
    Heroic => "heroic",
    Decisive => "decisive",
    Pyrrhic => "pyrrhic",
    Victory => "victory",
    HonourableDefeat => "honourable_defeat",
    Disaster => "disaster",
    Defeat => "defeat",
}
}

impl BattleOutcomeClass {
    pub fn is_victory(self) -> bool {
        matches!(
            self,
            BattleOutcomeClass::Heroic
                | BattleOutcomeClass::Decisive
                | BattleOutcomeClass::Pyrrhic
                | BattleOutcomeClass::Victory
        )
    }
}

/// Contents of `data/rules/battle_outcome.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleOutcomeRules {
    pub thresholds: OutcomeThresholds,
    /// What each class brings (every class must be listed).
    pub classes: BTreeMap<BattleOutcomeClass, OutcomeConsequence>,
    /// TW pursuit (ADR 0321): the victors chase the routed.
    #[serde(default)]
    pub pursuit: PursuitRules,
    /// TW pursuit (ADR 0321): experience earned by the regiments.
    #[serde(default)]
    pub unit_xp: UnitXpRules,
    /// TW pursuit (ADR 0321): what trophies and plundered baggage bring.
    #[serde(default)]
    pub spoils: SpoilsRules,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Pursuit of a beaten army after a 3D field battle: shares (0-1) of the
/// soldiers still alive in a fleeing regiment. Zeros mean no pursuit.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PursuitRules {
    /// Share of a routing regiment caught with no cavalry on the victors' side.
    pub routed_base: f64,
    /// Extra share per unit of ratio (victors' able cavalry / fleeing men).
    pub per_cavalry_ratio: f64,
    /// Ceiling of the share caught.
    pub max_share: f64,
    /// Share of a regiment that left the field in order (forced retreat).
    pub withdrawn_share: f64,
    /// Factor on the share for fleeing cavalry (they outrun the pursuers).
    pub fleeing_cavalry_factor: f64,
    /// Share of the caught that are taken alive (0 under "no quarter").
    pub captive_share: f64,
}

/// Experience of the regiments, in thousandths of a level
/// (`Unit::experience_residue`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitXpRules {
    /// For a regiment still fighting (or withdrawn in order) at the end.
    pub survival_milli: u32,
    /// Per 10 enemy soldiers killed by the regiment.
    pub per_ten_kills_milli: u32,
    /// Extra for the winning side.
    pub victory_milli: u32,
    /// Ceiling per regiment and battle.
    pub max_milli: u32,
}

/// Trophies and plunder (campaign side). Gold comes from the loser's
/// treasury, never beyond what it holds.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpoilsRules {
    /// Gold per standard taken.
    pub standard_gold: i64,
    /// Ruler prestige per standard taken.
    pub standard_prestige: i32,
    /// Extra prestige for the enemy general's banner.
    pub general_standard_prestige: i32,
    /// Gold when the enemy camp and baggage were looted.
    pub baggage_gold: i64,
    /// Prestige lost by the ruler of a side that lost a standard.
    pub standard_lost_prestige: i32,
}

/// Thresholds of the classification (shares 0-1 of the strength engaged).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeThresholds {
    /// Heroic: own strength / enemy strength at most this (1 : 1.5).
    pub heroic_max_strength_ratio: f64,
    /// Decisive: enemy losses at least this...
    pub decisive_enemy_losses_min: f64,
    /// ... and own losses below this.
    pub decisive_own_losses_max: f64,
    /// Pyrrhic: own losses at least this.
    pub pyrrhic_own_losses_min: f64,
    /// Honourable defeat: losses inflicted / losses suffered at least this.
    pub honourable_inflicted_ratio_min: f64,
    /// Disaster: own losses at least this (or the general killed or taken).
    pub disaster_own_losses_min: f64,
}

/// Consequences of a class for the side that earned it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeConsequence {
    /// French label ("Victoire héroïque").
    pub label: String,
    /// Multiplier of the general's battle experience.
    pub xp_multiplier: f64,
    /// Prestige of the faction's ruler.
    pub prestige: i32,
    /// Morale modifier of the surviving armies (points of 0-100 morale).
    pub morale: i8,
    /// Turns the morale modifier lasts (0: none).
    pub morale_turns: u8,
}

impl BattleOutcomeRules {
    /// Consequences of `class` (neutral ones if the file omits it).
    pub fn consequence(&self, class: BattleOutcomeClass) -> OutcomeConsequence {
        self.classes
            .get(&class)
            .cloned()
            .unwrap_or_else(|| OutcomeConsequence {
                label: class.key().to_owned(),
                xp_multiplier: 1.0,
                prestige: 0,
                morale: 0,
                morale_turns: 0,
            })
    }
}

crate::bundled_rules!(BattleOutcomeRules, "rules/battle_outcome.json", default);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_round_trip_through_json() {
        let rules = BattleOutcomeRules::default();
        let json = serde_json::to_string(&rules).unwrap();
        let back: BattleOutcomeRules = serde_json::from_str(&json).unwrap();
        assert_eq!(back, rules);
        for class in BattleOutcomeClass::ALL {
            assert!(rules.classes.contains_key(&class), "{class:?}");
        }
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<BattleOutcomeClass>();
    }
}
