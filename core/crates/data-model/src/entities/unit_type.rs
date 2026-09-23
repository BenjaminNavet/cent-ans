//! Unit type: a battle unit (`unit_type.schema.json`).

use serde::{Deserialize, Serialize};

use crate::common::{Cost, LocalizedName, SocialClass, Sources, UnitCategory};
use crate::ids::{BuildingId, CultureId, FactionId, TechnologyId, UnitTypeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ability {
    Stakes,
    ShieldWall,
    PikeSquare,
    Volley,
    Skirmish,
    Dismount,
    ChargeLance,
    Pavise,
    WallBreach,
    WallAssault,
    RainPenalty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitStats {
    pub melee: u8,
    pub ranged: u8,
    /// Range in metres (0 = melee only).
    #[serde(default)]
    pub range: u32,
    pub armor: u8,
    pub morale: u8,
    pub speed: u8,
    pub ammo: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge: Option<u8>,
    /// Damage against walls (siege engines).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub siege_attack: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitType {
    pub id: UnitTypeId,
    pub name: LocalizedName,
    pub category: UnitCategory,
    /// Social class the unit is recruited from.
    pub source_class: SocialClass,
    #[serde(default)]
    pub mounted: bool,
    #[serde(default)]
    pub mercenary: bool,
    /// Head count of a full-strength unit.
    pub soldiers: u32,
    pub cost: Cost,
    /// Pay per season in livres tournois.
    pub upkeep: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recruit_time_turns: Option<u32>,
    pub stats: UnitStats,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<Ability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_technology: Option<TechnologyId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_building: Option<BuildingId>,
    /// Province cultures allowed to recruit (empty = all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_culture: Vec<CultureId>,
    /// Factions allowed to recruit (empty = all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_faction: Vec<FactionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
