//! TW reinf (ADR 0330): an allied army near a besieged place joins the 3D
//! assault on the defender's side, late and behind the besiegers.
use data_model::test_support::{fac, game_data};
use sim_battle::{BattleSim, EntryEdge, SideId};
use sim_campaign::march::px_per_km;
use sim_campaign::{Army, ArmyId, ArmyPosition, Order, Unit};

use super::m8_battle::{besiege_guyenne, guyenne};

#[test]
fn a_relief_army_joins_the_defence_behind_the_besiegers() {
    let data = game_data();
    let (mut state, army) = besiege_guyenne(data, 4);
    let city = guyenne(&state);
    let garrison = state.settlement_state(&city).unwrap().garrison.len();
    let place = data.settlement_point(&city).unwrap();
    let ppk = px_per_km(data);
    let relief_id = ArmyId::from_index(950);
    let mut relief = Army::new(
        fac("fac_england"),
        ArmyPosition::Field {
            x: place[0] + 15.0 * ppk,
            y: place[1],
        },
        vec![
            Unit::fresh(&data.unit_types[&data_model::UnitTypeId::new("unit_knights").unwrap()]),
            Unit::fresh(
                &data.unit_types[&data_model::UnitTypeId::new("unit_men_at_arms_foot").unwrap()],
            ),
        ],
    );
    relief.movement_left = 10_000;
    state.armies.insert(relief_id.clone(), relief);
    state
        .submit_order(data, Order::Assault { army })
        .unwrap();
    let setup = state.battle_setup(data, 0).unwrap();
    let defenders = &setup.defender.units;
    assert_eq!(defenders.len(), garrison + 2);
    assert!(defenders[..garrison].iter().all(|u| u.arrival_s.is_none()));
    for late in &defenders[garrison..] {
        assert!(late.arrival_s.unwrap() > 60.0);
        assert_eq!(late.entry_edge, EntryEdge::Rear);
    }
    // The 3D sim keeps them off the field until their hour.
    let mut sim_setup = setup.clone();
    sim_setup.player_side = None;
    let mut sim = BattleSim::new(sim_setup, 2).unwrap();
    assert_eq!(sim.reserves(SideId::Defender), 2);
    assert!(sim.next_arrival_in(SideId::Defender).is_some());
    while !sim.is_finished() {
        sim.step();
    }
    let outcome = sim.outcome().unwrap();
    // Losses cover the garrison and the relief regiments: the result applies.
    assert_eq!(outcome.defender.losses.len(), garrison + 2);
    let strength_before: u32 = state.armies[&relief_id].units.iter().map(|u| u.strength).sum();
    state.resolve_pending_battle(data, 0, &outcome).unwrap();
    let strength_after: u32 = state
        .armies
        .get(&relief_id)
        .map_or(0, |a| a.units.iter().map(|u| u.strength).sum());
    assert!(strength_after <= strength_before);
    // Out of reach, the relief army is not part of the assault.
    let (mut state, army) = besiege_guyenne(data, 4);
    let mut far = Army::new(
        fac("fac_england"),
        ArmyPosition::Field {
            x: place[0] + 60.0 * ppk,
            y: place[1],
        },
        vec![Unit::fresh(
            &data.unit_types[&data_model::UnitTypeId::new("unit_knights").unwrap()],
        )],
    );
    far.movement_left = 10_000;
    state.armies.insert(ArmyId::from_index(951), far);
    state.submit_order(data, Order::Assault { army }).unwrap();
    let setup = state.battle_setup(data, 0).unwrap();
    assert_eq!(setup.defender.units.len(), garrison);
}
