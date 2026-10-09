//! Open-field planning: charges, detours, reserves, reactions and leader orders.
//!
//! Sub-modules: `charge` (does a charge break on the site, detours),
//! `measures` (rescue, shooter share, height edge, losses, advance steps),
//! `reserve` (the reserve regiment), `orders` (reactions, leader orders).
//! This file holds the per-step plan of the whole side, [`plan_field`].

use super::*;

mod charge;
mod measures;
mod orders;
mod reserve;
pub(in crate::ai) use self::charge::*;
pub(in crate::ai) use self::measures::*;
pub(in crate::ai) use self::orders::*;
pub(in crate::ai) use self::reserve::*;

pub(in crate::ai) fn plan_field(view: &mut View) {
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
    let holds_heights = view.side == SideId::Defender
        && ratio < tuning().hold_ratio
        && height_edge(view) > tuning().hold_height;
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
    let holds_river = view.side == SideId::Defender && ratio < tuning().hold_ratio && river_ahead;
    // R4: an attacker clearly above an enemy made mostly of shooters waits
    // on its heights for a while rather than walking down into the arrows
    // (then attacks: no frozen battle).
    let attacker_holds = view.side == SideId::Attacker
        && ratio < tuning().hold_ratio
        && elapsed < tuning().attacker_hold_time
        && enemy_shooter_share(view) > tuning().shooter_army
        && side_losses(view.units, view.side)
            <= side_losses(view.units, view.side.other()) + tuning().duel_loss_margin
        && height_edge(view) > tuning().hold_height;
    // R4: an evenly matched defender whose enemy is marching on it waits
    // for it on its ground (behind its hedge, on its crest) rather than
    // leaving it as soon as its archers have thinned the enemy ranks.
    // An army made mostly of shooters needs the enemy to come to it
    // (Crécy, Agincourt): its strength counts only while it holds.
    let receives = view.side == SideId::Defender
        && (ratio < tuning().hold_ratio
            || roles
                .shooters
                .iter()
                .map(|&i| unit_power(&view.units[i]))
                .sum::<f64>()
                > tuning().shooter_army * own_power)
        && enemy_melee.iter().any(|&j| {
            let e = &view.units[j];
            matches!(e.state, UnitState::Marching | UnitState::Charging)
                && front_ref
                    .iter()
                    .any(|&i| dist(&view.units[i], e) < tuning().receive_distance)
        });
    let defensive = match view.side {
        SideId::Defender => {
            (ratio < 0.85 || holds_heights || holds_river || receives)
                && (elapsed < tuning().defender_patience
                    || view.sim.quiet_for(tuning().defender_quiet))
        }
        SideId::Attacker => (ratio < 0.8 && elapsed < tuning().attacker_wait) || attacker_holds,
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
        > side_losses(view.units, view.side.other()) + tuning().duel_loss_margin;
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
        && contact < tuning().duel_range
        && elapsed
            < if press && !(winning_duel && view.side == SideId::Attacker) {
                tuning().attacker_duel_time
            } else if view.side == SideId::Attacker {
                duel_rules.attacker_limit(winning_duel)
            } else {
                tuning().duel_time
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
        && contact < tuning().duel_range
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
        && elapsed < tuning().attacker_patience
        && contact < tuning().assault_range;
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
            tuning().shooters_ahead
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
            field.height(crest.0, crest.1) - field.height(x, z) > tuning().hold_height
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
            plan.near.1 - ahead.1 * 8.0 - view.forward * tuning().shooters_ahead,
        ));
        let wait = plan.covered > 0
            && elapsed < tuning().crossing_patience
            && shooters_have_ammo
            && !roles.shooters.is_empty()
            && !losing;
        if wait {
            // Do not file across under the arrows: duel from the bank first.
            (plan.near.0 - ahead.0 * 60.0, plan.near.1 - ahead.1 * 60.0)
        } else {
            (
                plan.far.0 + ahead.0 * tuning().bridgehead_depth,
                plan.far.1 + ahead.1 * tuning().bridgehead_depth,
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
    // RX batsim: the crossing is filed through a few regiments at a time, and the
    // regiments already across hold the bridgehead until the line is across.
    let queue = crossing.map(|plan| crossing_queue(view, &plan, &roles.line));
    for (i, x, z) in slots {
        if !view.free(i) {
            continue;
        }
        if let (Some(plan), Some(q)) = (crossing, queue.as_ref()) {
            let near_foe = view
                .nearest_enemy(i, |e| e.state != UnitState::Routing)
                .is_some_and(|(_, d)| d < tuning().counter_charge_distance);
            if q.waiting.contains(&i) {
                view.halt(i);
                continue;
            }
            if q.across.contains(&i) {
                if !near_foe {
                    view.move_to(i, x, z, false, Some(facing));
                    continue;
                }
            } else if !near_foe && !crossing_now(view, i) {
                // Make for the entry of the crossing rather than swim across
                // (men drowned by the dozen off the bridge).
                let u = &view.units[i];
                if (u.x - plan.near.0).hypot(u.z - plan.near.1) > tuning().crossing_entry_m {
                    view.move_to(i, plan.near.0, plan.near.1, false, Some(facing));
                    continue;
                }
            }
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
        let ahead = (view.units[i].z - line_center.1) * view.forward > tuning().line_slack
            && !crossing_now(view, i);
        match target {
            Some((_, d)) if ahead && !defensive && d >= tuning().charge_distance => view.halt(i),
            Some((j, d)) if !defensive && !duel && d < tuning().charge_distance * 2.0 => {
                let j = opposite(view, i).unwrap_or(j);
                let run = d < tuning().charge_distance && !steep_charge(view, i, j);
                view.attack(i, j, run);
            }
            Some((j, d)) if d < tuning().counter_charge_distance => view.attack(i, j, true),
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
                    && u.missile_timer < tuning().under_fire
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

    plan_engines(view, &roles.engines, anchor);

    react(view, &roles);
    plan_orders(view, defensive);
    // CB2: guard for a defensive line, skirmish for light shooters.
    modes::plan_modes(view, &roles, defensive);
    // CB4: one simple rule per active ability.
    abilities::plan_abilities(view, defensive, false);
}

/// Engines stay behind the line and shoot at will.
fn plan_engines(view: &mut View, engines: &[usize], anchor: (f64, f64)) {
    for &i in engines {
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
}
