//! Lot WR armies (ADR 0305): far reinforcements (committed share, movement
//! spent), tests reading crate-private items.
use data_model::test_support::{fac, game_data};
use data_model::GameData;

use crate::state::{Army, ArmyId, ArmyPosition, CampaignState, Unit};

fn unit(data: &GameData, id: &str) -> Unit {
    Unit::fresh(&data.unit_types[&data_model::UnitTypeId::new(id).unwrap()])
}

fn start() -> (&'static GameData, CampaignState) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.chronicle.disabled = true;
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

fn field_army(
    state: &mut CampaignState,
    index: u32,
    faction: &str,
    point: [f32; 2],
    units: Vec<Unit>,
) -> ArmyId {
    let id = ArmyId::from_index(index);
    let mut army = Army::new(
        fac(faction),
        ArmyPosition::Field {
            x: point[0],
            y: point[1],
        },
        units,
    );
    army.movement_left = 10_000;
    state.armies.insert(id.clone(), army);
    id
}

use crate::march::px_per_km;
use sim_battle::BattleOpening;

/// Lead (France) at (1000, 1000), an English army 2 km east, and a French
/// ally `distance_km` west with `movement` points left.
fn setup(
    distance_km: f32,
    movement: u32,
) -> (&'static GameData, CampaignState, ArmyId, ArmyId, ArmyId) {
    let (data, mut state) = start();
    state
        .armies
        .retain(|_, a| a.faction != fac("fac_france") && a.faction != fac("fac_england"));
    let ppk = px_per_km(data);
    let lead = field_army(
        &mut state,
        900,
        "fac_france",
        [1000.0, 1000.0],
        vec![unit(data, "unit_knights")],
    );
    let enemy = field_army(
        &mut state,
        901,
        "fac_england",
        [1000.0 + 2.0 * ppk, 1000.0],
        vec![unit(data, "unit_knights")],
    );
    let ally = field_army(
        &mut state,
        902,
        "fac_france",
        [1000.0 - distance_km * ppk, 1000.0],
        vec![unit(data, "unit_knights")],
    );
    state.armies.get_mut(&ally).unwrap().movement_left = movement;
    (data, state, lead, enemy, ally)
}

fn ally_men(distance_km: f32) -> (u32, u32) {
    let (data, state, lead, enemy, ally) = setup(distance_km, 10_000);
    let full = state.armies[&ally].total_strength();
    let battle =
        crate::movement::field_battle_setup(&state, data, &lead, &enemy, BattleOpening::Standard)
            .unwrap();
    assert_eq!(battle.attackers.len(), 2);
    let engaged: u32 = battle
        .attacker_side
        .units
        .iter()
        .map(|u| u.strength)
        .sum::<u32>()
        - state.armies[&lead].total_strength();
    (engaged, full)
}

#[test]
fn auto_resolution_commits_a_decreasing_share_of_a_far_reinforcement() {
    let rules = game_data().free_movement_rules().clone();
    let (near, full) = ally_men((rules.reinforce_full_radius_km - 1.0) as f32);
    assert_eq!(near, full);
    let (mid, _) =
        ally_men(((rules.reinforce_full_radius_km + rules.reinforce_radius_km) / 2.0) as f32);
    let (far, _) = ally_men((rules.reinforce_radius_km - 0.5) as f32);
    assert!(mid < full && far < mid, "{near} {mid} {far}");
    // The floor is respected.
    assert!(far as f64 >= f64::from(full) * rules.reinforce_min_percent / 100.0 - 1.0);
}

#[test]
fn far_reinforcement_spends_its_movement_and_a_near_one_does_not() {
    let (data, mut state, lead, enemy, ally) = setup(20.0, 10_000);
    let near = field_army(
        &mut state,
        903,
        "fac_france",
        [1000.0 - 3.0 * px_per_km(data), 1000.0],
        vec![unit(data, "unit_knights")],
    );
    state.armies.get_mut(&near).unwrap().movement_left = 10_000;
    let mut events = Vec::new();
    crate::movement::auto_fight_with_opening(
        &mut state,
        data,
        &lead,
        &enemy,
        BattleOpening::Standard,
        &mut events,
    );
    let rules = data.free_movement_rules();
    let expected = crate::march::km_to_grid_points(data, 20.0 - rules.engage_radius_km);
    assert_eq!(state.armies[&ally].movement_left, 10_000 - expected);
    assert_eq!(state.armies[&near].movement_left, 10_000);
}
