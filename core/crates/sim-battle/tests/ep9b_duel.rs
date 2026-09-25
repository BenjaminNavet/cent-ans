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
    flat_battle_with(seed, &KINDS)
}

/// The same field, the attacker's army cycling through `attacker_kinds`
/// (the defender keeps [`KINDS`]).
fn flat_battle_with(seed: u64, attacker_kinds: &[&str]) -> BattleSim {
    let data = data();
    let army: Vec<&str> = (0..60).map(|i| KINDS[i % KINDS.len()]).collect();
    let attackers: Vec<&str> = (0..60)
        .map(|i| attacker_kinds[i % attacker_kinds.len()])
        .collect();
    let mut setup = setup(units(&data, &attackers), units(&data, &army), None);
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
    let results: Vec<_> = seeds().map(|seed| play(&mut flat_battle(seed))).collect();
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

/// A small battle on bare flat ground, both AIs.
fn small_battle(attacker: &[&str], defender: &[&str], seed: u64) -> BattleSim {
    let data = data();
    let mut setup = setup(units(&data, attacker), units(&data, defender), None);
    setup.village = Some(false);
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

/// Probe (ignored): duel state of a few small battles over time.
#[test]
#[ignore = "probe"]
fn probe_duel() {
    let cases: [(&[&str], &[&str]); 3] = [
        (
            &[
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_knights",
            ],
            &[
                "unit_crossbowmen",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_urban_militia",
                "unit_urban_militia",
                "unit_knights",
            ],
        ),
        (
            &[
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_urban_militia",
                "unit_knights",
            ],
            &[
                "unit_crossbowmen",
                "unit_crossbowmen",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_urban_militia",
                "unit_urban_militia",
                "unit_knights",
            ],
        ),
        (
            &[
                "unit_longbowmen",
                "unit_longbowmen",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_knights",
            ],
            &[
                "unit_crossbowmen",
                "unit_men_at_arms_foot",
                "unit_men_at_arms_foot",
                "unit_urban_militia",
                "unit_knights",
            ],
        ),
    ];
    let passive = std::env::var("EP9B_PASSIVE").is_ok();
    let limited = std::env::var("EP9B_LIMITED").is_ok();
    for (k, (att, def)) in cases.iter().enumerate() {
        for seed in 1..4 {
            let mut sim = small_battle(att, def, seed);
            if passive {
                sim.set_ai(SideId::Defender, false);
            }
            if limited {
                let mut rules = sim.duel_rules().clone();
                rules.winning_duel_max_seconds = rules.duel_limit_seconds;
                sim.set_duel_rules(rules);
            }
            let window = sim.duel_rules().window_seconds;
            let mut next = 150.0;
            let mut first_melee = None;
            let mut line = String::new();
            while !sim.is_finished() && sim.elapsed() < 1800.0 {
                sim.step();
                if first_melee.is_none()
                    && sim
                        .units()
                        .iter()
                        .any(|u| u.state == sim_battle::UnitState::Melee)
                {
                    first_melee = Some(sim.elapsed());
                }
                if sim.elapsed() >= next && next <= 330.0 {
                    next += 30.0;
                    let own = sim.recent_missile_losses(SideId::Attacker, window);
                    let enemy = sim.recent_missile_losses(SideId::Defender, window);
                    let won = if sim.duel_rules().winning(own, enemy) {
                        "W"
                    } else {
                        ""
                    };
                    line.push_str(&format!(
                        " t{:.0}:{:.3}/{:.3}{won}",
                        sim.elapsed(),
                        own,
                        enemy
                    ));
                }
            }
            println!(
                "case {k} seed {seed}: {:?} {:?} at {:.0} s, first melee {:?};{line}",
                sim.winner(),
                sim.end_kind(),
                sim.elapsed(),
                first_melee.map(|t| t.round())
            );
        }
    }
}

/// Mean `z` of the attacker's regiments of `kind` still present.
fn mean_z(sim: &BattleSim, kind: &str) -> f64 {
    let zs: Vec<f64> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker && u.unit_type == kind && u.present())
        .map(|u| u.z)
        .collect();
    zs.iter().sum::<f64>() / zs.len().max(1) as f64
}

/// Probe (ignored): the attacker's longbows replace its crossbows (it wins
/// the duel); where its men-at-arms stand over time.
#[test]
#[ignore = "probe"]
fn probe_epic_duel() {
    let kinds = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_knights",
        "unit_urban_militia",
        "unit_longbowmen",
    ];
    for limited in [false, true] {
        let mut sim = flat_battle_with(1, &kinds);
        if limited {
            let mut rules = sim.duel_rules().clone();
            rules.winning_duel_max_seconds = rules.duel_limit_seconds;
            sim.set_duel_rules(rules);
        }
        let window = sim.duel_rules().window_seconds;
        let mut next = 150.0;
        let mut line = String::new();
        while !sim.is_finished() && sim.elapsed() < 1800.0 {
            sim.step();
            if sim.elapsed() >= next {
                next += 30.0;
                let own = sim.recent_missile_losses(SideId::Attacker, window);
                let enemy = sim.recent_missile_losses(SideId::Defender, window);
                let won = if sim.duel_rules().winning(own, enemy) {
                    "W"
                } else {
                    ""
                };
                line.push_str(&format!(
                    " t{:.0}:z{:.0},{:.3}/{:.3}{won}",
                    sim.elapsed(),
                    mean_z(&sim, "unit_men_at_arms_foot"),
                    own,
                    enemy
                ));
            }
        }
        println!(
            "limited {limited}: {:?} {:?} at {:.0} s;{line}",
            sim.winner(),
            sim.end_kind(),
            sim.elapsed()
        );
    }
}
