//! TW m2a: armies unpaid for too long desert (`unpaid_seasons`).

use sim_campaign::test_support::{idle, start_quiet};
use sim_campaign::{ArmyId, EventKind};

use data_model::test_support::{fac, game_data};

fn strength(state: &sim_campaign::CampaignState, army: &ArmyId) -> u32 {
    state.armies[army].total_strength()
}

#[test]
fn unpaid_armies_desert_after_the_threshold_and_pay_resets_it() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = start_quiet(data, "fac_france", 5);
    let army = sim_campaign::test_support::main_army(&state, "fac_france");
    let threshold = data.economy_rules.desertion_after_unpaid_seasons;
    assert_eq!(threshold, 2);
    let before = strength(&state, &army);
    state.factions.get_mut(&france).unwrap().treasury = -10_000_000;
    // Seasons below the threshold: the counter climbs, nobody leaves.
    state.end_turn_with(data, idle);
    assert_eq!(state.armies[&army].unpaid_seasons, 1);
    assert_eq!(strength(&state, &army), before, "no desertion yet");
    // At the threshold: men desert and the journal says so.
    state.end_turn_with(data, idle);
    assert_eq!(state.armies[&army].unpaid_seasons, 2);
    let after = strength(&state, &army);
    assert!(after < before, "{after} < {before}");
    assert!(state.events().iter().any(|e| {
        e.kind == EventKind::Attrition
            && e.faction.as_ref() == Some(&france)
            && e.text_fr.contains("désertent")
    }));
    // A full treasury clears the counter.
    state.factions.get_mut(&france).unwrap().treasury = 10_000_000;
    state.end_turn_with(data, idle);
    assert_eq!(state.armies[&army].unpaid_seasons, 0);
}

#[test]
fn the_counter_is_saved_and_old_saves_load_with_zero() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    let army = sim_campaign::test_support::main_army(&state, "fac_france");
    state.armies.get_mut(&army).unwrap().unpaid_seasons = 3;
    let json = serde_json::to_string(&state.armies[&army]).unwrap();
    let back: sim_campaign::state::Army = serde_json::from_str(&json).unwrap();
    assert_eq!(back.unpaid_seasons, 3);
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value.as_object_mut().unwrap().remove("unpaid_seasons");
    let old: sim_campaign::state::Army = serde_json::from_value(value).unwrap();
    assert_eq!(old.unpaid_seasons, 0);
}
