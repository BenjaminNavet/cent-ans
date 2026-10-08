//! P1 regression: the 1337 setup and the AI share one frontier
//! classification, so no starting garrison is split into a leaderless army
//! on the first turn (`prov_normandie_ouest` did).

use data_model::ProvinceId;
use sim_campaign::Place;
use sim_campaign::{CampaignState, GarrisonRole, Order};

use data_model::test_support::{fac, game_data};

#[test]
fn starting_garrisons_follow_the_shared_classification() {
    let mut data = game_data().clone();
    // JR4b: the great realms short of money start with lighter garrisons
    // (`starting_budget`); the role sizes are checked without that trim.
    if let Some(rules) = data.settlement_rules.as_mut() {
        rules.starting_budget = None;
    }
    let state = CampaignState::new_1337(&data, fac("fac_england"), 1).unwrap();
    for id in state.provinces.keys() {
        // Lot C4: the province garrison is held by its city.
        let owner = state.province_owner(id).unwrap();
        let role = state.garrison_role(&data, owner, id);
        let garrison = &state.city_state(id).unwrap().garrison;
        assert_eq!(garrison.len(), role.garrison_size(), "{id}");
    }
    let normandie_ouest = ProvinceId::new("prov_normandie_ouest").unwrap();
    assert!(state.is_frontier(&data, &fac("fac_france"), &normandie_ouest));
    assert_eq!(
        state.garrison_role(&data, &fac("fac_france"), &normandie_ouest),
        GarrisonRole::Frontier
    );
    // The classification still leaves an interior to the realm.
    let interior = state
        .provinces
        .keys()
        .find(|id| {
            state.province_owner(id) == Some(&fac("fac_france"))
                && state.garrison_role(&data, &fac("fac_france"), id) == GarrisonRole::Interior
        })
        .cloned();
    assert!(interior.is_some(), "France has interior provinces");
}

#[test]
fn no_leaderless_army_is_split_from_a_garrison_on_the_first_turn() {
    let data = game_data();
    for player in ["fac_england", "fac_france"] {
        let state = CampaignState::new_1337(data, fac(player), 1).unwrap();
        for faction in state.factions.keys() {
            let splits: Vec<Place> = ai::plan_turn(&state, data, faction)
                .into_iter()
                .filter_map(|order| match order {
                    Order::CreateArmy { settlement, .. } => Some(settlement),
                    _ => None,
                })
                .collect();
            assert!(
                splits.is_empty(),
                "{faction} splits starting garrisons: {splits:?}"
            );
        }
    }
}
