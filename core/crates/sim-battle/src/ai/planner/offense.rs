//! IA-PLAN-2: the offensive roles `fix` and `strike`, applied over the legacy
//! placement of `plan_field` for the regiments of the divisions that play them.
//!
//! `fix`: the foot of the division attacks the division target and keeps it
//! busy. `strike`: the foot marches round the flank of the division target and
//! falls on it once it is beside it (the horse is left to the grouped charge,
//! `ai/mass.rs`). `wear` and `screen` give no order of their own. A charge
//! that would break on the site (hedge, ditch, river, stakes) is never ordered
//! here: the legacy detours handle it.

use super::division::Division;
use super::roles::Role;
use super::*;

/// Applies `role` to the regiments of `division`.
pub(super) fn apply(view: &mut View, division: &Division, role: Role) {
    if !matches!(role, Role::Fix | Role::Strike) {
        return;
    }
    let Some(target) = division
        .target
        .and_then(|id| view.units.iter().position(|u| u.id == id))
        .filter(|&j| view.units[j].able())
    else {
        return;
    };
    let members: Vec<usize> = division
        .regiments
        .iter()
        .filter_map(|&id| view.units.iter().position(|u| u.id == id))
        .filter(|i| view.own.contains(i))
        .collect();
    // A regiment the legacy placement already sends at an enemy keeps that
    // order: the roles only move those that would wait or hold the line.
    let ordered: Vec<u32> = view
        .commands
        .iter()
        .filter_map(|c| match c {
            Command::Attack { units, .. } => Some(units.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    for i in members {
        let unit = &view.units[i];
        if role == Role::Fix && (ordered.contains(&unit.id) || unit.target.is_some()) {
            continue;
        }
        if !view.free(i)
            || unit.is_general
            || is_horse(unit)
            || (unit.can_shoot() && unit.ammo > 0)
            || charge_breaks(view, i, target)
            || stakes_in_path(view.units, (unit.x, unit.z), &view.units[target])
        {
            continue;
        }
        let d = dist(unit, &view.units[target]);
        let run = d < tuning().charge_distance && !steep_charge(view, i, target);
        if role == Role::Strike && d > tuning().charge_distance {
            let (px, pz) = flank_point(&view.units[target], unit);
            view.move_to(i, px, pz, false, None);
        } else {
            view.attack(i, target, run);
        }
    }
}

/// Where to march to come at `e` from the side `from` stands on.
fn flank_point(e: &Unit, from: &Unit) -> (f64, f64) {
    let (rx, rz) = e.right();
    let side = if (from.x - e.x) * rx + (from.z - e.z) * rz >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let (w, _) = e.extent();
    let (fx, fz) = e.forward();
    (
        e.x + rx * side * (w * 0.5 + 35.0) - fx * 15.0,
        e.z + rz * side * (w * 0.5 + 35.0) - fz * 15.0,
    )
}
