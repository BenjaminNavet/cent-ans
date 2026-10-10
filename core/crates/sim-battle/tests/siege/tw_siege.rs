//! TW siege (ADR 0329): wall-tower and sortie rules in `data/rules/siege_works.json`,
//! counter-battery and the attacker's second point of assault.

use crate::common;

use common::*;
use sim_battle::{BattleSim, SideId, SiegeSetup, SiegeWorkRules};

fn siege(attackers: &[&str], defenders: &[&str], fortification: u32, seed: u64) -> BattleSim {
    let data = data();
    let setup = setup(
        units(data, attackers),
        units(data, defenders),
        Some(SiegeSetup {
            fortification,
            breach: 0,
            ..Default::default()
        }),
    );
    BattleSim::new(setup, seed).unwrap()
}

fn ram_id(sim: &BattleSim) -> usize {
    sim.units().iter().position(|u| u.ram).unwrap()
}

/// Puts the ram `offset` metres in front of the gate.
fn ram_at_gate(sim: &mut BattleSim, offset: f64) -> usize {
    let ram = ram_id(sim);
    let works = sim.siege().unwrap();
    let gate = &works.pieces[works.gate];
    let (mx, mz) = gate.midpoint();
    let (nx, nz) = gate.outward();
    place(sim, ram as u32, mx + nx * offset, mz + nz * offset, 0.0);
    ram
}

#[test]
fn tower_and_sortie_rules_keep_the_old_values_at_fortification_3() {
    let rules = SiegeWorkRules::bundled();
    assert_eq!(rules.tower.range(3), 180.0);
    assert_eq!(rules.tower.shots(3), 5.0);
    assert_eq!(rules.tower.reload_s, 8.0);
    assert_eq!(rules.tower.accuracy, 0.3);
    assert_eq!(rules.tower.accuracy_range_loss, 0.5);
    assert_eq!(rules.tower.lethality, 0.6);
    assert_eq!(rules.sortie.ratio, 0.5);
    assert_eq!(rules.sortie.delay_s, 120.0);
    // The per-level hooks are inert in the shipped data.
    assert_eq!(rules.tower.range(5), rules.tower.range(1));
}

#[test]
fn wall_towers_loose_bolts_at_the_ram() {
    let mut sim = siege(&["unit_men_at_arms_foot"], &["unit_urban_militia"], 3, 31);
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Defender);
    let ram = ram_at_gate(&mut sim, 40.0);
    let hp0 = sim.units()[ram].hp;
    run(&mut sim, 60.0);
    assert!(
        sim.units()[ram].hp < hp0,
        "the towers around the gate shoot the ram: {} / {hp0}",
        sim.units()[ram].hp
    );
}

#[test]
fn wall_shooters_are_ordered_onto_the_ram() {
    let mut sim = siege(
        &["unit_men_at_arms_foot"],
        &["unit_crossbowmen", "unit_crossbowmen"],
        3,
        32,
    );
    lab(&mut sim);
    sim.set_ai(SideId::Defender, true);
    let works = sim.siege().unwrap().clone();
    let gate = &works.pieces[works.gate];
    let (mx, mz) = gate.midpoint();
    let (nx, nz) = gate.outward();
    let ram = ram_at_gate(&mut sim, 50.0);
    for (k, i) in sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender && u.can_shoot())
        .map(|u| u.id)
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
    {
        place(
            &mut sim,
            i,
            mx - nx * 2.0 + k as f64 * 6.0,
            mz - nz * 2.0,
            0.0,
        );
        sim.units_mut()[i as usize].on_wall = true;
    }
    let ram_unit = sim.units()[ram].id;
    let hp0 = sim.units()[ram].hp;
    // A foot regiment stands nearer to the wall than the ram: at will, the
    // shooters would take it; the AI points them at the ram instead.
    let foot = sim
        .units()
        .iter()
        .position(|u| u.side == SideId::Attacker && !u.ram)
        .unwrap();
    place(
        &mut sim,
        foot as u32,
        mx + nx * 25.0 + 30.0,
        mz + nz * 25.0,
        0.0,
    );
    let mut aimed = false;
    for _ in 0..200 {
        sim.step();
        aimed |= sim
            .units()
            .iter()
            .any(|u| u.side == SideId::Defender && u.target == Some(ram_unit));
    }
    assert!(aimed, "a wall shooter targets the ram");
    assert!(sim.units()[ram].hp < hp0);
}

#[test]
fn engines_duel_each_other_before_the_walls() {
    let mut sim = siege(
        &["unit_trebuchet", "unit_men_at_arms_foot"],
        &["unit_bombard", "unit_urban_militia"],
        3,
        33,
    );
    lab(&mut sim);
    sim.set_ai(SideId::Attacker, true);
    let treb = sim
        .units()
        .iter()
        .position(|u| u.wall_breaker() && u.side == SideId::Attacker)
        .unwrap();
    let bomb = sim
        .units()
        .iter()
        .position(|u| u.side == SideId::Defender && u.category == data_model::UnitCategory::Siege)
        .unwrap();
    let works = sim.siege().unwrap().clone();
    let piece = works.front_walls()[0];
    let (mx, mz) = works.pieces[piece].midpoint();
    let (nx, nz) = works.pieces[piece].outward();
    let range = f64::from(sim.units()[treb].stats.range);
    place(
        &mut sim,
        treb as u32,
        mx + nx * range * 0.5,
        mz + nz * range * 0.5,
        0.0,
    );
    place(&mut sim, bomb as u32, mx - nx * 8.0, mz - nz * 8.0, 0.0);
    // A friend on the wall walk spots the bombard for the trebuchet.
    let scout = sim
        .units()
        .iter()
        .position(|u| u.side == SideId::Attacker && !u.wall_breaker() && !u.ram)
        .unwrap();
    place(&mut sim, scout as u32, mx + nx * 40.0, mz + nz * 40.0, 0.0);
    let bomb_id = sim.units()[bomb].id;
    let hp0 = sim.units()[bomb].hp;
    let mut aimed = false;
    for _ in 0..80 {
        sim.step();
        aimed |= sim.units()[treb].target == Some(bomb_id);
    }
    assert!(aimed, "the attacker's engine answers the garrison's");
    assert!(
        sim.units()[bomb].hp < hp0,
        "and its stone lands on the bombard"
    );
}

#[test]
fn the_second_assault_threshold_is_data_driven() {
    let rules = &SiegeWorkRules::bundled().second_assault;
    assert!(rules.applies(
        rules.min_power_ratio,
        rules.min_foot_regiments,
        rules.min_engines
    ));
    assert!(!rules.applies(
        rules.min_power_ratio - 0.01,
        rules.min_foot_regiments,
        rules.min_engines
    ));
    assert!(!rules.applies(
        rules.min_power_ratio,
        rules.min_foot_regiments - 1,
        rules.min_engines
    ));
    assert!(!rules.applies(
        rules.min_power_ratio,
        rules.min_foot_regiments,
        rules.min_engines - 1
    ));
}

/// Number of distinct front stretches battered after `seconds`.
fn battered_stretches(attackers: &[&str], defenders: &[&str], seconds: f64) -> usize {
    let mut sim = siege(attackers, defenders, 5, 34);
    lab(&mut sim);
    sim.set_ai(SideId::Attacker, true);
    hold_fire(&mut sim, SideId::Defender);
    // From the first stone to `seconds` later (no stretch falls in that time).
    let hurt = |sim: &BattleSim| {
        let works = sim.siege().unwrap();
        works
            .front_walls()
            .into_iter()
            .any(|p| works.pieces[p].hp < works.pieces[p].max_hp)
    };
    for _ in 0..4000 {
        if hurt(&sim) {
            break;
        }
        sim.step();
    }
    run(&mut sim, seconds);
    let works = sim.siege().unwrap();
    works
        .front_walls()
        .into_iter()
        .filter(|&p| works.pieces[p].hp < works.pieces[p].max_hp)
        .count()
}

#[test]
fn a_superior_attacker_batters_two_stretches_at_once() {
    let strong = [
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_trebuchet",
        "unit_trebuchet",
    ];
    let split = battered_stretches(&strong, &["unit_urban_militia"], 40.0);
    assert!(split >= 2, "two stretches battered: {split}");
    // Outnumbered, or too few engines: one point of assault.
    let weak = battered_stretches(
        &["unit_men_at_arms_foot", "unit_trebuchet", "unit_trebuchet"],
        &[
            "unit_urban_militia",
            "unit_urban_militia",
            "unit_urban_militia",
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
        ],
        40.0,
    );
    assert_eq!(weak, 1, "no split without the upper hand");
}
