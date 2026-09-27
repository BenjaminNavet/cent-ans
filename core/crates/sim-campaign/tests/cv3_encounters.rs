//! Lot CV3-3: map encounters (spawn, expiry, trigger, choices, battle and
//! join outcomes, vision, old saves, data references).

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn start(data: &GameData, faction: &str, seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac(faction), seed).expect("1337 start");
    state.chronicle.disabled = true;
    state
}

#[test]
#[ignore = "CV3-3 skeleton"]
fn sites_spawn_within_the_rules() {
    let data = data();
    let _ = start(&data, "fac_france", 1);
}
