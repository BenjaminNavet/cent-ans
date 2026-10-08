//! Net balance (UI audit A3, E1): the projected « solde » shown before the
//! turn is the one booked by the economy phase.

use data_model::FactionId;
use sim_campaign::test_support::idle;
use sim_campaign::CampaignState;

use data_model::test_support::game_data;

#[test]
fn projected_net_income_is_the_booked_balance() {
    let data = game_data();
    let france = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(data, france.clone(), 7).expect("1337 start");
    state.chronicle.disabled = true;
    assert_eq!(state.faction_net_last_turn(&france), Some(0));
    let economy = state.faction_economy(data, &france).unwrap();
    let net = economy.net_income();
    assert_eq!(
        net,
        economy.projected_income + economy.trade_income
            - economy.army_upkeep
            - economy.building_upkeep
            - economy.administration_upkeep
            - economy.table_upkeep
    );
    state.end_turn_with(data, idle);
    assert_eq!(state.faction_net_last_turn(&france), Some(net));
    assert_eq!(
        state.faction_net_last_turn(&FactionId::new("fac_nobody").unwrap()),
        None
    );
}
