//! EP7: historical battle maps (ADR 0035): Crécy, Agincourt, Poitiers.
//!
//! Each map loads, lays its site (real relief, woods, water, decor) and its
//! historical deployment; with the AI on both sides the historical result
//! is the most likely one, never a certainty.
//!
//! `cargo test --release -p sim-battle --test ep7_historical -- --ignored --nocapture survey`
//! prints the survey (winner, end, duration, losses) over more seeds.

mod common;

use std::path::PathBuf;

use common::*;
use sim_battle::{BattleSim, HistoricalMap, SideId};

/// The historical maps shipped.
const MAPS: &[&str] = &["crecy"];

fn map(id: &str) -> HistoricalMap {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/battle_maps")
        .join(format!("{id}.json"));
    let text = std::fs::read_to_string(&path).expect("map file");
    HistoricalMap::from_json(&text).expect("map parses")
}

fn start(id: &str, seed: u64) -> (HistoricalMap, BattleSim) {
    let data = data();
    let map = map(id);
    let setup = map
        .battle_setup(
            &data.unit_types,
            data.battle_orders.values().cloned().collect(),
            Some(data.battle_standard_rules.clone()),
            None,
        )
        .expect("setup");
    let sim = map.start(setup, seed).expect("battle starts");
    (map, sim)
}

#[test]
fn crecy_loads_and_deploys() {
    let (map, sim) = start("crecy", 1);
    assert!(sim.is_historical());
    assert_eq!(sim.field().width, map.field.width_m);
    for u in sim.units() {
        assert!(u.present(), "every regiment deployed: {}", u.name);
        assert!(sim.field().inside(u.x, u.z));
    }
}

/// Result of one AI-vs-AI run.
struct Run {
    winner: Option<SideId>,
    end: String,
    seconds: f64,
    lost: [u32; 2],
    initial: [u32; 2],
}

fn play(id: &str, seed: u64, cap: f64) -> Run {
    let (_, mut sim) = start(id, seed);
    let initial = [
        sim.strength(SideId::Attacker),
        sim.strength(SideId::Defender),
    ];
    run(&mut sim, cap);
    let end = sim
        .outcome()
        .map_or("unfinished".to_owned(), |o| format!("{:?}", o.end));
    Run {
        winner: sim.winner(),
        end,
        seconds: sim.elapsed(),
        lost: [
            initial[0].saturating_sub(sim.strength(SideId::Attacker)),
            initial[1].saturating_sub(sim.strength(SideId::Defender)),
        ],
        initial,
    }
}

/// Plays `seeds` and returns how many the historical winner took.
fn survey(id: &str, seeds: std::ops::Range<u64>) -> usize {
    let winner = map(id).historical_winner;
    let mut wins = 0;
    for seed in seeds {
        let started = std::time::Instant::now();
        let r = play(id, seed, 2400.0);
        if r.winner == Some(winner) {
            wins += 1;
        }
        println!(
            "{id} seed {seed:2}: winner {:?} ({}) at {:5.0} s, lost {}/{} vs {}/{} [{:.1} s]",
            r.winner,
            r.end,
            r.seconds,
            r.lost[0],
            r.initial[0],
            r.lost[1],
            r.initial[1],
            started.elapsed().as_secs_f64()
        );
    }
    wins
}

/// Prints the course of a battle every 30 s and its journal
/// (`EP7_MAP=crecy EP7_SEED=1`).
#[test]
#[ignore]
fn trace() {
    let id = std::env::var("EP7_MAP").unwrap_or_else(|_| "crecy".to_owned());
    let seed = std::env::var("EP7_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let (_, mut sim) = start(&id, seed);
    for _ in 0..80 {
        run(&mut sim, 30.0);
        let mut line = format!("t={:4.0}", sim.elapsed());
        for side in SideId::BOTH {
            let units: Vec<_> = sim.units().iter().filter(|u| u.side == side).collect();
            let z = units.iter().map(|u| u.z).sum::<f64>() / units.len() as f64;
            let count =
                |f: &dyn Fn(&sim_battle::Unit) -> bool| units.iter().filter(|u| f(u)).count();
            line += &format!(
                " | {} z {:5.0} mov {} melee {} rout {} str {}",
                side.key(),
                z,
                count(&|u| u.destination.is_some() || u.target.is_some()),
                count(&|u| u.state == sim_battle::UnitState::Melee),
                count(&|u| u.state == sim_battle::UnitState::Routing),
                sim.strength(side)
            );
        }
        println!("{line}");
        if sim.is_finished() {
            break;
        }
    }
    for e in sim.events() {
        println!("{:5.0} {}", e.time, e.text_fr);
    }
}

#[test]
#[ignore]
fn survey_all() {
    for &id in MAPS {
        let wins = survey(id, 1..31);
        println!("{id}: historical winner {wins}/30");
    }
}

/// Crécy: with the AI on both sides the English win most of the time
/// (>= 70 % of the seeds), never always.
#[test]
fn crecy_is_most_often_an_english_victory() {
    let wins = (1..21)
        .filter(|&seed| play("crecy", seed, 2400.0).winner == Some(SideId::Defender))
        .count();
    assert!((14..20).contains(&wins), "English won {wins}/20");
}
