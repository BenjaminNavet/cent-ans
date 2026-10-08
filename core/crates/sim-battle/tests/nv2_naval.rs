//! NV2 tests: general boarding at l'Écluse — simultaneous boardings,
//! targets shared out among the attackers, boarding along the chained
//! lines (scenario formation data), all deterministic.

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::load::load_entities;
use data_model::{NavalData, UnitType, UnitTypeId};
use sim_battle::naval::{NavalEventKind, NavalScenario, NavalSetup, NavalSim};
use sim_battle::SideId;

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

/// Enemy pairs lashed together right now.
fn boardings(sim: &NavalSim) -> usize {
    sim.ships
        .iter()
        .filter(|s| s.side == SideId::Attacker && s.is_afloat())
        .map(|s| {
            s.grappled
                .iter()
                .filter(|&&g| {
                    let o = &sim.ships[g as usize];
                    o.side != s.side && o.is_afloat()
                })
                .count()
        })
        .sum()
}

/// Sluys timeline of boardings (run with `--ignored --nocapture`).
#[test]
#[ignore]
fn probe_boardings() {
    let data = data();
    let setup = scenario(&data, "sluys");
    for seed in 1..=3 {
        let run = general_boarding(&setup, seed);
        println!("seed {seed}: {run:?}");
    }
}

/// What the general boarding of a Sluys run looked like.
#[derive(Debug, PartialEq)]
struct Boarding {
    /// Time the English called the general boarding.
    assault: Option<f64>,
    /// Time of the first grapple.
    first_contact: Option<f64>,
    /// English ships that boarded within 90 s of the first contact.
    first_wave: usize,
    /// English ships that boarded during the battle.
    boarders: usize,
    /// Most enemy pairs lashed together at one moment.
    peak: usize,
    /// Boardings across a taken chained ship (no grapple thrown).
    across_chains: usize,
    french_assault: Option<f64>,
    english_lost: u32,
    winner: Option<SideId>,
}

fn general_boarding(setup: &NavalSetup, seed: u64) -> Boarding {
    let mut sim = NavalSim::new(setup.clone(), seed).unwrap();
    sim.set_ai(SideId::Attacker, true);
    let mut peak = 0;
    while !sim.is_finished() {
        sim.step();
        peak = peak.max(boardings(&sim));
    }
    let english = |id: u32| sim.ships[id as usize].side == SideId::Attacker;
    let mut grappled = std::collections::BTreeSet::new();
    let mut boarded: BTreeMap<u32, f64> = BTreeMap::new();
    let mut first_contact = None;
    let mut across_chains = 0;
    for event in &sim.log {
        match event.kind {
            NavalEventKind::Grapple { ship, other } => {
                grappled.insert((ship, other));
                if english(ship) {
                    first_contact.get_or_insert(event.time);
                }
            }
            NavalEventKind::Board { ship, other } if english(ship) => {
                boarded.entry(ship).or_insert(event.time);
                if !grappled.contains(&(ship, other)) {
                    across_chains += 1;
                }
            }
            _ => {}
        }
    }
    let first_wave = first_contact.map_or(0, |t0| {
        boarded.values().filter(|&&t| t <= t0 + 90.0).count()
    });
    let outcome = sim.outcome();
    Boarding {
        assault: sim.assault[SideId::Attacker.index()],
        first_contact,
        first_wave,
        boarders: boarded.len(),
        peak,
        across_chains,
        french_assault: sim.assault[SideId::Defender.index()],
        english_lost: outcome.attacker.men_lost,
        winner: outcome.winner,
    }
}

#[test]
fn sluys_chained_lines_come_from_the_scenario_formation() {
    let data = data();
    let setup = scenario(&data, "sluys");
    let french = &setup.defender.ships;
    let line = |chain: u32| -> Vec<[f64; 2]> {
        french
            .iter()
            .filter(|s| s.chain == Some(chain))
            .map(|s| s.position.expect("placed in its line"))
            .collect()
    };
    assert_eq!(
        line(1),
        vec![
            [250.0, -24.0],
            [250.0, -12.0],
            [250.0, 0.0],
            [250.0, 12.0],
            [250.0, 24.0]
        ]
    );
    let second = line(2);
    assert_eq!(second.len(), 6);
    assert!(second.iter().all(|p| (p[0] - 335.0).abs() < 1e-9));
    assert!((second[0][1] + 30.0).abs() < 1e-9 && (second[5][1] - 30.0).abs() < 1e-9);
    for ship in french.iter().filter(|s| s.chain.is_some()) {
        assert_eq!(ship.heading_deg, Some(180.0));
    }
    // Free ships keep their own place.
    let galley = french.iter().find(|s| s.chain.is_none()).unwrap();
    assert_eq!(galley.position, Some([300.0, -170.0]));
}

#[test]
fn sluys_is_a_general_boarding() {
    let data = data();
    let setup = scenario(&data, "sluys");
    let mut across = 0;
    for seed in 1..=3 {
        let run = general_boarding(&setup, seed);
        assert_eq!(run.winner, Some(SideId::Attacker), "seed {seed}: {run:?}");
        // The chained French wait: only the English call the assault, and
        // they call it before the first grapple.
        assert_eq!(run.french_assault, None);
        let assault = run.assault.expect("the English call the general boarding");
        let contact = run.first_contact.expect("the English board");
        assert!(assault < contact, "seed {seed}: {run:?}");
        assert!(run.first_wave >= 4, "seed {seed}: {run:?}");
        assert!(run.boarders >= 5, "seed {seed}: {run:?}");
        assert!(run.peak >= 3, "seed {seed}: {run:?}");
        across += run.across_chains;
    }
    assert!(across >= 3, "boarding along the chains: {across}");
}

#[test]
fn attackers_share_their_targets_out() {
    let data = data();
    let setup = scenario(&data, "sluys");
    let capacity = setup.rules.boarders_per_target;
    let mut sim = NavalSim::new(setup, 2).unwrap();
    sim.set_ai(SideId::Attacker, true);
    while sim.assault[SideId::Attacker.index()].is_none() {
        sim.step();
    }
    let mut targets = std::collections::BTreeSet::new();
    // From the call to the first grapple, no enemy ship draws more than
    // `boarders_per_target` boarders.
    while boardings(&sim) == 0 && !sim.is_finished() {
        sim.step();
        for enemy in sim.ships.iter().filter(|s| s.side == SideId::Defender) {
            let on_it = sim
                .ships
                .iter()
                .filter(|s| {
                    s.side == SideId::Attacker
                        && s.is_afloat()
                        && s.order.key() == "board"
                        && s.order.target() == Some(enemy.id)
                })
                .count() as u32;
            assert!(on_it <= capacity, "{} boarders on {}", on_it, enemy.name);
            if on_it > 0 {
                targets.insert(enemy.id);
            }
        }
    }
    assert!(targets.len() >= 4, "targets spread: {targets:?}");
}

#[test]
fn the_general_boarding_is_deterministic() {
    let data = data();
    let setup = scenario(&data, "sluys");
    assert_eq!(general_boarding(&setup, 7), general_boarding(&setup, 7));
    let run = |seed| {
        let mut sim = NavalSim::new(setup.clone(), seed).unwrap();
        sim.set_ai(SideId::Attacker, true);
        sim.run_to_end();
        sim.log
    };
    assert_eq!(run(4), run(4));
}
