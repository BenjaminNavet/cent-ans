//! UI audit A3, lot U3 « économie lisible »: budget history of the purse
//! (12 seasons kept, treasury chained season to season, the unexplained
//! part isolated in `other`), signed budget lines and old saves.

use sim_campaign::economy_balance::{BudgetLineKind, BUDGET_HISTORY_SEASONS};
use sim_campaign::CampaignState;

use data_model::test_support::{fac, game_data};

#[test]
fn history_is_empty_before_the_first_turn() {
    let data = game_data();
    let state = CampaignState::new_1337(data, fac("fac_france"), 7).expect("1337 start");
    assert!(state.budget_history(&fac("fac_france")).is_empty());
    let economy = state
        .faction_economy(data, &fac("fac_france"))
        .expect("economy");
    let lines = economy.budget_lines(state.last_budget(&fac("fac_france")));
    assert_eq!(lines.len(), BudgetLineKind::ALL.len());
    assert!(lines.iter().all(|line| line.last.is_none()));
    // Receipts positive, charges negative, and the lines sum to the balance.
    assert!(lines[0].projected > 0);
    for line in &lines {
        if line.kind.is_charge() {
            assert!(line.projected <= 0, "{:?} should be a charge", line.kind);
        }
    }
    let sum: i64 = lines.iter().map(|line| line.projected).sum();
    assert_eq!(sum, economy.net_income());
}

#[test]
fn history_chains_the_treasury_and_keeps_twelve_seasons() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 11).expect("1337 start");
    let start = state.faction_state(&france).unwrap().treasury;
    for _ in 0..(BUDGET_HISTORY_SEASONS + 3) {
        state.end_turn(data);
    }
    let history = state.budget_history(&france);
    assert_eq!(history.len(), BUDGET_HISTORY_SEASONS);
    assert_eq!(history.first().unwrap().turn, 3);
    assert_eq!(
        history.last().unwrap().turn as usize,
        BUDGET_HISTORY_SEASONS + 2
    );
    assert_eq!(
        history.last().unwrap().treasury,
        state.faction_state(&france).unwrap().treasury
    );
    // Each season's change links two consecutive treasuries.
    for pair in history.windows(2) {
        assert_eq!(pair[0].treasury + pair[1].change(), pair[1].treasury);
    }
    // The booked net is the one reported for the last turn.
    let last = history.last().unwrap();
    assert_eq!(Some(last.net()), state.faction_net_last_turn(&france));
    assert_ne!(start, last.treasury);
    // Lines now carry the season just resolved and its delta.
    let economy = state.faction_economy(data, &france).unwrap();
    let lines = economy.budget_lines(Some(last));
    let receipts = &lines[0];
    assert_eq!(receipts.last, Some(last.receipts));
    assert_eq!(receipts.delta(), Some(receipts.projected - last.receipts));
    let other = lines
        .iter()
        .find(|l| l.kind == BudgetLineKind::Other)
        .unwrap();
    assert_eq!(other.last, Some(last.other));
}

#[test]
fn old_saves_without_history_load() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 5).expect("1337 start");
    state.end_turn(data);
    let mut json = serde_json::to_value(&state).expect("serialize");
    for faction in json["factions"].as_object_mut().unwrap().values_mut() {
        faction.as_object_mut().unwrap().remove("budget_history");
    }
    let restored: CampaignState = serde_json::from_value(json).expect("old save loads");
    assert!(restored.budget_history(&france).is_empty());
    assert!(restored.last_budget(&france).is_none());
}
