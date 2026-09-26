//! EP10 (ADR 0067): direction of the rout and contagion of morale.
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

/// Probe (ignored): regiments of the line giving way by contagion for
/// several morale levels of the shaken line.
#[test]
#[ignore = "probe"]
fn probe() {
    let trace = std::env::var("EP10_TRACE").is_ok();
    println!(
        "| Moral de la ligne | Régiments qui cèdent (sur {}) | Course latérale de l'aile (m) |",
        LINE - 1
    );
    println!("|---|---|---|");
    for morale in [24.0, 27.0, 30.0, 35.0, 40.0] {
        let (routed, lateral) = contagion(morale, 120.0, trace);
        println!("| {morale:.0} | {routed} | {lateral:.0} |");
    }
}

/// The symmetric epic battle of `ep9b_duel` (60 regiments of 120 men a
/// side, flat bare ground, no stakes, both AIs).
fn symmetric_battle(seed: u64) -> BattleSim {
    const KINDS: [&str; 5] = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_knights",
        "unit_urban_militia",
        "unit_crossbowmen",
    ];
    let data = data();
    let army: Vec<&str> = (0..60).map(|i| KINDS[i % KINDS.len()]).collect();
    let mut setup = setup(units(&data, &army), units(&data, &army), None);
    setup.village = Some(false);
    for unit in setup
        .attacker
        .units
        .iter_mut()
        .chain(setup.defender.units.iter_mut())
    {
        unit.soldiers = 120;
        unit.max_soldiers = 120;
        unit.abilities.retain(|a| *a != data_model::Ability::Stakes);
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

/// Probe (ignored): routs, rallies and regiments gone from the field per
/// side in the symmetric epic battle (`EP10_SEEDS=1..11`).
#[test]
#[ignore = "probe"]
fn probe_symmetric() {
    let seeds: Vec<u64> = std::env::var("EP10_SEEDS")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once("..")?;
            Some((a.parse().ok()?..b.parse().ok()?).collect())
        })
        .unwrap_or_else(|| (1..11).collect());
    for seed in seeds {
        let mut sim = symmetric_battle(seed);
        let mut routs = [0usize; 2];
        let mut was = vec![false; sim.units().len()];
        let mut first_rout = [f64::NAN; 2];
        while !sim.is_finished() && sim.elapsed() < 1800.0 {
            sim.step();
            for (i, u) in sim.units().iter().enumerate() {
                let r = u.state == UnitState::Routing;
                if r && !was[i] {
                    if std::env::var("EP10_TRACE").is_ok() && sim.elapsed() < 300.0 {
                        println!(
                            "  {:.1} {:?} {} #{} at ({:.0},{:.0}) hp {:.0}",
                            sim.elapsed(),
                            u.side,
                            u.unit_type,
                            u.id,
                            u.x,
                            u.z,
                            u.hp
                        );
                    }
                    routs[u.side.index()] += 1;
                    if first_rout[u.side.index()].is_nan() {
                        first_rout[u.side.index()] = sim.elapsed();
                    }
                }
                was[i] = r;
            }
        }
        let count = |side: SideId, pred: &dyn Fn(&sim_battle::Unit) -> bool| {
            sim.units()
                .iter()
                .filter(|u| u.side == side && !u.synthetic && pred(u))
                .count()
        };
        let rallies = |side: SideId| {
            sim.events()
                .iter()
                .filter(|e| e.side == Some(side) && e.text_fr.contains("se rallient"))
                .count()
        };
        println!(
            "seed {seed:2}: {:?} at {:.0} s | att routs {} (first {:.0} s) rallies {} gone {} | def routs {} (first {:.0} s) rallies {} gone {}",
            sim.winner(),
            sim.elapsed(),
            routs[0],
            first_rout[0],
            rallies(SideId::Attacker),
            count(SideId::Attacker, &|u| u.left_field),
            routs[1],
            first_rout[1],
            rallies(SideId::Defender),
            count(SideId::Defender, &|u| u.left_field),
        );
    }
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

/// The small mixed battle of `b6::battles_without_a_site_are_unchanged`
/// (6 French regiments against 4 English, plains, no village, both AIs).
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

/// Probe (ignored): winners of the small mixed battle of `b6` over seeds
/// 0-63, and the routs of seed `EP10_B6_SEED` (default 3).
#[test]
#[ignore = "probe"]
fn probe_small_battle() {
    let mut french = 0;
    let mut durations = Vec::new();
    for seed in 0..64 {
        let mut sim = small_mixed_battle(seed);
        run_to_end(&mut sim);
        if sim.winner() == Some(SideId::Attacker) {
            french += 1;
        }
        durations.push(sim.elapsed());
    }
    durations.sort_by(f64::total_cmp);
    println!(
        "French win {french}/64, duration min {:.0} med {:.0} max {:.0}",
        durations[0], durations[32], durations[63]
    );
    let seed = std::env::var("EP10_B6_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let mut sim = small_mixed_battle(seed);
    run_to_end(&mut sim);
    for e in sim.events() {
        let t = &e.text_fr;
        if t.contains("déroute") || t.contains("rallient") || t.contains("brisée") {
            println!("{:5.0} {t}", e.time);
        }
    }
}
