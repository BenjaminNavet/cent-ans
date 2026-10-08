//! EP10 (ADR 0068): direction of the rout and contagion of morale.
//!
//! Reproduction of the SG5 night diagnosis: a defensive line whose wing
//! regiment is routed by an enemy on its flank. Before EP10 the fugitive
//! ran away from the nearest enemy (the flank, so along the line) and every
//! routing friend within 120 m shook the line wherever it stood: the line
//! gave way regiment after regiment.
//!
//! `cargo test --release -p sim-battle --test ep10_rout -- --ignored --nocapture probe`

mod common;

use common::*;
use sim_battle::{BattleSim, SideId, UnitState};

/// Regiments in the defensive line.
const LINE: usize = 8;
/// Interval between the centres of two regiments of the line (metres).
const INTERVAL: f64 = 50.0;
/// z of the defensive line, and of the enemy line facing it.
const LINE_Z: f64 = 600.0;
const ENEMY_Z: f64 = 510.0;

/// Mirrored lines of men-at-arms on foot (defender at `LINE_Z` facing the
/// attacker at `ENEMY_Z`), both idle; a regiment of knights stands on the
/// left flank of the defender's wing regiment (unit 0 of the defender), whose
/// morale is broken. The rest of the defensive line is shaken: morale and
/// morale cap `line_morale`.
fn flank_rout(line_morale: f64) -> (BattleSim, Vec<usize>, usize) {
    let data = data();
    let foot = vec!["unit_men_at_arms_foot"; LINE];
    let mut attackers = foot.clone();
    attackers.push("unit_knights");
    let mut sim = BattleSim::new(
        setup(units(&data, &attackers), units(&data, &foot), None),
        1,
    )
    .unwrap();
    lab(&mut sim);
    {
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
    }
    let x0 = 400.0;
    let mut line = Vec::new();
    let mut knights = 0;
    let (mut a, mut d) = (0usize, 0usize);
    let ids: Vec<(u32, SideId, bool)> = sim
        .units()
        .iter()
        .map(|u| (u.id, u.side, u.mounted))
        .collect();
    for (id, side, mounted) in ids {
        match side {
            SideId::Defender => {
                place(
                    &mut sim,
                    id,
                    x0 + d as f64 * INTERVAL,
                    LINE_Z,
                    std::f64::consts::PI,
                );
                line.push(id as usize);
                d += 1;
            }
            SideId::Attacker if mounted => {
                // On the flank of the wing, facing it, out of contact.
                place(&mut sim, id, x0 - 70.0, LINE_Z, std::f64::consts::FRAC_PI_2);
                knights = id as usize;
            }
            SideId::Attacker => {
                place(&mut sim, id, x0 + a as f64 * INTERVAL, ENEMY_Z, 0.0);
                a += 1;
            }
        }
    }
    for (k, &i) in line.iter().enumerate() {
        let unit = &mut sim.units_mut()[i];
        if k == 0 {
            unit.morale = 15.0;
        } else {
            unit.morale = line_morale;
            unit.morale_cap = line_morale;
        }
    }
    (sim, line, knights)
}

/// Regiments of the line (beyond the wing) that routed within `seconds`,
/// and the lateral run of the wing regiment along the line (metres).
fn contagion(line_morale: f64, seconds: f64, trace: bool) -> (usize, f64) {
    let (mut sim, line, _) = flank_rout(line_morale);
    let wing_x = sim.units()[line[0]].x;
    let mut routed = vec![false; line.len()];
    let mut lateral: f64 = 0.0;
    let steps = (seconds / sim_battle::DT).round() as usize;
    for step in 0..steps {
        sim.step();
        for (k, &i) in line.iter().enumerate() {
            if sim.units()[i].state == UnitState::Routing {
                routed[k] = true;
            }
        }
        let wing = &sim.units()[line[0]];
        if wing.present() {
            lateral = lateral.max(wing.x - wing_x);
        }
        if trace && step % 50 == 0 {
            let states: Vec<String> = line
                .iter()
                .map(|&i| {
                    let u = &sim.units()[i];
                    format!(
                        "{:.0}{}",
                        u.morale,
                        if u.state == UnitState::Routing {
                            "R"
                        } else {
                            ""
                        }
                    )
                })
                .collect();
            println!(
                "  t {:>5.1} wing ({:.0},{:.0}) morale {}",
                sim.elapsed(),
                wing.x,
                wing.z,
                states.join(" ")
            );
        }
    }
    (routed[1..].iter().filter(|r| **r).count(), lateral)
}

/// The SG5 case: before EP10 the whole shaken line (morale 30) gave way
/// behind its routed wing, 7 regiments out of 7, the fugitive running 113 m
/// along the line. Now the wing runs to the rear and the line holds.
#[test]
fn a_wing_routed_from_the_flank_does_not_break_the_line() {
    let (routed, lateral) = contagion(30.0, 120.0, false);
    assert!(routed <= 1, "{routed} regiments of the line gave way");
    assert!(lateral < 25.0, "the wing ran {lateral:.0} m along the line");
}
