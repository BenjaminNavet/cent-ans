//! Lot TW balance (ADR 0328): rock-paper-scissors duel matrix, one regiment
//! against one regiment on open ground (A/B duel tool, ADR 0148 spirit).
//! `cargo test -p sim-battle --test combat tw_balance::print_matrix -- --ignored --nocapture`
//! prints the full matrix; the other test pins the readable relations.

use crate::common;

use common::*;
use sim_battle::{BattleSim, Command};

const ROW: &[&str] = &[
    "unit_welsh_spearmen",
    "unit_urban_militia",
    "unit_men_at_arms_foot",
    "unit_knights",
    "unit_mounted_sergeants",
    "unit_hobelars",
    "unit_longbowmen",
    "unit_crossbowmen",
    "unit_genoese_crossbowmen",
    "unit_flemish_pikemen",
];

/// Net result of `a` against `b` (>0: `a` wins), in shares of initial hit
/// points left: `a_left - b_left`. Both start 300 m apart facing each other
/// and attack; 10 minutes at most; the mean of several seeds.
pub fn duel(a: &str, b: &str) -> f64 {
    let data = data();
    let seeds = [3u64, 7, 11];
    let mut total = 0.0;
    for seed in seeds {
        let mut sim =
            BattleSim::new(setup(vec![unit(data, a)], vec![unit(data, b)], None), seed).unwrap();
        lab(&mut sim);
        apply_overrides(&mut sim, [a, b]);
        place(&mut sim, 0, 600.0, 250.0, 0.0);
        place(&mut sim, 1, 600.0, 550.0, std::f64::consts::PI);
        let hp0 = (sim.units()[0].hp, sim.units()[1].hp);
        for (id, target) in [(0u32, 1u32), (1, 0)] {
            // Horsemen gallop in (the AI runs its cavalry), foot walks.
            let run = sim.units()[id as usize].stats.charge.is_some();
            sim.issue_command(Command::Attack {
                units: vec![id],
                target,
                run,
                queue: false,
            })
            .unwrap();
        }
        // The defender side's order goes through the same command path.
        run(&mut sim, 600.0);
        let left = |i: usize, hp: f64| {
            let u = &sim.units()[i];
            if u.state == sim_battle::UnitState::Routing {
                0.0
            } else {
                u.hp / hp
            }
        };
        if std::env::var("VERBOSE").is_ok() {
            println!(
                "  {a} vs {b} seed {seed}: {:.2} / {:.2} ({:?}/{:?})",
                left(0, hp0.0),
                left(1, hp0.1),
                sim.units()[0].state,
                sim.units()[1].state
            );
        }
        total += left(0, hp0.0) - left(1, hp0.1);
    }
    total / seeds.len() as f64
}

/// Lab knobs (`OVR=unit_x.field=v;...`, `NOSTAKES=1`) to try a calibration
/// without recompiling the data.
fn apply_overrides(sim: &mut BattleSim, ids: [&str; 2]) {
    let ovr = std::env::var("OVR").unwrap_or_default();
    for (i, id) in ids.iter().enumerate() {
        let unit = &mut sim.units_mut()[i];
        if std::env::var("NOSTAKES").is_ok() {
            unit.remove_ability(data_model::Ability::Stakes);
        }
        for item in ovr.split(';').filter(|s| !s.is_empty()) {
            let (key, value) = item.split_once('=').unwrap();
            let (id_, field) = key.split_once('.').unwrap();
            if id_ != *id {
                continue;
            }
            let v: f64 = value.parse().unwrap();
            match field {
                "reload_s" => unit.stats.reload_s = Some(v as u8),
                "ranged" => unit.stats.ranged = v as u8,
                "melee" => unit.stats.melee = v as u8,
                "armor" => unit.stats.armor = v as u8,
                "morale" => unit.stats.morale = v as u8,
                "charge" => unit.stats.charge = Some(v as u8),
                "range" => unit.stats.range = v as u32,
                "ammo" => {
                    unit.stats.ammo = v as u32;
                    unit.ammo = v as u32;
                }
                other => panic!("unknown field {other}"),
            }
        }
    }
}

#[test]
#[ignore]
fn print_matrix() {
    print!("{:<24}", "row beats col");
    for b in ROW {
        print!("{:>7}", &b[5..9]);
    }
    println!();
    for a in ROW {
        print!("{:<24}", &a[5..]);
        for b in ROW {
            print!("{:>7.2}", if a == b { 0.0 } else { duel(a, b) });
        }
        println!();
    }
}

/// The relations a player reads like in Medieval II (ADR 0328). Margins are
/// loose: they pin the direction of the triangle, not the exact numbers.
#[test]
fn rock_paper_scissors_is_readable() {
    let beats = |a: &str, b: &str, margin: f64| {
        let score = duel(a, b);
        assert!(score >= margin, "{a} vs {b}: {score:.2} < {margin}");
    };
    // Spears stop horsemen of their price class; heavy horse still rides
    // militia down.
    beats("unit_welsh_spearmen", "unit_mounted_sergeants", 0.3);
    beats("unit_flemish_pikemen", "unit_knights", 0.3);
    beats("unit_knights", "unit_urban_militia", 0.3);
    // Horse catches shooters; shooters shred foot.
    beats("unit_knights", "unit_longbowmen", 0.2);
    beats("unit_knights", "unit_crossbowmen", 0.3);
    beats("unit_mounted_sergeants", "unit_genoese_crossbowmen", 0.3);
    beats("unit_longbowmen", "unit_men_at_arms_foot", 0.5);
    beats("unit_longbowmen", "unit_welsh_spearmen", 0.5);
    // Quality beats numbers on foot.
    beats("unit_men_at_arms_foot", "unit_urban_militia", 0.4);
    beats("unit_men_at_arms_foot", "unit_welsh_spearmen", 0.4);
}
