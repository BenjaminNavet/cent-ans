//! Battle simulation tests (spec `docs/design/m7-battles.md` § 1) on the real
//! unit types of `data/unit_types`.

use std::path::PathBuf;

use data_model::{GameData, Terrain, UnitTypeId};
use sim_battle::{
    BattleSeason, BattleSetup, BattleSim, Command, CommandError, Formation, GeneralSetup, SideId,
    SideSetup, UnitFate, UnitSetup, UnitState, Weather, DT,
};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn unit(data: &GameData, id: &str) -> UnitSetup {
    let unit_type = &data.unit_types[&UnitTypeId::new(id).unwrap()];
    UnitSetup::from_unit_type(unit_type, unit_type.soldiers, unit_type.stats.morale, 0)
}

fn side(faction: &str, name: &str, units: Vec<UnitSetup>) -> SideSetup {
    SideSetup {
        faction: faction.to_owned(),
        faction_name: name.to_owned(),
        army: String::new(),
        units,
        general: None,
        forced_march: false,
        entrenched: false,
        start_fatigue: 0.0,
    }
}

fn setup(attacker: Vec<UnitSetup>, defender: Vec<UnitSetup>) -> BattleSetup {
    BattleSetup {
        crossing: None,
        province: "prov_test".to_owned(),
        province_name: "Test".to_owned(),
        terrain: Terrain::Plains,
        river: false,
        season: BattleSeason::Summer,
        coastal: false,
        village: None,
        attacker: side("fac_france", "France", attacker),
        defender: side("fac_england", "Angleterre", defender),
        player_side: None,
        siege: None,
        siege_layout: None,
        orders: Vec::new(),
        abilities: Vec::new(),
        standards: None,
        decor_plan: None,
        opening: Default::default(),
    }
}

/// A sim with both AIs off, flat ground and clear weather: a laboratory.
fn lab(attacker: Vec<UnitSetup>, defender: Vec<UnitSetup>) -> BattleSim {
    let mut sim = BattleSim::new(setup(attacker, defender), 7).unwrap();
    sim.set_ai(SideId::Attacker, false);
    sim.set_ai(SideId::Defender, false);
    sim.set_weather(Weather::Clear);
    sim.set_end_conditions(false);
    sim
}

fn place(sim: &mut BattleSim, id: u32, x: f64, z: f64, facing: f64) {
    let unit = &mut sim.units_mut()[id as usize];
    unit.x = x;
    unit.z = z;
    unit.facing = facing;
}

fn run(sim: &mut BattleSim, seconds: f64) {
    let steps = (seconds / DT).round() as u64;
    for _ in 0..steps {
        sim.step();
    }
}

fn losses(sim: &BattleSim, id: u32) -> f64 {
    let u = &sim.units()[id as usize];
    f64::from(u.initial_soldiers) - u.hp
}

fn army(data: &GameData) -> Vec<UnitSetup> {
    vec![
        unit(data, "unit_men_at_arms_foot"),
        unit(data, "unit_men_at_arms_foot"),
        unit(data, "unit_longbowmen"),
        unit(data, "unit_longbowmen"),
        unit(data, "unit_knights"),
    ]
}

#[test]
fn same_setup_and_commands_give_the_same_battle() {
    let data = data();
    let play = || {
        let mut s = setup(army(&data), army(&data));
        s.player_side = Some(SideId::Attacker);
        let mut sim = BattleSim::new(s, 1337).unwrap();
        for tick in 0..3000u32 {
            if tick == 100 {
                sim.issue_command(Command::Move {
                    units: vec![0, 1],
                    x: 600.0,
                    z: 400.0,
                    run: false,
                    facing: Some(0.0),
                    queue: false,
                    width: None,
                    match_speed: false,
                    group_tag: None,
                })
                .unwrap();
            }
            if tick == 900 {
                sim.issue_command(Command::Attack {
                    units: vec![4],
                    target: 5,
                    run: true,
                    queue: false,
                })
                .unwrap();
            }
            sim.tick(DT);
        }
        let units: Vec<(u32, u64, u64, String)> = sim
            .units()
            .iter()
            .map(|u| {
                (
                    u.id,
                    u.hp.to_bits(),
                    u.x.to_bits(),
                    u.state.key().to_owned(),
                )
            })
            .collect();
        (units, sim.events().len(), sim.outcome())
    };
    assert_eq!(play(), play());
}

#[test]
fn shooting_out_of_range_has_no_effect() {
    let data = data();
    let mut sim = lab(
        vec![unit(&data, "unit_longbowmen")],
        vec![unit(&data, "unit_urban_militia")],
    );
    place(&mut sim, 0, 600.0, 200.0, 0.0);
    place(&mut sim, 1, 600.0, 200.0 + 300.0, std::f64::consts::PI);
    run(&mut sim, 60.0);
    assert_eq!(losses(&sim, 1), 0.0, "220 m bows cannot reach 300 m");
    assert_eq!(sim.units()[0].ammo, 48, "no arrow wasted");
    // In range, they shoot.
    place(&mut sim, 1, 600.0, 200.0 + 150.0, std::f64::consts::PI);
    run(&mut sim, 30.0);
    assert!(losses(&sim, 1) > 5.0);
    assert!(sim.units()[0].ammo < 48);
}

#[test]
fn rain_weakens_longbows() {
    let data = data();
    let shoot = |weather: Weather| {
        let mut sim = lab(
            vec![unit(&data, "unit_longbowmen")],
            vec![unit(&data, "unit_urban_militia")],
        );
        sim.set_weather(weather);
        place(&mut sim, 0, 600.0, 200.0, 0.0);
        place(&mut sim, 1, 600.0, 350.0, std::f64::consts::PI);
        // Steady targets: nobody routs out of range during the test.
        sim.units_mut()[1].morale = 100.0;
        sim.units_mut()[1].morale_cap = 100.0;
        run(&mut sim, 40.0);
        losses(&sim, 1)
    };
    let clear = shoot(Weather::Clear);
    let rain = shoot(Weather::Rain);
    assert!(clear > 0.0);
    assert!(
        (rain / clear - 0.6).abs() < 0.05,
        "rain: -40 % ({rain} vs {clear})"
    );
}

/// Knights charge a militia line from `angle` (0 = in front) and we count the
/// militia losses after 6 s of melee.
fn charge_losses(data: &GameData, from_flank: bool) -> f64 {
    let mut sim = lab(
        vec![unit(data, "unit_knights")],
        vec![unit(data, "unit_urban_militia")],
    );
    // The line faces -z (towards the attacker's edge); steady troops so that
    // the first shock does not rout them outright.
    place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
    sim.units_mut()[1].morale = 100.0;
    sim.units_mut()[1].morale_cap = 100.0;
    if from_flank {
        place(
            &mut sim,
            0,
            600.0 - 150.0,
            400.0,
            std::f64::consts::FRAC_PI_2,
        );
    } else {
        place(&mut sim, 0, 600.0, 400.0 - 150.0, 0.0);
    }
    sim.issue_command(Command::Attack {
        units: vec![0],
        target: 1,
        run: true,
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
    run(&mut sim, 6.0);
    losses(&sim, 1)
}

#[test]
fn flank_charge_is_deadlier() {
    let data = data();
    let front = charge_losses(&data, false);
    let flank = charge_losses(&data, true);
    assert!(front > 0.0);
    assert!(flank > front * 1.3, "flank {flank} vs front {front}");
}

#[test]
fn schiltron_stops_cavalry() {
    let data = data();
    let charge = |formation: Formation| {
        let mut sim = lab(
            vec![unit(&data, "unit_knights")],
            vec![unit(&data, "unit_flemish_pikemen")],
        );
        place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
        place(&mut sim, 0, 600.0, 250.0, 0.0);
        sim.apply_command(
            Command::Formation {
                units: vec![1],
                kind: formation,
            },
            None,
        )
        .unwrap();
        sim.issue_command(Command::Attack {
            units: vec![0],
            target: 1,
            run: true,
            queue: false,
        })
        .unwrap();
        run(&mut sim, 40.0);
        (losses(&sim, 1), losses(&sim, 0))
    };
    let (line_losses, _) = charge(Formation::Line);
    let (square_losses, knights_losses) = charge(Formation::Square);
    assert!(
        square_losses < line_losses * 0.5,
        "square {square_losses} vs line {line_losses}"
    );
    assert!(knights_losses > square_losses, "the pikes win the exchange");
    // Cavalry cannot form a schiltron, infantry cannot form a wedge.
    let mut sim = lab(
        vec![unit(&data, "unit_knights")],
        vec![unit(&data, "unit_flemish_pikemen")],
    );
    assert!(matches!(
        sim.issue_command(Command::Formation {
            units: vec![0],
            kind: Formation::Square
        }),
        Err(CommandError::InvalidFormation { .. })
    ));
    assert!(sim
        .issue_command(Command::Formation {
            units: vec![1],
            kind: Formation::Wedge
        })
        .is_err());
}

#[test]
fn running_tires_and_rest_recovers() {
    let data = data();
    let march = |run_flag: bool| {
        let mut sim = lab(
            vec![unit(&data, "unit_men_at_arms_foot")],
            vec![unit(&data, "unit_urban_militia")],
        );
        place(&mut sim, 1, 1100.0, 750.0, 0.0);
        place(&mut sim, 0, 100.0, 100.0, 0.0);
        sim.issue_command(Command::Move {
            units: vec![0],
            x: 100.0,
            z: 700.0,
            run: run_flag,
            facing: None,
            queue: false,
            width: None,
            match_speed: false,
            group_tag: None,
        })
        .unwrap();
        run(&mut sim, 60.0);
        sim
    };
    let walked = march(false);
    let mut ran = march(true);
    let walk_fatigue = walked.units()[0].fatigue;
    let run_fatigue = ran.units()[0].fatigue;
    assert!(
        run_fatigue > walk_fatigue * 3.0,
        "{run_fatigue} vs {walk_fatigue}"
    );
    assert!(ran.units()[0].z > walked.units()[0].z, "running is faster");
    ran.issue_command(Command::Halt { units: vec![0] }).unwrap();
    run(&mut ran, 10.0);
    assert!(ran.units()[0].fatigue < run_fatigue, "rest recovers");
}

#[test]
fn rout_then_rally_far_from_the_enemy() {
    let data = data();
    let mut sim = lab(
        vec![unit(&data, "unit_men_at_arms_foot")],
        vec![unit(&data, "unit_urban_militia")],
    );
    place(&mut sim, 0, 600.0, 400.0, 0.0);
    place(&mut sim, 1, 600.0, 700.0, std::f64::consts::PI);
    sim.units_mut()[0].morale = 10.0;
    sim.step();
    assert_eq!(sim.units()[0].state, UnitState::Routing);
    assert!(
        sim.issue_command(Command::Halt { units: vec![0] }).is_err(),
        "a routing unit ignores orders"
    );
    let mut rallied = false;
    for _ in 0..1200 {
        sim.step();
        if sim.units()[0].state == UnitState::Rallied {
            rallied = true;
            break;
        }
    }
    assert!(rallied, "morale {}", sim.units()[0].morale);
    assert!(sim.units()[0].morale > 40.0);
    assert!(sim
        .events()
        .iter()
        .any(|e| e.text_fr.contains("se rallient")));
}

#[test]
fn battle_ends_with_a_winner_and_an_outcome() {
    let data = data();
    let mut sim = BattleSim::new(setup(army(&data), army(&data)), 3).unwrap();
    let mut steps = 0;
    while !sim.is_finished() && steps < 36_000 {
        sim.step();
        steps += 1;
    }
    assert!(sim.is_finished());
    let outcome = sim.outcome().expect("finished battle has an outcome");
    assert!(outcome.duration <= 3600.0 + 1e-6);
    assert!(outcome.attacker.total_losses + outcome.defender.total_losses > 0);
    let loser = outcome.side(outcome.winner.other());
    assert!(loser.routed);
    assert_eq!(loser.morale_delta, -20);
    assert!(sim.issue_command(Command::Halt { units: vec![0] }).is_err());
}

#[test]
fn battle_times_out_in_favour_of_the_defender() {
    let data = data();
    let mut sim = lab(
        vec![unit(&data, "unit_urban_militia")],
        vec![unit(&data, "unit_urban_militia")],
    );
    sim.set_end_conditions(true);
    for _ in 0..36_001 {
        sim.step();
    }
    assert!(sim.is_finished());
    assert_eq!(sim.winner(), Some(SideId::Defender));
}

#[test]
fn general_death_shakes_the_army() {
    let data = data();
    let mut s = setup(
        vec![
            unit(&data, "unit_knights"),
            unit(&data, "unit_men_at_arms_foot"),
        ],
        vec![unit(&data, "unit_men_at_arms_foot")],
    );
    s.attacker.general = Some(GeneralSetup {
        character: "chr_test".to_owned(),
        name: "Le connétable".to_owned(),
        command: 8,
        unit_index: 0,
        morale_bonus: 0.0,
        charge_percent: 0.0,
        ranged_percent: 0.0,
        defense_percent: 0.0,
        sovereign: false,
    });
    let mut sim = BattleSim::new(s, 5).unwrap();
    sim.set_ai(SideId::Attacker, false);
    sim.set_ai(SideId::Defender, false);
    assert!(sim.units()[0].is_general);
    // The general's knights are nearly wiped out, in contact with the enemy.
    place(&mut sim, 2, 600.0, 400.0, std::f64::consts::PI);
    place(&mut sim, 0, 600.0, 393.0, 0.0);
    place(&mut sim, 1, 200.0, 100.0, 0.0);
    sim.units_mut()[0].hp = 0.5;
    let mut before = sim.units()[1].morale;
    for _ in 0..600 {
        before = sim.units()[1].morale;
        sim.step();
        if !sim.general_alive(SideId::Attacker) {
            break;
        }
    }
    assert!(!sim.general_alive(SideId::Attacker));
    let after = sim.units()[1].morale;
    assert!(before - after >= 24.0, "{before} -> {after}");
    assert!(sim.events().iter().any(|e| e.text_fr.contains("est tombé")));
    // Settle the battle: the attacker withdraws.
    sim.issue_command(Command::Withdraw { units: vec![1] })
        .unwrap();
    run(&mut sim, 200.0);
    let outcome = sim.outcome().expect("finished");
    assert!(outcome.attacker.general_killed);
    assert!(!outcome.defender.general_killed);
}

#[test]
fn losses_convert_back_to_campaign_units() {
    let data = data();
    let mut empty = unit(&data, "unit_longbowmen");
    empty.soldiers = 0;
    let mut sim = lab(
        vec![
            unit(&data, "unit_men_at_arms_foot"),
            empty,
            unit(&data, "unit_knights"),
        ],
        vec![unit(&data, "unit_urban_militia")],
    );
    // The empty unit is not deployed: ids 0 (men-at-arms), 1 (knights), 2 (militia).
    assert_eq!(sim.units().len(), 3);
    assert_eq!(sim.units()[1].setup_index, 2);
    place(&mut sim, 2, 600.0, 400.0, std::f64::consts::PI);
    place(&mut sim, 0, 600.0, 390.0, 0.0);
    run(&mut sim, 20.0);
    sim.set_end_conditions(true);
    // The knights withdraw intact, the men-at-arms keep fighting.
    sim.issue_command(Command::Withdraw { units: vec![1] })
        .unwrap();
    let mut steps = 0;
    while !sim.is_finished() && steps < 36_000 {
        sim.step();
        steps += 1;
    }
    let outcome = sim.outcome().unwrap();
    assert_eq!(outcome.attacker.losses.len(), 3);
    assert_eq!(outcome.attacker.losses[1], 0, "absent unit lost nothing");
    assert_eq!(outcome.attacker.losses[2], 0, "withdrawn knights saved");
    let men_at_arms = &sim.units()[0];
    assert_eq!(
        outcome.attacker.losses[0],
        men_at_arms.initial_soldiers - men_at_arms.soldiers()
    );
    assert_eq!(
        outcome.attacker.total_losses,
        outcome.attacker.losses.iter().sum::<u32>()
    );
    assert_eq!(outcome.defender.losses.len(), 1);
    // JSON round trip (the format crossing the GDExtension bridge).
    let json = serde_json::to_string(&outcome).unwrap();
    let back: sim_battle::BattleOutcome = serde_json::from_str(&json).unwrap();
    assert_eq!(back, outcome);
}

#[test]
fn commands_are_validated() {
    let data = data();
    let mut s = setup(army(&data), army(&data));
    s.player_side = Some(SideId::Attacker);
    let mut sim = BattleSim::new(s, 1).unwrap();
    assert_eq!(
        sim.issue_command(Command::Halt { units: vec![5] }),
        Err(CommandError::NotYours(5))
    );
    assert_eq!(
        sim.issue_command(Command::Halt { units: vec![] }),
        Err(CommandError::NoUnits)
    );
    assert_eq!(
        sim.issue_command(Command::Attack {
            units: vec![0],
            target: 1,
            run: true,
            queue: false
        }),
        Err(CommandError::FriendlyTarget(1))
    );
    assert_eq!(
        sim.issue_command(Command::FireAtWill {
            units: vec![0],
            enabled: false
        }),
        Err(CommandError::NoMissile(0))
    );
    assert_eq!(
        sim.issue_command(Command::Move {
            units: vec![0],
            x: -10.0,
            z: 5.0,
            run: false,
            facing: None,
            queue: false,
            width: None,
            match_speed: false,
            group_tag: None,
        }),
        Err(CommandError::OutsideField)
    );
    assert!(sim
        .issue_command(Command::FireAtWill {
            units: vec![2, 3],
            enabled: false
        })
        .is_ok());
    // Commands deserialize from the bridge's JSON form.
    let command: Command =
        serde_json::from_str(r#"{"type": "move", "units": [0], "x": 600, "z": 300, "run": true}"#)
            .unwrap();
    assert!(sim.issue_command(command).is_ok());
}

#[test]
fn stakes_break_a_frontal_cavalry_charge() {
    let data = data();
    let charge = |stakes: bool| {
        let mut sim = lab(
            vec![unit(&data, "unit_knights")],
            vec![unit(&data, "unit_longbowmen")],
        );
        place(&mut sim, 1, 600.0, 400.0, std::f64::consts::PI);
        place(&mut sim, 0, 600.0, 250.0, 0.0);
        sim.units_mut()[1].ammo = 0;
        if !stakes {
            // Same archers without their stakes.
            sim.units_mut()[1]
                .abilities
                .retain(|a| *a != data_model::Ability::Stakes);
        }
        if stakes {
            run(&mut sim, 16.0);
            assert!(sim.units()[1].stakes_planted);
        }
        sim.issue_command(Command::Attack {
            units: vec![0],
            target: 1,
            run: true,
            queue: false,
        })
        .unwrap();
        run(&mut sim, 30.0);
        (losses(&sim, 0), losses(&sim, 1))
    };
    let (knights_plain, archers_plain) = charge(false);
    let (knights_stakes, archers_stakes) = charge(true);
    assert!(
        knights_stakes > knights_plain,
        "knights {knights_stakes} vs {knights_plain}"
    );
    assert!(
        archers_stakes < archers_plain,
        "archers {archers_stakes} vs {archers_plain}"
    );
}

#[test]
fn full_ai_battle_on_a_river_field_finishes() {
    let data = data();
    let mut s = setup(army(&data), army(&data));
    s.river = true;
    s.terrain = Terrain::Hills;
    s.season = BattleSeason::Winter;
    let mut sim = BattleSim::new(s, 99).unwrap();
    assert!(sim.field().river.is_some());
    assert_eq!(sim.field().heights.len(), sim.field().nx * sim.field().nz);
    let mut steps = 0;
    while !sim.is_finished() && steps < 36_000 {
        sim.tick(1.0);
        steps += 10;
    }
    assert!(sim.is_finished());
    let transforms = sim.soldier_transforms(SideId::Attacker, None);
    let living: u32 = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker && u.present())
        .map(|u| u.soldiers())
        .sum();
    assert_eq!(transforms.len() as u32, living);
}

#[test]
fn general_retreat_reports_withdrawn_regiments() {
    let data = data();
    let mut sim = lab(
        vec![unit(&data, "unit_knights"), unit(&data, "unit_longbowmen")],
        vec![unit(&data, "unit_urban_militia")],
    );
    sim.set_end_conditions(true);
    run(&mut sim, 1.0);
    // Q2: the whole attacker army sounds the retreat; the battle ends at once.
    sim.issue_command(Command::Withdraw { units: vec![0, 1] })
        .unwrap();
    run(&mut sim, 1.0);
    assert!(sim.is_finished());
    assert_eq!(sim.units()[0].fate(), UnitFate::Withdrawn);
    assert_eq!(sim.units()[1].fate(), UnitFate::Withdrawn);
    assert_eq!(sim.units()[2].fate(), UnitFate::Held);
    let outcome = sim.outcome().unwrap();
    assert_eq!(outcome.winner, SideId::Defender);
    assert!(outcome.attacker.withdrew);
    assert!(!outcome.defender.withdrew);
}

#[test]
fn unit_fate_follows_losses_and_rout() {
    let data = data();
    let mut sim = lab(
        vec![unit(&data, "unit_knights")],
        vec![unit(&data, "unit_urban_militia")],
    );
    assert_eq!(sim.units()[0].fate(), UnitFate::Held);
    sim.units_mut()[0].state = UnitState::Routing;
    assert_eq!(sim.units()[0].fate(), UnitFate::Routed);
    sim.units_mut()[1].hp = 0.0;
    assert_eq!(sim.units()[1].fate(), UnitFate::Destroyed);
}
