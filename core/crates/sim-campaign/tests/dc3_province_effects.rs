//! Lot DC3 (ADR 0082): the buildings of a province's secondary places weigh their
//! kind's `province_effect_percent` in the province-wide effects (order, health,
//! goods, supply, growth room), the city's in full.

use std::path::PathBuf;

use data_model::{EffectKind, FactionId, GameData, ProvinceId, SettlementKind};
use sim_campaign::buildings::province_effect_percent;
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 7).expect("1337 start")
}

/// Unrest effect of the buildings of `province`, each place weighed by `weight`.
fn unrest_of(
    state: &CampaignState,
    data: &GameData,
    province: &ProvinceId,
    weight: impl Fn(SettlementKind) -> f64,
) -> f64 {
    state
        .settlements
        .values()
        .filter(|s| &s.province == province)
        .flat_map(|s| {
            let w = weight(s.kind);
            s.buildings
                .iter()
                .filter_map(|id| data.buildings.get(id))
                .flat_map(|b| b.effects.iter())
                .filter(|e| e.effect == EffectKind::Unrest && e.class.is_none())
                .map(move |e| e.value * w)
        })
        .sum()
}

#[test]
fn secondary_places_weigh_half_on_their_province() {
    let data = data();
    let state = start(&data);
    assert_eq!(province_effect_percent(&data, SettlementKind::City), 100);
    for kind in [
        SettlementKind::Town,
        SettlementKind::Castle,
        SettlementKind::Abbey,
        SettlementKind::Village,
    ] {
        assert_eq!(province_effect_percent(&data, kind), 50, "{kind:?}");
    }
    // A province whose secondary places hold appeasing buildings (parish churches, abbeys).
    let province = state
        .provinces
        .keys()
        .find(|p| {
            unrest_of(&state, &data, p, |k| {
                f64::from(u8::from(k != SettlementKind::City))
            }) < 0.0
        })
        .expect("some province has a church outside its city")
        .clone();
    let weighed = unrest_of(&state, &data, &province, |k| {
        f64::from(province_effect_percent(&data, k)) / 100.0
    });
    let full = unrest_of(&state, &data, &province, |_| 1.0);
    let effects = state.province_building_effects(&data, &province);
    let got = effects.unrest.flat + effects.unrest.percent;
    assert!(
        (got - weighed).abs() < 1e-9,
        "{province}: {got} vs {weighed}"
    );
    assert!(
        got > full,
        "{province}: the secondary places count for less ({got} vs {full})"
    );
}

#[test]
fn full_weights_keep_the_growth_room_of_before() {
    let mut data = data();
    let state = start(&data);
    let province = ProvinceId::new("prov_anjou").unwrap();
    let weighed = state.province_capacity(&data, &province);
    if let Some(rules) = data.settlement_rules.as_mut() {
        rules.province_effect_percent.clear();
    }
    let full = state.province_capacity(&data, &province);
    let buildings: Vec<_> = state
        .settlements
        .values()
        .filter(|s| s.province == province)
        .flat_map(|s| s.buildings.iter().cloned())
        .collect();
    assert_eq!(
        full,
        sim_campaign::buildings::capacity(&data, &province, &buildings)
    );
    assert!(weighed <= full);
}
