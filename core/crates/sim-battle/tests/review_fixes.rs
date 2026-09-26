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

/// Archers ordered to shoot a regiment hidden in a wood (seen from 60 m
/// only) close in instead of standing in range without a shot.
#[test]
fn archers_close_in_on_a_hidden_target() {
    let data = data();
    let mut sim = lab_sim(
        vec![unit(&data, "unit_longbowmen")],
        vec![unit(&data, "unit_urban_militia")],
    );
    sim.field_mut().forests.clear();
    sim.field_mut().forest_parts.clear();
    sim.field_mut().forests.push(sim_battle::Zone {
        x: 600.0,
        z: 520.0,
        radius: 40.0,
    });
    place(&mut sim, 0, 600.0, 380.0, 0.0);
    place(&mut sim, 1, 600.0, 520.0, std::f64::consts::PI);
    let before = sim.units()[1].hp;
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 1,
        run: false,
    })
    .unwrap();
    run(&mut sim, 60.0);
    assert!(sim.units()[0].z > 440.0, "z {}", sim.units()[0].z);
    assert!(sim.units()[1].hp < before, "the hidden regiment is shot at");
}

/// Archers whose ordered target is locked in a melee shoot, at will, the
/// nearest enemy they can.
#[test]
fn archers_fall_back_on_a_free_enemy() {
    let data = data();
    let mut sim = lab_sim(
        vec![
            unit(&data, "unit_longbowmen"),
            unit(&data, "unit_men_at_arms_foot"),
        ],
        vec![
            unit(&data, "unit_urban_militia"),
            unit(&data, "unit_urban_militia"),
        ],
    );
    sim.field_mut().forests.clear();
    sim.field_mut().forest_parts.clear();
    place(&mut sim, 0, 600.0, 380.0, 0.0);
    place(&mut sim, 1, 520.0, 440.0, 0.0);
    place(&mut sim, 2, 520.0, 480.0, std::f64::consts::PI);
    place(&mut sim, 3, 680.0, 480.0, std::f64::consts::PI);
    sim.issue_command(Command::Attack {
        units: vec![1],
        target: 2,
        run: false,
    })
    .unwrap();
    for _ in 0..600 {
        sim.step();
        if sim.units()[2].state == UnitState::Melee {
            break;
        }
    }
    assert_eq!(sim.units()[2].state, UnitState::Melee, "contact reached");
    let before = sim.units()[3].hp;
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 2,
        run: false,
    })
    .unwrap();
    run(&mut sim, 20.0);
    assert!(sim.units()[3].hp < before, "the free regiment is shot at");
}
