//! Active abilities of a regiment in battle (lot CB4, `battle_ability.schema.json`,
//! `data/battle_abilities/`, historian review `docs/research/cb4-capacites.md`).
//! The rules live in `sim-battle` (`abilities.rs`); this is only the catalogue:
//! names, texts, eligibility and every number of the ability.

use serde::{Deserialize, Serialize};

use crate::common::Sources;
use crate::entities::unit_type::Ability;

/// What the ability does (each kind has its own rule in `sim-battle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbilityKind {
    /// « Tir tendu » : archers aim flat at short range (range down,
    /// accuracy up, above all against horses).
    AimedShot,
    /// « Dresser les pavois » : crossbowmen plant their pavises (missiles
    /// from the front hurt far less once they stand).
    Pavise,
    /// « Se rallier à la bannière » : heavy horse halts and reforms after
    /// the shock (morale recovered faster).
    BannerRally,
    /// « Serrer les rangs » : foot close up (front holds better, flanks and
    /// missiles hurt more, slower, more tiring).
    CloseRanks,
    /// « Piques plantées » : pikes grounded against a frontal charge.
    PlantedPikes,
}

/// A condition of the ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbilityCondition {
    /// The regiment stands still: using the ability halts it, and any move
    /// ends it.
    Stationary,
    /// Not in melee.
    NotEngaged,
    /// Missiles left.
    #[serde(rename = "ammo_gt_0")]
    AmmoGt0,
}

/// Regiments that have the ability: a listed unit type, or a listed
/// passive ability, or (with `dismounted`) any regiment fighting on foot
/// after dismounting; then none of `exclude_abilities` and the `mounted`
/// filter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleAbilityFilter {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unit_types: Vec<String>,
    /// Passive abilities that grant it (any of them).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<Ability>,
    /// Also every regiment fighting on foot after dismounting.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dismounted: bool,
    /// Passive abilities that exclude it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_abilities: Vec<Ability>,
    /// Only on horseback (`true`) or on foot (`false`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mounted: Option<bool>,
}

fn one() -> f64 {
    1.0
}

fn is_one(value: &f64) -> bool {
    (*value - 1.0).abs() < f64::EPSILON
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Numbers of the ability's effect, all neutral by default (1 for the
/// factors, 0 for the bonuses). Only those of its kind are read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleAbilityEffects {
    /// Shooting range multiplier (aimed shot).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub range_factor: f64,
    /// Accuracy multiplier of the regiment's volleys.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub accuracy_factor: f64,
    /// Casualties multiplier of its volleys against mounted targets.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub vs_mounted_factor: f64,
    /// Reload time multiplier.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub reload_factor: f64,
    /// Missile casualties taken (from the front only with `frontal_only`).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub missile_taken_factor: f64,
    /// The protection (missiles, charge) only holds from the front.
    #[serde(default, skip_serializing_if = "is_false")]
    pub frontal_only: bool,
    /// Speed multiplier (the run is forbidden below 1).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub speed_factor: f64,
    /// Melee casualties taken from the front.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub melee_taken_front_factor: f64,
    /// Melee casualties taken on the flank or in the rear.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub melee_taken_flank_factor: f64,
    /// Men knocked down and cohesion lost to a charge.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub charge_taken_factor: f64,
    /// Melee casualties taken from horsemen in front.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub horse_taken_front_factor: f64,
    /// Melee casualties dealt to horsemen in front.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub vs_horse_front_factor: f64,
    /// A frontal cavalry charge breaks on the regiment (as on pikes).
    #[serde(default, skip_serializing_if = "is_false")]
    pub stops_charge: bool,
    /// Fatigue multiplier in melee.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub melee_fatigue_factor: f64,
    /// Morale points recovered at once.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub morale_bonus: f64,
    /// Morale points recovered per second out of contact (up to the
    /// regiment's ceiling).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub morale_recovery: f64,
}

impl Default for BattleAbilityEffects {
    fn default() -> Self {
        BattleAbilityEffects {
            range_factor: 1.0,
            accuracy_factor: 1.0,
            vs_mounted_factor: 1.0,
            reload_factor: 1.0,
            missile_taken_factor: 1.0,
            frontal_only: false,
            speed_factor: 1.0,
            melee_taken_front_factor: 1.0,
            melee_taken_flank_factor: 1.0,
            charge_taken_factor: 1.0,
            horse_taken_front_factor: 1.0,
            vs_horse_front_factor: 1.0,
            stops_charge: false,
            melee_fatigue_factor: 1.0,
            morale_bonus: 0.0,
            morale_recovery: 0.0,
        }
    }
}

/// When the battle AI uses the ability (one simple rule per kind in
/// `sim-battle/src/ai_abilities.rs`; only the fields of its kind are read).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleAbilityAi {
    /// An enemy (mounted with `mounted_enemy`) closer than this, in metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enemy_within: Option<f64>,
    /// Only against a mounted enemy.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mounted_enemy: bool,
    /// The enemy stands in front of the regiment.
    #[serde(default, skip_serializing_if = "is_false")]
    pub in_front: bool,
    /// The regiment took missile casualties within this many seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub under_fire_within: Option<f64>,
    /// The regiment's morale below this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub morale_below: Option<f64>,
    /// No enemy closer than this (metres).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_enemy_within: Option<f64>,
    /// Only for a side on the defensive.
    #[serde(default, skip_serializing_if = "is_false")]
    pub when_defensive: bool,
    /// Only for a side on the attack.
    #[serde(default, skip_serializing_if = "is_false")]
    pub when_attacking: bool,
    /// Also in siege battles (field battles only otherwise).
    #[serde(default, skip_serializing_if = "is_false")]
    pub in_sieges: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleAbility {
    /// `ability_*`, equal to the file name.
    pub id: String,
    pub kind: AbilityKind,
    /// Order of the buttons on the unit card (ascending) and of Alt+1…4.
    #[serde(default)]
    pub rank: u8,
    /// French name (« Tir tendu »).
    pub name: String,
    /// French description with its historical reference.
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub eligible: BattleAbilityFilter,
    /// Seconds before it can be used again, counted from its end.
    pub cooldown: f64,
    /// Seconds it lasts (0: until a condition falls or it is lifted).
    #[serde(default)]
    pub duration: f64,
    /// Seconds before the effect counts (pavises planted, pikes grounded).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub setup_time: f64,
    /// Conditions held while it lasts (it ends when one falls).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<AbilityCondition>,
    /// Conditions checked only when it is used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub start_conditions: Vec<AbilityCondition>,
    #[serde(default)]
    pub effects: BattleAbilityEffects,
    /// Journal line; placeholder `{unit}`.
    pub journal: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<BattleAbilityAi>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
