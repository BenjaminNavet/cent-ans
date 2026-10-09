//! Lot WH `econ`: readable economy (unrest terms, income breakdown,
//! per-province tax, costly edicts, slot cap, city buildings).

use data_model::test_support::{fac, game_data};
use sim_campaign::population::{equilibrium, unrest_breakdown};
use sim_campaign::CampaignState;

#[test]
fn unrest_terms_sum_to_the_equilibrium_target() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    // A taxed ("high") faction and a ravaged province.
    state.factions.get_mut(&fac("fac_france")).unwrap().tax_rate =
        sim_campaign::economy::TaxRate::High;
    let ids: Vec<_> = state.provinces.keys().take(40).cloned().collect();
    state.provinces.get_mut(&ids[3]).unwrap().devastation = 60;
    for id in &ids {
        let targets = equilibrium(&state, data, id).expect("equilibrium");
        let terms = unrest_breakdown(&state, data, id).expect("terms");
        assert_eq!(targets.len(), terms.len());
        for ((class, _, gauges), (term_class, rows)) in targets.iter().zip(&terms) {
            assert_eq!(class, term_class);
            let sum: f64 = rows.iter().map(|(_, v)| v).sum();
            assert!((sum.clamp(0.0, 100.0) - gauges.unrest).abs() < 1e-9);
        }
    }
}

#[test]
fn income_breakdown_adds_up_everywhere() {
    use sim_campaign::economy::TaxRate;
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    let first = state.provinces.keys().next().cloned().unwrap();
    state.provinces.get_mut(&first).unwrap().devastation = 40;
    for faction in ["fac_france", "fac_england", "fac_burgundy", "fac_luna"] {
        let faction = fac(faction);
        if let Some(f) = state.factions.get_mut(&faction) {
            f.tax_rate = TaxRate::High;
        }
        let breakdown = state.faction_income_breakdown(data, &faction);
        assert_eq!(breakdown.total(), state.faction_income(data, &faction));
        assert_eq!(
            breakdown.domain,
            state.faction_domain_income(data, &faction)
        );
        let tech = sim_campaign::research::faction_province_tech_effects(&state, data, &faction);
        let rate = state.factions[&faction].tax_rate;
        for id in state.provinces.keys() {
            let lines = state.province_income_breakdown(data, id, &faction, rate, &tech);
            assert_eq!(
                lines.total(),
                state.province_gross_income(data, id, &faction, rate, &tech)
            );
        }
    }
    // Settlements: the shares add up to the displayed figure.
    let faction = fac("fac_france");
    let tech = sim_campaign::research::faction_province_tech_effects(&state, data, &faction);
    for id in state.settlements.keys() {
        let lines = state.settlement_income_breakdown(data, id, TaxRate::Normal, &tech);
        let expected = state
            .settlement_tax(data, id, TaxRate::Normal, &tech)
            .round() as i64;
        assert_eq!(lines.total(), expected, "{id}");
    }
}

#[test]
fn a_province_taxed_low_pays_less_and_grumbles_less() {
    use sim_campaign::economy::TaxRate;
    use sim_campaign::Order;
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let province = state
        .controlled_provinces(&france)
        .find(|id| state.holds_whole_province(&france, id))
        .cloned()
        .unwrap();
    let tech = sim_campaign::research::faction_province_tech_effects(&state, data, &france);
    let normal = state.province_gross_income(data, &province, &france, TaxRate::Normal, &tech);
    let unrest = |s: &CampaignState| {
        sim_campaign::population::equilibrium(s, data, &province).unwrap()[0]
            .2
            .unrest
    };
    let unrest_normal = unrest(&state);
    state
        .submit_order(
            data,
            Order::SetProvinceTax {
                province: province.clone(),
                rate: Some(TaxRate::Low),
            },
        )
        .expect("order accepted");
    let low = state.province_gross_income(data, &province, &france, TaxRate::Normal, &tech);
    assert!(low < normal, "{low} < {normal}");
    let ratio = low as f64 / normal as f64;
    let expected = TaxRate::Low.multiplier(&data.economy_rules);
    assert!((ratio - expected).abs() < 0.1, "{ratio} vs {expected}");
    assert!(unrest(&state) <= unrest_normal);
    // Once per turn.
    assert!(state
        .submit_order(
            data,
            Order::SetProvinceTax {
                province: province.clone(),
                rate: Some(TaxRate::High),
            },
        )
        .is_err());
    // Other provinces keep the faction bracket.
    let other = state
        .controlled_provinces(&france)
        .find(|id| **id != province)
        .cloned()
        .unwrap();
    assert_eq!(state.effective_province_tax(&other), TaxRate::Normal);
    assert_eq!(state.effective_province_tax(&province), TaxRate::Low);
}

fn whole_french_province(state: &CampaignState) -> data_model::ProvinceId {
    let france = fac("fac_france");
    let found = state
        .controlled_provinces(&france)
        .find(|id| state.holds_whole_province(&france, id))
        .cloned();
    found.expect("a wholly held province")
}

#[test]
fn a_costly_edict_needs_its_building_and_is_paid_each_season() {
    use data_model::EdictId;
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let province = whole_french_province(&state);
    let city = state.province_city_id(&province).cloned().unwrap();
    let edict = EdictId::new("edict_free_fair").unwrap();
    let market = data_model::BuildingId::new("bld_market").unwrap();
    // Without a market (or better) the fair is refused, with its reason.
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .buildings
        .retain(|b| !data.building_satisfies(b, &market));
    let refusal = sim_campaign::edicts::set_edict(&mut state, data, &france, &province, &edict)
        .expect_err("no market");
    assert!(refusal.to_string().contains("Marché"), "{refusal}");
    let option = state
        .edict_options(data, &province)
        .into_iter()
        .find(|o| o.edict == edict)
        .unwrap();
    assert!(!option.available);
    assert_eq!(option.cost_money, 50);
    assert!(option.requirements.iter().any(|(_, met)| !met));
    // With one, it is adopted, and the court pays it once it is in force.
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .buildings
        .push(market);
    let before = sim_campaign::economy::TurnBudget::compute(&state, data, &france);
    sim_campaign::edicts::set_edict(&mut state, data, &france, &province, &edict).unwrap();
    state.turn += 5;
    assert_eq!(state.faction_edict_upkeep(data, &france), 50);
    let after = sim_campaign::economy::TurnBudget::compute(&state, data, &france);
    assert_eq!(after.administration - before.administration, 50);
}

#[test]
fn an_edict_the_purse_cannot_carry_is_dropped() {
    use data_model::EdictId;
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let province = whole_french_province(&state);
    let city = state.province_city_id(&province).cloned().unwrap();
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .buildings
        .push(data_model::BuildingId::new("bld_market").unwrap());
    let edict = EdictId::new("edict_free_fair").unwrap();
    sim_campaign::edicts::set_edict(&mut state, data, &france, &province, &edict).unwrap();
    state.turn += 5;
    assert_eq!(state.province_edict(data, &province), edict);
    state.factions.get_mut(&france).unwrap().treasury = -1_000_000;
    state.end_turn_with(data, |_, _, _| Vec::new());
    assert_ne!(state.province_edict(data, &province), edict);
}

#[test]
fn the_inquisition_costs_prestige_once() {
    use data_model::EdictId;
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let province = whole_french_province(&state);
    let ruler = state.factions[&france].ruler.clone().expect("ruler");
    state.characters.get_mut(&ruler).unwrap().prestige = 2;
    let edict = EdictId::new("edict_inquisition").unwrap();
    let refused = sim_campaign::edicts::set_edict(&mut state, data, &france, &province, &edict);
    assert!(refused.unwrap_err().to_string().contains("prestige"));
    state.characters.get_mut(&ruler).unwrap().prestige = 12;
    sim_campaign::edicts::set_edict(&mut state, data, &france, &province, &edict).unwrap();
    assert_eq!(state.characters[&ruler].prestige, 7);
}

#[test]
fn the_slot_cap_stops_new_chains_but_not_upgrades() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    // A village (cap 2): fill it with two chains, the third is refused.
    let village = state
        .settlements
        .iter()
        .find(|(_, s)| {
            s.kind == data_model::SettlementKind::Village
                && s.controller == france
                && s.construction.is_none()
        })
        .map(|(id, _)| id.clone())
        .expect("a French village");
    let cap = data.settlement_rules.as_ref().unwrap().building_slot_cap
        [&data_model::SettlementKind::Village];
    state
        .settlements
        .get_mut(&village)
        .unwrap()
        .buildings
        .clear();
    state.factions.get_mut(&france).unwrap().treasury = 1_000_000;
    let usage = state.slot_usage(data, &village);
    assert_eq!((usage.used, usage.max), (0, Some(cap)));
    let roots: Vec<_> = state
        .buildable(data, &village)
        .into_iter()
        .filter(|o| o.available)
        .map(|o| o.building)
        .collect();
    assert!(roots.len() > cap, "enough chains to test the cap");
    for building in &roots[..cap] {
        state
            .settlements
            .get_mut(&village)
            .unwrap()
            .buildings
            .push(building.clone());
    }
    assert!(state.slot_usage(data, &village).full());
    let blocked: Vec<_> = state
        .buildable(data, &village)
        .into_iter()
        .filter(|o| {
            o.reason
                .as_deref()
                .is_some_and(|r| r.starts_with("emplacements pleins"))
        })
        .collect();
    assert!(!blocked.is_empty(), "new chains are refused");
    // Upgrades of what stands are not.
    let stands = &state.settlements[&village].buildings;
    let upgrades = data
        .buildings
        .values()
        .filter(|b| b.allowed_in(data_model::SettlementKind::Village))
        .filter(|b| b.upgrades_from.as_ref().is_some_and(|p| stands.contains(p)))
        .count();
    let offered = state
        .buildable(data, &village)
        .iter()
        .filter(|o| {
            data.buildings[&o.building]
                .upgrades_from
                .as_ref()
                .is_some_and(|p| stands.contains(p))
                && !o
                    .reason
                    .as_deref()
                    .unwrap_or("")
                    .starts_with("emplacements pleins")
        })
        .count();
    assert_eq!(upgrades, offered);
}

#[test]
fn city_buildings_are_kept_to_cities_and_minor_ones_to_minor_places() {
    use data_model::SettlementKind::*;
    let data = game_data();
    for id in [
        "bld_town_hall",
        "bld_bailiwick",
        "bld_corn_hall",
        "bld_provostry",
    ] {
        let building = &data.buildings[&data_model::BuildingId::new(id).unwrap()];
        assert!(building.allowed_in(City), "{id}");
        assert!(
            !building.allowed_in(Village) && !building.allowed_in(Town),
            "{id}"
        );
    }
    for id in [
        "bld_tithe_barn",
        "bld_banal_oven",
        "bld_toll_post",
        "bld_manor_chapel",
    ] {
        let building = &data.buildings[&data_model::BuildingId::new(id).unwrap()];
        assert!(!building.allowed_in(City), "{id}");
        assert!(building.allowed_in(Village), "{id}");
    }
}
