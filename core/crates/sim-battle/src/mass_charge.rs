//! IA-CAV: the coordinated charge of the horse (`data/rules/battle_mass_charge.json`).

use serde::{Deserialize, Serialize};

/// How the battle AI lines up its horse and launches it together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct MassChargeRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    /// Targeted shooters lie within this distance of the group (metres).
    pub(crate) reach_m: f64,
    /// Squadrons line up this far beyond the shooters' effective range.
    pub(crate) stage_margin_m: f64,
    /// A squadron this close to its place in the line is ready (metres).
    pub(crate) ready_m: f64,
    /// A squadron standing still this long near its place counts as ready.
    pub(crate) stalled_s: f64,
    /// The group must weigh at least this times the shooters it rides at.
    pub(crate) min_power_ratio: f64,
    /// Shooters within this distance of the target form its cluster (metres).
    pub(crate) cluster_m: f64,
}

data_model::bundled_rules!(MassChargeRules, "rules/battle_mass_charge.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load() {
        let rules = MassChargeRules::bundled();
        assert!(rules.ready_m > 0.0 && rules.reach_m > rules.stage_margin_m);
    }
}
