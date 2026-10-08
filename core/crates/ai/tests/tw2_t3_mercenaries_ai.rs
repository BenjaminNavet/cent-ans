//! Lot TW2-T3 (ADR 0103): the campaign AI hires mercenary companies when it
//! is rich and one of its armies is threatened.
use data_model::test_support::{fac, game_data};

use data_model::{GameData, SettlementId};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, Unit};

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

fn paris(state: &CampaignState) -> SettlementId {
    let capital = state.factions[&fac("fac_france")].capital.clone();
    state.province_city_id(&capital).cloned().unwrap()
}

/// England played by the player; the French AI army in Paris in 1362 with
/// `treasury` livres; an English army `enemy_factor` times as strong camps
/// `distance_km` east of Paris.
fn setup(
    treasury: i64,
    enemy_factor: usize,
    distance_km: f32,
) -> (&'static GameData, CampaignState, ArmyId) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 5).unwrap();
    state.year = 1362;
    let paris = paris(&state);
    let french = main_army(&state, "fac_france");
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::Settlement(paris.clone());
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = treasury;
    let english = main_army(&state, "fac_england");
    let point = data.settlement_point(&paris).unwrap();
    let px = sim_campaign::march::px_per_km(data);
    let french_units = state.armies[&french].units.clone();
    let army = state.armies.get_mut(&english).unwrap();
    army.position = ArmyPosition::field([point[0] + distance_km * px, point[1]]);
    army.units = (0..enemy_factor)
        .flat_map(|_| french_units.iter().cloned())
        .collect::<Vec<Unit>>();
    assert!(state.is_at_war(&fac("fac_france"), &fac("fac_england")));
    (data, state, french)
}

fn hires(orders: &[Order]) -> Vec<&Order> {
    orders
        .iter()
        .filter(|o| matches!(o, Order::HireMercenary { .. }))
        .collect()
}

#[test]
fn a_rich_threatened_realm_hires_companies_the_core_accepts() {
    let (data, mut state, french) = setup(400_000, 3, 5.0);
    let orders = ai::plan_turn_sequential(&state, data, &fac("fac_france"));
    let hired = hires(&orders);
    assert!(!hired.is_empty(), "no hire in {orders:?}");
    assert!(hired.len() as u32 <= data.mercenary_rules.hires_per_faction_per_turn);
    assert!(hired
        .iter()
        .all(|o| matches!(o, Order::HireMercenary { army, .. } if army == &french)));
    let before = state.armies[&french].units.len();
    for order in hired {
        state
            .apply_order(data, &fac("fac_france"), order.clone())
            .expect("the core accepts the AI's hires");
    }
    assert!(state.armies[&french].units.len() > before);
    // Same orders on the parallel planner.
    let (_, fresh, _) = setup(400_000, 3, 5.0);
    assert_eq!(
        ai::plan_turn(&fresh, data, &fac("fac_france")),
        ai::plan_turn_sequential(&fresh, data, &fac("fac_france"))
    );
}

#[test]
fn no_hire_when_poor_or_safe() {
    // Poor.
    let (data, state, _) = setup(1_000, 3, 5.0);
    assert!(hires(&ai::plan_turn_sequential(&state, data, &fac("fac_france"))).is_empty());
    // Rich, but the enemy is far away.
    let (data, state, _) = setup(400_000, 3, 400.0);
    assert!(hires(&ai::plan_turn_sequential(&state, data, &fac("fac_france"))).is_empty());
}
