//! RG10 (ADR 0344): the AI specialises its settlements, sparingly.

use data_model::test_support::{fac, game_data};
use sim_campaign::CampaignState;

#[test]
fn the_ai_adopts_vocations_in_a_campaign() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    state.chronicle.disabled = true;
    for _ in 0..8 {
        state.end_turn_with(data, ai::plan_turn);
    }
    let player = state.player_faction.clone();
    let adopted = state
        .settlements
        .values()
        .filter(|s| s.vocation.is_some() && s.controller != player)
        .count();
    let held = state
        .settlements
        .values()
        .filter(|s| s.controller != player)
        .count();
    // The AI specialises some of its settlements, never most of them.
    assert!(adopted > 0, "the AI never chose a vocation");
    // CC C6: 400 places only (was 1656), each holds more buildings (4 to 6 slots), so two of
    // one family no longer make a specialisation: `ai_min_buildings` 2 -> 3 brings the share
    // from 52 % (193 of 369) back to 20 % (72 of 369).
    assert!(adopted * 3 < held, "{adopted} of {held} is too many");
    // Every chosen vocation exists in the data.
    assert!(state
        .settlements
        .values()
        .filter_map(|s| s.vocation.as_deref())
        .all(|id| data.vocation_rules.get(id).is_some()));
}
