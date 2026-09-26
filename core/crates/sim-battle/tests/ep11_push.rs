//! EP11 (ADR 0070): continuous push of the lines in melee.
//!
//! Lab scenarios on a flat, bare field, both AIs off: two regiments put in
//! contact front to front fight where they stand. A heavier line drives a
//! lighter one back a few metres per ten seconds and follows it; the files
//! of a wide regiment wrap round a narrow one; a regiment pushed against a
//! friend or the edge of the field cannot give ground and compresses.
//!
//! `cargo test --release -p sim-battle --test ep11_push -- --ignored --nocapture probe`

mod common;

use common::*;
use sim_battle::{BattleSim, Formation, SideId, Unit, UnitState, DT};

const FRONT_Z: f64 = 400.0;
const X: f64 = 600.0;

/// Flat bare field with the attackers (facing +z) and the defenders (facing
/// −z) given; all regiments are set aside at first (placed by the caller).
fn lab_field(attackers: &[&str], defenders: &[&str]) -> BattleSim {
    let data = data();
    let mut sim = BattleSim::new(
        setup(units(&data, attackers), units(&data, defenders), None),
        1,
    )
    .unwrap();
    lab(&mut sim);
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

/// Puts the attacker `a` and the defender `d` front to front at `FRONT_Z`
/// (a gap of `gap` metres between the two rectangles).
fn face_off(sim: &mut BattleSim, a: u32, d: u32, x: f64, gap: f64) {
    let (_, depth_a) = sim.units()[a as usize].extent();
    let (_, depth_d) = sim.units()[d as usize].extent();
    place(sim, a, x, FRONT_Z - depth_a * 0.5, 0.0);
    place(
        sim,
        d,
        x,
        FRONT_Z + gap + depth_d * 0.5,
        std::f64::consts::PI,
    );
    // Each holds on to the other (a column shortened by its losses opens a
    // gap in front of it).
    sim.units_mut()[a as usize].target = Some(d);
    sim.units_mut()[d as usize].target = Some(a);
}

/// Steady morale: this lab is about the push, not the rout.
fn steady(sim: &mut BattleSim) {
    for unit in sim.units_mut() {
        unit.morale = 100.0;
        unit.morale_cap = 100.0;
    }
}

fn id_of(sim: &BattleSim, side: SideId, k: usize) -> u32 {
    sim.units()
        .iter()
        .filter(|u| u.side == side)
        .nth(k)
        .unwrap()
        .id
}

fn unit(sim: &BattleSim, id: u32) -> &Unit {
    &sim.units()[id as usize]
}

/// Men-at-arms on foot against urban militia, front to front; returns the
/// sim, the attacker and the defender ids.
fn heavy_against_light() -> (BattleSim, u32, u32) {
    let mut sim = lab_field(&["unit_men_at_arms_foot"], &["unit_urban_militia"]);
    let (a, d) = (
        id_of(&sim, SideId::Attacker, 0),
        id_of(&sim, SideId::Defender, 0),
    );
    face_off(&mut sim, a, d, X, 0.3);
    steady(&mut sim);
    (sim, a, d)
}

/// Keeps the morale steady every step (the lab measures the push only).
fn run_steady(sim: &mut BattleSim, seconds: f64) {
    let steps = (seconds / DT).round() as usize;
    for _ in 0..steps {
        steady(sim);
        sim.step();
    }
}

#[test]
fn a_heavy_line_drives_a_light_one_back() {
    let (mut sim, a, d) = heavy_against_light();
    let z0 = unit(&sim, d).z;
    let za0 = unit(&sim, a).z;
    run_steady(&mut sim, 20.0);
    let (ua, ud) = (unit(&sim, a), unit(&sim, d));
    assert_eq!(ud.state, UnitState::Melee);
    let recoil = ud.z - z0;
    // A few metres per ten seconds: a slow, continuous recoil.
    assert!(
        (2.0..=9.0).contains(&recoil),
        "militia gave {recoil:.2} m in 20 s"
    );
    assert!((ud.push.ground_lost - recoil).abs() < 0.5);
    // The men-at-arms follow and stay in contact.
    let advance = ua.z - za0;
    assert!(
        (advance - recoil).abs() < 0.5,
        "advance {advance:.2} vs recoil {recoil:.2}"
    );
    assert!(ua.push.speed > 0.0 && ud.push.speed < 0.0);
    // The front of the pusher bulges, the front of the pushed dents.
    assert!(ua.push.bulge > 0.5, "bulge {}", ua.push.bulge);
    assert!(ud.push.bulge < 0.0, "dent {}", ud.push.bulge);
}

#[test]
fn without_push_the_lines_stand() {
    let (mut sim, _a, d) = heavy_against_light();
    let mut rules = sim.push_rules().clone();
    rules.pressure.max_speed_mps = 0.0;
    sim.set_push_rules(rules);
    let z0 = unit(&sim, d).z;
    run_steady(&mut sim, 20.0);
    assert!((unit(&sim, d).z - z0).abs() < 0.05);
}

#[test]
fn equal_lines_hold_each_other() {
    let mut sim = lab_field(&["unit_men_at_arms_foot"], &["unit_men_at_arms_foot"]);
    let (a, d) = (
        id_of(&sim, SideId::Attacker, 0),
        id_of(&sim, SideId::Defender, 0),
    );
    face_off(&mut sim, a, d, X, 0.3);
    let (za, zd) = (unit(&sim, a).z, unit(&sim, d).z);
    run_steady(&mut sim, 20.0);
    assert!((unit(&sim, a).z - za).abs() < 0.3);
    assert!((unit(&sim, d).z - zd).abs() < 0.3);
}

#[test]
fn the_push_is_deterministic() {
    let run_once = || {
        let (mut sim, a, d) = heavy_against_light();
        run_steady(&mut sim, 15.0);
        (
            unit(&sim, a).z.to_bits(),
            unit(&sim, d).z.to_bits(),
            unit(&sim, d).hp.to_bits(),
        )
    };
    assert_eq!(run_once(), run_once());
}

/// Men-at-arms in line (22 m) against militia in column (6.6 m wide).
fn wide_against_narrow(bonus: Option<f64>) -> (BattleSim, u32, u32) {
    let mut sim = lab_field(&["unit_men_at_arms_foot"], &["unit_urban_militia"]);
    let (a, d) = (
        id_of(&sim, SideId::Attacker, 0),
        id_of(&sim, SideId::Defender, 0),
    );
    sim.units_mut()[d as usize].formation = Formation::Column;
    face_off(&mut sim, a, d, X, 0.3);
    steady(&mut sim);
    if let Some(bonus) = bonus {
        let mut rules = sim.push_rules().clone();
        rules.wrap.flank_damage_bonus = bonus;
        sim.set_push_rules(rules);
    }
    (sim, a, d)
}

#[test]
fn wide_files_wrap_round_a_narrow_regiment() {
    let (mut sim, a, d) = wide_against_narrow(None);
    let flat: Vec<[f64; 4]> = sim.soldier_transforms(SideId::Attacker, None);
    run_steady(&mut sim, 25.0);
    let ua = unit(&sim, a);
    let ud = unit(&sim, d);
    assert_eq!(
        ua.push.wrap_of,
        Some(d),
        "a ({:.1},{:.1}) {:?} d ({:.1},{:.1}) {:?} {:?}",
        ua.x,
        ua.z,
        ua.state,
        ud.x,
        ud.z,
        ud.state,
        ud.extent()
    );
    assert!(
        ua.push.wrap[0] > 0.9 && ua.push.wrap[1] > 0.9,
        "wrap {:?}",
        ua.push.wrap
    );
    // The overhanging files swung forward: the outermost figures now stand
    // well ahead of the front rank, beside the narrow column.
    let front = |poses: &[[f64; 4]], ua: &Unit| {
        poses
            .iter()
            .map(|p| {
                let (fx, fz) = ua.forward();
                (p[0] - ua.x) * fx + (p[2] - ua.z) * fz
            })
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let bent: Vec<[f64; 4]> = sim.soldier_transforms(SideId::Attacker, None);
    let ahead_before = front(&flat, unit(&sim, a));
    let ahead_after = front(&bent, unit(&sim, a));
    assert!(
        ahead_after > ahead_before + 4.0,
        "front {ahead_before:.1} -> {ahead_after:.1}"
    );
    // And they strike the column's flanks.
    let (mut plain, _, _) = wide_against_narrow(Some(0.0));
    run_steady(&mut plain, 25.0);
    let lost_wrapped = unit(&sim, d).initial_soldiers as f64 - unit(&sim, d).hp;
    let lost_plain = unit(&plain, d).initial_soldiers as f64 - unit(&plain, d).hp;
    assert!(
        lost_wrapped > lost_plain * 1.1,
        "wrapped {lost_wrapped:.1} vs plain {lost_plain:.1}"
    );
}

/// Heavy against light, with a second militia regiment just behind the
/// first (1 m) when `backed`.
fn pushed_against_a_friend(backed: bool) -> (BattleSim, u32, u32) {
    let mut sim = lab_field(
        &["unit_men_at_arms_foot"],
        &["unit_urban_militia", "unit_urban_militia"],
    );
    let (a, d) = (
        id_of(&sim, SideId::Attacker, 0),
        id_of(&sim, SideId::Defender, 0),
    );
    let b = id_of(&sim, SideId::Defender, 1);
    face_off(&mut sim, a, d, X, 0.3);
    let (_, depth) = unit(&sim, d).extent();
    let zd = unit(&sim, d).z;
    let (bx, bz) = if backed {
        (X, zd + depth + 1.0)
    } else {
        (X + 300.0, zd + 300.0)
    };
    place(&mut sim, b, bx, bz, std::f64::consts::PI);
    steady(&mut sim);
    (sim, a, d)
}

#[test]
fn a_line_pushed_against_a_friend_compresses() {
    let (mut sim, _a, d) = pushed_against_a_friend(true);
    let z0 = unit(&sim, d).z;
    run_steady(&mut sim, 20.0);
    let ud = unit(&sim, d);
    assert!(
        (ud.z - z0).abs() < 1.0,
        "gave {:.2} m against a friend",
        ud.z - z0
    );
    assert!(
        ud.push.compression > 0.5,
        "compression {}",
        ud.push.compression
    );
    let lost_backed = f64::from(ud.initial_soldiers) - ud.hp;
    // Free to give ground, the same regiment suffers less.
    let (mut free, _, d2) = pushed_against_a_friend(false);
    run_steady(&mut free, 20.0);
    let uf = unit(&free, d2);
    assert!(uf.push.compression < 0.05);
    let lost_free = f64::from(uf.initial_soldiers) - uf.hp;
    assert!(
        lost_backed > lost_free * 1.1,
        "backed {lost_backed:.1} vs free {lost_free:.1}"
    );
}

#[test]
fn a_line_pushed_against_the_field_edge_compresses() {
    let (mut sim, a, d) = heavy_against_light();
    let depth_field = sim.field().depth;
    let (_, depth_d) = unit(&sim, d).extent();
    let (_, depth_a) = unit(&sim, a).extent();
    // The militia's centre 0.5 m from the edge clamp.
    let zd = depth_field - 1.5;
    place(&mut sim, d, X, zd, std::f64::consts::PI);
    place(
        &mut sim,
        a,
        X,
        zd - depth_d * 0.5 - 0.3 - depth_a * 0.5,
        0.0,
    );
    run_steady(&mut sim, 20.0);
    let ud = unit(&sim, d);
    assert!(ud.z <= depth_field - 1.0 + 1e-9);
    assert!(
        ud.push.compression > 0.5,
        "compression {}",
        ud.push.compression
    );
}

/// Probe (ignored): ground given and shape over time, heavy against light.
#[test]
#[ignore = "probe"]
fn probe() {
    let (mut sim, a, d) = heavy_against_light();
    let z0 = unit(&sim, d).z;
    for t in 0..=12 {
        let (ua, ud) = (unit(&sim, a), unit(&sim, d));
        println!(
            "t {:>3} s  recoil {:>5.2} m  speed {:>5.2}  bulge a {:>5.2} d {:>5.2}  hp a {:>5.1} d {:>5.1}",
            t * 5,
            ud.z - z0,
            ud.push.speed,
            ua.push.bulge,
            ud.push.bulge,
            ua.hp,
            ud.hp
        );
        run_steady(&mut sim, 5.0);
    }
}

/// Probe (ignored): a historical map (`EP11_MAP`, `EP11_SEED`; `EP11_OFF=1`
/// without push): every 30 s, the regiments in melee with their push state.
#[test]
#[ignore = "probe"]
fn probe_historical() {
    let id = std::env::var("EP11_MAP").unwrap_or_else(|_| "azincourt".to_owned());
    let seed = std::env::var("EP11_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/battle_maps")
        .join(format!("{id}.json"));
    let map =
        sim_battle::HistoricalMap::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    let data = data();
    let setup = map
        .battle_setup(
            &data.unit_types,
            data.battle_orders.values().cloned().collect(),
            Some(data.battle_standard_rules.clone()),
            None,
        )
        .unwrap();
    let mut sim = map.start(setup, seed).unwrap();
    if std::env::var("EP11_OFF").is_ok() {
        let mut rules = sim.push_rules().clone();
        rules.pressure.max_speed_mps = 0.0;
        rules.wrap.flank_damage_bonus = 0.0;
        sim.set_push_rules(rules);
    }
    let every: f64 = std::env::var("EP11_EVERY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30.0);
    while !sim.is_finished() && sim.elapsed() < 2400.0 {
        run(&mut sim, every);
        println!(
            "t={:.0} str {} / {}",
            sim.elapsed(),
            sim.strength(SideId::Attacker),
            sim.strength(SideId::Defender)
        );
        for u in sim.units().iter().filter(|u| u.state == UnitState::Melee) {
            println!(
                "   {} {:>2} {:<24} hp {:>5.0} mor {:>3.0} ({:>4.0},{:>4.0}) spd {:>5.2} lost {:>5.1} comp {:.2} wrap {:.2}/{:.2} of {:?}",
                u.side.key(),
                u.id,
                u.unit_type,
                u.hp,
                u.morale,
                u.x,
                u.z,
                u.push.speed,
                u.push.ground_lost,
                u.push.compression,
                u.push.wrap[0],
                u.push.wrap[1],
                u.push.wrap_of
            );
        }
    }
    for e in sim.events() {
        println!("{:5.0} {}", e.time, e.text_fr);
    }
}

/// Probe (ignored): cost of a step at the EP1 scale (60 regiments of 120 men
/// a side, 14 400 soldiers, both AIs), with and without the push: mean ms per
/// tick over each 60 s window, with the number of regiments in melee.
#[test]
#[ignore = "probe"]
fn probe_step_cost() {
    const KINDS: [&str; 5] = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_knights",
        "unit_urban_militia",
        "unit_crossbowmen",
    ];
    let data = data();
    let army: Vec<&str> = (0..60).map(|i| KINDS[i % KINDS.len()]).collect();
    for push in [false, true] {
        let mut battle = setup(units(&data, &army), units(&data, &army), None);
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
        if !push {
            let mut rules = sim.push_rules().clone();
            rules.pressure.max_speed_mps = 0.0;
            rules.wrap.grow_per_second = 0.0;
            sim.set_push_rules(rules);
        }
        let mut total = 0.0;
        let mut ticks = 0u32;
        let mut worst: f64 = 0.0;
        let window = (60.0 / DT).round() as u32;
        while !sim.is_finished() && sim.elapsed() < 600.0 {
            let mut elapsed = 0.0;
            let mut melee_peak = 0;
            for _ in 0..window {
                let start = std::time::Instant::now();
                sim.step();
                elapsed += start.elapsed().as_secs_f64();
                melee_peak = melee_peak.max(
                    sim.units()
                        .iter()
                        .filter(|u| u.state == UnitState::Melee)
                        .count(),
                );
            }
            let ms = elapsed * 1000.0 / f64::from(window);
            total += elapsed * 1000.0;
            ticks += window;
            worst = worst.max(ms);
            println!(
                "push {push}: t {:>4.0} s  {ms:.3} ms/tick  melee peak {melee_peak}",
                sim.elapsed()
            );
        }
        println!(
            "push {push}: mean {:.3} ms/tick, worst window {worst:.3} ms/tick",
            total / f64::from(ticks)
        );
    }
}
