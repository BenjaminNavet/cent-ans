//! Lot NT6c: a weakened AI army rests to rebuild its ranks (in a friendly
//! place or entrenched on friendly lands) when no enemy is near, and leaves
//! once rebuilt (`stances::rest_plan`, `postures.rest` of `data/ai/grid.json`).

use std::path::PathBuf;

use ai::stances::{self, RestPlan};
use data_model::{FactionId, GameData};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, Stance};

fn real_data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    ai::feudal::install();
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn offset(data: &GameData, point: [f32; 2], east_km: f32) -> [f32; 2] {
    [
        point[0] + east_km * sim_campaign::march::px_per_km(data),
        point[1],
    ]
}

fn main_army(state: &CampaignState, faction: &FactionId) -> ArmyId {
    state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

/// Scales the men of `army` to `percent` of their full strength.
fn set_share(state: &mut CampaignState, army: &ArmyId, percent: u32) {
    for unit in &mut state.armies.get_mut(army).unwrap().units {
        unit.strength = unit.max_strength * percent / 100;
    }
}

fn share(state: &CampaignState, army: &ArmyId) -> u32 {
    let units = &state.armies[army].units;
    100 * units.iter().map(|u| u.strength).sum::<u32>()
        / units.iter().map(|u| u.max_strength).sum::<u32>()
}

/// France at peace, its main army in a French place at `percent`.
fn scene(data: &GameData, percent: u32) -> (CampaignState, ArmyId, FactionId) {
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 3).unwrap();
    let army = main_army(&state, &france);
    let place = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == france)
        .map(|(id, _)| id.clone())
        .unwrap();
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::Settlement(place);
    set_share(&mut state, &army, percent);
    (state, army, france)
}

fn set_stance_orders(orders: &[Order], army: &ArmyId, stance: Stance) -> bool {
    orders
        .iter()
        .any(|o| matches!(o, Order::SetStance { army: a, stance: s } if a == army && *s == stance))
}

fn moves(orders: &[Order], army: &ArmyId) -> bool {
    orders
        .iter()
        .any(|o| matches!(o, Order::MoveArmy { army: a, .. } if a == army))
}

#[test]
fn a_weakened_army_in_a_friendly_place_rests() {
    let data = real_data();
    let (mut state, army, france) = scene(&data, 40);
    state.season = sim_campaign::Season::Summer;
    assert_eq!(
        stances::rest_plan(&state, &data, &france, &army),
        RestPlan::Rest { entrench: false }
    );
    assert!(!moves(&ai::plan_turn(&state, &data, &france), &army));
    // Strong enough: not resting.
    set_share(&mut state, &army, 90);
    assert_eq!(
        stances::rest_plan(&state, &data, &france, &army),
        RestPlan::None
    );
    // Feature off: never.
    let (state, army, france) = scene(&data, 40);
    let mut off = data.clone();
    off.ai_grid.postures.rest.enabled = false;
    assert_eq!(
        stances::rest_plan(&state, &off, &france, &army),
        RestPlan::None
    );
}

#[test]
fn a_weakened_army_in_the_open_on_friendly_lands_entrenches_to_rest() {
    let data = real_data();
    let (mut state, army, france) = scene(&data, 40);
    let place = state.armies[&army].settlement().cloned().unwrap();
    let at = data.settlement_point(&place).unwrap();
    let spot = offset(&data, at, 3.0);
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::field(spot);
    let allowance = state.army_grid_allowance(&data, &state.armies[&army]);
    state.armies.get_mut(&army).unwrap().movement_left = allowance;
    let plan = stances::rest_plan(&state, &data, &france, &army);
    if plan == (RestPlan::Rest { entrench: true }) {
        let orders = ai::plan_turn(&state, &data, &france);
        assert!(set_stance_orders(&orders, &army, Stance::Entrenched));
        assert!(!moves(&orders, &army));
    } else {
        // The cell may refuse the camp (terrain): then the usual plan applies.
        assert_eq!(plan, RestPlan::None);
    }
}

#[test]
fn a_menace_within_reach_stops_the_rest() {
    let data = real_data();
    let (mut state, army, france) = scene(&data, 40);
    let england = fac("fac_england");
    for (a, b) in [(&france, &england), (&england, &france)] {
        state
            .factions
            .get_mut(a)
            .unwrap()
            .at_war_with
            .insert(b.clone());
    }
    let place = state.armies[&army].settlement().cloned().unwrap();
    let at = data.settlement_point(&place).unwrap();
    let enemy = main_army(&state, &england);
    state.armies.get_mut(&enemy).unwrap().position = ArmyPosition::field(offset(&data, at, 20.0));
    assert_eq!(
        stances::rest_plan(&state, &data, &france, &army),
        RestPlan::None
    );
    // Far away: rests again.
    state.armies.get_mut(&enemy).unwrap().position = ArmyPosition::field(offset(&data, at, 400.0));
    assert_eq!(
        stances::rest_plan(&state, &data, &france, &army),
        RestPlan::Rest { entrench: false }
    );
}

#[test]
fn a_rebuilt_camp_is_left() {
    let data = real_data();
    let (mut state, army, france) = scene(&data, 40);
    let place = state.armies[&army].settlement().cloned().unwrap();
    let at = data.settlement_point(&place).unwrap();
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::field(offset(&data, at, 3.0));
    state.armies.get_mut(&army).unwrap().stance = Stance::Entrenched;
    // Still weak: keeps the camp.
    assert_eq!(
        stances::rest_plan(&state, &data, &france, &army),
        RestPlan::Rest { entrench: true }
    );
    // Rebuilt to the exit threshold: leaves it.
    set_share(&mut state, &army, 90);
    assert!(share(&state, &army) >= 85);
    assert_eq!(
        stances::rest_plan(&state, &data, &france, &army),
        RestPlan::Leave
    );
    let orders = ai::plan_turn(&state, &data, &france);
    assert!(
        set_stance_orders(&orders, &army, Stance::Normal) || moves(&orders, &army),
        "the rebuilt army leaves its camp: {orders:?}"
    );
}
