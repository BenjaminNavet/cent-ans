//! Water, bridges and roads in the battle (lot EP3, [`crate::hydro`]).
//!
//! - Deep water stops horsemen and engines (outside a rout or a withdrawal):
//!   they go round by a bridge or a ford ([`BattleSim::water_route`]).
//! - Foot soldiers may swim across when it is much shorter than the way
//!   round, at a heavy price: very slow, tiring, shaken, and some drown
//!   (more in armour).
//! - A regiment wider than a bridge files across it slowly and fights badly
//!   from it; whoever holds the bridgehead strikes harder at the enemy on
//!   the deck. Blows struck from a ford or a stream are weaker.
//! - Marching in column on a road is faster.
//!
//! Deterministic: no random draw (drowning is a share of the regiment).

use data_model::UnitCategory;

use super::{BattleSim, DT};
use crate::hydro::{Crossing, Water, WaterRules};
use crate::impact::LossCause;
use crate::unit::{Formation, Unit};

/// A regiment this close to the near end of a crossing heads for its far end.
const AT_CROSSING: f64 = 4.0;
/// A swim is only worth it when the way round is longer than this many
/// metres more (foot).
const SWIM_PENALTY: f64 = 350.0;

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

impl BattleSim {
    /// EP3: crossings of the main river (bridges and fords), read once.
    pub(crate) fn crossings(&self) -> &[Crossing] {
        self.crossings.get_or_init(|| self.field.crossings())
    }

    /// Does deep water stop this regiment (horsemen, engines)?
    pub(crate) fn stopped_by_deep_water(unit: &Unit) -> bool {
        let m = &WaterRules::bundled().movement;
        (unit.mounted && m.deep_blocks_mounted)
            || (unit.category == UnitCategory::Siege && m.deep_blocks_engines)
    }

    /// A step from `from` to `to` that would take horsemen or an engine from
    /// the bank (or a bridge) into deep water.
    pub(super) fn water_blocks(&self, index: usize, from: (f64, f64), to: (f64, f64)) -> bool {
        let unit = &self.units[index];
        if self.field.river.is_none() || !Self::stopped_by_deep_water(unit) {
            return false;
        }
        let deep = |p: (f64, f64)| self.field.water_kind(p.0, p.1).is_some_and(Water::deep);
        deep(to) && !deep(from)
    }

    /// Where the straight path from `from` to `to` meets the river's centre
    /// line (the two points on either side), by bisection.
    fn river_meeting(&self, from: (f64, f64), to: (f64, f64)) -> Option<(f64, f64)> {
        let river = self.field.river.as_ref()?;
        let side = |p: (f64, f64)| p.1 - river.center_z(p.0);
        let (mut a, mut b) = (0.0, 1.0);
        let at = |t: f64| (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        let sa = side(from);
        if sa * side(to) > 0.0 {
            return None;
        }
        for _ in 0..24 {
            let m = (a + b) * 0.5;
            if side(at(m)) * sa > 0.0 {
                a = m;
            } else {
                b = m;
            }
        }
        Some(at((a + b) * 0.5))
    }

    /// EP3: waypoint towards (tx, tz) across the river: straight when the
    /// path crosses by a ford or a bridge (or does not cross), else by the
    /// best crossing; foot soldiers swim only when the way round is much
    /// longer. A regiment on a bridge or in a ford first reaches the far
    /// bank.
    pub(super) fn water_route(&self, index: usize, tx: f64, tz: f64) -> (f64, f64) {
        let Some(river) = &self.field.river else {
            return (tx, tz);
        };
        let unit = &self.units[index];
        let from = (unit.x, unit.z);
        let here = self.field.water_kind(from.0, from.1);
        if here.is_some_and(Water::deep) {
            // Already swimming (or fleeing across): carry on.
            return (tx, tz);
        }
        let north_to = river.north_of(tx, tz);
        // A target in the river or on a bridge is met where it stands.
        if river.in_water(tx, tz) || self.field.bridge_at(tx, tz).is_some() {
            return (tx, tz);
        }
        // On a bridge of the river: to the end on the target's side first.
        if let Some(b) = self.field.bridge_at(from.0, from.1) {
            if b.stream.is_none() {
                let [a, e] = b.ends();
                let end = if (e.1 > a.1) == north_to { e } else { a };
                if dist(from, end) > 1.0 && dist(from, (tx, tz)) > 1.0 {
                    return end;
                }
            }
        }
        // In a ford: across to the far bank at the same x first.
        if here == Some(Water::Ford) {
            let c = river.center_z(from.0);
            let half = river.width_at(from.0) * 0.5 + 3.0;
            let bank = (from.0, if north_to { c + half } else { c - half });
            if river.north_of(from.0, from.1) != north_to || (from.1 - c).abs() < half - 3.0 {
                return bank;
            }
        }
        if river.north_of(from.0, from.1) == north_to {
            return (tx, tz);
        }
        let Some(meet) = self.river_meeting(from, (tx, tz)) else {
            return (tx, tz);
        };
        let north_from = !north_to;
        let to = (tx, tz);
        let best = self
            .crossings()
            .iter()
            .map(|c| {
                let (near, far) = crossing_ends(river, c, meet, north_from);
                let cost = dist(from, near) + dist(near, far) + dist(far, to);
                (near, far, cost)
            })
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let Some((near, far, cost)) = best else {
            return (tx, tz);
        };
        if !Self::stopped_by_deep_water(unit) && dist(from, to) + SWIM_PENALTY < cost {
            return (tx, tz);
        }
        if dist(from, near) > AT_CROSSING {
            near
        } else {
            far
        }
    }

    /// EP3: melee multiplier of `attacker`'s blows at `defender` from water
    /// and bridges (weak from a ford, a stream or deep water, squeezed on a
    /// bridge; strong for the holder of the bridgehead).
    pub(super) fn water_melee_factor(&self, attacker: &Unit, defender: &Unit) -> f64 {
        if self.field.river.is_none() && self.field.streams.is_empty() {
            return 1.0;
        }
        let rules = WaterRules::bundled();
        let c = &rules.combat;
        let mut k = match self.field.water_kind(attacker.x, attacker.z) {
            Some(Water::Deep) => c.deep_attacker,
            Some(Water::Ford) | Some(Water::Stream(_)) | Some(Water::Oxbow) => c.ford_attacker,
            Some(Water::Pool) | None => 1.0,
        };
        let on_bridge = self.field.bridge_at(attacker.x, attacker.z);
        if let Some(b) = on_bridge {
            k *= (b.width / attacker.extent().0.max(1.0))
                .clamp(rules.movement.bridge_min_squeeze, 1.0);
        }
        if on_bridge.is_none() {
            if let Some(b) = self.field.bridge_at(defender.x, defender.z) {
                let holds = b
                    .ends()
                    .iter()
                    .any(|&e| dist(e, (attacker.x, attacker.z)) <= c.bridge_head_reach_m);
                if holds {
                    k *= c.bridge_holder_bonus;
                }
            }
        }
        k
    }

    /// EP3: speed multiplier of water, banks, bridges and roads.
    pub(super) fn water_speed(&self, unit: &Unit) -> f64 {
        let m = &WaterRules::bundled().movement;
        let mut k = self.field.water_speed_factor(unit.x, unit.z, unit.mounted);
        if let Some(b) = self.field.bridge_at(unit.x, unit.z) {
            k *= (b.width / unit.extent().0.max(1.0)).clamp(m.bridge_min_squeeze, 1.0);
        }
        if self.field.road_at(unit.x, unit.z).is_some() {
            k *= if unit.formation == Formation::Column {
                m.road_column
            } else {
                m.road_other
            };
        }
        k
    }

    /// EP3: why a charge from `from` at a regiment at `to` breaks on the
    /// water or a steep bank, if it does.
    pub(super) fn water_breaks_charge(
        &self,
        from: (f64, f64),
        to: (f64, f64),
    ) -> Option<&'static str> {
        let f = &self.field;
        if f.river.is_none() && f.streams.is_empty() {
            return None;
        }
        let wet = |p: (f64, f64)| {
            matches!(
                f.water_kind(p.0, p.1),
                Some(Water::Deep | Water::Ford | Water::Stream(_) | Water::Oxbow)
            )
        };
        if wet(from) || wet(to) || f.bridge_at(from.0, from.1).is_some() {
            return Some("s'enlise dans l'eau");
        }
        let steep = |p: (f64, f64)| f.bank_kind(p.0, p.1) == Some(crate::hydro::BankKind::Steep);
        if steep(to) {
            return Some("se brise sur la berge");
        }
        None
    }

    /// EP3: regiments in deep water tire, lose heart and drown (a share of
    /// the men each second, more in armour). Called after the movement.
    pub(super) fn resolve_water(&mut self) {
        if self.field.river.is_none() {
            return;
        }
        let rules = &WaterRules::bundled().deep_water;
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.present() || u.synthetic || u.on_wall {
                continue;
            }
            if !self.field.water_kind(u.x, u.z).is_some_and(Water::deep) {
                continue;
            }
            let armour = f64::from(u.stats.armor).clamp(0.0, 100.0) / 100.0;
            let loss =
                u.hp.max(0.0) * rules.drown_share_per_s * (1.0 + rules.armour_weight * armour) * DT;
            let id = u.id;
            let unit = &mut self.units[i];
            unit.hp -= loss;
            unit.tick_losses += loss;
            unit.loss_cause = LossCause::Drowned;
            unit.loss_by = None;
            unit.fatigue = (unit.fatigue + rules.fatigue_per_s * DT).clamp(0.0, 100.0);
            unit.morale -= rules.morale_per_s * DT;
            if !self.drown_announced.contains(&id) {
                self.drown_announced.push(id);
                let text = format!(
                    "Les {} se jettent à l'eau : des hommes se noient.",
                    self.unit_label(i)
                );
                let side = self.units[i].side;
                self.log(text, Some(side));
            }
        }
    }
}

/// Near and far ends of crossing `c` for a regiment coming from the north
/// (`north_from`) or the south; fords are crossed at the x of the path's
/// meeting with the river, clamped into the ford.
fn crossing_ends(
    river: &crate::field::River,
    c: &Crossing,
    meet: (f64, f64),
    north_from: bool,
) -> ((f64, f64), (f64, f64)) {
    let (near, far) = if north_from {
        (c.north, c.south)
    } else {
        (c.south, c.north)
    };
    let Some(f) = c.ford.map(|i| river.fords[i]) else {
        return (near, far);
    };
    let x = meet
        .0
        .clamp(f.x - f.half_width * 0.6, f.x + f.half_width * 0.6);
    let cz = river.center_z(x);
    let half = river.width_at(x) * 0.5 + 4.0;
    let (n, s) = ((x, cz + half), (x, cz - half));
    if north_from {
        (n, s)
    } else {
        (s, n)
    }
}
