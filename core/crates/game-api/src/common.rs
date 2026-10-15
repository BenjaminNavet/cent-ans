//! Small shared view types.

use data_model::GameData;
use serde::Serialize;
use sim_campaign::Unit;

/// One unit of a garrison or an army roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnitView {
    pub unit_type: String,
    /// French display name.
    pub name: String,
    pub strength: u32,
    pub max_strength: u32,
    pub morale: u8,
    pub experience: u8,
}

pub(crate) fn unit_views(data: &GameData, units: &[Unit]) -> Vec<UnitView> {
    units
        .iter()
        .map(|unit| UnitView {
            unit_type: unit.unit_type.as_str().to_owned(),
            name: data.unit_types.get(&unit.unit_type).map_or_else(
                || unit.unit_type.as_str().to_owned(),
                |t| t.name.display.clone(),
            ),
            strength: unit.strength,
            max_strength: unit.max_strength,
            morale: unit.morale,
            experience: unit.experience,
        })
        .collect()
}

/// A signed line `{key, value}` of an income or unrest breakdown; the UI maps `key` to French.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyedAmount {
    pub key: String,
    pub value: i64,
}

/// `{id, name}` reference to a faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FactionRef {
    pub id: String,
    pub name: String,
}

pub(crate) fn faction_refs(data: &GameData, ids: &[data_model::FactionId]) -> Vec<FactionRef> {
    ids.iter()
        .map(|id| FactionRef {
            id: id.as_str().to_owned(),
            name: data.faction_name(id),
        })
        .collect()
}
