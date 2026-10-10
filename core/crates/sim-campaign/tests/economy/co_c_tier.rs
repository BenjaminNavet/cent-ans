//! Lot CO-C (ADR 0292): the visual development tier of a settlement.

use data_model::test_support::{fac, game_data};
use sim_campaign::development_tier::{tier_for, tier_progress_percent};
use sim_campaign::CampaignState;

#[test]
fn tier_for_follows_the_thresholds() {
    let t = [10, 25, 40, 60, 80];
    assert_eq!(tier_for(0, 20, &t), 1);
    assert_eq!(tier_for(1, 20, &t), 1);
    assert_eq!(tier_for(2, 20, &t), 2);
    assert_eq!(tier_for(7, 20, &t), 3);
    assert_eq!(tier_for(11, 20, &t), 4);
    assert_eq!(tier_for(15, 20, &t), 5);
    assert_eq!(tier_for(16, 20, &t), 6);
    assert_eq!(tier_for(5, 0, &t), 1);
}

#[test]
fn empty_is_one_full_is_six_and_growth_is_monotone() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    let province = state.provinces.keys().next().unwrap().clone();
    let city = state.province_city_id(&province).unwrap().clone();
    {
        let live = state.settlements.get_mut(&city).unwrap();
        live.buildings.clear();
        live.fortification_level = 0;
    }
    assert_eq!(state.settlement_tier(data, &city), 1);
    // Fill the settlement with the best chains of its kind, one by one.
    let kind = state.settlement_state(&city).unwrap().kind;
    let mut by_tier: Vec<_> = data
        .buildings
        .values()
        .filter(|b| b.allowed_in(kind))
        .collect();
    by_tier.sort_by_key(|b| std::cmp::Reverse(b.tier));
    let mut previous = 1;
    let mut chains = std::collections::BTreeSet::new();
    for b in by_tier {
        let mut root = b.id.clone();
        while let Some(p) = data.buildings[&root].upgrades_from.clone() {
            if !data.buildings[&p].allowed_in(kind) {
                break;
            }
            root = p;
        }
        if !chains.insert(root) {
            continue;
        }
        state
            .settlements
            .get_mut(&city)
            .unwrap()
            .buildings
            .push(b.id.clone());
        let tier = state.settlement_tier(data, &city);
        assert!(tier >= previous, "tier must not fall while building");
        previous = tier;
    }
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .fortification_level = 4;
    assert_eq!(state.settlement_tier(data, &city), 6);
}

#[test]
fn tier_progress_runs_from_threshold_to_threshold() {
    let t = [10, 25, 40, 60, 80];
    assert_eq!(tier_progress_percent(0, 100, &t), 0);
    assert_eq!(tier_progress_percent(5, 100, &t), 50);
    assert_eq!(tier_progress_percent(10, 100, &t), 0);
    assert_eq!(tier_progress_percent(50, 100, &t), 50);
    assert_eq!(tier_progress_percent(80, 100, &t), 100);
    assert_eq!(tier_progress_percent(0, 0, &t), 100);
}
