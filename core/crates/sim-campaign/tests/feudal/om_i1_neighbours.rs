//! OM I1: `neighbour_factions` (one pass) agrees with `are_neighbors` (per
//! pair) on the real 443-province map.

use std::collections::BTreeSet;

use data_model::FactionId;
use sim_campaign::CampaignState;

use data_model::test_support::game_data;

#[test]
fn neighbour_factions_match_pairwise_neighbours() {
    let data = game_data();
    let state = CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 1).unwrap();
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    let mut checked = 0;
    for a in ids.iter().step_by(7) {
        let fast = state.neighbour_factions(data, a);
        let slow: BTreeSet<FactionId> = ids
            .iter()
            .filter(|b| state.are_neighbors(data, a, b))
            .cloned()
            .collect();
        assert_eq!(fast, slow, "{a}");
        checked += 1;
    }
    assert!(checked > 20);
    let france = FactionId::new("fac_france").unwrap();
    assert!(state
        .neighbour_factions(data, &france)
        .contains(&FactionId::new("fac_burgundy").unwrap()));
}
