//! OMR R1: the planning scope's indexes answer like the plain walks
//! (faction power, neighbours), and never outlive the scope.

use std::collections::BTreeSet;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

use data_model::test_support::game_data;

fn neighbours_by_walk(
    state: &CampaignState,
    data: &GameData,
    ids: &[FactionId],
    a: &FactionId,
) -> BTreeSet<FactionId> {
    ids.iter()
        .filter(|b| state.are_neighbors(data, a, b))
        .cloned()
        .collect()
}

#[test]
fn scope_answers_match_the_walks() {
    let data = game_data();
    let france = FactionId::new("fac_france").unwrap();
    let state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    // Outside any scope: the plain walks.
    let powers: Vec<f64> = ids.iter().map(|f| state.faction_power(f)).collect();
    let neighbours: Vec<BTreeSet<FactionId>> = ids
        .iter()
        .map(|a| neighbours_by_walk(&state, data, &ids, a))
        .collect();
    let incomes: Vec<i64> = ids.iter().map(|f| state.faction_income(data, f)).collect();
    let rivals: Vec<BTreeSet<FactionId>> = ids
        .iter()
        .map(|f| sim_campaign::diplomacy::rivals(&state, f))
        .collect();
    let listed: Vec<BTreeSet<FactionId>> = ids
        .iter()
        .map(|a| state.neighbour_factions(data, a))
        .collect();
    {
        let _scope = state.planning_scope();
        let _nested = state.planning_scope();
        for (i, a) in ids.iter().enumerate() {
            assert_eq!(state.faction_power(a).to_bits(), powers[i].to_bits(), "{a}");
            assert_eq!(state.faction_power_walk(a).to_bits(), powers[i].to_bits());
            assert_eq!(
                neighbours_by_walk(&state, data, &ids, a),
                neighbours[i],
                "{a}"
            );
            assert_eq!(state.neighbour_factions(data, a), listed[i], "{a}");
            // Twice: computed, then read from the memo.
            for _ in 0..2 {
                assert_eq!(state.faction_income(data, a), incomes[i], "{a}");
            }
            assert_eq!(state.faction_income_uncached(data, a), incomes[i]);
            for _ in 0..2 {
                assert_eq!(sim_campaign::diplomacy::rivals(&state, a), rivals[i], "{a}");
            }
            assert_eq!(sim_campaign::diplomacy::rivals_walk(&state, a), rivals[i]);
        }
        let unknown = FactionId::new("fac_nobody").unwrap();
        assert_eq!(state.faction_power(&unknown), 0.0);
        assert!(!state.are_neighbors(data, &unknown, &france));
    }
    assert!(powers.iter().any(|p| *p > 0.0));
    assert!(incomes.iter().any(|i| *i > 0));
    assert!(rivals.iter().any(|r| !r.is_empty()));
    assert!(neighbours.iter().filter(|n| !n.is_empty()).count() > 20);
}

#[test]
fn scope_never_serves_a_changed_state() {
    let data = game_data();
    let france = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let before = state.faction_power(&france);
    {
        let _scope = state.planning_scope();
        assert_eq!(state.faction_power(&france), before);
        // A clone lives elsewhere: changed, it is answered by the walk.
        let mut copy = state.clone();
        for settlement in copy.settlements.values_mut() {
            if settlement.controller == france {
                settlement.garrison.clear();
            }
        }
        copy.armies.retain(|_, a| a.faction != france);
        assert_eq!(copy.faction_power(&france), 0.0);
        assert_eq!(state.faction_power(&france), before);
    }
    // After the scope: the state changes and is read afresh.
    state.armies.retain(|_, a| a.faction != france);
    assert_eq!(
        state.faction_power(&france).to_bits(),
        state.faction_power_walk(&france).to_bits()
    );
    assert!(state.faction_power(&france) < before);
}
