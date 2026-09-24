//! Lot C1 (settlements skeleton): every province starts with settlements and
//! exactly one city, and the settlement state survives a save round trip.
//! See `docs/design/2026-09-24-echelle-colonies.md` § 4.2 and § 8.

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementKind};
use sim_campaign::{CampaignState, STATE_VERSION};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn france(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 7).expect("1337 start")
}

#[test]
fn every_province_has_one_city_after_setup() {
    let data = data();
    let state = france(&data);
    assert!(!state.provinces.is_empty());
    for (province_id, province) in &state.provinces {
        let settlements = state.province_settlements(&data, province_id);
        assert!(!settlements.is_empty(), "{province_id} has no settlement");
        let cities: Vec<_> = settlements
            .iter()
            .filter(|(id, _)| data.settlements[*id].kind == SettlementKind::City)
            .collect();
        assert_eq!(cities.len(), 1, "{province_id} must have exactly one city");
        let (city_id, city) = cities[0];
        assert_eq!(settlements[0].0, *city_id, "the city comes first");
        // In 1337 the city follows its province; enclaves only elsewhere.
        assert_eq!(city.controller, city.owner);
        if data.settlements[*city_id].owner.is_none() {
            assert_eq!(city.owner, province.owner, "{city_id} owner");
        }
        assert!(
            city.garrison.is_empty(),
            "province garrison stays live in C1"
        );
        assert_eq!(
            state.settlement_state(city_id),
            Some(*city),
            "accessor agrees"
        );
    }
    assert_eq!(state.settlements.len(), data.settlements.len());
}

#[test]
fn settlements_survive_a_save_round_trip() {
    let data = data();
    let state = france(&data);
    assert_eq!(state.state_version, STATE_VERSION);
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).expect("save loads");
    assert_eq!(loaded.settlements, state.settlements);
}

#[test]
fn saves_without_settlements_still_load() {
    let data = data();
    let state = france(&data);
    let mut value: serde_json::Value = serde_json::from_str(&state.save_json()).unwrap();
    value.as_object_mut().unwrap().remove("settlements");
    let loaded = CampaignState::load_json(&value.to_string()).expect("older v4 save loads");
    assert!(loaded.settlements.is_empty());
}
