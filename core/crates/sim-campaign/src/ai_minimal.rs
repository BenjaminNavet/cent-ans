//! Minimal campaign AI (spec § 4).
//!
//! Per faction and per turn: recruit the cheapest available unit in the
//! capital's city when affordable; fold surplus capital garrison into the
//! army standing there; send every idle army to the best weakly defended
//! hostile settlement in `Siege` stance (cities first, then the others by
//! weight and fortification, lot C4), otherwise towards the most threatened
//! friendly settlement.
//!
//! Lives here rather than in the `ai` crate because `ai` depends on this crate;
//! `ai::plan_turn` re-exports [`plan_turn`].
//!
//! Lot M2 (mechanical adaptation, the real grid AI is lot M3): the plans
//! are still made on the settlement graph from each army's anchor (its
//! settlement, or the nearest one in the field); the orders are `MoveArmy`
//! towards the chosen settlement, walked on the navigation grid.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData, SettlementId};

use crate::movement::{dijkstra, edges, points_per_step};
use crate::orders::Order;
use crate::state::{ArmyId, CampaignState, Stance};

/// Maximum path cost the AI considers for an offensive, in province steps
/// (times `MovementRules::points_per_step`).
pub const OFFENSIVE_RANGE: u32 = 4;
/// Preference (in province steps) for a hostile city over another settlement.
pub const CITY_PREFERENCE_STEPS: f64 = 2.0;
/// Preference (in province steps) for a place the faction owns de jure
/// but an enemy holds (lot C7a: win back lost places).
pub const RECLAIM_PREFERENCE_STEPS: f64 = 1.5;
/// The AI attacks when its power exceeds the defence by this factor.
pub const ATTACK_SUPERIORITY: f64 = 1.5;
/// Garrison units kept in the capital before the surplus joins the field army.
pub const CAPITAL_GARRISON_KEEP: usize = 4;

/// Orders of `faction` for this turn (pure: reads the state, never mutates it).
pub fn plan_turn(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let Some(faction_state) = state.factions.get(faction) else {
        return Vec::new();
    };
    if !faction_state.alive {
        return Vec::new();
    }
    // Diplomacy first (M5): peace, alliances, wars change what the armies do.
    let mut orders = crate::diplomacy::plan_diplomacy(state, data, faction);
    let capital = faction_state.capital.clone();
    let capital_city = state.province_city_id(&capital).cloned();

    // Research (M6): pick the cheapest available technology when idle.
    if let Some(technology) = crate::research::ai_choose_research(state, data, faction) {
        orders.push(Order::Research { technology });
    }
    // The table (H3).
    orders.extend(crate::table::ai_choose_diets(state, data, faction));
    // Coinage (H5).
    orders.extend(crate::coinage::ai_choose_coinage(state, data, faction));
    // Ransoms (H6).
    orders.extend(crate::ransom::ai_ransom_orders(state, data, faction));
    // Chivalric order (H6).
    orders.extend(crate::chivalry::ai_found_order(state, data, faction));

    // In debt and still losing money: dismiss the most expensive field unit.
    if faction_state.treasury < 0 && faction_state.income_last_turn < faction_state.upkeep_last_turn
    {
        if let Some(order) = disband_most_expensive(state, data, faction) {
            orders.push(order);
        }
    }

    // Recruit the cheapest affordable unit in the capital's city.
    if let Some(city) = capital_city
        .as_ref()
        .filter(|_| state.holds_province(faction, &capital))
    {
        let cheapest = state
            .recruitable(data, city)
            .into_iter()
            .filter(|o| o.available && i64::from(o.cost) < faction_state.treasury)
            .min_by_key(|o| (o.cost, o.unit_type.clone()));
        if let Some(option) = cheapest {
            orders.push(Order::Recruit {
                settlement: city.into(),
                unit_type: option.unit_type,
            });
        }
    }

    let own_armies: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == faction)
        .map(|(id, _)| id.clone())
        .collect();

    // Surplus capital garrison joins the army standing in the capital.
    if let Some((city, garrison_len)) = capital_city.as_ref().and_then(|city| {
        state
            .settlements
            .get(city)
            .filter(|s| &s.controller == faction)
            .map(|s| (city, s.garrison.len()))
    }) {
        if garrison_len > CAPITAL_GARRISON_KEEP {
            if let Some(target) = own_armies.iter().find(|id| state.armies[*id].is_at(city)) {
                orders.push(Order::CreateArmy {
                    settlement: city.into(),
                    units_from_garrison: (CAPITAL_GARRISON_KEEP..garrison_len).collect(),
                    general: None,
                });
                orders.push(Order::MergeArmies {
                    source: state.peek_next_army_id(),
                    target: target.clone(),
                });
            }
        }
    }

    // Lot M2: where every army stands on the settlement graph.
    let anchors: BTreeMap<ArmyId, SettlementId> = state
        .armies
        .iter()
        .filter_map(|(id, a)| Some((id.clone(), state.army_anchor(data, a)?)))
        .collect();
    for army_id in own_armies {
        let army = &state.armies[&army_id];
        if !army.planned_path.is_empty() {
            continue;
        }
        let Some(anchor) = anchors.get(&army_id).cloned() else {
            continue;
        };
        let power = state.army_power(data, &army_id);
        let step = points_per_step(data);
        let range = (f64::from(OFFENSIVE_RANGE) * step).round() as u32;
        let cap = state.army_movement_allowance(data, army);
        let table = dijkstra(state, data, faction, &anchor, Some(range), Some(cap));

        // Offensive: the best weakly defended hostile settlement, cities
        // first, then the others by weight and fortification.
        let target = table
            .iter()
            .filter(|(id, _)| state.is_hostile_settlement(faction, id))
            .filter(|(id, _)| {
                state.settlement_defensive_power(data, id) * ATTACK_SUPERIORITY < power
            })
            .map(|(id, reach)| {
                let mut score = target_score(state, data, id, reach.cost, step);
                if state
                    .settlements
                    .get(id)
                    .is_some_and(|s| &s.owner == faction)
                {
                    score -= RECLAIM_PREFERENCE_STEPS;
                }
                (score, id)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)))
            .map(|(_, id)| id.clone());
        if let Some(target) = target {
            if army.is_at(&target) {
                if army.stance != Stance::Siege {
                    orders.push(Order::SetStance {
                        army: army_id.clone(),
                        stance: Stance::Siege,
                    });
                }
                continue;
            }
            orders.push(Order::SetStance {
                army: army_id.clone(),
                stance: Stance::Siege,
            });
            orders.push(Order::move_to(army_id.clone(), target));
            continue;
        }

        // Keep an ongoing siege.
        if army
            .settlement()
            .and_then(|s| state.settlements.get(s))
            .is_some_and(|s| s.siege.as_ref().is_some_and(|s| &s.attacker == faction))
        {
            continue;
        }

        // Defence: the most threatened friendly settlement within reach.
        let threatened = table
            .iter()
            .filter(|(id, _)| state.is_friendly_settlement(faction, id))
            .filter(|(id, _)| threat_at(state, data, &anchors, faction, id) > 0.0)
            .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
            .map(|(id, _)| id.clone());
        let place = army.settlement();
        match threatened {
            Some(target) if !army.is_at(&target) => {
                if army.stance != Stance::Normal {
                    orders.push(Order::SetStance {
                        army: army_id.clone(),
                        stance: Stance::Normal,
                    });
                }
                orders.push(Order::move_to(army_id.clone(), target));
            }
            // Lot C7a: an idle army outside friendly places goes home (lot
            // M2: an army in the field too).
            _ if place.is_none_or(|p| {
                !state.is_friendly_settlement(faction, p) && !state.is_hostile_settlement(faction, p)
            }) =>
            {
                let home = table
                    .iter()
                    .filter(|(id, _)| {
                        state
                            .settlements
                            .get(*id)
                            .is_some_and(|s| &s.controller == faction)
                            && state.hostile_armies_at(faction, id).is_empty()
                    })
                    .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
                    .map(|(id, _)| id.clone());
                if let Some(home) = home.filter(|h| !army.is_at(h)) {
                    orders.push(Order::move_to(army_id.clone(), home));
                }
            }
            _ => {
                if army.stance != Stance::Normal
                    && !place.is_some_and(|p| state.is_hostile_settlement(faction, p))
                {
                    orders.push(Order::SetStance {
                        army: army_id.clone(),
                        stance: Stance::Normal,
                    });
                }
            }
        }
    }
    orders
}

/// Dismisses the costliest unit of the largest army (never its last unit), or a
/// garrison unit when no army can spare one.
fn disband_most_expensive(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<Order> {
    let upkeep =
        |unit: &crate::state::Unit| data.unit_types.get(&unit.unit_type).map_or(0, |t| t.upkeep);
    let army = state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == faction && a.units.len() > 1)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())));
    if let Some((id, army)) = army {
        let (index, _) = army
            .units
            .iter()
            .enumerate()
            .max_by_key(|(i, u)| (upkeep(u), std::cmp::Reverse(*i)))?;
        return Some(Order::DisbandUnit {
            army: Some(id.clone()),
            settlement: None,
            unit_index: index,
        });
    }
    let (settlement_id, settlement) = state
        .settlements
        .iter()
        .filter(|(_, s)| &s.controller == faction && s.garrison.len() > 1)
        .max_by_key(|(id, s)| (s.garrison.len(), std::cmp::Reverse((*id).clone())))?;
    let (index, _) = settlement
        .garrison
        .iter()
        .enumerate()
        .max_by_key(|(i, u)| (upkeep(u), std::cmp::Reverse(*i)))?;
    Some(Order::DisbandUnit {
        army: None,
        settlement: Some(settlement_id.into()),
        unit_index: index,
    })
}

/// Lower is better: path cost in steps, minus a bonus for a city
/// ([`CITY_PREFERENCE_STEPS`]) and for the settlement's weight, plus half a
/// step per fortification level.
pub fn target_score(
    state: &CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    cost: u32,
    step: f64,
) -> f64 {
    let is_city = state
        .settlement_province(settlement)
        .and_then(|p| state.province_city_id(p))
        == Some(settlement);
    let weight = crate::settlements::weight_share(data, settlement);
    let fortification = f64::from(state.fortification_level(data, settlement));
    f64::from(cost) / step.max(1.0)
        - if is_city { CITY_PREFERENCE_STEPS } else { 0.0 }
        - 2.0 * weight
        + 0.5 * fortification
}

/// Strength of hostile armies anchored on or one edge away from
/// `settlement` (lot M2: an army in the field is anchored on the nearest
/// settlement).
fn threat_at(
    state: &CampaignState,
    data: &GameData,
    anchors: &BTreeMap<ArmyId, SettlementId>,
    faction: &FactionId,
    settlement: &SettlementId,
) -> f64 {
    let mut places = vec![settlement.clone()];
    places.extend(edges(data, settlement).into_iter().map(|(n, _)| n));
    anchors
        .iter()
        .filter(|(_, anchor)| places.contains(anchor))
        .filter_map(|(id, _)| state.armies.get(id))
        .filter(|a| state.is_at_war(faction, &a.faction))
        .map(|a| f64::from(a.total_strength()))
        .sum()
}

