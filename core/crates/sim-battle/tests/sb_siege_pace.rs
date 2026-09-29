//! SB (TW2, ADR 0107): pace of siege destruction per fortification level.
//! A full ram crew against an unguarded gate, one engine alone against a
//! front wall piece. The probe (`cargo test -p sim-battle --test
//! sb_siege_pace -- --ignored --nocapture`) prints the table recorded in the
//! ADR; the plain tests pin the targets of the spec at level 3.

mod common;

use common::*;
use data_model::GameData;
use sim_battle::{BattleSim, Command, SideId, SiegeFxKind, SiegeSetup};

const BESIEGERS: [&str; 2] = ["unit_men_at_arms_foot", "unit_urban_militia"];
const GARRISON: [&str; 2] = ["unit_urban_militia", "unit_crossbowmen"];
/// Longest measured assault, in seconds.
const LIMIT_S: f64 = 900.0;

fn siege(data: &GameData, extra: &[&str], fortification: u32) -> BattleSim {
    let mut attackers = units(data, &BESIEGERS);
    attackers.extend(units(data, extra));
    let setup = setup(
        attackers,
        units(data, &GARRISON),
        Some(SiegeSetup {
            fortification,
            breach: 0,
            engines: None,
        }),
    );
    let mut sim = BattleSim::new(setup, 7).unwrap();
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Defender);
    // Every defender far at the back of the town: nothing hinders the works.
    let (cx, cz) = sim.siege().unwrap().center;
    for i in 0..sim.units().len() {
        if sim.units()[i].side == SideId::Defender {
            place(
                &mut sim,
                i as u32,
                cx + (i as f64 - 6.0) * 12.0,
                cz + 100.0,
                0.0,
            );
        }
    }
    sim
}

/// One step with the attackers kept steady and at full crew: the wall
/// towers shoot the lone engine or ram, which would lose men and heart; the
/// spec targets the pace of a full crew.
fn steady_step(sim: &mut BattleSim) {
    for unit in sim.units_mut().iter_mut() {
        if unit.side == SideId::Attacker {
            unit.morale = unit.morale_cap;
            unit.hp = f64::from(unit.initial_soldiers);
        }
    }
    sim.step();
}

/// Seconds for a full ram crew placed against the gate to bring it down.
fn ram_seconds(data: &GameData, fortification: u32) -> f64 {
    let mut sim = siege(data, &[], fortification);
    let ram = sim.units().iter().position(|u| u.ram).unwrap() as u32;
    let works = sim.siege().unwrap().clone();
    let gate = &works.pieces[works.gate];
    let (mx, mz) = gate.midpoint();
    let (nx, nz) = gate.outward();
    let band = works.band();
    place(
        &mut sim,
        ram,
        mx + nx * (band + 1.0),
        mz + nz * (band + 1.0),
        (-nx).atan2(-nz),
    );
    let mut t = 0.0;
    while t < LIMIT_S {
        steady_step(&mut sim);
        t += sim_battle::DT;
        if !sim.siege().unwrap().pieces[works.gate].intact() {
            return t;
        }
    }
    f64::INFINITY
}

/// Seconds and shots for one engine to breach a front wall piece.
fn engine_pace(data: &GameData, fortification: u32, engine: &str) -> (f64, usize) {
    let mut sim = siege(data, &[engine], fortification);
    let id = sim.units().iter().position(|u| u.wall_breaker()).unwrap() as u32;
    let works = sim.siege().unwrap().clone();
    let piece = works.front_walls()[0];
    sim.issue_command(Command::TargetWall {
        units: vec![id],
        piece,
    })
    .unwrap();
    let (mx, mz) = works.pieces[piece].midpoint();
    let (nx, nz) = works.pieces[piece].outward();
    place(
        &mut sim,
        id,
        mx + nx * 150.0,
        mz + nz * 150.0,
        (-nx).atan2(-nz),
    );
    let mut t = 0.0;
    while t < LIMIT_S {
        steady_step(&mut sim);
        t += sim_battle::DT;
        if !sim.siege().unwrap().pieces[piece].intact() {
            break;
        }
    }
    let shots = sim
        .siege_fx()
        .iter()
        .filter(|fx| matches!(fx.kind, SiegeFxKind::EngineShot { piece: p, .. } if p == piece))
        .count();
    let t = if sim.siege().unwrap().pieces[piece].intact() {
        f64::INFINITY
    } else {
        t
    };
    (t, shots)
}

fn mmss(seconds: f64) -> String {
    if seconds.is_finite() {
        let s = seconds.round() as u64;
        format!("{}:{:02}", s / 60, s % 60)
    } else {
        "—".to_owned()
    }
}

#[test]
#[ignore = "probe: prints the pace table of ADR 0107"]
fn probe_siege_pace_per_level() {
    let data = data();
    let rules = sim_battle::SiegeWorkRules::bundled();
    println!("| Niveau | PV porte | Bélier | PV pan | Trébuchet | Bombarde | Mangonneau |");
    println!("|---|---|---|---|---|---|---|");
    for fort in 0..=5 {
        let (wall, gate) = rules.hp(fort);
        let ram = ram_seconds(&data, fort);
        let cell = |engine: &str| {
            let (t, n) = engine_pace(&data, fort, engine);
            format!("{} ({n} tirs)", mmss(t))
        };
        println!(
            "| {fort} | {gate:.0} | {} | {wall:.0} | {} | {} | {} |",
            mmss(ram),
            cell("unit_trebuchet"),
            cell("unit_bombard"),
            cell("unit_mangonel"),
        );
    }
}

/// Spec SB at level 3: the gate falls in 40-60 s to a full ram crew; a
/// trebuchet breaches a wall piece in 6-10 shots, a bombard faster than a
/// mangonel.
#[test]
fn level_three_meets_the_pace_targets() {
    let data = data();
    let ram = ram_seconds(&data, 3);
    assert!((40.0..=60.0).contains(&ram), "gate {ram:.0} s");
    let (_, trebuchet) = engine_pace(&data, 3, "unit_trebuchet");
    assert!((6..=10).contains(&trebuchet), "{trebuchet} trebuchet shots");
    let (bombard_s, _) = engine_pace(&data, 3, "unit_bombard");
    let (mangonel_s, _) = engine_pace(&data, 3, "unit_mangonel");
    assert!(bombard_s < mangonel_s, "{bombard_s:.0} vs {mangonel_s:.0}");
}

/// Level 5 holds 1.5-2 times as long as level 3; levels 0-1 fall fast.
#[test]
fn stronger_towns_hold_longer() {
    let data = data();
    let (ram3, ram5) = (ram_seconds(&data, 3), ram_seconds(&data, 5));
    let ratio = ram5 / ram3;
    assert!((1.4..=2.1).contains(&ratio), "gate ×{ratio:.2}");
    let (_, shots3) = engine_pace(&data, 3, "unit_trebuchet");
    let (_, shots5) = engine_pace(&data, 5, "unit_trebuchet");
    let ratio = shots5 as f64 / shots3 as f64;
    assert!((1.4..=2.1).contains(&ratio), "wall ×{ratio:.2}");
    assert!(ram_seconds(&data, 1) < 30.0);
    let (_, shots1) = engine_pace(&data, 1, "unit_trebuchet");
    assert!(shots1 <= 5, "{shots1} shots at level 1");
}

/// SB: the gate counts as under attack while a ram batters it, and a few
/// seconds after; a wall piece while an engine keeps shooting at it; an
/// untouched piece never.
#[test]
fn pieces_under_attack_are_flagged() {
    let data = data();
    let mut sim = siege(&data, &["unit_trebuchet"], 3);
    let works = sim.siege().unwrap().clone();
    assert!(works.pieces.iter().all(|p| !p.under_attack()));
    // Ram against the gate.
    let ram = sim.units().iter().position(|u| u.ram).unwrap() as u32;
    let gate = &works.pieces[works.gate];
    let (mx, mz) = gate.midpoint();
    let (nx, nz) = gate.outward();
    let band = works.band();
    place(
        &mut sim,
        ram,
        mx + nx * (band + 1.0),
        mz + nz * (band + 1.0),
        (-nx).atan2(-nz),
    );
    // Trebuchet against a front wall piece.
    let engine = sim.units().iter().position(|u| u.wall_breaker()).unwrap() as u32;
    let piece = works.front_walls()[0];
    sim.issue_command(Command::TargetWall {
        units: vec![engine],
        piece,
    })
    .unwrap();
    let (px, pz) = works.pieces[piece].midpoint();
    let (qx, qz) = works.pieces[piece].outward();
    place(
        &mut sim,
        engine,
        px + qx * 150.0,
        pz + qz * 150.0,
        (-qx).atan2(-qz),
    );
    for _ in 0..(20.0 / sim_battle::DT) as usize {
        steady_step(&mut sim);
    }
    let now = sim.siege().unwrap();
    assert!(now.pieces[now.gate].under_attack(), "ram at the gate");
    assert!(now.pieces[piece].under_attack(), "engine shooting");
    assert!(now.pieces[piece].hp < now.pieces[piece].max_hp);
    let untouched = (0..now.pieces.len())
        .find(|&p| p != piece && p != now.gate && now.pieces[p].hp == now.pieces[p].max_hp)
        .unwrap();
    assert!(!now.pieces[untouched].under_attack());
    // The ram leaves: the gate is no longer under attack a few seconds on.
    place(&mut sim, ram, mx + nx * 80.0, mz + nz * 80.0, 0.0);
    let settle = sim_battle::siege::UNDER_ATTACK_CONTACT_S + 1.0;
    for _ in 0..(settle / sim_battle::DT) as usize {
        steady_step(&mut sim);
    }
    let now = sim.siege().unwrap();
    assert!(!now.pieces[now.gate].under_attack(), "ram gone");
}

/// SB: rams and siege towers are listed with their crew as strength.
#[test]
fn siege_engines_report_their_strength() {
    use sim_battle::SiegeEngineKind;
    let data = data();
    let mut sim = siege(&data, &["unit_siege_tower"], 2);
    let engines = sim.siege_engines();
    assert!(engines.iter().any(|e| e.kind == SiegeEngineKind::Ram));
    assert!(engines.iter().any(|e| e.kind == SiegeEngineKind::Tower));
    for e in &engines {
        assert_eq!(e.side, SideId::Attacker);
        assert!((e.hp - e.max_hp).abs() < 1e-9);
    }
    let ram = engines
        .iter()
        .find(|e| e.kind == SiegeEngineKind::Ram)
        .unwrap()
        .unit;
    sim.units_mut()[ram as usize].hp *= 0.5;
    let hurt = sim
        .siege_engines()
        .into_iter()
        .find(|e| e.unit == ram)
        .unwrap();
    assert!((hurt.hp / hurt.max_hp - 0.5).abs() < 1e-9);
}
