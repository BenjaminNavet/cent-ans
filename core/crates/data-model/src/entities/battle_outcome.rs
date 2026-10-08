//! Nuanced battle outcomes (lot CV3-1), mirroring
//! `data/schemas/battle_outcome_rules.schema.json`
//! (`data/rules/battle_outcome.json`). Spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 3.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Class of a battle result for one side, tested in declaration order
/// within victories then within defeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleOutcomeClass {
    Heroic,
    Decisive,
    Pyrrhic,
    Victory,
    HonourableDefeat,
    Disaster,
    Defeat,
}

impl BattleOutcomeClass {
    pub const ALL: [BattleOutcomeClass; 7] = [
        BattleOutcomeClass::Heroic,
        BattleOutcomeClass::Decisive,
        BattleOutcomeClass::Pyrrhic,
        BattleOutcomeClass::Victory,
        BattleOutcomeClass::HonourableDefeat,
        BattleOutcomeClass::Disaster,
        BattleOutcomeClass::Defeat,
    ];

    /// The `snake_case` key used in JSON files and by the bridge.
    pub fn key(self) -> &'static str {
        match self {
            BattleOutcomeClass::Heroic => "heroic",
            BattleOutcomeClass::Decisive => "decisive",
            BattleOutcomeClass::Pyrrhic => "pyrrhic",
            BattleOutcomeClass::Victory => "victory",
            BattleOutcomeClass::HonourableDefeat => "honourable_defeat",
            BattleOutcomeClass::Disaster => "disaster",
            BattleOutcomeClass::Defeat => "defeat",
        }
    }

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
