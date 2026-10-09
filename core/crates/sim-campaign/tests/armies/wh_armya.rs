//! Lot WH armya (ADR 0272, 0273): rising upkeep and leaderless armies,
//! reinforcement radius, pace by composition, "lifts the siege" forecast.
use data_model::test_support::{fac, game_data};
use data_model::GameData;
use sim_campaign::march::px_per_km;
use sim_campaign::test_support::{capital_city, main_army};
use sim_campaign::economy::TurnBudget;
use sim_campaign::{
    movement, Army, ArmyId, ArmyPosition, CampaignState, Order, OrderError, Season, Stance, Unit,

};

fn unit(data: &GameData, id: &str) -> Unit {
    Unit::fresh(&data.unit_types[&data_model::UnitTypeId::new(id).unwrap()])
}

fn start() -> (&'static GameData, CampaignState) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.chronicle.disabled = true;
    state.season = Season::Summer;
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    (data, state)
}

/// A field army of `faction` at `point` with the given units.
fn field_army(
    state: &mut CampaignState,
    index: u32,
    faction: &str,
    point: [f32; 2],
    units: Vec<Unit>,
) -> ArmyId {
    let id = ArmyId::from_index(index);
    let mut army = Army::new(fac(faction), ArmyPosition::Field { x: point[0], y: point[1] }, units);
    army.movement_left = 10_000;
    state.armies.insert(id.clone(), army);
    id
}

#[test]
fn cavalry_only_host_is_faster_and_mixed_is_not() {
    let (data, mut state) = start();
    let army = main_army(&state, "fac_france");
    let paris = capital_city(&state, "fac_france");
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::Settlement(paris);
    let set = |state: &mut CampaignState, ids: &[&str]| {
        let units = ids.iter().map(|id| unit(data, id)).collect();
        state.armies.get_mut(&army).unwrap().units = units;
        state.army_movement_allowance(data, &state.armies[&army])
    };
    let cavalry = set(&mut state, &["unit_knights", "unit_knights"]);
    let mixed = set(&mut state, &["unit_knights", "unit_men_at_arms_foot"]);
    let foot = set(&mut state, &["unit_men_at_arms_foot", "unit_men_at_arms_foot"]);
    assert_eq!(mixed, foot, "the slowest family sets the pace");
    let bonus = data.army_rules.pace_percent_by_category.cavalry;
    assert!(bonus > 0.0);
    let expected = (f64::from(foot) * (1.0 + bonus / 100.0)).round();
    assert!((f64::from(cavalry) - expected).abs() <= 1.0, "{cavalry} vs {expected}");
    // A siege train still slows the army down.
    let train = set(&mut state, &["unit_knights", "unit_trebuchet"]);
    assert!(train < mixed, "{train} < {mixed}");
}

#[test]
fn leaderless_army_is_slower_and_cannot_besiege_or_ambush() {
    let (data, mut state) = start();
    let army = main_army(&state, "fac_france");
    let paris = capital_city(&state, "fac_france");
    {
        let a = state.armies.get_mut(&army).unwrap();
        a.position = ArmyPosition::Settlement(paris);
        a.stance = Stance::Normal;
    }
    let general = state.armies[&army].general.clone();
    assert!(general.is_some(), "the starting army has a general");
    let led = state.army_movement_allowance(data, &state.armies[&army]);
    state.armies.get_mut(&army).unwrap().general = None;
    let leaderless = state.army_movement_allowance(data, &state.armies[&army]);
    let factor = 1.0 + data.army_rules.leaderless.movement_percent / 100.0;
    assert!(leaderless < led);
    assert!(f64::from(leaderless) <= f64::from(led) * factor + 2.0);
    for stance in [Stance::Siege, Stance::Ambush] {
        let result = state.submit_order(data, Order::SetStance { army: army.clone(), stance });
        assert!(matches!(result, Err(OrderError::StanceRefused(_))), "{stance:?}: {result:?}");
    }
}

#[test]
fn armies_beyond_the_free_ones_cost_more() {
    let (data, mut state) = start();
    let france = fac("fac_france");
    state.armies.retain(|_, a| a.faction != france);
    let n = data.army_rules.upkeep.free_armies as usize + 4;
    for i in 0..n {
        field_army(&mut state, 800 + i as u32, "fac_france", [100.0, 100.0], vec![unit(data, "unit_knights")]);
    }
    let split = state.faction_upkeep(data, &france);
    let mut flat = data.clone();
    flat.army_rules.upkeep.extra_army_upkeep_percent = 0.0;
    let base = state.faction_upkeep(&flat, &france);
    assert!(split > base, "{split} > {base}");
    let surcharge = state.faction_army_surcharge(data, &france);
    assert_eq!(surcharge, split - base);
    let budget = TurnBudget::compute(&state, data, &france);
    assert_eq!(budget.army_surcharge, surcharge);
    assert_eq!(budget.army_count as usize, n);
    // The same regiments in one army pay no surcharge.
    state.armies.retain(|_, a| a.faction != france);
    field_army(&mut state, 850, "fac_france", [100.0, 100.0], (0..n).map(|_| unit(data, "unit_knights")).collect());
    assert_eq!(state.faction_upkeep(data, &france), base);
    assert_eq!(state.faction_army_surcharge(data, &france), 0);
}

#[test]
fn the_free_armies_pay_nothing_extra() {
    let (data, mut state) = start();
    let france = fac("fac_france");
    state.armies.retain(|_, a| a.faction != france);
    for i in 0..data.army_rules.upkeep.free_armies {
        field_army(&mut state, 800 + i, "fac_france", [100.0, 100.0], vec![unit(data, "unit_knights")]);
    }
    assert_eq!(state.faction_army_surcharge(data, &france), 0);
}

fn reinforcement_setup(distance_km: f32, movement_left: u32) -> (&'static GameData, CampaignState, ArmyId, ArmyId) {
    let (data, mut state) = start();
    state.armies.retain(|_, a| a.faction != fac("fac_france") && a.faction != fac("fac_england"));
    let ppk = px_per_km(data);
    let lead = field_army(&mut state, 900, "fac_france", [1000.0, 1000.0], vec![unit(data, "unit_knights")]);
    field_army(&mut state, 901, "fac_england", [1000.0 + 2.0 * ppk, 1000.0], vec![unit(data, "unit_knights")]);
    let ally = field_army(&mut state, 902, "fac_france", [1000.0 - distance_km * ppk, 1000.0], vec![unit(data, "unit_knights")]);
    state.armies.get_mut(&ally).unwrap().movement_left = movement_left;
    (data, state, lead, ally)
}

#[test]
fn allies_within_the_reinforce_radius_join_with_movement_left() {
    let radius = game_data().free_movement_rules().reinforce_radius_km as f32;
    assert!(radius > 10.0);
    let enemy = fac("fac_england");
    // Inside the radius with movement: joins.
    let (data, state, lead, ally) = reinforcement_setup(radius - 5.0, 10_000);
    let coalition = movement::battle_coalition(&state, data, &lead, &enemy);
    assert_eq!(coalition, vec![lead.clone(), ally.clone()]);
    // Beyond the radius: alone.
    let (data, state, lead, _) = reinforcement_setup(radius + 15.0, 10_000);
    assert_eq!(movement::battle_coalition(&state, data, &lead, &enemy).len(), 1);
    // Inside the radius but no movement left: alone.
    let (data, state, lead, _) = reinforcement_setup(radius - 5.0, 0);
    assert_eq!(movement::battle_coalition(&state, data, &lead, &enemy).len(), 1);
    // A besieging army does not leave its siege.
    let (data, mut state, lead, ally) = reinforcement_setup(radius - 5.0, 10_000);
    state.armies.get_mut(&ally).unwrap().stance = Stance::Siege;
    assert_eq!(movement::battle_coalition(&state, data, &lead, &enemy).len(), 1);
}

#[test]
fn forecast_lists_far_reinforcements_as_late() {
    let radius = game_data().free_movement_rules().reinforce_radius_km as f32;
    let (data, mut state, lead, ally) = reinforcement_setup(radius - 5.0, 10_000);
    let enemy = ArmyId::from_index(901);
    state.pending_battles.push(sim_campaign::BattleRequest {
        attacker: lead.clone(),
        defender: enemy,
        location: capital_city(&state, "fac_france"),
        province: state.settlement_province(&capital_city(&state, "fac_france")).unwrap().clone(),
        siege: false,
        opening: Default::default(),
    });
    let forecast = state.battle_forecast(data, 0).unwrap();
    let joined = &forecast.attacker_reinforcements;
    assert_eq!(joined.len(), 1, "{forecast:?}");
    assert!(joined[0].late);
    assert!(joined[0].distance_km > data.free_movement_rules().engage_radius_km);
    assert_eq!(joined[0].army, ally.to_string());
    assert!(forecast.modifiers.iter().any(|m| m.contains("rejoignent de loin")));
}
