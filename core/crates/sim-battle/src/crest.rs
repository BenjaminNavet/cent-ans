//! SG4 (after R4, ADR 0046): the height advantage in melee. A regiment
//! fighting downhill strikes harder, one fighting uphill strikes weaker;
//! the rules are in `data/rules/battle_crest.json` (schema
//! `data/schemas/battle_crest_rules.schema.json`).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/battle_crest.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrestRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Share of melee damage gained (downhill) or lost (uphill) per metre
    /// of height over the enemy.
    pub melee_per_m: f64,
    /// Height differences up to this are mere folds of the ground: no
    /// advantage (metres); beyond it the advantage grows.
    pub min_height_m: f64,
    /// Height difference beyond which the advantage stops growing (metres).
    pub max_height_m: f64,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_crest.json");

impl CrestRules {
    /// `data/rules/battle_crest.json` as compiled into the crate.
    pub fn bundled() -> &'static CrestRules {
        static RULES: OnceLock<CrestRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_crest.json is valid")
        })
    }

    /// Melee damage factor of a regiment standing `height_over` metres above
    /// its enemy (negative: below it).
    pub fn melee_factor(&self, height_over: f64) -> f64 {
        let h = height_over.clamp(-self.max_height_m, self.max_height_m);
        let beyond = (h.abs() - self.min_height_m).max(0.0);
        1.0 + self.melee_per_m * beyond * h.signum()
    }
}

/// SG5: contents of `data/rules/battle_crest_defence.json`: how a
/// defensive side on its heights keeps its shooters and horse from
/// breaking its own line (ADR 0046 § Suite SG5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrestDefenceRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The horse counter-charges enemy horse this close to itself (metres).
    pub horse_counter_m: f64,
    /// ... or enemy horse this close to one of its foot regiments (metres).
    pub horse_guard_m: f64,
    /// The horse holds the wing this far beyond the end of the line (metres).
    pub horse_wing_gap_m: f64,
    /// ... and this far behind it (metres).
    pub horse_wing_depth_m: f64,
}

const BUNDLED_DEFENCE: &str = include_str!("../../../../data/rules/battle_crest_defence.json");

impl CrestDefenceRules {
    /// `data/rules/battle_crest_defence.json` as compiled into the crate.
    pub fn bundled() -> &'static CrestDefenceRules {
        static RULES: OnceLock<CrestDefenceRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED_DEFENCE)
                .expect("data/rules/battle_crest_defence.json is valid")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downhill_strikes_harder_within_the_cap() {
        let rules = CrestRules::bundled();
        assert!(rules.melee_factor(4.0) > 1.0);
        assert!(rules.melee_factor(-4.0) < 1.0);
        assert_eq!(rules.melee_factor(0.0), 1.0);
        assert_eq!(rules.melee_factor(rules.min_height_m * 0.5), 1.0);
        assert_eq!(
            rules.melee_factor(100.0),
            rules.melee_factor(rules.max_height_m)
        );
    }
}
