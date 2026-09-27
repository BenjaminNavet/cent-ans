//! CV3-2 (spec campagne vivante § 1.3, ADR 0094): how a battle opens after a
//! campaign stance. Rules in `data/rules/battle_opening.json` (schema
//! `data/schemas/battle_opening_rules.schema.json`):
//!
//! - **ambush**: the victim is caught in marching column along the longest
//!   road through the field (else the long axis of the map), with no
//!   deployment phase; the ambusher deploys on one or two flanks of the
//!   column, preferring the flank with woods and hedges;
//! - **forced march**: the side starts tired, placed automatically, with no
//!   deployment phase;
//! - **entrenched camp**: stakes planted from the start and a low palisade
//!   in front of the line ([`crate::site::ObstacleKind::Palisade`]).
//!
//! The placement itself is in `sim/opening.rs`.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Marching column of the ambushed side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColumnRules {
    /// Gap between two regiments of the column (metres).
    pub gap_m: f64,
    /// The column keeps this far from the field edges (metres).
    pub edge_margin_m: f64,
    /// A road shorter than this (inside the field) is not used: the column
    /// then follows the long axis of the map (metres).
    pub min_road_length_m: f64,
}

/// Deployment zone of the ambusher along the column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlankRules {
    /// Near edge of the zone, from the axis of the column (metres).
    pub near_m: f64,
    /// Far edge of the zone, from the axis of the column (metres).
    pub far_m: f64,
    /// The zone runs this far past each end of the column (metres).
    pub extra_length_m: f64,
    /// Step of the cover sampling of a flank (metres).
    pub sample_step_m: f64,
    /// Score of a sample in a wood.
    pub forest_weight: f64,
    /// Score of a sample near a hedge or a fence.
    pub hedge_weight: f64,
    /// A sample this close to a hedge counts as covered (metres).
    pub hedge_reach_m: f64,
    /// The second flank opens when its score reaches this share of the
    /// best one (both flanks when neither has any cover).
    pub second_flank_ratio: f64,
    /// Gap between two ambushing regiments placed by default (metres).
    pub unit_gap_m: f64,
}

/// Low palisade of an entrenched camp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PalisadeRules {
    /// Distance between the front of the camp and the palisade (metres).
    pub distance_m: f64,
    /// The palisade runs this far past each end of the line (metres).
    pub margin_m: f64,
    /// Length of one piece of palisade (metres).
    pub segment_m: f64,
    /// Speed multiplier of foot crossing it.
    pub foot_crossing_factor: f64,
    /// Speed multiplier of horse crossing it.
    pub horse_crossing_factor: f64,
    /// Divides the melee losses of a regiment behind it attacked across it.
    pub melee_defense: f64,
    /// The defender must stand this close to the palisade (metres).
    pub reach_m: f64,
}

/// Contents of `data/rules/battle_opening.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpeningRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub column: ColumnRules,
    pub flank: FlankRules,
    pub palisade: PalisadeRules,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_opening.json");

impl OpeningRules {
    /// `data/rules/battle_opening.json` as compiled into the crate.
    pub fn bundled() -> &'static OpeningRules {
        static RULES: OnceLock<OpeningRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_opening.json is valid")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_rules_load() {
        let rules = OpeningRules::bundled();
        assert!(rules.flank.far_m > rules.flank.near_m);
        assert!(rules.palisade.melee_defense >= 1.0);
        assert!(rules.palisade.horse_crossing_factor <= rules.palisade.foot_crossing_factor);
    }
}
