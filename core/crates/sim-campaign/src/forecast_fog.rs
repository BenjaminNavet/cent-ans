//! Fog of the pre-battle forecast (M5, ADR 0722).
//!
//! The odds shown on the pre-battle screen are blurred according to what the
//! viewer knows of the enemy army. The noise is deterministic for a given
//! battle and touches only the displayed forecast: the auto-resolver and the
//! 3D battle never see it.

use data_model::{ForecastFogRules, GameData};

use crate::state::{Army, CampaignState};

/// What the viewer knows of the enemy side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Knowledge {
    /// Not in sight.
    Unseen,
    /// In sight of scouts or the vision mask.
    Scouted,
    /// A spy is near.
    Spied,
    /// Spied and seen from a friendly settlement.
    SpiedFromPlace,
}

impl Knowledge {
    /// Half-width of the relative noise for this knowledge level.
    pub fn spread(self, rules: &ForecastFogRules) -> f64 {
        let _ = rules;
        todo!("M5")
    }
}

impl CampaignState {
    /// Knowledge the player has of `enemy`.
    pub fn forecast_knowledge(&self, data: &GameData, enemy: &Army) -> Knowledge {
        let _ = (data, enemy);
        todo!("M5")
    }
}

/// Deterministic multiplier in `1 ± spread` from `seed`.
pub fn fog_factor(seed: u64, spread: f64) -> f64 {
    let _ = (seed, spread);
    todo!("M5")
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "M5 skeleton"]
    fn tx_m5_prevision_fog_factor_within_spread() {}
}
