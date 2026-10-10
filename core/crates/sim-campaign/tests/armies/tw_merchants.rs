//! TW misc-camp (ADR 0332): merchants, trading posts, buy-outs, wartime expulsion.

use data_model::test_support::{fac, game_data};
use data_model::{AgentActionKind, AgentKind, FactionId, GameData, SettlementId};
use sim_campaign::agents::{self, AgentId};
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::test_support::start;
use sim_campaign::{CampaignState, Order};

fn paris(state: &CampaignState) -> SettlementId {
    state
        .province_city_id(&data_model::ProvinceId::new("prov_ile_de_france").unwrap())
        .unwrap()
        .clone()
}

fn set_war(state: &mut CampaignState, a: &FactionId, b: &FactionId, at_war: bool) {
    for (x, y) in [(a, b), (b, a)] {
        let f = state.factions.get_mut(x).unwrap();
        if at_war {
            f.at_war_with.insert(y.clone());
            f.allies.remove(y);
        } else {
            f.at_war_with.remove(y);
        }
    }
}

/// A merchant of `faction` standing in `at` (bypassing the recruitment place).
fn merchant(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    at: &SettlementId,
) -> AgentId {
    state.factions.get_mut(faction).unwrap().treasury += 10_000;
    let home = state
        .settlements
        .iter()
        .find(|(_, s)| &s.controller == faction && s.kind == data_model::SettlementKind::City)
        .map(|(id, _)| id.clone())
        .unwrap();
    let id = state
        .recruit_agent(data, faction, &home, AgentKind::Merchant)
        .unwrap();
    let agent = state.agents.agents.get_mut(&id).unwrap();
    agent.location = at.clone();
    agent.movement_points = 500;
    id
}

fn sure_data() -> GameData {
    let mut data = game_data().clone();
    let mut rules = agents::rules(&data).clone();
    for action in rules.actions.values_mut() {
        action.base_chance = 100;
        action.death_risk = 0;
    }
    rules.max_chance = 100;
    rules.min_chance = 100;
    data.agent_rules = Some(rules);
    data
}

fn act(state: &mut CampaignState, data: &GameData, id: &AgentId, action: AgentActionKind) {
    let faction = state.agent(id).unwrap().faction.clone();
    state
        .agent_act(data, &faction, id, action, None, None)
        .unwrap_or_else(|e| panic!("{action:?} refused: {e}"));
}

#[test]
fn merchant_is_recruited_and_capped() {
    let data = game_data();
    let mut state = start(data, "fac_france", 7);
    let france = fac("fac_france");
    state.factions.get_mut(&france).unwrap().treasury = 5_000;
    let paris = paris(&state);
    let max = agents::rules(data).types[&AgentKind::Merchant].max_per_faction;
    for _ in 0..max {
        state
            .recruit_agent(data, &france, &paris, AgentKind::Merchant)
            .expect("a merchant in Paris");
    }
    assert!(state
        .recruit_agent(data, &france, &paris, AgentKind::Merchant)
        .is_err());
    // A merchant offers exactly the two merchant actions.
    let id = state.agents_of(&france)[0].0.clone();
    let bar = state.agent_actions(data, &id);
    assert_eq!(bar.len(), 2);
}

#[test]
fn trading_post_pays_each_season_more_with_the_seal() {
    let data = sure_data();
    let income = |level: u8| {
        let mut state = start(&data, "fac_france", 7);
        let france = fac("fac_france");
        let paris = paris(&state);
        let id = merchant(&mut state, &data, &france, &paris);
        let seal = state.agents.agents.get_mut(&id).unwrap();
        seal.level = level;
        seal.experience = if level >= 5 { 14 } else { 0 };
        act(&mut state, &data, &id, AgentActionKind::TradePost);
        assert_eq!(state.agent(&id).unwrap().post.as_ref(), Some(&paris));
        state.end_turn(&data);
        state
            .agents
            .merchant_income_last_turn
            .get(&france)
            .copied()
            .unwrap_or(0)
    };
    let (low, high) = (income(1), income(5));
    assert!(low >= 40, "a city post pays at least its base: {low}");
    assert!(high > low, "seal 5 earns more: {high} vs {low}");
}

#[test]
fn a_second_post_in_the_same_place_is_refused_to_the_same_merchant() {
    let data = sure_data();
    let mut state = start(&data, "fac_france", 7);
    let france = fac("fac_france");
    let paris = paris(&state);
    let id = merchant(&mut state, &data, &france, &paris);
    act(&mut state, &data, &id, AgentActionKind::TradePost);
    state.agents.agents.get_mut(&id).unwrap().acted = false;
    assert!(state
        .agent_act(&data, &france, &id, AgentActionKind::TradePost, None, None)
        .is_err());
}

#[test]
fn a_rival_merchant_buys_out_the_post_and_a_host_takes_its_share() {
    let data = sure_data();
    let mut state = start(&data, "fac_england", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    set_war(&mut state, &france, &england, false);
    let paris = paris(&state);
    let english = merchant(&mut state, &data, &england, &paris);
    act(&mut state, &data, &english, AgentActionKind::TradePost);
    // Alone in a foreign city: the French master takes his share.
    state.end_turn(&data);
    let paid = &state.agents.merchant_income_last_turn;
    assert!(paid.get(&england).copied().unwrap_or(0) > 0);
    assert!(paid.get(&france).copied().unwrap_or(0) > 0, "host share");
    // A French merchant outbids him.
    let french = merchant(&mut state, &data, &france, &paris);
    state.agents.agents.get_mut(&french).unwrap().acted = false;
    state.agents.agents.get_mut(&english).unwrap().post = Some(paris.clone());
    act(&mut state, &data, &french, AgentActionKind::Outbid);
    assert_eq!(state.agent(&english).unwrap().post, None);
    assert_eq!(state.agent(&french).unwrap().post.as_ref(), Some(&paris));
}

#[test]
fn outbid_needs_a_rival_post() {
    let data = sure_data();
    let mut state = start(&data, "fac_france", 7);
    let france = fac("fac_france");
    let paris = paris(&state);
    let id = merchant(&mut state, &data, &france, &paris);
    assert!(state
        .agent_act(&data, &france, &id, AgentActionKind::Outbid, None, None)
        .is_err());
}

#[test]
fn war_drives_a_foreign_merchant_out() {
    let data = sure_data();
    let mut state = start(&data, "fac_england", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    set_war(&mut state, &france, &england, false);
    let paris = paris(&state);
    let english = merchant(&mut state, &data, &england, &paris);
    act(&mut state, &data, &english, AgentActionKind::TradePost);
    set_war(&mut state, &france, &england, true);
    state.end_turn(&data);
    assert_eq!(state.agent(&english).unwrap().post, None);
}

#[test]
fn leaving_the_place_drops_the_post() {
    let data = sure_data();
    let mut state = start(&data, "fac_france", 7);
    let france = fac("fac_france");
    let paris = paris(&state);
    let id = merchant(&mut state, &data, &france, &paris);
    act(&mut state, &data, &id, AgentActionKind::TradePost);
    let other = state
        .settlements
        .iter()
        .find(|(s, st)| *s != &paris && st.controller == france)
        .map(|(s, _)| s.clone())
        .unwrap();
    state.agents.agents.get_mut(&id).unwrap().location = other;
    state.end_turn(&data);
    assert_eq!(state.agent(&id).unwrap().post, None);
    assert_eq!(
        state.agents.merchant_income_last_turn.get(&france),
        None,
        "no post, no income"
    );
}

#[test]
fn ai_merchant_opens_a_post_where_it_stands() {
    let data = sure_data();
    let mut state = start(&data, "fac_england", 7);
    let france = fac("fac_france");
    let paris = paris(&state);
    merchant(&mut state, &data, &france, &paris);
    let orders = agents::plan_agents(&PlanCache::new(&state), &data, &france);
    assert!(orders.iter().any(|o| matches!(
        o,
        Order::AgentAction {
            action: AgentActionKind::TradePost,
            ..
        }
    )));
}
