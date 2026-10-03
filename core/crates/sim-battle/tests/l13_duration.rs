//! A6-L13 (ADR 0180): duration of field battles, AI against AI, without
//! rendering. Ignored survey: prints and writes (`L13_OUT`) one line per
//! battle `name<TAB>seed<TAB>winner<TAB>seconds<TAB>lost_a<TAB>lost_d`, then
//! the median duration per scenario.
//!
//! `L13_OUT=/path/out.tsv cargo test --release -p sim-battle --test l13_duration -- --ignored --nocapture survey`
//! (`L13_SEEDS=6` by default).

mod common;

use std::fmt::Write as _;
use std::path::PathBuf;

use common::*;
use data_model::{GameData, Terrain, UnitTypeId};
use serde::Deserialize;
use sim_battle::{BattleSeason, BattleSim, HistoricalMap, SideId, UnitSetup};

#[derive(Deserialize)]
struct Scenario {
    name: String,
    attacker: Vec<(String, usize)>,
    defender: Vec<(String, usize)>,
    terrain: Terrain,
    #[serde(default)]
    season: BattleSeason,
    #[serde(default)]
    river: bool,
}

#[derive(Deserialize)]
struct Fixture {
    scenarios: Vec<Scenario>,
}

fn comp(data: &GameData, c: &[(String, usize)]) -> Vec<UnitSetup> {
    let mut out = Vec::new();
    for (id, n) in c {
        let t = &data.unit_types[&UnitTypeId::new(id).unwrap()];
        for _ in 0..*n {
            out.push(UnitSetup::from_unit_type(t, t.soldiers, t.stats.morale, 0));
        }
    }
    out
}

/// Mixed army of about `men` soldiers (regiments of 60 to 120 men).
fn mixed(men: u32, archer: &str) -> Vec<(String, usize)> {
    let n = (men / 85) as usize;
    vec![
        ("unit_men_at_arms_foot".into(), n / 3),
        (archer.into(), n / 3),
        ("unit_urban_militia".into(), n / 6),
        ("unit_knights".into(), n - n / 3 - n / 3 - n / 6),
    ]
}

type Row = (String, u64, Option<SideId>, f64, u32, u32);

fn field(data: &GameData, s: &Scenario, seed: u64) -> Row {
    let mut setup = setup(comp(data, &s.attacker), comp(data, &s.defender), None);
    setup.terrain = s.terrain;
    setup.river = s.river;
    setup.season = s.season;
    setup.village = Some(false);
    let sim = BattleSim::new(setup, seed).unwrap();
    finish(&s.name, seed, sim)
}

fn finish(name: &str, seed: u64, mut sim: BattleSim) -> Row {
    // `L13_PACE=file.json`: tuning override of data/rules/battle_pace.json.
    if let Ok(path) = std::env::var("L13_PACE") {
        let rules = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        sim.set_pace(rules);
    }
    let a0 = sim.strength(SideId::Attacker);
    let d0 = sim.strength(SideId::Defender);
    while !sim.is_finished() && sim.elapsed() < 3600.0 {
        sim.step();
    }
    (
        name.to_owned(),
        seed,
        sim.winner(),
        sim.elapsed(),
        a0.saturating_sub(sim.strength(SideId::Attacker)),
        d0.saturating_sub(sim.strength(SideId::Defender)),
    )
}

fn historical(data: &GameData, id: &str, seed: u64) -> Row {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/battle_maps")
        .join(format!("{id}.json"));
    let map = HistoricalMap::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut setup = map
        .battle_setup(
            &data.unit_types,
            data.battle_orders.values().cloned().collect(),
            Some(data.battle_standard_rules.clone()),
            None,
        )
        .unwrap();
    setup.abilities = data.battle_abilities.values().cloned().collect();
    let sim = map.start(setup, seed).unwrap();
    finish(id, seed, sim)
}

#[test]
#[ignore = "survey"]
fn survey() {
    let data = data();
    let seeds: u64 = std::env::var("L13_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(6);
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../ai/tests/fixtures/auto_resolve_scenarios.json");
    let fixture: Fixture =
        serde_json::from_str(&std::fs::read_to_string(fixture_path).unwrap()).unwrap();
    let mut scenarios = fixture.scenarios;
    scenarios.push(Scenario {
        name: "campagne 600 contre 550 mixte".into(),
        attacker: mixed(600, "unit_crossbowmen"),
        defender: mixed(550, "unit_longbowmen"),
        terrain: Terrain::Plains,
        season: BattleSeason::Summer,
        river: false,
    });
    let rows: Vec<Row> = std::thread::scope(|scope| {
        let data = &data;
        let mut handles = Vec::new();
        for s in &scenarios {
            handles.push(scope.spawn(move || {
                (0..seeds).map(|k| field(data, s, k)).collect::<Vec<_>>()
            }));
        }
        for id in ["crecy", "azincourt", "poitiers"] {
            handles.push(scope.spawn(move || {
                (1..=seeds)
                    .map(|k| historical(data, id, k))
                    .collect::<Vec<_>>()
            }));
        }
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    let mut tsv = String::new();
    for r in &rows {
        let w = match r.2 {
            Some(SideId::Attacker) => "A",
            Some(SideId::Defender) => "D",
            None => "-",
        };
        let _ = writeln!(tsv, "{}\t{}\t{}\t{:.0}\t{}\t{}", r.0, r.1, w, r.3, r.4, r.5);
    }
    if let Ok(out) = std::env::var("L13_OUT") {
        std::fs::write(out, &tsv).unwrap();
    }
    let mut names: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    names.dedup();
    let mut medians = Vec::new();
    for n in names {
        let mut t: Vec<f64> = rows.iter().filter(|r| r.0 == n).map(|r| r.3).collect();
        t.sort_by(f64::total_cmp);
        let m = t[t.len() / 2];
        medians.push(m);
        println!("{m:>6.0} s  {n}");
    }
    medians.sort_by(f64::total_cmp);
    println!("MEDIAN OF MEDIANS {:.0} s", medians[medians.len() / 2]);
}
