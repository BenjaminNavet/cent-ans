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
        queue: false,
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
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
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
        queue: false,
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
        queue: false,
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
        queue: false,
    })
    .unwrap();
    run(&mut sim, 20.0);
    assert!(sim.units()[3].hp < before, "the free regiment is shot at");
}

/// In a siege, a regiment withdrawing from inside the town goes round the
/// houses and out through the breach (it used to walk into the wall).
#[test]
fn withdrawal_from_the_town_goes_through_the_breach() {
    let data = data();
    let attacker = common::units(&data, &["unit_men_at_arms_foot"]);
    let defender = common::units(&data, &["unit_urban_militia"]);
    let siege = sim_battle::SiegeSetup {
        fortification: 2,
        breach: 0,
        engines: None,
    };
    let mut sim = BattleSim::new(setup(attacker, defender, Some(siege)), 11).unwrap();
    lab(&mut sim);
    let works = sim.siege().unwrap().clone();
    let breach = *works.front_walls().last().unwrap();
    sim.siege_mut().unwrap().pieces[breach].hp = 0.0;
    let (cx, cz) = works.center;
    place(&mut sim, 0, cx, cz, 0.0);
    place(&mut sim, 1, 1100.0, 780.0, 0.0);
    sim.issue_command(Command::Withdraw { units: vec![0] })
        .unwrap();
    let mut left = false;
    for _ in 0..6000 {
        sim.step();
        if sim.units()[0].left_field {
            left = true;
            break;
        }
    }
    let u = &sim.units()[0];
    assert!(left, "stuck at ({:.0}, {:.0})", u.x, u.z);
}

/// The AI pulls a bled regiment out of a melee fought near its own edge
/// with a move kept on the field (it used to be refused as outside).
#[test]
fn ai_pull_out_stays_on_the_field() {
    let data = data();
    let foot = || unit(&data, "unit_men_at_arms_foot");
    let mut sim = lab_sim(
        vec![foot(), foot(), foot(), foot(), foot()],
        vec![unit(&data, "unit_urban_militia")],
    );
    sim.field_mut().forests.clear();
    sim.field_mut().forest_parts.clear();
    // Unit 0 fights near the attacker's edge; unit 4 is the reserve.
    place(&mut sim, 0, 600.0, 30.0, 0.0);
    place(&mut sim, 1, 300.0, 60.0, 0.0);
    place(&mut sim, 2, 400.0, 60.0, 0.0);
    place(&mut sim, 3, 800.0, 60.0, 0.0);
    place(&mut sim, 4, 900.0, 20.0, 0.0);
    place(&mut sim, 5, 600.0, 80.0, std::f64::consts::PI);
    sim.apply_command(
        Command::Attack {
            units: vec![5],
            target: 0,
            run: false,
            queue: false,
        },
        None,
    )
    .unwrap();
    for _ in 0..600 {
        sim.step();
        if sim.units()[0].state == UnitState::Melee {
            break;
        }
    }
    assert_eq!(sim.units()[0].state, UnitState::Melee, "contact reached");
    let soldiers = f64::from(sim.units()[0].initial_soldiers);
    sim.units_mut()[0].morale = 25.0;
    sim.units_mut()[0].hp = soldiers * 0.4;
    let commands = sim_battle::ai::plan(&sim, sim_battle::SideId::Attacker);
    let moves: Vec<(f64, f64)> = commands
        .iter()
        .filter_map(|c| match c {
            Command::Move { units, x, z, .. } if units.contains(&0) => Some((*x, *z)),
            _ => None,
        })
        .collect();
    assert!(!moves.is_empty(), "the bled regiment is pulled out");
    for (x, z) in moves {
        assert!(sim.field().inside(x, z), "({x:.0}, {z:.0}) off the field");
    }
}

/// A regiment killed by missiles during the step is no longer the melee
/// opponent of that step: the blow goes to a living enemy in contact.
#[test]
fn no_blow_wasted_on_a_regiment_shot_dead_this_step() {
    let data = data();
    let mut sim = lab_sim(
        vec![
            unit(&data, "unit_men_at_arms_foot"),
            unit(&data, "unit_longbowmen"),
        ],
        vec![
            unit(&data, "unit_urban_militia"),
            unit(&data, "unit_urban_militia"),
        ],
    );
    sim.field_mut().forests.clear();
    sim.field_mut().forest_parts.clear();
    let depth = |sim: &BattleSim, i: usize| sim.units()[i].extent().1;
    let front = 400.0 + (depth(&sim, 0) + depth(&sim, 3)) * 0.5 + 0.5;
    place(&mut sim, 0, 600.0, 400.0, 0.0);
    place(&mut sim, 1, 600.0, 300.0, 0.0);
    // A routing, nearly spent regiment right on top of the men-at-arms
    // (their nearest contact), a fresh one in front of them.
    place(&mut sim, 2, 600.0, 400.0, 0.0);
    place(&mut sim, 3, 600.0, front, std::f64::consts::PI);
    {
        let units = sim.units_mut();
        units[2].state = UnitState::Routing;
        units[2].hp = 0.5;
        units[1].reload = 0.0;
    }
    let before = sim.units()[3].hp;
    sim.step();
    assert!(!sim.units()[2].present(), "shot dead");
    assert!(
        sim.units()[3].hp < before,
        "the fresh regiment takes the blow"
    );
}

/// The siege pathing keeps its obstacle cells between searches: the same
/// way twice, and a new one once a wall comes down.
#[test]
fn siege_pathing_cache_follows_the_walls() {
    let data = data();
    let attacker = common::units(&data, &["unit_men_at_arms_foot"]);
    let defender = common::units(&data, &["unit_urban_militia"]);
    let siege = sim_battle::SiegeSetup {
        fortification: 2,
        breach: 0,
        engines: None,
    };
    let mut sim = BattleSim::new(setup(attacker, defender, Some(siege)), 11).unwrap();
    lab(&mut sim);
    let works = sim.siege().unwrap().clone();
    let side = sim_battle::SideId::Attacker;
    let (gx, gz) = works.pieces[works.gate].midpoint();
    let from = (gx, gz - 80.0);
    let to = works.center;
    let first = sim.siege_route(side, from, to);
    assert_eq!(first, sim.siege_route(side, from, to));
    let breach = *works.front_walls().last().unwrap();
    sim.siege_mut().unwrap().pieces[breach].hp = 0.0;
    let opened = sim.siege_route(side, from, to);
    assert!(opened.is_some(), "a way through the breach");
    assert_ne!(first, opened);
    let mut fresh = BattleSim::new(sim.setup().clone(), 11).unwrap();
    fresh.siege_mut().unwrap().pieces[breach].hp = 0.0;
    assert_eq!(opened, fresh.siege_route(side, from, to));
}

/// Review point left open after fix 2, rechecked after CB4 (RS-D), fixed by
/// RS-J: crossbowmen behind their pavises, ordered to shoot a regiment
/// hidden in a wood within bowshot, lower their pavises and close in like
/// the archers of `archers_close_in_on_a_hidden_target`, then shoot.
#[test]
fn pavised_crossbowmen_close_in_on_a_hidden_target() {
    let data = data();
    let mut battle = setup(
        vec![unit(&data, "unit_crossbowmen")],
        vec![unit(&data, "unit_urban_militia")],
        None,
    );
    battle.abilities = data.battle_abilities.values().cloned().collect();
    let mut sim = BattleSim::new(battle, 7).unwrap();
    lab(&mut sim);
    sim.field_mut().forests.clear();
    sim.field_mut().forest_parts.clear();
    sim.field_mut().forests.push(sim_battle::Zone {
        x: 600.0,
        z: 520.0,
        radius: 40.0,
    });
    place(&mut sim, 0, 600.0, 380.0, 0.0);
    place(&mut sim, 1, 600.0, 520.0, std::f64::consts::PI);
    sim.issue_command(Command::UseAbility {
        units: vec![0],
        ability: "ability_pavise".to_owned(),
    })
    .unwrap();
    run(&mut sim, 6.0);
    assert!(sim.units()[0].pavise.is_some(), "pavises up");
    let before = sim.units()[1].hp;
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 1,
        run: false,
        queue: false,
    })
    .unwrap();
    run(&mut sim, 60.0);
    assert!(sim.units()[0].z > 440.0, "z {}", sim.units()[0].z);
    assert!(sim.units()[1].hp < before, "the hidden regiment is shot at");
}

/// RS-J non-regression: the same crossbowmen facing a regiment in the open
/// within bowshot keep their pavises up, hold their ground and shoot.
#[test]
fn pavised_crossbowmen_hold_and_shoot_a_visible_target() {
    let data = data();
    let mut battle = setup(
        vec![unit(&data, "unit_crossbowmen")],
        vec![unit(&data, "unit_urban_militia")],
        None,
    );
    battle.abilities = data.battle_abilities.values().cloned().collect();
    let mut sim = BattleSim::new(battle, 7).unwrap();
    lab(&mut sim);
    sim.field_mut().forests.clear();
    sim.field_mut().forest_parts.clear();
    place(&mut sim, 0, 600.0, 380.0, 0.0);
    place(&mut sim, 1, 600.0, 520.0, std::f64::consts::PI);
    sim.issue_command(Command::UseAbility {
        units: vec![0],
        ability: "ability_pavise".to_owned(),
    })
    .unwrap();
    run(&mut sim, 6.0);
    assert!(sim.units()[0].pavise.is_some(), "pavises up");
    let before = sim.units()[1].hp;
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 1,
        run: false,
        queue: false,
    })
    .unwrap();
    run(&mut sim, 30.0);
    assert!(sim.units()[0].pavise.is_some(), "pavises still up");
    assert!(
        (sim.units()[0].z - 380.0).abs() < 1.0,
        "z {}",
        sim.units()[0].z
    );
    assert!(
        sim.units()[1].hp < before,
        "the visible regiment is shot at"
    );
}
