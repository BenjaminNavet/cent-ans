//! AI use of the WH agent actions (ADR 0300): assassination and poison
//! against the most dangerous enemy general, ambush of a marching enemy
//! army, guiding a marching friendly army. All thresholds live in
//! `data/rules/agents.json` (`ai_agents`); every roll is derived from
//! (seed, turn, agent), so the main random stream is never consumed.

use super::*;
use crate::state::Army;

/// Salt of the AI's agent rolls (never renumber: saved-game reproducibility).
const SALT: u32 = 0xA1_0300;

/// Orders of a spy or herald that is going to use one of the WH actions this
/// season, `None` when it should behave as usual.
pub(super) fn plan_special(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    id: &AgentId,
    agent: &Agent,
) -> Option<Vec<Order>> {
    let state = cache.state();
    let ai = &rules(data).ai_agents;
    let treasury = state.factions.get(faction)?.treasury;
    let place = state.settlements.get(&agent.location)?;
    let scale = scale_percent(state, data);
    // One roll, shared by the actions of one agent: each action takes a
    // slice of [0, 100), so they never fire together.
    let mut rng = derived_rng(state.seed, state.turn, id.index(), SALT);
    let roll = rng.below(100);
    let chance = |base: u32| base * scale / 100;
    let affordable = |action, character: Option<&CharacterId>, min_odds: u32| {
        let (odds, cost) = state
            .agent_action_odds(data, id, action, None, character)
            .ok()?;
        (odds >= min_odds && treasury - cost >= ai.reserve).then_some(())
    };
    let go = |action, character: Option<CharacterId>| {
        Some(vec![Order::AgentAction {
            agent: id.clone(),
            action,
            target: None,
            character,
        }])
    };
    match agent.kind {
        AgentKind::Spy => {
            let assassinate = chance(ai.assassinate_chance);
            let poison = chance(ai.poison_chance);
            let ambush = chance(ai.ambush_chance);
            if roll < assassinate + poison && strikes_left(state, data, faction) {
                let victim = most_dangerous(state, data, agent, &place.controller)?;
                let action = if roll < assassinate {
                    AgentActionKind::Assassinate
                } else {
                    AgentActionKind::Poison
                };
                affordable(action, Some(&victim), ai.min_strike_odds)?;
                return go(action, Some(victim));
            }
            if roll >= assassinate + poison
                && roll < assassinate + poison + ambush
                && marching_army_near(state, data, &agent.location, |a| {
                    state.is_at_war(faction, &a.faction)
                        && a.total_strength() >= ai.min_ambush_strength
                })
            {
                affordable(AgentActionKind::Ambush, None, ai.min_support_odds)?;
                return go(AgentActionKind::Ambush, None);
            }
            None
        }
        AgentKind::Emissary => {
            if roll < chance(ai.guide_chance)
                && marching_army_near(state, data, &agent.location, |a| {
                    state.is_allied(faction, &a.faction)
                        && a.total_strength() >= ai.min_guide_strength
                        && state
                            .army_province(data, a)
                            .is_some_and(|p| !state.is_friendly_territory(faction, &p))
                })
            {
                affordable(AgentActionKind::GuideArmy, None, ai.min_support_odds)?;
                return go(AgentActionKind::GuideArmy, None);
            }
            None
        }
        AgentKind::Preacher | AgentKind::Merchant => None,
    }
}

/// Percent by which the difficulty scales the AI's chances (100: neutral).
fn scale_percent(state: &CampaignState, data: &GameData) -> u32 {
    let delta = state.difficulty_modifiers(data).ai_aggression_delta;
    (100 + delta * rules(data).ai_agents.aggression_percent_per_point).max(0) as u32
}

/// `false` once the faction used up its strikes of the sliding window.
fn strikes_left(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    let ai = &rules(data).ai_agents;
    let recent = state.agents.strike_log.get(faction).map_or(0, |log| {
        log.iter()
            .filter(|t| state.turn.saturating_sub(**t) < ai.window_turns)
            .count()
    });
    (recent as u32) < ai.strike_cap
}

/// Some army satisfying `keep`, marching, within reach of `place`.
fn marching_army_near(
    state: &CampaignState,
    data: &GameData,
    place: &SettlementId,
    keep: impl Fn(&Army) -> bool,
) -> bool {
    !state
        .armies_in_reach(data, place, |a| a.destination.is_some() && keep(a))
        .is_empty()
}

/// The enemy general in reach of `place` leading the strongest army (at
/// least `min_general_strength`), else, if the rules allow it, the governor
/// of the province.
fn most_dangerous(
    state: &CampaignState,
    data: &GameData,
    agent: &Agent,
    master: &FactionId,
) -> Option<CharacterId> {
    let place = &agent.location;
    let ai = &rules(data).ai_agents;
    let ruler_min_level = rules(data)
        .actions
        .get(&AgentActionKind::Assassinate)
        .and_then(|a| a.target_ruler_min_level);
    let general = state
        .armies_in_reach(data, place, |a| &a.faction == master)
        .into_iter()
        .filter_map(|id| state.armies.get(&id))
        .filter(|a| a.total_strength() >= ai.min_general_strength)
        .filter_map(|a| {
            let general = a.general.clone()?;
            let alive = state
                .characters
                .get(&general)
                .is_some_and(|c| c.alive && !c.captive);
            let is_ruler =
                state.factions.get(master).and_then(|f| f.ruler.as_ref()) == Some(&general);
            (alive && (!is_ruler || ruler_min_level.is_some_and(|min| agent.level >= min)))
                .then_some((a.total_strength(), general))
        })
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
        .map(|(_, general)| general);
    if general.is_some() || !ai.strike_governors {
        return general;
    }
    let province = state.settlement_province(place)?;
    state
        .characters
        .iter()
        .find(|(_, c)| {
            c.alive
                && !c.captive
                && &c.faction == master
                && c.governor_of.as_ref() == Some(province)
        })
        .map(|(id, _)| id.clone())
}
