//! EQ7: the attacking horse waits for its foot before riding into the
//! range of archers who still have arrows.

mod common;

use common::*;
use sim_battle::{BattleSim, SideId};

/// The small mixed battle of `b6` (no site, AI on both sides).
fn small_mixed_battle(seed: u64) -> BattleSim {
    let data = data();
    let french = [
        "unit_knights",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_crossbowmen",
        "unit_crossbowmen",
        "unit_knights",
    ];
    let english = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_knights",
    ];
    let mut battle = setup(units(&data, &french), units(&data, &english), None);
    battle.village = Some(false);
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    sim
}

/// Probe (ignored): French wins over seeds 0-63, then a trace of seed
/// `EQ7_SEED` (default 3) every 10 s.
#[test]
#[ignore = "probe"]
fn probe_mixed_battle() {
    let seeds: u64 = std::env::var("EQ7_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);
    let mut french = 0;
    for seed in 0..seeds {
        let mut sim = small_mixed_battle(seed);
        run_to_end(&mut sim);
        french += usize::from(sim.winner() == Some(SideId::Attacker));
    }
    println!("French win {french}/{seeds}");
    let Some(seed) = std::env::var("EQ7_SEED").ok().and_then(|s| s.parse().ok()) else {
        return;
    };
    let mut sim = small_mixed_battle(seed);
    let mut step = 0;
    while !sim.is_finished() && sim.elapsed() < 400.0 {
        while sim.elapsed() < f64::from(step) * 10.0 && !sim.is_finished() {
            sim.step();
        }
        step += 1;
        println!("t={:.0}", sim.elapsed());
        for u in sim.units() {
            println!(
                "  {:?} {:24} {:5.0} {:5.0} hp {:4.0} mor {:3.0} ammo {:3} {:?} tgt {:?}",
                u.side, u.name, u.x, u.z, u.hp, u.morale, u.ammo, u.state, u.target
            );
        }
    }
    println!("winner {:?} at {:.0}", sim.winner(), sim.elapsed());
}
