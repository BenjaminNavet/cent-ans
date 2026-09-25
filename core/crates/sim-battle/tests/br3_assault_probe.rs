//! BR3 probe: siege assaults with both sides under AI, generic town and two
//! landmark towns (Paris, Rouen), over several seeds. Prints the attacker's
//! win rate, the median battle length, the losses and the houses burnt, to
//! compare the town layouts before and after densification.
//!
//! `cargo test -p sim-battle --release --test br3_assault_probe -- --ignored --nocapture`
//! (`SEEDS=n`, `LIMIT_S=s` override the defaults).

mod common;

use common::*;
use data_model::GameData;
use sim_battle::{BattleSim, SideId, SiegeLayout, SiegeSetup, UnitSetup};

const BESIEGERS: [&str; 11] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_trebuchet",
    "unit_mangonel",
    "unit_siege_tower",
];
const GARRISON: [&str; 6] = [
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_crossbowmen",
    "unit_crossbowmen",
    "unit_men_at_arms_foot",
    "unit_welsh_spearmen",
];

struct Outcome {
    attacker_won: bool,
    ended: f64,
    attacker_lost: f64,
    defender_lost: f64,
    burnt: usize,
    houses: usize,
}

fn assault(data: &GameData, town: Option<&str>, seed: u64, limit_s: f64) -> Outcome {
    let attackers: Vec<UnitSetup> = units(data, &BESIEGERS);
    let mut setup = setup(
        attackers,
        units(data, &GARRISON),
        Some(SiegeSetup {
            fortification: 2,
            breach: 40,
        }),
    );
    setup.siege_layout =
        town.map(|id| SiegeLayout::from_landmark(&data.landmarks[id]).expect("siege.battle"));
    let mut sim = BattleSim::new(setup, seed).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    if sim.is_deploying() {
        sim.start_battle().expect("start");
    }
    while !sim.is_finished() && sim.elapsed() < limit_s {
        sim.step();
    }
    let lost = |side: SideId| {
        sim.units()
            .iter()
            .filter(|u| u.side == side && !u.synthetic)
            .map(|u| f64::from(u.initial_soldiers) - u.hp.max(0.0))
            .sum::<f64>()
    };
    let works = sim.siege().unwrap();
    Outcome {
        attacker_won: sim.winner() == Some(SideId::Attacker),
        ended: sim.elapsed(),
        attacker_lost: lost(SideId::Attacker),
        defender_lost: lost(SideId::Defender),
        burnt: works.houses.iter().filter(|h| h.fire.burnt()).count(),
        houses: works.houses.iter().filter(|h| !h.suburb).count(),
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

#[test]
#[ignore = "probe: prints assault outcomes of the generic and landmark towns"]
fn probe_town_assaults() {
    let data = data();
    let seeds: u64 = std::env::var("SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let limit: f64 = std::env::var("LIMIT_S")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800.0);
    println!("| Ville | Maisons | Victoires assaillant | Durée médiane (s) | Pertes assaillant | Pertes garnison | Maisons brûlées (moy.) |");
    println!("|---|---|---|---|---|---|---|");
    for town in [None, Some("paris"), Some("rouen")] {
        let runs: Vec<Outcome> = (1..=seeds)
            .map(|s| assault(&data, town, s, limit))
            .collect();
        let n = runs.len() as f64;
        let wins = runs.iter().filter(|r| r.attacker_won).count();
        println!(
            "| {} | {} | {}/{} | {:.0} | {:.0} | {:.0} | {:.1} |",
            town.unwrap_or("générique"),
            runs[0].houses,
            wins,
            runs.len(),
            median(runs.iter().map(|r| r.ended).collect()),
            runs.iter().map(|r| r.attacker_lost).sum::<f64>() / n,
            runs.iter().map(|r| r.defender_lost).sum::<f64>() / n,
            runs.iter().map(|r| r.burnt as f64).sum::<f64>() / n,
        );
    }
}

/// Houses caught by the fire 10 minutes after the house nearest the square
/// is set alight (lab: no AI, clear weather).
fn fire_spread(data: &GameData, town: Option<&str>, seed: u64) -> (usize, usize) {
    let mut setup = setup(
        units(data, &["unit_urban_militia"]),
        units(data, &["unit_urban_militia"]),
        Some(SiegeSetup {
            fortification: 2,
            breach: 0,
        }),
    );
    setup.siege_layout =
        town.map(|id| SiegeLayout::from_landmark(&data.landmarks[id]).expect("siege.battle"));
    let mut sim = BattleSim::new(setup, seed).unwrap();
    lab(&mut sim);
    // Tuning sweeps: `FIRE_CHANCE`, `FIRE_REACH` override the spread.
    let mut rules = sim.fire_rules().unwrap().clone();
    if let Some(v) = std::env::var("FIRE_CHANCE")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        rules.spread.chance_per_period = v;
    }
    if let Some(v) = std::env::var("FIRE_REACH")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        rules.spread.edge_distance_m = v;
    }
    sim.set_fire_rules(Some(rules));
    let works = sim.siege().unwrap();
    let (cx, cz) = works.center;
    let first = (0..works.houses.len())
        .filter(|&i| !works.houses[i].suburb)
        .min_by(|&a, &b| {
            let d = |i: usize| (works.houses[i].x - cx).hypot(works.houses[i].z - cz);
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    let total = works.houses.iter().filter(|h| !h.suburb).count();
    sim.ignite_house(first);
    run(&mut sim, 600.0);
    let caught = sim
        .siege()
        .unwrap()
        .houses
        .iter()
        .filter(|h| !h.suburb && (h.fire.burning() || h.fire.burnt()))
        .count();
    (caught, total)
}

#[test]
#[ignore = "probe: prints how far a fire spreads in each town"]
fn probe_fire_spread() {
    let data = data();
    println!("| Ville | Maisons | Touchées après 10 min (moy.) | Part |");
    println!("|---|---|---|---|");
    for town in [None, Some("paris"), Some("rouen")] {
        let runs: Vec<(usize, usize)> = (1..=10).map(|s| fire_spread(&data, town, s)).collect();
        let mean = runs.iter().map(|r| r.0 as f64).sum::<f64>() / runs.len() as f64;
        println!(
            "| {} | {} | {:.1} | {:.0} % |",
            town.unwrap_or("générique"),
            runs[0].1,
            mean,
            100.0 * mean / runs[0].1 as f64
        );
    }
}

/// Cost of the figure poses (BR3 push-out) in a crowded siege at 2.5
/// figures per soldier, against the bare layout of the figures.
#[test]
#[ignore = "probe: prints the cost of the figure poses"]
fn probe_figure_cost() {
    let data = data();
    let mut attackers = Vec::new();
    let mut defenders = Vec::new();
    for _ in 0..4 {
        attackers.extend(units(&data, &BESIEGERS[..8]));
        defenders.extend(units(&data, &GARRISON));
    }
    let s = setup(
        attackers,
        defenders,
        Some(SiegeSetup {
            fortification: 2,
            breach: 60,
        }),
    );
    let mut sim = BattleSim::new(s, 3).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    if sim.is_deploying() {
        sim.start_battle().expect("start");
    }
    run(&mut sim, 240.0);
    let scale = 2.5;
    let figures: usize = sim
        .units()
        .iter()
        .map(|u| sim.soldier_poses(u, scale).len())
        .sum();
    let frames = 50;
    let t = std::time::Instant::now();
    for _ in 0..frames {
        for u in sim.units() {
            std::hint::black_box(sim.soldier_poses(u, scale));
        }
    }
    let with = t.elapsed().as_secs_f64() * 1e6 / f64::from(frames);
    let t = std::time::Instant::now();
    for _ in 0..frames {
        for u in sim.units() {
            std::hint::black_box(u.figure_positions(scale));
        }
    }
    let raw = t.elapsed().as_secs_f64() * 1e6 / f64::from(frames);
    println!(
        "{} regiments, {figures} figures: poses {with:.0} µs/frame, bare layout {raw:.0} µs/frame",
        sim.units().len()
    );
}
