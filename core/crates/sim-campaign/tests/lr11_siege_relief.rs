//! LR-11: when is a siege lifted? Only when the besiegers lose a battle (a
//! relief army or a sortie) or march off. A relief army beaten before the
//! walls leaves the siege running; besiegers put to flight by a sortie fall
//! back like any beaten army instead of standing idle under the walls.
use data_model::test_support::fac;
use sim_campaign::test_support::{idle, main_army};

use std::path::PathBuf;

use data_model::{FactionId, GameData, MapRasters, NavGrid, SettlementId, PLAIN_COST};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, OrderOutcome, Stance};

/// The real data on an all-plain grid (known geometry, as in `m2_free_movement`).
fn plain_data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let mut data = GameData::load(&root).expect("game data loads").0;
    let mut grid = NavGrid::uniform(2048, 2048, 2, 1.438, PLAIN_COST);
    grid.refresh_min_cost();
    data.set_map_rasters(MapRasters {
        navgrid: grid,
        provinces: None,
    });
    data
}

/// A map point `km` kilometres east of `point`.
fn east(data: &GameData, point: [f32; 2], km: f32) -> [f32; 2] {
    [point[0] + km * data.navgrid().px_per_km() as f32, point[1]]
}

/// Keeps only `strength` men in each unit of `army` (at most `units` units).
fn weaken(state: &mut CampaignState, army: &ArmyId, units: usize, strength: u32) {
    let a = state.armies.get_mut(army).unwrap();
    a.units.truncate(units);
    for unit in &mut a.units {
        unit.strength = strength.min(unit.max_strength);
    }
}

/// France's main army besieges Bordeaux (England's other armies removed,
/// France's own kept away); the siege has begun and counts one turn.
fn french_siege_of_bordeaux(data: &GameData) -> (CampaignState, ArmyId, SettlementId) {
    let bordeaux = SettlementId::new("set_bordeaux").unwrap();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.interactive_battles = false;
    let french = main_army(&state, "fac_france");
    state.armies.retain(|id, _| *id == french);
    let point = data.settlement_point(&bordeaux).unwrap();
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(east(data, point, -12.0));
    match state
        .submit_order_outcome(data, Order::move_to(french.clone(), bordeaux.clone()))
        .expect("march accepted")
    {
        OrderOutcome::Moved(_) => {}
        other => panic!("expected a march, got {other:?}"),
    }
    assert!(state.settlements[&bordeaux].siege.is_some());
    (state, french, bordeaux)
}

/// An English army of `units` units of `strength` men, `km` east of Bordeaux.
fn english_relief(
    state: &mut CampaignState,
    data: &GameData,
    bordeaux: &SettlementId,
    units: usize,
    strength: u32,
) -> ArmyId {
    let fresh = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    let english = main_army(&fresh, "fac_england");
    let mut army = fresh.armies[&english].clone();
    let point = data.settlement_point(bordeaux).unwrap();
    army.position = ArmyPosition::field(east(data, point, 6.0));
    army.stance = Stance::Normal;
    army.clear_plan();
    army.movement_left = state.army_grid_allowance(data, &army);
    state.armies.insert(english.clone(), army);
    weaken(state, &english, units, strength);
    english
}

/// England attacks the French besiegers during its own AI turn.
fn english_attack(state: &mut CampaignState, data: &GameData, english: &ArmyId, french: &ArmyId) {
    let order = Order::Attack {
        army: english.clone(),
        target_army: french.clone(),
    };
    let mut events = Vec::new();
    state.play_ai_turn(
        data,
        &fac("fac_england"),
        &|_: &CampaignState, _: &GameData, _: &FactionId| vec![order.clone()],
        &mut events,
    );
    assert!(
        state
            .armies
            .get(french)
            .is_some_and(|a| a.fought_turn == Some(state.turn())),
        "the relief army attacked the besiegers"
    );
}

#[test]
fn a_beaten_relief_army_leaves_the_siege_running() {
    let data = plain_data();
    let (mut state, french, bordeaux) = french_siege_of_bordeaux(&data);
    state.end_turn_with(&data, idle);
    let english = english_relief(&mut state, &data, &bordeaux, 2, 40);
    english_attack(&mut state, &data, &english, &french);
    let army = &state.armies[&french];
    assert!(army.is_at(&bordeaux), "the besiegers hold their lines");
    assert_eq!(army.stance, Stance::Siege);
    assert!(state.settlements[&bordeaux].siege.is_some());
    state.end_turn_with(&data, idle);
    let siege = state.settlements[&bordeaux]
        .siege
        .as_ref()
        .expect("the siege goes on after the relief army's defeat");
    assert_eq!(siege.attacker, fac("fac_france"));
}

#[test]
fn a_victorious_relief_army_lifts_the_siege() {
    let data = plain_data();
    let (mut state, french, bordeaux) = french_siege_of_bordeaux(&data);
    state.end_turn_with(&data, idle);
    weaken(&mut state, &french, 2, 30);
    let english = english_relief(&mut state, &data, &bordeaux, 12, 10_000);
    english_attack(&mut state, &data, &english, &french);
    assert!(state
        .armies
        .get(&french)
        .is_none_or(|a| !a.is_at(&bordeaux)));
    state.end_turn_with(&data, idle);
    assert!(state.settlements[&bordeaux].siege.is_none());
}

#[test]
fn besiegers_put_to_flight_by_a_sortie_fall_back() {
    let data = plain_data();
    let (mut state, french, bordeaux) = french_siege_of_bordeaux(&data);
    weaken(&mut state, &french, 1, 20);
    // A strong garrison sallies out at the end of the turn.
    let garrison = &mut state.settlements.get_mut(&bordeaux).unwrap().garrison;
    let model = garrison[0].clone();
    for _ in 0..8 {
        garrison.push(model.clone());
    }
    for unit in garrison.iter_mut() {
        unit.strength = unit.max_strength;
    }
    state.end_turn_with(&data, idle);
    assert!(state.settlements[&bordeaux].siege.is_none());
    if let Some(army) = state.armies.get(&french) {
        assert!(
            !army.is_at(&bordeaux),
            "beaten besiegers do not stand idle under the walls"
        );
        assert_eq!(army.stance, Stance::Normal);
    }
}
