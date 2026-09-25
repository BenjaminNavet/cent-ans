//! Recruitment doctrines of the AI (lot E1), mirroring
//! `data/schemas/ai_doctrine.schema.json` (`data/ai/doctrines.json`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{FactionId, UnitTypeId};

/// Target army mix of one faction: relative weight of each unit type among
/// its field regiments. Unit types the faction cannot raise somewhere are
/// simply skipped there; the weights are renormalised over what is
/// available.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Doctrine {
    pub mix: BTreeMap<UnitTypeId, u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Contents of `data/ai/doctrines.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiDoctrines {
    /// Doctrine of every faction without its own.
    pub default: Doctrine,
    #[serde(default)]
    pub factions: BTreeMap<FactionId, Doctrine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl AiDoctrines {
    /// Doctrine of `faction` (its own, or the default one).
    pub fn of(&self, faction: &FactionId) -> &Doctrine {
        self.factions.get(faction).unwrap_or(&self.default)
    }
}
