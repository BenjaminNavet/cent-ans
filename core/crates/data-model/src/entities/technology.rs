//! Technology: a node of the military or civil tree (`technology.schema.json`).

use crate::key_enum;
use serde::{Deserialize, Serialize};

use crate::common::{Effect, HistoricalDate, LocalizedName, Sources};
use crate::ids::{BuildingId, TechnologyId, UnitTypeId};

key_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TechBranch {
    Military => "military",
    Civil => "civil",
    /// H4: medicine and herbalism.
    Medicine => "medicine",
}
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TechUnlocks {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub units: Vec<UnitTypeId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buildings: Vec<BuildingId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Technology {
    pub id: TechnologyId,
    pub name: LocalizedName,
    pub branch: TechBranch,
    pub tier: u8,
    /// Research points of the branch.
    pub cost: u32,
    pub prerequisites: Vec<TechnologyId>,
    /// Indicative historical date, used to slow down early research.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_year: Option<HistoricalDate>,
    #[serde(default)]
    pub unlocks: TechUnlocks,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
    /// H4: codex entries (`cdx_…`, category `plante`) of the plants the
    /// technology brings into the Herbarium.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub herbs: Vec<String>,
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<TechBranch>();
    }
}
