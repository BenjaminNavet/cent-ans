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

impl Default for PopulationRules {
    /// Fallback when `data/rules/population.json` is absent; kept equal to
    /// that file (checked by `tests/real_data.rs`).
    fn default() -> Self {
        PopulationRules {
            tax_unrest_weight: 130.0,
            garrison_relief_per_100_men: 1.0,
            garrison_relief_max: 10.0,
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

fn default_disorder_weight() -> f64 {
    0.5
}

fn default_disorder_max() -> f64 {
    20.0
}

/// RS-B (ADR 0100): 75 before.
fn default_revolt_threshold() -> f64 {
    74.0
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
