//! TW reinf (ADR 0330): allied reinforcements with a timed arrival, by the
//! edge they come from; relief armies of a siege enter behind the besiegers.

use crate::common;

use common::*;
use sim_battle::{BattleSim, EntryEdge, SideId, SiegeSetup};

fn delayed_battle(arrival: f64, edge: EntryEdge, seed: u64) -> BattleSim {
    let data = data();
    let mut attackers = units(data, &["unit_men_at_arms_foot", "unit_longbowmen"]);
    let mut late = units(data, &["unit_knights", "unit_men_at_arms_foot"]);
    for u in &mut late {
        u.arrival_s = Some(arrival);
        u.entry_edge = edge;
    }
    attackers.extend(late);
    let defenders = units(data, &["unit_men_at_arms_foot", "unit_longbowmen"]);
    let mut sim = BattleSim::new(setup(attackers, defenders, None), seed).unwrap();
    lab(&mut sim);
    sim
}

#[test]
fn a_delayed_regiment_waits_then_enters_on_time() {
    let mut sim = delayed_battle(30.0, EntryEdge::Own, 1);
    assert!(sim.units()[2].reserve && sim.units()[3].reserve);
    assert!(!sim.units()[0].reserve);
    let eta = sim.next_arrival_in(SideId::Attacker).unwrap();
    assert!((eta - 30.0).abs() < 1.0, "eta {eta}");
    assert_eq!(sim.next_arrival_in(SideId::Defender), None);
    while sim.elapsed() < 29.0 {
        sim.step();
    }
    assert!(sim.units()[2].reserve, "not before its hour");
    while sim.elapsed() < 31.0 {
        sim.step();
    }
    assert!(!sim.units()[2].reserve && sim.units()[2].present());
    assert_eq!(sim.next_arrival_in(SideId::Attacker), None);
    assert!(
        sim.units()[2].z < 60.0,
        "own edge, z = {}",
        sim.units()[2].z
    );
}

#[test]
fn the_entry_edge_follows_the_origin() {
    for (edge, west) in [(EntryEdge::West, true), (EntryEdge::East, false)] {
        let mut sim = delayed_battle(5.0, edge, 2);
        while sim.elapsed() < 6.0 {
            sim.step();
        }
        let x = sim.units()[2].x;
        let mid = sim.field().width * 0.5;
        assert_eq!(x < mid, west, "edge {edge:?}: x = {x}");
        assert!(
            x < 120.0 || x > sim.field().width - 120.0,
            "near the edge: {x}"
        );
    }
    let mut sim = delayed_battle(5.0, EntryEdge::Rear, 2);
    while sim.elapsed() < 6.0 {
        sim.step();
    }
    assert!(
        sim.units()[2].z > 300.0,
        "behind the enemy line, z = {}",
        sim.units()[2].z
    );
}

#[test]
fn timed_arrivals_are_deterministic() {
    let run = |seed| {
        let mut sim = delayed_battle(20.0, EntryEdge::West, seed);
        for _ in 0..2000 {
            sim.step();
        }
        sim.units()
            .iter()
            .map(|u| (u.x.to_bits(), u.z.to_bits(), u.reserve))
            .collect::<Vec<_>>()
    };
    assert_eq!(run(7), run(7));
}

#[test]
fn a_relief_army_enters_a_siege_behind_the_besiegers() {
    let data = data();
    let mut garrison = units(data, &["unit_urban_militia", "unit_crossbowmen"]);
    let mut relief = units(data, &["unit_knights", "unit_men_at_arms_foot"]);
    for u in &mut relief {
        u.arrival_s = Some(40.0);
        u.entry_edge = EntryEdge::Rear;
    }
    garrison.extend(relief);
    let besiegers = units(
        data,
        &[
            "unit_men_at_arms_foot",
            "unit_longbowmen",
            "unit_urban_militia",
        ],
    );
    let siege = SiegeSetup {
        fortification: 2,
        breach: 0,
        ..Default::default()
    };
    let mut sim = BattleSim::new(setup(besiegers, garrison, Some(siege)), 3).unwrap();
    lab(&mut sim);
    assert!(sim.units()[5].reserve && sim.units()[6].reserve);
    assert_eq!(sim.reserves(SideId::Defender), 2);
    while sim.elapsed() < 41.0 {
        sim.step();
    }
    let knight = &sim.units()[5];
    assert!(!knight.reserve && knight.side == SideId::Defender);
    assert!(
        knight.z < 60.0,
        "arrives at the attackers' edge, z = {}",
        knight.z
    );
}
