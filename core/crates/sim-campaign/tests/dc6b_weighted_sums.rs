//! Lot DC6b (ADR 0082): the research points of the secondary places weigh their
//! kind's `research_percent`, their religious buildings against heresy their kind's
//! `province_effect_percent` (like the province-wide effects of lot DC3).

use std::path::PathBuf;

use data_model::{BuildingId, FactionId, GameData, SettlementKind};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 7).expect("1337 start")
}

/// Research of France after adding ten scriptoriums (1 point each) to places of `kind`.
fn research_with_scriptoriums(data: &GameData, kind: SettlementKind) -> (u32, u32) {
    let mut state = start(data);
    let france = FactionId::new("fac_france").unwrap();
    let scriptorium = BuildingId::new("bld_scriptorium").unwrap();
    let before = state.research_points_per_turn(data, &france);
    let places: Vec<_> = state
        .settlements
        .iter()
        .filter(|(_, s)| {
            s.controller == france && s.kind == kind && !s.buildings.contains(&scriptorium)
        })
        .map(|(id, _)| id.clone())
        .take(10)
        .collect();
    assert_eq!(places.len(), 10, "ten French places of kind {kind:?}");
    for place in places {
        state
            .settlements
            .get_mut(&place)
            .unwrap()
            .buildings
            .push(scriptorium.clone());
    }
    (before, state.research_points_per_turn(data, &france))
}

#[test]
fn an_abbey_library_counts_less_than_a_city_one() {
    let data = data();
    let weight = sim_campaign::buildings::research_percent(&data, SettlementKind::Abbey);
    assert!(weight < 100, "abbeys weigh less than cities");
    let (before, after) = research_with_scriptoriums(&data, SettlementKind::Abbey);
    assert_eq!(after, before + weight / 10, "ten abbey scriptoriums");
    let (before, after) = research_with_scriptoriums(&data, SettlementKind::City);
    assert_eq!(after, before + 10, "ten city scriptoriums weigh ten points");
}

#[test]
fn full_weights_count_every_library() {
    let mut data = data();
    if let Some(rules) = data.settlement_rules.as_mut() {
        rules.research_percent.clear();
    }
    let (before, after) = research_with_scriptoriums(&data, SettlementKind::Abbey);
    assert_eq!(after, before + 10);
}

/// Religious buildings of `province`, each place weighed by `weight`.
fn religious_of(
    state: &CampaignState,
    data: &GameData,
    province: &data_model::ProvinceId,
    weight: impl Fn(SettlementKind) -> f64,
) -> f64 {
    state
        .settlements
        .values()
        .filter(|s| &s.province == province)
        .map(|s| {
            let count = s
                .buildings
                .iter()
                .filter_map(|b| data.buildings.get(b))
                .filter(|b| b.category == data_model::BuildingCategory::Religious)
                .count();
            count as f64 * weight(s.kind)
        })
        .sum()
}

#[test]
fn secondary_churches_weigh_half_against_heresy() {
    let mut data = data();
    let state = start(&data);
    let oxford = data_model::ProvinceId::new("prov_oxford").unwrap();
    let weighed = sim_campaign::religion::weighted_religious_buildings(&state, &data, &oxford);
    let half = |k: SettlementKind| if k == SettlementKind::City { 1.0 } else { 0.5 };
    assert!((weighed - religious_of(&state, &data, &oxford, half)).abs() < 1e-9);
    let raw = religious_of(&state, &data, &oxford, |_| 1.0);
    assert!(weighed < raw, "Oxford has secondary churches");
    if let Some(rules) = data.settlement_rules.as_mut() {
        rules.province_effect_percent.clear();
    }
    let full = sim_campaign::religion::weighted_religious_buildings(&state, &data, &oxford);
    assert!((full - raw).abs() < 1e-9);
}
