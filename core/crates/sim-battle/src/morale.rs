//! Morale and fatigue modifiers of a regiment, in one place. Values in
//! `data/rules/battle_morale.json` (schema `battle_morale_rules.schema.json`);
//! the fighting rates by battle mode are in [`crate::pace::Pace`].

use serde::{Deserialize, Serialize};

use crate::capture::LastStandRules;
use crate::field::Weather;
use crate::pace::Pace;
use crate::rout::ContagionRules;
use crate::sim::DT;
use crate::unit::{Unit, UnitState};

/// Contents of `data/rules/battle_morale.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoraleRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Fatigue above which a regiment is exhausted.
    pub exhausted_fatigue: f64,
    pub cover: CoverRules,
    /// Morale lost per second per fatigue point above `exhausted_fatigue`.
    pub exhaustion_morale_per_s: f64,
    pub outnumbered_melee: OutnumberedRules,
    pub general_aura: AuraRules,
    pub recovery: RecoveryRules,
    pub rally: RallyRules,
    pub fatigue_per_s: FatigueRules,
}

/// Factors on the morale lost to casualties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverRules {
    /// Ram or siege tower.
    pub siege_machine: f64,
    /// Regiment standing on the rampart.
    pub on_wall: f64,
}

/// A regiment in melee under this share of its men loses morale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutnumberedRules {
    pub hp_fraction: f64,
    pub morale_per_s: f64,
}

/// Morale regained near the general: `base + command * per_command` a second.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuraRules {
    pub radius_m: f64,
    pub base_per_s: f64,
    pub per_command_per_s: f64,
}

/// Morale regained per second, by situation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryRules {
    pub routing_per_s: f64,
    pub calm_per_s: f64,
    pub calm_enemy_distance_m: f64,
    pub behind_walls_per_s: f64,
    /// The aura adds morale until the cap plus this margin.
    pub aura_cap_margin: f64,
    /// Melee holds firm (`Pace::melee_resolve_per_s`) above this share of men.
    pub resolve_hp_fraction: f64,
}

/// Conditions to rally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RallyRules {
    pub safe_distance_m: f64,
    pub pause_s: f64,
    pub min_hp_fraction: f64,
}

/// Fatigue gained per second by activity (negative: recovers).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FatigueRules {
    pub rest: f64,
    pub shooting: f64,
    pub run: f64,
    pub march: f64,
    pub charge: f64,
    pub rout: f64,
    pub snow_factor: f64,
    pub mounted_factor: f64,
}

data_model::bundled_rules!(MoraleRules, "rules/battle_morale.json");

/// What the regiment's surroundings contribute to one morale step.
pub struct MoraleContext<'a> {
    pub pace: &'a Pace,
    pub contagion: &'a ContagionRules,
    /// The siege garrison's last stand, when this regiment is in it.
    pub last_stand: Option<&'a LastStandRules>,
    /// Weight of the routing friends around (see `ContagionRules::weight`).
    pub routing_weight: f64,
    /// Distance to the nearest able enemy.
    pub nearest_enemy: f64,
    /// Morale per second from the general's aura (0 out of reach).
    pub aura: f64,
    /// In contact with an enemy.
    pub engaged: bool,
}

impl MoraleRules {
    /// Morale per second the general's aura gives at squared distance
    /// `dist2` from a general of `command`.
    pub fn aura(&self, command: f64, dist2: f64) -> f64 {
        let aura = &self.general_aura;
        if dist2 < aura.radius_m * aura.radius_m {
            aura.base_per_s + command * aura.per_command_per_s
        } else {
            0.0
        }
    }

    /// Morale of `unit` after one step: losses, flanking, exhaustion and
    /// contagion drain it; calm, walls, the last stand, the aura and the
    /// resolve of the melee give it back. Clamped to `0..=100`.
    pub fn next_morale(&self, unit: &Unit, ctx: &MoraleContext) -> f64 {
        let pace = ctx.pace;
        let mut morale = unit.morale;
        let (stand_loss, stand_contagion) = ctx.last_stand.map_or((1.0, 1.0), |stand| {
            (stand.loss_morale_factor, stand.contagion_factor)
        });
        let cover = if unit.ram || unit.siege_tower() {
            self.cover.siege_machine
        } else if unit.on_wall {
            self.cover.on_wall
        } else {
            1.0
        };
        let strength = f64::from(unit.max_soldiers);
        morale -= unit.tick_losses / strength
            * pace.loss_morale_factor
            * cover
            * stand_loss
            * unit.morale_loss_factor();
        if unit.flanked & 1 != 0 {
            morale -= pace.flank_morale_per_s * DT;
        }
        if unit.flanked & 2 != 0 {
            morale -= pace.rear_morale_per_s * DT;
        }
        if unit.fatigue > self.exhausted_fatigue {
            morale -= (unit.fatigue - self.exhausted_fatigue) * self.exhaustion_morale_per_s * DT;
        }
        if unit.state == UnitState::Melee && unit.hp < strength * self.outnumbered_melee.hp_fraction
        {
            morale -= self.outnumbered_melee.morale_per_s * DT;
        }
        morale -= ctx.contagion.morale_rate(ctx.routing_weight)
            * DT
            * stand_contagion
            * pace.contagion_factor;

        let recovery = &self.recovery;
        let aura = ctx.aura * DT;
        let cap = unit.morale_cap;
        if unit.state == UnitState::Routing {
            if ctx.nearest_enemy > self.rally.safe_distance_m {
                morale += recovery.routing_per_s * DT + aura;
            }
        } else if !ctx.engaged && ctx.nearest_enemy > recovery.calm_enemy_distance_m {
            if morale < cap {
                morale = (morale + recovery.calm_per_s * DT + aura).min(cap);
            }
        } else if unit.on_wall && unit.side == crate::setup::SideId::Defender {
            // Behind their walls the burghers stand firm.
            if morale < cap {
                morale = (morale + recovery.behind_walls_per_s * DT + aura).min(cap);
            }
        } else if let Some(stand) = ctx.last_stand {
            if morale < cap {
                morale = (morale + stand.morale_per_s * DT + aura).min(cap);
            }
        } else if morale < cap + recovery.aura_cap_margin {
            morale += aura;
        }
        if unit.state == UnitState::Melee
            && unit.flanked == 0
            && unit.hp >= strength * recovery.resolve_hp_fraction
            && morale < cap
        {
            morale = (morale + pace.melee_resolve_per_s * DT).min(cap);
        }
        morale.clamp(0.0, 100.0)
    }

    /// Fatigue of `unit` after one step (`0..=100`).
    pub fn next_fatigue(&self, unit: &Unit, pace: &Pace, weather: Weather) -> f64 {
        let rates = &self.fatigue_per_s;
        let mut rate = match unit.state {
            UnitState::Idle | UnitState::Rallied => rates.rest,
            UnitState::Shooting => rates.shooting,
            UnitState::Marching if unit.running || unit.withdrawing => rates.run,
            UnitState::Marching => rates.march,
            UnitState::Charging => rates.charge,
            UnitState::Melee | UnitState::Climbing => pace.melee_fatigue_per_s,
            UnitState::Routing => rates.rout,
        };
        if rate > 0.0 {
            rate *= unit.run_mode_fatigue();
            if weather == Weather::Snow {
                rate *= rates.snow_factor;
            }
            if unit.mounted {
                rate *= rates.mounted_factor;
            }
        }
        (unit.fatigue + rate * DT).clamp(0.0, 100.0)
    }

    /// A routing `unit` rallies once its morale tops the mode's
    /// `rally_morale`, no enemy is near and enough men are left.
    pub fn can_rally(&self, unit: &Unit, pace: &Pace, nearest_enemy: f64) -> bool {
        unit.state == UnitState::Routing
            && unit.morale > pace.rally_morale
            && nearest_enemy > self.rally.safe_distance_m
            && unit.hp >= f64::from(unit.max_soldiers) * self.rally.min_hp_fraction
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load_with_the_original_constants() {
        let rules = MoraleRules::bundled();
        assert_eq!(rules.exhausted_fatigue, 60.0);
        assert_eq!(rules.cover.on_wall, 0.7);
        assert_eq!(rules.rally.pause_s, 5.0);
        assert_eq!(rules.aura(2.0, 100.0), 0.2);
        assert_eq!(rules.aura(2.0, 151.0 * 151.0), 0.0);
    }
}
