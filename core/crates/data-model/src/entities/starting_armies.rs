//! Starting armies of the 1337 campaign (lot A6-L3b, ADR 0183), mirroring
//! `data/schemas/starting_armies.schema.json`
//! (`data/rules/starting_armies.json`): the main army of each faction and
//! the starting garrison of each province role.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{FactionId, UnitTypeId};

/// Starting garrisons by province role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartingGarrisons {
    pub capital: Vec<UnitTypeId>,
    pub frontier: Vec<UnitTypeId>,
    pub interior: Vec<UnitTypeId>,
}

/// Contents of `data/rules/starting_armies.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartingArmies {
    /// Main army of the factions without their own.
    pub default_army: Vec<UnitTypeId>,
    #[serde(default)]
    pub factions: BTreeMap<FactionId, Vec<UnitTypeId>>,
    pub garrisons: StartingGarrisons,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl StartingArmies {
    /// Main army of `faction` (its own, or the default one).
    pub fn army_of(&self, faction: &FactionId) -> &[UnitTypeId] {
        self.factions
            .get(faction)
            .map_or(self.default_army.as_slice(), Vec::as_slice)
    }
}
