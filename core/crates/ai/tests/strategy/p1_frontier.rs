//! P1 regression: the 1337 setup and the AI share one frontier
//! classification, so no starting garrison is split into a leaderless army
//! on the first turn (`prov_normandie_ouest`, now merged into `prov_normandie`, did).

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
        // The province garrison is held by its city.
        let owner = state.province_owner(id).unwrap();
        let role = state.garrison_role(&data, owner, id);
        let garrison = &state.city_state(id).unwrap().garrison;
        assert_eq!(garrison.len(), role.garrison_size(), "{id}");
    }
    let normandie = ProvinceId::new("prov_normandie").unwrap();
    assert!(state.is_frontier(&data, &fac("fac_france"), &normandie));
    assert_eq!(
        state.garrison_role(&data, &fac("fac_france"), &normandie),
        GarrisonRole::Frontier
    );
    // The classification still leaves an interior to the realms.
    // CC C3: France keeps three provinces, all border or coast, so the check
    // runs over every faction.
    let interior = state
        .provinces
        .keys()
        .find(|id| {
            state.province_owner(id).is_some_and(|owner| {
                state.garrison_role(&data, owner, id) == GarrisonRole::Interior
            })
        })
        .cloned();
    assert!(interior.is_some(), "some realm has interior provinces");
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
