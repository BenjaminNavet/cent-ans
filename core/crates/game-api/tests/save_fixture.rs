//! Frozen real saves (wrapper v1 around campaign state v9 to v12, England, seed 1337, 10
//! turns) must keep loading. Rule: every bump of `STATE_VERSION` or `SAVE_VERSION` adds a new fixture and
//! keeps this one loading through the migration chain.
//!
//! Regenerate (only for a deliberate new fixture): `CENT_ANS_WRITE_FIXTURE=1 cargo test -p
//! game-api --test save_fixture -- write_fixture`.

use std::path::PathBuf;

fn fixture(version: u32) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/save_v{version}.json"))
}

#[test]
fn write_fixture() {
    if std::env::var_os("CENT_ANS_WRITE_FIXTURE").is_none() {
        return;
    }
    let data = data_model::test_support::game_data();
    let mut state = sim_campaign::test_support::start_quiet(data, "fac_england", 1337);
    for _ in 0..10 {
        state.end_turn(data);
    }
    let dir = std::env::temp_dir().join(format!("cent-ans-fixture-{}", std::process::id()));
    let path =
        game_api::saves::write_save(&dir, "save_v12", &state, "2026-10-29T00:00:00").unwrap();
    std::fs::create_dir_all(fixture(12).parent().unwrap()).unwrap();
    std::fs::copy(path, fixture(12)).unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn frozen_v9_save_loads_plays_a_turn_and_resaves() {
    frozen_save_loads_plays_a_turn_and_resaves(9);
}

#[test]
fn frozen_v11_save_loads_plays_a_turn_and_resaves() {
    frozen_save_loads_plays_a_turn_and_resaves(11);
}

#[test]
fn frozen_v10_save_loads_plays_a_turn_and_resaves() {
    frozen_save_loads_plays_a_turn_and_resaves(10);
}

#[test]
fn frozen_v12_save_loads_plays_a_turn_and_resaves() {
    frozen_save_loads_plays_a_turn_and_resaves(12);
}

/// R1 (v11 -> v12): the renown of an older save restarts from the next season (its baseline is
/// taken then); the policy became the first ambition.
#[test]
fn a_v11_save_gains_its_renown_after_one_season() {
    let data = data_model::test_support::game_data();
    let mut state = game_api::saves::load_save(data, &fixture(11)).expect("v11 loads");
    state.end_turn(data);
    assert!(state.acts.renown.baseline_taken);
}

fn frozen_save_loads_plays_a_turn_and_resaves(version: u32) {
    let data = data_model::test_support::game_data();
    let mut state =
        game_api::saves::load_save(data, &fixture(version)).expect("frozen fixture loads");
    assert_eq!(state.turn(), 10);
    assert_eq!(state.player_faction().as_str(), "fac_england");
    state.end_turn(data);
    assert_eq!(state.turn(), 11);
    let dir = std::env::temp_dir().join(format!(
        "cent-ans-fixture-rt-{version}-{}",
        std::process::id()
    ));
    let path = game_api::saves::write_save(&dir, "again", &state, "2026-10-22T00:00:01").unwrap();
    let reloaded = game_api::saves::load_save(data, &path).unwrap();
    assert_eq!(reloaded.save_json(), state.save_json());
    let _ = std::fs::remove_dir_all(dir);
}
