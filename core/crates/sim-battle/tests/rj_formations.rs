//! RJ-a (ADR 0174): historical formations read from
//! `data/rules/unit_formations.json`, each allowed to some troops only, and
//! a progressive change of formation: the men walk from their old places to
//! their new ones over the reform duration, the regiment slower meanwhile;
//! deterministic.

mod common;

use common::*;
use sim_battle::replay::state_digest;
use sim_battle::{
    BattleSim, Command, CommandError, Formation, FormationRules, FormationShape, SideId, DT,
};

/// Men-at-arms on foot (0), pikemen (1), longbowmen (2), knights (3)
/// against militia far away (4).
fn lab_sim(seed: u64) -> BattleSim {
    let data = data();
    let mut sim = BattleSim::new(
        setup(
            units(
                data,
                &[
                    "unit_men_at_arms_foot",
                    "unit_flemish_pikemen",
                    "unit_longbowmen",
                    "unit_knights",
                ],
            ),
            units(data, &["unit_urban_militia"]),
            None,
        ),
        seed,
    )
    .unwrap();
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Attacker);
    place(&mut sim, 0, 300.0, 300.0, 0.0);
    place(&mut sim, 1, 400.0, 300.0, 0.0);
    place(&mut sim, 2, 500.0, 300.0, 0.0);
    place(&mut sim, 3, 600.0, 300.0, 0.0);
    place(&mut sim, 4, 1100.0, 900.0, 0.0);
    sim
}

fn form(sim: &mut BattleSim, id: u32, key: &str) -> Result<(), CommandError> {
    sim.apply_command(
        Command::Formation {
            units: vec![id],
            kind: Formation::of(key),
        },
        None,
    )
}

#[test]
fn the_bundled_formations_have_historical_names_and_tooltips() {
    let rules = FormationRules::bundled();
    rules.check().unwrap();
    assert!(rules.formations.len() >= 8, "{}", rules.formations.len());
    for key in [
        "line",
        "thin_line",
        "battle",
        "column",
        "square",
        "herse",
        "wedge",
        "conroi",
    ] {
        let f = Formation::of(key).def();
        assert!(!f.name.is_empty() && !f.short.is_empty(), "{key}");
        assert!(f.description.len() > 60, "{key}: tooltip too short");
        assert!(f.reform_s > 0.0, "{key}");
    }
    assert_eq!(Formation::default(), Formation::of("line"));
    assert_eq!(Formation::of("herse").shape(), FormationShape::Herse);
    // Keys in JSON, unknown keys refused.
    assert_eq!(
        serde_json::to_string(&Formation::of("battle")).unwrap(),
        "\"battle\""
    );
    let back: Formation = serde_json::from_str("\"square\"").unwrap();
    assert_eq!(back, Formation::of("square"));
    assert!(serde_json::from_str::<Formation>("\"phalanx\"").is_err());
}

#[test]
fn each_formation_is_allowed_to_its_troops_only() {
    let mut sim = lab_sim(1);
    // Herse: shooters only; wedge and conroi: horse only; battle and
    // schiltron: foot only.
    assert!(form(&mut sim, 0, "herse").is_err());
    assert!(form(&mut sim, 3, "battle").is_err());
    assert!(form(&mut sim, 3, "square").is_err());
    assert!(form(&mut sim, 0, "wedge").is_err());
    assert!(form(&mut sim, 2, "conroi").is_err());
    form(&mut sim, 2, "herse").unwrap();
    form(&mut sim, 0, "battle").unwrap();
    form(&mut sim, 1, "square").unwrap();
    form(&mut sim, 3, "conroi").unwrap();
    form(&mut sim, 0, "thin_line").unwrap();
    // The battle line is deeper than the thin line.
    let thin = sim.units()[0].ranks_files(sim.units()[0].soldiers()).0;
    form(&mut sim, 0, "battle").unwrap();
    let deep = sim.units()[0].ranks_files(sim.units()[0].soldiers()).0;
    assert!(deep > thin * 2, "{deep} vs {thin}");
}

#[test]
fn a_change_of_formation_takes_time_and_the_men_walk() {
    let mut sim = lab_sim(2);
    let before = sim.units()[0].soldier_positions();
    form(&mut sim, 0, "column").unwrap();
    let unit = &sim.units()[0];
    assert_eq!(unit.formation, Formation::of("column"));
    let reform = unit.reform.expect("a formation order is not instantaneous");
    assert!(reform.duration >= 3.0, "{}", reform.duration);
    // Right after the order, every man still stands in his old place.
    assert_eq!(unit.soldier_positions(), before);
    let target = {
        let mut done = unit.clone();
        done.reform = None;
        done.soldier_positions()
    };
    sim.step();
    let early = sim.units()[0].soldier_positions();
    let moved = |a: &[(f64, f64, f64)], b: &[(f64, f64, f64)]| {
        a.iter()
            .zip(b)
            .map(|(p, q)| (p.0 - q.0).hypot(p.1 - q.1))
            .fold(0.0, f64::max)
    };
    // One step in, the men have walked a little (at most a fast walk).
    let walked = moved(&early, &before);
    assert!(walked > 0.0 && walked < 3.0 * DT * 4.0, "{walked}");
    assert!(moved(&early, &target) > 1.0);
    assert!(sim.units()[0].reforming());
    run(&mut sim, reform.duration + DT);
    let unit = &sim.units()[0];
    assert!(unit.reform.is_none(), "formed once the duration is over");
    assert!(moved(&unit.soldier_positions(), &target) < 1e-9);
}

#[test]
fn ordering_back_the_formation_being_left_turns_the_men_round() {
    let mut sim = lab_sim(3);
    form(&mut sim, 0, "column").unwrap();
    run(&mut sim, 2.0);
    let halfway = sim.units()[0].soldier_positions();
    let done = sim.units()[0].reform.unwrap().progress();
    form(&mut sim, 0, "line").unwrap();
    let unit = &sim.units()[0];
    let reform = unit.reform.expect("walking back");
    assert!((reform.progress() - (1.0 - done)).abs() < 1e-9);
    // The same order twice keeps the walk under way.
    form(&mut sim, 0, "line").unwrap();
    assert_eq!(sim.units()[0].reform, Some(reform));
    // Nobody jumps when the order changes.
    let now = sim.units()[0].soldier_positions();
    let jump = now
        .iter()
        .zip(&halfway)
        .map(|(p, q)| (p.0 - q.0).hypot(p.1 - q.1))
        .fold(0.0, f64::max);
    assert!(jump < 4.0, "{jump}");
}

#[test]
fn a_reforming_regiment_is_slower_and_weaker() {
    let mut sim = lab_sim(4);
    let formed = sim.units()[0].clone();
    form(&mut sim, 0, "thin_line").unwrap();
    let reforming = &sim.units()[0];
    let rules = &FormationRules::bundled().reform;
    assert!(reforming.formation_speed() < formed.formation_speed());
    assert!((reforming.formation_speed() - formed.formation_speed() * rules.speed).abs() < 1e-9);
    assert!(reforming.melee_taken_factor() > formed.melee_taken_factor());
    assert!(reforming.morale_loss_factor() > formed.morale_loss_factor());
}

#[test]
fn bigger_and_greener_regiments_take_longer_to_reform() {
    let sim = lab_sim(5);
    let rules = FormationRules::bundled();
    let mut unit = sim.units()[0].clone();
    let column = Formation::of("column");
    let base = rules.reform_duration(&unit, column);
    unit.experience = 3;
    assert!(rules.reform_duration(&unit, column) < base);
    unit.experience = 0;
    unit.hp *= 3.0;
    assert!(rules.reform_duration(&unit, column) > base);
}

#[test]
fn formations_during_deployment_are_instant() {
    let data = data();
    let mut sim = BattleSim::new(
        setup(
            units(data, &["unit_men_at_arms_foot"]),
            units(data, &["unit_urban_militia"]),
            None,
        ),
        9,
    )
    .unwrap();
    assert!(sim.begin_deployment());
    form(&mut sim, 0, "column").unwrap();
    assert!(sim.units()[0].reform.is_none());
}

#[test]
fn reforming_is_deterministic() {
    let digest = |seed| {
        let mut sim = lab_sim(seed);
        form(&mut sim, 0, "battle").unwrap();
        form(&mut sim, 2, "herse").unwrap();
        form(&mut sim, 3, "wedge").unwrap();
        sim.apply_command(
            Command::Move {
                units: vec![3],
                x: 600.0,
                z: 500.0,
                run: false,
                facing: None,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
            },
            None,
        )
        .unwrap();
        run(&mut sim, 3.0);
        let mid = state_digest(&sim);
        run(&mut sim, 20.0);
        (mid, state_digest(&sim), sim.units()[3].soldier_positions())
    };
    assert_eq!(digest(11), digest(11));
}
