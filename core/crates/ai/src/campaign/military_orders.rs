//! WR ai-mil (ADR 0301): the AI uses the three military orders of the
//! WH `armyb` lot, once reserved to the player: `RecruitInto` (recruit
//! straight into an army standing in the settlement), the sortie of a
//! besieged garrison and the besieger's `DemandSurrender`. Thresholds in
//! `data/ai/campaign.json` (`military_orders`).

use sim_campaign::{ArmyId, Order};

use super::Context;

/// Rewrites the `Recruit` orders of a settlement where one of the faction's
/// armies stands under `recruit_into_target_units` into `RecruitInto` that
/// army, as long as the garrison keeps `recruit_into_min_garrison` units and
/// the army has room (units already queued for it counted).
pub(super) fn recruit_into_armies(ctx: &Context, orders: &mut [Order]) {
    let rules = &ctx.rules.military_orders;
    let cap = ctx.data.army_rules.cap();
    let target = rules.recruit_into_target_units.min(cap);
    for settlement in ctx.state.settlements.keys() {
        if !ctx.owns_settlement(settlement) {
            continue;
        }
        let place = &ctx.state.settlements[settlement];
        // Besieged: the garrison needs the men, the army will sally or leave.
        if place.siege.is_some() || place.garrison.len() < rules.recruit_into_min_garrison {
            continue;
        }
        let queued_for = |army: &ArmyId| {
            place
                .recruit_queue
                .iter()
                .filter(|r| r.into_army.as_ref() == Some(army))
                .count()
        };
        // The smallest army first (ties by id), a planned recruit at a time.
        let mut hosts: Vec<(usize, ArmyId)> = ctx
            .state
            .armies
            .iter()
            .filter(|(_, a)| &a.faction == ctx.faction && a.is_at(settlement))
            .map(|(id, a)| (a.units.len() + queued_for(id), id.clone()))
            .filter(|(size, _)| *size < target)
            .collect();
        hosts.sort();
        let mut hosts = hosts.into_iter();
        let Some((mut size, mut host)) = hosts.next() else {
            continue;
        };
        for order in orders.iter_mut() {
            let Order::Recruit {
                settlement: site,
                unit_type,
            } = order
            else {
                continue;
            };
            if ctx.state.resolve_place(site).ok().as_ref() != Some(settlement) {
                continue;
            }
            *order = Order::RecruitInto {
                settlement: site.clone(),
                unit_type: unit_type.clone(),
                army: host.clone(),
            };
            size += 1;
            if size >= target {
                match hosts.next() {
                    Some((next_size, next_host)) => (size, host) = (next_size, next_host),
                    None => break,
                }
            }
        }
    }
}

/// Sorties of the besieged garrisons and surrender demands of the
/// besiegers, issued before the armies move (a demand comes before any
/// assault).
pub(super) fn plan_siege_orders(ctx: &Context, orders: &mut Vec<Order>) {
    let rules = &ctx.rules.military_orders;
    let (state, data) = (ctx.state, ctx.data);
    for (id, place) in &state.settlements {
        let Some(siege) = place.siege.as_ref() else {
            continue;
        };
        if ctx.holds(place) {
            if place.garrison.is_empty() {
                continue;
            }
            let garrison = sim_campaign::state::unit_power(data, &place.garrison);
            let besieging: f64 = state
                .armies
                .iter()
                .filter(|(_, a)| a.is_at(id) && state.is_at_war(ctx.faction, &a.faction))
                .map(|(army, _)| state.army_power(data, army))
                .sum();
            if besieging <= 0.0 {
                continue;
            }
            let odds = (100.0 * garrison / (garrison + besieging)).round() as u32;
            let starving = siege.supplies <= rules.sortie_starving_supply;
            if odds >= rules.sortie_odds || (starving && odds >= rules.sortie_desperate_odds) {
                orders.push(Order::Sortie {
                    settlement: id.into(),
                });
            }
        } else if &siege.attacker == ctx.faction {
            let held_by_a_besieger = state
                .armies
                .values()
                .any(|a| &a.faction == ctx.faction && a.is_at(id));
            let due = rules.surrender_retry_turns == 0
                || state.turn.saturating_sub(siege.started_turn) % rules.surrender_retry_turns == 0;
            if held_by_a_besieger
                && due
                && sim_campaign::siege::surrender_chance(state, data, id)
                    .is_some_and(|chance| chance >= rules.surrender_min_chance)
            {
                orders.push(Order::DemandSurrender {
                    settlement: id.into(),
                });
            }
        }
    }
}
