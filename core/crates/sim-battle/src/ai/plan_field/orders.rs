//! Reactions and the leader's orders.

use super::super::*;

/// Reactions common to every plan: face flank attacks, pull out wavering
/// regiments.
pub(in crate::ai) fn react(view: &mut View, roles: &Roles) {
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
            let (x, z) = field.clamp_inside(u.x, u.z - view.forward * 60.0, tuning().field_margin);
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
            let (x, z) = field.clamp_inside(u.x, u.z - view.forward * 70.0, tuning().field_margin);
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
pub(in crate::ai) fn ordered_to_move(view: &View, id: u32) -> bool {
    view.commands.iter().any(|c| {
        matches!(
            c,
            Command::Move { .. } | Command::Attack { .. } | Command::Withdraw { .. }
        ) && c.units().contains(&id)
    })
}

pub(in crate::ai) fn rivals(order: &BattleOrder, own: &str, enemy: &str) -> bool {
    order.ai.as_ref().is_none_or(|ai| {
        ai.rivals.is_empty()
            || ai.rivals.iter().any(|[a, b]| {
                (a.as_str() == own && b.as_str() == enemy)
                    || (a.as_str() == enemy && b.as_str() == own)
            })
    })
}

/// Leader's orders of this step (after the movement plan).
pub(in crate::ai) fn plan_orders(view: &mut View, defensive: bool) {
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
