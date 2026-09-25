//! Tactical battle AI (spec `docs/design/m9-ai.md` § 2).

mod common;

use common::*;
use data_model::{GameData, Terrain};
use sim_battle::{BattleSim, Command, SideId, UnitSetup, UnitState};

const ARMY: [&str; 6] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
];

fn mirrored(data: &GameData, seed: u64) -> BattleSim {
    BattleSim::new(setup(units(data, &ARMY), units(data, &ARMY), None), seed).unwrap()
}

#[test]
fn ai_beats_a_passive_ai_at_equal_forces() {
    let data = data();
    for seed in 0..3 {
        for passive in SideId::BOTH {
            let mut sim = mirrored(&data, seed);
            sim.set_ai(passive, false);
            run_to_end(&mut sim);
            assert_eq!(
                sim.winner(),
                Some(passive.other()),
                "seed {seed}: the AI should beat a passive {:?}",
                passive
            );
        }
    }
}

#[test]
fn cavalry_charges_an_exposed_flank() {
    let data = data();
    let attacker: Vec<UnitSetup> = units(&data, &["unit_men_at_arms_foot", "unit_knights"]);
    let defender = units(&data, &["unit_urban_militia"]);
    let mut sim = BattleSim::new(setup(attacker, defender, None), 3).unwrap();
    lab(&mut sim);
    sim.set_ai(SideId::Attacker, true);
    // Men-at-arms (0) locked in melee with the militia (2), front to front;
    // the knights (1) wait on the left, well off the militia's front.
    place(&mut sim, 2, 600.0, 420.0, std::f64::consts::PI);
    place(&mut sim, 0, 600.0, 411.0, 0.0);
    place(&mut sim, 1, 380.0, 330.0, 0.0);
    let mut flank_hit = false;
    for _ in 0..1200 {
        sim.step();
        let knights = &sim.units()[1];
        let militia = &sim.units()[2];
        if knights.state == UnitState::Melee && militia.flanked != 0 {
            flank_hit = true;
            break;
        }
    }
    assert!(
        flank_hit,
        "the knights should hit the militia in the flank or rear"
    );
    assert_eq!(sim.units()[1].target, Some(2));
}

#[test]
fn archers_withdraw_when_engaged() {
    let data = data();
    let attacker = units(&data, &["unit_men_at_arms_foot", "unit_longbowmen"]);
    let defender = units(&data, &["unit_men_at_arms_foot"]);
    let mut sim = BattleSim::new(setup(attacker, defender, None), 5).unwrap();
    lab(&mut sim);
    sim.set_ai(SideId::Attacker, true);
    // Longbows (1) ahead of their men-at-arms (0); enemy men-at-arms (2)
    // charge the archers.
    place(&mut sim, 0, 600.0, 250.0, 0.0);
    place(&mut sim, 1, 600.0, 300.0, 0.0);
    place(&mut sim, 2, 600.0, 420.0, std::f64::consts::PI);
    sim.issue_command(Command::Attack {
        units: vec![2],
        target: 1,
        run: true,
    })
    .unwrap();
    let mut fell_back = false;
    for _ in 0..900 {
        sim.step();
        let (line, archers) = (&sim.units()[0], &sim.units()[1]);
        if archers.z < line.z - 15.0 {
            fell_back = true;
            break;
        }
    }
    assert!(fell_back, "the archers should fall back behind their line");
    let archers = &sim.units()[1];
    assert!(archers.present() && archers.state != UnitState::Routing);
}

#[test]
fn a_weaker_side_takes_the_high_ground_and_plants_stakes() {
    let data = data();
    let attacker = units(
        &data,
        &[
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_knights",
            "unit_knights",
            "unit_knights",
            "unit_crossbowmen",
        ],
    );
    let defender = units(
        &data,
        &[
            "unit_men_at_arms_foot",
            "unit_longbowmen",
            "unit_longbowmen",
        ],
    );
    let mut battle = setup(attacker, defender, None);
    battle.terrain = Terrain::Hills;
    let mut sim = BattleSim::new(battle, 9).unwrap();
    sim.set_ai(SideId::Attacker, false);
    let start: Vec<f64> = sim
        .units()
        .iter()
        .map(|u| sim.field().height(u.x, u.z))
        .collect();
    run(&mut sim, 90.0);
    let longbows: Vec<_> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender && u.unit_type == "unit_longbowmen")
        .collect();
    assert!(longbows.iter().all(|u| u.stakes_planted), "stakes planted");
    let line = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender && u.unit_type == "unit_men_at_arms_foot")
        .unwrap();
    assert!(
        sim.field().height(line.x, line.z) >= start[line.id as usize] - 0.5,
        "the line does not leave the high ground"
    );
    assert!(
        (line.z - 550.0).abs() < 160.0,
        "the weaker defender waits near its deployment"
    );
}

#[test]
fn ai_battles_last_minutes_and_either_side_can_win() {
    let data = data();
    let army = |n: usize| -> Vec<UnitSetup> {
        let kinds = [
            "unit_men_at_arms_foot",
            "unit_longbowmen",
            "unit_knights",
            "unit_urban_militia",
            "unit_crossbowmen",
        ];
        (0..n)
            .map(|i| unit(&data, kinds[i % kinds.len()]))
            .collect()
    };
    let (mut wins, mut total) = (0, 0.0);
    let runs = 8;
    for seed in 0..runs {
        let mut battle = setup(army(10), army(10), None);
        battle.river = seed % 2 == 0;
        let mut sim = BattleSim::new(battle, seed).unwrap();
        run_to_end(&mut sim);
        total += sim.elapsed();
        if sim.winner() == Some(SideId::Attacker) {
            wins += 1;
        }
    }
    let mean = total / f64::from(runs as u32);
    // EP9 (ADR 0056): a beaten army breaks before its last regiment flees;
    // the mean fell from about 6 to 5 minutes (rivers make it shorter).
    assert!(
        (240.0..=720.0).contains(&mean),
        "AI battles should last 4-12 minutes, mean {mean:.0} s"
    );
    assert!(
        (2..=6).contains(&wins),
        "neither side should win almost always: attacker {wins}/{runs}"
    );
}

#[test]
fn ai_battles_are_deterministic() {
    let data = data();
    let play = || {
        let mut sim = mirrored(&data, 42);
        run(&mut sim, 240.0);
        sim.units()
            .iter()
            .map(|u| (u.x, u.z, u.hp, u.morale))
            .collect::<Vec<_>>()
    };
    assert_eq!(play(), play());
}
