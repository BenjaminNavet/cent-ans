//! FE title transfers, forfeiture, inheritance, conquest, objectives (spec § 4.4-4.8), filled by F3.
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
#[ignore = "F3"]
fn guyenne_forfeiture() {}

#[test]
#[ignore = "F3"]
fn brittany_1341_two_claimants() {}

#[test]
#[ignore = "F3"]
fn burgundy_1361_duchy_and_county_split() {}

#[test]
#[ignore = "F3"]
fn personal_union() {}

#[test]
#[ignore = "F3"]
fn escheat_without_heir() {}

#[test]
#[ignore = "F3"]
fn victory_by_independence() {}
