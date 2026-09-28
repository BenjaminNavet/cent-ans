//! FE war escalation and private war (spec § 4.3), filled by F2.
#![allow(dead_code)] // helpers used once the lot fills the tests

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

#[allow(dead_code)]
fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

#[allow(dead_code)]
fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), 7).expect("1337 start")
}

#[test]
#[ignore = "F2"]
fn count_duke_king_chain() {}

#[test]
#[ignore = "F2"]
fn liege_shirks_protection() {}

#[test]
#[ignore = "F2"]
fn cascade_stops_at_a_shirking_liege() {}

#[test]
#[ignore = "F2"]
fn private_war_is_arbitrated_by_the_common_liege() {}
