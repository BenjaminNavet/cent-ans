//! Charge impacts and causes of death for the renderer (lot BV2, « bataille
//! vivante » 2).
//!
//! A charge that reaches its target is resolved by [`crate::BattleSim`] as an
//! [`ImpactEvent`]: how many men of the target the horses knock down (they
//! stop fighting for [`knockdown_time`] seconds), how deep the horses drive
//! into the mass, the cohesion (morale) the target loses, and, when pikes or
//! stakes stop the charge, how many riders are unhorsed. The rules live here
//! and in the simulation; the Godot side only draws what the event says
//! (men thrown back, getting up or dying, horses slowed, riders falling).
//!
//! Each regiment also remembers the cause of its latest casualties
//! ([`LossCause`]) and who inflicted them, so that the renderer can choose a
//! fitting death (arrow, melee, charge, cannonball, stakes, pikes, fire).
//!
//! Everything is deterministic: no random draw, the counts follow from the
//! regiments' statistics, formation and the angle of the attack. Numbers in
//! `data/rules/battle_impact.json` ([`ImpactRules`], ADR 0711).

use data_model::key_enum;
use data_model::{Ability, UnitCategory};
use serde::{Deserialize, Serialize};

use crate::unit::Unit;

/// Impacts kept for the renderer between two reads (older ones dropped).
pub(crate) const MAX_PENDING_IMPACTS: usize = 256;

/// Contents of `data/rules/battle_impact.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImpactRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    /// Seconds a knocked-down soldier stays on the ground before fighting again.
    pub knockdown_s: f64,
    /// Men one charging horse can knock down at most.
    pub knocked_per_rider: f64,
    /// Seconds the charge bonus (blows, push) lasts after the impact.
    pub charge_bonus_s: f64,
    pub mass: MassRules,
    pub knocked: KnockedRules,
    pub drive: DriveRules,
    pub cohesion: CohesionRules,
    pub stopped: StoppedRules,
}

/// Weight of a charge: charge/100 × lance × wedge × foot, capped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MassRules {
    pub lance_factor: f64,
    pub foot_factor: f64,
    pub max: f64,
}

/// Share of the target knocked down: weight × exposure × armour × side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnockedRules {
    pub exposure_ranged: f64,
    pub exposure_other: f64,
    pub armor_divisor: f64,
    pub armor_floor: f64,
    pub front_factor: f64,
    pub flank_factor: f64,
    pub rear_factor: f64,
    pub max_share: f64,
}

/// Metres the horses drive in, per rank of men knocked down.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriveRules {
    pub base_m_per_rank: f64,
    pub per_mass_m_per_rank: f64,
    pub max_m: f64,
}

/// Morale the target of a cavalry charge loses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohesionRules {
    pub front: f64,
    pub flank_rear: f64,
}

/// Charges stopped by pikes, stakes, hedges or ditches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoppedRules {
    pub pikes_loss_hp_fraction: f64,
    pub pikes_morale_loss: f64,
    pub stakes_loss_hp_fraction: f64,
    pub stakes_morale_loss: f64,
    pub broken_morale_loss: f64,
}

data_model::bundled_rules!(ImpactRules, "rules/battle_impact.json");

/// Seconds a knocked-down soldier stays on the ground before fighting again.
pub fn knockdown_time() -> f64 {
    ImpactRules::bundled().knockdown_s
}

key_enum! {
/// What happened when a charge reached its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactKind {
    /// The charge went home: men knocked down or thrown back.
    Shock => "shock",
    /// Levelled pikes stopped the horses: riders unhorsed, no knock-down.
    Pikes => "pikes",
    /// The horses impaled themselves on the archers' stakes.
    Stakes => "stakes",
    /// A hedge, a ditch or village lanes broke the charge before contact.
    Broken => "broken",
}
}

key_enum! {
/// Cause of a regiment's latest casualties (chooses the death drawn).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LossCause {
    #[default]
    Other => "other",
    Arrow => "arrow",
    Bolt => "bolt",
    /// Bombard ball.
    Ball => "ball",
    /// Trebuchet or mangonel stone.
    Stone => "stone",
    /// Handheld firearm bullet (couleuvriniers, lot UR2).
    Bullet => "bullet",
    /// Thrown javelin (jinetes, lot UR2).
    Javelin => "javelin",
    Melee => "melee",
    /// Melee while the attacker's charge impact lasts.
    Charge => "charge",
    Stakes => "stakes",
    Pikes => "pikes",
    Fire => "fire",
    /// EP3: swept away in deep water.
    Drowned => "drowned",
}
}

/// One charge impact, as resolved by the simulation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImpactEvent {
    /// Simulated time of the impact (seconds).
    pub time: f64,
    /// Charging regiment id.
    pub attacker: u32,
    /// Charged regiment id.
    pub defender: u32,
    pub kind: ImpactKind,
    /// Contact point (between the two regiments' centres, on the target's face).
    pub point: (f64, f64),
    /// Direction of the charge (radians, same convention as `Unit::facing`).
    pub heading: f64,
    /// Weight of the charge (0-2: charge statistic, lances, wedge, horses).
    pub mass: f64,
    /// Men of the target knocked down (alive, not fighting for [`knockdown_time`]).
    pub knocked: u32,
    /// Riders of the attacker unhorsed (dead in the rules) by pikes or stakes.
    pub unhorsed: u32,
    /// Metres the horses drive into the mass before they stop.
    pub depth: f64,
    /// Morale the target lost to the shock (cohesion).
    pub cohesion: f64,
}

/// Weight of a charge: charge statistic, couched lances, wedge; men on foot
/// weigh far less than horses.
pub fn charge_mass(attacker: &Unit) -> f64 {
    let rules = &ImpactRules::bundled().mass;
    let charge = f64::from(attacker.stats.charge.unwrap_or(20)) / 100.0;
    let lance = lance_factor(attacker);
    let wedge = attacker.formation_charge();
    let horses = if attacker.mounted {
        1.0
    } else {
        rules.foot_factor
    };
    (charge * lance * wedge * horses).clamp(0.0, rules.max)
}

/// Multiplier of couched lances (charge weight and charge bonus).
pub(crate) fn lance_factor(attacker: &Unit) -> f64 {
    if attacker.has(Ability::ChargeLance) {
        ImpactRules::bundled().mass.lance_factor
    } else {
        1.0
    }
}

/// True when levelled pikes stop a cavalry charge: pikemen (schiltron
/// ability) struck in front, or formed in a square.
pub(crate) fn pikes_stop(attacker: &Unit, defender: &Unit, angle: u8) -> bool {
    attacker.is_cavalry()
        && attacker.mounted
        && defender.has(Ability::PikeSquare)
        && (angle == 0 || defender.all_round())
}

/// Share (0-0.6) of the target's men a charge of weight `mass` knocks down,
/// struck in front (`angle` 0), on the flank (1) or in the rear (2).
pub fn knocked_share(mass: f64, defender: &Unit, angle: u8) -> f64 {
    if defender.braced() || (defender.is_cavalry() && defender.mounted) {
        return 0.0;
    }
    let rules = &ImpactRules::bundled().knocked;
    let exposure = match defender.category {
        UnitCategory::Ranged | UnitCategory::Siege => rules.exposure_ranged,
        _ => rules.exposure_other,
    };
    let armour =
        (1.0 - f64::from(defender.stats.armor) / rules.armor_divisor).clamp(rules.armor_floor, 1.0);
    let side = match angle {
        0 => rules.front_factor,
        1 => rules.flank_factor,
        _ => rules.rear_factor,
    };
    (mass * exposure * armour * side).clamp(0.0, rules.max_share)
}

/// Men knocked down by a charge (rounded, deterministic).
pub fn knocked_count(attacker: &Unit, defender: &Unit, angle: u8) -> u32 {
    let share = knocked_share(charge_mass(attacker), defender, angle);
    let reach = f64::from(defender.soldiers())
        .min(f64::from(attacker.soldiers()) * ImpactRules::bundled().knocked_per_rider);
    (share * reach).round().max(0.0) as u32
}

/// Metres the horses drive into the mass: a rank (about a metre) per front
/// of men knocked down, heavier charges further, capped.
pub(crate) fn drive_depth(mass: f64, knocked: u32, defender: &Unit) -> f64 {
    let rules = &ImpactRules::bundled().drive;
    let (_, files) = defender.ranks_files(defender.soldiers());
    let ranks = f64::from(knocked) / f64::from(files.max(1));
    (ranks * (rules.base_m_per_rank + rules.per_mass_m_per_rank * mass)).clamp(0.0, rules.max_m)
}

/// Morale a cavalry charge that goes home costs its target (cohesion):
/// more on the flank or in the rear.
pub fn shock_morale(angle: u8) -> f64 {
    let rules = &ImpactRules::bundled().cohesion;
    if angle == 0 {
        rules.front
    } else {
        rules.flank_rear
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<ImpactKind>();
        assert_keys_match_serde::<LossCause>();
    }
}
