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
    let mirror_only = std::env::var("EP3_MIRROR").is_ok();
    for terrain in [Terrain::Plains, Terrain::Hills, Terrain::Bocage] {
        let key = terrain.key();
        print(
            &format!("{key} mirror AI/AI"),
            &survey(&data, terrain, &ARMY, None),
        );
        if mirror_only {
            continue;
        }
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

#[test]
#[ignore = "debug trace: EP3_TRACE=terrain,seed"]
fn trace_river_battle() {
    let data = data();
    let spec = std::env::var("EP3_TRACE").unwrap_or_else(|_| "plains,0".into());
    let parts: Vec<&str> = spec.split(',').collect();
    let terrain = match parts[0] {
        "bocage" => Terrain::Bocage,
        "hills" => Terrain::Hills,
        _ => Terrain::Plains,
    };
    let seed: u64 = parts[1].parse().unwrap();
    let mut battle = setup(units(&data, &ARMY), units(&data, &ARMY), None);
    battle.terrain = terrain;
    battle.river = true;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    match parts.get(2).copied() {
        Some("def") => sim.set_ai(SideId::Defender, false),
        Some("att") => sim.set_ai(SideId::Attacker, false),
        _ => {}
    }
    let f = sim.field();
    let r = f.river.as_ref().unwrap();
    println!(
        "river width {:.0} fords {:?} bridges {:?}",
        r.width,
        r.fords.iter().map(|f| f.x as i32).collect::<Vec<_>>(),
        f.bridges
            .iter()
            .map(|b| (b.x as i32, b.z as i32, b.width as i32))
            .collect::<Vec<_>>()
    );
    let mut t = 0.0;
    while !sim.is_finished() {
        if sim.elapsed() >= t {
            println!("--- t={:.0}", sim.elapsed());
            for u in sim.units() {
                if !u.present() {
                    continue;
                }
                println!(
                    "{:>2} {:<3} {:<22} x{:>5.0} z{:>5.0} rz{:>5.0} hp{:>4.0} m{:>3.0} {:?} {:?} tgt{:?} dst{:?}",
                    u.id,
                    &u.side.key()[..3],
                    u.unit_type,
                    u.x,
                    u.z,
                    sim.field().river.as_ref().unwrap().center_z(u.x),
                    u.hp,
                    u.morale,
                    u.state,
                    sim.field().water_kind(u.x, u.z),
                    u.target,
                    u.destination.map(|(a, b)| (a as i32, b as i32))
                );
            }
            t += 30.0;
        }
        sim.step();
    }
    println!("winner {:?} at {:.0}", sim.winner(), sim.elapsed());
    for e in sim.events().iter().take(60) {
        println!("{:.0} {}", e.time, e.text_fr);
    }
}
