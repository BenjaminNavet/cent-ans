//! RX `batsim`: outcome survey and bridge / rout-contagion probes (ADR 0255).
//!
//! `cargo test --release -p sim-battle --test ai_tactics -- --ignored --nocapture rx_survey`
//! prints one line per seed: duration, winner, losses, regiments routed
//! without a loss, regiments destroyed with no kill credited.

use crate::common;

use crate::ep9_decisive::crecy;
use common::*;
use data_model::Terrain;
use sim_battle::{
    BattleCrossing, BattleSetup, BattleSim, CrossingStructure, SideId, UnitState,
};

const KINDS: [&str; 5] = [
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
    "unit_crossbowmen",
];

fn army(n: usize, offset: usize) -> Vec<sim_battle::UnitSetup> {
    let ids: Vec<&str> = (0..n).map(|i| KINDS[(i + offset) % KINDS.len()]).collect();
    units(data(), &ids)
}

pub fn bridge_setup(n: usize, structure: CrossingStructure) -> BattleSetup {
    let mut battle = setup(army(n, 0), army(n, 2), None);
    battle.terrain = Terrain::Plains;
    battle.crossing = Some(BattleCrossing {
        structure,
        name: "Pont".to_owned(),
        river: "Seine".to_owned(),
    });
    battle
}

#[derive(Default, Debug)]
pub struct Report {
    pub seconds: f64,
    pub winner: Option<SideId>,
    pub losses: [u32; 2],
    pub initial: [u32; 2],
    pub routed_no_loss: u32,
    pub destroyed_no_kill: u32,
    pub kills: [f64; 2],
    /// Regiments that went into deep water (men drowned, nobody credited).
    pub drowning_regiments: usize,
    pub peak_on_deck: usize,
}

/// Plays to the end (cap 1800 s) and collects the accounting.
pub fn probe(sim: &mut BattleSim) -> Report {
    let mut peak = 0;
    let deck = sim.field().bridges.first().cloned();
    let mut routed_zero = std::collections::BTreeSet::new();
    while !sim.is_finished() && sim.elapsed() < 1800.0 {
        sim.step();
        if let Some(b) = &deck {
            let on = sim
                .units()
                .iter()
                .filter(|u| u.side == SideId::Attacker && u.present() && { let (a, c) = b.local(u.x, u.z); a.abs() <= b.length * 0.5 && c.abs() <= b.width * 0.5 + 1.0 })
                .count();
            peak = peak.max(on);
        }
        for u in sim.units() {
            if u.state == UnitState::Routing && u.soldiers() == u.initial_soldiers {
                routed_zero.insert(u.id);
            }
        }
    }
    let mut r = Report {
        seconds: sim.elapsed(),
        winner: sim.winner(),
        peak_on_deck: peak,
        ..Default::default()
    };
    for u in sim.units() {
        let s = usize::from(u.side == SideId::Defender);
        r.initial[s] += u.initial_soldiers;
        r.kills[s] += u.kills;
        r.losses[s] += u.initial_soldiers.saturating_sub(u.soldiers());
        if u.soldiers() == 0 && u.kills.round() as i64 == 0 {
            r.destroyed_no_kill += 1;
            if std::env::var("RX_VERBOSE").is_ok() {
                println!(
                    "  no-kill destroyed: {} {:?} cause {:?} by {:?} initial {}",
                    u.id, u.category, u.loss_cause, u.loss_by, u.initial_soldiers
                );
            }
        }
    }
    r.drowning_regiments = sim
        .events()
        .iter()
        .filter(|e| e.text_fr.contains("se noient"))
        .count();
    r.routed_no_loss = routed_zero
        .iter()
        .filter(|id| {
            let u = &sim.units()[**id as usize];
            u.soldiers() == u.initial_soldiers
        })
        .count() as u32;
    r
}

#[test]
#[ignore]
fn rx_survey() {
    for (n, structure) in [
        (10, CrossingStructure::StoneBridge),
        (24, CrossingStructure::StoneBridge),
        (24, CrossingStructure::WoodBridge),
    ] {
        for seed in 0..8 {
            let mut sim = BattleSim::new(bridge_setup(n, structure), seed).unwrap();
            let r = probe(&mut sim);
            println!("{n} {structure:?} seed {seed}: {r:?}");
        }
    }
    let demo: BattleSetup =
        serde_json::from_str(include_str!("../fixtures/demo_battle_1337.json")).unwrap();
    for seed in 0..8 {
        let mut sim = BattleSim::new(demo.clone(), 1337 + seed).unwrap();
        let r = probe(&mut sim);
        println!("demo seed {seed}: {r:?}");
    }
}

#[test]
#[ignore]
fn rx_trace() {
    let mut sim = BattleSim::new(bridge_setup(10, CrossingStructure::StoneBridge), 0).unwrap();
    let b = sim.field().bridges.first().cloned().unwrap();
    let river = sim.field().river.clone().unwrap();
    println!("bridge {:?} dir {:?}", (b.x, b.z, b.length, b.width), b.dir);
    while !sim.is_finished() && sim.elapsed() < 280.0 {
        sim.step();
        if (sim.elapsed() * 10.0).round() as u64 % 50 == 0 && sim.elapsed() > 185.0 {
            let mut line = format!("{:5.0}:", sim.elapsed());
            for u in sim.units().iter().filter(|u| u.side == SideId::Attacker) {
                let d = (u.x - b.x).hypot(u.z - b.z);
                let north = river.north_of(u.x, u.z);
                line += &format!(" [{} {:?} d{:.0} {}]", u.id, u.category, d, if north { "N" } else { "S" });
            }
            println!("{line}");
        }
    }
}

/// Accounting: on dry ground what a side's regiments are credited with
/// killing is what the other side lost (drowning, stakes and pikes aside).
#[test]
fn kills_credited_match_the_enemy_losses() {
    for seed in 0..3 {
        let mut sim = crecy(seed, false);
        let r = probe(&mut sim);
        for s in 0..2 {
            let lost = f64::from(r.losses[1 - s]);
            assert!(
                (r.kills[s] - lost).abs() <= lost * 0.05 + 3.0,
                "seed {seed} side {s}: credited {:.0} kills, the enemy lost {lost}",
                r.kills[s]
            );
        }
    }
}
