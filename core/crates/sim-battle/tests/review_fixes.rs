//! Regressions of the 2026-09-26 code review (sim-battle).

mod common;

use common::{data, lab, place, run, setup, unit};
use sim_battle::{BattleSim, Command, UnitState};

fn lab_sim(
    attacker: Vec<sim_battle::UnitSetup>,
    defender: Vec<sim_battle::UnitSetup>,
) -> BattleSim {
    let mut sim = BattleSim::new(setup(attacker, defender, None), 7).unwrap();
    lab(&mut sim);
    sim
}

/// A regiment breaking off a melee with a move marches again once out of
/// contact (it used to stay `Melee` until it reached its destination).
#[test]
fn disengaged_regiment_marches_again() {
    let data = data();
    let mut sim = lab_sim(
        vec![unit(&data, "unit_men_at_arms_foot")],
        vec![unit(&data, "unit_urban_militia")],
    );
    place(&mut sim, 0, 600.0, 350.0, 0.0);
    place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 1,
        run: false,
    })
    .unwrap();
    for _ in 0..600 {
        sim.step();
        if sim.units()[0].state == UnitState::Melee {
            break;
        }
    }
    assert_eq!(sim.units()[0].state, UnitState::Melee, "contact reached");
    sim.issue_command(Command::Move {
        units: vec![0],
        x: 600.0,
        z: 100.0,
        run: true,
        facing: None,
    })
    .unwrap();
    // The enemy falls back out of reach: the regiment is free.
    place(&mut sim, 1, 600.0, 600.0, std::f64::consts::PI);
    run(&mut sim, 3.0);
    let u = &sim.units()[0];
    assert!(u.destination.is_some(), "still on its way");
    assert_eq!(u.state, UnitState::Marching);
}
