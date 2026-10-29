//! ECO-R2a (ADR XXXX): the feudal host gives soldiers.
//!
//! A vassal that answers its suzerain's summons no longer only declares the war: it raises a
//! **contingent**, an army under its own flag whose seasonal upkeep is at most
//! `host.contingent_percent_of_income` of its income, made of the `host.unit_mix`. The
//! contingent serves `host.free_seasons` at the vassal's expense, then either goes home or, if
//! the suzerain can pay, stays up to `host.paid_seasons` more seasons at the suzerain's pay
//! (`host.wage_percent` of its upkeep, paid to the vassal). Refusing the summons still opens a
//! felony case (§ 4.4).

use data_model::{FactionId, GameData, UnitTypeId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::{Army, ArmyId, ArmyPosition, CampaignState, Unit};

use super::{answers_host, can_serve, direct_vassals, felony};

/// Marks the contingent a vassal raised for the host of its suzerain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostService {
    /// The suzerain whose host the contingent joined.
    pub liege: FactionId,
    /// The enemy of that host.
    pub enemy: FactionId,
    /// Seasons served in the current phase (free, then paid).
    pub seasons: u32,
    /// `true`: the suzerain pays the contingent (second phase).
    pub paid: bool,
}

/// Seasonal upkeep of one regiment of `unit_type` (livres).
fn season_upkeep(data: &GameData, unit_type: &UnitTypeId) -> i64 {
    data.unit_types.get(unit_type).map_or(0, |t| {
        i64::from(t.upkeep) * data.economy_rules.upkeep_months_per_season
    })
}

/// The regiments `vassal` would raise: its upkeep budget (a share of its seasonal income) split
/// over the unit mix, within `min_units..=max_units`. Empty when the vassal has no income, the
/// share is `0` or the mix is empty.
pub fn contingent_units(state: &CampaignState, data: &GameData, vassal: &FactionId) -> Vec<Unit> {
    let rules = &data.feudal_rules.host;
    if rules.contingent_percent_of_income == 0 || rules.unit_mix.is_empty() {
        return Vec::new();
    }
    let income = state.faction_income(data, vassal);
    if income <= 0 {
        return Vec::new();
    }
    let budget = income * i64::from(rules.contingent_percent_of_income) / 100;
    let mut types: Vec<&UnitTypeId> = Vec::new();
    for line in &rules.unit_mix {
        let cost = season_upkeep(data, &line.unit_type).max(1);
        let share = budget * i64::from(line.share_percent) / 100;
        for _ in 0..share / cost {
            types.push(&line.unit_type);
        }
    }
    let max = rules.max_units.max(rules.min_units) as usize;
    types.truncate(max);
    // A poor vassal still sends its least expensive regiment, up to the minimum.
    if let Some(cheapest) = rules
        .unit_mix
        .iter()
        .map(|l| &l.unit_type)
        .min_by_key(|t| season_upkeep(data, t))
    {
        while types.len() < rules.min_units as usize {
            types.push(cheapest);
        }
    }
    types
        .into_iter()
        .filter_map(|id| data.unit_types.get(id))
        .map(Unit::fresh)
        .collect()
}

/// Does `vassal` already have a contingent in the field for `liege`?
fn has_contingent(state: &CampaignState, vassal: &FactionId, liege: &FactionId) -> bool {
    state.armies.values().any(|a| {
        &a.faction == vassal && a.host_service.as_ref().is_some_and(|s| &s.liege == liege)
    })
}

/// Where the contingent musters: the vassal's capital city, else any place it holds.
fn muster_place(state: &CampaignState, vassal: &FactionId) -> Option<data_model::SettlementId> {
    state.faction_capital_city(vassal).cloned().or_else(|| {
        state
            .settlements
            .iter()
            .find(|(_, s)| &s.controller == vassal)
            .map(|(id, _)| id.clone())
    })
}

/// Raises the contingent of `vassal` for the host of `liege` against `enemy`. `None` when the
/// vassal has nothing to raise, no place to muster or already has a contingent.
fn raise_contingent(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
    enemy: &FactionId,
) -> Option<ArmyId> {
    if has_contingent(state, vassal, liege) {
        return None;
    }
    let units = contingent_units(state, data, vassal);
    if units.is_empty() {
        return None;
    }
    let place = muster_place(state, vassal)?;
    let men: u32 = units.iter().map(|u| u.strength).sum();
    let mut army = Army::new(vassal.clone(), ArmyPosition::Settlement(place), units);
    army.host_service = Some(HostService {
        liege: liege.clone(),
        enemy: enemy.clone(),
        seasons: 0,
        paid: false,
    });
    let id = state.allocate_army_id();
    state.armies.insert(id.clone(), army);
    let text = format!(
        "{} lève {men} hommes pour l'ost de son suzerain {} contre {}.",
        data.faction_name(vassal),
        data.faction_name(liege),
        data.faction_name(enemy)
    );
    state.push_order_event(
        GameEvent::new(EventKind::Diplomacy, text)
            .faction(vassal)
            .army(&id),
    );
    Some(id)
}

/// Summons the direct vassals of `liege` against `enemy`. With `resummon`, a vassal already at
/// war with `enemy` (it answered earlier) is called again to raise a new contingent. Returns the
/// number of vassals reached by the summons.
pub(super) fn summon(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    enemy: &FactionId,
    resummon: bool,
) -> usize {
    let mut reached = 0;
    for vassal in direct_vassals(state, data, liege) {
        let at_war = state.is_at_war(&vassal, enemy);
        if &vassal == enemy || state.is_allied(&vassal, enemy) || (at_war && !resummon) {
            continue;
        }
        if at_war && has_contingent(state, &vassal, liege) {
            continue;
        }
        if !can_serve(state, data, &vassal, liege, enemy) {
            continue; // out of reach or independent in fact: not summoned (ADR 0114)
        }
        reached += 1;
        if answers_host(state, data, &vassal, liege, enemy) {
            if !at_war {
                state.start_war(&vassal, enemy);
                let text = format!(
                    "{} répond à l'ost de son suzerain {} contre {}.",
                    data.faction_name(&vassal),
                    data.faction_name(liege),
                    data.faction_name(enemy)
                );
                state.push_order_event(
                    GameEvent::new(EventKind::WarDeclared, text).faction(&vassal),
                );
            }
            raise_contingent(state, data, &vassal, liege, enemy);
        } else {
            let text = format!(
                "{} refuse l'ost de son suzerain {}.",
                data.faction_name(&vassal),
                data.faction_name(liege)
            );
            state.push_order_event(GameEvent::new(EventKind::Diplomacy, text).faction(&vassal));
            felony::on_host_refused(state, data, &vassal, liege);
        }
    }
    reached
}

/// `Order::SummonHost`: see [`super::order_summon_host`].
pub(super) fn order_summon(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    enemy: &FactionId,
) -> Result<(), super::FeudalError> {
    use super::FeudalError;
    if !state.factions.get(enemy).is_some_and(|f| f.alive) {
        return Err(FeudalError::DeadFaction(enemy.clone()));
    }
    if !state.is_at_war(liege, enemy) {
        return Err(FeudalError::HostNoWar(enemy.clone()));
    }
    if let Some(last) = state.feudal.host_summoned.get(liege) {
        let ready = last + data.feudal_rules.host.resummon_delay_seasons;
        if state.turn < ready {
            return Err(FeudalError::HostTooSoon(ready - state.turn));
        }
    }
    if direct_vassals(state, data, liege).is_empty() {
        return Err(FeudalError::HostNoVassal);
    }
    summon(state, data, liege, enemy, true);
    let turn = state.turn;
    state.feudal.host_summoned.insert(liege.clone(), turn);
    Ok(())
}

/// Seasonal pay of the suzerain for a contingent (livres).
fn wage(data: &GameData, army: &Army) -> i64 {
    let upkeep: i64 = army
        .units
        .iter()
        .map(|u| season_upkeep(data, &u.unit_type))
        .sum();
    upkeep * i64::from(data.feudal_rules.host.wage_percent) / 100
}

/// End-of-season life of the contingents: free service, then pay or the way home.
pub fn resolve_host_service(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.feudal_rules.host;
    let ids: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| a.host_service.is_some())
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        let Some(army) = state.armies.get(&id) else {
            continue;
        };
        let Some(service) = army.host_service.clone() else {
            continue;
        };
        let vassal = army.faction.clone();
        let alive = |f: &FactionId| state.factions.get(f).is_some_and(|x| x.alive);
        let war_on = alive(&vassal)
            && alive(&service.liege)
            && state.is_at_war(&vassal, &service.enemy)
            && state.is_at_war(&service.liege, &service.enemy);
        let in_battle = state
            .pending_battles
            .iter()
            .any(|b| b.attacker == id || b.defender == id);
        let name = data.faction_name(&vassal);
        let mut home = !war_on;
        let mut note = None;
        if war_on && !service.paid {
            let seasons = service.seasons + 1;
            if seasons >= rules.free_seasons {
                let due = wage(data, army);
                if rules.paid_seasons > 0
                    && state
                        .factions
                        .get(&service.liege)
                        .is_some_and(|f| f.treasury >= due)
                {
                    note = Some(format!(
                        "Le contingent de {name} reste à la solde de {}.",
                        data.faction_name(&service.liege)
                    ));
                    if let Some(s) = state
                        .armies
                        .get_mut(&id)
                        .and_then(|a| a.host_service.as_mut())
                    {
                        s.paid = true;
                        s.seasons = 0;
                    }
                } else {
                    home = true;
                }
            } else if let Some(s) = state
                .armies
                .get_mut(&id)
                .and_then(|a| a.host_service.as_mut())
            {
                s.seasons = seasons;
            }
        } else if war_on {
            let due = wage(data, army);
            let can_pay = state
                .factions
                .get(&service.liege)
                .is_some_and(|f| f.treasury >= due);
            if service.seasons >= rules.paid_seasons || !can_pay {
                home = true;
            } else {
                if let Some(f) = state.factions.get_mut(&service.liege) {
                    f.treasury -= due;
                }
                if let Some(f) = state.factions.get_mut(&vassal) {
                    f.treasury += due;
                }
                if let Some(s) = state
                    .armies
                    .get_mut(&id)
                    .and_then(|a| a.host_service.as_mut())
                {
                    s.seasons += 1;
                }
            }
        }
        if home && !in_battle {
            state.armies.remove(&id);
            events.push(
                GameEvent::new(
                    EventKind::Diplomacy,
                    format!("Le contingent de {name} rentre chez lui."),
                )
                .faction(&vassal),
            );
        } else if let Some(text) = note {
            events.push(GameEvent::new(EventKind::Diplomacy, text).faction(&vassal));
        }
    }
}
