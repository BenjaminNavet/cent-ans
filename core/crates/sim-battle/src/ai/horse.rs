//! Cavalry plans.

use super::*;

/// An enemy regiment attacking one of ours on the flank or rear.
pub(super) fn flanker(view: &View) -> Option<usize> {
    let units = view.units;
    for &i in &view.own {
        let u = &units[i];
        if u.flanked == 0 && u.state != UnitState::Melee {
            continue;
        }
        let found = view.able_enemies().find(|&j| {
            let e = &units[j];
            e.state == UnitState::Melee && dist(u, e) < 60.0 && attack_angle(u, e.x, e.z) > 0
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn plan_horse(
    view: &mut View,
    i: usize,
    roles: &Roles,
    anchor: (f64, f64),
    facing: f64,
    defensive: bool,
    enemy_melee: &[usize],
) {
    if !view.free(i) {
        return;
    }
    let units = view.units;
    let unit = &units[i];
    let general_only = unit.is_general && roles.horse.len() > 1;
    // Mounted shooters skirmish like foot shooters but keep their distance.
    if unit.can_shoot() && unit.ammo > 0 {
        if let Some((j, d)) = view.nearest_enemy(i, |_| true) {
            let e = &units[j];
            let range = view.sim.effective_range(unit, e.x, e.z);
            if d < SHOOTER_SAFETY && is_melee_troop(e) {
                view.move_to(i, unit.x, unit.z - view.forward * 80.0, true, None);
            } else if d <= range * 0.95 {
                view.halt(i);
            } else {
                view.move_to(i, e.x, e.z - view.forward * range * 0.85, false, None);
            }
        }
        return;
    }
    // 1. Enemy cavalry close to us or our shooters: counter-charge (SG4:
    // the shooters' guard too, not only the horse the enemy rides at).
    let threatened = view.able_enemies().find(|&j| {
        let e = &units[j];
        is_horse(e)
            && matches!(e.state, UnitState::Charging | UnitState::Marching)
            && (dist(unit, e) < 160.0
                || (!defensive
                    && dist(unit, e) < CAVALRY_REACH
                    && roles
                        .shooters
                        .iter()
                        .any(|&s| units[s].able() && dist(&units[s], e) < SHOOTER_GUARD)))
    });
    if let Some(j) = threatened {
        if !bristling(&units[j]) {
            view.attack(i, j, true);
            return;
        }
    }
    // 1b. Closing in: ride at the enemy horse (not bristling) within reach.
    if view.assault && !general_only && !unit.is_general {
        let mut horse: Vec<(usize, f64)> = view
            .able_enemies()
            .filter(|&j| is_horse(&units[j]) && !bristling(&units[j]))
            .filter(|&j| units[j].category != UnitCategory::Siege)
            .map(|j| (j, dist(unit, &units[j])))
            .filter(|&(_, d)| d < CAVALRY_REACH)
            .collect();
        horse.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        // B6: horsemen behind a hedge are ridden round, or left alone.
        for (j, d) in horse {
            if waits_for_foot(view, roles, i, j) {
                continue;
            }
            if charge_or_detour(view, i, j, d < CHARGE_DISTANCE * 3.0) {
                return;
            }
        }
    }
    // 2. Isolated shooters (not behind stakes facing us).
    let mut isolated: Vec<(usize, f64)> = view
        .able_enemies()
        .filter(|&j| is_shooter(&units[j]) && units[j].state != UnitState::Melee)
        .filter(|&j| {
            let e = &units[j];
            enemy_melee
                .iter()
                .all(|&m| dist(&units[m], e) > ISOLATION_DISTANCE)
                && !(e.stakes_planted && attack_angle(e, unit.x, unit.z) == 0)
        })
        .map(|j| (j, dist(unit, &units[j])))
        .filter(|&(_, d)| d < CAVALRY_REACH)
        .collect();
    isolated.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if !general_only {
        // B6: shooters behind a hedge are ridden round, or left alone.
        for (j, _) in isolated {
            if charge_or_detour(view, i, j, true) {
                return;
            }
        }
    }
    // 3. Exposed flank: an enemy regiment locked in melee with our troops.
    let exposed = view
        .able_enemies()
        .filter(|&j| {
            let e = &units[j];
            e.state == UnitState::Melee
                && !e.braced()
                && !e.has(Ability::PikeSquare)
                && !stakes_in_path(units, (unit.x, unit.z), e)
                && !charge_breaks(view, i, j)
        })
        .map(|j| (j, dist(unit, &units[j])))
        .filter(|&(_, d)| d < CAVALRY_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if let (Some((j, d)), false) = (exposed, general_only) {
        let e = &units[j];
        // A6 x LR: a charge already launched at `j` is not broken off to ride
        // round: the target turns to face the charge in the last 30 m (its
        // angle becomes frontal) and, with the slower A6 pace and the RJ-a
        // wedge, that happens beyond the 25 m below.
        let committed =
            unit.target.is_some_and(|t| t as usize == j) && unit.state == UnitState::Charging;
        if attack_angle(e, unit.x, unit.z) > 0 || d < 25.0 || committed {
            view.attack(i, j, true);
            return;
        }
        {
            // Ride round to its flank first.
            let (rx, rz) = e.right();
            let side = if (unit.x - e.x) * rx + (unit.z - e.z) * rz >= 0.0 {
                1.0
            } else {
                -1.0
            };
            let (w, _) = e.extent();
            let (fx, fz) = e.forward();
            let flank = |side: f64| {
                (
                    e.x + rx * side * (w * 0.5 + 35.0) - fx * 15.0,
                    e.z + rz * side * (w * 0.5 + 35.0) - fz * 15.0,
                )
            };
            // R2b: not by a flank that passes in front of stakes.
            let clear = |p: (f64, f64)| {
                !stakes_on_ride(units, e.side, (unit.x, unit.z), p, None)
                    && !stakes_on_ride(units, e.side, p, (e.x, e.z), Some(e.id))
            };
            let way = [flank(side), flank(-side)].into_iter().find(|&p| clear(p));
            if let Some((px, pz)) = way {
                view.move_to(i, px, pz, true, None);
                return;
            }
            // R2b: both flanks pass in front of stakes: next choice.
        }
    }
    // 4. Pursuit of routing regiments (B8: leashed — a rout that has
    // already fled too far from the battle line is left to run; chasing it
    // down would strand the horse and delay the main fight).
    let routing = view
        .enemies
        .iter()
        .copied()
        .filter(|&j| units[j].state == UnitState::Routing && units[j].present())
        .filter(|&j| !stakes_in_path(units, (unit.x, unit.z), &units[j]))
        .map(|j| (j, dist(unit, &units[j])))
        .filter(|&(_, d)| d < 350.0)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    if let Some((j, _)) = routing {
        let e = &units[j];
        if (e.x - anchor.0).hypot(e.z - anchor.1) < PURSUIT_LEASH {
            if unit.target != Some(e.id) {
                view.commands.push(Command::Attack {
                    units: vec![unit.id],
                    target: e.id,
                    run: true,
                    queue: false,
                });
            }
            return;
        }
    }
    // 5. A shaken or bled regiment right in front (not bristling): ride it down.
    let shaken = |e: &Unit| {
        !bristling(e)
            && !stakes_in_path(units, (unit.x, unit.z), e)
            && (e.morale < 40.0 || e.hp < f64::from(e.initial_soldiers) * 0.5)
    };
    if let Some((j, d)) = view.nearest_enemy(i, shaken) {
        if d < 110.0
            && !defensive
            && !general_only
            && !charge_breaks(view, i, j)
            && !waits_for_foot(view, roles, i, j)
        {
            view.attack(i, j, true);
            return;
        }
    }
    // Otherwise: hold the wing (the general's regiment behind the centre).
    let half = roles
        .line
        .iter()
        .map(|&k| units[k].extent().0 + 10.0)
        .sum::<f64>()
        * 0.5;
    let (x, z) = if general_only || unit.is_general {
        (anchor.0, anchor.1 - view.forward * 50.0)
    } else {
        let side = if unit.x >= anchor.0 { 1.0 } else { -1.0 };
        (
            anchor.0 + side * (half + 45.0),
            anchor.1 - view.forward * 15.0,
        )
    };
    let (x, z) = wing_behind_foot(view, roles, i, (x, z));
    view.move_to(i, x, z, false, Some(facing));
}

/// IA night (suite of EQ7): the attacker's horse holding its wing (or
/// behind the centre) while it waits for its foot, once under the enemy
/// arrows, stands no further forward than that foot. Posted from the advancing line's anchor, the knights used
/// to ride ahead of their slow foot and stand idle under the enemy longbows,
/// losing most of their men before their charge. Measured on
/// `tests/ia_survey.rs` (16 seeds, 4 reliefs, both sides): the French army
/// attacking an English army that waits for it 5 -> 44 wins of 64, against
/// a novice 38 -> 51; mirrored armies unchanged. The defender's horse, which
/// waits for the enemy, keeps its post (moving it cost 20 wins of 64).
pub(super) fn wing_behind_foot(
    view: &View,
    roles: &Roles,
    i: usize,
    post: (f64, f64),
) -> (f64, f64) {
    if view.side != SideId::Attacker {
        return post;
    }
    // Only a horse the enemy shooters have been hitting: elsewhere it keeps
    // its post ahead (the horse clashes that open a battle).
    if view.units[i].missile_timer >= WING_UNDER_FIRE_S {
        return post;
    }
    let foot: Vec<usize> = roles
        .line
        .iter()
        .copied()
        .filter(|&k| view.units[k].able())
        .collect();
    // Once the foot is about to strike, the horse moves up with it.
    let closing = foot.iter().any(|&k| {
        view.nearest_enemy(k, |_| true)
            .is_some_and(|(_, d)| d < WING_RELEASE_M)
    });
    if closing {
        return post;
    }
    match view.centroid(&foot) {
        Some((_, foot_z))
            if (post.1 - (foot_z - view.forward * WING_BEHIND_FOOT)) * view.forward > 0.0 =>
        {
            (post.0, foot_z - view.forward * WING_BEHIND_FOOT)
        }
        _ => post,
    }
}

/// IA night: the attacker's waiting horse stands this far (metres) behind
/// its foot's centre ...
pub(super) const WING_BEHIND_FOOT: f64 = 15.0;
/// ... until a foot regiment is this close (metres) to an enemy one (80 m:
/// 672 wins of 768 against the novice and the passive side, before 618;
/// 150 m: 651, 250 m: 610).
pub(super) const WING_RELEASE_M: f64 = 80.0;
/// IA night: the rule holds for a horse that took missile casualties within
/// this many seconds (40 s: 670 wins of 768, mirrored armies unchanged and
/// the demo's first contact still near 70 s; always: 672, but the first
/// contact at 192 s).
pub(super) const WING_UNDER_FIRE_S: f64 = 40.0;

/// EQ7 (suite of ADR 0052): the horse of an attacker with foot does not
/// ride at enemy horse or at a shaken regiment covered by enemy shooters who
/// still have arrows before its foot is close to that target, or already in
/// a melee (Crécy, Poitiers: the knights sent ahead alone broke under the
/// arrows). It holds the wing meanwhile, and goes on once committed close
/// to its target. The defender's horse, which waits for the enemy, and the
/// charge at isolated shooters (which pins them) are unchanged.
pub(super) fn waits_for_foot(view: &View, roles: &Roles, i: usize, j: usize) -> bool {
    if view.side != SideId::Attacker {
        return false;
    }
    let rules = HorseWaitRules::bundled();
    let units = view.units;
    let (unit, target) = (&units[i], &units[j]);
    if dist(unit, target) < rules.committed_m {
        return false;
    }
    let covered = view.able_enemies().any(|s| {
        let shooter = &units[s];
        is_shooter(shooter)
            && shooter.state != UnitState::Melee
            && dist(shooter, target)
                <= view.sim.effective_range(shooter, target.x, target.z) * rules.range_margin
    });
    let foot: Vec<&Unit> = roles
        .line
        .iter()
        .map(|&k| &units[k])
        .filter(|u| u.able())
        .collect();
    covered
        && !foot.is_empty()
        && !foot
            .iter()
            .any(|u| u.state == UnitState::Melee || dist(u, target) < rules.foot_close_m)
}
