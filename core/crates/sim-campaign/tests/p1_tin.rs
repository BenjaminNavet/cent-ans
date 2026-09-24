//! P1: tin (`res_tin`, Cornwall and Devon) feeds the tin blowing house.

use std::path::PathBuf;

use data_model::{BuildingId, FactionId, GameData, ProvinceId};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn option(state: &CampaignState, data: &GameData, province: &str) -> sim_campaign::BuildOption {
    let blowing_house = BuildingId::new("bld_tin_blowing_house").unwrap();
    state
        .buildable_in_province(data, &ProvinceId::new(province).unwrap())
        .into_iter()
        .find(|o| o.building == blowing_house)
        .expect("the blowing house is listed")
}

#[test]
fn tin_blowing_house_needs_tin() {
    let data = data();
    let state = CampaignState::new_1337(&data, FactionId::new("fac_england").unwrap(), 1).unwrap();
    for province in ["prov_cornwall", "prov_devon"] {
        let option = option(&state, &data, province);
        assert!(option.available, "{province}: {:?}", option.reason);
    }
    // Normandy has neither tin nor an English lord.
    let normandy = option(&state, &data, "prov_normandie");
    assert!(!normandy.available);
    // Kent is English but has no tin.
    let kent = option(&state, &data, "prov_kent");
    assert!(!kent.available);
    assert_eq!(kent.reason.as_deref(), Some("ressource requise absente"));
}
