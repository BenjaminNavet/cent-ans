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
