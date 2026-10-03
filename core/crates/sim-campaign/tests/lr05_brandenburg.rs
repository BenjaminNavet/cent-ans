//! Lot LR-05: every playable faction of 1337 holds at least one province at
//! turn 0 (the March of Brandenburg was stillborn from FE0 until HV8 gave its
//! capital back; these tests keep it that way).

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

#[test]
fn brandenburg_holds_the_march_at_turn_0() {
    let data = data();
    let state =
        CampaignState::new_1337(&data, FactionId::new("fac_papacy").unwrap(), 1).expect("setup");
    let march = ProvinceId::new("prov_brandenburg").unwrap();
    assert_eq!(
        state.province_owner(&march).map(|f| f.as_str()),
        Some("fac_brandenburg")
    );
}

#[test]
fn every_playable_faction_holds_a_province_at_turn_0() {
    let data = data();
    let state =
        CampaignState::new_1337(&data, FactionId::new("fac_papacy").unwrap(), 1).expect("setup");
    let landless: Vec<&str> = data
        .factions
        .values()
        .filter(|f| f.playable)
        // The crusade starts landless by design (ADR 0165).
        .filter(|f| f.id.as_str() != "fac_crusaders")
        .filter(|f| {
            !state
                .provinces
                .keys()
                .any(|p| state.province_owner(p) == Some(&f.id))
        })
        .map(|f| f.id.as_str())
        .collect();
    assert!(landless.is_empty(), "factions sans province : {landless:?}");
}
