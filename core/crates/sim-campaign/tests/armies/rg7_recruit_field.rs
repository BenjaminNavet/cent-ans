//! Lot RG7: recruit from an army standing in own territory (not in a place),
//! with the delivery and refusal rules.
use data_model::test_support::{fac, game_data};
use data_model::GameData;
use sim_campaign::test_support::{capital_city, main_army};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, OrderError, Season, Stance};

fn start() -> (&'static GameData, CampaignState) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.chronicle.disabled = true;
    state.season = Season::Summer;
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    (data, state)
}

fn field_point(data: &GameData, place: &data_model::SettlementId, dx: f32) -> ArmyPosition {
    let [x, y] = data.settlement_point(place).unwrap();
    ArmyPosition::Field { x: x + dx, y }
}

/// The French main army, trimmed, camped in the field next to Paris.
fn field_army(
    data: &GameData,
    state: &mut CampaignState,
) -> (ArmyId, data_model::SettlementId, data_model::UnitTypeId) {
    let army = main_army(state, "fac_france");
    let paris = capital_city(state, "fac_france");
    let entry = state.armies.get_mut(&army).unwrap();
    entry.units.truncate(3);
    entry.stance = Stance::Normal;
    entry.position = field_point(data, &paris, 2.0);
    let unit = state
        .recruitable(data, &paris)
        .into_iter()
        .find(|o| o.available)
        .expect("a recruitable unit")
        .unit_type;
    (army, paris, unit)
}

fn order(paris: &data_model::SettlementId, unit: &data_model::UnitTypeId, army: &ArmyId) -> Order {
    Order::RecruitInto {
        settlement: paris.clone().into(),
        unit_type: unit.clone(),
        army: army.clone(),
    }
}

#[test]
fn an_army_in_the_field_of_its_province_recruits_and_receives_the_unit() {
    let (data, mut state) = start();
    let (army, paris, unit) = field_army(data, &mut state);
    assert_eq!(state.army_recruit_blocker(data, &army), None);
    assert!(state.recruit_site_for_army(data, &army, &unit).is_some());
    let garrison = state.settlements[&paris].garrison.len();
    state
        .submit_order(data, order(&paris, &unit, &army))
        .unwrap();
    state.end_turn(data);
    assert_eq!(state.armies[&army].units.len(), 4);
    assert!(state.armies[&army].settlement().is_none());
    assert_eq!(state.settlements[&paris].garrison.len(), garrison);
}

#[test]
fn the_unit_goes_to_the_garrison_when_the_army_left_the_province() {
    let (data, mut state) = start();
    let (army, paris, unit) = field_army(data, &mut state);
    let garrison = state.settlements[&paris].garrison.len();
    state
        .submit_order(data, order(&paris, &unit, &army))
        .unwrap();
    let england = capital_city(&state, "fac_england");
    state.armies.get_mut(&army).unwrap().position = field_point(data, &england, 2.0);
    state.end_turn(data);
    assert_eq!(state.armies[&army].units.len(), 3);
    let total: usize = state
        .settlements
        .values()
        .filter(|s| s.province == state.settlements[&paris].province)
        .map(|s| s.garrison.len())
        .sum();
    assert!(total > garrison);
}

#[test]
fn recruiting_is_refused_outside_own_territory() {
    let (data, mut state) = start();
    let (army, paris, unit) = field_army(data, &mut state);
    let england = capital_city(&state, "fac_england");
    state.armies.get_mut(&army).unwrap().position = field_point(data, &england, 2.0);
    assert_eq!(
        state.army_recruit_blocker(data, &army).as_deref(),
        Some("Hors de votre territoire")
    );
    assert!(matches!(
        state.submit_order(data, order(&paris, &unit, &army)),
        Err(OrderError::RecruitUnavailable(_))
    ));
}

#[test]
fn the_field_surcharge_is_a_data_parameter_defaulting_to_zero() {
    let data = game_data();
    assert_eq!(data.economy_rules.field_recruit_surcharge_percent, 0);
}
