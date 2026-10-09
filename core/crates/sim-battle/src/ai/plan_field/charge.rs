//! Charges and detours: does a charge break on the site, and how to ride round it.

use super::super::*;

/// B6: does a charge of `i` at `j` break on the site (a hedge or a ditch in
/// front of the target)?
pub(in crate::ai) fn charge_breaks(view: &View, i: usize, j: usize) -> bool {
    let (u, e) = (&view.units[i], &view.units[j]);
    let field = view.sim.field();
    field.breaks_charge((u.x, u.z), (e.x, e.z))
        || charge_breaks_on_water(field, e)
        || ReliefMap::river_between(field, (u.x, u.z), (e.x, e.z))
}

/// EP3: a target standing in the water, on a bridge or on a steep bank.
pub(in crate::ai) fn charge_breaks_on_water(field: &crate::field::Battlefield, e: &Unit) -> bool {
    matches!(
        field.water_kind(e.x, e.z),
        Some(
            crate::hydro::Water::Deep
                | crate::hydro::Water::Ford
                | crate::hydro::Water::Stream(_)
                | crate::hydro::Water::Oxbow
        )
    ) || field.bridge_at(e.x, e.z).is_some()
        || field.bank_kind(e.x, e.z) == Some(crate::hydro::BankKind::Steep)
}

/// The point beyond the nearer end of `blocking`, on the target's side, from
/// which `probe` can ride on towards `to` clear of that one obstacle.
pub(in crate::ai) fn detour_past(
    probe: (f64, f64),
    to: (f64, f64),
    blocking: &Obstacle,
) -> Option<(f64, f64)> {
    let len = blocking.length().max(1e-6);
    let side_of = |p: (f64, f64)| {
        (blocking.b.0 - blocking.a.0) * (p.1 - blocking.a.1)
            - (blocking.b.1 - blocking.a.1) * (p.0 - blocking.a.0)
    };
    // Normal pointing to the target's side of the obstacle.
    let mut normal = (
        -(blocking.b.1 - blocking.a.1) / len,
        (blocking.b.0 - blocking.a.0) / len,
    );
    if side_of(to) < 0.0 {
        normal = (-normal.0, -normal.1);
    }
    [(blocking.a, blocking.b), (blocking.b, blocking.a)]
        .into_iter()
        .map(|(end, other)| {
            let out = ((end.0 - other.0) / len, (end.1 - other.1) / len);
            (
                end.0 + out.0 * tuning().detour_clearance + normal.0 * tuning().detour_depth,
                end.1 + out.1 * tuning().detour_clearance + normal.1 * tuning().detour_depth,
            )
        })
        .map(|w| {
            let path = (w.0 - probe.0).hypot(w.1 - probe.1) + (to.0 - w.0).hypot(to.1 - w.1);
            (w, path)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(w, _)| w)
}

/// B6/B8: a point beyond the hedges or ditches that break the charge of `i`
/// at `j`, from which the charge is clear; `None` when there is none (a
/// village, or no clear way round after `BattleAiRules::detour_hops` tries): the horse
/// waits. B8: a dense network of hedges (bocage) is walked hedge by hedge
/// instead of only trying the ends of the first one in the way, which left
/// the horse waiting in front of the next hedge over.
pub(in crate::ai) fn detour(view: &View, i: usize, j: usize) -> Option<(f64, f64)> {
    let (u, e) = (&view.units[i], &view.units[j]);
    let field = view.sim.field();
    let to = (e.x, e.z);
    let mut probe = (u.x, u.z);
    let mut moved = false;
    for _ in 0..tuning().detour_hops {
        let blocking = field.obstacles.iter().find(|o| {
            o.kind.breaks_charge()
                && o.distance(to.0, to.1) <= SiteRules::bundled().hedge_cover_reach_m
                && (o.crosses(probe, to)
                    || o.distance(probe.0, probe.1) <= 2.0 * SiteRules::bundled().obstacle_reach_m)
        });
        let Some(blocking) = blocking else {
            return moved.then_some(probe);
        };
        probe = detour_past(probe, to, blocking).filter(|&(x, z)| {
            field.inside(x, z) && !field.in_forest(x, z) && field.water_at(x, z).is_none()
        })?;
        moved = true;
    }
    // Still blocked after tuning().detour_hops: a network too dense to clear.
    None
}

/// B6: charge `j` when the charge is clear; otherwise ride round the
/// obstacle in the way. `false` when neither is possible (the caller moves
/// on to its next choice, or waits).
pub(in crate::ai) fn charge_or_detour(view: &mut View, i: usize, j: usize, run: bool) -> bool {
    if !charge_breaks(view, i, j) {
        view.attack(i, j, run);
        return true;
    }
    match detour(view, i, j) {
        Some((x, z)) => {
            view.move_to(i, x, z, true, None);
            true
        }
        None => false,
    }
}
