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

impl Default for PostureRules {
    /// Fallback when `data/rules/postures.json` is absent; kept equal to that
    /// file (checked by `tests/real_data.rs`).
    fn default() -> Self {
        PostureRules {
            ambush: AmbushRules {
                min_movement_left_percent: 25.0,
                detect_radius_army_km: 3.0,
                detect_radius_spy_km: 8.0,
                base_chance: 0.55,
                per_skill: 0.03,
                scout_malus: 0.4,
                scout_unit_types: [
                    "unit_hobelars",
                    "unit_jinetes",
                    "unit_akinci",
                    "unit_lithuanian_light_cavalry",
                    "unit_steppe_horse_archers",
                    "unit_mounted_archers",
                    "unit_mounted_sergeants",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
                chance_min: 0.10,
                chance_max: 0.90,
                terrain_bonus: [
                    (CoverClass::Forest, 0.10),
                    (CoverClass::Bocage, 0.05),
                    (CoverClass::Marsh, 0.05),
                ]
                .into_iter()
                .collect(),
                auto_attack_percent: 25.0,
            },
            forced_march: ForcedMarchRules {
                movement_bonus_percent: 50.0,
                supply_cost: 10,
                start_fatigue: 60.0,
                ambush_bonus: 0.10,
            },
            entrenched: EntrenchedRules {
                auto_defense_percent: 15.0,
                supply_saving_percent: 30.0,
            },
            cover: CoverRules {
                forest_threshold: 0.5,
                wetland_threshold: 0.4,
            },
            description: None,
        }
    }
}

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
