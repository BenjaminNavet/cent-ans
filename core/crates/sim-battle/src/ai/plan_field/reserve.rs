//! The reserve regiment's plan.

use super::super::*;

pub(in crate::ai) fn plan_reserve(
    view: &mut View,
    r: usize,
    line: &[usize],
    anchor: (f64, f64),
    facing: f64,
) {
    if !view.free(r) {
        return;
    }
    // Plug a gap: a line regiment routed, destroyed or wavering in melee.
    let units = view.units;
    let gap = view
        .sim
        .units()
        .iter()
        .enumerate()
        .filter(|(_, u)| u.side == view.side && u.category == UnitCategory::Infantry && !u.mounted)
        .filter(|(k, _)| *k != r)
        .find(|(k, u)| {
            (u.state == UnitState::Routing && u.present())
                || (line.contains(k) && u.state == UnitState::Melee && u.morale < 35.0)
        })
        .map(|(k, _)| k);
    if let Some(k) = gap {
        let (gx, gz) = (units[k].x, units[k].z);
        let enemy = view
            .able_enemies()
            .filter(|&j| view.reachable(r, j))
            .min_by(|&a, &b| {
                dist_to(&units[a], gx, gz)
                    .total_cmp(&dist_to(&units[b], gx, gz))
                    .then(a.cmp(&b))
            });
        if let Some(j) = enemy {
            if dist_to(&units[j], gx, gz) < 120.0 {
                view.attack(r, j, true);
                return;
            }
        }
    }
    // A flank attack on a friend: strike the attacker.
    if let Some(j) = flanker(view) {
        if dist(&units[r], &units[j]) < 250.0 {
            view.attack(r, j, true);
            return;
        }
    }
    if let Some((j, d)) = view.nearest_enemy(r, |e| e.state != UnitState::Routing) {
        if d < tuning().counter_charge_distance {
            view.attack(r, j, true);
            return;
        }
    }
    view.move_to(
        r,
        anchor.0,
        anchor.1 - view.forward * 60.0,
        false,
        Some(facing),
    );
}
