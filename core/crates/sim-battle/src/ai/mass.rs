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
    /// Each squadron's own target (the end of the line on its side).
    pub(super) ends: Vec<(usize, usize)>,
    /// Stakes in front: the charge goes for the flanks.
    pub(super) flank: bool,
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
    // An army with foot keeps its horse with it (EQ7: the horse waits for the
    // foot, the charge on isolated shooters is the old one-by-one ride).
    if !roles.line.is_empty() {
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
    // Stakes in front: each squadron rides round to the flank of the end of
    // the line on its own side (a pincer), never across the front.
    let flank = cluster.iter().any(|&j| staked(&units[j], centre));
    // Not worth riding at shooters the group cannot beat, unless the arrows
    // already fall on it (waiting would only bleed it).
    let own: f64 = group.iter().map(|&i| unit_power(&units[i])).sum();
    let theirs: f64 = cluster.iter().map(|&j| unit_power(&units[j])).sum();
    if own < theirs * rules.min_power_ratio && !hit {
        return None;
    }
    // The line forms beyond the longest reach of the cluster's shooters.
    let range = cluster
        .iter()
        .map(|&j| view.sim.effective_range(&units[j], centre.0, centre.1))
        .fold(0.0, f64::max)
        + rules.stage_margin_m;
    let (rx, rz) = units[first].right();
    let lateral = |x: f64, z: f64| x * rx + z * rz;
    let mean = cluster
        .iter()
        .map(|&j| lateral(units[j].x, units[j].z))
        .sum::<f64>()
        / cluster.len() as f64;
    let sides: &[f64] = if flank { &[-1.0, 1.0] } else { &[0.0] };
    let mut slots = Vec::new();
    let mut ends = Vec::new();
    for &side in sides {
        let members: Vec<usize> = group
            .iter()
            .copied()
            .filter(|&i| !flank || (lateral(units[i].x, units[i].z) >= mean) == (side > 0.0))
            .collect();
        let Some(mc) = view.centroid(&members) else {
            continue;
        };
        let target = if flank {
            *cluster
                .iter()
                .max_by(|&&a, &&b| {
                    (side * lateral(units[a].x, units[a].z))
                        .total_cmp(&(side * lateral(units[b].x, units[b].z)))
                        .then(b.cmp(&a))
                })
                .expect("non-empty")
        } else {
            first
        };
        let t = &units[target];
        let stage = if flank {
            // 75 degrees off the front of the line: its flank, past the stakes.
            let (fx, fz) = t.forward();
            let (tx, tz) = t.right();
            let (cos, sin) = (0.26, 0.97);
            (
                t.x + range * (fx * cos + side * tx * sin),
                t.z + range * (fz * cos + side * tz * sin),
            )
        } else {
            let (dx, dz) = (mc.0 - t.x, mc.1 - t.z);
            let d = dx.hypot(dz).max(1e-6);
            (t.x + dx / d * range, t.z + dz / d * range)
        };
        slots.extend(view.line_slots(&members, stage, facing));
        ends.extend(members.iter().map(|&i| (i, target)));
    }
    let ready = |&(i, x, z): &(usize, f64, f64)| {
        let u = &units[i];
        let off = dist_to(u, x, z);
        let t = ends.iter().find(|e| e.0 == i).map_or(first, |e| e.1);
        // Already launched, or already inside the cluster's reach.
        u.target
            .is_some_and(|k| cluster.iter().any(|&j| units[j].id == k))
            && (u.running || u.state == UnitState::Charging)
            || off < rules.ready_m
            || dist(u, &units[t]) < range - rules.stage_margin_m * 0.5
            || (u.still_time > rules.stalled_s && off < rules.ready_m * 3.0)
    };
    let go = slots.iter().all(ready) || hit;
    Some(Mass {
        target: first,
        cluster,
        slots,
        ends,
        flank,
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
    // Gallop at the end of the line on its side (stakes), or at the nearest
    // shooter of the cluster (spreads the squadrons).
    let unit = &view.units[i];
    let own_end = mass.ends.iter().find(|e| e.0 == i).map(|e| e.1);
    let j = if mass.flank {
        own_end.unwrap_or(mass.target)
    } else {
        mass.cluster
            .iter()
            .copied()
            .min_by(|&a, &b| {
                dist(unit, &view.units[a])
                    .total_cmp(&dist(unit, &view.units[b]))
                    .then(a.cmp(&b))
            })
            .unwrap_or(mass.target)
    };
    charge_or_detour(view, i, j, true)
}
