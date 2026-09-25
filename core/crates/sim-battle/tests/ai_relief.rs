//! R2b: the battle AI reads the relief (crests, reverse slopes, valleys,
//! lines of sight). Win rate of an active AI against a passive side, mirrored
//! armies, on each relief style.
//!
//! `cargo test --release -p sim-battle --test ai_relief -- --ignored --nocapture`
//! prints the survey table (32 battles per terrain).

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

/// Did the active AI beat a passive `passive` side on `terrain`?
fn active_wins(data: &GameData, terrain: Terrain, seed: u64, passive: SideId) -> bool {
    let mut battle = setup(units(data, &ARMY), units(data, &ARMY), None);
    battle.terrain = terrain;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(passive, false);
    run_to_end(&mut sim);
    if std::env::var("R2B_TIMES").is_ok() {
        println!(
            "{} {seed} {} {:.0}",
            terrain.key(),
            passive.other().key(),
            sim.elapsed()
        );
    }
    sim.winner() == Some(passive.other())
}

/// Victories of the active AI over `seeds` x both passive sides, with the
/// lost battles as `seed/active side`.
fn wins(data: &GameData, terrain: Terrain, seeds: std::ops::Range<u64>) -> (usize, Vec<String>) {
    let mut won = 0;
    let mut lost = Vec::new();
    for seed in seeds {
        for passive in SideId::BOTH {
            if active_wins(data, terrain, seed, passive) {
                won += 1;
            } else {
                lost.push(format!("{seed}/{}", passive.other().key()));
            }
        }
    }
    (won, lost)
}

#[test]
#[ignore = "survey: run in release with --nocapture"]
fn survey_active_against_passive() {
    let data = data();
    for terrain in [
        Terrain::Plains,
        Terrain::Bocage,
        Terrain::Hills,
        Terrain::Mountains,
    ] {
        // R2B_SEEDS=16..48 checks another sample.
        let seeds = std::env::var("R2B_SEEDS")
            .ok()
            .and_then(|s| {
                let (a, b) = s.split_once("..")?;
                Some(a.parse().ok()?..b.parse().ok()?)
            })
            .unwrap_or(0..16);
        let n = 2 * (seeds.end - seeds.start);
        let (won, lost) = wins(&data, terrain, seeds);
        println!("{:<10} {won}/{n}  lost: {}", terrain.key(), lost.join(" "));
    }
}

#[test]
#[ignore = "debug trace: R2B_TRACE=terrain,seed,passive"]
fn trace_one_battle() {
    let data = data();
    let spec = std::env::var("R2B_TRACE").unwrap_or_else(|_| "plains,0,defender".into());
    let parts: Vec<&str> = spec.split(',').collect();
    let terrain = match parts[0] {
        "bocage" => Terrain::Bocage,
        "hills" => Terrain::Hills,
        "mountains" => Terrain::Mountains,
        _ => Terrain::Plains,
    };
    let seed: u64 = parts[1].parse().unwrap();
    let passive = if parts[2] == "attacker" {
        SideId::Attacker
    } else {
        SideId::Defender
    };
    let mut battle = setup(units(&data, &ARMY), units(&data, &ARMY), None);
    battle.terrain = terrain;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(passive, false);
    let mut t = 0.0;
    while !sim.is_finished() {
        if sim.elapsed() >= t {
            println!("--- t={:.0}", sim.elapsed());
            for u in sim.units() {
                if !u.present() {
                    continue;
                }
                println!(
                    "{:>2} {:<3} {:<22} x{:>5.0} z{:>5.0} h{:>5.1} hp{:>4.0} m{:>3.0} {:?} amm{} stk{} tgt{:?} dst{:?}",
                    u.id,
                    &u.side.key()[..3],
                    u.unit_type,
                    u.x,
                    u.z,
                    sim.field().height(u.x, u.z),
                    u.hp,
                    u.morale,
                    u.state,
                    u.ammo,
                    u.stakes_planted as u8, u.target, u.destination.map(|(a, b)| (a as i32, b as i32))
                );
            }
            t += if sim.elapsed() > 380.0 { 5.0 } else { 30.0 };
        }
        sim.step();
    }
    println!("winner {:?} at {:.0}", sim.winner(), sim.elapsed());
    for e in sim.events() {
        println!("{:>5.0} {}", e.time, e.text_fr);
    }
}
