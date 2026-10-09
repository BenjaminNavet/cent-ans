//! Battle measures read by the field planner: rescue, shooter share, height edge, losses, advance steps.

use super::super::*;

/// R4: the nearest enemy in a melee with one of `shooters`, within
/// `BattleAiRules::rescue_distance` of line regiment `i`.
pub(in crate::ai) fn rescue(view: &View, i: usize, shooters: &[usize]) -> Option<usize> {
    let u = &view.units[i];
    view.able_enemies()
        .filter(|&j| view.units[j].state == UnitState::Melee)
        .filter(|&j| {
            shooters.iter().any(|&s| {
                view.units[s].state == UnitState::Melee
                    && dist(&view.units[s], &view.units[j]) < 30.0
            })
        })
        .map(|j| (j, dist(u, &view.units[j])))
        .filter(|&(_, d)| d < tuning().rescue_distance)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(j, _)| j)
}

/// Share of the able enemy's power in its shooters.
pub(in crate::ai) fn enemy_shooter_share(view: &View) -> f64 {
    let (shooters, all) = view
        .able_enemies()
        .filter(|&j| !view.units[j].synthetic)
        .fold((0.0, 0.0), |(s, a), j| {
            let p = unit_power(&view.units[j]);
            (s + if is_shooter(&view.units[j]) { p } else { 0.0 }, a + p)
        });
    if all > 0.0 {
        shooters / all
    } else {
        0.0
    }
}

/// R2b: mean ground under the own regiments minus that under the enemy's.
pub(in crate::ai) fn height_edge(view: &View) -> f64 {
    let field = view.sim.field();
    let mean = |list: &mut dyn Iterator<Item = usize>| {
        let (sum, n) = list
            .filter(|&i| !view.units[i].synthetic)
            .fold((0.0, 0.0), |(s, n), i| {
                let u = &view.units[i];
                (s + field.height(u.x, u.z), n + 1.0)
            });
        if n > 0.0 {
            sum / n
        } else {
            0.0
        }
    };
    mean(&mut view.own.iter().copied()) - mean(&mut view.able_enemies())
}

/// Share of its initial soldiers a side has lost (B4, archery duel).
pub(in crate::ai) fn side_losses(units: &[Unit], side: SideId) -> f64 {
    let (hp, initial) = units
        .iter()
        .filter(|u| u.side == side && !u.synthetic)
        .fold((0.0, 0.0), |(hp, initial), u| {
            (hp + u.hp.max(0.0), initial + f64::from(u.initial_soldiers))
        });
    if initial > 0.0 {
        1.0 - hp / initial
    } else {
        0.0
    }
}

/// Next step of an advancing line: 45 m towards the enemy's centroid (F5d:
/// armies that slipped past each other turn back instead of marching on to
/// the far edge).
pub(in crate::ai) fn advance(view: &View, from: (f64, f64)) -> (f64, f64) {
    let able: Vec<usize> = view.able_enemies().collect();
    let Some((ex, ez)) = view.centroid(&able) else {
        return (from.0, from.1 + view.forward * 45.0);
    };
    let (dx, dz) = (ex - from.0, ez - from.1);
    let d = dx.hypot(dz);
    let straight = if dz * view.forward > 0.5 * d {
        // Ahead: march forward, leaning towards a defender offset
        // sideways (B8).
        let lean = dx.clamp(-tuning().advance_lean_max, tuning().advance_lean_max);
        (from.0 + lean, from.1 + view.forward * 45.0)
    } else {
        let step = d.min(45.0) / d.max(1e-6);
        (from.0 + dx * step, from.1 + dz * step)
    };
    relief_step(view, from, straight)
}

/// R2b: of the step `from` -> `to` and the same step shifted aside, the one
/// the relief makes cheapest (round a steep rise by a valley or a shelf
/// rather than straight up it); the straight step on ties.
pub(in crate::ai) fn relief_step(view: &View, from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let field = view.sim.field();
    let (dx, dz) = (to.0 - from.0, to.1 - from.1);
    let len = dx.hypot(dz).max(1e-6);
    let side = (dz / len, -dx / len);
    let cost = |p: (f64, f64)| {
        ReliefMap::march_cost(field, from, p)
            + 40.0 * (ReliefMap::climb(field, from, p) - tuning().steep_climb).max(0.0)
    };
    let mut best = (to, cost(to) - tuning().relief_saving);
    for lean in tuning().relief_leans {
        let p = (to.0 + side.0 * lean, to.1 + side.1 * lean);
        if !field.inside(p.0, p.1)
            || field.in_forest(p.0, p.1)
            || field.water_at(p.0, p.1).is_some()
        {
            continue;
        }
        let c = cost(p);
        if c < best.1 {
            best = (p, c);
        }
    }
    best.0
}

/// R2b: would `i` charge `j` up a steep rise from afar? It then walks up
/// and charges once close.
pub(in crate::ai) fn steep_charge(view: &View, i: usize, j: usize) -> bool {
    let (u, e) = (&view.units[i], &view.units[j]);
    dist(u, e) > tuning().close_charge
        && ReliefMap::climb(view.sim.field(), (u.x, u.z), (e.x, e.z)) > tuning().steep_climb
}

/// Enemy regiment opposite `i` (smallest lateral offset, a bit of depth).
/// R4: the strength of the ground each one holds counts too, so that the
/// line strikes the weak point (the open ground rather than the hedge or
/// the crest).
pub(in crate::ai) fn opposite(view: &View, i: usize) -> Option<usize> {
    let u = &view.units[i];
    let field = view.sim.field();
    let map = view.sim.relief_map();
    let base = field.height(u.x, u.z);
    view.able_enemies()
        .filter(|&j| !is_horse(&view.units[j]) || view.units[j].state == UnitState::Melee)
        .filter(|&j| view.reachable(i, j))
        .map(|j| {
            let e = &view.units[j];
            let front = Front {
                center: (e.x, e.z),
                forward: -view.forward,
                width: e.extent().0,
            };
            let strength = score_position(field, map, front, base, false).total();
            (
                j,
                (e.x - u.x).abs()
                    + 0.3 * (e.z - u.z).abs()
                    + tuning().weak_point * strength.max(0.0),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(j, _)| j)
}
