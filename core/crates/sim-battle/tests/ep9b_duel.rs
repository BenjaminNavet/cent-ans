//! EP9b (ADR 0056, § EP9b): the attacker's archery duel. Mirrored epic
//! armies (60 regiments of 120 men a side, both AIs) on bare flat ground
//! without stakes: before EP9b the attacker's line walked into the arrows
//! after ``ATTACKER_DUEL_LIMIT` (180 s) even while its shooters were winning the
//! duel, its militia in front broke and the defender won 10 battles out
//! of 10.
//!
//! `cargo test --release -p sim-battle --test ep9b_duel -- --ignored --nocapture survey`
//! (`EP9B_SEEDS=1..11` by default).

mod common;

use common::*;
use data_model::Ability;
use sim_battle::{BattleEnd, BattleSim, SideId};

const KINDS: [&str; 5] = [
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
    "unit_crossbowmen",
];

fn seeds() -> std::ops::Range<u64> {
    std::env::var("EP9B_SEEDS")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once("..")?;
            Some(a.parse().ok()?..b.parse().ok()?)
        })
        .unwrap_or(1..11)
}

/// The symmetric case: 60 mirrored regiments a side, flat bare ground, no
/// stakes, clear weather.
fn flat_battle(seed: u64) -> BattleSim {
    let data = data();
    let army: Vec<&str> = (0..60).map(|i| KINDS[i % KINDS.len()]).collect();
    let mut setup = setup(units(&data, &army), units(&data, &army), None);
    setup.village = Some(false);
    for unit in setup
        .attacker
        .units
        .iter_mut()
        .chain(setup.defender.units.iter_mut())
    {
        unit.soldiers = 120;
        unit.max_soldiers = 120;
        unit.abilities.retain(|a| *a != Ability::Stakes);
    }
    let mut sim = BattleSim::new(setup, seed).unwrap();
    sim.set_weather(sim_battle::Weather::Clear);
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    let field = sim.field_mut();
    field.forests.clear();
    field.forest_parts.clear();
    field.mud.clear();
    field.mud_parts.clear();
    field.pools.clear();
    field.obstacles.clear();
    field.river = None;
    field.bridges.clear();
    for h in field.heights.iter_mut() {
        *h = 0.0;
    }
    sim
}

/// Plays a battle to its end (30 min cap); returns winner, end and time.
fn play(sim: &mut BattleSim) -> (Option<SideId>, Option<BattleEnd>, f64) {
    while !sim.is_finished() && sim.elapsed() < 1800.0 {
        sim.step();
    }
    (sim.winner(), sim.end_kind(), sim.elapsed())
}

/// Attacker wins, all results, over the seeds.
fn flat_results() -> (usize, Vec<(Option<SideId>, Option<BattleEnd>, f64)>) {
    let results: Vec<_> = seeds()
        .map(|seed| play(&mut flat_battle(seed)))
        .collect();
    let wins = results
        .iter()
        .filter(|r| r.0 == Some(SideId::Attacker))
        .count();
    (wins, results)
}

/// Symmetric epic battle on flat ground without stakes: the outcome is
/// open (the attacker wins 3 to 7 battles out of 10), every battle ends.
#[test]
#[ignore = "EP9b: enabled once the attacker's duel is fixed"]
fn symmetric_flat_battle_is_open() {
    let (wins, results) = flat_results();
    for r in &results {
        assert!(r.0.is_some() && r.2 < 1800.0, "undecided: {r:?}");
    }
    assert!(
        (3..=7).contains(&wins),
        "attacker won {wins}/10: {results:?}"
    );
}

/// Survey (ignored): winners, ends and durations on the symmetric field.
#[test]
#[ignore = "survey"]
fn survey() {
    let (wins, results) = flat_results();
    for (seed, r) in seeds().zip(&results) {
        println!("seed {seed}: {:?} {:?} at {:.0} s", r.0, r.1, r.2);
    }
    println!("attacker wins {wins}/{}", results.len());
}
