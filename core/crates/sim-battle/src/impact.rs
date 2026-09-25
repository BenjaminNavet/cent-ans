//! Charge impacts and causes of death for the renderer (lot BV2, « bataille
//! vivante » 2).
//!
//! A charge that reaches its target is resolved by [`crate::BattleSim`] as an
//! [`ImpactEvent`]: how many men of the target the horses knock down (they
//! stop fighting for [`KNOCKDOWN_TIME`] seconds), how deep the horses drive
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
//! regiments' statistics, formation and the angle of the attack.

use data_model::{Ability, UnitCategory};
use serde::{Deserialize, Serialize};

use crate::unit::{Formation, Unit};

/// Seconds a knocked-down soldier stays on the ground before fighting again.
pub const KNOCKDOWN_TIME: f64 = 3.0;
/// Impacts kept for the renderer between two reads (older ones dropped).
pub const MAX_PENDING_IMPACTS: usize = 256;
/// Men one charging horse can knock down at most.
pub const KNOCKED_PER_RIDER: f64 = 1.5;
/// Riders lost (share of the charging regiment) on a wall of levelled pikes.
pub const PIKE_STOP_LOSS: f64 = 0.08;

/// What happened when a charge reached its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactKind {
    /// The charge went home: men knocked down or thrown back.
    Shock,
    /// Levelled pikes stopped the horses: riders unhorsed, no knock-down.
    Pikes,
    /// The horses impaled themselves on the archers' stakes.
    Stakes,
    /// A hedge, a ditch or village lanes broke the charge before contact.
    Broken,
}

impl ImpactKind {
    pub fn key(self) -> &'static str {
        match self {
            ImpactKind::Shock => "shock",
            ImpactKind::Pikes => "pikes",
            ImpactKind::Stakes => "stakes",
            ImpactKind::Broken => "broken",
        }
    }
}

/// Cause of a regiment's latest casualties (chooses the death drawn).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LossCause {
    #[default]
    Other,
    Arrow,
    Bolt,
    /// Bombard ball.
    Ball,
    /// Trebuchet or mangonel stone.
    Stone,
    /// Handheld firearm bullet (couleuvriniers, lot UR2).
    Bullet,
    /// Thrown javelin (jinetes, lot UR2).
    Javelin,
    Melee,
    /// Melee while the attacker's charge impact lasts.
    Charge,
    Stakes,
    Pikes,
    Fire,
    /// EP3: swept away in deep water.
    Drowned,
}

impl LossCause {
    pub fn key(self) -> &'static str {
        match self {
            LossCause::Other => "other",
            LossCause::Arrow => "arrow",
            LossCause::Bolt => "bolt",
            LossCause::Ball => "ball",
            LossCause::Stone => "stone",
            LossCause::Bullet => "bullet",
            LossCause::Javelin => "javelin",
            LossCause::Melee => "melee",
            LossCause::Charge => "charge",
            LossCause::Stakes => "stakes",
            LossCause::Pikes => "pikes",
            LossCause::Fire => "fire",
            LossCause::Drowned => "drowned",
        }
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
    /// Men of the target knocked down (alive, not fighting for [`KNOCKDOWN_TIME`]).
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
    let charge = f64::from(attacker.stats.charge.unwrap_or(20)) / 100.0;
    let lance = if attacker.has(Ability::ChargeLance) {
        1.5
    } else {
        1.0
    };
    let wedge = if attacker.formation == Formation::Wedge {
        1.2
    } else {
        1.0
    };
    let horses = if attacker.mounted { 1.0 } else { 0.35 };
    (charge * lance * wedge * horses).clamp(0.0, 2.0)
}

/// True when levelled pikes stop a cavalry charge: pikemen (schiltron
/// ability) struck in front, or formed in a square.
pub fn pikes_stop(attacker: &Unit, defender: &Unit, angle: u8) -> bool {
    attacker.is_cavalry()
        && attacker.mounted
        && defender.has(Ability::PikeSquare)
        && (angle == 0 || defender.formation == Formation::Square)
}

/// Share (0-0.6) of the target's men a charge of weight `mass` knocks down,
/// struck in front (`angle` 0), on the flank (1) or in the rear (2).
pub fn knocked_share(mass: f64, defender: &Unit, angle: u8) -> f64 {
    if defender.formation == Formation::Square || (defender.is_cavalry() && defender.mounted) {
        return 0.0;
    }
    let exposure = match defender.category {
        UnitCategory::Ranged | UnitCategory::Siege => 0.5,
        _ => 0.35,
    };
    let armour = (1.0 - f64::from(defender.stats.armor) / 200.0).clamp(0.4, 1.0);
    let side = match angle {
        0 => 1.0,
        1 => 1.4,
        _ => 1.8,
    };
    (mass * exposure * armour * side).clamp(0.0, 0.6)
}

/// Men knocked down by a charge (rounded, deterministic).
pub fn knocked_count(attacker: &Unit, defender: &Unit, angle: u8) -> u32 {
    let share = knocked_share(charge_mass(attacker), defender, angle);
    let reach =
        f64::from(defender.soldiers()).min(f64::from(attacker.soldiers()) * KNOCKED_PER_RIDER);
    (share * reach).round().max(0.0) as u32
}

/// Metres the horses drive into the mass: a rank (about a metre) per front
/// of men knocked down, heavier charges further, at most six.
pub fn drive_depth(mass: f64, knocked: u32, defender: &Unit) -> f64 {
    let (_, files) = defender.ranks_files(defender.soldiers());
    let ranks = f64::from(knocked) / f64::from(files.max(1));
    (ranks * (0.8 + 0.4 * mass)).clamp(0.0, 6.0)
}

/// Morale a cavalry charge that goes home costs its target (cohesion):
/// more on the flank or in the rear.
pub fn shock_morale(angle: u8) -> f64 {
    if angle == 0 {
        8.0
    } else {
        15.0
    }
}
