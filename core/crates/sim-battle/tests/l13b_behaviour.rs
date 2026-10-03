//! A6-L13b (ADR 0180): behaviour probe of the field pace. Ignored survey: the
//! two behaviours the tuning tests defend, measured under a pace override
//! (`L13_PACE=file.json`) without recompiling:
//! - the active AI beats a passive side (`ai_relief.rs`, 32 battles per relief);
//! - the English on their ridge win the Crécy-like battle (`ep9_decisive.rs`).
//!
//! `cargo test --release -p sim-battle --test l13b_behaviour -- --ignored --nocapture`

mod common;

use common::*;
use data_model::Terrain;
use sim_battle::{BattleSim, GeneralSetup, SideId};

const ARMY: [&str; 6] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
];

fn with_pace(sim: &mut BattleSim) {
    if let Ok(path) = std::env::var("L13_PACE") {
        sim.set_pace(serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap());
    }
}

fn general(name: &str) -> GeneralSetup {
    GeneralSetup {
        character: format!("chr_{name}"),
        name: name.to_owned(),
        command: 5,
        unit_index: 0,
        morale_bonus: 0.0,
        charge_percent: 0.0,
        ranged_percent: 0.0,
        defense_percent: 0.0,
        sovereign: false,
    }
}

fn crecy(seed: u64) -> BattleSim {
    let data = data();
    let french = [
        "unit_knights",
        "unit_knights",
        "unit_knights",
        "unit_knights",
        "unit_crossbowmen",
        "unit_crossbowmen",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_urban_militia",
    ];
    let english = [
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_knights",
    ];
    let mut battle = setup(units(&data, &french), units(&data, &english), None);
    battle.village = Some(false);
    battle.attacker.general = Some(general("Philippe"));
    battle.defender.general = Some(general("Edouard"));
    battle.defender.general.as_mut().unwrap().unit_index = 4;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    // The ridge lies 20 m in front of the English line wherever the field
    // battle scale puts it (ADR 0180: wider gap).
    let crest = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender)
        .map(|u| u.z)
        .fold(f64::INFINITY, f64::min)
        - 20.0;
    let field = sim.field_mut();
    field.forests.clear();
    field.forest_parts.clear();
    field.mud.clear();
    field.mud_parts.clear();
    field.pools.clear();
    field.obstacles.clear();
    field.river = None;
    let (nx, res) = (field.nx, field.resolution);
    for (k, h) in field.heights.iter_mut().enumerate() {
        let z = (k / nx) as f64 * res;
        *h = 20.0 / (1.0 + (-(z - crest + 60.0) / 40.0).exp());
    }
    sim
}

#[test]
#[ignore = "survey"]
fn survey() {
    let data = data();
    let seeds: u64 = std::env::var("L13_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(16);
    let mut english = 0;
    let mut times = Vec::new();
    for seed in 0..seeds {
        let mut sim = crecy(seed);
        with_pace(&mut sim);
        while !sim.is_finished() && sim.elapsed() < 1800.0 {
            sim.step();
        }
        times.push(sim.elapsed());
        if std::env::var("L13_VERBOSE").is_ok() {
            println!("crecy seed {seed}: {:?} {:.0} s", sim.winner(), sim.elapsed());
            for e in sim.events().iter().filter(|e| !e.text_fr.contains("pieux")) {
                println!("   {:>4.0}s {}", e.time, e.text_fr);
            }
        }
        if sim.winner() == Some(SideId::Defender) {
            english += 1;
        }
    }
    times.sort_by(f64::total_cmp);
    println!(
        "crecy-like: English {english}/{seeds}, median {:.0} s",
        times[times.len() / 2]
    );
    for terrain in [
        Terrain::Plains,
        Terrain::Bocage,
        Terrain::Hills,
        Terrain::Mountains,
    ] {
        let (mut won, mut total, mut times) = (0, 0, Vec::new());
        for seed in 0..seeds {
            for passive in SideId::BOTH {
                let mut battle = setup(units(&data, &ARMY), units(&data, &ARMY), None);
                battle.terrain = terrain;
                let mut sim = BattleSim::new(battle, seed).unwrap();
                with_pace(&mut sim);
                sim.set_ai(passive, false);
                run_to_end(&mut sim);
                total += 1;
                times.push(sim.elapsed());
                won += usize::from(sim.winner() == Some(passive.other()));
            }
        }
        times.sort_by(f64::total_cmp);
        println!(
            "active AI vs passive on {}: {won}/{total}, median {:.0} s",
            terrain.key(),
            times[times.len() / 2]
        );
    }
}
