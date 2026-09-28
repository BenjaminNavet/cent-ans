//! FE deductions (spec § 3.3), obligations and loyalty (§ 4.1-4.2): F0 deductions active, F1 cases ignored until F1.

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

use sim_campaign::feudal;

#[test]
fn guyenne_owes_allegiance_to_england_then_france() {
    let data = data();
    let state = start(&data);
    assert_eq!(
        feudal::province_lieges(&state, &data, &prov("prov_guyenne")),
        vec![fac("fac_england"), fac("fac_france")]
    );
    assert_eq!(feudal::liege_of(&state, &data, &fac("fac_england")), None);
    assert!(feudal::title_vassals(&state, &data, &fac("fac_france")).contains(&fac("fac_england")));
}

#[test]
fn deduced_liege_matches_the_1337_suzerains() {
    let data = data();
    let state = start(&data);
    for (id, faction) in &data.factions {
        if let Some(suzerain) = &faction.suzerain {
            assert_eq!(
                feudal::liege_of(&state, &data, id).as_ref(),
                Some(suzerain),
                "{id}"
            );
        }
    }
    let vassals = feudal::direct_vassals(&state, &data, &fac("fac_france"));
    for v in ["fac_brittany", "fac_burgundy", "fac_flanders"] {
        assert!(vassals.contains(&fac(v)), "{v}");
    }
}

#[test]
#[ignore = "F1: suzerain derived from titles"]
fn state_suzerain_is_a_view_of_the_titles() {}

#[test]
#[ignore = "F1: maxim"]
fn king_does_not_levy_his_dukes_counts() {}

#[test]
#[ignore = "F1: tribute to the direct liege only"]
fn tribute_goes_to_the_direct_liege_only() {}

#[test]
#[ignore = "F1: loyalty thresholds from feudal.json"]
fn loyalty_thresholds_come_from_the_rules() {}

#[test]
fn title_holdings_survive_a_save_and_older_saves_are_refused() {
    let data = data();
    let state = start(&data);
    assert!(!state.feudal.holders.is_empty());
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).expect("round trip");
    assert_eq!(loaded.feudal, state.feudal);
    let old = json.replacen("\"state_version\":7", "\"state_version\":6", 1);
    assert!(matches!(
        CampaignState::load_json(&old),
        Err(sim_campaign::save::CampaignError::PreFeudalSave { found: 6, .. })
    ));
}
