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
        queue: false,
    };
    sim.issue_command(command).unwrap();
    // R2: the militia may break before the minute is out (relief of the
    // field), so the contact is looked for during the run.
    let mut reached = false;
    for _ in 0..60 {
        run(&mut sim, 1.0);
        reached |= sim.units()[1].state == UnitState::Melee;
    }
    assert!(
        (sim.units()[0].x - 600.0).abs() < 0.5,
        "the foot are not shoved"
    );
    assert!(reached, "the knights reach the enemy");
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
        queue: false,
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
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
    };
    sim.issue_command(far).unwrap();
    place(&mut sim, 2, 1100.0, 400.0, 0.0);
    sim.units_mut()[2].destination = Some((1100.0, 780.0));
    let plan = kinds(&sim, SideId::Defender);
    assert!(plan.contains(&(2, Formation::Column)), "{plan:?}");
}

#[test]
fn deployment_phase_validates_zone() {
    use sim_battle::{CommandError, SideId};
    let data = data();
    let army = ["unit_men_at_arms_foot", "unit_longbowmen", "unit_knights"];
    let mut s = setup(units(&data, &army), units(&data, &army), None);
    s.player_side = Some(SideId::Attacker);
    let mut sim = BattleSim::new(s, 4).unwrap();
    assert!(sim.begin_deployment());
    assert!(sim.is_deploying());
    let zone = sim.deployment_zone(SideId::Attacker);
    assert!(sim
        .units()
        .iter()
        .all(|u| { !u.present() || sim.deployment_zone(u.side).contains(u.x, u.z) }));
    // Inside the zone: accepted; beyond it or an enemy regiment: refused.
    sim.deploy_unit(0, 300.0, zone.z1 - 10.0, Some(0.3))
        .unwrap();
    assert_eq!((sim.units()[0].x, sim.units()[0].facing), (300.0, 0.3));
    let far = sim.deploy_unit(0, 300.0, zone.z1 + 50.0, None);
    assert_eq!(far, Err(CommandError::OutsideZone(0)));
    let text = sim.error_text(&far.unwrap_err());
    let name = &sim.units()[0].name;
    assert_eq!(
        text,
        format!("Les {name} doivent être placés dans votre zone de déploiement")
    );
    assert_eq!(
        sim.deploy_unit(3, 300.0, 700.0, None),
        Err(CommandError::NotYours(3))
    );
    // Time is frozen and move orders wait for the battle.
    sim.tick(5.0);
    assert_eq!(sim.ticks(), 0);
    let order = Command::Halt { units: vec![0] };
    assert_eq!(
        sim.issue_command(order.clone()),
        Err(CommandError::Deploying)
    );
    sim.start_battle().unwrap();
    assert!(!sim.is_deploying());
    assert_eq!(
        sim.deploy_unit(0, 300.0, 100.0, None),
        Err(CommandError::NotDeploying)
    );
    sim.issue_command(order).unwrap();
    sim.tick(1.0);
    assert_eq!(sim.ticks(), 10);
}

#[test]
fn siege_besiegers_deploy_outside_the_walls() {
    use sim_battle::{CommandError, SideId, SiegeSetup};
    let data = data();
    let attacker = units(&data, &["unit_men_at_arms_foot", "unit_longbowmen"]);
    let defender = units(&data, &["unit_urban_militia"]);
    let siege = SiegeSetup {
        fortification: 1,
        breach: 0,
        ..Default::default()
    };
    let mut sim = BattleSim::new(setup(attacker, defender, Some(siege)), 4).unwrap();
    assert!(sim.begin_deployment());
    let (cx, cz) = sim.siege().unwrap().center;
    assert_eq!(
        sim.deploy_unit(0, cx, cz, None),
        Err(CommandError::OutsideZone(0))
    );
    sim.deploy_unit(0, 500.0, 200.0, None).unwrap();
    // The garrison stays inside the ring.
    assert_eq!(
        sim.deploy_unit(2, 500.0, 200.0, None),
        Err(CommandError::OutsideZone(2))
    );
    sim.deploy_unit(2, cx + 10.0, cz, None).unwrap();
    let zone = sim.deployment_zone(SideId::Defender);
    assert!(zone.contains(cx, cz));
}

#[test]
fn siege_pathing_uses_breach() {
    use sim_battle::SiegeSetup;
    let data = data();
    let attacker = units(&data, &["unit_mounted_sergeants", "unit_men_at_arms_foot"]);
    let defender = units(&data, &["unit_urban_militia"]);
    let siege = SiegeSetup {
        fortification: 2,
        breach: 0,
        ..Default::default()
    };
    let mut sim = BattleSim::new(setup(attacker, defender, Some(siege)), 11).unwrap();
    lab(&mut sim);
    let works = sim.siege().unwrap().clone();
    assert!(
        works.houses.len() >= 12,
        "streets and houses: {}",
        works.houses.len()
    );
    // Open the last front wall, start far on the other side of the gate.
    let front = works.front_walls();
    let breach = *front.last().unwrap();
    sim.siege_mut().unwrap().pieces[breach].hp = 0.0;
    let (bx, _) = works.pieces[breach].midpoint();
    let (gx, gz) = works.pieces[works.gate].midpoint();
    place(&mut sim, 0, gx + (gx - bx) * 1.5, gz - 60.0, 0.0);
    place(&mut sim, 2, 1100.0, 780.0, 0.0);
    let (cx, cz) = works.center;
    let order = Command::Move {
        units: vec![0],
        x: cx,
        z: cz,
        run: true,
        facing: None,
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
    };
    sim.issue_command(order).unwrap();
    let mut arrived = false;
    for _ in 0..3000 {
        sim.step();
        let u = &sim.units()[0];
        assert!(
            works.house_at(u.x, u.z, -1.0).is_none(),
            "never through a house"
        );
        if (u.x - cx).powi(2) + (u.z - cz).powi(2) < 20.0 * 20.0 {
            arrived = true;
            break;
        }
    }
    let u = &sim.units()[0];
    assert!(
        arrived,
        "rides round to the breach, then up the street: ({:.0}, {:.0}) {:?}",
        u.x, u.z, u.state
    );
}

#[test]
fn towers_shoot_and_the_garrison_sallies() {
    use sim_battle::{SideId, SiegeSetup};
    let data = data();
    let attacker = units(&data, &["unit_men_at_arms_foot", "unit_men_at_arms_foot"]);
    let defender = units(&data, &["unit_men_at_arms_foot", "unit_urban_militia"]);
    let siege = SiegeSetup {
        fortification: 1,
        breach: 0,
        ..Default::default()
    };
    let mut sim = BattleSim::new(setup(attacker, defender, Some(siege)), 3).unwrap();
    lab(&mut sim);
    for id in [2, 3] {
        let halt = Command::FireAtWill {
            units: vec![id],
            enabled: false,
        };
        let _ = sim.issue_command(halt);
    }
    // A regiment 60 m in front of a tower, out of reach of the garrison.
    let tower = sim.siege().unwrap().towers[3];
    place(&mut sim, 0, tower.x, tower.z - 60.0, 0.0);
    place(&mut sim, 1, 100.0, 40.0, 0.0);
    let before = sim.units()[0].hp;
    run(&mut sim, 60.0);
    assert!(sim.units()[0].hp < before - 1.0, "the towers shoot");
    assert!(
        sim.units()[1].hp >= f64::from(sim.units()[1].initial_soldiers),
        "out of range"
    );
    // The besiegers bled white: the AI garrison opens its gate.
    sim.set_ai(SideId::Defender, true);
    place(&mut sim, 0, 300.0, 40.0, 0.0);
    for id in [0, 1] {
        sim.units_mut()[id].hp = 20.0;
    }
    run(&mut sim, 90.0);
    assert!(sim.siege().unwrap().sortie, "sortie");
    let sallied = sim
        .units()
        .iter()
        .any(|u| u.side == SideId::Defender && u.target.is_some_and(|t| t <= 1));
    let state: Vec<_> = sim
        .units()
        .iter()
        .map(|u| (u.id, u.side, u.state, u.on_wall, u.target, u.hp.round()))
        .collect();
    assert!(sallied, "the garrison falls on the besiegers: {state:?}");
    run(&mut sim, 120.0);
    let works = sim.siege().unwrap().clone();
    let out = sim
        .units()
        .iter()
        .any(|u| u.side == SideId::Defender && u.present() && !works.inside(u.x, u.z));
    assert!(out, "out through the gate");
}

// F5a § 5 (staggered reinforcements beyond 20 regiments): see `tests/f5d.rs`.

/// Same setup, seed and deployment: same battle, field and siege alike.
#[test]
fn f5_is_deterministic() {
    use sim_battle::SiegeSetup;
    let data = data();
    let army = [
        "unit_men_at_arms_foot",
        "unit_flemish_pikemen",
        "unit_longbowmen",
        "unit_knights",
        "unit_knights",
        "unit_urban_militia",
    ];
    for siege in [
        None,
        Some(SiegeSetup {
            fortification: 1,
            breach: 60,
            ..Default::default()
        }),
    ] {
        let run_once = || {
            let s = setup(units(&data, &army), units(&data, &army), siege.clone());
            let mut sim = BattleSim::new(s, 21).unwrap();
            assert!(sim.begin_deployment());
            let (x, z) = (sim.units()[0].x + 20.0, sim.units()[0].z - 10.0);
            sim.deploy_unit(0, x, z, None).unwrap();
            sim.start_battle().unwrap();
            run_to_end(&mut sim);
            let units: Vec<(u64, u64)> = sim
                .units()
                .iter()
                .map(|u| (u.x.to_bits(), u.hp.to_bits()))
                .collect();
            (sim.outcome(), sim.ticks(), units)
        };
        assert_eq!(run_once(), run_once());
    }
}
