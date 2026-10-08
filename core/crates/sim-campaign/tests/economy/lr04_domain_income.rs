//! Lot LR-04: the lord's demesne (`data/rules/economy.json` `domain_income`).
//! A one-province county taxing 50-80 livres could not pay a single garrison
//! unit beside its buildings (FE8 open point: Irish kingdoms, the Isles,
//! Luna, Urbino); the common demesne revenue lets it.

use data_model::SettlementKind;
use sim_campaign::CampaignState;

use data_model::test_support::{fac, game_data};

const POOR_COUNTIES: [&str; 8] = [
    "fac_connacht",
    "fac_desmond",
    "fac_leinster",
    "fac_thomond",
    "fac_tyrone",
    "fac_isles",
    "fac_luna",
    "fac_urbino",
];

#[test]
fn the_demesne_is_paid_while_the_seat_is_held() {
    let data = game_data();
    let amount = data.economy_rules.domain_income;
    assert!(amount > 0, "domain_income is set in economy.json");
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    let luna = fac("fac_luna");
    assert_eq!(state.faction_domain_income(data, &luna), amount);
    let before = state.faction_income(data, &luna);
    // The seat falls: no demesne any more.
    let seat = state.faction_capital_city(&luna).cloned().expect("seat");
    state.settlements.get_mut(&seat).unwrap().controller = fac("fac_aragon");
    assert_eq!(state.faction_domain_income(data, &luna), 0);
    assert!(state.faction_income(data, &luna) < before);
    // Rule absent: nothing.
    let mut without = data.clone();
    without.economy_rules.domain_income = 0;
    let state = CampaignState::new_1337(&without, fac("fac_france"), 1).unwrap();
    assert_eq!(state.faction_domain_income(&without, &luna), 0);
}

/// Receipts less buildings and court (without the idle hoard) cover at least
/// one urban militia paid at the city rate, the unit beside the household
/// guard (ADR 0117).
#[test]
fn a_poor_county_can_pay_one_garrison_unit() {
    let data = game_data();
    let state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    let militia = data
        .unit_types
        .values()
        .find(|t| t.id.as_str() == "unit_urban_militia")
        .expect("urban militia");
    let unit = i64::from(militia.upkeep)
        * data.economy_rules.upkeep_months_per_season
        * sim_campaign::economy::garrison_upkeep_percent(data, SettlementKind::City)
        / 100;
    for id in POOR_COUNTIES.map(fac) {
        let economy = state.faction_economy(data, &id).expect("economy");
        let income = state.faction_income(data, &id);
        let provinces = state.controlled_provinces(&id).count();
        let court =
            (income as f64 * data.economy_rules.administration_rate(provinces)).round() as i64;
        let receipts = economy.projected_income + economy.trade_income;
        let spare = receipts - economy.building_upkeep - court;
        assert!(
            spare >= unit,
            "{id}: receipts {receipts}, buildings {}, court {court}: {spare} < {unit}",
            economy.building_upkeep
        );
    }
}
