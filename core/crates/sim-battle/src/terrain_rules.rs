//! Profile of each province terrain on the battlefield (relief, woods,
//! villages, scatter of hills, woods and mud). Rules in
//! `data/rules/battle_terrain.json` (schema
//! `data/schemas/battle_terrain_rules.schema.json`).

use std::collections::BTreeMap;

use data_model::Terrain;
use serde::{Deserialize, Serialize};

use crate::relief::ReliefStyle;

/// Contents of `data/rules/battle_terrain.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    terrains: BTreeMap<Terrain, TerrainProfile>,
}

/// What one terrain draws on the battlefield.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainProfile {
    pub relief: ReliefStyle,
    /// Tree density of the woods, 0-1 (rendering).
    pub woodland: f64,
    /// Chance of a village or a farm.
    pub village_chance: f64,
    /// Share of the houses that are timber-framed (the rest are cottages).
    pub timbered_share: f64,
    /// Scatter on a standard field.
    pub scatter: Scatter,
}

/// Counts of landforms on a standard field (scaled by area on a larger one).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scatter {
    pub hills: usize,
    pub hill_height_m: f64,
    pub forests: usize,
    pub mud: usize,
}

data_model::bundled_rules!(TerrainRules, "rules/battle_terrain.json");

impl TerrainRules {
    /// The profile of `terrain`.
    pub fn of(terrain: Terrain) -> &'static TerrainProfile {
        Self::bundled()
            .terrains
            .get(&terrain)
            .unwrap_or_else(|| panic!("battle_terrain.json has no {terrain:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_terrain_has_a_profile() {
        for terrain in [
            Terrain::Plains,
            Terrain::Hills,
            Terrain::Mountains,
            Terrain::Forest,
            Terrain::Marsh,
            Terrain::Heath,
            Terrain::Bocage,
            Terrain::Steppe,
            Terrain::Desert,
        ] {
            let profile = TerrainRules::of(terrain);
            assert!(profile.woodland > 0.0 && profile.village_chance > 0.0);
        }
        assert_eq!(TerrainRules::of(Terrain::Hills).scatter.hills, 7);
    }
}
