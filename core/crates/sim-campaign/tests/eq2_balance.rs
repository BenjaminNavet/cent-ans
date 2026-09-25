//! Lot EQ2 (balance) integration tests: the province disorder gauge calms
//! down in a few seasons, starting settlements hold one step per building
//! chain, and a later step of a chain satisfies what requires an earlier one.

use std::path::PathBuf;

use data_model::{BuildingId, FactionId, GameData, ProvinceId, SettlementId};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn bld(id: &str) -> BuildingId {
    BuildingId::new(id).unwrap()
}

/// A province at 100 disorder left in peace falls back under 40 within
/// eight seasons (it stayed at 100 for years at 2 points a season).
#[test]
fn a_pacified_province_calms_down_in_a_few_seasons() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
    state.chronicle.disabled = true;
    let province = ProvinceId::new("prov_kent").unwrap();
    state.provinces.get_mut(&province).unwrap().unrest = 100;
    let mut seasons = 0;
    while state.provinces[&province].unrest >= 40 {
        state.end_turn(&data);
        seasons += 1;
        assert!(
            seasons <= 8,
            "still {} after {seasons} seasons",
            state.provinces[&province].unrest
        );
    }
}

/// No starting settlement holds two steps of one upgrade chain, two
/// branches of one chain, a building its kind forbids or a building whose
/// required building is missing: every starting state can be reached in
/// play.
#[test]
fn starting_settlements_hold_reachable_building_tiers() {
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
    let mut problems = Vec::new();
    for (id, settlement) in &state.settlements {
        let list = &settlement.buildings;
        for building in list {
            let Some(def) = data.buildings.get(building) else {
                problems.push(format!("{id}: unknown {building}"));
                continue;
            };
            if list
                .iter()
                .any(|other| other != building && data.building_satisfies(other, building))
            {
                problems.push(format!("{id}: {building} stacked under a later step"));
            }
            if !def.allowed_in(settlement.kind) {
                problems.push(format!(
                    "{id}: {building} not allowed in {:?}",
                    settlement.kind
                ));
            }
            if let Some(required) = &def.required_building {
                if !data.has_building(list, required) {
                    problems.push(format!("{id}: {building} without {required}"));
                }
            }
        }
        if data.normalize_building_tiers(list) != *list {
            problems.push(format!("{id}: two branches of one chain"));
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
    // Paris keeps the top of its chains (the fair, the cathedral).
    let paris = &state.settlements[&SettlementId::new("set_paris").unwrap()];
    assert!(paris.buildings.contains(&bld("bld_fair")));
    assert!(paris.buildings.contains(&bld("bld_cathedral")));
    assert!(!paris.buildings.contains(&bld("bld_market")));
}

/// A fair counts as the guild hall it replaced (culveriners and Flemish
/// pikemen are raised at the guild hall), a cathedral as the parish church.
#[test]
fn a_later_step_satisfies_an_earlier_requirement() {
    let data = data();
    assert!(data.building_satisfies(&bld("bld_fair"), &bld("bld_guild_hall")));
    assert!(data.building_satisfies(&bld("bld_fair"), &bld("bld_market")));
    assert!(data.building_satisfies(&bld("bld_cathedral"), &bld("bld_parish_church")));
    assert!(!data.building_satisfies(&bld("bld_market"), &bld("bld_fair")));
    assert!(!data.building_satisfies(&bld("bld_abbey"), &bld("bld_cathedral")));
    let stacked = [
        bld("bld_market"),
        bld("bld_guild_hall"),
        bld("bld_fair"),
        bld("bld_abbey"),
        bld("bld_cathedral"),
        bld("bld_windmill"),
    ];
    assert_eq!(
        data.normalize_building_tiers(&stacked),
        vec![bld("bld_fair"), bld("bld_cathedral"), bld("bld_windmill")]
    );
}
