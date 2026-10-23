//! ADR 0748: the journal of causes fills from the events of a turn and survives a save.

use sim_campaign::causes::CauseKind;
use sim_campaign::test_support::{idle, start_quiet};
use sim_campaign::{CampaignState, EventKind, Order};

use data_model::test_support::{fac, game_data};

#[test]
fn bankruptcy_of_the_player_is_explained_by_its_budget_lines() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = start_quiet(data, "fac_france", 5);
    state.factions.get_mut(&france).unwrap().treasury = -10_000_000;
    state.end_turn_with(data, idle);
    let event = state
        .events()
        .iter()
        .find(|e| e.kind == EventKind::Bankruptcy && e.faction.as_ref() == Some(&france))
        .expect("bankruptcy event");
    let id = event.cause_id.expect("the event points to its cause");
    let record = state.cause_journal().get(id).expect("recorded");
    assert_eq!(record.cause.kind, CauseKind::Bankruptcy);
    assert_eq!(record.cause.subject.as_ref(), Some(&france));
    assert!(!record.cause.factors.is_empty());
    let weights: Vec<u8> = record.cause.factors.iter().map(|f| f.weight).collect();
    assert!(
        weights.windows(2).all(|w| w[0] >= w[1]),
        "ranked: {weights:?}"
    );
    assert!(event.cause.is_none(), "the cause moved into the journal");
}

#[test]
fn war_declared_by_the_player_is_recorded_with_its_motive() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 14);
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: fac("fac_flanders"),
            },
        )
        .unwrap();
    state.end_turn_with(data, idle);
    let record = state
        .cause_journal()
        .records()
        .find(|r| r.cause.kind == CauseKind::WarDeclared)
        .expect("war cause");
    assert!(record.cause.concerns(&fac("fac_france")));
    assert!(
        record.cause.reason.is_none(),
        "the player's own war has no AI reason"
    );
    assert!(!record.cause.factors.is_empty());
}

#[test]
fn journal_round_trips_through_a_save() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = start_quiet(data, "fac_france", 5);
    state.factions.get_mut(&france).unwrap().treasury = -10_000_000;
    state.end_turn_with(data, idle);
    assert!(!state.cause_journal().is_empty());
    let loaded = CampaignState::load_json(&state.save_json()).expect("round trip");
    assert_eq!(loaded.cause_journal(), state.cause_journal());
}

#[test]
fn a_v9_save_without_journal_loads_with_an_empty_one() {
    let data = game_data();
    let state = start_quiet(data, "fac_france", 5);
    let mut value: serde_json::Value = serde_json::from_str(&state.save_json()).unwrap();
    value.as_object_mut().unwrap().remove("causes");
    value["state_version"] = serde_json::json!(9);
    let loaded = CampaignState::load_json(&value.to_string()).expect("v9 migrates");
    assert!(loaded.cause_journal().is_empty());
    assert_eq!(loaded.state_version, sim_campaign::STATE_VERSION);
}
