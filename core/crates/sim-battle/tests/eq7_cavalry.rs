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

/// The symmetric epic battle of `ep9b_duel` (60 regiments a side, flat).
fn symmetric_battle(seed: u64) -> BattleSim {
    use data_model::Ability;
    const KINDS: [&str; 5] = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_knights",
        "unit_urban_militia",
        "unit_crossbowmen",
    ];
    let data = data();
    // `EQ7_LONGBOWS`: the attacker's longbows replace its crossbows.
    let attacker_kinds = if std::env::var("EQ7_LONGBOWS").is_ok() {
        [KINDS[0], KINDS[1], KINDS[2], KINDS[3], KINDS[1]]
    } else {
        KINDS
    };
    let army: Vec<&str> = (0..60).map(|i| KINDS[i % KINDS.len()]).collect();
    let attackers: Vec<&str> = (0..60).map(|i| attacker_kinds[i % KINDS.len()]).collect();
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

/// Probe (ignored): the symmetric battle of `ep9b_duel`, per side and kind
/// every 30 s (seed `EQ7_SEED`, default 1).
#[test]
#[ignore = "probe"]
fn probe_symmetric_battle() {
    let seed = std::env::var("EQ7_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let mut sim = symmetric_battle(seed);
    const STEP: f64 = 15.0;
    let mut step = 0;
    while !sim.is_finished() && sim.elapsed() < 600.0 {
        while sim.elapsed() < f64::from(step) * STEP && !sim.is_finished() {
            sim.step();
        }
        step += 1;
        println!("t={:.0}", sim.elapsed());
        for side in [SideId::Attacker, SideId::Defender] {
            for kind in [
                "unit_knights",
                "unit_men_at_arms_foot",
                "unit_longbowmen",
                "unit_crossbowmen",
            ] {
                let us: Vec<_> = sim
                    .units()
                    .iter()
                    .filter(|u| u.side == side && u.unit_type == kind)
                    .collect();
                let hp: f64 = us.iter().map(|u| u.hp).sum();
                if us.is_empty() {
                    continue;
                }
                let z = us.iter().map(|u| u.z).sum::<f64>() / us.len() as f64;
                let melee = us
                    .iter()
                    .filter(|u| u.state == sim_battle::UnitState::Melee)
                    .count();
                let rout = us
                    .iter()
                    .filter(|u| u.state == sim_battle::UnitState::Routing)
                    .count();
                let ammo: u32 = us.iter().map(|u| u.ammo).sum::<u32>() / us.len() as u32;
                println!(
                    "  {side:?} {kind:24} hp {hp:5.0} z {z:5.0} melee {melee:2} rout {rout:2} ammo {ammo:2}"
                );
            }
        }
    }
    println!("winner {:?} at {:.0}", sim.winner(), sim.elapsed());
}
