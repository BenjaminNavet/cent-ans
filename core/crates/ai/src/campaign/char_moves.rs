//! Free characters walk to where they serve (WR armies, ADR 0305): a province
//! with a leaderless army of the faction, or with a city about to release a
//! field army, gets the best free commander of another province (the order is
//! `SendCharacter`; the trip lasts turns by distance, then `assign_generals`
//! and the army creation find the commander on the spot).

use std::collections::BTreeSet;

use data_model::{CharacterId, ProvinceId};
use sim_campaign::Order;

use super::Context;

/// Characters sent per turn at most.
const MAX_SENT_PER_TURN: usize = 2;

/// `busy`: characters already commanding, governing or named this turn.
pub(super) fn plan_character_moves(
    ctx: &Context,
    busy: &BTreeSet<CharacterId>,
    orders: &mut Vec<Order>,
) {
    let state = ctx.state;
    let year = state.year();
    let ruler = state.factions[ctx.faction].ruler.as_ref();
    let mut spoken_for: BTreeSet<CharacterId> = busy.clone();
    for order in orders.iter() {
        match order {
            Order::AssignGovernor { character, .. }
            | Order::AssignGeneral { character, .. }
            | Order::SendCharacter { character, .. } => {
                spoken_for.insert(character.clone());
            }
            _ => {}
        }
    }
    // Free commanders of the faction.
    let free: Vec<(&CharacterId, &sim_campaign::CharacterState)> = state
        .characters
        .iter()
        .filter(|(id, c)| {
            c.alive
                && !c.captive
                && &c.faction == ctx.faction
                && c.is_major(year)
                && c.army.is_none()
                && c.governor_of.is_none()
                && c.journey.is_none()
                && c.location.is_some()
                && Some(*id) != ruler
                && !spoken_for.contains(*id)
        })
        .collect();
    if free.is_empty() {
        return;
    }
    let demands = demand_provinces(ctx);
    let mut sent = 0;
    let mut taken: BTreeSet<&CharacterId> = BTreeSet::new();
    for province in demands {
        if sent >= MAX_SENT_PER_TURN {
            break;
        }
        // Somebody already waits there (or is on the way): nothing to do.
        let served = state.characters.iter().any(|(id, c)| {
            &c.faction == ctx.faction
                && c.alive
                && c.army.is_none()
                && c.governor_of.is_none()
                && (c.location.as_ref() == Some(&province) && c.journey.is_none()
                    || c.journey.as_ref().is_some_and(|j| j.to == province))
                && Some(id) != ruler
        });
        if served {
            continue;
        }
        let best = free
            .iter()
            .filter(|(id, c)| !taken.contains(id) && c.location.as_ref() != Some(&province))
            .max_by_key(|(id, c)| (c.skills.command, std::cmp::Reverse((*id).clone())));
        let Some((id, _)) = best else {
            continue;
        };
        taken.insert(id);
        sent += 1;
        orders.push(Order::SendCharacter {
            character: (*id).clone(),
            to: province,
        });
    }
}

/// Provinces that need a commander, most urgent first: those holding a
/// leaderless army, then those with a city whose garrison exceeds its role.
fn demand_provinces(ctx: &Context) -> Vec<ProvinceId> {
    let (state, data) = (ctx.state, ctx.data);
    let mut out: Vec<ProvinceId> = Vec::new();
    let owned = |p: &ProvinceId| ctx.owns(p);
    for army in state
        .armies
        .values()
        .filter(|a| &a.faction == ctx.faction && a.general.is_none())
    {
        if let Some(p) = state.army_province(data, army) {
            if owned(&p) && !out.contains(&p) {
                out.push(p);
            }
        }
    }
    let room = state.faction_army_count(ctx.faction) < data.army_rules.upkeep.free_armies as usize;
    if room {
        for (id, settlement) in &state.settlements {
            if !ctx.holds(settlement) || settlement.siege.is_some() || !ctx.is_city(id) {
                continue;
            }
            let role = state
                .garrison_role(data, ctx.faction, &settlement.province)
                .garrison_size();
            let province = settlement.province.clone();
            if settlement.garrison.len() > role + 1 && owned(&province) && !out.contains(&province)
            {
                out.push(province);
            }
        }
    }
    out
}
