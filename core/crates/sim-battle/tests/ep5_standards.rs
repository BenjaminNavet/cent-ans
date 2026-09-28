//! EP5 (ADR 0034): regimental standards fall, are raised or taken.

mod common;

use common::*;
use data_model::BattleStandardRules;
use sim_battle::unit::{Formation, StandardState};
use sim_battle::{BattleSim, Command, SideId, UnitState};

const ARMY: [&str; 6] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
];

fn duel(rules: BattleStandardRules, seed: u64) -> BattleSim {
    let data = data();
    let mut s = setup(
        units(&data, &["unit_men_at_arms_foot"]),
        units(&data, &["unit_urban_militia", "unit_men_at_arms_foot"]),
        None,
    );
    s.standards = Some(rules);
    let mut sim = BattleSim::new(s, seed).unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 600.0, 400.0, 0.0);
    place(&mut sim, 1, 600.0, 409.0, std::f64::consts::PI);
    place(&mut sim, 2, 200.0, 700.0, std::f64::consts::PI);
    for (unit, target) in [(0, 1), (1, 0)] {
        sim.apply_command(
            Command::Attack {
                units: vec![unit],
                target,
                run: false,
                queue: false,
            },
            None,
        )
        .unwrap();
    }
    sim
}

fn always_fall() -> BattleStandardRules {
    BattleStandardRules {
        heavy_losses_below: 1.01,
        fall_chance_per_loss_percent: 1.0,
        ..BattleStandardRules::default()
    }
}

#[test]
fn data_rules_match_the_default() {
    let data = data();
    assert_eq!(
        BattleStandardRules {
            description: None,
            ..data.battle_standard_rules.clone()
        },
        BattleStandardRules::default()
    );
}

#[test]
fn bearers_stand_in_the_front_rank_centre() {
    let data = data();
    let s = setup(
        units(&data, &["unit_men_at_arms_foot", "unit_longbowmen"]),
        units(&data, &["unit_knights"]),
        None,
    );
    let mut sim = BattleSim::new(s, 1).unwrap();
    lab(&mut sim);
    for scale in [1.0, 2.5] {
        for i in 0..3 {
            let unit = &sim.units()[i];
            let slots = unit.standard_slots(scale, sim.standard_bearers(i));
            assert!(!slots.is_empty());
            let positions = unit.figure_positions(scale);
            let (fx, fz) = unit.forward();
            let (rx, rz) = unit.right();
            for &slot in &slots {
                let (x, z, _) = positions[slot as usize];
                let ahead = (x - unit.x) * fx + (z - unit.z) * fz;
                let side = (x - unit.x) * rx + (z - unit.z) * rz;
                let (width, depth) = unit.extent();
                if unit.can_shoot() {
                    // Shooters: in the middle of the depth.
                    assert!(
                        ahead.abs() < depth * 0.4,
                        "archer bearer {ahead} of {depth}"
                    );
                } else {
                    assert!(ahead > depth * 0.2, "bearer {ahead} m ahead, depth {depth}");
                }
                if slots.len() == 1 {
                    assert!(side.abs() < width * 0.1 + 1.5, "bearer {side} m off centre");
                }
            }
        }
    }
    // Wedge: the tip.
    sim.units_mut()[2].formation = Formation::Wedge;
    assert_eq!(sim.units()[2].standard_slots(1.0, 1), vec![0]);
}

#[test]
fn large_regiments_carry_two_standards() {
    let data = data();
    let s = setup(
        units(&data, &["unit_urban_militia"]),
        units(&data, &["unit_knights"]),
        None,
    );
    let rules = BattleStandardRules::default();
    let sim = BattleSim::new(s, 1).unwrap();
    for (i, unit) in sim.units().iter().enumerate() {
        let expected = if unit.max_soldiers >= rules.two_bearers_from_soldiers {
            2
        } else {
            1
        };
        assert_eq!(sim.standard_bearers(i), expected, "{}", unit.unit_type);
        let slots = unit.standard_slots(1.0, expected);
        assert_eq!(slots.len() as u32, expected);
        if expected == 2 {
            assert_ne!(slots[0], slots[1]);
        }
    }
}

#[test]
fn standard_falls_under_heavy_losses_with_morale_effects() {
    let mut sim = duel(always_fall(), 4);
    let mut fell_at = None;
    for step in 0..600 {
        sim.step();
        if sim.units()[1].standard != StandardState::Carried {
            fell_at = Some(step);
            break;
        }
    }
    assert!(fell_at.is_some(), "the militia's standard never fell");
    assert!(has_event(&sim, "est tombé"));
    let StandardState::Fallen { timer, .. } = sim.units()[1].standard else {
        panic!(
            "expected a fallen standard, got {:?}",
            sim.units()[1].standard
        );
    };
    assert!(timer > 0.0);
}

#[test]
fn fallen_standard_weakens_blows_and_morale() {
    // Same duel twice; in the second the defender's standard is down.
    let run_with = |down: bool| {
        let mut sim = duel(BattleStandardRules::default(), 9);
        if down {
            let (x, z) = sim.units()[1].standard_point();
            sim.units_mut()[1].standard = StandardState::Fallen { x, z, timer: 1.0e6 };
        }
        run(&mut sim, 10.0);
        (sim.units()[0].hp, sim.units()[1].morale)
    };
    let (hp_up, morale_up) = run_with(false);
    let (hp_down, morale_down) = run_with(true);
    assert!(
        hp_down > hp_up,
        "the enemy should lose fewer men ({hp_down} vs {hp_up})"
    );
    assert!(
        morale_down < morale_up,
        "morale {morale_down} vs {morale_up}"
    );
}

#[test]
fn standard_is_raised_when_the_regiment_holds() {
    let mut sim = duel(BattleStandardRules::default(), 2);
    // The enemy far away: nobody holds the spot.
    place(&mut sim, 0, 100.0, 100.0, 0.0);
    let (x, z) = sim.units()[1].standard_point();
    sim.units_mut()[1].standard = StandardState::Fallen { x, z, timer: 1.0 };
    run(&mut sim, 1.5);
    assert_eq!(sim.units()[1].standard, StandardState::Carried);
    assert!(has_event(&sim, "est relevé"));
    assert!(sim.trophies().is_empty());
}

#[test]
fn standard_is_taken_when_the_enemy_holds_the_spot() {
    let mut sim = duel(BattleStandardRules::default(), 2);
    let (x, z) = sim.units()[1].standard_point();
    let morale_before = sim.units()[0].morale;
    sim.units_mut()[1].standard = StandardState::Fallen { x, z, timer: 0.05 };
    sim.step();
    assert_eq!(
        sim.units()[1].standard,
        StandardState::Captured {
            by: sim.units()[0].id
        }
    );
    assert!(sim.units()[0].morale >= morale_before);
    assert!(has_event(&sim, "prennent l'étendard"));
    assert_eq!(sim.trophies().len(), 1);
    assert_eq!(sim.trophies()[0].taken_by, SideId::Attacker);
    assert_eq!(sim.trophies()[0].unit_type, "unit_urban_militia");
}

#[test]
fn trophies_reach_the_outcome() {
    let mut sim = duel(BattleStandardRules::default(), 2);
    let (x, z) = sim.units()[1].standard_point();
    sim.units_mut()[1].standard = StandardState::Fallen { x, z, timer: 0.05 };
    sim.step();
    // Finish the battle: the defender's regiments flee.
    sim.set_end_conditions(true);
    for u in sim
        .units_mut()
        .iter_mut()
        .filter(|u| u.side == SideId::Defender)
    {
        u.state = UnitState::Routing;
        u.morale = 0.0;
    }
    run(&mut sim, 5.0);
    let outcome = sim.outcome().expect("finished");
    assert_eq!(outcome.winner, SideId::Attacker);
    assert_eq!(outcome.attacker.standards_taken.len(), 1);
    assert_eq!(outcome.defender.standards_lost, 1);
    assert!(outcome.defender.standards_taken.is_empty());
}

#[test]
fn standards_are_deterministic() {
    let data = data();
    let run_once = || {
        let mut sim =
            BattleSim::new(setup(units(&data, &ARMY), units(&data, &ARMY), None), 11).unwrap();
        run_to_end(&mut sim);
        let states: Vec<StandardState> = sim.units().iter().map(|u| u.standard).collect();
        (states, sim.trophies().to_vec(), sim.outcome())
    };
    assert_eq!(run_once(), run_once());
}

#[test]
fn full_battles_see_standards_fall() {
    // Balance probe: over a few AI battles some standards fall, few are taken.
    let data = data();
    let (mut fallen, mut taken) = (0, 0);
    for seed in 0..6 {
        let mut sim =
            BattleSim::new(setup(units(&data, &ARMY), units(&data, &ARMY), None), seed).unwrap();
        run_to_end(&mut sim);
        fallen += sim
            .events()
            .iter()
            .filter(|e| e.text_fr.contains("est tombé !") && e.text_fr.contains("étendard"))
            .count();
        taken += sim.trophies().len();
    }
    println!("EP5 probe: {fallen} standards fell, {taken} taken over 6 battles");
    assert!(fallen > 0, "no standard ever fell");
}
