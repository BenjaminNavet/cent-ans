//! WR armies (ADR 0305): the AI sends its free commanders to the province of a
//! leaderless army and names a chief to an existing chiefless army.

use data_model::test_support::{fac, game_data};
use sim_campaign::test_support::main_army;
use sim_campaign::{CampaignState, Order};

#[test]
fn the_ai_sends_a_free_commander_to_a_chiefless_army() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 5).unwrap();
    let main = main_army(&state, "fac_france");
    state
        .armies
        .retain(|id, a| a.faction != france || *id == main);
    let army_province = state
        .army_province(data, &state.armies[&main])
        .expect("province");
    if let Some(general) = state.armies.get_mut(&main).unwrap().general.take() {
        state.characters.get_mut(&general).unwrap().army = None;
    }
    // Every french character stands in another french province, free.
    let elsewhere = state
        .provinces
        .keys()
        .find(|p| state.holds_province(&france, p) && **p != army_province)
        .cloned()
        .unwrap();
    for c in state
        .characters
        .values_mut()
        .filter(|c| c.faction == france)
    {
        c.army = None;
        c.governor_of = None;
        c.location = Some(elsewhere.clone());
    }
    let orders = ai::plan_turn(&state, data, &france);
    let sent: Vec<_> = orders
        .iter()
        .filter_map(|o| match o {
            Order::SendCharacter { character, to } => Some((character.clone(), to.clone())),
            _ => None,
        })
        .collect();
    assert!(
        sent.iter().any(|(_, to)| *to == army_province),
        "no commander sent to {army_province}: {sent:?}"
    );
    for order in orders
        .iter()
        .filter(|o| matches!(o, Order::SendCharacter { .. }))
    {
        state.apply_order(data, &france, order.clone()).unwrap();
    }
}

#[test]
fn the_ai_names_a_chief_once_a_commander_is_on_the_spot() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 5).unwrap();
    let main = main_army(&state, "fac_france");
    state
        .armies
        .retain(|id, a| a.faction != france || *id == main);
    let army_province = state
        .army_province(data, &state.armies[&main])
        .expect("province");
    if let Some(general) = state.armies.get_mut(&main).unwrap().general.take() {
        state.characters.get_mut(&general).unwrap().army = None;
    }
    let year = state.year();
    let commander = state
        .characters
        .iter()
        .find(|(_, c)| c.faction == france && c.alive && c.is_major(year))
        .map(|(id, _)| id.clone())
        .unwrap();
    for c in state
        .characters
        .values_mut()
        .filter(|c| c.faction == france)
    {
        c.army = None;
        c.governor_of = None;
    }
    state.characters.get_mut(&commander).unwrap().location = Some(army_province);
    let orders = ai::plan_turn(&state, data, &france);
    assert!(orders
        .iter()
        .any(|o| matches!(o, Order::AssignGeneral { army, .. } if *army == main)));
}
