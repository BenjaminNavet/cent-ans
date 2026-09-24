//! M10 campaign objectives and outcome.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::victory::OutcomeKind;
use sim_campaign::{CampaignState, EventKind, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

#[test]
fn every_playable_faction_has_objectives() {
    let data = data();
    for id in ["fac_france", "fac_england", "fac_burgundy"] {
        let state = CampaignState::new_1337(&data, fac(id), 1).unwrap();
        let objectives = state.objectives(&data, &fac(id));
        assert!(objectives.len() >= 3, "{id}");
        assert!(
            !objectives.iter().all(|o| o.done),
            "{id} has not won in 1337"
        );
    }
}

#[test]
fn france_starts_with_paris_and_a_vassal_burgundy() {
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_france"), 2).unwrap();
    let objectives = state.objectives(&data, &fac("fac_france"));
    let crown = objectives.iter().find(|o| o.id == "obj_fr_crown").unwrap();
    assert!(crown.done);
    let expel = objectives.iter().find(|o| o.id == "obj_fr_expel").unwrap();
    assert!(!expel.done && expel.progress.contains("occupée"));
}

#[test]
fn meeting_every_objective_wins() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    // F9: the objectives must hold `hold_turns` (20) seasons in a row.
    let hold = data.factions[&fac("fac_france")]
        .victory
        .as_ref()
        .and_then(|v| v.hold_turns)
        .unwrap();
    let mut won_at = None;
    for season in 1..=hold + 2 {
        for p in state.provinces.values_mut() {
            if p.controller == fac("fac_england") {
                p.controller = fac("fac_france");
            }
        }
        let events = state.end_turn_with(&data, idle);
        if events.iter().any(|e| e.kind == EventKind::Victory) {
            won_at = Some(season);
            break;
        }
    }
    assert_eq!(
        won_at,
        Some(hold),
        "{:?}",
        state.objectives(&data, &fac("fac_france"))
    );
    let outcome = state.outcome.clone().unwrap();
    assert_eq!(outcome.kind, OutcomeKind::Victory);
    assert!(outcome.score > 0);
    // Reported once.
    let events = state.end_turn_with(&data, idle);
    assert!(!events.iter().any(|e| e.kind == EventKind::Victory));
}

#[test]
fn losing_every_province_is_a_defeat() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_navarre"), 4).unwrap();
    for p in state.provinces.values_mut() {
        if p.controller == fac("fac_navarre") {
            p.controller = fac("fac_castile");
        }
    }
    let events = state.end_turn_with(&data, idle);
    assert!(events.iter().any(|e| e.kind == EventKind::Defeat));
}

#[test]
fn the_end_year_closes_the_campaign() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 5).unwrap();
    state.year = 1454;
    let events = state.end_turn_with(&data, idle);
    assert!(events.iter().any(|e| e.kind == EventKind::CampaignEnded));
    let loaded = CampaignState::load_json(&state.save_json()).unwrap();
    assert_eq!(loaded.outcome, state.outcome);
    let _ = ProvinceId::new("prov_guyenne").unwrap();
}
