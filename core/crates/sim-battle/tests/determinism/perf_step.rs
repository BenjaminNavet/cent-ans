//! Step timing (run on demand):
//! `cargo test --release -p sim-battle --test perf_step -- --ignored --nocapture`.

use crate::common;

use std::time::{Duration, Instant};

use common::*;
use sim_battle::replay::state_digest;
use sim_battle::{BattleSetup, BattleSim, ReplayStart, SideId, DT};

const KINDS: [&str; 5] = [
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
    "unit_crossbowmen",
];

fn demo_sim() -> BattleSim {
    let setup: BattleSetup =
        serde_json::from_str(include_str!("../fixtures/demo_battle_1337.json")).unwrap();
    let mut sim = ReplayStart::plain(setup, 42).build().unwrap();
    let _ = sim.start_battle();
    sim
}

/// AI against AI, 60 regiments of 120 a side on the epic field.
fn epic_sim() -> BattleSim {
    let data = data();
    let kinds: Vec<&str> = (0..60).map(|i| KINDS[i % KINDS.len()]).collect();
    let mut battle = setup(units(data, &kinds), units(data, &kinds), None);
    for unit in battle
        .attacker
        .units
        .iter_mut()
        .chain(battle.defender.units.iter_mut())
    {
        unit.soldiers = 120;
        unit.max_soldiers = 120;
    }
    let mut sim = BattleSim::new(battle, 11).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    sim
}

fn measure(label: &str, make: fn() -> BattleSim, steps: u32) {
    let mut sim = make();
    let started = Instant::now();
    for _ in 0..steps {
        sim.tick(DT);
    }
    let plain = started.elapsed();
    let plain_digest = state_digest(&sim);

    let mut sim = make();
    let mut fork_time = Duration::ZERO;
    let started = Instant::now();
    for _ in 0..steps {
        let forked = Instant::now();
        let mut next = sim.fork_for_step();
        fork_time += forked.elapsed();
        next.tick(DT);
        sim.adopt_step(next);
    }
    let total = started.elapsed();
    println!("  events={}", sim.events().len());
    assert_eq!(plain_digest, state_digest(&sim), "fork path diverges");
    println!(
        "{label}: steps={steps} plain={:?}/step fork+step+adopt={:?}/step fork_only={:?}/fork digest={plain_digest}",
        plain / steps,
        total / steps,
        fork_time / steps,
    );
}

#[test]
#[ignore]
fn time_steps_and_forks() {
    measure("demo", demo_sim, 3000);
    measure(
        "epic",
        epic_sim,
        std::env::var("PERF_STEPS").map_or(1500, |s| s.parse().unwrap()),
    );
}
