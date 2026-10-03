//! Public order tuning (lot E2), mirroring
//! `data/schemas/population_rules.schema.json` (`data/rules/population.json`).

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/population.json`: the terms of each class's
/// unrest target (0-100) that are not building or event effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationRules {
    /// Unrest per unit of tax burden (Low 0.2, Normal 0.35, High 0.5).
    pub tax_unrest_weight: f64,
    /// Unrest relief per 100 men of garrison in the province...
    pub garrison_relief_per_100_men: f64,
    /// ... up to this much.
    pub garrison_relief_max: f64,
    /// LR-07: inhabitants a garrison is sized for. The relief is weighed by
    /// this over the province's population (a hundred men keep a village
    /// in order, not Paris). 0: no weighting.
    #[serde(default)]
    pub garrison_relief_reference_population: u64,
    /// LR-07: ceiling of that weight in a thinly peopled province (1: no
    /// bonus below the reference population).
    #[serde(default = "default_garrison_relief_max_weight")]
    pub garrison_relief_max_weight: f64,
    /// Ceiling of the relief brought by well-supplied goods (the term
    /// `(50 - goods satisfaction) / 3` never goes below minus this).
    pub goods_relief_max: f64,
    /// Unrest of a province held by another faction than its owner.
    pub occupation_unrest: f64,
    /// Unrest of a province whose ruler is of another faith.
    pub foreign_religion_unrest: f64,
    /// EQ1: weight of the province's recent disorders (captures, raids,
    /// regency: `ProvinceState::unrest`, which decays by itself) in each
    /// class's unrest target.
    #[serde(default = "default_disorder_weight")]
    pub disorder_unrest_weight: f64,
    /// EQ1: ceiling of that term (a province raided or taken again and
    /// again keeps its gauge at 100).
    #[serde(default = "default_disorder_max")]
    pub disorder_unrest_max: f64,
    /// EQ1: weighted unrest above which a province is at risk of revolt.
    #[serde(default = "default_revolt_threshold")]
    pub revolt_unrest_threshold: f64,
    /// EQ1: consecutive seasons above the threshold before a revolt; the
    /// count starts over after each revolt.
    #[serde(default = "default_revolt_seasons")]
    pub revolt_seasons: u32,
    /// Weighted unrest above which a revolting province falls to the rebels.
    #[serde(default = "default_revolt_control_threshold")]
    pub revolt_control_threshold: f64,
    /// EQ2: the province disorder gauge (`ProvinceState::unrest`) loses
    /// this much each season...
    #[serde(default = "default_disorder_decay_flat")]
    pub disorder_decay_flat: u8,
    /// ... plus this percentage of its value (a gauge at 100 falls by
    /// 12 a season, not 2: a pacified province calms down in a few
    /// seasons instead of a decade).
    #[serde(default = "default_disorder_decay_percent")]
    pub disorder_decay_percent: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PopulationRules {
    /// Unrest relief of `garrison_men` (already weighted by place) in a
    /// province of `population` inhabitants (E2, RS-B, LR-07).
    pub fn garrison_relief(&self, garrison_men: u32, population: u64) -> f64 {
        let relief = f64::from(garrison_men / 100) * self.garrison_relief_per_100_men;
        let reference = self.garrison_relief_reference_population;
        let weight = if reference > 0 {
            (reference as f64 / population.max(1) as f64).min(self.garrison_relief_max_weight)
        } else {
            1.0
        };
        (relief * weight).min(self.garrison_relief_max)
    }
}

impl Default for PopulationRules {
    /// Fallback when `data/rules/population.json` is absent; kept equal to
    /// that file (checked by `tests/real_data.rs`).
    fn default() -> Self {
        PopulationRules {
            tax_unrest_weight: 130.0,
            garrison_relief_per_100_men: 1.0,
            garrison_relief_max: 10.0,
            garrison_relief_reference_population: 100_000,
            garrison_relief_max_weight: default_garrison_relief_max_weight(),
            goods_relief_max: 10.0,
            occupation_unrest: 20.0,
            foreign_religion_unrest: 10.0,
            disorder_unrest_weight: default_disorder_weight(),
            disorder_unrest_max: default_disorder_max(),
            revolt_unrest_threshold: default_revolt_threshold(),
            revolt_seasons: default_revolt_seasons(),
            revolt_control_threshold: default_revolt_control_threshold(),
            disorder_decay_flat: default_disorder_decay_flat(),
            disorder_decay_percent: default_disorder_decay_percent(),
            description: None,
        }
    }
}

fn default_garrison_relief_max_weight() -> f64 {
    1.0
}

fn default_disorder_weight() -> f64 {
    0.5
}

fn default_disorder_max() -> f64 {
    20.0
}

fn default_revolt_threshold() -> f64 {
    75.0
}

/// RS-B (ADR 0100): 3 before.
fn default_revolt_seasons() -> u32 {
    2
}

fn default_revolt_control_threshold() -> f64 {
    90.0
}

fn default_disorder_decay_flat() -> u8 {
    2
}

fn default_disorder_decay_percent() -> u8 {
    10
}

#[cfg(test)]
mod tests {
    use super::PopulationRules;

    #[test]
    fn a_garrison_soothes_per_head_beyond_the_reference_population() {
        let rules = PopulationRules {
            garrison_relief_reference_population: 100_000,
            garrison_relief_max_weight: default_garrison_relief_max_weight(),
            ..PopulationRules::default()
        };
        // A small or average province: 1 point per 100 men, capped.
        assert_eq!(rules.garrison_relief(600, 50_000), 6.0);
        assert_eq!(rules.garrison_relief(600, 100_000), 6.0);
        assert_eq!(rules.garrison_relief(5_000, 80_000), 10.0);
        // Twice the reference population: half the relief.
        assert_eq!(rules.garrison_relief(600, 200_000), 3.0);
        // Without weighting, as before LR-07.
        let flat = PopulationRules {
            garrison_relief_reference_population: 0,
            ..rules
        };
        assert_eq!(flat.garrison_relief(600, 200_000), 6.0);
    }
}
