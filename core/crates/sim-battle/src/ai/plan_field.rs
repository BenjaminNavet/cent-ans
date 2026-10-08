//! Open-field planning: charges, detours, reserves, reactions and leader orders.

use super::*;

/// B6: does a charge of `i` at `j` break on the site (a hedge or a ditch in
/// front of the target, the lanes of a village)?
pub(super) fn charge_breaks(view: &View, i: usize, j: usize) -> bool {
    let (u, e) = (&view.units[i], &view.units[j]);
    let field = view.sim.field();
    field.breaks_charge((u.x, u.z), (e.x, e.z))
        || field.in_village(e.x, e.z)
        || charge_breaks_on_water(field, e)
        || ReliefMap::river_between(field, (u.x, u.z), (e.x, e.z))
}

/// EP3: a target standing in the water, on a bridge or on a steep bank.
pub(super) fn charge_breaks_on_water(field: &crate::field::Battlefield, e: &Unit) -> bool {
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

/// B8: dense bocage can chain several hedges between a horse and its target;
/// this many are tried in turn before giving up and waiting.
pub(super) const DETOUR_HOPS: u32 = 4;

/// The point beyond the nearer end of `blocking`, on the target's side, from
/// which `probe` can ride on towards `to` clear of that one obstacle.
pub(super) fn detour_past(
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
                end.0 + out.0 * DETOUR_CLEARANCE + normal.0 * DETOUR_DEPTH,
                end.1 + out.1 * DETOUR_CLEARANCE + normal.1 * DETOUR_DEPTH,
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
/// village, or no clear way round after [`DETOUR_HOPS`] tries): the horse
/// waits. B8: a dense network of hedges (bocage) is walked hedge by hedge
/// instead of only trying the ends of the first one in the way, which left
/// the horse waiting in front of the next hedge over.
pub(super) fn detour(view: &View, i: usize, j: usize) -> Option<(f64, f64)> {
    let (u, e) = (&view.units[i], &view.units[j]);
    let field = view.sim.field();
    if field.in_village(e.x, e.z) {
        return None;
    }
    let to = (e.x, e.z);
    let mut probe = (u.x, u.z);
    let mut moved = false;
    for _ in 0..DETOUR_HOPS {
        let blocking = field.obstacles.iter().find(|o| {
            o.kind.breaks_charge()
                && o.distance(to.0, to.1) <= crate::site::HEDGE_COVER_REACH
                && (o.crosses(probe, to) || o.distance(probe.0, probe.1) <= 2.0 * OBSTACLE_REACH)
        });
        let Some(blocking) = blocking else {
            return moved.then_some(probe);
        };
        probe = detour_past(probe, to, blocking).filter(|&(x, z)| {
            field.inside(x, z) && !field.in_forest(x, z) && field.water_at(x, z).is_none()
        })?;
        moved = true;
    }
    // Still blocked after DETOUR_HOPS: a network too dense to clear.
    None
}

/// B6: horsemen ride this far past the end of a hedge before charging.
pub(super) const DETOUR_CLEARANCE: f64 = 35.0;
/// RJ-a: ... and this far out on the target's side, so that the charge is
/// not ridden along the hedge (within its reach it would break; the old
/// 10 m only worked while an instant wedge lengthened the riders' reach).
pub(super) const DETOUR_DEPTH: f64 = 25.0;

/// B6: charge `j` when the charge is clear; otherwise ride round the
/// obstacle in the way. `false` when neither is possible (the caller moves
/// on to its next choice, or waits).
pub(super) fn charge_or_detour(view: &mut View, i: usize, j: usize, run: bool) -> bool {
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

pub(super) fn plan_field(view: &mut View) {
    let roles = roles(view);
    // A6-L13b: the patience clocks run at the pace of the battle.
    let elapsed = view.sim.elapsed() / view.sim.ai_patience_factor();
    let own_power = view.power(true);
    let enemy_power = view.power(false).max(1.0);
    let ratio = own_power / enemy_power;
    let enemy_melee: Vec<usize> = view
        .able_enemies()
        .filter(|&j| is_melee_troop(&view.units[j]))
        .collect();
    // Distance from our line to the nearest able enemy.
    let front_ref: Vec<usize> = if roles.line.is_empty() {
        view.own.clone()
    } else {
        roles.line.clone()
    };
    let contact = front_ref
        .iter()
        .filter_map(|&i| view.nearest_enemy(i, |_| true).map(|(_, d)| d))
        .fold(f64::INFINITY, f64::min);
    // R2b: a defender standing clearly above the enemy does not give up its
    // ground to meet it (unless much stronger).
    let holds_heights =
        view.side == SideId::Defender && ratio < HOLD_RATIO && height_edge(view) > HOLD_HEIGHT;
    // EP3: a defender behind a river holds it (unless much stronger).
    let enemy_center = {
        let able: Vec<usize> = view.able_enemies().collect();
        view.centroid(&able)
    };
    let river_ahead = enemy_center.is_some_and(|e| {
        ReliefMap::river_between(
            view.sim.field(),
            deployment_center(view.sim.field(), view.side),
            e,
        )
    });
    let holds_river = view.side == SideId::Defender && ratio < HOLD_RATIO && river_ahead;
    // R4: an attacker clearly above an enemy made mostly of shooters waits
    // on its heights for a while rather than walking down into the arrows
    // (then attacks: no frozen battle).
    let attacker_holds = view.side == SideId::Attacker
        && ratio < HOLD_RATIO
        && elapsed < ATTACKER_HOLD_TIME
        && enemy_shooter_share(view) > SHOOTER_ARMY
        && side_losses(view.units, view.side)
            <= side_losses(view.units, view.side.other()) + DUEL_LOSS_MARGIN
        && height_edge(view) > HOLD_HEIGHT;
    // R4: an evenly matched defender whose enemy is marching on it waits
    // for it on its ground (behind its hedge, on its crest) rather than
    // leaving it as soon as its archers have thinned the enemy ranks.
    // An army made mostly of shooters needs the enemy to come to it
    // (Crécy, Agincourt): its strength counts only while it holds.
    let receives = view.side == SideId::Defender
        && (ratio < HOLD_RATIO
            || roles
                .shooters
                .iter()
                .map(|&i| unit_power(&view.units[i]))
                .sum::<f64>()
                > SHOOTER_ARMY * own_power)
        && enemy_melee.iter().any(|&j| {
            let e = &view.units[j];
            matches!(e.state, UnitState::Marching | UnitState::Charging)
                && front_ref
                    .iter()
                    .any(|&i| dist(&view.units[i], e) < RECEIVE_DISTANCE)
        });
    let defensive = match view.side {
        SideId::Defender => {
            (ratio < 0.85 || holds_heights || holds_river || receives)
                && (elapsed < DEFENDER_PATIENCE || view.sim.quiet_for(DEFENDER_QUIET))
        }
        SideId::Attacker => (ratio < 0.8 && elapsed < ATTACKER_WAIT) || attacker_holds,
    };
    let shooters_have_ammo = roles
        .shooters
        .iter()
        .any(|&i| view.units[i].ammo > view.units[i].stats.ammo / 5);
    let enemy_shooters = view
        .able_enemies()
        .filter(|&j| is_shooter(&view.units[j]))
        .count();
    let own_ranged: f64 = roles
        .shooters
        .iter()
        .map(|&i| unit_power(&view.units[i]))
        .sum();
    let enemy_ranged: f64 = view
        .able_enemies()
        .filter(|&j| is_shooter(&view.units[j]))
        .map(|j| unit_power(&view.units[j]))
        .sum();
    // The archery duel: hold the line while our shooters are winning it.
    // F5d: a clearly stronger attacker facing a defender that waits for it
    // takes the initiative (short duel, the horse rides at the enemy horse).
    // B4: a side bleeding faster than its enemy under the arrows stops
    // trading volleys and closes in.
    let press = view.side == SideId::Attacker && ratio * 0.85 > 1.0;
    let losing = side_losses(view.units, view.side)
        > side_losses(view.units, view.side.other()) + DUEL_LOSS_MARGIN;
    // EP9 (ADR 0056): the side that sought the battle must attack; it trades
    // volleys only for a while. EP9b: longer while its shooters are clearly
    // winning the duel (sliding window, `data/rules/battle_duel.json`).
    let duel_rules = view.sim.duel_rules();
    let window = duel_rules.window_seconds;
    let winning_duel = duel_rules.winning(
        view.sim.recent_missile_losses(view.side, window),
        view.sim.recent_missile_losses(view.side.other(), window),
    );
    let duel = !roles.shooters.is_empty()
        && shooters_have_ammo
        && contact < DUEL_RANGE
        && elapsed
            < if press && !(winning_duel && view.side == SideId::Attacker) {
                ATTACKER_DUEL_TIME
            } else if view.side == SideId::Attacker {
                duel_rules.attacker_limit(winning_duel)
            } else {
                DUEL_TIME
            }
        && !losing
        && (enemy_shooters == 0 || own_ranged >= enemy_ranged * 0.8);

    // EP9b: closing in, the militia (low base morale) marches in a second
    // echelon behind the solid foot rather than leading the assault through
    // the arrows. The line is measured on its first echelon (the second
    // follows it).
    let (first, second): (Vec<usize>, Vec<usize>) = roles
        .line
        .iter()
        .partition(|&&i| view.units[i].morale_cap >= duel_rules.second_echelon_morale);
    let echelons = !defensive
        && !duel
        && contact < DUEL_RANGE
        && contact >= duel_rules.second_echelon_closes_m
        && !first.is_empty()
        && !second.is_empty();
    let line_center = view
        .centroid(if echelons { &first } else { &roles.line })
        .or_else(|| view.centroid(&view.own))
        .expect("own not empty");
    let facing = if view.forward > 0.0 {
        0.0
    } else {
        std::f64::consts::PI
    };
    // Only to open the fight (first minutes, nobody locked yet).
    let melee = view.units.iter().any(|u| u.state == UnitState::Melee);
    // B4: an attacker bleeding under the enemy arrows (and not waiting as
    // the weaker side) also sends its horse at the enemy horse.
    let opens = press || (view.side == SideId::Attacker && losing);
    view.assault = opens
        && !defensive
        && !duel
        && !melee
        && elapsed < ATTACKER_PATIENCE
        && contact < ASSAULT_RANGE;
    // B6/R4: a defensive side takes the best ground within reach, cover
    // and relief scored together: behind a hedge, a ditch or in a village
    // (shooters just behind it, the line behind them), or on a crest.
    // EP3 first: the river bank at the crossing the enemy would take when
    // the river lies between us and the enemy and the crossing is within
    // reach (ADR 0033, "Cohabitation avec R4").
    let ground = if defensive && !attacker_holds {
        let bank = enemy_center.and_then(|e| river_hold(view.sim.field(), view.side, e));
        Some(match bank {
            Some(bank) => (bank.center, Some(bank)),
            None => defensive_ground(view, &roles),
        })
    } else {
        None
    };
    let cover = ground.and_then(|(_, c)| c);
    // EP3: an advancing side with the river in front picks its crossing.
    let crossing = if defensive {
        None
    } else {
        crossing_plan(view, line_center, &roles.line)
    };
    let mut shooter_anchor = None;
    // Where the line stands this step.
    let anchor = if let Some(c) = cover {
        let back = if roles.shooters.is_empty() {
            5.0
        } else {
            SHOOTERS_AHEAD
        };
        (c.center.0, c.z_at(c.center.0) - view.forward * back)
    } else if let Some((crest, _)) = ground {
        let post = reverse_slope_anchor(view, crest, !roles.shooters.is_empty());
        // SG5: an army on its heights, with its shooters holding the crest
        // in front, stands its line back out of reach of their rout (and of the horse
        // fighting in front of them): a broken regiment there no longer
        // carries the line with it before the melee (ADR 0046 § Suite SG5).
        // Measured from the crest itself: the line stepping back must not
        // change the decision.
        let field = view.sim.field();
        let enemies: Vec<usize> = view.able_enemies().collect();
        let below = view.centroid(&enemies).is_some_and(|(x, z)| {
            field.height(crest.0, crest.1) - field.height(x, z) > HOLD_HEIGHT
        });
        // A small force keeps its line by its shooters (R4); an army
        // deploys in depth.
        let rules = CrestDefenceRules::bundled();
        // A historical deployment (EP7) keeps the regiments on their posts.
        let posted = roles
            .line
            .iter()
            .any(|&i| view.sim.scenario_post(i).is_some());
        if !roles.shooters.is_empty()
            && below
            && !posted
            && roles.line.len() >= rules.min_line_regiments
        {
            (post.0, post.1 - view.forward * rules.line_setback_m)
        } else {
            post
        }
    } else if attacker_holds {
        // R4: an attacker above an enemy of shooters waits on its heights.
        line_center
    } else if let Some(plan) = crossing {
        // Shooters cover the crossing from the own bank.
        let (dx, dz) = (plan.far.0 - plan.near.0, plan.far.1 - plan.near.1);
        let len = dx.hypot(dz).max(1e-6);
        let ahead = (dx / len, dz / len);
        shooter_anchor = Some((
            plan.near.0 - ahead.0 * 8.0,
            plan.near.1 - ahead.1 * 8.0 - view.forward * SHOOTERS_AHEAD,
        ));
        let wait = plan.covered > 0
            && elapsed < CROSSING_PATIENCE
            && shooters_have_ammo
            && !roles.shooters.is_empty()
            && !losing;
        if wait {
            // Do not file across under the arrows: duel from the bank first.
            (plan.near.0 - ahead.0 * 60.0, plan.near.1 - ahead.1 * 60.0)
        } else {
            (
                plan.far.0 + ahead.0 * BRIDGEHEAD_DEPTH,
                plan.far.1 + ahead.1 * BRIDGEHEAD_DEPTH,
            )
        }
    } else if duel && contact < 260.0 {
        line_center
    } else {
        advance(view, line_center)
    };

    // Line (EP9b: in two echelons when closing in).
    let slots = if !echelons {
        view.line_slots(&roles.line, anchor, facing)
    } else {
        let depth = duel_rules.second_echelon_depth_m;
        let behind = (anchor.0, anchor.1 - view.forward * depth);
        let mut slots = view.line_slots(&first, anchor, facing);
        slots.extend(view.line_slots(&second, behind, facing));
        slots
    };
    for (i, x, z) in slots {
        if !view.free(i) {
            continue;
        }
        // ADR 0052: archers out of arrows fall only on horsemen already held
        // in a melee (Agincourt); they do not walk alone into fresh knights.
        let spent_archers = view.units[i].category == UnitCategory::Ranged;
        let target = view.nearest_enemy(i, |e| {
            e.state != UnitState::Routing
                && !(spent_archers && is_horse(e) && e.state != UnitState::Melee)
        });
        // R2b: a regiment well ahead of the line waits for it rather than
        // arriving alone under the enemy arrows (fast archers out of
        // arrows outpace the men-at-arms).
        // EP3: a regiment in the river or on a bridge finishes crossing.
        let ahead =
            (view.units[i].z - line_center.1) * view.forward > LINE_SLACK && !crossing_now(view, i);
        match target {
            Some((_, d)) if ahead && !defensive && d >= CHARGE_DISTANCE => view.halt(i),
            Some((j, d)) if !defensive && !duel && d < CHARGE_DISTANCE * 2.0 => {
                let j = opposite(view, i).unwrap_or(j);
                let run = d < CHARGE_DISTANCE && !steep_charge(view, i, j);
                view.attack(i, j, run);
            }
            Some((j, d)) if d < COUNTER_CHARGE_DISTANCE => view.attack(i, j, true),
            // R4: a defensive line comes to the help of its shooters caught
            // in a melee in front of it (the men-at-arms beside the archers).
            _ if defensive && rescue(view, i, &roles.shooters).is_some() => {
                let j = rescue(view, i, &roles.shooters).expect("checked");
                view.attack(i, j, true);
            }
            _ => {
                // R2b: an advancing line under arrows closes at the run
                // rather than walking up to the enemy shooters.
                let u = &view.units[i];
                let run = !defensive
                    && !duel
                    && u.missile_timer < UNDER_FIRE
                    && (z - u.z) * view.forward > 5.0;
                view.move_to(i, x, z, run, Some(facing));
            }
        }
    }

    // Shooters: in front of the line, halt in range, fall back when threatened.
    let slots = cover
        .map(|c| cover_slots(view, &roles.shooters, &c))
        .unwrap_or_default();
    for &i in &roles.shooters {
        let slot = slots
            .iter()
            .find(|s| s.0 == i)
            .map(|&(_, x, z)| (x, z))
            .zip(cover);
        let at = shooter_anchor.unwrap_or(anchor);
        let line_z = if shooter_anchor.is_some() {
            at.1
        } else {
            line_center.1
        };
        let crest = ground.filter(|(_, c)| c.is_none()).map(|(p, _)| p);
        plan_shooter(view, i, at, line_z, facing, defensive, slot, crest);
    }

    // Reserve.
    if let Some(r) = roles.reserve {
        plan_reserve(view, r, &roles.line, anchor, facing);
    }

    // Cavalry.
    for &i in &roles.horse {
        plan_horse(view, i, &roles, anchor, facing, defensive, &enemy_melee);
    }

    // Engines: stay behind the line, shoot at will.
    for &i in &roles.engines {
        let unit = &view.units[i];
        if let Some((j, d)) = view.nearest_enemy(i, |_| true) {
            let e = &view.units[j];
            let range = view.sim.effective_range(unit, e.x, e.z);
            if d > range * 0.95 && unit.ammo > 0 {
                let x = unit.x;
                let z = (anchor.1 - view.forward * 70.0).max(0.0);
                if (z - unit.z) * view.forward > 5.0 {
                    view.move_to(i, x, z, false, None);
                }
            } else {
                view.halt(i);
            }
        }
    }

    react(view, &roles);
    plan_orders(view, defensive);
    // CB2: guard for a defensive line, skirmish for light shooters.
    modes::plan_modes(view, &roles, defensive);
    // CB4: one simple rule per active ability.
    abilities::plan_abilities(view, defensive, false);
}

/// R2b: a defender this much higher than the enemy (mean ground under the
/// regiments, metres) holds its heights ...
pub const HOLD_HEIGHT: f64 = 6.0;
/// ... unless it is this much stronger.
pub const HOLD_RATIO: f64 = 1.25;

/// R4: a defensive line regiment helps its shooters in a melee this close.
pub const RESCUE_DISTANCE: f64 = 90.0;

/// R4: the nearest enemy in a melee with one of `shooters`, within
/// [`RESCUE_DISTANCE`] of line regiment `i`.
pub(super) fn rescue(view: &View, i: usize, shooters: &[usize]) -> Option<usize> {
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
        .filter(|&(_, d)| d < RESCUE_DISTANCE)
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(j, _)| j)
}

/// R4: shooters run to the military crest when the enemy is this close.
pub const POST_RUN: f64 = 400.0;
/// R4: a defender receives an enemy marching on it from this close.
pub const RECEIVE_DISTANCE: f64 = 250.0;
/// R4: an attacker above an enemy of shooters waits at most this long.
pub const ATTACKER_HOLD_TIME: f64 = 150.0;
/// R4: ... when shooters make more than this share of the enemy's power.
pub const SHOOTER_ARMY: f64 = 0.5;

/// Share of the able enemy's power in its shooters.
pub(super) fn enemy_shooter_share(view: &View) -> f64 {
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
pub(super) fn height_edge(view: &View) -> f64 {
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
pub(super) fn side_losses(units: &[Unit], side: SideId) -> f64 {
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

/// B8: within the forward arc, the line leans this many metres (at most)
/// towards a defender offset sideways from dead ahead, instead of marching
/// straight past it. A defender squarely in front (`dx` ~ 0) is unaffected.
pub(super) const ADVANCE_LEAN_MAX: f64 = 20.0;

/// Next step of an advancing line: 45 m towards the enemy's centroid (F5d:
/// armies that slipped past each other turn back instead of marching on to
/// the far edge).
pub(super) fn advance(view: &View, from: (f64, f64)) -> (f64, f64) {
    let able: Vec<usize> = view.able_enemies().collect();
    let Some((ex, ez)) = view.centroid(&able) else {
        return (from.0, from.1 + view.forward * 45.0);
    };
    let (dx, dz) = (ex - from.0, ez - from.1);
    let d = dx.hypot(dz);
    let straight = if dz * view.forward > 0.5 * d {
        // Ahead: march forward, leaning towards a defender offset
        // sideways (B8).
        let lean = dx.clamp(-ADVANCE_LEAN_MAX, ADVANCE_LEAN_MAX);
        (from.0 + lean, from.1 + view.forward * 45.0)
    } else {
        let step = d.min(45.0) / d.max(1e-6);
        (from.0 + dx * step, from.1 + dz * step)
    };
    relief_step(view, from, straight)
}

/// R2b: lateral shifts tried for each step of an advancing line.
pub(super) const RELIEF_LEANS: [f64; 4] = [-15.0, 15.0, -30.0, 30.0];
/// R2b: a shifted step must save this much march (metres) to be taken.
pub(super) const RELIEF_SAVING: f64 = 4.0;

/// R2b: of the step `from` -> `to` and the same step shifted aside, the one
/// the relief makes cheapest (round a steep rise by a valley or a shelf
/// rather than straight up it); the straight step on ties.
pub(super) fn relief_step(view: &View, from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let field = view.sim.field();
    let (dx, dz) = (to.0 - from.0, to.1 - from.1);
    let len = dx.hypot(dz).max(1e-6);
    let side = (dz / len, -dx / len);
    let cost = |p: (f64, f64)| {
        ReliefMap::march_cost(field, from, p)
            + 40.0 * (ReliefMap::climb(field, from, p) - STEEP_CLIMB).max(0.0)
    };
    let mut best = (to, cost(to) - RELIEF_SAVING);
    for lean in RELIEF_LEANS {
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
pub(super) fn steep_charge(view: &View, i: usize, j: usize) -> bool {
    let (u, e) = (&view.units[i], &view.units[j]);
    dist(u, e) > CLOSE_CHARGE
        && ReliefMap::climb(view.sim.field(), (u.x, u.z), (e.x, e.z)) > STEEP_CLIMB
}

/// Enemy regiment opposite `i` (smallest lateral offset, a bit of depth).
/// R4: the strength of the ground each one holds counts too, so that the
/// line strikes the weak point (the open ground rather than the hedge or
/// the crest).
pub(super) fn opposite(view: &View, i: usize) -> Option<usize> {
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
                (e.x - u.x).abs() + 0.3 * (e.z - u.z).abs() + WEAK_POINT * strength.max(0.0),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(j, _)| j)
}

pub(super) fn plan_reserve(
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
        if d < COUNTER_CHARGE_DISTANCE {
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

/// Reactions common to every plan: face flank attacks, pull out wavering
/// regiments.
pub(super) fn react(view: &mut View, roles: &Roles) {
    let units = view.units;
    let field = view.sim.field();
    let own = view.own.clone();
    for i in own {
        let u = &units[i];
        if u.state != UnitState::Melee || u.climbing.is_some() {
            continue;
        }
        // R2b: horsemen riding down a rout into the front of enemy stakes
        // break off.
        let rout_into_stakes = is_horse(u)
            && u.target
                .and_then(|t| units.get(t as usize))
                .is_some_and(|t| {
                    t.state == UnitState::Routing && stakes_in_path(units, (u.x, u.z), t)
                });
        if rout_into_stakes {
            let (x, z) = field.clamp_inside(u.x, u.z - view.forward * 60.0, FIELD_MARGIN);
            view.commands.push(Command::Move {
                units: vec![u.id],
                x,
                z,
                run: true,
                facing: None,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
            });
            continue;
        }
        // Shaken and bled: pull out before it breaks (if a reserve exists).
        if u.morale < 28.0
            && u.hp < f64::from(u.initial_soldiers) * 0.5
            && roles.reserve.is_some_and(|r| r != i)
            && !u.is_general
        {
            let (x, z) = field.clamp_inside(u.x, u.z - view.forward * 70.0, FIELD_MARGIN);
            view.commands.push(Command::Move {
                units: vec![u.id],
                x,
                z,
                run: true,
                facing: None,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
            });
            continue;
        }
        // Attacked on the flank: face the attacker.
        if u.flanked != 0 {
            let flanker = view.able_enemies().find(|&j| {
                let e = &units[j];
                dist(u, e) < 50.0 && attack_angle(u, e.x, e.z) > 0 && e.state == UnitState::Melee
            });
            if let Some(j) = flanker {
                if u.target != Some(units[j].id) {
                    view.commands.push(Command::Attack {
                        units: vec![u.id],
                        target: units[j].id,
                        run: false,
                        queue: false,
                    });
                }
            }
        }
    }
}

/// Is `id` already given a move (or attack, withdraw) in this step?
pub(super) fn ordered_to_move(view: &View, id: u32) -> bool {
    view.commands.iter().any(|c| {
        matches!(
            c,
            Command::Move { .. } | Command::Attack { .. } | Command::Withdraw { .. }
        ) && c.units().contains(&id)
    })
}

pub(super) fn rivals(order: &BattleOrder, own: &str, enemy: &str) -> bool {
    order.ai.as_ref().is_none_or(|ai| {
        ai.rivals.is_empty()
            || ai.rivals.iter().any(|[a, b]| {
                (a.as_str() == own && b.as_str() == enemy)
                    || (a.as_str() == enemy && b.as_str() == own)
            })
    })
}

/// Leader's orders of this step (after the movement plan).
pub(super) fn plan_orders(view: &mut View, defensive: bool) {
    let sim = view.sim;
    let side = view.side;
    let own_faction = &sim.setup().side(side).faction;
    let enemy_faction = &sim.setup().side(side.other()).faction;
    let mut orders: Vec<&BattleOrder> = sim.order_catalog().iter().collect();
    orders.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.id.cmp(&b.id)));
    for order in orders {
        let Some(ai) = &order.ai else {
            continue;
        };
        if sim.order_unavailable(side, order).is_some()
            || !rivals(order, own_faction, enemy_faction)
        {
            continue;
        }
        if let Some(max) = ai.max_strength_ratio {
            if view.power(true) / view.power(false).max(1.0) >= max {
                continue;
            }
        }
        let mut targets = sim.order_targets(side, order, &[]);
        if let Some(reach) = ai.enemy_within {
            let close = targets.iter().any(|&i| {
                view.nearest_enemy(i, |_| true)
                    .is_some_and(|(_, d)| d < reach)
            });
            if !close {
                continue;
            }
        }
        match order.kind {
            BattleOrderKind::Dismount => {
                if ai.when_defensive && !defensive {
                    continue;
                }
                let units = view.units;
                targets.retain(|&i| view.free(i) && units[i].state != UnitState::Charging);
            }
            BattleOrderKind::Pavise => {
                let window = ai.under_fire_within.unwrap_or(f64::INFINITY);
                let units = view.units;
                targets.retain(|&i| {
                    let u = &units[i];
                    u.missile_timer <= window
                        && u.destination.is_none()
                        && matches!(u.state, UnitState::Idle | UnitState::Shooting)
                        && !ordered_to_move(view, u.id)
                });
            }
            BattleOrderKind::WarCry | BattleOrderKind::NoQuarter | BattleOrderKind::Rally => {}
        }
        if targets.len() < ai.min_units as usize {
            continue;
        }
        let units = if order.scope == BattleOrderScope::Selected {
            targets.iter().map(|&i| view.units[i].id).collect()
        } else {
            Vec::new()
        };
        view.commands.push(Command::LeaderOrder {
            side: Some(side),
            order: order.id.clone(),
            units,
        });
    }
}
