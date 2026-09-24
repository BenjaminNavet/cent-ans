//! Building: a province improvement (`building.schema.json`).

use serde::{Deserialize, Serialize};

use crate::common::{Cost, Effect, LocalizedName, Sources};
use crate::entities::settlement::SettlementKind;
use crate::ids::{BuildingId, ResourceId, TechnologyId, UnitTypeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingCategory {
    Production,
    Commerce,
    Military,
    Religious,
    Sanitary,
    Fortification,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Building {
    pub id: BuildingId,
    pub name: LocalizedName,
    pub category: BuildingCategory,
    pub tier: u8,
    /// Previous step of the upgrade chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgrades_from: Option<BuildingId>,
    pub cost: Cost,
    pub build_time_turns: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upkeep: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_technology: Option<TechnologyId>,
    /// Other building that must be present (outside the upgrade chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_building: Option<BuildingId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_resource: Option<ResourceId>,
    #[serde(default)]
    pub requires_coastal: bool,
    #[serde(default)]
    pub requires_river: bool,
    #[serde(default)]
    pub unique_per_faction: bool,
    /// Settlement kinds where it can be built (lot C4); `None`: every kind
    /// but `village`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement_kinds: Option<Vec<SettlementKind>>,
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enables_units: Vec<UnitTypeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

impl Building {
    /// `true` when the building may stand in a settlement of `kind`
    /// (`settlement_kinds`, default: anything but a village).
    pub fn allowed_in(&self, kind: SettlementKind) -> bool {
        match &self.settlement_kinds {
            Some(kinds) => kinds.contains(&kind),
            None => kind != SettlementKind::Village,
        }
    }
}
