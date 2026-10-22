//! Graded revolts, mirroring `data/schemas/revolt_rules.schema.json`
//! (`data/rules/revolt.json`, ADR 0731).

use serde::{Deserialize, Serialize};

/// Size and make-up of the rebel army raised by an uprising.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RebelLevy {
    /// Inhabitants of the province for one rebel regiment.
    pub population_per_regiment: u64,
    pub min_regiments: u32,
    pub max_regiments: u32,
    /// Regiment type raised among the peasants.
    pub peasant_unit_type: String,
    /// Regiment type raised among the burghers (in proportion to their
    /// share of the population).
    pub burgher_unit_type: String,
}

/// Contents of `data/rules/revolt.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevoltRules {
    /// Tension at which the province rises (ceiling of the gauge).
    pub tension_max: u32,
    /// Rise per season spent above the unrest threshold.
    pub tension_rise_base: f64,
    /// Extra rise per point of unrest above the threshold.
    pub tension_rise_per_point: f64,
    /// Fall per season below the threshold.
    pub tension_decay: f64,
    /// Tension levels that issue a warning when crossed upwards.
    pub warning_tensions: Vec<u32>,
    /// Tension right after the uprising.
    pub tension_after_revolt: u32,
    /// Share of the city garrison lost to the uprising.
    pub garrison_loss_percent: u32,
    pub rebel_levy: RebelLevy,
    /// The garrison of the city harries the rebels each season: percent of
    /// the rebel strength lost per 1.0 of garrison-to-rebel strength ratio...
    pub garrison_sortie_percent_per_ratio: f64,
    /// ... up to this percent per season.
    pub garrison_sortie_max_percent: f64,
    /// Seasons the rebel army must hold the province before the city falls.
    pub rebel_siege_seasons: u32,
    /// The rebel army disperses under this share of its raised strength.
    pub rebel_rout_percent: u32,
    /// Tension ceiling once the revolt is put down.
    pub repression_tension: u32,
    /// Recent-disorder points removed by the repression.
    pub repression_disorder_relief: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl RevoltRules {
    /// Tension added by one season at weighted unrest `weighted` over
    /// `threshold` (0 when below it).
    pub fn rise(&self, weighted: f64, threshold: f64) -> f64 {
        if weighted > threshold {
            self.tension_rise_base + self.tension_rise_per_point * (weighted - threshold)
        } else {
            0.0
        }
    }

    /// Fewest seasons a province needs to rise from a calm state (unrest
    /// at its ceiling of 100).
    pub fn min_seasons_to_revolt(&self) -> u32 {
        let top = self.tension_rise_base + self.tension_rise_per_point * 100.0;
        (f64::from(self.tension_max) / top.max(1.0)).ceil() as u32
    }
}

crate::bundled_rules!(RevoltRules, "rules/revolt.json", default);
