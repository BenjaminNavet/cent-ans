//! Agent AI.

use super::*;

// ----- AI -----------------------------------------------------------------------

/// Agent orders of an AI faction (called by `ai::plan_turn`): see the design § 10.
pub fn plan_agents(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let state = cache.state();
    let Some(f) = state.factions.get(faction) else {
        return Vec::new();
    };
    if !f.alive || faction.as_str() == REBELS_FACTION || faction.as_str() == PAPACY_FACTION {
        return Vec::new();
    }
    // F8: a playable faction too poor for a network of agents (their
    // upkeep) behaves like a minor power.
    let playable = data.factions.get(faction).is_some_and(|d| d.playable)
        && cache.faction_income(data, faction) >= rules(data).ai_network_min_income;
    let mut orders = Vec::new();
    // Recruitment: one agent of each missing kind, one per season.
    if playable || f.treasury > 2 * AI_RECRUIT_RESERVE {
        for kind in AgentKind::ALL {
            if state.count_agents(faction, kind) > 0 {
                continue;
            }
            // Minor powers keep a single spy, and only while at war.
            if !playable && (kind != AgentKind::Spy || f.at_war_with.is_empty()) {
                continue;
            }
            if kind == AgentKind::Preacher && !heresy_or_schism(state, faction) {
                continue;
            }
            let t = &rules(data).types[&kind];
            let cost = crate::coinage::priced(state, faction, i64::from(t.cost));
            if f.treasury < AI_RECRUIT_RESERVE + cost {
                break;
            }
            let place = ai_recruit_place(state, data, faction, kind);
            if let Some(settlement) = place {
                orders.push(Order::RecruitAgent {
                    settlement: settlement.into(),
                    kind,
                });
                break;
            }
        }
    }
    // OMR R3 (ADR 0117): a realm in debt dismisses its costliest agent each
    // season (their upkeep kept small counties in the red).
    let dismissed = (f.treasury < 0)
        .then(|| {
            state
                .agents_of(faction)
                .into_iter()
                .max_by_key(|(id, a)| {
                    let upkeep = rules(data).types.get(&a.kind).map_or(0, |t| t.upkeep);
                    (upkeep, std::cmp::Reverse((*id).clone()))
                })
                .map(|(id, _)| id.clone())
        })
        .flatten();
    if let Some(agent) = &dismissed {
        orders.push(Order::DismissAgent {
            agent: agent.clone(),
        });
    }
    for (id, agent) in state.agents_of(faction) {
        if agent.acted || dismissed.as_ref() == Some(id) {
            continue;
        }
        // WR ai-agents (ADR 0300): strikes, ambushes and guides, rare.
        if let Some(special) = super::ai_strikes::plan_special(cache, data, faction, id, agent) {
            orders.extend(special);
            continue;
        }
        let plan = match agent.kind {
            AgentKind::Spy => ai_spy(state, data, faction, id, agent),
            AgentKind::Emissary => ai_emissary(cache, data, faction, id, agent),
            AgentKind::Preacher => ai_preacher(state, data, faction, id, agent),
            AgentKind::Merchant => ai_merchant(state, data, faction, id, agent),
        };
        orders.extend(plan);
    }
    orders
}

/// A preacher is worth his upkeep against a heresy at home or during the Schism.
fn heresy_or_schism(state: &CampaignState, faction: &FactionId) -> bool {
    state.schism
        || state
            .controlled_provinces(faction)
            .any(|p| state.provinces.get(p).is_some_and(|p| p.heresy > 0))
}

fn ai_recruit_place(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    kind: AgentKind,
) -> Option<SettlementId> {
    let capital = state.factions.get(faction).map(|f| f.capital.clone());
    let capital_city = capital
        .as_ref()
        .and_then(|p| state.province_city_id(p))
        .cloned();
    if let Some(city) = capital_city {
        if state.check_recruit(data, faction, &city, kind).is_ok() {
            return Some(city);
        }
    }
    state
        .settlements
        .keys()
        .find(|id| state.check_recruit(data, faction, id, kind).is_ok())
        .cloned()
}

/// Act when an action is worth it, else walk towards `goal` and try `then`
/// if the goal is within reach this season.
fn act_or_walk(
    state: &CampaignState,
    data: &GameData,
    id: &AgentId,
    agent: &Agent,
    act_now: Option<(AgentActionKind, Option<SettlementId>, Option<CharacterId>)>,
    goal: Option<(SettlementId, AgentActionKind)>,
) -> Vec<Order> {
    if let Some((action, target, character)) = act_now {
        return vec![Order::AgentAction {
            agent: id.clone(),
            action,
            target,
            character,
        }];
    }
    let Some((goal, then)) = goal else {
        return Vec::new();
    };
    if goal == agent.location
        || agent.destination.as_ref() == Some(&goal) && agent.movement_points == 0
    {
        return Vec::new();
    }
    let mut orders = vec![Order::MoveAgent {
        agent: id.clone(),
        target: goal.clone(),
    }];
    if state.agent_reachable(data, id).contains_key(&goal) {
        orders.push(Order::AgentAction {
            agent: id.clone(),
            action: then,
            target: Some(goal),
            character: None,
        });
    }
    orders
}

fn ai_spy(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    id: &AgentId,
    agent: &Agent,
) -> Vec<Order> {
    let odds = |action| state.agent_action_odds(data, id, action, None, None).ok();
    let here = state.settlements.get(&agent.location);
    let besieged = here.is_some_and(|s| s.siege.is_some());
    let mut act = None;
    if besieged && odds(AgentActionKind::Sabotage).is_some_and(|(c, _)| c >= 25) {
        act = Some((AgentActionKind::Sabotage, None, None));
    } else if let Some(province) = state.settlement_province(&agent.location) {
        let known = state.agents.intel.iter().any(|i| {
            &i.faction == faction && &i.province == province && i.until_turn > state.turn + 1
        });
        if !known && odds(AgentActionKind::Scout).is_some_and(|(c, _)| c >= 40) {
            act = Some((AgentActionKind::Scout, None, None));
        } else if odds(AgentActionKind::Incite).is_some_and(|(c, _)| c >= 35) {
            act = Some((AgentActionKind::Incite, None, None));
        } else if odds(AgentActionKind::Sabotage).is_some_and(|(c, _)| c >= 35) {
            act = Some((AgentActionKind::Sabotage, None, None));
        }
    }
    // Goal: the nearest enemy city not yet under our eyes.
    let goal = if act.is_none() {
        nearest_city(state, data, agent, |state, city, province| {
            let controller = state.settlements.get(city).map(|s| &s.controller);
            controller.is_some_and(|c| state.is_at_war(faction, c))
                && !state
                    .agents
                    .intel
                    .iter()
                    .any(|i| &i.faction == faction && &i.province == province)
                && &agent.location != city
        })
        .map(|g| (g, AgentActionKind::Scout))
    } else {
        None
    };
    act_or_walk(state, data, id, agent, act, goal)
}

fn ai_emissary(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    id: &AgentId,
    agent: &Agent,
) -> Vec<Order> {
    let state = cache.state();
    let odds = |action| state.agent_action_odds(data, id, action, None, None).ok();
    // 1. Buy back a captive: only one held for money that the treasury can
    // pay for (else steps 2 and 3, rather than waiting at his captor's).
    let treasury = state.factions.get(faction).map_or(0, |f| f.treasury);
    let action_cost = rules(data)
        .actions
        .get(&AgentActionKind::Ransom)
        .map_or(0, |r| i64::from(r.cost.flat));
    let action_cost = crate::coinage::priced(state, faction, action_cost);
    let captor = state
        .characters
        .iter()
        .find(|(cid, c)| {
            c.alive
                && c.captive
                && &c.faction == faction
                && c.captor.is_some()
                && herald_may_ransom(state, cid)
                && ransom_price(state, data, cid, agent.level) + action_cost <= treasury
        })
        .and_then(|(_, c)| c.captor.clone());
    if let Some(captor) = captor {
        if odds(AgentActionKind::Ransom).is_some() {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                Some((AgentActionKind::Ransom, None, None)),
                None,
            );
        }
        let goal = faction_city(state, data, agent, &captor);
        if let Some(goal) = goal {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                None,
                Some((goal, AgentActionKind::Ransom)),
            );
        }
    }
    // 2. A truce with an enemy that is winning.
    let losing_to = state.factions.get(faction).and_then(|f| {
        f.at_war_with
            .iter()
            .find(|enemy| {
                enemy.as_str() != REBELS_FACTION
                    && &state.player_faction != *enemy
                    && state.war_score(data, faction, enemy) < -15
            })
            .cloned()
    });
    if let Some(enemy) = losing_to {
        let here = state
            .settlements
            .get(&agent.location)
            .map(|s| s.controller.clone());
        if here.as_ref() == Some(&enemy) && odds(AgentActionKind::Truce).is_some() {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                Some((AgentActionKind::Truce, None, None)),
                None,
            );
        }
        if let Some(goal) = faction_city(state, data, agent, &enemy) {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                None,
                Some((goal, AgentActionKind::Truce)),
            );
        }
    }
    // 3. An embassy to the least friendly neighbour at peace.
    if odds(AgentActionKind::Parley).is_some() {
        return act_or_walk(
            state,
            data,
            id,
            agent,
            Some((AgentActionKind::Parley, None, None)),
            None,
        );
    }
    let neighbours = cache.neighbour_factions(data, faction);
    let neighbour = state
        .factions
        .iter()
        .filter(|(other, f)| {
            *other != faction
                && f.alive
                && other.as_str() != REBELS_FACTION
                && !state.is_at_war(faction, other)
                && !state.is_allied(faction, other)
                && neighbours.contains(*other)
                && !f.modifiers.iter().any(|m| {
                    &m.with == faction
                        && m.reason_fr == PARLEY_REASON
                        && m.expires_turn > state.turn
                })
        })
        .map(|(other, _)| (cache.attitude(data, other, faction).0, other.clone()))
        // Only courts that need winning over (C6 balance: no opinion inflation).
        .filter(|(attitude, _)| *attitude < AI_PARLEY_ATTITUDE)
        .min()
        .map(|(_, other)| other);
    if let Some(other) = neighbour {
        if let Some(goal) = faction_city(state, data, agent, &other) {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                None,
                Some((goal, AgentActionKind::Parley)),
            );
        }
    }
    Vec::new()
}

fn ai_preacher(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    id: &AgentId,
    agent: &Agent,
) -> Vec<Order> {
    let here = state.settlement_province(&agent.location).cloned();
    let here_heresy = here
        .as_ref()
        .and_then(|p| state.provinces.get(p))
        .map_or(0, |p| p.heresy);
    let preach_ok = state
        .agent_action_odds(data, id, AgentActionKind::Preach, None, None)
        .is_ok();
    if here_heresy > 0 && preach_ok {
        return act_or_walk(
            state,
            data,
            id,
            agent,
            Some((AgentActionKind::Preach, None, None)),
            None,
        );
    }
    // The most heretical friendly province.
    let target = state
        .controlled_provinces(faction)
        .filter_map(|p| state.provinces.get(p).map(|s| (p.clone(), s.heresy)))
        .filter(|(_, heresy)| *heresy > 0)
        .max_by_key(|(_, heresy)| *heresy)
        .and_then(|(p, _)| state.province_city_id(&p).cloned());
    if let Some(goal) = target {
        return act_or_walk(
            state,
            data,
            id,
            agent,
            None,
            Some((goal, AgentActionKind::Preach)),
        );
    }
    // ADR 0326: a province of another faith, to bring round.
    if data.religion_rules.is_some() {
        let converting = |p: &ProvinceId| state.conversion_goal(data, p).is_some();
        if here.as_ref().is_some_and(converting) && preach_ok {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                Some((AgentActionKind::Preach, None, None)),
                None,
            );
        }
        let goal = state
            .controlled_provinces(faction)
            .filter(|p| converting(p))
            .max_by_key(|p| state.provinces.get(*p).map_or(0, |s| s.conversion_progress))
            .and_then(|p| state.province_city_id(p).cloned());
        if let Some(goal) = goal {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                None,
                Some((goal, AgentActionKind::Preach)),
            );
        }
    }
    // Otherwise calm the most restless province where he stands.
    let restless = here
        .as_ref()
        .and_then(|p| state.provinces.get(p))
        .is_some_and(|p| p.population.peasants.unrest >= 40 || p.population.clergy.unrest >= 40);
    if restless && preach_ok {
        return act_or_walk(
            state,
            data,
            id,
            agent,
            Some((AgentActionKind::Preach, None, None)),
            None,
        );
    }
    Vec::new()
}

/// A merchant buys out a rival's post where he stands, else sets up his own,
/// else walks to the nearest friendly or neutral city without a post.
fn ai_merchant(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    id: &AgentId,
    agent: &Agent,
) -> Vec<Order> {
    if agent.post.as_ref() == Some(&agent.location) {
        return Vec::new();
    }
    let odds = |action| state.agent_action_odds(data, id, action, None, None).ok();
    let act = if odds(AgentActionKind::Outbid).is_some_and(|(c, _)| c >= 40) {
        Some((AgentActionKind::Outbid, None, None))
    } else if odds(AgentActionKind::TradePost).is_some() {
        Some((AgentActionKind::TradePost, None, None))
    } else {
        None
    };
    let goal = if act.is_none() {
        nearest_city(state, data, agent, |state, city, _| {
            &agent.location != city
                && state.settlements.get(city).is_some_and(|s| {
                    !state.is_at_war(faction, &s.controller)
                        && !state
                            .agents
                            .agents
                            .values()
                            .any(|a| a.post.as_ref() == Some(city))
                })
        })
        .map(|g| (g, AgentActionKind::TradePost))
    } else {
        None
    };
    act_or_walk(state, data, id, agent, act, goal)
}

/// Nearest city (by agent path cost) satisfying `wanted`.
fn nearest_city(
    state: &CampaignState,
    data: &GameData,
    agent: &Agent,
    wanted: impl Fn(&CampaignState, &SettlementId, &ProvinceId) -> bool,
) -> Option<SettlementId> {
    let cap = state.agent_movement_allowance(data, agent.kind);
    let table = AgentTable::search(data, &agent.location, None, cap, None);
    state
        .provinces
        .iter()
        .filter(|(province, p)| wanted(state, &p.city, province))
        .filter_map(|(_, p)| table.cost(&p.city).map(|cost| (cost, p.city.clone())))
        .min()
        .map(|(_, city)| city)
}

/// Nearest city held by `faction`.
fn faction_city(
    state: &CampaignState,
    data: &GameData,
    agent: &Agent,
    faction: &FactionId,
) -> Option<SettlementId> {
    nearest_city(state, data, agent, |state, city, _| {
        state
            .settlements
            .get(city)
            .is_some_and(|s| &s.controller == faction)
    })
}
