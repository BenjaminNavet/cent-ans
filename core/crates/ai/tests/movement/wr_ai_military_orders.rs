//! WR ai-mil (ADR 0301): the AI recruits into armies, sallies out of a
//! besieged place and calls besieged garrisons to surrender.

use data_model::test_support::{fac, game_data};
use data_model::{GameData, SettlementId};
use sim_campaign::test_support::{capital_city, idle, main_army};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, Season, Stance};

fn order_places(orders: &[Order], wanted: impl Fn(&Order) -> bool) -> usize {
    orders.iter().filter(|o| wanted(o)).count()
}

/// France with a rich treasury and its main army parked in Paris.
fn paris_with_army(units: usize) -> (&'static GameData, CampaignState, ArmyId, SettlementId) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 5).unwrap();
    state.chronicle.disabled = true;
    state.season = Season::Summer;
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 200_000;
    let army = main_army(&state, "fac_france");
    let paris = capital_city(&state, "fac_france");
    let entry = state.armies.get_mut(&army).unwrap();
    entry.position = ArmyPosition::Settlement(paris.clone());
    entry.stance = Stance::Normal;
    entry.units.truncate(units);
    (data, state, army, paris)
}

#[test]
fn recruits_join_a_small_army_standing_in_the_settlement() {
    let (data, state, army, paris) = paris_with_army(3);
    assert!(state.settlements[&paris].garrison.len() >= 3);
    let orders = ai::plan_turn(&state, data, &fac("fac_france"));
    let into: Vec<&Order> = orders
        .iter()
        .filter(|o| matches!(o, Order::RecruitInto { army: a, .. } if *a == army))
        .collect();
    assert!(!into.is_empty(), "recruits go into the army: {orders:?}");
    // No plain recruit is left for the settlement while the army is small.
    assert_eq!(
        order_places(&orders, |o| matches!(o, Order::Recruit { settlement, .. }
            if *settlement == paris.clone().into())),
        0
    );
    // Every order is accepted, and the army fills up within the cap.
    let mut state = state;
    for order in into {
        state
            .apply_order(data, &fac("fac_france"), order.clone())
            .unwrap();
    }
}

#[test]
fn a_full_army_gets_no_recruits() {
    let (data, mut state, army, _) = paris_with_army(40);
    // Fill up to the target by cloning.
    let unit = state.armies[&army].units[0].clone();
    let entry = state.armies.get_mut(&army).unwrap();
    while entry.units.len() < 30 {
        entry.units.push(unit.clone());
    }
    let orders = ai::plan_turn(&state, data, &fac("fac_france"));
    assert_eq!(
        order_places(&orders, |o| matches!(o, Order::RecruitInto { .. })),
        0
    );
}

/// England besieges French Boulogne; the siege has begun.
fn besieged_boulogne(
    english_units: usize,
    garrison_units: usize,
) -> (CampaignState, SettlementId, ArmyId) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 4).unwrap();
    state.chronicle.disabled = true;
    let english = main_army(&state, "fac_england");
    let boulogne = SettlementId::new("set_boulogne").unwrap();
    state
        .armies
        .get_mut(&english)
        .unwrap()
        .units
        .truncate(english_units);
    let garrison = &mut state.settlements.get_mut(&boulogne).unwrap().garrison;
    while garrison.len() < garrison_units {
        let copy = garrison[0].clone();
        garrison.push(copy);
    }
    garrison.truncate(garrison_units);
    let entry = state.armies.get_mut(&english).unwrap();
    entry.position = ArmyPosition::Settlement(boulogne.clone());
    entry.stance = Stance::Siege;
    state.end_turn_with(data, idle);
    assert!(state.settlements[&boulogne].siege.is_some(), "siege begun");
    (state, boulogne, english)
}

fn has_sortie(orders: &[Order], place: &SettlementId) -> bool {
    orders
        .iter()
        .any(|o| matches!(o, Order::Sortie { settlement } if *settlement == place.clone().into()))
}

#[test]
fn a_strong_garrison_sallies_out_a_weak_one_does_not() {
    let data = game_data();
    let (mut state, boulogne, english) = besieged_boulogne(8, 8);
    state.armies.get_mut(&english).unwrap().units.truncate(1);
    for unit in &mut state.armies.get_mut(&english).unwrap().units {
        unit.strength = unit.max_strength.clamp(1, 20);
    }
    let orders = ai::plan_turn(&state, data, &fac("fac_france"));
    assert!(has_sortie(&orders, &boulogne), "{orders:?}");

    let (state, boulogne, _) = besieged_boulogne(30, 3);
    let orders = ai::plan_turn(&state, data, &fac("fac_france"));
    assert!(!has_sortie(&orders, &boulogne));
}

#[test]
fn a_starving_garrison_sallies_out_if_not_hopeless() {
    let data = game_data();
    // Even sides: below the usual threshold, above the desperate one.
    let (mut state, boulogne, english) = besieged_boulogne(8, 6);
    let power = |state: &CampaignState| {
        let garrison = sim_campaign::state::unit_power(data, &state.settlements[&boulogne].garrison);
        let besieger = state.army_power(data, &english);
        100.0 * garrison / (garrison + besieger)
    };
    let share = power(&state);
    let rules = &data_model::AiCampaign::bundled().military_orders;
    assert!(
        share < f64::from(rules.sortie_odds) && share >= f64::from(rules.sortie_desperate_odds),
        "scenario needs a middling share, got {share}"
    );
    assert!(!has_sortie(
        &ai::plan_turn(&state, data, &fac("fac_france")),
        &boulogne
    ));
    state
        .settlements
        .get_mut(&boulogne)
        .unwrap()
        .siege
        .as_mut()
        .unwrap()
        .supplies = 5;
    assert!(has_sortie(
        &ai::plan_turn(&state, data, &fac("fac_france")),
        &boulogne
    ));
}

fn demands(orders: &[Order], place: &SettlementId) -> usize {
    order_places(orders, |o| {
        matches!(o, Order::DemandSurrender { settlement } if *settlement == place.clone().into())
    })
}

#[test]
fn the_besieger_demands_surrender_only_when_likely_and_not_every_turn() {
    let data = game_data();
    let (mut state, boulogne, _) = besieged_boulogne(8, 8);
    let england = fac("fac_england");
    let turn = state.turn;
    let set = |state: &mut CampaignState, supplies: u8, breach: u8, started: u32| {
        let s = state
            .settlements
            .get_mut(&boulogne)
            .unwrap()
            .siege
            .as_mut()
            .unwrap();
        s.supplies = supplies;
        s.breach = breach;
        s.started_turn = started;
    };
    // Full stores: no chance, no demand.
    set(&mut state, 100, 0, turn);
    assert_eq!(demands(&ai::plan_turn(&state, data, &england), &boulogne), 0);
    // Empty stores, open walls: certain, on the first due turn.
    set(&mut state, 0, 100, turn);
    let orders = ai::plan_turn(&state, data, &england);
    assert_eq!(demands(&orders, &boulogne), 1);
    // The turn after a (failed) demand: not due again.
    set(&mut state, 0, 100, turn - 1);
    assert_eq!(demands(&ai::plan_turn(&state, data, &england), &boulogne), 0);
    // Applied, the demand takes the place.
    set(&mut state, 0, 100, turn);
    for order in orders.into_iter().filter(|o| matches!(o, Order::DemandSurrender { .. })) {
        state.apply_order(data, &england, order).unwrap();
    }
    assert_eq!(state.settlements[&boulogne].controller, england);
}
