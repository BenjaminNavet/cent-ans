//! F1 « règles inertes » integration tests: building, technology and trait
//! effects that used to be displayed without effect, allied armies joining
//! battles, and the new chronicle effects (capture, ransom, delayed events).
//! See `docs/design/v2-finalisation.md` (lot F1).

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn france(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start")
}

/// A planner that does nothing: only the orders the test submits apply.
#[allow(dead_code)]
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

#[test]
fn data_loads_for_f1() {
    let data = data();
    let _ = france(&data, 1);
}
