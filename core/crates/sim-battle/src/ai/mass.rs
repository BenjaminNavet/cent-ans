//! IA-CAV: the horse lines up out of range, then charges together.
//!
//! Stateless like the rest of the AI: the plan is derived from the battle
//! state at each decision step. Squadrons (free regiments of horse) pick the
//! same target cluster of unprotected shooters, line up abreast beyond the
//! shooters' effective range (at the walk), and all gallop once every one of
//! them is in place, or as soon as one of them is hit (waiting under the
//! arrows would only bleed the line), or one has already launched.

use super::*;
use crate::mass_charge::MassChargeRules;

/// The coordinated charge of one side's horse for this decision step.
pub(super) struct Mass {
    /// Enemy shooter regiment at the centre of the target cluster.
    pub(super) target: usize,
    /// Shooters of the cluster (including the target).
    pub(super) cluster: Vec<usize>,
    /// Squadrons taking part and their place in the line.
    pub(super) slots: Vec<(usize, f64, f64)>,
    /// Every squadron is ready (or one is under fire): ride.
    pub(super) go: bool,
}

impl Mass {
    pub(super) fn slot(&self, i: usize) -> Option<(f64, f64)> {
        self.slots
            .iter()
            .find(|s| s.0 == i)
            .map(|&(_, x, z)| (x, z))
    }
}

/// Isolated shooter: no enemy foot or horse near it.
fn isolated(view: &View, j: usize, enemy_melee: &[usize]) -> bool {
    let units = view.units;
    let e = &units[j];
    is_shooter(e)
        && e.state != UnitState::Melee
        && enemy_melee
            .iter()
            .all(|&m| dist(&units[m], e) > tuning().isolation_distance)
}

/// Behind stakes facing `from`: a charge from there would break.
fn staked(e: &Unit, from: (f64, f64)) -> bool {
    e.stakes_planted && attack_angle(e, from.0, from.1) == 0
}

pub(super) fn plan_mass(
    view: &View,
    roles: &Roles,
    facing: f64,
    defensive: bool,
    enemy_melee: &[usize],
) -> Option<Mass> {
    let rules = MassChargeRules::bundled();
    let units = view.units;
    let group: Vec<usize> = roles
        .horse
        .iter()
        .copied()
        .filter(|&i| view.free(i))
        .filter(|&i| {
            let u = &units[i];
            !(u.can_shoot() && u.ammo > 0) && !(u.is_general && roles.horse.len() > 1)
        })
        .collect();
    // A side holding its ground keeps its horse back, until the arrows reach it.
    let hit = group
        .iter()
        .any(|&i| units[i].missile_timer < tuning().under_fire);
    if defensive && !hit {
        return None;
    }
    let centre = view.centroid(&group)?;
    let reach = |j: usize| dist_to(&units[j], centre.0, centre.1);
    let candidates: Vec<usize> = view
        .able_enemies()
        .filter(|&j| reach(j) < rules.reach_m)
        .filter(|&j| isolated(view, j, enemy_melee))
        .collect();
    let first = *candidates
        .iter()
        .min_by(|&&a, &&b| reach(a).total_cmp(&reach(b)).then(a.cmp(&b)))?;
    let cluster: Vec<usize> = candidates
        .iter()
        .copied()
        .filter(|&j| dist(&units[j], &units[first]) < rules.cluster_m)
        .collect();
    // Stakes in front: ride round to the flank of the end of the line.
    let flank = cluster.iter().any(|&j| staked(&units[j], centre));
    let (target, side) = if flank {
        let (rx, rz) = units[first].right();
        let lateral = |j: usize| units[j].x * rx + units[j].z * rz;
        let mean = cluster.iter().map(|&j| lateral(j)).sum::<f64>() / cluster.len() as f64;
        let side = if centre.0 * rx + centre.1 * rz >= mean {
            1.0
        } else {
            -1.0
        };
        let end = *cluster
            .iter()
            .max_by(|&&a, &&b| (side * lateral(a)).total_cmp(&(side * lateral(b))).then(b.cmp(&a)))
            .expect("non-empty");
        (end, side)
    } else {
        (first, 0.0)
    };
    let t = &units[target];
    // Not worth riding at shooters the group cannot beat.
    let own: f64 = group.iter().map(|&i| unit_power(&units[i])).sum();
    let theirs: f64 = cluster.iter().map(|&j| unit_power(&units[j])).sum();
    if own < theirs * rules.min_power_ratio {
        return None;
    }
    // The line forms beyond the longest reach of the cluster's shooters.
    let range = cluster
        .iter()
        .map(|&j| view.sim.effective_range(&units[j], centre.0, centre.1))
        .fold(0.0, f64::max)
        + rules.stage_margin_m;
    let stage = if flank {
        // 75 degrees off the front of the line: its flank, past the stakes.
        let (fx, fz) = t.forward();
        let (rx, rz) = t.right();
        let (cos, sin) = (0.26, 0.97);
        (
            t.x + range * (fx * cos + side * rx * sin),
            t.z + range * (fz * cos + side * rz * sin),
        )
    } else {
        let (dx, dz) = (centre.0 - t.x, centre.1 - t.z);
        let d = dx.hypot(dz).max(1e-6);
        (t.x + dx / d * range, t.z + dz / d * range)
    };
    let slots = view.line_slots(&group, stage, facing);
    let ready = |&(i, x, z): &(usize, f64, f64)| {
        let u = &units[i];
        let off = dist_to(u, x, z);
        // Already launched, or already inside the cluster's reach.
        u.target.is_some_and(|k| cluster.iter().any(|&j| units[j].id == k))
            && (u.running || u.state == UnitState::Charging)
            || off < rules.ready_m
            || dist(u, t) < range - rules.stage_margin_m * 0.5
            || (u.still_time > rules.stalled_s && off < rules.ready_m * 3.0)
    };
    let go = slots.iter().all(ready) || hit;
    Some(Mass {
        target,
        cluster,
        slots,
        go,
    })
}

/// One squadron's part in the coordinated charge. `true` when it gave its
/// orders.
pub(super) fn ride(view: &mut View, i: usize, mass: &Mass) -> bool {
    let Some((sx, sz)) = mass.slot(i) else {
        return false;
    };
    if !mass.go {
        let far = {
            let u = &view.units[i];
            dist_to(u, sx, sz) > 120.0
        };
        // The walk keeps the line; a laggard trots up.
        view.move_to(i, sx, sz, far, None);
        return true;
    }
    // Gallop at the nearest shooter of the cluster (spreads the squadrons).
    let unit = &view.units[i];
    let j = mass
        .cluster
        .iter()
        .copied()
        .filter(|&j| !staked(&view.units[j], (unit.x, unit.z)))
        .min_by(|&a, &b| {
            dist(unit, &view.units[a])
                .total_cmp(&dist(unit, &view.units[b]))
                .then(a.cmp(&b))
        })
        .unwrap_or(mass.target);
    charge_or_detour(view, i, j, true)
}
