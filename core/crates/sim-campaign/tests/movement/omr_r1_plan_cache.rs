//! The plan cache answers like the plain walks of the state (faction power,
//! neighbours, income, rivals), asked twice (computed, then memoised).

use std::collections::BTreeSet;

use data_model::test_support::game_data;
use data_model::FactionId;
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::CampaignState;

#[test]
fn cache_answers_match_the_walks() {
    let data = game_data();
    let france = FactionId::new("fac_france").unwrap();
    let state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let cache = PlanCache::new(&state);
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    for a in &ids {
        for _ in 0..2 {
            assert_eq!(
                cache.faction_power(a).to_bits(),
                state.faction_power(a).to_bits(),
                "{a}"
            );
            assert_eq!(
                cache.faction_income(data, a),
                state.faction_income(data, a),
                "{a}"
            );
            assert_eq!(
                cache.rivals(a),
                sim_campaign::diplomacy::rivals(&state, a),
                "{a}"
            );
        }
        assert_eq!(
            cache.neighbour_factions(data, a),
            state.neighbour_factions(data, a),
            "{a}"
        );
        for b in &ids {
            assert_eq!(
                cache.are_neighbors(data, a, b),
                state.are_neighbors(data, a, b),
                "{a} {b}"
            );
        }
        assert_eq!(
            cache.coalition_power(a).to_bits(),
            state.coalition_power(a).to_bits(),
            "{a}"
        );
    }
    let unknown = FactionId::new("fac_nobody").unwrap();
    assert_eq!(cache.faction_power(&unknown), 0.0);
    assert!(!cache.are_neighbors(data, &unknown, &france));
    assert!(ids.iter().any(|f| cache.faction_power(f) > 0.0));
    assert!(ids.iter().any(|f| cache.faction_income(data, f) > 0));
    assert!(ids.iter().any(|f| !cache.rivals(f).is_empty()));
    let bordering: usize = ids
        .iter()
        .filter(|f| cache.neighbour_factions(data, f) != BTreeSet::new())
        .count();
    assert!(bordering > 20);
}
