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
    let _ = SideId::Attacker;
}
