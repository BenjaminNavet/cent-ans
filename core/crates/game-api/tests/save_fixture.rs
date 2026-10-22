//! A frozen real save (wrapper v1 around campaign state v9, England, seed 1337, 10 turns) must
//! keep loading. Rule: every bump of `STATE_VERSION` or `SAVE_VERSION` adds a new fixture and
//! keeps this one loading through the migration chain.
//!
//! Regenerate (only for a deliberate new fixture): `CENT_ANS_WRITE_FIXTURE=1 cargo test -p
//! game-api --test save_fixture -- write_fixture`.

use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/save_v9.json")
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
    let path = game_api::saves::write_save(&dir, "save_v9", &state, "2026-10-22T00:00:00").unwrap();
    std::fs::create_dir_all(fixture().parent().unwrap()).unwrap();
    std::fs::copy(path, fixture()).unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn frozen_v9_save_loads_plays_a_turn_and_resaves() {
    let data = data_model::test_support::game_data();
    let mut state = game_api::saves::load_save(data, &fixture()).expect("fixture v9 loads");
    assert_eq!(state.turn(), 10);
    assert_eq!(state.player_faction().as_str(), "fac_england");
    state.end_turn(data);
    assert_eq!(state.turn(), 11);
    let dir = std::env::temp_dir().join(format!("cent-ans-fixture-rt-{}", std::process::id()));
    let path = game_api::saves::write_save(&dir, "again", &state, "2026-10-22T00:00:01").unwrap();
    let reloaded = game_api::saves::load_save(data, &path).unwrap();
    assert_eq!(reloaded.save_json(), state.save_json());
    let _ = std::fs::remove_dir_all(dir);
}
