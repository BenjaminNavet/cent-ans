//! Seasonal replenishment of field armies and recruitment pools of the
//! settlements (lot TW2-T2, ADR 0102), mirroring
//! `data/schemas/replenishment_rules.schema.json` (`data/rules/replenishment.json`).
//! Spec `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T2.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::common::UnitCategory;
use crate::entities::settlement::SettlementKind;

/// Contents of `data/rules/replenishment.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplenishmentRules {
    pub replenishment: ArmyReplenishmentRules,
    pub recruit_pool: RecruitPoolRules,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

crate::bundled_rules!(ReplenishmentRules, "rules/replenishment.json", default);

/// Seasonal replenishment of a field army: percent of its missing men.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmyReplenishmentRules {
    /// Base rate (percent of the missing men) by the territory the army
    /// stands on.
    pub territory_percent: TerritoryPercent,
    /// Multiplier (percent) of the army's stance.
    pub stance_percent: StancePercent,
    /// Multiplier (percent) when the army stands in a friendly settlement
    /// (replaces the stance multiplier when higher).
    pub in_settlement_percent: u32,
    /// Multiplier (percent) in winter.
    pub winter_percent: u32,
    /// Below this supply (0-100) the army replenishes less.
    pub low_supply_threshold: u8,
    /// Multiplier (percent) below `low_supply_threshold`.
    pub low_supply_percent: u32,
    /// Bonus (percent of the rate) per governance level of the general.
    pub general_governance_percent_per_level: i32,
    /// Bonus (percent of the rate) of the general's learned skills.
    #[serde(default)]
    pub general_skill_percent: BTreeMap<String, i32>,
    /// Bonus or malus (percent of the rate) of the general's traits.
    #[serde(default)]
    pub general_trait_percent: BTreeMap<String, i32>,
    /// Bonus (percent of the rate) per `garrison` point of the buildings of
    /// the army's own province.
    pub building_garrison_point_percent: i32,
    /// Bonus (percent of the rate) per `recruit_slots` point of the
    /// buildings of the army's own province (muster field, stables...).
    pub building_recruit_slot_percent: i32,
    /// Ceiling of the building bonus (percent of the rate).
    pub building_bonus_max_percent: i32,
    /// Ceiling of the final rate (percent of the missing men).
    pub max_percent: u32,
    /// Price of one man regained, percent of the per-man recruitment money
    /// (`cost.money / soldiers` of the unit type).
    pub cost_percent: u32,
}

/// Base replenishment rate by territory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerritoryPercent {
    pub own: u32,
    pub ally: u32,
    pub neutral: u32,
    pub hostile: u32,
}

/// Multiplier (percent) of each army stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StancePercent {
    pub normal: u32,
    pub raid: u32,
    pub siege: u32,
    pub ambush: u32,
    pub forced_march: u32,
    pub entrenched: u32,
}

/// Recruitment pools (Medieval II): per settlement and per unit type, a
/// capped reserve filled every season and drawn by each recruit. Amounts in
/// thousandths of a unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecruitPoolRules {
    /// Units a settlement of each kind holds at most.
    pub base_cap: KindValues,
    /// Thousandths of a unit gained each season by kind.
    pub base_rate_milli: KindValues,
    /// Extra units held by the city of the faction capital.
    pub capital_cap_bonus: u32,
    /// Extra thousandths gained each season in the capital city.
    pub capital_rate_milli: u32,
    /// Extra thousandths per fortification level of the settlement.
    pub fortification_rate_milli_per_level: u32,
    /// Extra units held per `recruit_slots` point of the settlement's
    /// buildings (muster field, stables, armoury).
    pub recruit_slot_cap_bonus: u32,
    /// Extra thousandths per `recruit_slots` point.
    pub recruit_slot_rate_milli: u32,
    /// Multiplier (percent) of the rate by unit category (knights and siege
    /// engines come back slower).
    pub category_rate_percent: CategoryPercent,
}

/// One value per settlement kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindValues {
    pub city: u32,
    pub town: u32,
    pub castle: u32,
    pub abbey: u32,
    pub village: u32,
}

impl KindValues {
    pub fn get(&self, kind: SettlementKind) -> u32 {
        match kind {
            SettlementKind::City => self.city,
            SettlementKind::Town => self.town,
            SettlementKind::Castle => self.castle,
            SettlementKind::Abbey => self.abbey,
            SettlementKind::Village => self.village,
        }
    }
}

/// One percent per unit category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryPercent {
    pub infantry: u32,
    pub ranged: u32,
    pub cavalry: u32,
    pub siege: u32,
}

impl CategoryPercent {
    pub fn get(&self, category: UnitCategory) -> u32 {
        match category {
            UnitCategory::Infantry => self.infantry,
            UnitCategory::Ranged => self.ranged,
            UnitCategory::Cavalry => self.cavalry,
            UnitCategory::Siege => self.siege,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_parse() {
        let rules = ReplenishmentRules::default();
        assert_eq!(rules.replenishment.territory_percent.hostile, 0);
        assert!(rules.replenishment.territory_percent.own > 0);
        assert!(rules.recruit_pool.base_cap.city >= 1);
    }
}
