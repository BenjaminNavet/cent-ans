//! Lot WH armyb (ADR 0279): recruit into an army, forced sortie, raid supplies,
//! surrender demand.
use data_model::test_support::{fac, game_data};
use data_model::GameData;
use sim_campaign::test_support::{capital_city, main_army};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, OrderError, Season, Stance};

fn start() -> (&'static GameData, CampaignState) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.chronicle.disabled = true;
    state.season = Season::Summer;
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    (data, state)
}

fn recruitable_unit(
    state: &CampaignState,
    data: &GameData,
    place: &data_model::SettlementId,
) -> data_model::UnitTypeId {
    state
        .recruitable(data, place)
        .into_iter()
        .find(|o| o.available)
        .expect("a recruitable unit")
        .unit_type
}

/// The main French army parked in Paris.
fn army_in_paris(state: &mut CampaignState) -> (ArmyId, data_model::SettlementId) {
    let army = main_army(state, "fac_france");
    let paris = capital_city(state, "fac_france");
    let entry = state.armies.get_mut(&army).unwrap();
    entry.position = ArmyPosition::Settlement(paris.clone());
    entry.stance = Stance::Normal;
    (army, paris)
}

#[test]
fn recruit_into_army_joins_the_army_not_the_garrison() {
    let (data, mut state) = start();
    let (army, paris) = army_in_paris(&mut state);
    state.armies.get_mut(&army).unwrap().units.truncate(3);
    let unit = recruitable_unit(&state, data, &paris);
    let garrison = state.settlements[&paris].garrison.len();
    state
        .submit_order(
            data,
            Order::RecruitInto {
                settlement: paris.clone().into(),
                unit_type: unit,
                army: army.clone(),
            },
        )
        .unwrap();
    state.end_turn(data);
    assert_eq!(state.armies[&army].units.len(), 4);
    assert_eq!(state.settlements[&paris].garrison.len(), garrison);
}

#[test]
fn recruit_into_army_falls_back_to_garrison_when_the_army_left() {
    let (data, mut state) = start();
    let (army, paris) = army_in_paris(&mut state);
    state.armies.get_mut(&army).unwrap().units.truncate(3);
    let unit = recruitable_unit(&state, data, &paris);
    let garrison = state.settlements[&paris].garrison.len();
    state
        .submit_order(
            data,
            Order::RecruitInto {
                settlement: paris.clone().into(),
                unit_type: unit,
                army: army.clone(),
            },
        )
        .unwrap();
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::Field { x: 1.0, y: 1.0 };
    state.end_turn(data);
    assert_eq!(state.armies[&army].units.len(), 3);
    assert_eq!(state.settlements[&paris].garrison.len(), garrison + 1);
}

#[test]
fn recruit_into_full_army_is_refused_and_foreign_army_too() {
    let (data, mut state) = start();
    let (army, paris) = army_in_paris(&mut state);
    let unit = recruitable_unit(&state, data, &paris);
    let cap = data.army_rules.cap();
    while state.armies[&army].units.len() < cap {
        let copy = state.armies[&army].units[0].clone();
        state.armies.get_mut(&army).unwrap().units.push(copy);
    }
    let order = |army: &ArmyId| Order::RecruitInto {
        settlement: paris.clone().into(),
        unit_type: unit.clone(),
        army: army.clone(),
    };
    assert!(matches!(
        state.submit_order(data, order(&army)),
        Err(OrderError::ArmyFull { .. })
    ));
    let english = main_army(&state, "fac_england");
    assert!(state.submit_order(data, order(&english)).is_err());
}
