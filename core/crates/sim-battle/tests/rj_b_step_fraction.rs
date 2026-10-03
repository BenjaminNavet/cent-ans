//! RJ-b: the part of the next fixed step already elapsed, which the renderer
//! uses to blend the poses of the two latest steps.

use sim_battle::{BattleSetup, BattleSim, DT};

fn demo() -> BattleSim {
    let setup: BattleSetup =
        serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap();
    BattleSim::new(setup, 1337).unwrap()
}

#[test]
fn step_fraction_follows_the_accumulator() {
    let mut sim = demo();
    assert!(sim.can_step(), "the demo battle starts without deployment");
    assert_eq!(sim.step_fraction(), 0.0);
    sim.tick(DT * 0.25);
    assert_eq!(sim.ticks(), 0);
    assert!((sim.step_fraction() - 0.25).abs() < 1e-9);
    sim.tick(DT * 0.5);
    assert!((sim.step_fraction() - 0.75).abs() < 1e-9);
    sim.tick(DT * 0.5);
    assert_eq!(sim.ticks(), 1);
    assert!((sim.step_fraction() - 0.25).abs() < 1e-9);
    // Several steps in one call: only the remainder counts.
    sim.tick(DT * 3.5);
    assert_eq!(sim.ticks(), 4);
    assert!((sim.step_fraction() - 0.75).abs() < 1e-9);
}

#[test]
fn step_fraction_is_one_when_no_step_can_run() {
    let mut sim = demo();
    sim.tick(DT * 0.5);
    sim.begin_deployment();
    assert!(!sim.can_step());
    assert_eq!(sim.step_fraction(), 1.0);
}
