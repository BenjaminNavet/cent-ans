//! Lot LR-15: the capital of a faction that holds no city (ADR 0165) — the
//! seat that stands for it never is another realm's city, the start and both
//! AIs neither panic nor act in a place the faction does not control.
use data_model::test_support::{fac, game_data};

use data_model::{FactionId, GameData, ProvinceId, SettlementKind};
use sim_campaign::{CampaignState, Order, Place};

fn data() -> &'static GameData {
    ai::feudal::install();
    game_data()
}

/// Brittany loses every city to France; its towns and castles stay.
fn cityless_brittany(data: &GameData) -> (CampaignState, FactionId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).expect("1337 start");
    let realm = fac("fac_brittany");
    let france = fac("fac_france");
    let ids: Vec<_> = state.settlements.keys().cloned().collect();
    for id in ids {
        let is_city = state
            .settlement_province(&id)
            .and_then(|p| state.province_city_id(p))
            == Some(&id);
        let place = state.settlements.get_mut(&id).unwrap();
        if place.controller == realm && is_city {
            place.controller = france.clone();
            place.owner = france.clone();
        }
    }
    (state, realm)
}

/// Every order that names a settlement names one `faction` controls.
fn assert_orders_at_home(state: &CampaignState, faction: &FactionId, orders: &[Order]) {
    for order in orders {
        let place = match order {
            Order::Recruit { settlement, .. } | Order::CreateArmy { settlement, .. } => settlement,
            _ => continue,
        };
        let Place::Settlement(id) = place else {
            continue;
        };
        assert_eq!(
            state.settlements.get(id).map(|s| &s.controller),
            Some(faction),
            "{order:?} acts in a place {faction} does not control"
        );
    }
}

#[test]
fn the_seat_of_a_cityless_realm_is_a_place_it_holds() {
    let data = data();
    let (state, realm) = cityless_brittany(data);
    let capital = state.factions[&realm].capital.clone();
    assert!(!state.controls_province(&realm, &capital));
    let seat = state
        .faction_seat(&realm)
        .expect("Brittany still holds places");
    let place = &state.settlements[&seat];
    assert_eq!(place.controller, realm);
    assert_ne!(place.kind, SettlementKind::City);
    assert_ne!(place.kind, SettlementKind::Village);
    assert_eq!(state.faction_capital_city(&realm), None);
    // A realm that holds its capital keeps its capital's city.
    let france = fac("fac_france");
    let paris = state.factions[&france].capital.clone();
    assert_eq!(
        state.faction_seat(&france).as_ref(),
        state.province_city_id(&paris)
    );
}

#[test]
fn both_ais_plan_a_cityless_realm_at_home() {
    let data = data();
    let (mut state, realm) = cityless_brittany(data);
    state.factions.get_mut(&realm).unwrap().treasury = 5_000;
    let orders = ai::plan_turn(&state, data, &realm);
    assert_orders_at_home(&state, &realm, &orders);
    let orders = sim_campaign::ai_minimal::plan_turn(&state, data, &realm);
    assert_orders_at_home(&state, &realm, &orders);
}

#[test]
fn a_realm_that_holds_nothing_has_no_seat_and_plans_without_panic() {
    let data = data();
    let (mut state, realm) = cityless_brittany(data);
    let france = fac("fac_france");
    for place in state.settlements.values_mut() {
        if place.controller == realm {
            place.controller = france.clone();
            place.owner = france.clone();
        }
    }
    assert_eq!(state.faction_seat(&realm), None);
    let orders = ai::plan_turn(&state, data, &realm);
    assert_orders_at_home(&state, &realm, &orders);
    let orders = sim_campaign::ai_minimal::plan_turn(&state, data, &realm);
    assert_orders_at_home(&state, &realm, &orders);
}

#[test]
fn a_capital_held_by_another_realm_does_not_station_the_starting_army_there() {
    // Data whose capital lies in another realm's province (as the crusade's
    // Cyprus): the main army starts in a place of its own.
    let mut data = data().clone();
    let poland = fac("fac_poland");
    let bohemia = ProvinceId::new("prov_bohemia").unwrap();
    assert_ne!(data.provinces[&bohemia].owner, poland);
    data.factions.get_mut(&poland).unwrap().capital = bohemia;
    let state = CampaignState::new_1337(&data, fac("fac_france"), 1).expect("1337 start");
    let stations: Vec<_> = state
        .armies
        .values()
        .filter(|a| a.faction == poland)
        .map(|a| a.position.clone())
        .collect();
    assert!(!stations.is_empty(), "Poland fields its starting army");
    for position in stations {
        let sim_campaign::state::ArmyPosition::Settlement(id) = position else {
            panic!("starting army off a settlement: {position:?}");
        };
        assert_eq!(state.settlements[&id].controller, poland, "{id}");
    }
}

#[test]
fn a_cityless_realm_plays_turns_without_panic() {
    let data = data();
    let (mut state, realm) = cityless_brittany(data);
    state.interactive_battles = false;
    for _ in 0..2 {
        state.end_turn_with(data, ai::plan_turn);
    }
    if let Some(seat) = state.faction_seat(&realm) {
        assert_eq!(state.settlements[&seat].controller, realm);
    }
}
