//! Lot C5 (settlement panel): the options the bridge exposes for one
//! settlement come from the core — build options filtered by
//! `Building::allowed_in`, recruitment options and orders addressed to a
//! settlement other than the province's city.
//! See `docs/design/2026-09-24-echelle-colonies.md` § 4.3 and § 6.

use data_model::{FactionId, GameData, SettlementId, SettlementKind};
use sim_campaign::{CampaignState, Order, Place};

use data_model::test_support::game_data;

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 7).expect("1337 start")
}

/// First settlement of `kind` owned and controlled by France, by id.
fn french_settlement(state: &CampaignState, kind: SettlementKind) -> SettlementId {
    let france = FactionId::new("fac_france").unwrap();
    state
        .settlements
        .iter()
        .find(|(_, s)| s.kind == kind && s.owner == france && s.controller == france)
        .map(|(id, _)| id.clone())
        .expect("France holds a settlement of this kind")
}

#[test]
fn build_options_follow_the_settlement_kind() {
    let data = game_data();
    let state = start(data);
    let village = french_settlement(&state, SettlementKind::Village);
    let options = state.buildable(data, &village);
    assert!(!options.is_empty(), "a village can build something");
    for option in &options {
        let building = &data.buildings[&option.building];
        assert!(
            building.allowed_in(SettlementKind::Village),
            "{} is not allowed in a village",
            option.building
        );
    }
    let city = state.provinces[&state.settlements[&village].province]
        .city
        .clone();
    assert!(
        state.buildable(data, &city).len() > options.len(),
        "a city offers more buildings than a village"
    );
}

#[test]
fn recruit_and_build_orders_address_a_non_city_settlement() {
    let data = game_data();
    let mut state = start(data);
    let town = french_settlement(&state, SettlementKind::Town);
    let province = state.settlements[&town].province.clone();
    let city = state.provinces[&province].city.clone();
    assert_ne!(town, city);

    let recruit = state
        .recruitable(data, &town)
        .into_iter()
        .find(|option| option.available)
        .expect("a town can recruit at least one unit");
    state
        .submit_order(
            data,
            Order::Recruit {
                settlement: Place::Settlement(town.clone()),
                unit_type: recruit.unit_type.clone(),
            },
        )
        .expect("recruit in a town");
    let queued: Vec<_> = state.settlements[&town]
        .recruit_queue
        .iter()
        .map(|r| r.unit_type.clone())
        .collect();
    assert_eq!(queued, vec![recruit.unit_type]);
    assert!(state.settlements[&city].recruit_queue.is_empty());

    let build = state
        .buildable(data, &town)
        .into_iter()
        .find(|option| option.available)
        .expect("a town can build at least one building");
    state
        .submit_order(
            data,
            Order::Build {
                settlement: Place::Settlement(town.clone()),
                building: build.building.clone(),
            },
        )
        .expect("build in a town");
    let construction = state.settlements[&town]
        .construction
        .as_ref()
        .expect("the town has a construction");
    assert_eq!(construction.building, build.building);
    assert!(
        state
            .buildable(data, &town)
            .iter()
            .all(|option| option.building != build.building || !option.available),
        "the building under way cannot be ordered again"
    );
}
