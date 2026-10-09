//! Shared view of the battle for one side and its command helpers.

use super::*;
use crate::geom::segment_distance;

pub(super) fn dist(a: &Unit, b: &Unit) -> f64 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

pub(super) fn dist_to(a: &Unit, x: f64, z: f64) -> f64 {
    ((a.x - x).powi(2) + (a.z - z).powi(2)).sqrt()
}

/// Rough fighting value of a regiment (for the posture decision).
pub fn unit_power(unit: &Unit) -> f64 {
    let s = &unit.stats;
    let quality = f64::from(s.melee)
        + f64::from(s.ranged) * 0.8
        + f64::from(s.armor) * 0.6
        + f64::from(s.charge.unwrap_or(0)) * 0.3;
    unit.hp.max(0.0) * quality / 100.0 * (0.5 + unit.morale / 200.0)
}

pub(super) fn is_shooter(unit: &Unit) -> bool {
    unit.can_shoot() && unit.ammo > 0 && !unit.mounted && unit.category != UnitCategory::Siege
}

pub(super) fn is_horse(unit: &Unit) -> bool {
    unit.mounted || unit.category == UnitCategory::Cavalry
}

pub(super) fn is_melee_troop(unit: &Unit) -> bool {
    matches!(
        unit.category,
        UnitCategory::Infantry | UnitCategory::Cavalry
    ) && !unit.can_shoot()
}

/// Dangerous to charge head on for cavalry.
pub(super) fn bristling(unit: &Unit) -> bool {
    unit.stakes_planted || unit.braced() || unit.has(Ability::PikeSquare)
}

/// R2b: would horsemen riding from `from` at `target` pass in front of
/// enemy stakes (another regiment's planted stakes within `BattleAiRules::stakes_guard`
/// of the ride, the horsemen coming from their front)? The ride ends on
/// the stakes: a routing regiment flees through its archers, a melee drifts.
pub(super) fn stakes_in_path(units: &[Unit], from: (f64, f64), target: &Unit) -> bool {
    stakes_on_ride(
        units,
        target.side,
        from,
        (target.x, target.z),
        Some(target.id),
    )
}

/// R2b: does a ride from `from` to `to` pass in front of planted stakes of
/// `foe` (`except` the regiment charged)?
pub(super) fn stakes_on_ride(
    units: &[Unit],
    foe: SideId,
    from: (f64, f64),
    to: (f64, f64),
    except: Option<u32>,
) -> bool {
    units.iter().any(|k| {
        Some(k.id) != except
            && k.side == foe
            && k.stakes_planted
            && k.able()
            && attack_angle(k, from.0, from.1) == 0
            && segment_distance((k.x, k.z), from, to) < tuning().stakes_guard
    })
}

/// Snapshot of the battle from one side's point of view.
pub(super) struct View<'a> {
    pub(super) sim: &'a BattleSim,
    pub(super) side: SideId,
    pub(super) units: &'a [Unit],
    pub(super) own: Vec<usize>,
    pub(super) enemies: Vec<usize>,
    /// +1 when the enemy lies towards +z (attacker), -1 otherwise.
    pub(super) forward: f64,
    /// F5d: a clearly stronger attacker closing in (no duel) with the enemy
    /// within `BattleAiRules::assault_range`: the horse goes for the enemy horse.
    pub(super) assault: bool,
    pub(super) commands: Vec<Command>,
}

impl<'a> View<'a> {
    pub(super) fn new(sim: &'a BattleSim, side: SideId) -> Self {
        let units = sim.units();
        let own = (0..units.len())
            .filter(|&i| units[i].side == side && units[i].able())
            .collect();
        let enemies = (0..units.len())
            .filter(|&j| units[j].side != side && units[j].present() && !units[j].synthetic)
            .filter(|&j| units[j].able() || units[j].state == UnitState::Routing)
            .collect();
        View {
            sim,
            side,
            units,
            own,
            enemies,
            forward: if side == SideId::Attacker { 1.0 } else { -1.0 },
            assault: false,
            commands: Vec::new(),
        }
    }

    pub(super) fn able_enemies(&self) -> impl Iterator<Item = usize> + '_ {
        self.enemies
            .iter()
            .copied()
            .filter(|&j| self.units[j].able())
    }

    pub(super) fn engaged(&self, i: usize) -> bool {
        self.units[i].state == UnitState::Melee
    }

    /// Nearest able enemy satisfying `keep`, with its distance.
    pub(super) fn nearest_enemy(
        &self,
        i: usize,
        keep: impl Fn(&Unit) -> bool,
    ) -> Option<(usize, f64)> {
        let unit = &self.units[i];
        self.able_enemies()
            .filter(|&j| keep(&self.units[j]))
            .map(|j| (j, dist(unit, &self.units[j])))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
    }

    /// Can `i` reach `j` without an intact wall in between (siege)?
    pub(super) fn reachable(&self, i: usize, j: usize) -> bool {
        let (a, b) = (&self.units[i], &self.units[j]);
        self.sim
            .siege()
            .is_none_or(|w| w.crosses_intact((a.x, a.z), (b.x, b.z)).is_none())
    }

    pub(super) fn power(&self, side_own: bool) -> f64 {
        let list: Vec<usize> = if side_own {
            self.own.clone()
        } else {
            self.able_enemies().collect()
        };
        list.iter()
            .filter(|&&i| !self.units[i].synthetic)
            .map(|&i| unit_power(&self.units[i]))
            .sum()
    }

    pub(super) fn centroid(&self, list: &[usize]) -> Option<(f64, f64)> {
        if list.is_empty() {
            return None;
        }
        let n = list.len() as f64;
        Some(list.iter().fold((0.0, 0.0), |(x, z), &i| {
            (x + self.units[i].x / n, z + self.units[i].z / n)
        }))
    }

    // ----- command helpers -----------------------------------------------

    pub(super) fn free(&self, i: usize) -> bool {
        let u = &self.units[i];
        !matches!(
            u.state,
            UnitState::Melee | UnitState::Rallied | UnitState::Climbing
        ) && u.climbing.is_none()
    }

    /// Moves `i` to (x, z) unless it is already going (or standing) there.
    pub(super) fn move_to(&mut self, i: usize, x: f64, z: f64, run: bool, facing: Option<f64>) {
        let u = &self.units[i];
        if !self.free(i) {
            return;
        }
        let field = self.sim.field();
        let (x, z) = field.clamp_inside(x, z, tuning().field_margin);
        let z = dry_z(field, x, z, u.z, self.forward);
        let far = match u.destination {
            Some((dx, dz)) => (dx - x).powi(2) + (dz - z).powi(2) > 36.0,
            None => dist_to(u, x, z) > 6.0,
        };
        if far || u.target.is_some() || (u.destination.is_some() && u.running != run) {
            self.commands.push(Command::Move {
                units: vec![u.id],
                x,
                z,
                run,
                facing,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
            });
        }
    }

    pub(super) fn attack(&mut self, i: usize, j: usize, run: bool) {
        let u = &self.units[i];
        if u.climbing.is_some() {
            return;
        }
        if u.target != Some(self.units[j].id) || (u.running != run && !self.engaged(i)) {
            self.commands.push(Command::Attack {
                units: vec![u.id],
                target: self.units[j].id,
                run,
                queue: false,
            });
        }
    }

    pub(super) fn halt(&mut self, i: usize) {
        let u = &self.units[i];
        if (u.destination.is_some() || u.target.is_some()) && self.free(i) {
            self.commands.push(Command::Halt { units: vec![u.id] });
        }
    }

    /// Slots along a line centred on `center`, perpendicular to `facing`,
    /// in lateral order of the regiments.
    pub(super) fn line_slots(
        &self,
        ids: &[usize],
        center: (f64, f64),
        facing: f64,
    ) -> Vec<(usize, f64, f64)> {
        let right = (facing.cos(), -facing.sin());
        let mut order: Vec<(usize, f64)> = ids
            .iter()
            .map(|&i| (i, self.units[i].x * right.0 + self.units[i].z * right.1))
            .collect();
        order.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        let widths: Vec<f64> = order
            .iter()
            .map(|&(i, _)| self.units[i].extent().0 + 10.0)
            .collect();
        let total: f64 = widths.iter().sum();
        let mut offset = -total * 0.5;
        order
            .iter()
            .zip(widths)
            .map(|(&(i, _), w)| {
                let lateral = offset + w * 0.5;
                offset += w;
                (
                    i,
                    center.0 + right.0 * lateral,
                    center.1 + right.1 * lateral,
                )
            })
            .collect()
    }
}

/// F5d: a destination in deep water (fords excepted) moves to the bank on
/// the side where the regiment stands, or across when it is already wading
/// (`forward` = +1 towards +z).
pub fn dry_z(field: &crate::field::Battlefield, x: f64, z: f64, from_z: f64, forward: f64) -> f64 {
    let Some(river) = &field.river else {
        return z;
    };
    let center = river.center_z(x);
    let reach = river.width_at(x) * 0.5 + tuning().river_margin;
    if river.in_ford(x) || (z - center).abs() > reach {
        return z;
    }
    let side = if (from_z - center).abs() <= reach {
        forward
    } else {
        (from_z - center).signum()
    };
    (center + side * (reach + 1.0)).clamp(10.0, field.depth - 10.0)
}
