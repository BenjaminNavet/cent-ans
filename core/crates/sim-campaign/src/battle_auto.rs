//! Automatic battle resolution (spec § 1.4).
//!
//! [`resolve_auto`] is a pure function of two sides, a context and the
//! campaign RNG. Each side's power is
//! `Σ strength × attack × (1 + experience/10)` scaled by morale, supply, the
//! general's command, terrain and the enemy's armour (against ranged units).
//! The power ratio decides the winner and the proportional losses.

use serde::{Deserialize, Serialize};

use crate::rng::CampaignRng;

/// One unit as seen by the battle resolver (no id: purely numbers).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleUnit {
    pub strength: u32,
    pub max_strength: u32,
    /// 0-10.
    pub experience: u8,
    /// 0-100.
    pub morale: u8,
    pub melee: u8,
    pub ranged: u8,
    pub armor: u8,
    /// Uses `ranged` instead of `melee` as its attack value.
    pub is_ranged: bool,
}

/// One army in a battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Side {
    pub units: Vec<BattleUnit>,
    /// Command skill of the general (0 when none).
    pub general_command: u8,
    /// 0-100.
    pub supply: u8,
    /// Flat morale bonus from the general's traits/skills (spec § 2
    /// `ArmyMorale`).
    pub general_morale_bonus: f64,
    /// Percent bonus to melee power from the general's traits/skills (spec §
    /// 2 `BattleCharge`).
    pub general_charge_percent: f64,
    /// Percent bonus to ranged power (spec § 2 `BattleRanged`).
    pub general_ranged_percent: f64,
    /// Percent reduction to incoming damage, folded into effective armour
    /// (spec § 2 `BattleDefense`).
    pub general_defense_percent: f64,
}

impl Default for Side {
    fn default() -> Self {
        Side {
            units: Vec::new(),
            general_command: 0,
            supply: 100,
            general_morale_bonus: 0.0,
            general_charge_percent: 0.0,
            general_ranged_percent: 0.0,
            general_defense_percent: 0.0,
        }
    }
}

/// Situation modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BattleContext {
    /// Defender holds hills, forest or mountains (+15 %).
    pub defender_terrain_bonus: bool,
    /// Attacker crosses a river (-20 %).
    pub river_crossing: bool,
    /// Attacker assaults fortifications (-30 %).
    pub walls: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Winner {
    Attacker,
    Defender,
}

/// What happened to one side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SideOutcome {
    /// Effective power after all modifiers (before the random jitter).
    pub power: f64,
    /// Casualties per unit, same order as `Side::units`.
    pub losses: Vec<u32>,
    pub total_losses: u32,
    pub morale_delta: i32,
    /// Average morale fell below the rout threshold.
    pub routed: bool,
    pub general_captured: bool,
}

/// Result of [`resolve_auto`]; the caller decides where the loser retreats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleResult {
    pub winner: Winner,
    pub attacker: SideOutcome,
    pub defender: SideOutcome,
}

/// Morale below which a side routs after defeat.
pub const ROUT_MORALE: f64 = 25.0;
/// Probability (per cent) that the losing general is captured.
pub const CAPTURE_CHANCE_PERCENT: u32 = 10;

fn average(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// Head-count weighted average armour of a side (0-100).
pub fn average_armor(side: &Side) -> f64 {
    let total: u32 = side.units.iter().map(|u| u.strength).sum();
    if total == 0 {
        return 0.0;
    }
    side.units
        .iter()
        .map(|u| f64::from(u.strength) * f64::from(u.armor))
        .sum::<f64>()
        / f64::from(total)
}

/// Effective power of `side` facing an enemy of average armour `enemy_armor`
/// (increased by the enemy general's `BattleDefense`, spec § 2).
pub fn side_power(side: &Side, enemy_armor: f64, modifier: f64) -> f64 {
    let base: f64 = side
        .units
        .iter()
        .map(|unit| {
            let attack = if unit.is_ranged {
                f64::from(unit.ranged)
                    * (1.0 - enemy_armor / 200.0)
                    * (1.0 + side.general_ranged_percent / 100.0)
            } else {
                f64::from(unit.melee) * (1.0 + side.general_charge_percent / 100.0)
            };
            f64::from(unit.strength) / 100.0 * attack * (1.0 + f64::from(unit.experience) / 10.0)
        })
        .sum();
    let morale = (average(side.units.iter().map(|u| f64::from(u.morale)))
        + side.general_morale_bonus)
        .clamp(0.0, 100.0);
    let morale_factor = 0.5 + morale / 200.0;
    let supply_factor = 0.7 + 0.3 * f64::from(side.supply) / 100.0;
    let general_factor = 1.0 + f64::from(side.general_command) * 0.03;
    base * morale_factor * supply_factor * general_factor * modifier
}

/// `average_armor` plus the general's `BattleDefense` bonus, folded in the
/// same units (percentage points of the 0-100 armour scale, spec § 2).
pub fn effective_armor(side: &Side) -> f64 {
    (average_armor(side) + side.general_defense_percent).clamp(0.0, 100.0)
}

/// Resolves a battle. Deterministic for a given RNG state.
pub fn resolve_auto(
    attacker: &Side,
    defender: &Side,
    context: &BattleContext,
    rng: &mut CampaignRng,
) -> BattleResult {
    let mut attacker_modifier = 1.0;
    if context.river_crossing {
        attacker_modifier *= 0.8;
    }
    if context.walls {
        attacker_modifier *= 0.7;
    }
    let defender_modifier = if context.defender_terrain_bonus {
        1.15
    } else {
        1.0
    };
    let attacker_power = side_power(attacker, effective_armor(defender), attacker_modifier);
    let defender_power = side_power(defender, effective_armor(attacker), defender_modifier);

    // Fortune of war: ±10 % on each side.
    let attacker_roll = attacker_power * (0.9 + 0.2 * rng.unit_f64());
    let defender_roll = defender_power * (0.9 + 0.2 * rng.unit_f64());
    let winner = if attacker_roll >= defender_roll {
        Winner::Attacker
    } else {
        Winner::Defender
    };
    let (winning_roll, losing_roll) = match winner {
        Winner::Attacker => (attacker_roll, defender_roll),
        Winner::Defender => (defender_roll, attacker_roll),
    };
    let ratio = if losing_roll <= f64::EPSILON {
        2.0
    } else {
        (winning_roll / losing_roll).min(2.0)
    };
    let advantage = (ratio - 1.0).clamp(0.0, 1.0);
    let loser_fraction = 0.15 + 0.25 * advantage;
    let winner_fraction = 0.15 - 0.10 * advantage;

    let mut outcome = |side: &Side, power: f64, won: bool| -> SideOutcome {
        let fraction = if won { winner_fraction } else { loser_fraction };
        let losses: Vec<u32> = side
            .units
            .iter()
            .map(|unit| ((f64::from(unit.strength) * fraction).round() as u32).min(unit.strength))
            .collect();
        let total_losses = losses.iter().sum();
        let morale_delta = if won { 5 } else { -20 };
        let morale_after = average(
            side.units
                .iter()
                .map(|u| (f64::from(u.morale) + f64::from(morale_delta)).max(0.0)),
        );
        let routed = !won && morale_after < ROUT_MORALE;
        let general_captured =
            !won && side.general_command > 0 && rng.below(100) < CAPTURE_CHANCE_PERCENT;
        SideOutcome {
            power,
            losses,
            total_losses,
            morale_delta,
            routed,
            general_captured,
        }
    };
    let attacker_outcome = outcome(attacker, attacker_power, winner == Winner::Attacker);
    let defender_outcome = outcome(defender, defender_power, winner == Winner::Defender);
    BattleResult {
        winner,
        attacker: attacker_outcome,
        defender: defender_outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(strength: u32, melee: u8, ranged: u8, armor: u8, is_ranged: bool) -> BattleUnit {
        BattleUnit {
            strength,
            max_strength: strength,
            experience: 0,
            morale: 60,
            melee,
            ranged,
            armor,
            is_ranged,
        }
    }

    fn side(units: Vec<BattleUnit>) -> Side {
        Side {
            units,
            ..Default::default()
        }
    }

    fn infantry(count: usize) -> Side {
        side((0..count).map(|_| unit(100, 50, 0, 40, false)).collect())
    }

    #[test]
    fn symmetric_sides_have_equal_power() {
        let result = resolve_auto(
            &infantry(4),
            &infantry(4),
            &BattleContext::default(),
            &mut CampaignRng::from_seed(3),
        );
        assert!((result.attacker.power - result.defender.power).abs() < 1e-9);
        assert_eq!(result.attacker.losses.len(), 4);
        assert_eq!(result.defender.losses.len(), 4);
    }

    #[test]
    fn deterministic_for_same_seed() {
        let a = resolve_auto(
            &infantry(3),
            &infantry(5),
            &BattleContext::default(),
            &mut CampaignRng::from_seed(42),
        );
        let b = resolve_auto(
            &infantry(3),
            &infantry(5),
            &BattleContext::default(),
            &mut CampaignRng::from_seed(42),
        );
        assert_eq!(a, b);
    }

    #[test]
    fn numbers_win_and_loser_bleeds_more() {
        let mut wins = 0;
        for seed in 0..50 {
            let result = resolve_auto(
                &infantry(8),
                &infantry(3),
                &BattleContext::default(),
                &mut CampaignRng::from_seed(seed),
            );
            if result.winner == Winner::Attacker {
                wins += 1;
                let attacker_rate = f64::from(result.attacker.total_losses) / 800.0;
                let defender_rate = f64::from(result.defender.total_losses) / 300.0;
                assert!(defender_rate > attacker_rate);
                assert!((0.05..=0.15).contains(&attacker_rate));
                assert!((0.15..=0.40).contains(&defender_rate));
            }
        }
        assert_eq!(wins, 50, "8 units against 3 must always win");
    }

    #[test]
    fn terrain_favours_defender() {
        let neutral = resolve_auto(
            &infantry(4),
            &infantry(4),
            &BattleContext::default(),
            &mut CampaignRng::from_seed(1),
        );
        let hills = resolve_auto(
            &infantry(4),
            &infantry(4),
            &BattleContext {
                defender_terrain_bonus: true,
                river_crossing: true,
                walls: false,
            },
            &mut CampaignRng::from_seed(1),
        );
        assert!(hills.defender.power > neutral.defender.power);
        assert!(hills.attacker.power < neutral.attacker.power);
    }

    #[test]
    fn armour_blunts_archers() {
        let archers = side(vec![unit(100, 20, 70, 20, true); 4]);
        let light = side(vec![unit(100, 40, 0, 10, false); 4]);
        let heavy = side(vec![unit(100, 40, 0, 80, false); 4]);
        let ctx = BattleContext::default();
        let vs_light = resolve_auto(&archers, &light, &ctx, &mut CampaignRng::from_seed(1));
        let vs_heavy = resolve_auto(&archers, &heavy, &ctx, &mut CampaignRng::from_seed(1));
        assert!(vs_light.attacker.power > vs_heavy.attacker.power);
    }
}
