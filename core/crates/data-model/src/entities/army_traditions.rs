//! Army traditions (lot TW2-T5, ADR 0112), mirroring
//! `data/schemas/army_traditions_rules.schema.json`
//! (`data/rules/army_traditions.json`). Spec
//! `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T5.

use serde::{Deserialize, Serialize};

/// The rules file as bundled at build time: the fallback when
/// `data/rules/army_traditions.json` is absent (test fixtures).
const BUNDLED: &str = include_str!("../../../../../data/rules/army_traditions.json");

/// Contents of `data/rules/army_traditions.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmyTraditionRules {
    pub experience: ArmyExperienceRules,
    pub branches: Vec<TraditionBranchDef>,
    pub traditions: Vec<ArmyTradition>,
    pub ai: TraditionAiWeights,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for ArmyTraditionRules {
    fn default() -> Self {
        serde_json::from_str(BUNDLED).expect("bundled data/rules/army_traditions.json is valid")
    }
}

impl ArmyTraditionRules {
    /// The tradition `id`, if any.
    pub fn tradition(&self, id: &str) -> Option<&ArmyTradition> {
        self.traditions.iter().find(|t| t.id == id)
    }

    /// The branch `branch`, if described.
    pub fn branch(&self, branch: TraditionBranch) -> Option<&TraditionBranchDef> {
        self.branches.iter().find(|b| b.id == branch)
    }

    /// Number of ranks (one tradition choice each).
    pub fn max_rank(&self) -> u8 {
        self.experience.rank_thresholds.len() as u8
    }
}

/// Army experience per battle and rank thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmyExperienceRules {
    /// Experience of any battle fought (times the outcome's multiplier).
    pub battle_fought: u32,
    /// Experience added on a victory.
    pub battle_won: u32,
    /// No experience against an enemy under this percent of the army.
    pub min_enemy_percent: u32,
    /// Cumulated experience of each rank, increasing.
    pub rank_thresholds: Vec<u32>,
}

/// The five branches of the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraditionBranch {
    March,
    Stewardship,
    Shooting,
    Assault,
    Discipline,
}

impl TraditionBranch {
    pub const ALL: [TraditionBranch; 5] = [
        TraditionBranch::March,
        TraditionBranch::Stewardship,
        TraditionBranch::Shooting,
        TraditionBranch::Assault,
        TraditionBranch::Discipline,
    ];

    pub fn key(self) -> &'static str {
        match self {
            TraditionBranch::March => "march",
            TraditionBranch::Stewardship => "stewardship",
            TraditionBranch::Shooting => "shooting",
            TraditionBranch::Assault => "assault",
            TraditionBranch::Discipline => "discipline",
        }
    }
}

/// A branch as the UI shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraditionBranchDef {
    pub id: TraditionBranch,
    /// French name.
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// One tradition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmyTradition {
    pub id: String,
    pub branch: TraditionBranch,
    /// Tier in its branch: tier N needs tier N-1 of the same branch.
    pub tier: u8,
    /// French name.
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub effects: TraditionEffects,
}

/// Effects of a tradition (summed over the traditions of an army).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraditionEffects {
    /// Movement points of the season, percent.
    #[serde(default)]
    pub movement_percent: i32,
    /// Replenishment rate bonus (TW2-T2), percent of the rate.
    #[serde(default)]
    pub replenish_percent: i32,
    /// Ranged points of the shooting units.
    #[serde(default)]
    pub ranged: i32,
    /// Siege duration cut, percent (as the general's `SiegeSpeed`).
    #[serde(default)]
    pub siege_speed_percent: i32,
    /// Morale points (0-100) of every unit in battle.
    #[serde(default)]
    pub morale: i32,
}

impl TraditionEffects {
    pub fn add(&mut self, other: &TraditionEffects) {
        self.movement_percent += other.movement_percent;
        self.replenish_percent += other.replenish_percent;
        self.ranged += other.ranged;
        self.siege_speed_percent += other.siege_speed_percent;
        self.morale += other.morale;
    }

    /// French summary: « +8 % de mouvement, +3 de moral ».
    pub fn text_fr(&self) -> String {
        let mut parts = Vec::new();
        if self.movement_percent != 0 {
            parts.push(format!("{:+} % de mouvement", self.movement_percent));
        }
        if self.replenish_percent != 0 {
            parts.push(format!("{:+} % de reconstitution", self.replenish_percent));
        }
        if self.ranged != 0 {
            parts.push(format!("{:+} de tir", self.ranged));
        }
        if self.siege_speed_percent != 0 {
            parts.push(format!("sièges {} % plus courts", self.siege_speed_percent));
        }
        if self.morale != 0 {
            parts.push(format!("{:+} de moral", self.morale));
        }
        parts.join(", ")
    }
}

/// Weights of the AI's choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraditionAiWeights {
    pub base_weight: BranchWeights,
    /// Times the share of shooters in the faction's recruitment doctrine
    /// (shooting branch).
    pub doctrine_ranged_weight: u32,
    /// Times the share of missing men of the army (stewardship branch).
    pub missing_men_weight: u32,
    /// Added to the assault branch while the army besieges a place.
    pub besieging_weight: u32,
    /// Added to a branch the army has already started.
    pub started_branch_weight: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One weight per branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BranchWeights {
    pub march: u32,
    pub stewardship: u32,
    pub shooting: u32,
    pub assault: u32,
    pub discipline: u32,
}

impl BranchWeights {
    pub fn get(&self, branch: TraditionBranch) -> u32 {
        match branch {
            TraditionBranch::March => self.march,
            TraditionBranch::Stewardship => self.stewardship,
            TraditionBranch::Shooting => self.shooting,
            TraditionBranch::Assault => self.assault,
            TraditionBranch::Discipline => self.discipline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_parse() {
        let rules = ArmyTraditionRules::default();
        assert!((3..=4).contains(&rules.max_rank()));
        for branch in TraditionBranch::ALL {
            assert!(rules.branch(branch).is_some(), "{branch:?}");
            assert!(rules
                .traditions
                .iter()
                .any(|t| t.branch == branch && t.tier == 1));
        }
    }
}
