//! Thresholds of the swing causes, mirroring `data/schemas/cause_thresholds.schema.json`
//! (`data/rules/cause_thresholds.json`, XP-POURQUOI): how large a change of the treasury, of the
//! unrest of a province or of an AI's attitude must be to enter the journal of causes.
//! The file is optional: without it the defaults below (those of the shipped file) apply.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CauseThresholds {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Share of the treasury before the season (0.10 = 10 %).
    pub treasury_pct: f64,
    /// Smallest absolute change, in livres.
    pub treasury_min: i64,
    /// Change of a province's unrest, in points.
    pub unrest_points: f64,
    /// Ascending bounds of the attitude tiers.
    pub attitude_bands: Vec<i32>,
    /// Swing causes of the player per season, at most.
    pub max_per_season_player: u32,
    /// Causes of the « Pourquoi » block of the season story.
    pub story_causes: u32,
    /// Terms under a figure of the top bar, at most.
    pub top_terms: u32,
}

impl Default for CauseThresholds {
    fn default() -> Self {
        CauseThresholds {
            description: String::new(),
            treasury_pct: 0.10,
            treasury_min: 300,
            unrest_points: 10.0,
            attitude_bands: vec![-50, -20, 20, 50],
            max_per_season_player: 6,
            story_causes: 3,
            top_terms: 3,
        }
    }
}

impl CauseThresholds {
    /// Tier index (0 = lowest) of an attitude value.
    pub fn attitude_tier(&self, attitude: i32) -> usize {
        self.attitude_bands
            .iter()
            .filter(|bound| attitude >= **bound)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_the_bounds() {
        let t = CauseThresholds::default();
        assert_eq!(t.attitude_tier(-80), 0);
        assert_eq!(t.attitude_tier(-20), 2);
        assert_eq!(t.attitude_tier(0), 2);
        assert_eq!(t.attitude_tier(60), 4);
    }
}
