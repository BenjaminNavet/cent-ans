//! EP3: AI-against-AI probe on river fields (before/after the water lot).
//!
//! `cargo test --release -p sim-battle --test ep3_probe -- --ignored --nocapture`
//! (`EP3_SEEDS=0..32` for another sample). Only the public API of the
//! simulation is used, so the same probe runs on the code before EP3.

mod common;

use common::*;
use data_model::{GameData, Terrain};
use sim_battle::{BattleSim, SideId};

const ARMY: [&str; 6] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
];

/// A stronger attacker (eight regiments) against the six of `ARMY`.
const STRONG: [&str; 8] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_knights",
    "unit_urban_militia",
];

struct Tally {
    attacker_wins: u32,
    battles: u32,
    seconds: f64,
    attacker_losses: f64,
    defender_losses: f64,
}

fn losses(sim: &BattleSim, side: SideId) -> f64 {
    let (hp, initial) = sim
        .units()
        .iter()
        .filter(|u| u.side == side && !u.synthetic)
        .fold((0.0, 0.0), |(hp, n), u| {
            (hp + u.hp.max(0.0), n + f64::from(u.initial_soldiers))
        });
    1.0 - hp / initial.max(1.0)
}

fn battle(
    data: &GameData,
    terrain: Terrain,
    seed: u64,
    attacker: &[&str],
    passive: Option<SideId>,
) -> BattleSim {
    let mut setup = setup(units(data, attacker), units(data, &ARMY), None);
    setup.terrain = terrain;
    setup.river = true;
    let mut sim = BattleSim::new(setup, seed).unwrap();
    if let Some(side) = passive {
        sim.set_ai(side, false);
    }
    run_to_end(&mut sim);
    sim
}

fn seeds() -> std::ops::Range<u64> {
    std::env::var("EP3_SEEDS")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once("..")?;
            Some(a.parse().ok()?..b.parse().ok()?)
        })
        .unwrap_or(0..16)
}

fn survey(data: &GameData, terrain: Terrain, attacker: &[&str], passive: Option<SideId>) -> Tally {
    let mut tally = Tally {
        attacker_wins: 0,
        battles: 0,
        seconds: 0.0,
        attacker_losses: 0.0,
        defender_losses: 0.0,
    };
    for seed in seeds() {
        let sim = battle(data, terrain, seed, attacker, passive);
        tally.battles += 1;
        tally.seconds += sim.elapsed();
        tally.attacker_losses += losses(&sim, SideId::Attacker);
        tally.defender_losses += losses(&sim, SideId::Defender);
        if sim.winner() == Some(SideId::Attacker) {
            tally.attacker_wins += 1;
        }
    }
    tally
}

fn print(label: &str, t: &Tally) {
    let n = f64::from(t.battles.max(1));
    println!(
        "{label:<34} attacker {:>2}/{:<2}  mean {:>4.0} s  losses att {:>3.0} % def {:>3.0} %",
        t.attacker_wins,
        t.battles,
        t.seconds / n,
        100.0 * t.attacker_losses / n,
        100.0 * t.defender_losses / n,
    );
}

#[test]
#[ignore = "survey: prints AI-against-AI results on river fields"]
fn survey_river_battles() {
    let data = data();
    for terrain in [Terrain::Plains, Terrain::Hills, Terrain::Bocage] {
        let key = terrain.key();
        print(
            &format!("{key} mirror AI/AI"),
            &survey(&data, terrain, &ARMY, None),
        );
        print(
            &format!("{key} strong attacker AI/AI"),
            &survey(&data, terrain, &STRONG, None),
        );
        print(
            &format!("{key} attacker vs passive def"),
            &survey(&data, terrain, &ARMY, Some(SideId::Defender)),
        );
        print(
            &format!("{key} defender vs passive att"),
            &survey(&data, terrain, &ARMY, Some(SideId::Attacker)),
        );
    }
}
