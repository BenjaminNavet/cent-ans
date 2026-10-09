//! Campaign army rules (ADR 0128), mirroring
//! `data/schemas/army_rules.schema.json` (`data/rules/armies.json`), and the
//! siege engines built on the spot (N7), mirroring
//! `data/schemas/siege_engine_rules.schema.json`
//! (`data/rules/siege_engines.json`).

use serde::{Deserialize, Serialize};

use crate::ids::UnitTypeId;

/// Contents of `data/rules/armies.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmyRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Regiments an army may hold at most (formation, merge, mercenaries,
    /// encounter joins).
    pub max_units: u32,
    /// Landing on a hostile shore.
    pub landing: LandingRules,
    /// Pace change (percent) of an army dragging siege engines.
    pub siege_train_pace_percent: f64,
    /// WH armya: upkeep of the armies beyond the free ones.
    #[serde(default)]
    pub upkeep: ArmyUpkeepRules,
    /// WH armya: an army without a general.
    #[serde(default)]
    pub leaderless: LeaderlessRules,
    /// WH armya: pace (percent, on the season's points) of an army by its
    /// slowest unit family; a cavalry-only host outruns a mixed one.
    #[serde(default)]
    pub pace_percent_by_category: CategoryPace,
}

/// Rising upkeep of a faction's armies (ADR 0272).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmyUpkeepRules {
    /// Armies a faction keeps without surcharge (the biggest ones).
    pub free_armies: u32,
    /// The k-th army beyond the free ones pays its upkeep plus
    /// `k * extra_army_upkeep_percent` percent.
    pub extra_army_upkeep_percent: f64,
}

impl Default for ArmyUpkeepRules {
    /// No surcharge (old data).
    fn default() -> Self {
        Self {
            free_armies: u32::MAX,
            extra_army_upkeep_percent: 0.0,
        }
    }
}

/// Handicaps of an army with no general (ADR 0272).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaderlessRules {
    /// Movement points change, percent (negative).
    #[serde(default)]
    pub movement_percent: f64,
    /// Cannot start a siege of a defended place (nor take the siege stance).
    #[serde(default)]
    pub no_siege: bool,
    /// Cannot lay an ambush.
    #[serde(default)]
    pub no_ambush: bool,
}

/// Pace change by unit family, percent.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryPace {
    #[serde(default)]
    pub infantry: f64,
    #[serde(default)]
    pub ranged: f64,
    #[serde(default)]
    pub cavalry: f64,
    #[serde(default)]
    pub siege: f64,
}

impl CategoryPace {
    /// Pace change of one family.
    pub fn of(&self, category: crate::UnitCategory) -> f64 {
        match category {
            crate::UnitCategory::Infantry => self.infantry,
            crate::UnitCategory::Ranged => self.ranged,
            crate::UnitCategory::Cavalry => self.cavalry,
            crate::UnitCategory::Siege => self.siege,
        }
    }
}

/// Cost of landing in hostile territory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandingRules {
    /// Strength lost by each unit, percent.
    pub loss_percent: u32,
    /// Multiplier of the loss in winter.
    pub winter_factor: u32,
    /// Morale lost by each unit.
    pub morale_loss: u8,
}

crate::bundled_rules!(ArmyRules, "rules/armies.json", default);

impl ArmyRules {
    /// [`ArmyRules::max_units`] as a length.
    pub fn cap(&self) -> usize {
        self.max_units as usize
    }
}

/// Role of a built engine in the siege battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltEngineKind {
    Ladders,
    Ram,
    Tower,
}

/// One engine of `data/rules/siege_engines.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiegeEngineRule {
    pub id: String,
    pub kind: BuiltEngineKind,
    /// French display name.
    pub name: String,
    /// Work points to finish it (0: ready when the siege begins).
    pub work: u32,
    /// A6-L2: extra work points per wall level of the besieged place, from
    /// `SiegeEngineRules::scaling_min_wall_level` up (0: none).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub work_per_wall_level: u32,
    /// Unit type of the siege tower in battle (`kind: tower`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tower_unit_type: Option<UnitTypeId>,
    /// NT9: once ready, the assailant's damage in an auto-resolved assault
    /// behind standing walls is raised by this percentage (the ram breaks
    /// the gate).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub auto_assault_bonus_percent: u32,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

/// Contents of `data/rules/siege_engines.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiegeEngineRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Men of the besieging armies for one work point per turn.
    pub men_per_work_point: u32,
    /// Work points per turn at least.
    pub min_work_per_turn: u32,
    /// A6-L2: engines cost `work_per_wall_level` more per wall level only
    /// against walls of at least this level (a palisade needs no build-up).
    #[serde(default = "default_scaling_min_wall_level")]
    pub scaling_min_wall_level: u32,
    /// Engines, in building order.
    pub engines: Vec<SiegeEngineRule>,
}

fn default_scaling_min_wall_level() -> u32 {
    3
}

impl SiegeEngineRule {
    /// Work points to finish this engine against walls of `walls` level.
    pub fn cost(&self, scaling_min_wall_level: u32, walls: u32) -> u32 {
        if walls >= scaling_min_wall_level {
            self.work
                .saturating_add(self.work_per_wall_level.saturating_mul(walls))
        } else {
            self.work
        }
    }
}

crate::bundled_rules!(SiegeEngineRules, "rules/siege_engines.json", default);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_parse() {
        assert_eq!(ArmyRules::default().max_units, 40);
        let rules = ArmyRules::default();
        assert!(rules.upkeep.free_armies < 100);
        assert!(rules.leaderless.movement_percent < 0.0);
        assert!(rules.pace_percent_by_category.cavalry > 0.0);
        let engines = SiegeEngineRules::default();
        assert_eq!(engines.engines[0].kind, BuiltEngineKind::Ladders);
    }
}
