//! Economy tuning (lot EQ1), mirroring
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
    /// B7c: a construction's `cost.resources` come from the provinces
    /// producing them that the faction can reach (`goods`); each missing
    /// unit is imported at the resource's `base_price` times this.
    #[serde(default = "default_resource_import_multiplier")]
    pub resource_import_multiplier: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for EconomyRules {
    /// Fallback when `data/rules/economy.json` is absent; kept equal to that
    /// file (checked by `tests/real_data.rs`).
    fn default() -> Self {
        EconomyRules {
            event_treasury_reference_income: 4000,
            event_treasury_min_scale: 0.25,
            resource_import_multiplier: default_resource_import_multiplier(),
            description: None,
        }
    }
}

fn default_resource_import_multiplier() -> u32 {
    100
}
