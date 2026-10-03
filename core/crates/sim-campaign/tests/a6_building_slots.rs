//! Lot A6-L15 (ADR 0181): the slot grid derived from the building chains.

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

#[test]
fn city_slots_cover_every_chain_once() {
    let data = data();
    let state = CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 1).unwrap();
    let province = state.provinces.keys().next().unwrap().clone();
    let city = state.province_city_id(&province).unwrap().clone();
    let slots = state.building_slots(&data, &city);
    assert!(
        slots.len() >= 10,
        "a city has many slots, got {}",
        slots.len()
    );
    // Each standing building belongs to exactly one slot.
    let live = state.settlement_state(&city).unwrap();
    for building in &live.buildings {
        let holders = slots
            .iter()
            .filter(|s| s.built.as_ref() == Some(building))
            .count();
        assert_eq!(holders, 1, "{building} must sit in one slot");
    }
    for slot in &slots {
        assert_eq!(slot.built.is_some(), slot.level > 0);
        assert!(slot.level <= slot.max_level);
        if slot.built.is_none() {
            assert_eq!(slot.next.len(), 1, "an empty slot offers its root");
        }
    }
}
