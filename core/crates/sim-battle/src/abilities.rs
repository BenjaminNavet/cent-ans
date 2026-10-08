//! Active abilities of the regiments (lot CB4, plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § CB4,
//! historian review `docs/research/cb4-capacites.md`).
//!
//! The catalogue comes from `data/battle_abilities/` through
//! [`crate::BattleSetup::abilities`]; every number (cooldown, duration,
//! setup time, effects, AI thresholds) is read from it. Five abilities,
//! sober and historical (no hero ability):
//!
//! - **aimed shot** (« Tir tendu », archers): shorter range, better aim,
//!   above all at horses; not in melee nor without arrows;
//! - **pavise** (« Dresser les pavois », crossbowmen, culveriners): the
//!   regiment halts, the pavises count after a setup time and only against
//!   missiles from the front; the first move lifts them (it takes over the
//!   leader's order `order_pavise`, kept in the code for older replays);
//! - **banner rally** (« Se rallier à la bannière », heavy horse): halts
//!   and reforms, morale recovered faster; not in melee;
//! - **close ranks** (« Serrer les rangs », foot with `shield_wall`, and
//!   dismounted riders): the front holds better, the flanks, missiles and
//!   fatigue cost more, slower, no run;
//! - **planted pikes** (« Piques plantées », pikemen): a frontal cavalry
//!   charge breaks; only from the front, standing still.
//!
//! A regiment uses one ability at a time. Its cooldown runs from the end of
//! the effect. An ability ends when its duration is over, when one of its
//! `conditions` falls (the reason is kept for the unit card), when the
//! regiment routs or leaves, or when the player lifts it (a second use).
//! The effects are applied where the rules are: `sim.rs` (`speed`,
//! `effective_range`, `fire`, `melee_damage`, `charge_impact`).

use std::collections::BTreeMap;

use data_model::{Ability, AbilityCondition, AbilityKind, BattleAbility, BattleAbilityEffects};
use serde::{Deserialize, Serialize};

use crate::command::CommandError;
use crate::sim::{BattleSim, DT};
use crate::unit::{Unit, UnitState};

/// The ability a regiment is using.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveAbility {
    pub id: String,
    pub kind: AbilityKind,
    /// Simulated time it was used.
    pub since: f64,
}

/// The regiment's last ability that a fallen condition (or a rout) ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndedAbility {
    pub id: String,
    /// Why (French): « l'unité s'est mise en mouvement »…
    pub reason: String,
    pub time: f64,
}

/// Ability state of one regiment (left out of the JSON when empty).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UnitAbilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<ActiveAbility>,
    /// Simulated time from which each used ability can be used again.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ready_at: BTreeMap<String, f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<EndedAbility>,
}

impl UnitAbilities {
    pub fn is_empty(&self) -> bool {
        self.active.is_none() && self.ready_at.is_empty() && self.ended.is_none()
    }

    /// Kind of the ability in use.
    pub fn active_kind(&self) -> Option<AbilityKind> {
        self.active.as_ref().map(|a| a.kind)
    }
}

/// One ability as shown on the unit card (by rank).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbilityView {
    pub id: String,
    pub kind: AbilityKind,
    pub rank: u8,
    pub name: String,
    pub description: String,
    pub icon: String,
    /// Can be used (or lifted, when active) now.
    pub available: bool,
    /// Why it cannot be used now (French), empty when available.
    pub reason: String,
    pub active: bool,
    /// The effect counts (setup time over).
    pub effective: bool,
    /// Seconds before the effect counts.
    pub setup_remaining: f64,
    /// Seconds left of the effect (0 when inactive or open-ended).
    pub remaining: f64,
    pub duration: f64,
    pub cooldown: f64,
    pub cooldown_remaining: f64,
    /// Why this ability last ended, when a condition (or the rout) ended it.
    pub ended_reason: String,
}

/// Does `unit` have `ability`?
pub fn eligible(ability: &BattleAbility, unit: &Unit) -> bool {
    let filter = &ability.eligible;
    let granted = filter.unit_types.iter().any(|t| **t == *unit.unit_type)
        || filter.abilities.iter().any(|a| unit.has(*a))
        || (filter.dismounted && unit.dismounted);
    granted
        && !filter.exclude_abilities.iter().any(|a| unit.has(*a))
        && filter.mounted.is_none_or(|m| m == unit.mounted)
        && !unit.synthetic
}

/// Why `condition` does not hold for `unit` (`None`: it holds).
pub fn condition_fails(unit: &Unit, condition: AbilityCondition) -> Option<&'static str> {
    match condition {
        AbilityCondition::Stationary => (unit.destination.is_some()
            || matches!(unit.state, UnitState::Marching | UnitState::Charging))
        .then_some("l'unité s'est mise en mouvement"),
        AbilityCondition::NotEngaged => {
            (unit.state == UnitState::Melee).then_some("au contact de l'ennemi")
        }
        AbilityCondition::AmmoGt0 => (unit.ammo == 0).then_some("plus de traits"),
    }
}

/// Does a protection with these `effects` cover a blow from `angle`
/// (0 front, 1 flank, 2 rear)?
pub fn covers(effects: &BattleAbilityEffects, angle: u8) -> bool {
    !effects.frontal_only || angle == 0
}

impl BattleSim {
    /// The battle's ability catalogue.
    pub fn ability_catalog(&self) -> &[BattleAbility] {
        &self.setup().abilities
    }

    pub fn find_ability(&self, id: &str) -> Option<&BattleAbility> {
        self.setup().abilities.iter().find(|a| a.id == id)
    }

    /// Abilities of `unit`, by rank then id (at most a few).
    pub fn abilities_of(&self, unit: &Unit) -> Vec<&BattleAbility> {
        let mut list: Vec<&BattleAbility> = self
            .ability_catalog()
            .iter()
            .filter(|a| eligible(a, unit))
            .collect();
        list.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.id.cmp(&b.id)));
        list
    }

    /// Effects of the ability `unit` is using, once they count (setup time
    /// over).
    pub fn ability_effects(&self, unit: &Unit) -> Option<&BattleAbilityEffects> {
        let active = unit.ability_state.active.as_ref()?;
        let ability = self.find_ability(&active.id)?;
        (self.elapsed() - active.since + 1e-9 >= ability.setup_time).then_some(&ability.effects)
    }

    /// Multiplier of the missile casualties `target` takes from a volley
    /// coming from `angle` (0 front, 1 flank, 2 rear): its pavises (the
    /// ability once planted and from the front; else the passive cover of a
    /// pavise regiment that does not march, 0.6; or the leader's order of
    /// older replays), and the dense mass of the close ranks.
    pub fn missile_cover(&self, target: &Unit, angle: u8) -> f64 {
        let passive = if target.has(Ability::Pavise) && target.state != UnitState::Marching {
            0.6
        } else {
            1.0
        };
        let effects = self.ability_effects(target);
        if target.ability_state.active_kind() == Some(AbilityKind::Pavise) {
            return match effects {
                Some(e) if covers(e, angle) => e.missile_taken_factor,
                _ => passive,
            };
        }
        let ability = effects
            .filter(|e| covers(e, angle))
            .map_or(1.0, |e| e.missile_taken_factor);
        target.pavise.unwrap_or(passive) * ability
    }

    /// CB4: multiplier of the melee casualties `defender` takes from
    /// `attacker` striking from `angle` (close ranks, planted pikes), and of
    /// those `attacker` deals (planted pikes against horsemen in front).
    pub fn ability_melee_factor(&self, attacker: &Unit, defender: &Unit, angle: u8) -> f64 {
        let mut factor = 1.0;
        if let Some(e) = self.ability_effects(defender) {
            factor *= if angle == 0 {
                e.melee_taken_front_factor
            } else {
                e.melee_taken_flank_factor
            };
            if angle == 0 && attacker.is_cavalry() {
                factor *= e.horse_taken_front_factor;
            }
        }
        if let Some(e) = self.ability_effects(attacker) {
            if defender.is_cavalry()
                && e.vs_horse_front_factor != 1.0
                && crate::sim::attack_angle(attacker, defender.x, defender.z) == 0
            {
                factor *= e.vs_horse_front_factor;
            }
        }
        factor
    }

    /// CB4: does a charge of `attacker` from `angle` break on the planted
    /// pikes of `defender`?
    pub fn ability_stops_charge(&self, attacker: &Unit, defender: &Unit, angle: u8) -> bool {
        attacker.is_cavalry()
            && attacker.mounted
            && self
                .ability_effects(defender)
                .is_some_and(|e| e.stops_charge && covers(e, angle))
    }

    /// CB4: multiplier of the men a charge knocks down and of the cohesion
    /// lost by `defender` (close ranks).
    pub fn ability_charge_taken(&self, defender: &Unit) -> f64 {
        self.ability_effects(defender)
            .map_or(1.0, |e| e.charge_taken_factor)
    }

    /// Why `unit` cannot use `ability` now (`None`: it can, or it can lift
    /// it when it is in use).
    pub fn ability_unavailable(&self, unit: &Unit, ability: &BattleAbility) -> Option<String> {
        if self.is_finished() {
            return Some("la bataille est terminée".to_owned());
        }
        if self.is_deploying() {
            return Some("pas pendant le déploiement".to_owned());
        }
        if !unit.present() {
            return Some("hors du champ de bataille".to_owned());
        }
        if unit.state == UnitState::Routing {
            return Some("en déroute".to_owned());
        }
        if unit
            .ability_state
            .active
            .as_ref()
            .is_some_and(|a| a.id == ability.id)
        {
            return None;
        }
        let remaining = self.ability_cooldown_remaining(unit, &ability.id);
        if remaining > 1e-9 {
            return Some(format!("recharge, encore {} s", remaining.ceil() as i64));
        }
        // Using a `stationary` ability halts the regiment: only the other
        // conditions must already hold.
        ability
            .start_conditions
            .iter()
            .chain(&ability.conditions)
            .filter(|&&c| c != AbilityCondition::Stationary)
            .find_map(|&c| condition_fails(unit, c))
            .map(str::to_owned)
    }

    /// Seconds before `unit` can use ability `id` again.
    pub fn ability_cooldown_remaining(&self, unit: &Unit, id: &str) -> f64 {
        unit.ability_state
            .ready_at
            .get(id)
            .map_or(0.0, |&t| (t - self.elapsed()).max(0.0))
    }

    /// The ability buttons of `unit`'s card.
    pub fn ability_views(&self, unit: &Unit) -> Vec<AbilityView> {
        let now = self.elapsed();
        self.abilities_of(unit)
            .into_iter()
            .map(|ability| {
                let reason = self.ability_unavailable(unit, ability);
                let active = unit
                    .ability_state
                    .active
                    .as_ref()
                    .filter(|a| a.id == ability.id);
                let since = active.map_or(now, |a| a.since);
                let setup_remaining = if active.is_some() {
                    (ability.setup_time - (now - since)).max(0.0)
                } else {
                    0.0
                };
                let remaining = match active {
                    Some(a) if ability.duration > 0.0 => {
                        (ability.duration - (now - a.since)).max(0.0)
                    }
                    _ => 0.0,
                };
                let ended_reason = unit
                    .ability_state
                    .ended
                    .as_ref()
                    .filter(|e| e.id == ability.id)
                    .map(|e| e.reason.clone())
                    .unwrap_or_default();
                AbilityView {
                    id: ability.id.clone(),
                    kind: ability.kind,
                    rank: ability.rank,
                    name: ability.name.clone(),
                    description: ability.description.clone(),
                    icon: ability.icon.clone().unwrap_or_default(),
                    available: reason.is_none(),
                    reason: reason.unwrap_or_default(),
                    active: active.is_some(),
                    effective: active.is_some() && setup_remaining <= 1e-9,
                    setup_remaining,
                    remaining,
                    duration: ability.duration,
                    cooldown: ability.cooldown,
                    cooldown_remaining: self.ability_cooldown_remaining(unit, &ability.id),
                    ended_reason,
                }
            })
            .collect()
    }

    /// Validates and applies `Command::UseAbility`: starts ability `id` on
    /// the named regiments that have it and can use it, or lifts it when
    /// every one of them already uses it. The regiments are already checked
    /// (ours, present, not routing).
    pub(crate) fn use_ability(&mut self, units: &[u32], id: &str) -> Result<(), CommandError> {
        let ability = self
            .find_ability(id)
            .cloned()
            .ok_or_else(|| CommandError::UnknownAbility(id.to_owned()))?;
        let mut having: Vec<usize> = units
            .iter()
            .map(|&u| u as usize)
            .filter(|&i| eligible(&ability, &self.units()[i]))
            .collect();
        having.sort_unstable();
        having.dedup();
        let refused = |reason: String| CommandError::AbilityUnavailable {
            ability: ability.name.clone(),
            reason,
        };
        if having.is_empty() {
            return Err(refused(
                "aucune des unités désignées n'a cette capacité".to_owned(),
            ));
        }
        let in_use = |sim: &BattleSim, i: usize| {
            sim.units()[i]
                .ability_state
                .active
                .as_ref()
                .is_some_and(|a| a.id == ability.id)
        };
        if having.iter().all(|&i| in_use(self, i)) {
            for &i in &having {
                self.end_ability(i, None);
            }
            return Ok(());
        }
        let mut first_reason = None;
        let mut started = 0;
        for &i in &having {
            if in_use(self, i) {
                continue;
            }
            match self.ability_unavailable(&self.units()[i], &ability) {
                Some(reason) => {
                    first_reason.get_or_insert(reason);
                }
                None => {
                    self.start_ability(i, &ability);
                    started += 1;
                }
            }
        }
        if started == 0 {
            return Err(refused(first_reason.unwrap_or_default()));
        }
        Ok(())
    }

    /// Regiment `i` starts `ability` (its other ability, if any, ends).
    fn start_ability(&mut self, i: usize, ability: &BattleAbility) {
        if self.units()[i].ability_state.active.is_some() {
            self.end_ability(i, None);
        }
        let now = self.elapsed();
        let stationary = ability.conditions.contains(&AbilityCondition::Stationary);
        let unit = &mut self.units_mut()[i];
        if stationary {
            unit.destination = None;
            unit.destination_facing = None;
            unit.order_queue.clear();
            unit.running = false;
            unit.match_speed = false;
            unit.group_tag = None;
            unit.disengaging = false;
            // Behind the pavises the crossbowmen keep their target and
            // wait for it to come within range (as under the old order).
            if ability.kind != AbilityKind::Pavise {
                unit.target = None;
            }
            if matches!(unit.state, UnitState::Marching | UnitState::Charging) {
                unit.state = UnitState::Idle;
            }
        }
        if ability.effects.speed_factor < 1.0 {
            unit.running = false;
        }
        if ability.kind == AbilityKind::Pavise {
            // The marker of raised pavises (movement, AI and rendering read
            // it); the cover itself follows the ability (`fire`).
            unit.pavise = Some(ability.effects.missile_taken_factor);
        }
        if ability.effects.morale_bonus > 0.0 {
            unit.morale = (unit.morale + ability.effects.morale_bonus).min(100.0);
        }
        unit.ability_state.active = Some(ActiveAbility {
            id: ability.id.clone(),
            kind: ability.kind,
            since: now,
        });
        unit.ability_state.ended = None;
        let name = self.unit_label(i);
        let side = self.units()[i].side;
        self.log(ability.journal.replace("{unit}", &name), Some(side));
    }

    /// Ends the ability of regiment `i`; `reason` is kept for the card when
    /// a condition (or the rout) ended it. Its cooldown starts now.
    pub(crate) fn end_ability(&mut self, i: usize, reason: Option<&str>) {
        let now = self.elapsed();
        let Some(active) = self.units_mut()[i].ability_state.active.take() else {
            return;
        };
        let cooldown = self.find_ability(&active.id).map_or(0.0, |a| a.cooldown);
        let unit = &mut self.units_mut()[i];
        unit.ability_state
            .ready_at
            .insert(active.id.clone(), now + cooldown);
        if active.kind == AbilityKind::Pavise {
            unit.pavise = None;
        }
        unit.ability_state.ended = reason.map(|r| EndedAbility {
            id: active.id,
            reason: r.to_owned(),
            time: now,
        });
    }

    /// One step of the abilities in use: end, then the effects over time
    /// (morale recovered at the banner, fatigue of the close ranks, no run).
    pub(crate) fn tick_abilities(&mut self) {
        let now = self.elapsed();
        let melee_rate = self.pace().melee_fatigue_per_s;
        for i in 0..self.units().len() {
            let Some(active) = self.units()[i].ability_state.active.clone() else {
                continue;
            };
            let Some(ability) = self.find_ability(&active.id).cloned() else {
                self.units_mut()[i].ability_state.active = None;
                continue;
            };
            let unit = &self.units()[i];
            let reason = if !unit.present() {
                Some("hors du champ de bataille")
            } else if unit.state == UnitState::Routing {
                Some("en déroute")
            } else if ability.kind == AbilityKind::Pavise && unit.pavise.is_none() {
                Some("pavois levés")
            } else {
                ability
                    .conditions
                    .iter()
                    .find_map(|&c| condition_fails(unit, c))
            };
            if let Some(reason) = reason {
                self.end_ability(i, Some(reason));
                continue;
            }
            if ability.duration > 0.0 && now - active.since + 1e-9 >= ability.duration {
                self.end_ability(i, None);
                continue;
            }
            let effective = now - active.since + 1e-9 >= ability.setup_time;
            let effects = &ability.effects;
            let unit = &mut self.units_mut()[i];
            if effects.speed_factor < 1.0 {
                unit.running = false;
            }
            if !effective {
                continue;
            }
            if effects.morale_recovery > 0.0
                && unit.state != UnitState::Melee
                && unit.morale < unit.morale_cap
            {
                unit.morale = (unit.morale + effects.morale_recovery * DT).min(unit.morale_cap);
            }
            if effects.melee_fatigue_factor != 1.0 && unit.state == UnitState::Melee {
                // Extra melee fatigue on top of `MoraleRules::next_fatigue`.
                unit.fatigue = (unit.fatigue
                    + melee_rate * (effects.melee_fatigue_factor - 1.0) * DT)
                    .clamp(0.0, 100.0);
            }
        }
    }
}
