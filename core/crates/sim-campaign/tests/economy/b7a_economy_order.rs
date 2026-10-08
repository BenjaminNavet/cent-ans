//! Lot B7a: rules the interface describes — devastated land feeds armies
//! badly, the court and bankruptcy tuning is read from `data/`.

use data_model::{GameData, SettlementId};
use sim_campaign::test_support::idle;
use sim_campaign::{ArmyId, ArmyPosition, CampaignState};

use data_model::test_support::{fac, game_data};

fn english_army(state: &CampaignState) -> ArmyId {
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == fac("fac_england"))
        .max_by_key(|(_, a)| a.total_strength())
        .map(|(id, _)| id.clone())
        .expect("an English army")
}

/// Supply of the main English army after one season on `settlement`,
/// starting at `supply`, the settlement's province devastated at
/// `devastation`.
fn supply_after_a_season(data: &GameData, settlement: &str, supply: u8, devastation: u8) -> u8 {
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 8).unwrap();
    let army = english_army(&state);
    let settlement = SettlementId::new(settlement).unwrap();
    let province = state.settlements[&settlement].province.clone();
    state.provinces.get_mut(&province).unwrap().devastation = devastation;
    let entry = state.armies.get_mut(&army).unwrap();
    entry.position = ArmyPosition::Settlement(settlement);
    entry.supply = supply;
    state.end_turn_with(data, idle);
    state.army(&army).expect("army survives").supply
}

#[test]
fn a_devastated_enemy_province_starves_armies_faster() {
    let data = game_data();
    let clean = supply_after_a_season(data, "set_boulogne", 100, 0);
    let ravaged = supply_after_a_season(data, "set_boulogne", 100, 100);
    assert!(clean < 100, "supply falls abroad ({clean})");
    let clean_loss = 100 - clean;
    let ravaged_loss = 100 - ravaged;
    let expected =
        f64::from(clean_loss) * (1.0 + data.economy_rules.supply_devastation_loss_percent / 100.0);
    assert!(
        (f64::from(ravaged_loss) - expected).abs() <= 1.0,
        "loss {ravaged_loss} in ravaged land, {clean_loss} in clean land"
    );
}

#[test]
fn a_devastated_home_province_feeds_armies_slower() {
    let data = game_data();
    let clean = supply_after_a_season(data, "set_cantorbery", 0, 0);
    let ravaged = supply_after_a_season(data, "set_cantorbery", 0, 100);
    assert!(clean >= 40, "full recovery at home ({clean})");
    let expected = f64::from(clean)
        * (1.0 - data.economy_rules.supply_devastation_recovery_cut_percent / 100.0);
    assert!(
        (f64::from(ravaged) - expected).abs() <= 1.0,
        "recovery {ravaged} in ravaged land, {clean} in clean land"
    );
}

#[test]
fn economy_tuning_comes_from_data() {
    let data = game_data();
    let rules = &data.economy_rules;
    assert_eq!(rules.opulence_seasons, 6);
    assert_eq!(rules.opulence_percent, 20);
    assert_eq!(rules.bankruptcy_morale_penalty, 10);
    assert!((rules.administration_rate(0) - rules.administration_base).abs() < 1e-9);
    assert!((rules.administration_rate(1000) - rules.administration_max).abs() < 1e-9);
}
