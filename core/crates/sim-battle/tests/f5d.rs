//! F5d tests: the Godot demo battle engages, siege balance, staggered reinforcements.

mod common;

use common::*;
use sim_battle::{BattleSetup, BattleSim, SideId, SiegeSetup, UnitState};

/// Escalade without breach nor engines (probe `siege 0 ""`): about half
/// the assaults succeed, as in M8 (the F5a tower fire had cut it to 20 %).
#[test]
fn a_ladder_escalade_wins_about_half_the_time() {
    let data = data();
    let besiegers = [
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_urban_militia",
        "unit_urban_militia",
        "unit_urban_militia",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_knights",
    ];
    let garrison = [
        "unit_urban_militia",
        "unit_urban_militia",
        "unit_crossbowmen",
        "unit_crossbowmen",
        "unit_men_at_arms_foot",
    ];
    let mut wins = 0;
    for seed in 0..6 {
        let siege = SiegeSetup {
            fortification: 2,
            breach: 0,
        };
        let battle = setup(
            units(&data, &besiegers),
            units(&data, &garrison),
            Some(siege),
        );
        let mut sim = BattleSim::new(battle, seed).unwrap();
        run_to_end(&mut sim);
        wins += u32::from(sim.winner() == Some(SideId::Attacker));
    }
    assert!((2..=4).contains(&wins), "escalade wins {wins}/6");
}

#[test]
fn surplus_regiments_wait_and_march_in_from_their_edge() {
    use sim_battle::MAX_ON_FIELD;
    let data = data();
    let many: Vec<&str> = ["unit_men_at_arms_foot", "unit_longbowmen"].repeat(12);
    let battle = setup(units(&data, &many), units(&data, &many[..8]), None);
    let mut sim = BattleSim::new(battle, 4).unwrap();
    lab(&mut sim);
    let waiting: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.reserve)
        .map(|u| u.id)
        .collect();
    assert_eq!(
        waiting,
        vec![20, 21, 22, 23],
        "the last four attackers wait"
    );
    assert!(sim
        .units()
        .iter()
        .filter(|u| u.reserve)
        .all(|u| !u.present()));
    assert_eq!(sim.reserves(SideId::Attacker), 4);
    // A fielded regiment is destroyed: the first waiting one marches in.
    sim.units_mut()[3].hp = 0.0;
    sim.step();
    let fresh = &sim.units()[20];
    assert!(!fresh.reserve && fresh.present());
    assert!(
        fresh.z < 30.0,
        "enters by the attacker's edge, z = {}",
        fresh.z
    );
    assert_eq!(sim.reserves(SideId::Attacker), 3);
    let fielded = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker && u.present());
    assert_eq!(fielded.count(), MAX_ON_FIELD);
}

/// The standalone demo of `battle.tscn` (France 1337, main French army
/// against the main English one), dumped from `sim-campaign`.
fn demo_setup() -> BattleSetup {
    serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap()
}

/// Like the scene's `--screenshot`: both sides driven by the AI.
fn demo_sim() -> BattleSim {
    let mut sim = BattleSim::new(demo_setup(), 1337).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim
}

fn first_contact(sim: &mut BattleSim, limit: f64) -> Option<f64> {
    while sim.elapsed() < limit && !sim.is_finished() {
        sim.step();
        if sim.units().iter().any(|u| u.state == UnitState::Melee) {
            return Some(sim.elapsed());
        }
    }
    None
}

#[test]
fn demo_battle_reaches_contact_quickly() {
    let mut sim = demo_sim();
    let contact = first_contact(&mut sim, 300.0);
    assert!(contact.is_some_and(|t| t < 150.0), "contact at {contact:?}");
}

#[test]
fn demo_battle_nobody_halts_in_the_river() {
    let mut sim = demo_sim();
    let river = sim.field().river.clone().unwrap();
    while sim.elapsed() < 150.0 {
        sim.step();
        for unit in sim.units().iter().filter(|u| u.state == UnitState::Idle) {
            let wet = river.in_water(unit.x, unit.z) && !river.in_ford(unit.x);
            assert!(
                !wet,
                "{} stands in the river at {:.0} s",
                unit.name,
                sim.elapsed()
            );
        }
    }
}

#[test]
fn demo_battle_nobody_deploys_in_the_river() {
    let mut sim = demo_sim();
    assert!(sim.begin_deployment());
    let river = sim
        .field()
        .river
        .clone()
        .expect("the demo field has a river");
    for unit in sim.units() {
        let wet = river.in_water(unit.x, unit.z) && !river.in_ford(unit.x);
        assert!(!wet, "{} deployed in the river", unit.name);
    }
}

#[test]
fn destinations_in_the_river_move_to_a_bank() {
    let sim = demo_sim();
    let field = sim.field();
    let river = field.river.clone().unwrap();
    let x = 600.0;
    assert!(!river.in_ford(x));
    let c = river.center_z(x);
    // Standing on the south bank: stay south; already wading: cross.
    let south = sim_battle::ai::dry_z(field, x, c, c - 80.0, 1.0);
    let wading = sim_battle::ai::dry_z(field, x, c, c + 2.0, 1.0);
    assert!(south < c - river.width * 0.5 && !river.in_water(x, south));
    assert!(wading > c + river.width * 0.5 && !river.in_water(x, wading));
    // Fords and dry land are left alone.
    let ford = river.fords[0].x;
    let ford_c = river.center_z(ford);
    assert_eq!(sim_battle::ai::dry_z(field, ford, ford_c, 0.0, 1.0), ford_c);
    assert_eq!(
        sim_battle::ai::dry_z(field, x, c - 100.0, 0.0, 1.0),
        c - 100.0
    );
}

#[test]
#[ignore]
fn trace_big() {
    let data = data();
    let kinds = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_knights",
        "unit_urban_militia",
        "unit_crossbowmen",
    ];
    let army: Vec<&str> = (0..30).map(|i| kinds[i % 5]).collect();
    let mut battle = setup(units(&data, &army), units(&data, &army), None);
    battle.river = false;
    let mut sim = BattleSim::new(battle, 7).unwrap();
    while sim.elapsed() < 1500.0 && !sim.is_finished() {
        sim.step();
    }
    for u in sim.units().iter().filter(|u| u.present() || u.reserve) {
        println!(
            "{:?} #{} {} ({:.0},{:.0}) {:?} dest {:?} tgt {:?} res {} hp {:.0}",
            u.side, u.id, u.name, u.x, u.z, u.state, u.destination, u.target, u.reserve, u.hp
        );
    }
}

#[test]
#[ignore]
fn trace_demo() {
    let mut probe = demo_sim();
    println!("contact {:?}", first_contact(&mut probe, 300.0));
    while !probe.is_finished() {
        probe.step();
    }
    println!("end {:.0} s winner {:?}", probe.elapsed(), probe.winner());
    let mut sim = demo_sim();
    let river = sim.field().river.clone().unwrap();
    println!(
        "river z0 {:.0} amp {:.0} fords {:?}",
        river.z0, river.amplitude, river.fords
    );
    for step in 0..=10 {
        while sim.elapsed() < step as f64 * 30.0 {
            sim.step();
        }
        println!("t={:.0}", sim.elapsed());
        for u in sim.units() {
            println!(
                "  {:?} {:24} {:6.0} {:6.0} rz {:4.0} {:?} {:?} dest {:?} tgt {:?} wet {}",
                u.side,
                u.name,
                u.x,
                u.z,
                river.center_z(u.x),
                u.state,
                u.formation,
                u.destination.map(|d| (d.0 as i32, d.1 as i32)),
                u.target,
                river.in_water(u.x, u.z)
            );
        }
    }
}
