//! Lot DP2 (ADR 0075): the AI's routes respect the right of passage at
//! peace and break it at war according to its temper.

use ai::grid::GridPlanner;
use data_model::{FactionId, GameData, SettlementId};
use sim_campaign::movement::edges;
use sim_campaign::passage::trespassed_owner;
use sim_campaign::CampaignState;

use data_model::test_support::{fac, game_data};

/// A French place with a direct road into the lands of a faction at peace
/// with France: (French place, foreign place, owner).
fn border_crossing(
    state: &CampaignState,
    data: &GameData,
) -> (SettlementId, SettlementId, FactionId) {
    let fr = fac("fac_france");
    for (id, s) in &state.settlements {
        if s.controller != fr {
            continue;
        }
        for (next, _) in edges(data, id) {
            let owner = state
                .settlement_province(&next)
                .and_then(|p| trespassed_owner(state, &fr, p));
            if let Some(owner) = owner {
                return (id.clone(), next, owner);
            }
        }
    }
    panic!("no French border with a faction at peace");
}

fn set_aggression(data: &mut GameData, faction: &FactionId, value: u8) {
    data.factions
        .get_mut(faction)
        .unwrap()
        .ai_personality
        .as_mut()
        .unwrap()
        .aggression = Some(value);
}

#[test]
fn routes_avoid_neutral_lands_unless_the_temper_allows_it() {
    let mut data = game_data().clone();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 1).unwrap();
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    let (from, into, _owner) = border_crossing(&state, &data);
    let reachable = |state: &CampaignState, data: &GameData| {
        let planner = GridPlanner::new(state, data, &fr);
        planner
            .table(&from, u32::MAX / 4, u32::MAX / 4, 1e9)
            .contains_key(&into)
    };
    // At peace with everyone: the border holds.
    for f in state.factions.values_mut() {
        f.at_war_with.clear();
    }
    set_aggression(&mut data, &fr, 100);
    assert!(!reachable(&state, &data), "no trespass at peace");
    // At war with England, a peaceful temper still respects it...
    state
        .factions
        .get_mut(&fr)
        .unwrap()
        .at_war_with
        .insert(en.clone());
    state
        .factions
        .get_mut(&en)
        .unwrap()
        .at_war_with
        .insert(fr.clone());
    set_aggression(&mut data, &fr, 0);
    data.ai_diplomacy.passage.ai_violate_attitude = -1000;
    data.ai_diplomacy.passage.ai_violate_power_ratio = 1000.0;
    assert!(!reachable(&state, &data));
    // ...an aggressive one marches through.
    set_aggression(&mut data, &fr, 100);
    assert!(reachable(&state, &data));
    // Without the rules, the old behaviour.
    set_aggression(&mut data, &fr, 0);
    data.ai_diplomacy.passage.enabled = false;
    assert!(reachable(&state, &data));
}
