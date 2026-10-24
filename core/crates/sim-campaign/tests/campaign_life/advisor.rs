//! GAMEPLAY2 (ADR 0789): the counsellor speaks at useful moments without chattering, and the
//! change of posture of a neighbouring house is announced in the chronicle.

use data_model::test_support::{fac, game_data};
use sim_campaign::test_support::{idle, start_quiet};
use sim_campaign::EventKind;

#[test]
fn the_counsellor_points_out_idle_sites_once_in_a_while() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 11);
    assert!(
        state.advisor.notes.is_empty(),
        "silent before the first season"
    );
    let mut said: Vec<(u32, String)> = Vec::new();
    for _ in 0..10 {
        state.end_turn_with(data, idle);
        assert!(
            state.advisor.notes.len() <= data.advisor_rules.max_per_season as usize,
            "never more than the bound per season"
        );
        for note in &state.advisor.notes {
            said.push((state.turn, note.id.clone()));
        }
    }
    let build: Vec<u32> = said
        .iter()
        .filter(|(_, id)| id == "build_opportunity")
        .map(|(turn, _)| *turn)
        .collect();
    assert!(
        !build.is_empty(),
        "France starts rich with idle sites: {said:?}"
    );
    let cooldown = data
        .advisor_rules
        .advices
        .iter()
        .find(|a| a.id == "build_opportunity")
        .unwrap()
        .cooldown_seasons;
    for pair in build.windows(2) {
        assert!(pair[1] - pair[0] >= cooldown, "{build:?}");
    }
}

#[test]
fn a_change_of_doctrine_of_a_neighbour_is_announced() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 11);
    let england = fac("fac_england");
    // England was following another posture last time the counsellor looked.
    let now = sim_campaign::doctrines::active(&state, &england)
        .id
        .to_owned();
    let before = if now == "balanced" {
        "chevauchee_pillage"
    } else {
        "balanced"
    };
    state
        .advisor
        .doctrines
        .insert(england.clone(), before.to_owned());
    let events = state.end_turn_with(data, idle);
    let news: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EventKind::Diplomacy && e.text_fr.contains("change de posture"))
        .collect();
    assert_eq!(news.len(), 1, "{events:#?}");
    assert!(
        news[0].text_fr.contains(&data.faction_name(&england)),
        "{}",
        news[0].text_fr
    );
    assert_eq!(state.advisor.doctrines[&england], now);
    // Said once: the next season is silent.
    let again = state.end_turn_with(data, idle);
    assert!(!again
        .iter()
        .any(|e| e.text_fr.contains("change de posture")));
}
