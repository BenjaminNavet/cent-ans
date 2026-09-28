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
    /// Strategic doctrine by rank of the primary title (`county`, `duchy`,
    /// `kingdom`), lot FE5: « survival first » for the counties.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rank_strategies: BTreeMap<String, RankStrategy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Strategic doctrine of the factions of one rank (lot FE5, spec § 5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankStrategy {
    /// Doctrine id, for the logs (`survival_first`).
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A war is declared only if our power and our allies' reach this
    /// multiple of the target's, its allies' and the suzerains likely to
    /// come to its help.
    pub min_war_power_ratio: f64,
    /// A sovereign faction facing a neighbour at war with it this many
    /// times stronger seeks a protector (homage to a friendly lord).
    pub seek_protection_power_ratio: f64,
}

impl AiDoctrines {
    /// Doctrine of `faction` (its own, or the default one).
    pub fn of(&self, faction: &FactionId) -> &Doctrine {
        self.factions.get(faction).unwrap_or(&self.default)
    }

    /// Strategic doctrine of a faction whose primary title has rank `rank`.
    pub fn strategy(&self, rank: &str) -> Option<&RankStrategy> {
        self.rank_strategies.get(rank)
    }
}
