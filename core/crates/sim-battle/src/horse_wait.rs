//! EQ7 (suite of ADR 0052): the horse of an army with foot waits for it
//! before riding into the range of enemy shooters who still have arrows
//! (`data/rules/battle_horse_wait.json`).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// When the battle AI holds its horse out of the enemy shooters' reach.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HorseWaitRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The horse charges a covered target once its foot is this close to
    /// it (metres).
    pub foot_close_m: f64,
    /// A target is covered within a shooter's effective range times this.
    pub range_margin: f64,
    /// A troop this close to its target (metres) no longer stops.
    pub committed_m: f64,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_horse_wait.json");

impl HorseWaitRules {
    /// `data/rules/battle_horse_wait.json` as compiled into the crate.
    pub fn bundled() -> &'static HorseWaitRules {
        static RULES: OnceLock<HorseWaitRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_horse_wait.json is valid")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load() {
        let rules = HorseWaitRules::bundled();
        assert!(rules.committed_m <= rules.foot_close_m);
        assert!(rules.range_margin >= 1.0);
    }
}
