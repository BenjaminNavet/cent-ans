//! IA night (2026-09-30): how well the battle AI plays, against three
//! baselines, and what it costs.
//!
//! - `naive`: a novice player — every regiment attacks the nearest enemy
//!   regiment, re-targeted every 2 s like the AI;
//! - `passive`: the other side's AI is off (units only defend and shoot at
//!   will);
//! - `mirror`: the AI on both sides.
//!
//! `cargo test --release -p sim-battle --test ia_survey -- --ignored --nocapture`
//! prints the survey table. `IA_SEEDS=0..16` (default), `IA_TERRAINS=plains,hills`,
//! `IA_ARMIES=mirror,en_vs_fr,fr_vs_en`, `IA_FOES=naive,passive`, `IA_TRACE=1`
//! (one line per battle), `IA_SELF=naive` (the measured side played by the
//! novice: the baseline).

mod common;

use std::time::Instant;

use common::*;
use data_model::{GameData, Terrain};
use sim_battle::{BattleSim, Command, SideId, UnitState, DT};

/// Mirrored mixed army (tests/ai.rs).
const MIXED: [&str; 6] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
];
/// English army of the period: bows and dismounted men-at-arms.
const ENGLISH: [&str; 7] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_urban_militia",
];
/// French army of the period: knights, crossbows, foot.
const FRENCH: [&str; 7] = [
    "unit_knights",
    "unit_knights",
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_crossbowmen",
    "unit_crossbowmen",
    "unit_urban_militia",
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Foe {
    Naive,
    Passive,
    Mirror,
}

/// Novice orders for `side`: each able regiment attacks the nearest able
/// enemy regiment.
fn naive_orders(sim: &BattleSim, side: SideId) -> Vec<Command> {
    let units = sim.units();
    let able = |u: &sim_battle::Unit| {
        u.hp > 0.0 && !u.left_field && u.state != UnitState::Routing && !u.synthetic
    };
    let mut commands = Vec::new();
    for own in units.iter().filter(|u| u.side == side && able(u)) {
        let nearest = units
            .iter()
            .filter(|e| e.side != side && able(e))
            .min_by(|a, b| {
                let da = (a.x - own.x).powi(2) + (a.z - own.z).powi(2);
                let db = (b.x - own.x).powi(2) + (b.z - own.z).powi(2);
                da.total_cmp(&db)
            });
        if let Some(enemy) = nearest {
            if own.target != Some(enemy.id) {
                commands.push(Command::Attack {
                    units: vec![own.id],
                    target: enemy.id,
                    run: false,
                    queue: false,
                });
            }
        }
    }
    commands
}

struct Outcome {
    ai_won: bool,
    ai_lost: bool,
    seconds: f64,
    /// Share of the AI side's starting soldiers still standing.
    ai_left: f64,
    foe_left: f64,
}

fn battle(
    data: &GameData,
    terrain: Terrain,
    ai_army: &[&str],
    foe_army: &[&str],
    ai_side: SideId,
    foe: Foe,
    seed: u64,
) -> Outcome {
    let (attacker, defender) = match ai_side {
        SideId::Attacker => (units(data, ai_army), units(data, foe_army)),
        SideId::Defender => (units(data, foe_army), units(data, ai_army)),
    };
    let mut battle = setup(attacker, defender, None);
    battle.terrain = terrain;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    let foe_side = ai_side.other();
    let start_ai = sim.strength(ai_side).max(1) as f64;
    let start_foe = sim.strength(foe_side).max(1) as f64;
    if foe != Foe::Mirror {
        sim.set_ai(foe_side, false);
    }
    // `IA_SELF=naive`: the measured side is played by the novice too (the
    // baseline the AI must beat).
    let self_naive = std::env::var("IA_SELF").is_ok_and(|v| v == "naive");
    if self_naive {
        sim.set_ai(ai_side, false);
    }
    let period = (2.0 / DT).round() as u64;
    // `IA_LOG=1`: a snapshot of every regiment each 20 s (one battle).
    let log = std::env::var("IA_LOG").is_ok();
    let mut tick = 0u64;
    while !sim.is_finished() {
        if foe == Foe::Naive && tick % period == 0 {
            for command in naive_orders(&sim, foe_side) {
                let _ = sim.apply_command(command, Some(foe_side));
            }
        }
        if self_naive && tick % period == 0 {
            for command in naive_orders(&sim, ai_side) {
                let _ = sim.apply_command(command, Some(ai_side));
            }
        }
        sim.step();
        tick += 1;
        if log && tick % (20.0 / DT).round() as u64 == 0 {
            println!("t={:.0}", sim.elapsed());
            for u in sim.units().iter().filter(|u| !u.synthetic) {
                println!(
                    "  {:>2} {:?} {:<22} {:?} x{:.0} z{:.0} hp{:.0}/{} m{:.0} ammo{} tgt{:?}",
                    u.id,
                    u.side,
                    u.unit_type,
                    u.state,
                    u.x,
                    u.z,
                    u.hp,
                    u.initial_soldiers,
                    u.morale,
                    u.ammo,
                    u.target
                );
            }
        }
    }
    Outcome {
        ai_won: sim.winner() == Some(ai_side),
        ai_lost: sim.winner() == Some(foe_side),
        seconds: sim.elapsed(),
        ai_left: sim.strength(ai_side) as f64 / start_ai,
        foe_left: sim.strength(foe_side) as f64 / start_foe,
    }
}

fn seeds() -> std::ops::Range<u64> {
    std::env::var("IA_SEEDS")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once("..")?;
            Some(a.parse().ok()?..b.parse().ok()?)
        })
        .unwrap_or(0..16)
}

fn terrains() -> Vec<Terrain> {
    let all = [
        Terrain::Plains,
        Terrain::Bocage,
        Terrain::Hills,
        Terrain::Mountains,
    ];
    match std::env::var("IA_TERRAINS") {
        Ok(list) => all
            .into_iter()
            .filter(|t| list.split(',').any(|k| k == t.key()))
            .collect(),
        Err(_) => all.to_vec(),
    }
}

/// (name, AI army, foe army) pairs.
fn armies() -> Vec<(&'static str, &'static [&'static str], &'static [&'static str])> {
    let all: Vec<(&str, &[&str], &[&str])> = vec![
        ("mirror", &MIXED, &MIXED),
        ("en_vs_fr", &ENGLISH, &FRENCH),
        ("fr_vs_en", &FRENCH, &ENGLISH),
    ];
    match std::env::var("IA_ARMIES") {
        Ok(list) => all
            .into_iter()
            .filter(|(n, _, _)| list.split(',').any(|k| k == *n))
            .collect(),
        Err(_) => all,
    }
}

fn foes() -> Vec<Foe> {
    match std::env::var("IA_FOES") {
        Ok(list) => [Foe::Naive, Foe::Passive, Foe::Mirror]
            .into_iter()
            .filter(|f| {
                list.split(',')
                    .any(|k| k.eq_ignore_ascii_case(&format!("{f:?}")))
            })
            .collect(),
        Err(_) => vec![Foe::Naive, Foe::Passive, Foe::Mirror],
    }
}

#[test]
#[ignore]
fn survey() {
    let data = data();
    let trace = std::env::var("IA_TRACE").is_ok();
    let clock = Instant::now();
    println!("| foe | armies | terrain | AI side | won | lost | draw | AI left | foe left | s |");
    println!("|---|---|---|---|---|---|---|---|---|---|");
    let mut total = (0usize, 0usize, 0usize);
    for foe in foes() {
        for (name, ai_army, foe_army) in armies() {
            for terrain in terrains() {
                for ai_side in SideId::BOTH {
                    let (mut won, mut lost, mut n) = (0, 0, 0);
                    let (mut left, mut foe_left, mut secs) = (0.0, 0.0, 0.0);
                    for seed in seeds() {
                        let o = battle(&data, terrain, ai_army, foe_army, ai_side, foe, seed);
                        if trace {
                            println!(
                                "{foe:?} {name} {} {} {seed}: won {} lost {} {:.0} s, left {:.2}/{:.2}",
                                terrain.key(),
                                ai_side.key(),
                                o.ai_won,
                                o.ai_lost,
                                o.seconds,
                                o.ai_left,
                                o.foe_left
                            );
                        }
                        won += o.ai_won as usize;
                        lost += o.ai_lost as usize;
                        n += 1;
                        left += o.ai_left;
                        foe_left += o.foe_left;
                        secs += o.seconds;
                    }
                    total.0 += won;
                    total.1 += lost;
                    total.2 += n;
                    let n_f = n as f64;
                    println!(
                        "| {foe:?} | {name} | {} | {} | {won} | {lost} | {} | {:.2} | {:.2} | {:.0} |",
                        terrain.key(),
                        ai_side.key(),
                        n - won - lost,
                        left / n_f,
                        foe_left / n_f,
                        secs / n_f
                    );
                }
            }
        }
    }
    println!(
        "total: won {} lost {} of {} ({:.0} s wall)",
        total.0,
        total.1,
        total.2,
        clock.elapsed().as_secs_f64()
    );
}

/// Time spent in `ai::plan` alone per decision step, at 20, 63 and 105
/// regiments a side, 90 s into a mirrored battle.
#[test]
#[ignore]
fn plan_cost() {
    let data = data();
    for per_side in [20usize, 63, 105] {
        let army: Vec<&str> = MIXED.iter().copied().cycle().take(per_side).collect();
        let mut battle = setup(units(&data, &army), units(&data, &army), None);
        battle.terrain = Terrain::Plains;
        let mut sim = BattleSim::new(battle, 1).unwrap();
        run(&mut sim, 90.0);
        let mut best = f64::MAX;
        let mut total = 0.0;
        let reps = 20;
        for _ in 0..reps {
            let t = Instant::now();
            for side in SideId::BOTH {
                std::hint::black_box(sim_battle::ai::plan(&sim, side));
            }
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            best = best.min(ms);
            total += ms;
        }
        let t = Instant::now();
        let steps = 50;
        for _ in 0..steps {
            sim.step();
        }
        let step_ms = t.elapsed().as_secs_f64() * 1000.0 / steps as f64;
        println!(
            "{per_side} regiments a side: ai::plan both sides best {best:.2} ms, mean {:.2} ms; step mean {step_ms:.2} ms",
            total / reps as f64
        );
    }
}
