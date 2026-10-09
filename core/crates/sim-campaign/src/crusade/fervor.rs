//! Fervour scale: floor, changes, alms and zeal, fed by the war hooks.

use super::*;

/// Level fervour cannot fall below.
pub(super) fn floor(rules: &CrusadeRules, crusade: &CrusadeState) -> u8 {
    if crusade.target_taken {
        rules.fervor.target_floor.min(100)
    } else {
        0
    }
}

/// Moves the gauge by `delta` within `floor..=100` and notes the points
/// really gained or lost under `cause` (the notes of an earlier turn are
/// dropped first).
pub(super) fn change(state: &mut CampaignState, rules: &CrusadeRules, cause: &str, delta: i32) {
    let turn = state.turn;
    let Some(crusade) = state.crusade.as_mut() else {
        return;
    };
    let before = i32::from(crusade.fervor);
    let low = i32::from(floor(rules, crusade));
    let after = (before + delta).clamp(low, 100);
    crusade.fervor = after as u8;
    let applied = after - before;
    if applied == 0 {
        return;
    }
    if crusade.changes_turn != turn {
        crusade.last_changes.clear();
        crusade.changes_turn = turn;
    }
    match crusade.last_changes.iter_mut().find(|(c, _)| c == cause) {
        Some(entry) => entry.1 += applied,
        None => crusade.last_changes.push((cause.to_owned(), applied)),
    }
}

/// Name of the vow's goal: the city of the target province.
pub(super) fn target_name(state: &CampaignState, data: &GameData, rules: &CrusadeRules) -> String {
    match state.province_city_id(&rules.target_province) {
        Some(city) => data.settlement_name(city),
        None => rules.target_province.to_string(),
    }
}

/// Alms `faction` receives this turn (0 unless it is the crusaders):
/// `base + per_fervor × fervour`. Part of
/// [`CampaignState::faction_income`], so the treasury, the
/// projection, the budget and the AI all see them.
pub fn alms(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    let Some(rules) = &data.crusade_rules else {
        return 0;
    };
    if &rules.faction != faction {
        return 0;
    }
    match (&state.crusade, active(state, data)) {
        (Some(crusade), Some(_)) => alms_at(rules, crusade.fervor),
        _ => 0,
    }
}

pub(super) fn alms_at(rules: &CrusadeRules, fervor: u8) -> i64 {
    rules.alms.base + rules.alms.per_fervor * i64::from(fervor)
}

/// Ports `faction` holds, the best landing first: a port of the coastal
/// Holy Land, then the base settlement, then the others in id order.
pub(super) fn held_ports(
    state: &CampaignState,
    data: &GameData,
    rules: &CrusadeRules,
) -> Vec<SettlementId> {
    let mut ports: Vec<(u8, SettlementId)> = state
        .settlements
        .iter()
        .filter(|(id, s)| {
            s.controller == rules.faction && data.settlements.get(*id).is_some_and(|d| d.port)
        })
        .map(|(id, s)| {
            let rank = if rules.coastal_holy_land.contains(&s.province) {
                0
            } else if *id == rules.base_settlement {
                1
            } else {
                2
            };
            (rank, id.clone())
        })
        .collect();
    ports.sort();
    ports.into_iter().map(|(_, id)| id).collect()
}

/// A field battle, assault, sortie or sea fight between `winner` and
/// `loser` was decided; `attacker` is the one of the two that sought it. A
/// victory over another faith lifts the fervour and a defeat lowers it; a
/// battle the crusade itself sought against its own faith lowers it too
/// (attacked by brothers in faith, it only defends itself: no malus).
/// Rebels and kindred churches (schismatics, not infidels) count for
/// neither.
pub fn on_battle(
    state: &mut CampaignState,
    data: &GameData,
    winner: &FactionId,
    loser: &FactionId,
    attacker: &FactionId,
) {
    let Some(rules) = active(state, data) else {
        return;
    };
    let (won, other) = if winner == &rules.faction {
        (true, loser)
    } else if loser == &rules.faction {
        (false, winner)
    } else {
        return;
    };
    let relation = if other.is_rebels() {
        None
    } else {
        Some(religion::faith_relation(state, data, &rules.faction, other))
    };
    match relation {
        Some(FaithRelation::Same | FaithRelation::RivalObedience) if attacker == &rules.faction => {
            change(
                state,
                rules,
                "Bataille livrée contre des frères de foi",
                rules.fervor.battle_same_faith,
            )
        }
        Some(FaithRelation::Different) if won => change(
            state,
            rules,
            "Victoire sur une autre foi",
            rules.fervor.battle_won_other_faith,
        ),
        _ => {}
    }
    if !won {
        change(state, rules, "Bataille perdue", rules.fervor.battle_lost);
    }
}

/// `taker` took `settlement` by arms: a place of the Holy Land lifts the
/// fervour; the city of the target province delivers it (event for all,
/// capital moved, prestige, floor). Also notes the loss of the target.
pub fn on_settlement_taken(
    state: &mut CampaignState,
    data: &GameData,
    taker: &FactionId,
    settlement: &SettlementId,
    events: &mut Vec<GameEvent>,
) {
    let Some(rules) = active(state, data) else {
        return;
    };
    let taken_before = state.crusade.as_ref().is_some_and(|c| c.target_taken);
    sync_target(state, data, events);
    let delivered = !taken_before && state.crusade.as_ref().is_some_and(|c| c.target_taken);
    if taker != &rules.faction {
        return;
    }
    let in_holy_land = state
        .settlement_province(settlement)
        .is_some_and(|p| rules.holy_land.contains(p));
    // JR5: each place lifts the fervour once, however often it changes hands.
    let first = in_holy_land
        && state.crusade.as_mut().is_some_and(|c| {
            let new = !c.counted_places.contains(settlement);
            if new {
                c.counted_places.push(settlement.clone());
            }
            new
        });
    if in_holy_land && first && !delivered {
        change(
            state,
            rules,
            "Place prise en Terre sainte",
            rules.fervor.holy_land_settlement_taken,
        );
    }
}

/// `aggressor` declared war on `target`: a crusade that turns on its own
/// faith loses its fervour.
pub fn on_war_declared(
    state: &mut CampaignState,
    data: &GameData,
    aggressor: &FactionId,
    target: &FactionId,
) {
    let Some(rules) = active(state, data) else {
        return;
    };
    if aggressor != &rules.faction || target.is_rebels() {
        return;
    }
    if matches!(
        religion::faith_relation(state, data, aggressor, target),
        FaithRelation::Same | FaithRelation::RivalObedience
    ) {
        change(
            state,
            rules,
            "Guerre déclarée à des frères de foi",
            rules.fervor.war_declared_same_faith,
        );
    }
}

/// Morale points (0-100 scale) the armies of `faction` get from fervour
/// (« Élan de la Croix »): a bonus at or above the high threshold, a malus
/// below the low one.
pub fn zeal_morale(state: &CampaignState, data: &GameData, faction: &FactionId) -> i32 {
    let Some(rules) = &data.crusade_rules else {
        return 0;
    };
    if &rules.faction != faction || active(state, data).is_none() {
        return 0;
    }
    state
        .crusade
        .as_ref()
        .map_or(0, |c| zeal_at(rules, c.fervor))
}

pub(super) fn zeal_at(rules: &CrusadeRules, fervor: u8) -> i32 {
    if fervor >= rules.zeal.high_threshold {
        rules.zeal.high_morale
    } else if fervor < rules.zeal.low_threshold {
        rules.zeal.low_morale
    } else {
        0
    }
}
