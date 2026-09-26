//! EP11 (ADR 0070): continuous push of the lines in melee. Rules in
//! `data/rules/battle_push.json` (schema
//! `data/schemas/battle_push_rules.schema.json`); the per-tick resolution is
//! in `sim/push.rs`. Field battles only: sieges keep their fixed melee.
//!
//! - **Pressure.** Each regiment in contact leans on its opponent with a
//!   pressure ([`PressureRules::pressure`]): weight per man (armour, horse)
//!   × √ranks (capped) × freshness × morale × charge impetus × formation.
//!   The relative gap `(Pa − Pb)/(Pa + Pb)` sets the recoil speed of the
//!   weaker ([`PressureRules::recoil_speed`]): nothing inside a dead band,
//!   then up to a few metres per ten seconds; the winner follows.
//! - **Compression.** A pushed regiment that cannot give ground (wall,
//!   house, deep water, field edge, a friendly regiment close behind)
//!   compresses ([`PushShape::compression`], 0-1): it strikes less, takes
//!   more and loses morale ([`PushRules::melee_factor`]).
//! - **Wrap.** The files of a regiment that overhang a narrower opponent
//!   swing round its corners ([`PushShape::wrap`]) and strike its flanks.
//! - **Bulge.** The front of the pusher bulges and the front of the pushed
//!   dents around the point of contact. Rendering only
//!   ([`deform_figures`]), like the squeezed ranks of a compressed regiment.

use std::sync::OnceLock;

use data_model::Ability;
use serde::{Deserialize, Serialize};

use crate::unit::{Formation, Unit};

/// Contents of `data/rules/battle_push.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub pressure: PressureRules,
    pub compression: CompressionRules,
    pub wrap: WrapRules,
    pub bulge: BulgeRules,
}

/// Pressure of a regiment and speed of the recoil.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PressureRules {
    /// Fastest recoil (m/s).
    pub max_speed_mps: f64,
    /// Relative pressure gap under which the lines stand.
    pub dead_band: f64,
    /// Relative pressure gap from which the recoil is at full speed.
    pub full_speed_share: f64,
    /// Weight of a rider (an unarmoured foot soldier weighs 1).
    pub mounted_weight: f64,
    /// Weight added per point of armour.
    pub armor_weight_per_point: f64,
    /// Ranks beyond this one add no push.
    pub rank_cap: u32,
    /// Lowest freshness factor.
    pub min_freshness: f64,
    /// Freshness = 1 − fatigue / `fatigue_scale`.
    pub fatigue_scale: f64,
    /// While the charge impetus lasts: × (1 + charge/100 × this).
    pub charge_factor: f64,
    /// Square / schiltron.
    pub square_factor: f64,
    /// Archers behind their planted stakes.
    pub stakes_factor: f64,
    /// Pikemen (schiltron ability) against riders.
    pub pikes_against_horse_factor: f64,
    /// Share of the opponent's recoil the pusher follows.
    pub follow_share: f64,
}

/// A pushed regiment that cannot give ground.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompressionRules {
    /// Compression gained per second at full push.
    pub gain_per_second: f64,
    /// Compression lost per second when free.
    pub decay_per_second: f64,
    /// A friend closer than this behind blocks the recoil (metres).
    pub friend_gap_m: f64,
    /// Melee losses taken × (1 + compression × this).
    pub damage_taken_bonus: f64,
    /// Melee blows dealt × (1 − compression × this).
    pub fighting_malus: f64,
    /// Morale lost per second at compression 1.
    pub morale_per_second: f64,
    /// Rendering: share of the depth lost by the ranks at compression 1.
    pub squeeze: f64,
}

/// Files overhanging a narrower opponent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WrapRules {
    /// Overhang on one side from which its files wrap (metres).
    pub min_overhang_m: f64,
    /// Wrap gained per second (0-1).
    pub grow_per_second: f64,
    /// Wrap lost per second once the overhang is gone.
    pub decay_per_second: f64,
    /// Pivot of the overhanging files at full wrap (degrees).
    pub max_turn_deg: f64,
    /// Blows dealt to the wrapped opponent × (1 + mean wrap × this).
    pub flank_damage_bonus: f64,
}

/// Rendering: shape of the front.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulgeRules {
    /// Deepest bulge or dent (metres).
    pub max_m: f64,
    /// Bulge per m/s of push.
    pub per_mps: f64,
    /// Bulge of any line in melee (the front ranks surge forward).
    pub melee_surge_m: f64,
    /// Speed at which the front takes its shape (m/s).
    pub rate_mps: f64,
    /// Share of the bulge the rear rank keeps.
    pub rear_share: f64,
}

/// EP11: shape and push state of a regiment (see the module doc). All zero
/// out of melee.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PushShape {
    /// Forward offset of the front at the point of contact (metres): > 0
    /// bulging (pushing), < 0 dented (giving ground). Rendering.
    pub bulge: f64,
    /// Lateral local coordinate of the point of contact (metres, + right).
    pub bulge_at: f64,
    /// 0-1: pushed against an obstacle it cannot give ground to.
    pub compression: f64,
    /// Wrap (0-1) of the overhanging files, `[left, right]`.
    pub wrap: [f64; 2],
    /// Lateral local coordinates where the wrapped opponent ends,
    /// `[left, right]` (the pivots of the wrapping files).
    pub wrap_edge: [f64; 2],
    /// Id of the regiment being wrapped.
    pub wrap_of: Option<u32>,
    /// Speed of the push this tick along the front (m/s): > 0 driving the
    /// opponent back, < 0 giving ground.
    pub speed: f64,
    /// Ground given since the start of the battle (metres).
    pub ground_lost: f64,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_push.json");

impl PushRules {
    /// `data/rules/battle_push.json` as compiled into the crate.
    pub fn bundled() -> &'static PushRules {
        static RULES: OnceLock<PushRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_push.json is valid")
        })
    }

    /// Melee blows of `attacker` on `defender` are multiplied by this:
    /// the attacker's compression hampers it, the defender's exposes it,
    /// wrapped files strike the defender's flanks.
    pub fn melee_factor(&self, attacker: &Unit, defender: &Unit) -> f64 {
        let (a, d) = (&attacker.push, &defender.push);
        let mut factor = (1.0 - a.compression * self.compression.fighting_malus)
            * (1.0 + d.compression * self.compression.damage_taken_bonus);
        if a.wrap_of == Some(defender.id) {
            factor *= 1.0 + (a.wrap[0] + a.wrap[1]) * 0.5 * self.wrap.flank_damage_bonus;
        }
        factor
    }
}

impl PressureRules {
    /// Push of `unit` against `opponent` (arbitrary units; only ratios count).
    pub fn pressure(&self, unit: &Unit, opponent: &Unit) -> f64 {
        let mut weight = 1.0 + f64::from(unit.stats.armor) * self.armor_weight_per_point;
        if unit.mounted {
            weight *= self.mounted_weight;
        }
        let (ranks, _) = unit.ranks_files(unit.soldiers());
        let depth = f64::from(ranks.min(self.rank_cap).max(1)).sqrt();
        let freshness = (1.0 - unit.fatigue / self.fatigue_scale).max(self.min_freshness);
        let morale = 0.5 + unit.morale.clamp(0.0, 100.0) / 200.0;
        let mut pressure = weight * depth * freshness * morale;
        if unit.charge_timer > 0.0 {
            let charge = f64::from(unit.stats.charge.unwrap_or(20));
            pressure *= 1.0 + charge / 100.0 * self.charge_factor;
        }
        if unit.formation == Formation::Square {
            pressure *= self.square_factor;
        }
        if unit.stakes_planted {
            pressure *= self.stakes_factor;
        }
        if opponent.mounted && unit.has(Ability::PikeSquare) {
            pressure *= self.pikes_against_horse_factor;
        }
        pressure
    }

    /// Recoil speed (m/s) of the weaker of two regiments pushing with
    /// `stronger` and `weaker` (≥ 0).
    pub fn recoil_speed(&self, stronger: f64, weaker: f64) -> f64 {
        let total = stronger + weaker;
        if total <= 0.0 {
            return 0.0;
        }
        let share = (stronger - weaker) / total;
        if share <= self.dead_band {
            return 0.0;
        }
        let t = ((share - self.dead_band) / (self.full_speed_share - self.dead_band).max(1e-6))
            .min(1.0);
        // Smoothstep: no jolt when a line tips over the dead band.
        self.max_speed_mps * t * t * (3.0 - 2.0 * t)
    }
}

/// Local `(lateral, forward)` coordinates of the world point in `unit`'s frame.
pub fn to_local(unit: &Unit, x: f64, z: f64) -> (f64, f64) {
    let (dx, dz) = (x - unit.x, z - unit.z);
    let (fx, fz) = unit.forward();
    let (rx, rz) = unit.right();
    (dx * rx + dz * rz, dx * fx + dz * fz)
}

/// World `(x, z)` of the local point of `unit`'s frame.
pub fn to_world(unit: &Unit, lx: f64, lz: f64) -> (f64, f64) {
    let (fx, fz) = unit.forward();
    let (rx, rz) = unit.right();
    (unit.x + rx * lx + fx * lz, unit.z + rz * lx + fz * lz)
}

/// Where `opponent` ends laterally in `unit`'s frame (`[left, right]`
/// local coordinates), when it stands in front of `unit` and overlaps its
/// front; `None` otherwise.
pub fn opponent_span(unit: &Unit, opponent: &Unit) -> Option<[f64; 2]> {
    let (_, ahead) = to_local(unit, opponent.x, opponent.z);
    if ahead <= 0.0 {
        return None;
    }
    let (w, d) = opponent.extent();
    let (fx, fz) = opponent.forward();
    let (rx, rz) = opponent.right();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for (sx, sz) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
        let cx = opponent.x + rx * w * 0.5 * sx + fx * d * 0.5 * sz;
        let cz = opponent.z + rz * w * 0.5 * sx + fz * d * 0.5 * sz;
        let (lx, _) = to_local(unit, cx, cz);
        lo = lo.min(lx);
        hi = hi.max(lx);
    }
    let (own_w, _) = unit.extent();
    let half = own_w * 0.5;
    (lo < half && hi > -half).then_some([lo, hi])
}

/// Rendering: bends the world `(x, z, angle)` figure positions of `unit`
/// (bulge or dent of the front, squeezed ranks, wrapping files). No-op out
/// of melee (all-zero shape).
pub fn deform_figures(unit: &Unit, rules: &PushRules, positions: &mut [(f64, f64, f64)]) {
    let shape = &unit.push;
    let bulge = shape.bulge;
    let squeeze = shape.compression * rules.compression.squeeze;
    let wraps = shape.wrap;
    if bulge.abs() < 1e-3 && squeeze < 1e-3 && wraps[0] < 1e-3 && wraps[1] < 1e-3 {
        return;
    }
    let (w, d) = unit.extent();
    let half_w = (w * 0.5).max(0.5);
    let front = d * 0.5;
    let max_turn = rules.wrap.max_turn_deg.to_radians();
    let rear_share = rules.bulge.rear_share;
    for pos in positions.iter_mut() {
        let (mut lx, mut lz) = to_local(unit, pos.0, pos.1);
        let mut angle = pos.2;
        if squeeze > 0.0 {
            lz = front - (front - lz) * (1.0 - squeeze);
        }
        if bulge.abs() >= 1e-3 {
            let u = (lx - shape.bulge_at) / half_w;
            let profile = (1.0 - u * u).max(0.0);
            let rank = ((front - lz) / d.max(0.5)).clamp(0.0, 1.0);
            lz += bulge * profile * (1.0 - (1.0 - rear_share) * rank);
        }
        for (s, sign) in [(0usize, -1.0), (1usize, 1.0)] {
            let wrap = wraps[s];
            if wrap < 1e-3 {
                continue;
            }
            let pivot = shape.wrap_edge[s];
            let ex = lx - pivot;
            if ex * sign <= 0.0 {
                continue;
            }
            let ez = lz - front;
            let theta = wrap * max_turn;
            let (sn, cs) = theta.sin_cos();
            // Right wing turns anticlockwise in (lateral, forward), left
            // wing clockwise: both swing forward round the opponent.
            let (rx, rz) = if sign > 0.0 {
                (ex * cs - ez * sn, ex * sn + ez * cs)
            } else {
                (ex * cs + ez * sn, -ex * sn + ez * cs)
            };
            lx = pivot + rx;
            lz = front + rz;
            angle -= sign * theta;
        }
        let (x, z) = to_world(unit, lx, lz);
        *pos = (x, z, angle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load() {
        let rules = PushRules::bundled();
        assert!(rules.pressure.max_speed_mps > 0.0);
        assert!(rules.pressure.dead_band < rules.pressure.full_speed_share);
    }

    #[test]
    fn recoil_speed_has_a_dead_band_and_a_ceiling() {
        let p = &PushRules::bundled().pressure;
        assert_eq!(p.recoil_speed(1.0, 1.0), 0.0);
        assert_eq!(p.recoil_speed(1.05, 1.0), 0.0);
        let mid = p.recoil_speed(1.6, 1.0);
        assert!(mid > 0.0 && mid < p.max_speed_mps, "{mid}");
        assert!((p.recoil_speed(10.0, 1.0) - p.max_speed_mps).abs() < 1e-9);
        assert!(p.recoil_speed(2.0, 1.0) > mid);
    }
}
