//! Lot RS-B (ADR 0100): the economy constants come from `data/rules/economy.json`,
//! and the garrison that keeps a province in order and its plague resistance weigh
//! each place's kind `province_effect_percent`, like the province-wide building
//! effects of lot DC3.

use std::path::PathBuf;

use data_model::{BuildingId, FactionId, GameData, ProvinceId, SettlementKind};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 7).expect("1337 start")
}

/// A province whose controller holds a garrisoned secondary place.
fn province_with_secondary_garrison(state: &CampaignState) -> ProvinceId {
    state
        .provinces
        .keys()
        .find(|id| {
            let controller = state.province_controller(id).cloned();
            state.settlements_of(id).any(|(_, s)| {
                s.kind != SettlementKind::City
                    && Some(&s.controller) == controller.as_ref()
                    && s.garrison_strength() > 0
            })
        })
        .cloned()
        .expect("a secondary place with a garrison")
}

#[test]
fn secondary_garrisons_weigh_half_in_public_order() {
    let mut data = data();
    let state = start(&data);
    let province = province_with_secondary_garrison(&state);
    let controller = state.province_controller(&province).cloned().unwrap();
    let expected: u32 = state
        .settlements_of(&province)
        .filter(|(_, s)| s.controller == controller)
        .map(|(_, s)| {
            let percent = sim_campaign::buildings::province_effect_percent(&data, s.kind);
            s.garrison_strength() * percent
        })
        .sum::<u32>()
        / 100;
    let weighted = state.weighted_garrison_strength(&data, &province);
    assert_eq!(weighted, expected);
    let raw = state.province_garrison_strength(&province);
    assert!(weighted < raw, "the secondary garrison counts for less");
    if let Some(rules) = data.settlement_rules.as_mut() {
        rules.province_effect_percent.clear();
    }
    assert_eq!(state.weighted_garrison_strength(&data, &province), raw);
}

/// Plague resistance of the capital's province after an apothecary is added to a
/// place of `kind` there.
fn resistance_with_apothecary(data: &GameData, kind: SettlementKind) -> (f64, f64) {
    let mut state = start(data);
    let apothecary = BuildingId::new("bld_apothecary").unwrap();
    let france = FactionId::new("fac_france").unwrap();
    let (place, province) = state
        .settlements
        .iter()
        .filter(|(_, s)| s.controller == france && s.kind == kind)
        .filter(|(_, s)| !state.province_buildings(&s.province).contains(&apothecary))
        .map(|(id, s)| (id.clone(), s.province.clone()))
        .next()
        .expect("a French place without apothecary in its province");
    let before = sim_campaign::medicine::plague_resistance(&state, data, &province);
    state
        .settlements
        .get_mut(&place)
        .unwrap()
        .buildings
        .push(apothecary);
    (
        before,
        sim_campaign::medicine::plague_resistance(&state, data, &province),
    )
}

#[test]
fn a_village_apothecary_resists_plague_less_than_a_city_one() {
    let data = data();
    let (before, after) = resistance_with_apothecary(&data, SettlementKind::City);
    let city_gain = after - before;
    let (before, after) = resistance_with_apothecary(&data, SettlementKind::Village);
    let village_gain = after - before;
    assert!(city_gain > 0.0);
    let weight = f64::from(sim_campaign::buildings::province_effect_percent(
        &data,
        SettlementKind::Village,
    )) / 100.0;
    assert!(
        (village_gain - city_gain * weight).abs() < 1e-9,
        "village {village_gain} vs city {city_gain}"
    );
}

#[test]
fn tax_efficiency_is_read_from_the_data() {
    let mut data = data();
    let state = start(&data);
    let paris = ProvinceId::new("prov_ile_de_france").unwrap();
    let province = state.provinces.get(&paris).expect("Île-de-France");
    let income = sim_campaign::economy::province_income(&data.economy_rules, province);
    assert!(income > 0.0);
    data.economy_rules.tax_efficiency *= 2.0;
    let doubled = sim_campaign::economy::province_income(&data.economy_rules, province);
    assert!((doubled - 2.0 * income).abs() < 1e-6);
}

#[test]
fn tax_brackets_come_from_the_data() {
    use sim_campaign::economy::TaxRate;
    let data = data();
    let rules = &data.economy_rules;
    assert!(TaxRate::Low.multiplier(rules) < TaxRate::Normal.multiplier(rules));
    assert!(TaxRate::Normal.multiplier(rules) < TaxRate::High.multiplier(rules));
    assert!(TaxRate::Low.burden(rules) < TaxRate::High.burden(rules));
    assert_eq!(rules.upkeep_months_per_season, 4);
}
