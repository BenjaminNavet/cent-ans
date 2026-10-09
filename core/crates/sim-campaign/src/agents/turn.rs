//! End of turn: walking, resolution and season start.

use super::*;

pub(super) fn failure_text(
    data: &GameData,
    agent: &Agent,
    spec: &ActionSpec,
    plan: &ActionPlan,
    state: &CampaignState,
) -> String {
    let place = data.settlement_name(&plan.target);
    let extra = fill(
        &spec.failure_extra_fr,
        &[
            ("cost", &plan.cost.to_string()),
            ("target", &data.faction_name(&plan.target_faction)),
            (
                "captive",
                &plan
                    .character
                    .as_ref()
                    .map_or_else(String::new, |c| state.character_name(data, c)),
            ),
        ],
    );
    format!(
        "{} échoue à {} ({place}, {} % de chances){extra}.",
        agent.name,
        spec.name.to_lowercase(),
        plan.chance
    )
}

/// Fills the `{key}` placeholders of a French template.
pub(super) fn fill(template: &str, vars: &[(&str, &str)]) -> String {
    vars.iter().fold(template.to_owned(), |text, (key, value)| {
        text.replace(&format!("{{{key}}}"), value)
    })
}

/// Walks `id` along the path to its destination while its points allow.
pub(super) fn walk_agent(state: &mut CampaignState, data: &GameData, id: &AgentId) {
    let Some(agent) = state.agents.agents.get(id) else {
        return;
    };
    let Some(target) = agent.destination.clone() else {
        return;
    };
    let cap = state.agent_movement_allowance(data, agent.kind);
    let Some(path) = state.agent_find_path(data, id, &target) else {
        state.agents.agents.get_mut(id).expect("exists").destination = None;
        return;
    };
    let agent = state.agents.agents.get_mut(id).expect("exists");
    for next in path {
        let Some(cost) = movement::edge_cost(data, &agent.location, &next) else {
            break;
        };
        let cost = cost.min(cap.max(1));
        if cost > agent.movement_points {
            break;
        }
        agent.movement_points -= cost;
        agent.location = next;
    }
    if agent.location == target {
        agent.destination = None;
    }
}

// ----- end of turn ------------------------------------------------------------

/// Phase of `end_turn` (after religion): upkeep, dead factions, stale
/// intelligence, passive counter-espionage.
pub(crate) fn resolve_agents(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules(data).clone();
    // Agents of dead factions vanish.
    let factions = &state.factions;
    state
        .agents
        .agents
        .retain(|_, a| factions.get(&a.faction).is_some_and(|f| f.alive));
    let turn = state.turn;
    state.agents.intel.retain(|i| i.until_turn > turn);
    // Upkeep.
    let mut upkeep: BTreeMap<FactionId, i64> = BTreeMap::new();
    for agent in state.agents.agents.values() {
        let base = rules.types.get(&agent.kind).map_or(0, |t| t.upkeep);
        *upkeep.entry(agent.faction.clone()).or_default() +=
            crate::coinage::priced(state, &agent.faction, i64::from(base));
    }
    for (faction, amount) in &upkeep {
        if let Some(f) = state.factions.get_mut(faction) {
            f.treasury -= amount;
        }
    }
    state.agents.upkeep_last_turn = upkeep;
    // Passive counter-espionage: foreign spies where the master has a spy.
    let mut caught: Vec<(AgentId, ProvinceId, FactionId)> = Vec::new();
    for (id, agent) in &state.agents.agents {
        if agent.kind != AgentKind::Spy {
            continue;
        }
        let Some(province) = state.settlement_province(&agent.location).cloned() else {
            continue;
        };
        let Some(master) = state.province_controller(&province).cloned() else {
            continue;
        };
        if state.is_allied(&agent.faction, &master) {
            continue;
        }
        let best = state
            .agents_in_province(&province)
            .filter(|(_, a)| a.kind == AgentKind::Spy && a.faction == master)
            .map(|(_, a)| a.level)
            .max();
        let Some(best) = best else {
            continue;
        };
        let chance = (rules.passive_counter_base
            + rules.passive_counter_per_level * (i32::from(best) - i32::from(agent.level)))
        .clamp(0, rules.passive_counter_cap) as u32;
        let mut rng = derived_rng(state.seed, turn, id.index(), 0x00FF_0000);
        if rng.below(100) < chance {
            caught.push((id.clone(), province, master));
        }
    }
    let player = state.player_faction.clone();
    for (id, province, master) in caught {
        let Some(victim) = state.agents.agents.remove(&id) else {
            continue;
        };
        if victim.faction == player || master == player {
            let text = if victim.faction == player {
                format!(
                    "{} est démasqué par les espions {} et pendu.",
                    victim.name,
                    crate::events::de(&data.faction_name(&master))
                )
            } else {
                format!(
                    "Vos espions démasquent {}, agent {}, qui est pendu.",
                    victim.name,
                    crate::events::de(&data.faction_name(&victim.faction))
                )
            };
            events.push(
                GameEvent::new(EventKind::Agent, text)
                    .province(&province)
                    .faction(&victim.faction),
            );
        }
    }
}

/// Start of a season (after the date advanced): fresh movement points, the
/// action back, and the march towards the destination resumes.
pub(crate) fn start_season(state: &mut CampaignState, data: &GameData) {
    let ids: Vec<AgentId> = state.agents.agents.keys().cloned().collect();
    for id in ids {
        let kind = state.agents.agents[&id].kind;
        let points = state.agent_movement_allowance(data, kind);
        let agent = state.agents.agents.get_mut(&id).expect("exists");
        agent.movement_points = points;
        agent.acted = false;
        walk_agent(state, data, &id);
    }
}
