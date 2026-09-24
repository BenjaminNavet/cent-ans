//! F5a tests: collisions, formations AI, deployment, siege pathing, reinforcements.

mod common;

use common::*;
use sim_battle::{BattleSim, Command, UnitState, FRIEND_GAP};

fn gap(sim: &BattleSim, a: usize, b: usize) -> f64 {
    let (a, b) = (&sim.units()[a], &sim.units()[b]);
    (a.x - b.x).abs() - (a.extent().0 + b.extent().0) * 0.5
}

#[test]
fn friendly_regiments_separate() {
    let data = data();
    let attacker = units(&data, &["unit_men_at_arms_foot", "unit_men_at_arms_foot"]);
    let defender = units(&data, &["unit_urban_militia"]);
    let mut sim = BattleSim::new(setup(attacker, defender, None), 1).unwrap();
    lab(&mut sim);
    // Two regiments standing half on top of each other drift apart softly.
    place(&mut sim, 0, 600.0, 200.0, 0.0);
    place(&mut sim, 1, 610.0, 200.0, 0.0);
    sim.step();
    let moved = (sim.units()[1].x - 610.0).abs();
    assert!(moved > 0.0 && moved <= 0.31, "soft push, got {moved}");
    run(&mut sim, 30.0);
    assert!(gap(&sim, 0, 1) >= FRIEND_GAP - 0.2, "separated");
    assert!((sim.units()[0].z - 200.0).abs() < 1.0, "sideways only");
}

#[test]
fn a_charge_passes_through_friends() {
    let data = data();
    let attacker = units(&data, &["unit_men_at_arms_foot", "unit_knights"]);
    let defender = units(&data, &["unit_urban_militia"]);
    let mut sim = BattleSim::new(setup(attacker, defender, None), 1).unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 600.0, 300.0, 0.0);
    place(&mut sim, 1, 600.0, 200.0, 0.0);
    place(&mut sim, 2, 600.0, 420.0, std::f64::consts::PI);
    let command = Command::Attack {
        units: vec![1],
        target: 2,
        run: true,
    };
    sim.issue_command(command).unwrap();
    run(&mut sim, 60.0);
    assert!(
        (sim.units()[0].x - 600.0).abs() < 0.5,
        "the foot are not shoved"
    );
    assert_eq!(
        sim.units()[1].state,
        UnitState::Melee,
        "the knights reach the enemy"
    );
}

#[test]
fn ai_changes_formation() {
    use sim_battle::formation_ai::plan_formations;
    use sim_battle::{Formation, SideId};
    let data = data();
    let attacker = units(&data, &["unit_knights"]);
    let defender = units(&data, &["unit_flemish_pikemen", "unit_urban_militia"]);
    let mut sim = BattleSim::new(setup(attacker, defender, None), 2).unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 600.0, 300.0, 0.0);
    place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
    place(&mut sim, 2, 900.0, 400.0, std::f64::consts::PI);
    // Knights charging the militia: wedge. Pikemen 100 m from horse: schiltron.
    let charge = Command::Attack {
        units: vec![0],
        target: 2,
        run: true,
    };
    sim.issue_command(charge).unwrap();
    place(&mut sim, 0, 800.0, 300.0, 0.0);
    let kinds = |sim: &BattleSim, side| -> Vec<(u32, Formation)> {
        plan_formations(sim, side)
            .into_iter()
            .filter_map(|c| match c {
                Command::Formation { units, kind } => Some((units[0], kind)),
                _ => None,
            })
            .collect()
    };
    assert_eq!(kinds(&sim, SideId::Attacker), vec![(0, Formation::Wedge)]);
    place(&mut sim, 0, 640.0, 310.0, 0.0);
    assert_eq!(kinds(&sim, SideId::Defender), vec![(1, Formation::Square)]);
    // A long march far from the enemy: column.
    place(&mut sim, 0, 100.0, 50.0, 0.0);
    let far = Command::Move {
        units: vec![2],
        x: 900.0,
        z: 750.0,
        run: false,
        facing: None,
    };
    sim.issue_command(far).unwrap();
    place(&mut sim, 2, 1100.0, 400.0, 0.0);
    sim.units_mut()[2].destination = Some((1100.0, 780.0));
    let plan = kinds(&sim, SideId::Defender);
    assert!(plan.contains(&(2, Formation::Column)), "{plan:?}");
}

#[test]
#[ignore]
fn deployment_phase_validates_zone() {}

#[test]
#[ignore]
fn siege_pathing_uses_breach() {}

#[test]
#[ignore]
fn reinforcements_enter_from_edge() {}

#[test]
#[ignore]
fn f5_is_deterministic() {}
