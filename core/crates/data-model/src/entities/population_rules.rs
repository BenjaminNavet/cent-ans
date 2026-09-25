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
    /// Ceiling of the relief brought by well-supplied goods (the term
    /// `(50 - goods satisfaction) / 3` never goes below minus this).
    pub goods_relief_max: f64,
    /// Unrest of a province held by another faction than its owner.
    pub occupation_unrest: f64,
    /// Unrest of a province whose ruler is of another faith.
    pub foreign_religion_unrest: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for PopulationRules {
    /// Fallback when `data/rules/population.json` is absent; kept equal to
    /// that file (checked by `tests/real_data.rs`).
    fn default() -> Self {
        PopulationRules {
            tax_unrest_weight: 100.0,
            garrison_relief_per_100_men: 1.0,
            garrison_relief_max: 10.0,
            goods_relief_max: 10.0,
            occupation_unrest: 35.0,
            foreign_religion_unrest: 10.0,
            description: None,
        }
    }
}
