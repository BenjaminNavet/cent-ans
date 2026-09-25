//! Naval combat formulas shared by the real-time battle ([`super::NavalSim`])
//! and the auto-resolve ([`super::auto_resolve`]): volleys between ships,
//! boarding melee, fire. Pure functions over [`Ship`]s and [`NavalRules`].

use data_model::NavalRules;
use serde::Serialize;

use super::ship::{Crew, Ship};
use crate::shot::MissileKind;

/// Wind over the battle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Wind {
    /// Direction the wind blows towards, radians (x-z plane).
    pub to: f64,
    /// 0-1.
    pub strength: f64,
}

impl Wind {
    pub fn vector(self) -> (f64, f64) {
        (self.to.cos(), self.to.sin())
    }

    /// +1 when `to` lies straight downwind of `from`, -1 straight upwind.
    pub fn alignment(self, from: (f64, f64), to: (f64, f64)) -> f64 {
        let (dx, dz) = (to.0 - from.0, to.1 - from.1);
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1e-6 {
            return 0.0;
        }
        let (wx, wz) = self.vector();
        (dx * wx + dz * wz) / len
    }

    /// Speed factor of a square-rigged ship on `heading`: 0 in the wind's
    /// eye (within `close_hauled_deg`), best on a broad reach.
    pub fn sail_factor(self, heading: f64, rules: &NavalRules) -> f64 {
        let (wx, wz) = self.vector();
        let cos = heading.cos() * wx + heading.sin() * wz;
        // 0° = running before the wind, 180° = head to wind.
        let off = cos.clamp(-1.0, 1.0).acos().to_degrees();
        let limit = 180.0 - rules.close_hauled_deg;
        let polar = if off <= 90.0 {
            0.85 + 0.15 * off / 90.0
        } else if off <= limit {
            1.0 - 0.55 * (off - 90.0) / (limit - 90.0).max(1.0)
        } else {
            0.0
        };
        polar * (rules.wind_speed_min + (1.0 - rules.wind_speed_min) * self.strength)
    }

    /// The heading nearest `desired` a sail ship can hold (tacking when the
    /// course lies in the wind's eye).
    pub fn best_heading(self, desired: f64, rules: &NavalRules) -> f64 {
        let eye = self.to + std::f64::consts::PI;
        let off = wrap(desired - eye);
        let limit = rules.close_hauled_deg.to_radians();
        if off.abs() >= limit {
            desired
        } else if off >= 0.0 {
            eye + limit
        } else {
            eye - limit
        }
    }
}

/// Angle wrapped to `(-π, π]`.
pub fn wrap(angle: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let mut a = angle % tau;
    if a > std::f64::consts::PI {
        a -= tau;
    } else if a <= -std::f64::consts::PI {
        a += tau;
    }
    a
}

/// Accuracy factor of shooting from `height` onto a deck at `target` height.
pub fn height_factor(height: f64, target: f64, rules: &NavalRules) -> f64 {
    1.0 + ((height - target) * rules.height_per_m).clamp(-rules.height_cap, rules.height_cap)
}

/// Fighting factor of men who must climb `climb` metres to the enemy deck
/// (negative: they jump down).
pub fn climb_factor(climb: f64, rules: &NavalRules) -> f64 {
    1.0 - (climb * rules.climb_per_m).clamp(-rules.climb_cap, rules.climb_cap)
}

/// Result of one volley of one crew group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volley {
    pub missiles: f64,
    /// Men to kill before the target's armour (see [`Ship::take_losses`]).
    pub hits: f64,
    /// Fire added to the target.
    pub fire: f64,
    pub kind: MissileKind,
}

/// Where a crew group shoots from: the castles hold `castle_capacity`
/// shooters (spread over the groups in order), the rest shoot from the deck.
pub fn castle_shooters(ship: &Ship, group: usize) -> f64 {
    let capacity = f64::from(ship.class.castle_capacity);
    let before: f64 = ship.crew[..group]
        .iter()
        .filter(|c| c.shoots())
        .map(|c| c.men)
        .sum();
    (capacity - before).clamp(0.0, ship.crew[group].men)
}

/// One volley of `shooter.crew[group]` at `target`, `distance` metres away.
/// `alignment` is the wind alignment from shooter to target (+1: the
/// shooter is straight upwind).
#[allow(clippy::too_many_arguments)]
pub fn volley(
    shooter: &Ship,
    group: usize,
    target: &Ship,
    distance: f64,
    wind: Wind,
    alignment: f64,
    rain: bool,
    rules: &NavalRules,
) -> Volley {
    let crew: &Crew = &shooter.crew[group];
    let gauge = alignment * wind.strength * rules.wind_gauge;
    let range = crew.range * (1.0 + gauge);
    let empty = Volley {
        missiles: 0.0,
        hits: 0.0,
        fire: 0.0,
        kind: crew.missile,
    };
    if !crew.shoots() || distance > range || !target.is_afloat() {
        return empty;
    }
    let castle = castle_shooters(shooter, group);
    let deck = crew.men - castle;
    let target_deck = target.deck_height();
    let height = (castle * height_factor(shooter.castle_height(), target_deck, rules)
        + deck * height_factor(shooter.deck_height(), target_deck, rules))
        / crew.men.max(1.0);
    let falloff = 1.0 - 0.6 * (distance / range).powi(2);
    let weather = if rain && crew.rain_penalty { 0.6 } else { 1.0 };
    // Men sheltering in the target's castles take less of the fire.
    let men = target.fighting_men().max(1.0);
    let sheltered = (f64::from(target.class.castle_capacity) / men).min(1.0) * 0.5;
    let exposure = 1.0 - sheltered * (1.0 - rules.castle_cover);
    let hits = crew.men
        * crew.ranged
        * 0.01
        * rules.ranged_lethality
        * height
        * falloff
        * (1.0 + gauge)
        * weather
        * exposure
        * (1.0 - target.class.bulwark);
    let fire = if shooter.fire_arrows {
        crew.men
            * rules.fire_arrow_share
            * rules.fire_per_missile
            * (1.0 - target.class.fire_resistance)
    } else {
        0.0
    };
    Volley {
        missiles: crew.men,
        hits: hits.max(0.0),
        fire,
        kind: crew.missile,
    }
}

/// Seconds between two volleys of a crew group.
pub fn reload_seconds(crew: &Crew, rules: &NavalRules) -> f64 {
    match crew.missile {
        MissileKind::Bolt => rules.crossbow_reload_s,
        _ => rules.bow_reload_s,
    }
}

/// Men each ship of a grappled pair kills in `dt` seconds of boarding.
/// `share_a` / `share_b` split a ship's power between its enemies,
/// `support_a` / `support_b` are the reinforcements over the chains.
#[allow(clippy::too_many_arguments)]
pub fn melee_exchange(
    a: &Ship,
    b: &Ship,
    share_a: f64,
    share_b: f64,
    support_a: f64,
    support_b: f64,
    dt: f64,
    rules: &NavalRules,
) -> (f64, f64) {
    let climb_a = b.deck_height() - a.deck_height();
    let castle_a = if a.class.has_castles() {
        1.0 + rules.castle_defense
    } else {
        1.0
    };
    let castle_b = if b.class.has_castles() {
        1.0 + rules.castle_defense
    } else {
        1.0
    };
    let power_a = a.melee_power() * share_a * (1.0 + support_a) * castle_a;
    let power_b = b.melee_power() * share_b * (1.0 + support_b) * castle_b;
    // Each side fights partly on its own deck, partly on the other's: the
    // climb counts for the attack on the higher deck.
    let factor_a = climb_factor(climb_a, rules);
    let factor_b = climb_factor(-climb_a, rules);
    let on_b = rules.melee_lethality * power_a * factor_a * dt;
    let on_a = rules.melee_lethality * power_b * factor_b * dt;
    (on_b, on_a)
}

/// One step of a fire aboard `ship`: growth with the wind, the sailors
/// fighting it, hull and crew burnt. Returns the men killed.
pub fn burn(ship: &mut Ship, wind: Wind, dt: f64, rules: &NavalRules) -> f64 {
    if ship.fire <= 0.0 {
        return 0.0;
    }
    let sailors = if ship.sailors_initial > 0.0 {
        (ship.sailors / ship.sailors_initial).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let fighting = if ship.is_afloat() {
        rules.fire_fighting * sailors
    } else {
        0.0
    };
    let growth = ship.fire * rules.fire_growth * (0.5 + wind.strength);
    ship.fire = (ship.fire + (growth - fighting) * dt).clamp(0.0, 1.0);
    if ship.fire < 0.01 {
        ship.fire = 0.0;
        return 0.0;
    }
    ship.hull -= ship.fire * rules.fire_hull * (1.0 - ship.class.fire_resistance) * dt;
    ship.morale = (ship.morale - ship.fire * 3.0 * dt).max(0.0);
    ship.take_share(ship.fire * rules.fire_crew * dt)
}
