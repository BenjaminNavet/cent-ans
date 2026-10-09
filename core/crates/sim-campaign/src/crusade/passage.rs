//! The crusade target and the passages of crusaders (landing, relief, desertion).

use super::*;

/// Takes or loses the target province according to who holds its city.
pub(super) fn sync_target(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let Some(rules) = active(state, data) else {
        return;
    };
    let held = state.controls_province(&rules.faction, &rules.target_province);
    let taken = state.crusade.as_ref().is_some_and(|c| c.target_taken);
    let name = target_name(state, data, rules);
    let faction = data.faction_name(&rules.faction);
    if held && !taken {
        let capital = state
            .factions
            .get(&rules.faction)
            .map(|f| f.capital.clone());
        let first = !state.crusade.as_ref().is_some_and(|c| c.target_ever_taken);
        if let Some(crusade) = state.crusade.as_mut() {
            crusade.target_taken = true;
            crusade.target_ever_taken = true;
            crusade.former_capital = capital.filter(|c| *c != rules.target_province);
        }
        // The gain and the prestige come with the first deliverance only.
        if first {
            change(
                state,
                rules,
                &format!("{name} délivrée"),
                rules.fervor.target_taken,
            );
            state.change_ruler_prestige(&rules.faction, rules.target_taken_prestige);
        }
        // The floor also lifts a gauge the gain left below it.
        change(state, rules, &format!("{name} délivrée"), 0);
        if let Some(f) = state.factions.get_mut(&rules.faction) {
            f.capital = rules.target_province.clone();
        }
        events.push(
            GameEvent::new(
                EventKind::Crusade,
                format!(
                    "{name} délivrée ! {} tient la cité de son vœu et y établit son siège.",
                    crate::events::capitalize(&faction)
                ),
            )
            .province(&rules.target_province)
            .faction(&rules.faction)
            .public(),
        );
    } else if !held && taken {
        let former = state.crusade.as_mut().and_then(|crusade| {
            crusade.target_taken = false;
            crusade.former_capital.take()
        });
        // Back to the former seat.
        if let (Some(former), Some(f)) = (former, state.factions.get_mut(&rules.faction)) {
            if f.capital == rules.target_province {
                f.capital = former;
            }
        }
        events.push(
            GameEvent::new(
                EventKind::Crusade,
                format!("{name} est perdue : la ferveur de {faction} n'a plus de plancher."),
            )
            .province(&rules.target_province)
            .faction(&rules.faction)
            .public()
            .loss(),
        );
    }
}

/// End of turn: the target's status, wear of the vow, peace with the
/// target's holder, landings, desertion, cooldown. The alms are paid with
/// the taxes ([`alms`]); the amount of the season is only noted here.
pub(crate) fn resolve_crusade(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let Some(rules) = active(state, data) else {
        return;
    };
    // The economy has just paid the alms at this fervour.
    let paid = state
        .crusade
        .as_ref()
        .map_or(0, |c| alms_at(rules, c.fervor));
    if let Some(crusade) = state.crusade.as_mut() {
        crusade.alms_last_turn = paid;
    }
    sync_target(state, data, events);
    change(
        state,
        rules,
        "Le vœu s'use",
        -i32::from(rules.fervor.decay_per_turn),
    );
    // JR4: exaltation does not last; above the high threshold it falls back
    // faster.
    let exalted = state
        .crusade
        .as_ref()
        .is_some_and(|c| c.fervor >= rules.zeal.high_threshold);
    if exalted {
        change(
            state,
            rules,
            "L'exaltation retombe",
            -i32::from(rules.fervor.decay_above_high),
        );
    }
    // Peace or truce with the master of the target, without holding it.
    let holder = state.province_controller(&rules.target_province).cloned();
    if let Some(holder) = holder {
        if holder != rules.faction
            && !holder.is_rebels()
            && !state.is_at_war(&rules.faction, &holder)
        {
            change(
                state,
                rules,
                &format!("Paix avec le maître de {}", target_name(state, data, rules)),
                rules.fervor.truce_with_target_holder_per_turn,
            );
        }
    }
    land_contingents(state, data, rules, events);
    desert(state, data, rules, events);
    relieve_sieges(state, data, rules, events);
    if let Some(crusade) = state.crusade.as_mut() {
        crusade.preach_cooldown = crusade.preach_cooldown.saturating_sub(1);
        crusade.relief_cooldown = crusade.relief_cooldown.saturating_sub(1);
    }
}

/// Generator of the contingents: seeded by the campaign seed and the turn,
/// it never draws from [`CampaignState::rng`].
pub(super) fn passage_rng(seed: u64, turn: u32, index: usize) -> CampaignRng {
    CampaignRng::from_seed(
        seed ^ 0x4352_5553_4144_4553
            ^ (u64::from(turn) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (index as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F),
    )
}

/// `count` unit types drawn from the weighted table (types missing from the
/// data are left out of the draw).
pub(super) fn draw_units(
    data: &GameData,
    unit_table: &[data_model::CrusadePassageUnit],
    rng: &mut CampaignRng,
    count: u32,
) -> Vec<UnitTypeId> {
    let table: Vec<(&UnitTypeId, u32)> = unit_table
        .iter()
        .filter(|e| e.weight > 0 && data.unit_types.contains_key(&e.unit))
        .map(|e| (&e.unit, e.weight))
        .collect();
    let total: u32 = table.iter().map(|(_, w)| w).sum();
    if total == 0 {
        return Vec::new();
    }
    (0..count)
        .map(|_| {
            let mut roll = rng.below(total);
            for (unit, weight) in &table {
                if roll < *weight {
                    return (*unit).clone();
                }
                roll -= weight;
            }
            table[0].0.clone()
        })
        .collect()
}

/// Lands the contingents due at the start of the coming turn: in their
/// port, else in another held port, else they are lost.
pub(super) fn land_contingents(
    state: &mut CampaignState,
    data: &GameData,
    rules: &CrusadeRules,
    events: &mut Vec<GameEvent>,
) {
    let next_turn = state.turn + 1;
    let Some(crusade) = state.crusade.as_mut() else {
        return;
    };
    let (due, waiting): (Vec<PendingPassage>, Vec<PendingPassage>) =
        std::mem::take(&mut crusade.pending_passages)
            .into_iter()
            .partition(|p| p.arrival_turn <= next_turn);
    crusade.pending_passages = waiting;
    for (index, passage) in due.into_iter().enumerate() {
        let booked_held = state
            .settlements
            .get(&passage.port)
            .is_some_and(|s| s.controller == rules.faction);
        let port = if booked_held {
            Some(passage.port.clone())
        } else {
            held_ports(state, data, rules).into_iter().next()
        };
        let Some(port) = port else {
            events.push(
                GameEvent::new(
                    EventKind::Crusade,
                    format!(
                        "Les volontaires du passage ne trouvent aucun port où débarquer : \
                         le contingent ({}) se disperse.",
                        count_noun(passage.units, "unité", "unités")
                    ),
                )
                .faction(&rules.faction),
            );
            continue;
        };
        let mut rng = passage_rng(state.seed, state.turn, index);
        let units = draw_units(data, &rules.passage.unit_table, &mut rng, passage.units);
        let landed = disembark(state, data, rules, &port, &units);
        let mut event = GameEvent::new(
            EventKind::Crusade,
            format!(
                "Un contingent de volontaires débarque à {} : {}.",
                data.settlement_name(&port),
                count_noun(landed, "unité", "unités")
            ),
        )
        .faction(&rules.faction);
        if let Some(province) = state.settlement_province(&port) {
            event = event.province(province);
        }
        events.push(event);
    }
}

/// Free places in the garrison of `settlement` (its kind's cap).
pub(super) fn garrison_room(
    state: &CampaignState,
    data: &GameData,
    settlement: &SettlementId,
) -> usize {
    let Some(place) = state.settlements.get(settlement) else {
        return 0;
    };
    data.settlement_rules
        .as_ref()
        .and_then(|r| r.garrison_cap.get(&place.kind).copied())
        .map_or(usize::MAX, |cap| cap.saturating_sub(place.garrison.len()))
}

/// JR5: lands `units` at `port` within its garrison cap; the surplus fills
/// the other held ports, then joins (or forms) a field army of the faction
/// at `port`. Returns the units landed.
pub(super) fn disembark(
    state: &mut CampaignState,
    data: &GameData,
    rules: &CrusadeRules,
    port: &SettlementId,
    units: &[UnitTypeId],
) -> u32 {
    let mut left: Vec<UnitTypeId> = units
        .iter()
        .filter(|id| data.unit_types.contains_key(*id))
        .cloned()
        .collect();
    let total = left.len() as u32;
    let mut ports = vec![port.clone()];
    ports.extend(
        held_ports(state, data, rules)
            .into_iter()
            .filter(|p| p != port),
    );
    for place in ports {
        if left.is_empty() {
            break;
        }
        let room = garrison_room(state, data, &place).min(left.len());
        let batch: Vec<UnitTypeId> = left.drain(..room).collect();
        spawn_units_at_settlement(state, data, &place, &batch);
    }
    if left.is_empty() {
        return total;
    }
    let fresh: Vec<Unit> = left
        .iter()
        .filter_map(|id| data.unit_types.get(id))
        .map(Unit::fresh)
        .collect();
    let cap = data.army_rules.cap();
    let position = crate::state::ArmyPosition::Settlement(port.clone());
    let existing = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == rules.faction && a.position == position && a.units.len() < cap)
        .map(|(id, _)| id.clone());
    let mut fresh = fresh.into_iter();
    if let Some(id) = existing {
        if let Some(army) = state.armies.get_mut(&id) {
            let room = cap.saturating_sub(army.units.len());
            army.units.extend(fresh.by_ref().take(room));
        }
    }
    let rest: Vec<Unit> = fresh.collect();
    if !rest.is_empty() {
        let id = state.allocate_army_id();
        let mut army = crate::state::Army::new(rules.faction.clone(), position, rest);
        army.movement_left = state.army_grid_allowance(data, &army);
        state.armies.insert(id, army);
    }
    total
}

/// JR4b « appel à défendre »: the master of a place of the Holy Land the
/// crusade besieges throws a relief levy into it, once per siege, at most
/// once every `cooldown_turns` turns, within the place's garrison cap. The
/// levy goes into the besieged garrison rather than the field: a besieged
/// place keeps its men when its master is in debt, a field levy would be
/// dismissed the next season by an indebted planner.
pub(super) fn relieve_sieges(
    state: &mut CampaignState,
    data: &GameData,
    rules: &CrusadeRules,
    events: &mut Vec<GameEvent>,
) {
    let Some(relief) = &rules.relief else {
        return;
    };
    let besieged: Vec<SettlementId> = state
        .settlements
        .iter()
        .filter(|(_, s)| {
            rules.holy_land.contains(&s.province)
                && !s.controller.is_rebels()
                && s.controller != rules.faction
                && s.siege
                    .as_ref()
                    .is_some_and(|g| g.attacker == rules.faction)
        })
        .map(|(id, _)| id.clone())
        .collect();
    let Some(crusade) = state.crusade.as_mut() else {
        return;
    };
    // A siege lifted or ended: the next one may be relieved again.
    crusade.relieved.retain(|id| besieged.contains(id));
    if crusade.relief_cooldown > 0 {
        return;
    }
    let Some(place) = besieged
        .iter()
        .find(|id| !crusade.relieved.contains(id))
        .cloned()
    else {
        return;
    };
    let Some(settlement) = state.settlements.get(&place) else {
        return;
    };
    let master = settlement.controller.clone();
    let cap = data
        .settlement_rules
        .as_ref()
        .and_then(|r| r.garrison_cap.get(&settlement.kind).copied())
        .unwrap_or(usize::MAX);
    let room = cap.saturating_sub(settlement.garrison.len()) as u32;
    let count = relief.units.min(room);
    if let Some(crusade) = state.crusade.as_mut() {
        crusade.relieved.push(place.clone());
        crusade.relief_cooldown = relief.cooldown_turns;
    }
    if count == 0 {
        return;
    }
    let index = state
        .settlements
        .keys()
        .position(|id| *id == place)
        .unwrap_or(0);
    let mut rng = passage_rng(state.seed ^ 0x5245_4C49_4546, state.turn, index);
    let units = draw_units(data, &relief.unit_table, &mut rng, count);
    let landed = spawn_units_at_settlement(state, data, &place, &units);
    if landed == 0 {
        return;
    }
    let name = data.settlement_name(&place);
    let mut event = GameEvent::new(
        EventKind::Crusade,
        format!(
            "Appel à défendre {name} contre {} : {} de secours {} dans la place.",
            data.faction_name(&rules.faction),
            count_noun(landed, "unité", "unités"),
            if landed > 1 { "entrent" } else { "entre" },
        ),
    )
    .faction(&master)
    // The besieging crusade reads it too, whoever plays.
    .public();
    if let Some(province) = state.settlement_province(&place) {
        event = event.province(province);
    }
    events.push(event);
}

/// « Débandade »: below the threshold a share of each unit's men goes home.
pub(super) fn desert(
    state: &mut CampaignState,
    data: &GameData,
    rules: &CrusadeRules,
    events: &mut Vec<GameEvent>,
) {
    let fervor = state.crusade.as_ref().map_or(100, |c| c.fervor);
    let percent = rules.desertion.percent_at(fervor);
    if percent == 0 {
        return;
    }
    let mut lost = 0;
    for army in state.armies.values_mut() {
        if army.faction != rules.faction {
            continue;
        }
        for unit in &mut army.units {
            // Rounded down: a company never deserts to the last man.
            let leaving = unit.strength * percent / 100;
            unit.strength -= leaving;
            lost += leaving;
        }
    }
    if lost > 0 {
        events.push(
            GameEvent::new(
                EventKind::Crusade,
                format!(
                    "Débandade : la ferveur retombe et {lost} hommes de {} rentrent chez eux.",
                    data.faction_name(&rules.faction)
                ),
            )
            .faction(&rules.faction),
        );
    }
}
