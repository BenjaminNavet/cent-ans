//! Preaching a passage and the crusade AI.

use super::*;

/// Cost of preaching at the faction's current prices (H5).
pub fn passage_cost(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    data.crusade_rules.as_ref().map_or(0, |rules| {
        crate::coinage::priced(state, faction, rules.passage.cost)
    })
}

/// Why `faction` cannot preach the passage now (`None`: it can).
pub fn preach_blocker(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<CrusadeError> {
    let Some(rules) = active(state, data).filter(|rules| &rules.faction == faction) else {
        return Some(CrusadeError::NotCrusaders);
    };
    let crusade = state.crusade.as_ref()?;
    if crusade.preach_cooldown > 0 {
        return Some(CrusadeError::Cooldown(crusade.preach_cooldown));
    }
    if held_ports(state, data, rules).is_empty() {
        return Some(CrusadeError::NoPort);
    }
    let needed = passage_cost(state, data, faction);
    let available = state.factions.get(faction).map_or(0, |f| f.treasury);
    if available < needed {
        return Some(CrusadeError::InsufficientFunds { needed, available });
    }
    None
}

/// `preach_passage`: pays, raises fervour, books a contingent sized by the
/// fervour of the call for the best held port, and starts the cooldown.
pub fn preach_passage(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Result<(), CrusadeError> {
    if let Some(error) = preach_blocker(state, data, faction) {
        return Err(error);
    }
    let rules = active(state, data).ok_or(CrusadeError::NotCrusaders)?;
    let port = held_ports(state, data, rules)
        .into_iter()
        .next()
        .ok_or(CrusadeError::NoPort)?;
    let cost = passage_cost(state, data, faction);
    if let Some(f) = state.factions.get_mut(faction) {
        f.treasury -= cost;
    }
    change(state, rules, "Passage prêché", rules.fervor.preach);
    let turn = state.turn;
    let mut units = 0;
    if let Some(crusade) = state.crusade.as_mut() {
        units = rules.passage.units_at(crusade.fervor);
        crusade.preach_cooldown = rules.passage.cooldown_turns;
        crusade.pending_passages.push(PendingPassage {
            arrival_turn: turn + rules.passage.delay_turns,
            port: port.clone(),
            units,
        });
    }
    let mut event = GameEvent::new(
        EventKind::Crusade,
        format!(
            "{} prêche le passage : {} de volontaires {} à {} dans {}.",
            crate::events::capitalize(&data.faction_name(faction)),
            count_noun(units, "unité", "unités"),
            if units > 1 {
                "sont attendues"
            } else {
                "est attendue"
            },
            data.settlement_name(&port),
            count_noun(rules.passage.delay_turns, "tour", "tours")
        ),
    )
    .faction(faction);
    if let Some(province) = state.settlement_province(&port) {
        event = event.province(province);
    }
    state.push_order_event(event);
    Ok(())
}

/// Adds fresh units of `units` to the garrison of `settlement`; returns how
/// many were added (unknown unit types and settlements add nothing).
pub fn spawn_units_at_settlement(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    units: &[UnitTypeId],
) -> u32 {
    let Some(place) = state.settlements.get_mut(settlement) else {
        return 0;
    };
    let fresh: Vec<Unit> = units
        .iter()
        .filter_map(|id| data.unit_types.get(id))
        .map(Unit::fresh)
        .collect();
    let added = fresh.len() as u32;
    place.garrison.extend(fresh);
    added
}

/// AI: gold the crusade keeps aside for the passage (its price once the
/// cooldown is about to end and a port is held; 0 for every other faction
/// and for a crusade the player leads). Without it the planner spent the
/// alms on recruits and buildings, the passage was never affordable again
/// and the fervour wore down to nothing.
pub fn ai_passage_reserve(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    if faction == &state.player_faction {
        return 0;
    }
    let Some(rules) = active(state, data).filter(|rules| &rules.faction == faction) else {
        return 0;
    };
    let soon = state
        .crusade
        .as_ref()
        .is_some_and(|c| c.preach_cooldown <= 1);
    if soon && !held_ports(state, data, rules).is_empty() {
        passage_cost(state, data, faction)
    } else {
        0
    }
}

/// AI: preaches as soon as the action is available and affordable.
pub fn ai_preach(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    if preach_blocker(state, data, faction).is_none() {
        vec![Order::PreachPassage]
    } else {
        Vec::new()
    }
}
