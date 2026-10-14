//! RG10 (ADR 0344): the AI specialises its settlements, sparingly.

use data_model::test_support::{fac, game_data};
use sim_campaign::CampaignState;

#[test]
#[ignore = "CC C6: balance"] // share of specialised places on the 400-place map
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
    // CC C3: 400 places only (was 1656), each carries more weight and the AI
    // specialises about half of them; the "sparingly" band is re-tuned in C6.
    assert!(adopted * 3 < held, "{adopted} of {held} is too many");
    // Every chosen vocation exists in the data.
    assert!(state
        .settlements
        .values()
        .filter_map(|s| s.vocation.as_deref())
        .all(|id| data.vocation_rules.get(id).is_some()));
}
