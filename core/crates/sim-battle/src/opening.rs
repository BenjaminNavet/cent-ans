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

use crate::field::Battlefield;

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

/// Length of a polyline.
pub fn polyline_length(points: &[(f64, f64)]) -> f64 {
    points
        .windows(2)
        .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
        .sum()
}

/// The longest run of consecutive points of `points` inside `[lo, hi]²`
/// (one rectangle per axis).
fn longest_inside(points: &[(f64, f64)], lo: (f64, f64), hi: (f64, f64)) -> Vec<(f64, f64)> {
    let inside = |p: &(f64, f64)| p.0 >= lo.0 && p.0 <= hi.0 && p.1 >= lo.1 && p.1 <= hi.1;
    let mut best: Vec<(f64, f64)> = Vec::new();
    let mut run: Vec<(f64, f64)> = Vec::new();
    for p in points.iter().chain(std::iter::once(&(f64::NAN, f64::NAN))) {
        if inside(p) {
            run.push(*p);
        } else {
            if polyline_length(&run) > polyline_length(&best) {
                best = std::mem::take(&mut run);
            }
            run.clear();
        }
    }
    best
}

/// Centre line of an ambushed column on `field`: the longest road inside
/// the field (margins of `column.edge_margin_m`), else the long axis of the
/// map. `(path, on_road)`.
pub fn column_path(field: &Battlefield, rules: &OpeningRules) -> (Vec<(f64, f64)>, bool) {
    let m = rules.column.edge_margin_m;
    let (w, d) = (field.width, field.depth);
    let (lo, hi) = ((m, m), (w - m, d - m));
    let best = field
        .roads
        .iter()
        .map(|r| longest_inside(&r.points, lo, hi))
        .filter(|p| p.len() >= 2)
        .max_by(|a, b| polyline_length(a).total_cmp(&polyline_length(b)));
    if let Some(path) = best.filter(|p| polyline_length(p) >= rules.column.min_road_length_m) {
        return (path, true);
    }
    let path = if w >= d {
        vec![(m, d * 0.5), (w - m, d * 0.5)]
    } else {
        vec![(w * 0.5, m), (w * 0.5, d - m)]
    };
    (path, false)
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
