//! WR `ai-diplo` (ADR 0302): the AI eases a province on the brink of revolt to
//! « Bas » and takes the realm's bracket back once it is calm.

use data_model::test_support::fac;
use data_model::ProvinceId;
use sim_campaign::{CampaignState, Order, TaxRate};

fn set_unrest(state: &mut CampaignState, province: &ProvinceId, value: u8) {
    let classes = &mut state.provinces.get_mut(province).unwrap().population;
    classes.peasants.unrest = value;
    classes.burghers.unrest = value;
    classes.clergy.unrest = value;
    classes.nobility.unrest = value;
}

fn province_orders(orders: &[Order], province: &ProvinceId) -> Vec<Option<TaxRate>> {
    orders
        .iter()
        .filter_map(|o| match o {
            Order::SetProvinceTax { province: p, rate } if p == province => Some(*rate),
            _ => None,
        })
        .collect()
}

#[test]
fn a_province_on_the_brink_is_eased_then_released_when_calm() {
    let data = super::common::data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 7).expect("start");
    state.chronicle.disabled = true;
    let province = state
        .provinces
        .keys()
        .find(|p| state.controls_province(&france, p))
        .cloned()
        .expect("France holds a province");
    state.factions.get_mut(&france).unwrap().tax_rate = TaxRate::Normal;
    let threshold = data_model::AiCampaign::bundled()
        .economy
        .province_tax_relief_unrest;
    assert!(threshold <= 75.0, "the pass is on: {threshold}");

    set_unrest(&mut state, &province, 85);
    let orders = ai::plan_turn(&state, data, &france);
    assert_eq!(
        province_orders(&orders, &province),
        vec![Some(TaxRate::Low)]
    );

    // Calm: a province that carries its own bracket goes back to the realm's.
    sim_campaign::province_tax::set_province_tax(
        &mut state,
        &france,
        &province,
        Some(TaxRate::Low),
    )
    .unwrap();
    state.turn += 1;
    set_unrest(&mut state, &province, 5);
    let orders = ai::plan_turn(&state, data, &france);
    assert_eq!(province_orders(&orders, &province), vec![None]);

    // Still grumbling: it keeps its bracket (no order).
    set_unrest(&mut state, &province, 50);
    let orders = ai::plan_turn(&state, data, &france);
    assert!(province_orders(&orders, &province).is_empty());
}
