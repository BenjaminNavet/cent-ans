//! Morale under missiles (ADR 0052).
//!
//! An arrow or a bolt that strikes a mounted regiment wounds horses as well
//! as riders: the maddened horses rear, throw their riders and turn back into
//! the ranks (Crécy, Poitiers, Agincourt). On top of the ordinary morale cost
//! of losses, a mounted regiment loses `mounted_panic_per_loss` points of
//! morale per whole strength killed by missiles. The rules live in
//! `data/rules/missile_morale.json` (schema
//! `data/schemas/missile_morale_rules.schema.json`).

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/missile_morale.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissileMoraleRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Extra morale a mounted regiment loses per whole strength killed by
    /// missiles (wounded horses).
    pub mounted_panic_per_loss: f64,
}

data_model::bundled_rules!(MissileMoraleRules, "rules/missile_morale.json");

impl MissileMoraleRules {

    /// Extra morale a regiment loses when missiles kill `kills` of its
    /// `max_soldiers` men: nothing on foot, the panic of the horses mounted.
    pub fn panic(&self, mounted: bool, kills: f64, max_soldiers: u32) -> f64 {
        if mounted && max_soldiers > 0 {
            kills / f64::from(max_soldiers) * self.mounted_panic_per_loss
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load() {
        assert!(MissileMoraleRules::bundled().mounted_panic_per_loss > 0.0);
    }

    #[test]
    fn only_horsemen_panic() {
        let rules = MissileMoraleRules::bundled();
        assert_eq!(rules.panic(false, 30.0, 60), 0.0);
        let half = rules.panic(true, 30.0, 60);
        assert!((half - rules.mounted_panic_per_loss / 2.0).abs() < 1e-9);
    }
}
