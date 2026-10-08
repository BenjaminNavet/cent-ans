//! Army stances of the campaign (lot CV3-1: ambush, forced march, entrenched
//! camp), mirroring `data/schemas/posture_rules.schema.json`
//! (`data/rules/postures.json`). Spec `docs/design/2026-09-27-campagne-vivante.md` § 1.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::cover::CoverClass;

/// Contents of `data/rules/postures.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostureRules {
    pub ambush: AmbushRules,
    pub forced_march: ForcedMarchRules,
    pub entrenched: EntrenchedRules,
    /// How a cell of the navigation grid is classed as covered.
    pub cover: CoverRules,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Ambush (`Ambush`): hidden in cover, sprung on an enemy march.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmbushRules {
    /// Share (percent) of the turn's movement that must be left to set up.
    pub min_movement_left_percent: f64,
    /// An enemy army closer than this (km) sees the ambush.
    pub detect_radius_army_km: f64,
    /// An enemy spy closer than this (km, from its settlement) sees it.
    pub detect_radius_spy_km: f64,
    /// Base success chance (0-1).
    pub base_chance: f64,
    /// Added per point of ambush skill of the general (0-1 per point).
    pub per_skill: f64,
    /// Removed × the victim's share of scouts (light horse) in its strength.
    pub scout_malus: f64,
    /// Unit types counted as scouts (light cavalry).
    pub scout_unit_types: Vec<String>,
    /// Bounds of the success chance.
    pub chance_min: f64,
    pub chance_max: f64,
    /// Chance bonus of the cover class of the ambusher's cell.
    pub terrain_bonus: BTreeMap<CoverClass, f64>,
    /// Auto-resolve: `BattleCharge` percent of the ambusher after a sprung
    /// ambush.
    pub auto_attack_percent: f64,
}

/// Forced march (`ForcedMarch`): more movement, tired troops.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForcedMarchRules {
    /// Movement bonus (percent of the turn's allowance).
    pub movement_bonus_percent: f64,
    /// Supply lost at the end of the turn.
    pub supply_cost: u8,
    /// Fatigue (0-100 gauge of `sim-battle`) of every regiment at the start
    /// of a battle.
    pub start_fatigue: f64,
    /// Added to the ambush success chance against an army in forced march.
    pub ambush_bonus: f64,
}

/// Entrenched camp (`Entrenched`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntrenchedRules {
    /// Auto-resolve: `BattleDefense` percent of the entrenched side.
    pub auto_defense_percent: f64,
    /// Share (percent) of the supply loss saved each turn.
    pub supply_saving_percent: f64,
}

/// Cover sampling of the navigation grid (`data-model::cover`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverRules {
    /// Forest cover (0-1, from the forest raster of `map/forest_cover.json`)
    /// from which a cell is a forest.
    pub forest_threshold: f64,
    /// Wetland density (0-1, `map/wetlands.png`: marshes, ponds, wet
    /// meadows) from which a cell is a marsh.
    pub wetland_threshold: f64,
}

crate::bundled_rules!(PostureRules, "rules/postures.json", default);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_round_trip_through_json() {
        let rules = PostureRules::default();
        let json = serde_json::to_string(&rules).unwrap();
        let back: PostureRules = serde_json::from_str(&json).unwrap();
        assert_eq!(back, rules);
    }
}
