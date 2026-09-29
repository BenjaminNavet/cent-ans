//! Campaign army rules (lot NT5, N6, ADR 0128), mirroring
//! `data/schemas/army_rules.schema.json` (`data/rules/armies.json`), and the
//! siege engines built on the spot (N7), mirroring
//! `data/schemas/siege_engine_rules.schema.json`
//! (`data/rules/siege_engines.json`).

use serde::{Deserialize, Serialize};

use crate::ids::UnitTypeId;

/// The army rules as bundled at build time: the fallback when
/// `data/rules/armies.json` is absent (test fixtures).
const BUNDLED_ARMIES: &str = include_str!("../../../../../data/rules/armies.json");
/// The engine rules as bundled at build time.
const BUNDLED_ENGINES: &str = include_str!("../../../../../data/rules/siege_engines.json");

/// Contents of `data/rules/armies.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmyRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Regiments an army may hold at most (formation, merge, mercenaries,
    /// encounter joins).
    pub max_units: u32,
}

impl Default for ArmyRules {
    fn default() -> Self {
        serde_json::from_str(BUNDLED_ARMIES).expect("bundled data/rules/armies.json is valid")
    }
}

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
    /// Unit type of the siege tower in battle (`kind: tower`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tower_unit_type: Option<UnitTypeId>,
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
    /// Engines, in building order.
    pub engines: Vec<SiegeEngineRule>,
}

impl Default for SiegeEngineRules {
    fn default() -> Self {
        serde_json::from_str(BUNDLED_ENGINES)
            .expect("bundled data/rules/siege_engines.json is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_parse() {
        assert_eq!(ArmyRules::default().max_units, 20);
        let engines = SiegeEngineRules::default();
        assert_eq!(engines.engines[0].kind, BuiltEngineKind::Ladders);
    }
}
