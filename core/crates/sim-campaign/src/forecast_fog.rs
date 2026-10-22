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
        match self {
            Self::Unseen => rules.spread_unseen,
            Self::Scouted => rules.spread_scouted,
            Self::Spied => rules.spread_spied,
            Self::SpiedFromPlace => rules.spread_spied_from_place,
        }
    }
}

impl CampaignState {
    /// Knowledge the player has of an enemy standing at `point`: `army` is
    /// the enemy army, `None` for the garrison of a place.
    pub fn forecast_knowledge(
        &self,
        data: &GameData,
        point: [f32; 2],
        army: Option<&Army>,
    ) -> Knowledge {
        let player = &self.player_faction;
        let ambush = &data.posture_rules.ambush;
        let px_per_km = f64::from(crate::march::px_per_km(data));
        let near = |other: [f32; 2], radius_km: f64| {
            let (dx, dy) = (
                f64::from(other[0] - point[0]),
                f64::from(other[1] - point[1]),
            );
            (dx * dx + dy * dy).sqrt() / px_per_km <= radius_km
        };
        let spied = self.agents.agents.values().any(|agent| {
            agent.kind == data_model::AgentKind::Spy
                && self.is_allied(player, &agent.faction)
                && data
                    .settlement_point(&agent.location)
                    .is_some_and(|p| near(p, ambush.detect_radius_spy_km))
        });
        if spied {
            let from_place = self.settlements.iter().any(|(id, s)| {
                self.is_allied(player, &s.controller)
                    && data
                        .settlement_point(id)
                        .is_some_and(|p| near(p, ambush.detect_radius_army_km))
            });
            return if from_place {
                Knowledge::SpiedFromPlace
            } else {
                Knowledge::Spied
            };
        }
        let vision = self.vision(data, player);
        let seen = match army {
            Some(army) => vision.sees_army(self, data, army),
            None => vision.mask.sees_point(data, point),
        };
        if seen {
            Knowledge::Scouted
        } else {
            Knowledge::Unseen
        }
    }
}

/// Deterministic multiplier in `1 +/- spread` from `seed` (splitmix64).
pub fn fog_factor(seed: u64, spread: f64) -> f64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let unit = (z >> 11) as f64 / (1u64 << 53) as f64; // [0, 1)
    1.0 + spread * (2.0 * unit - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tx_m5_prevision_fog_factor_within_spread() {
        for seed in 0..2000u64 {
            let factor = fog_factor(seed, 0.15);
            assert!((0.85..=1.15).contains(&factor));
            assert_eq!(factor, fog_factor(seed, 0.15));
        }
        assert_eq!(fog_factor(7, 0.0), 1.0);
    }

    #[test]
    fn tx_m5_prevision_spread_follows_knowledge() {
        let rules = ForecastFogRules::default();
        let spreads = [
            Knowledge::Unseen,
            Knowledge::Scouted,
            Knowledge::Spied,
            Knowledge::SpiedFromPlace,
        ]
        .map(|k| k.spread(&rules));
        assert!(spreads.windows(2).all(|w| w[0] > w[1]));
    }
}
