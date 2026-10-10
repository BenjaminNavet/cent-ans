//! Lot WR `ai-agents` (ADR 0300): the AI uses assassination, poison, ambush
//! and guide, rarely, never in peace, never without an agent.

use data_model::test_support::{fac, game_data};
use data_model::{
    AgentActionKind, AgentKind, BuildingCategory, FactionId, GameData, SettlementId, SettlementKind,
};
use sim_campaign::agents::{self, AgentId};
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::state::MoveTarget;
use sim_campaign::test_support::start_quiet;
use sim_campaign::{CampaignState, Order};

/// Rules where an agent facing a target always acts and every action succeeds.
fn eager(assassinate: u32, poison: u32, ambush: u32, guide: u32) -> GameData {
    let mut data = game_data().clone();
    let mut rules = agents::rules(&data).clone();
    for action in rules.actions.values_mut() {
        action.base_chance = 100;
        action.death_risk = 0;
    }
    rules.max_chance = 100;
    rules.min_chance = 100;
    let ai = &mut rules.ai_agents;
    ai.assassinate_chance = assassinate;
    ai.poison_chance = poison;
    ai.ambush_chance = ambush;
    ai.guide_chance = guide;
    ai.min_general_strength = 0;
    ai.min_ambush_strength = 0;
    ai.min_guide_strength = 0;
    ai.reserve = 0;
    ai.strike_cap = 2;
    ai.window_turns = 40;
    ai.aggression_percent_per_point = 0;
    data.agent_rules = Some(rules);
    data
}

fn war(state: &mut CampaignState, a: &FactionId, b: &FactionId) {
    for (x, y) in [(a, b), (b, a)] {
        let f = state.factions.get_mut(x).unwrap();
        f.at_war_with.insert(y.clone());
        f.allies.remove(y);
    }
}

fn place(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    kind: AgentKind,
    at: &SettlementId,
) -> AgentId {
    state.factions.get_mut(faction).unwrap().treasury += 10_000;
    let own = state
        .settlements
        .iter()
        .filter(|(_, s)| &s.controller == faction)
        .find(|(_, s)| {
            s.kind == SettlementKind::City
                && s.buildings.iter().any(|b| {
                    data.buildings
                        .get(b)
                        .is_some_and(|d| d.category == BuildingCategory::Religious)
                })
        })
        .or_else(|| {
            state
                .settlements
                .iter()
                .find(|(_, s)| &s.controller == faction && s.kind == SettlementKind::Abbey)
        })
        .map(|(id, _)| id.clone())
        .expect("a place to recruit");
    let id = state.recruit_agent(data, faction, &own, kind).unwrap();
    let agent = state.agents.agents.get_mut(&id).unwrap();
    agent.location = at.clone();
    agent.movement_points = 500;
    id
}

/// The first stationed army of `faction` led by a general, and its place.
fn stationed(state: &CampaignState, faction: &FactionId) -> (sim_campaign::ArmyId, SettlementId) {
    state
        .armies
        .iter()
        .find_map(|(id, a)| {
            (&a.faction == faction && a.general.is_some())
                .then(|| a.settlement().cloned().map(|s| (id.clone(), s)))
                .flatten()
        })
        .expect("a stationed army with a general")
}

fn actions_of(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<AgentActionKind> {
    agents::plan_agents(&PlanCache::new(state), data, faction)
        .into_iter()
        .filter_map(|o| match o {
            Order::AgentAction { action, .. } => Some(action),
            _ => None,
        })
        .collect()
}

const STRIKES: [AgentActionKind; 2] = [AgentActionKind::Assassinate, AgentActionKind::Poison];

#[test]
fn a_spy_assassinates_the_general_of_an_enemy_at_war() {
    let data = eager(100, 0, 0, 0);
    let mut state = start_quiet(&data, "fac_france", 3);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    war(&mut state, &france, &england);
    let (army, there) = english_lieutenant_army(&mut state);
    let general = state.armies[&army].general.clone().unwrap();
    place(&mut state, &data, &france, AgentKind::Spy, &there);
    let orders = agents::plan_agents(&PlanCache::new(&state), &data, &france);
    assert!(
        orders.iter().any(|o| matches!(o,
            Order::AgentAction { action: AgentActionKind::Assassinate, character: Some(c), .. } if *c == general)),
        "{orders:?}"
    );
}

#[test]
fn poison_is_the_rarer_option_of_the_same_roll() {
    let data = eager(0, 100, 0, 0);
    let mut state = start_quiet(&data, "fac_france", 3);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    war(&mut state, &france, &england);
    let (_, there) = english_lieutenant_army(&mut state);
    place(&mut state, &data, &france, AgentKind::Spy, &there);
    let actions = actions_of(&state, &data, &france);
    assert!(actions.contains(&AgentActionKind::Poison), "{actions:?}");
    assert!(!actions.contains(&AgentActionKind::Assassinate));
}

#[test]
fn no_strike_in_peace_nor_without_an_agent() {
    let data = eager(100, 100, 100, 100);
    let mut state = start_quiet(&data, "fac_france", 3);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let (_, there) = english_lieutenant_army(&mut state);
    for (x, y) in [(&france, &england), (&england, &france)] {
        state.factions.get_mut(x).unwrap().at_war_with.remove(y);
    }
    // At peace: a spy standing in an English town does nothing hostile.
    place(&mut state, &data, &france, AgentKind::Spy, &there);
    let actions = actions_of(&state, &data, &france);
    assert!(actions.iter().all(|a| !STRIKES.contains(a)), "{actions:?}");
    assert!(!actions.contains(&AgentActionKind::Ambush), "{actions:?}");
    // At war but with no agent at all.
    let mut state = start_quiet(&data, "fac_france", 3);
    war(&mut state, &france, &england);
    state.agents.agents.clear();
    let actions = actions_of(&state, &data, &france);
    assert!(actions.iter().all(|a| !STRIKES.contains(a)), "{actions:?}");
}

#[test]
fn strikes_are_capped_per_window() {
    let data = eager(100, 0, 0, 0);
    let mut state = start_quiet(&data, "fac_france", 3);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    war(&mut state, &france, &england);
    let (_, there) = english_lieutenant_army(&mut state);
    place(&mut state, &data, &france, AgentKind::Spy, &there);
    assert!(actions_of(&state, &data, &france).contains(&AgentActionKind::Assassinate));
    let turn = state.turn;
    state
        .agents
        .strike_log
        .insert(france.clone(), vec![turn, turn]);
    let capped = actions_of(&state, &data, &france);
    assert!(capped.iter().all(|a| !STRIKES.contains(a)), "{capped:?}");
    // Old strikes leave the window.
    state.turn = turn + 41;
    state
        .agents
        .strike_log
        .insert(france.clone(), vec![turn, turn]);
    assert!(actions_of(&state, &data, &france).contains(&AgentActionKind::Assassinate));
}

#[test]
fn a_spy_ambushes_a_marching_enemy_army_and_a_herald_guides_a_friendly_one() {
    let data = eager(0, 0, 100, 100);
    let mut state = start_quiet(&data, "fac_france", 3);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    war(&mut state, &france, &england);
    let (army, there) = english_lieutenant_army(&mut state);
    place(&mut state, &data, &france, AgentKind::Spy, &there);
    // A stationed army does not march: no ambush.
    assert!(!actions_of(&state, &data, &france).contains(&AgentActionKind::Ambush));
    state.armies.get_mut(&army).unwrap().destination = Some(MoveTarget::Settlement(there.clone()));
    assert!(actions_of(&state, &data, &france).contains(&AgentActionKind::Ambush));

    // A French army marching through English country, a herald beside it.
    let (own, _) = stationed(&state, &france);
    state.armies.get_mut(&own).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(there.clone());
    state.armies.get_mut(&own).unwrap().destination = Some(MoveTarget::Settlement(there.clone()));
    place(&mut state, &data, &france, AgentKind::Emissary, &there);
    assert!(actions_of(&state, &data, &france).contains(&AgentActionKind::GuideArmy));
}

/// An English stationed army led by a lieutenant (never the sovereign, whom
/// a seal-1 spy cannot reach), and its place.
fn english_lieutenant_army(state: &mut CampaignState) -> (sim_campaign::ArmyId, SettlementId) {
    let england = fac("fac_england");
    let (army, place) = stationed(state, &england);
    let ruler = state.factions[&england].ruler.clone();
    let year = state.year;
    if state.armies[&army].general == ruler {
        let lieutenant = state
            .characters
            .iter()
            .find(|(id, c)| {
                c.alive
                    && c.faction == england
                    && !c.captive
                    && c.army.is_none()
                    && c.is_major(year)
                    && Some(*id) != ruler.as_ref()
            })
            .map(|(id, _)| id.clone())
            .expect("an English lieutenant");
        state.characters.get_mut(&lieutenant).unwrap().army = Some(army.clone());
        state.armies.get_mut(&army).unwrap().general = Some(lieutenant);
    }
    (army, place)
}
