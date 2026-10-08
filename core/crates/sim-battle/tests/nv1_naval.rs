//! NV1 tests: naval battles (ADR 0028) — shooting from the castles, height
//! of the sides, grapples and boarding, fire and fireships, rams, capture,
//! sinking, determinism, auto-resolve calibrated on the real-time battle.

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::load::load_entities;
use data_model::{NavalData, NavalRules, UnitType, UnitTypeId};
use sim_battle::naval::{
    auto_resolve, CrewSetup, NavalCommand, NavalCommandError, NavalEventKind, NavalScenario,
    NavalSetup, NavalSideSetup, NavalSim, ShipFate, ShipSetup, ShipStatus,
};
use sim_battle::{BattleSeason, SideId, UnitSetup};

fn root() -> PathBuf {
    data_model::test_support::data_dir()
}

struct Data {
    naval: NavalData,
    units: BTreeMap<UnitTypeId, UnitType>,
}

fn data() -> Data {
    let root = root();
    Data {
        naval: NavalData::load(&root).expect("naval data loads"),
        units: load_entities(&root.join("unit_types"), |u: &UnitType| &u.id).expect("unit types"),
    }
}

fn scenario(data: &Data, id: &str) -> NavalSetup {
    let path = root().join("naval/scenarios").join(format!("{id}.json"));
    let text = std::fs::read_to_string(path).expect("scenario file");
    let scenario: NavalScenario = serde_json::from_str(&text).expect("scenario parses");
    scenario
        .to_setup(&data.naval.ship_classes, &data.units, &data.naval.rules)
        .expect("scenario resolves")
}

type ShipSpec<'a> = (&'a str, &'a [(&'a str, u32)]);

/// A fleet of `ships` (class id, [(unit type, men)]) sharing one regiment
/// per unit type.
fn fleet(data: &Data, name: &str, ships: &[ShipSpec]) -> NavalSideSetup {
    let mut units: Vec<UnitSetup> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = Vec::new();
    for (n, (class, crew)) in ships.iter().enumerate() {
        let class = data.naval.ship_classes[*class].clone();
        let mut crew_setup = Vec::new();
        for (unit_type, men) in crew.iter() {
            let i = *index.entry((*unit_type).to_owned()).or_insert_with(|| {
                let ut = &data.units[*unit_type];
                units.push(UnitSetup::from_unit_type(ut, 0, 70, 0));
                units.len() - 1
            });
            units[i].soldiers += men;
            units[i].max_soldiers = units[i].max_soldiers.max(units[i].soldiers);
            crew_setup.push(CrewSetup { unit: i, men: *men });
        }
        out.push(ShipSetup {
            name: format!("{name} {n}"),
            class,
            crew: crew_setup,
            fireship: false,
            chain: None,
            fire_arrows: false,
            position: None,
            heading_deg: None,
            flagship: false,
        });
    }
    NavalSideSetup {
        faction: format!("fac_{}", name.to_lowercase()),
        faction_name: name.to_owned(),
        army: String::new(),
        admiral: String::new(),
        units,
        ships: out,
        hold: false,
    }
}

fn battle(
    data: &Data,
    attacker: NavalSideSetup,
    defender: NavalSideSetup,
    gauge: Option<SideId>,
) -> NavalSetup {
    NavalSetup {
        sea_zone: "sea_channel".to_owned(),
        place_name: "Manche".to_owned(),
        season: BattleSeason::Summer,
        rain: false,
        wind_to_deg: None,
        wind_strength: Some(0.6),
        gauge,
        shore: false,
        attacker,
        defender,
        player_side: None,
        rules: data.naval.rules.clone(),
    }
}

/// Two ships `gap` metres apart, bow to bow along x, nobody commanding.
fn duel(data: &Data, a: ShipSpec, d: ShipSpec, gap: f64) -> NavalSim {
    let mut attacker = fleet(data, "Angleterre", &[a]);
    let mut defender = fleet(data, "France", &[d]);
    attacker.ships[0].position = Some([-gap * 0.5, 0.0]);
    defender.ships[0].position = Some([gap * 0.5, 0.0]);
    defender.ships[0].heading_deg = Some(180.0);
    let mut setup = battle(data, attacker, defender, None);
    setup.wind_to_deg = Some(90.0);
    setup.player_side = Some(SideId::Attacker);
    let mut sim = NavalSim::new(setup, 11).unwrap();
    sim.set_ai(SideId::Defender, false);
    sim
}

const MAA: &str = "unit_men_at_arms_foot";
const BOWS: &str = "unit_longbowmen";
const XBOWS: &str = "unit_genoese_crossbowmen";
const MILITIA: &str = "unit_urban_militia";

#[test]
fn rules_file_matches_defaults() {
    let data = data();
    let mut rules = data.naval.rules.clone();
    rules.description = None;
    assert_eq!(rules, NavalRules::default());
    assert_eq!(data.naval.ship_classes.len(), 4);
    let england = data_model::FactionId::new("fac_england").unwrap();
    assert!(data.naval.fleet_of(&england).is_some());
}

/// Timeline of a scenario (run with `--ignored --nocapture`).
#[test]
#[ignore]
fn probe_timeline() {
    let data = data();
    let id = std::env::var("NV1_SCENARIO").unwrap_or_else(|_| "sluys".to_owned());
    let setup = scenario(&data, &id);
    let mut sim = NavalSim::new(setup, 1).unwrap();
    sim.set_ai(SideId::Attacker, true);
    while !sim.is_finished() {
        sim.advance(30.0);
        let line: Vec<String> = sim
            .ships
            .iter()
            .map(|s| {
                format!(
                    "{}{}:{:.0}/{:.0}{}{}",
                    if s.side == SideId::Attacker { "A" } else { "D" },
                    s.id,
                    s.fighting_men(),
                    s.morale,
                    match s.status {
                        ShipStatus::Afloat => "",
                        ShipStatus::Captured { .. } => "P",
                        ShipStatus::Sunk | ShipStatus::Sinking { .. } => "S",
                        ShipStatus::Abandoned => "X",
                        ShipStatus::Escaped => "E",
                    },
                    if s.fire > 0.0 {
                        format!("f{:.1}", s.fire)
                    } else {
                        String::new()
                    }
                )
            })
            .collect();
        println!("{:5.0}s {}", sim.elapsed, line.join(" "));
    }
    println!("{:?}", sim.outcome());
}

#[test]
fn same_seed_same_battle() {
    let data = data();
    let setup = scenario(&data, "sluys");
    let run = |seed| {
        let mut sim = NavalSim::new(setup.clone(), seed).unwrap();
        sim.set_ai(SideId::Attacker, true);
        let outcome = sim.run_to_end();
        (outcome, sim.log.len(), sim.ticks)
    };
    assert_eq!(run(5), run(5));
}

#[test]
fn high_cog_wins_the_boarding_against_a_low_galley() {
    let data = data();
    // Same men-at-arms on both; the galley's men must climb 1.6 m.
    let mut sim = duel(
        &data,
        ("ship_galley", &[(MAA, 50)]),
        ("ship_cog", &[(MAA, 50)]),
        40.0,
    );
    sim.issue(NavalCommand::Board { ship: 0, target: 1 })
        .unwrap();
    let outcome = sim.run_to_end();
    assert!(sim
        .log
        .iter()
        .any(|e| matches!(e.kind, NavalEventKind::Grapple { .. })));
    assert_eq!(outcome.winner, Some(SideId::Defender), "{outcome:?}");
    assert_eq!(outcome.attacker.ships[0].fate, ShipFate::Captured);
    assert_eq!(outcome.defender.prizes, vec!["ship_galley".to_owned()]);
    // The other way round, the cog's men jump down and win.
    let mut sim = duel(
        &data,
        ("ship_cog", &[(MAA, 50)]),
        ("ship_galley", &[(MAA, 50)]),
        40.0,
    );
    sim.issue(NavalCommand::Board { ship: 0, target: 1 })
        .unwrap();
    let outcome = sim.run_to_end();
    assert_eq!(outcome.winner, Some(SideId::Attacker), "{outcome:?}");
    assert_eq!(outcome.attacker.prizes, vec!["ship_galley".to_owned()]);
}

#[test]
fn archers_in_the_castles_outshoot_a_low_deck() {
    let data = data();
    let mut sim = duel(
        &data,
        ("ship_cog", &[(BOWS, 50)]),
        ("ship_galley", &[(BOWS, 50)]),
        120.0,
    );
    sim.issue(NavalCommand::Hold { ship: 0 }).unwrap();
    sim.advance(120.0);
    let shots = sim.take_shots();
    assert!(shots.iter().any(|s| s.shooter == 0) && shots.iter().any(|s| s.shooter == 1));
    let cog = sim.ships[0].soldiers();
    let galley = sim.ships[1].soldiers();
    assert!(galley < cog - 3.0, "cog {cog:.1} galley {galley:.1}");
}

#[test]
fn the_weather_gauge_favours_the_fleet_upwind() {
    let data = data();
    let archers: &[ShipSpec] = &[
        ("ship_cog", &[(BOWS, 50), (MAA, 20)]),
        ("ship_cog", &[(BOWS, 50), (MAA, 20)]),
        ("ship_cog", &[(BOWS, 50), (MAA, 20)]),
    ];
    let mut wins = [0, 0];
    for seed in 0..6 {
        for gauge in SideId::BOTH {
            let setup = battle(
                &data,
                fleet(&data, "Angleterre", archers),
                fleet(&data, "France", archers),
                Some(gauge),
            );
            let mut sim = NavalSim::new(setup, seed).unwrap();
            assert_eq!(sim.gauge(), Some(gauge));
            if sim.run_to_end().winner == Some(gauge) {
                wins[gauge.index()] += 1;
            }
        }
    }
    assert!(wins[0] + wins[1] >= 8, "gauge side wins {wins:?} of 12");
}

#[test]
fn fire_arrows_burn_a_ship_that_is_abandoned_and_sinks() {
    let data = data();
    let mut sim = duel(
        &data,
        ("ship_nef", &[(BOWS, 110)]),
        ("ship_cog", &[(MILITIA, 30)]),
        110.0,
    );
    sim.issue(NavalCommand::FireArrows {
        ship: 0,
        enabled: true,
    })
    .unwrap();
    sim.issue(NavalCommand::Hold { ship: 0 }).unwrap();
    let outcome = sim.run_to_end();
    assert!(sim
        .log
        .iter()
        .any(|e| e.kind == NavalEventKind::Ignite { ship: 1 }));
    assert!(sim.log.iter().any(|e| matches!(
        e.kind,
        NavalEventKind::Abandon { ship: 1 } | NavalEventKind::Sinking { ship: 1 }
    )));
    assert_eq!(outcome.winner, Some(SideId::Attacker));
    assert_eq!(
        outcome.defender.ships[0].fate,
        ShipFate::Sunk,
        "{outcome:?}"
    );
    assert!(outcome.defender.drowned > 0);
}

#[test]
fn a_fireship_sets_its_target_alight() {
    let data = data();
    let mut attacker = fleet(
        &data,
        "Castille",
        &[("ship_barge", &[(MAA, 4)]), ("ship_galley", &[(MAA, 20)])],
    );
    attacker.ships[0].fireship = true;
    attacker.ships[0].position = Some([-60.0, 0.0]);
    attacker.ships[1].position = Some([-700.0, 400.0]);
    let mut defender = fleet(&data, "Angleterre", &[("ship_cog", &[(MAA, 40)])]);
    defender.ships[0].position = Some([60.0, 0.0]);
    defender.hold = true;
    let mut setup = battle(&data, attacker, defender, None);
    setup.wind_to_deg = Some(0.0);
    let mut sim = NavalSim::new(setup, 3).unwrap();
    sim.advance(120.0);
    assert!(
        sim.log
            .iter()
            .any(|e| e.kind == NavalEventKind::Fireship { ship: 0, other: 2 }),
        "{:?}",
        sim.log
    );
    assert!(sim.ships[2].fire > 0.3 || !sim.ships[2].is_afloat());
    assert_eq!(sim.ships[0].status, ShipStatus::Abandoned);
}

#[test]
fn a_galley_spur_holes_a_low_hull_more_than_a_cog() {
    let data = data();
    let hull_loss = |target: &str| {
        let mut sim = duel(
            &data,
            ("ship_galley", &[(MAA, 30)]),
            (target, &[(MILITIA, 10)]),
            200.0,
        );
        sim.issue(NavalCommand::Ram { ship: 0, target: 1 }).unwrap();
        sim.advance(120.0);
        let damage = sim
            .log
            .iter()
            .find_map(|e| match e.kind {
                NavalEventKind::Ram { damage, .. } => Some(damage),
                _ => None,
            })
            .expect("the galley rams");
        damage
    };
    let barge = hull_loss("ship_barge");
    let cog = hull_loss("ship_cog");
    assert!(barge > cog * 2.0, "barge {barge:.1} cog {cog:.1}");
}

#[test]
fn commands_are_checked() {
    let data = data();
    let mut sim = duel(
        &data,
        ("ship_cog", &[(MAA, 30)]),
        ("ship_cog", &[(MAA, 30)]),
        300.0,
    );
    assert_eq!(
        sim.issue(NavalCommand::Ram { ship: 0, target: 1 }),
        Err(NavalCommandError::NoRam)
    );
    assert_eq!(
        sim.issue(NavalCommand::Shoot { ship: 0, target: 1 }),
        Err(NavalCommandError::NoShooters)
    );
    assert_eq!(
        sim.issue(NavalCommand::Board { ship: 0, target: 0 }),
        Err(NavalCommandError::BadTarget(0))
    );
    assert_eq!(
        sim.issue(NavalCommand::Hold { ship: 9 }),
        Err(NavalCommandError::UnknownShip(9))
    );
    let setup = scenario(&data, "sluys");
    let mut sluys = NavalSim::new(setup, 1).unwrap();
    let chained = sluys.ships.iter().find(|s| s.chain.is_some()).unwrap().id;
    assert_eq!(
        sluys.issue(NavalCommand::Move {
            ship: chained,
            x: 0.0,
            z: 0.0
        }),
        Err(NavalCommandError::Chained)
    );
}

#[test]
fn sluys_is_an_english_victory_and_the_genoese_get_away() {
    let data = data();
    let setup = scenario(&data, "sluys");
    assert!(setup.ship_count() >= 20);
    let mut escapes = 0;
    for seed in 1..=4 {
        let mut sim = NavalSim::new(setup.clone(), seed).unwrap();
        sim.set_ai(SideId::Attacker, true);
        assert_eq!(sim.gauge(), Some(SideId::Attacker));
        let outcome = sim.run_to_end();
        assert_eq!(
            outcome.winner,
            Some(SideId::Attacker),
            "seed {seed}: {outcome:?}"
        );
        let french = &outcome.defender;
        assert!(french.count(ShipFate::Captured) + french.count(ShipFate::Sunk) >= 8);
        assert!(outcome.attacker.prizes.len() >= 6);
        assert!(
            outcome.attacker.men_lost > 40,
            "the English pay for it: {}",
            outcome.attacker.men_lost
        );
        assert!(french.men_lost > outcome.attacker.men_lost * 3);
        escapes += french
            .ships
            .iter()
            .filter(|s| s.class == "ship_galley" && s.fate == ShipFate::Escaped)
            .count();
        // Losses per regiment never exceed the men embarked.
        for (side, result) in [
            (&setup.attacker, &outcome.attacker),
            (&setup.defender, french),
        ] {
            for (unit, lost) in side.units.iter().zip(&result.unit_losses) {
                assert!(*lost <= unit.soldiers);
            }
        }
    }
    assert!(escapes >= 2, "Barbavera's galleys escape: {escapes}");
}

#[test]
fn la_rochelle_the_galleys_burn_the_english_fleet() {
    let data = data();
    let setup = scenario(&data, "la_rochelle");
    let mut wins = 0;
    for seed in 1..=4 {
        let mut sim = NavalSim::new(setup.clone(), seed).unwrap();
        sim.set_ai(SideId::Attacker, true);
        let outcome = sim.run_to_end();
        if outcome.winner == Some(SideId::Attacker) {
            wins += 1;
        }
        assert!(sim
            .log
            .iter()
            .any(|e| matches!(e.kind, NavalEventKind::Ignite { .. })));
    }
    assert!(wins >= 3, "Castile wins {wins} of 4");
}

/// Reference fights for the calibration of the auto-resolve.
fn calibration_matrix(data: &Data) -> Vec<(String, NavalSetup)> {
    let cog_bows: ShipSpec = ("ship_cog", &[(BOWS, 50), (MAA, 20)]);
    let cog_maa: ShipSpec = ("ship_cog", &[(MAA, 60)]);
    let cog_militia: ShipSpec = ("ship_cog", &[(MILITIA, 70)]);
    let nef_mixed: ShipSpec = ("ship_nef", &[(BOWS, 50), (MAA, 60)]);
    let galley_xbows: ShipSpec = ("ship_galley", &[(XBOWS, 50)]);
    let galley_maa: ShipSpec = ("ship_galley", &[(XBOWS, 25), (MAA, 25)]);
    let barge_bows: ShipSpec = ("ship_barge", &[(BOWS, 30), (MAA, 10)]);
    let cases: Vec<(&str, Vec<ShipSpec>, Vec<ShipSpec>)> = vec![
        ("archers vs milice", vec![cog_bows; 4], vec![cog_militia; 4]),
        (
            "hommes d'armes vs milice",
            vec![cog_maa; 3],
            vec![cog_militia; 3],
        ),
        (
            "milice ×2 vs hommes d'armes",
            vec![cog_militia; 6],
            vec![cog_maa; 3],
        ),
        (
            "nefs vs cogues d'archers",
            vec![nef_mixed; 3],
            vec![cog_bows; 3],
        ),
        ("cogues vs galères", vec![cog_maa; 3], vec![galley_maa; 3]),
        (
            "galères ×2 vs cogues",
            vec![galley_maa; 6],
            vec![cog_militia; 3],
        ),
        (
            "galères d'arbalétriers vs barges",
            vec![galley_xbows; 3],
            vec![barge_bows; 3],
        ),
        ("barges vs cogues", vec![barge_bows; 4], vec![cog_maa; 2]),
        (
            "cogues d'archers vs galères d'arbalétriers",
            vec![cog_bows; 3],
            vec![galley_xbows; 3],
        ),
        (
            "petite flotte vs grande",
            vec![cog_bows; 2],
            vec![cog_bows; 5],
        ),
        ("milice vs archers", vec![cog_militia; 4], vec![cog_bows; 3]),
        (
            "nef seule vs deux barges",
            vec![nef_mixed],
            vec![barge_bows; 2],
        ),
    ];
    let mut out = Vec::new();
    for (name, a, d) in cases {
        for gauge in SideId::BOTH {
            let setup = battle(
                data,
                fleet(data, "Angleterre", &a),
                fleet(data, "France", &d),
                Some(gauge),
            );
            out.push((format!("{name} (au vent : {})", gauge.key()), setup));
        }
    }
    out.push(("l'Écluse".to_owned(), scenario(data, "sluys")));
    out.push(("La Rochelle".to_owned(), scenario(data, "la_rochelle")));
    out
}

/// Winner of the real-time battle over three seeds (majority), AI both sides.
fn reference_winner(setup: &NavalSetup) -> Option<SideId> {
    let mut count = [0, 0, 0];
    for seed in [3, 17, 29] {
        let mut setup = setup.clone();
        setup.player_side = None;
        let mut sim = NavalSim::new(setup, seed).unwrap();
        match sim.run_to_end().winner {
            Some(SideId::Attacker) => count[0] += 1,
            Some(SideId::Defender) => count[1] += 1,
            None => count[2] += 1,
        }
    }
    if count[0] >= 2 {
        Some(SideId::Attacker)
    } else if count[1] >= 2 {
        Some(SideId::Defender)
    } else {
        None
    }
}

#[test]
fn auto_resolve_agrees_with_the_real_time_battle() {
    let data = data();
    let matrix = calibration_matrix(&data);
    let mut agree = 0;
    let mut report = Vec::new();
    for (name, setup) in &matrix {
        let reference = reference_winner(setup);
        let mut auto = [0, 0, 0];
        for seed in [3, 17, 29] {
            match auto_resolve(setup, seed).winner {
                Some(SideId::Attacker) => auto[0] += 1,
                Some(SideId::Defender) => auto[1] += 1,
                None => auto[2] += 1,
            }
        }
        let auto_winner = if auto[0] >= 2 {
            Some(SideId::Attacker)
        } else if auto[1] >= 2 {
            Some(SideId::Defender)
        } else {
            None
        };
        if auto_winner == reference {
            agree += 1;
        }
        report.push(format!("{name}: 3D {reference:?} auto {auto_winner:?}"));
    }
    let share = agree as f64 / matrix.len() as f64;
    println!(
        "{}\naccord {agree}/{} ({:.0} %)",
        report.join("\n"),
        matrix.len(),
        share * 100.0
    );
    assert!(share >= 0.8, "{}", report.join("\n"));
}

#[test]
fn auto_resolve_is_deterministic_and_reports_prizes() {
    let data = data();
    let setup = scenario(&data, "sluys");
    let a = auto_resolve(&setup, 9);
    assert_eq!(a, auto_resolve(&setup, 9));
    assert!(a.auto);
    assert_eq!(a.winner, Some(SideId::Attacker));
    let captured = a.defender.count(ShipFate::Captured);
    assert_eq!(a.attacker.prizes.len(), captured);
    assert!(a.attacker.men_lost < a.defender.men_lost);
}
