//! Economy tuning (lots EQ1, B7a, SV4), mirroring
//! `data/schemas/economy_rules.schema.json` (`data/rules/economy.json`).

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/economy.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EconomyRules {
    /// Seasonal income (livres) at and above which a chronicle event's
    /// treasury effect applies in full; a poorer faction pays (or gains) in
    /// proportion to its income...
    pub event_treasury_reference_income: i64,
    /// ... but never less than this share of the written amount (0-1).
    pub event_treasury_min_scale: f64,
    /// B7a: base share of income spent on the court and administration (0-1).
    #[serde(default = "default_administration_base")]
    pub administration_base: f64,
    /// B7a: extra share per province controlled (0-1).
    #[serde(default = "default_administration_per_province")]
    pub administration_per_province: f64,
    /// B7a: ceiling of the administration share (0-1).
    #[serde(default = "default_administration_max")]
    pub administration_max: f64,
    /// B7a (M10/F4): a treasury above this many seasons of income feeds an
    /// opulent court...
    #[serde(default = "default_opulence_seasons")]
    pub opulence_seasons: i64,
    /// ... which spends this percentage of the excess every season.
    #[serde(default = "default_opulence_percent")]
    pub opulence_percent: i64,
    /// B7a: morale lost every season by every unit (armies and garrisons)
    /// while the treasury is negative.
    #[serde(default = "default_bankruptcy_morale_penalty")]
    pub bankruptcy_morale_penalty: u8,
    /// B7a: outside friendly territory, extra supply loss (percent of the
    /// seasonal loss) in a province devastated at 100; proportional below.
    #[serde(default = "default_supply_devastation_loss_percent")]
    pub supply_devastation_loss_percent: f64,
    /// B7a: in friendly territory, share of the seasonal supply recovery
    /// (percent) lost in a province devastated at 100; proportional below.
    #[serde(default = "default_supply_devastation_recovery_cut_percent")]
    pub supply_devastation_recovery_cut_percent: f64,
    /// B7c: a construction's `cost.resources` come from the provinces
    /// producing them that the faction can reach (`goods`); each missing
    /// unit is imported at the resource's `base_price` times this.
    #[serde(default = "default_resource_import_multiplier")]
    pub resource_import_multiplier: u32,
    /// SV4: supply lost per season outside friendly territory (points).
    #[serde(default = "default_supply_loss")]
    pub supply_loss: u8,
    /// SV4: supply lost per winter season outside friendly territory.
    #[serde(default = "default_supply_loss_winter")]
    pub supply_loss_winter: u8,
    /// SV4: supply regained per season in friendly territory.
    #[serde(default = "default_supply_recovery")]
    pub supply_recovery: u8,
    /// SV4: strength lost (percent) each season an army sits at zero supply.
    #[serde(default = "default_starvation_loss_percent")]
    pub starvation_loss_percent: u32,
    /// SV4: devastation healed per season in every province.
    #[serde(default = "default_devastation_decay")]
    pub devastation_decay: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_administration_base() -> f64 {
    0.08
}
fn default_administration_per_province() -> f64 {
    0.01
}
fn default_administration_max() -> f64 {
    0.35
}
fn default_opulence_seasons() -> i64 {
    6
}
fn default_opulence_percent() -> i64 {
    20
}
fn default_bankruptcy_morale_penalty() -> u8 {
    10
}
fn default_supply_devastation_loss_percent() -> f64 {
    50.0
}
fn default_supply_devastation_recovery_cut_percent() -> f64 {
    50.0
}

impl EconomyRules {
    /// Share of income taken by the court and administration for a realm of
    /// `provinces` provinces.
    pub fn administration_rate(&self, provinces: usize) -> f64 {
        (self.administration_base + self.administration_per_province * provinces as f64)
            .min(self.administration_max)
    }
}

impl Default for EconomyRules {
    /// Fallback when `data/rules/economy.json` is absent; kept equal to that
    /// file (checked by `tests/real_data.rs`).
    fn default() -> Self {
        EconomyRules {
            event_treasury_reference_income: 4000,
            event_treasury_min_scale: 0.25,
            administration_base: default_administration_base(),
            administration_per_province: default_administration_per_province(),
            administration_max: default_administration_max(),
            opulence_seasons: default_opulence_seasons(),
            opulence_percent: default_opulence_percent(),
            bankruptcy_morale_penalty: default_bankruptcy_morale_penalty(),
            supply_devastation_loss_percent: default_supply_devastation_loss_percent(),
            supply_devastation_recovery_cut_percent:
                default_supply_devastation_recovery_cut_percent(),
            resource_import_multiplier: default_resource_import_multiplier(),
            supply_loss: default_supply_loss(),
            supply_loss_winter: default_supply_loss_winter(),
            supply_recovery: default_supply_recovery(),
            starvation_loss_percent: default_starvation_loss_percent(),
            devastation_decay: default_devastation_decay(),
            description: None,
        }
    }
}

fn default_resource_import_multiplier() -> u32 {
    100
}
fn default_supply_loss() -> u8 {
    20
}
fn default_supply_loss_winter() -> u8 {
    35
}
fn default_supply_recovery() -> u8 {
    40
}
fn default_starvation_loss_percent() -> u32 {
    10
}
fn default_devastation_decay() -> u8 {
    5
}
