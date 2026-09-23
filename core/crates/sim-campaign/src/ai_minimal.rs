//! Minimal campaign AI (spec § 4).
//!
//! Per faction and per turn: recruit the cheapest available unit in the capital
//! when affordable; fold surplus capital garrison into the army standing there;
//! send every idle army to the nearest weakly defended hostile province in
//! `Siege` stance, otherwise towards the most threatened friendly province.
//!
//! Lives here rather than in the `ai` crate because `ai` depends on this crate;
//! `ai::plan_turn` re-exports [`plan_turn`].

use data_model::{FactionId, GameData, ProvinceId};

use crate::movement::{dijkstra, edges, path_to};
use crate::orders::Order;
use crate::state::{ArmyId, CampaignState, Stance};

/// Maximum path cost the AI considers for an offensive.
pub const OFFENSIVE_RANGE: u32 = 6;
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
    let mut orders = Vec::new();
    let capital = faction_state.capital.clone();

    // Research (M6): pick the cheapest available technology when idle.
    if let Some(technology) = crate::research::ai_choose_research(state, data, faction) {
        orders.push(Order::Research { technology });
    }

    // In debt and still losing money: dismiss the most expensive field unit.
    if faction_state.treasury < 0 && faction_state.income_last_turn < faction_state.upkeep_last_turn
    {
        if let Some(order) = disband_most_expensive(state, data, faction) {
            orders.push(order);
        }
    }

    // Recruit the cheapest affordable unit in the capital.
    if state
        .provinces
        .get(&capital)
        .is_some_and(|p| &p.controller == faction && &p.owner == faction)
    {
        let cheapest = state
            .recruitable(data, &capital)
            .into_iter()
            .filter(|o| o.available && i64::from(o.cost) < faction_state.treasury)
            .min_by_key(|o| (o.cost, o.unit_type.clone()));
        if let Some(option) = cheapest {
            orders.push(Order::Recruit {
                province: capital.clone(),
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
    if let Some(garrison_len) = state.provinces.get(&capital).map(|p| p.garrison.len()) {
        if garrison_len > CAPITAL_GARRISON_KEEP {
            if let Some(target) = own_armies
                .iter()
                .find(|id| state.armies[*id].location == capital)
            {
                orders.push(Order::CreateArmy {
                    province: capital.clone(),
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

    for army_id in own_armies {
        let army = &state.armies[&army_id];
        if !army.path.is_empty() {
            continue;
        }
        let power = state.army_power(data, &army_id);
        let table = dijkstra(state, data, faction, &army.location, Some(OFFENSIVE_RANGE));

        // Offensive: nearest weakly defended hostile province.
        let target = table
            .iter()
            .filter(|(id, _)| state.is_hostile_territory(faction, id))
            .filter(|(id, _)| state.defensive_power(data, id) * ATTACK_SUPERIORITY < power)
            .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
            .map(|(id, _)| id.clone());
        if let Some(target) = target {
            if target == army.location {
                if army.stance != Stance::Siege {
                    orders.push(Order::SetStance {
                        army: army_id.clone(),
                        stance: Stance::Siege,
                    });
                }
                continue;
            }
            if let Some(path) = path_to(&table, &target) {
                orders.push(Order::SetStance {
                    army: army_id.clone(),
                    stance: Stance::Siege,
                });
                orders.push(Order::MoveArmy {
                    army: army_id.clone(),
                    path,
                });
                continue;
            }
        }

        // Keep an ongoing siege.
        if state
            .provinces
            .get(&army.location)
            .is_some_and(|p| p.siege.as_ref().is_some_and(|s| &s.attacker == faction))
        {
            continue;
        }

        // Defence: the most threatened friendly province within reach.
        let threatened = table
            .iter()
            .filter(|(id, _)| state.is_friendly_territory(faction, id))
            .filter(|(id, _)| threat_at(state, data, faction, id) > 0.0)
            .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
            .map(|(id, _)| id.clone());
        match threatened {
            Some(target) if target != army.location => {
                if let Some(path) = path_to(&table, &target) {
                    if army.stance != Stance::Normal {
                        orders.push(Order::SetStance {
                            army: army_id.clone(),
                            stance: Stance::Normal,
                        });
                    }
                    orders.push(Order::MoveArmy {
                        army: army_id.clone(),
                        path,
                    });
                }
            }
            _ => {
                if army.stance != Stance::Normal
                    && !state.is_hostile_territory(faction, &army.location)
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
            province: None,
            unit_index: index,
        });
    }
    let (province_id, province) = state
        .provinces
        .iter()
        .filter(|(_, p)| &p.controller == faction && p.garrison.len() > 1)
        .max_by_key(|(id, p)| (p.garrison.len(), std::cmp::Reverse((*id).clone())))?;
    let (index, _) = province
        .garrison
        .iter()
        .enumerate()
        .max_by_key(|(i, u)| (upkeep(u), std::cmp::Reverse(*i)))?;
    Some(Order::DisbandUnit {
        army: None,
        province: Some(province_id.clone()),
        unit_index: index,
    })
}

/// Strength of hostile armies inside or one edge away from `province`.
fn threat_at(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    province: &ProvinceId,
) -> f64 {
    let inside: f64 = state
        .hostile_armies_in(faction, province)
        .into_iter()
        .map(|id| f64::from(state.armies[&id].total_strength()))
        .sum();
    let adjacent: f64 = edges(data, province)
        .into_iter()
        .flat_map(|(neighbor, _)| state.hostile_armies_in(faction, &neighbor))
        .map(|id| f64::from(state.armies[&id].total_strength()))
        .sum();
    inside + adjacent
}
