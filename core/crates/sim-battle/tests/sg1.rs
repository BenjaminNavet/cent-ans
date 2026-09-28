//! SG1: siege assault events for the renderer, the ram's blows, boiling oil
//! and the garrison falling back to the square (`docs/wip/sg1-sieges.md`).

mod common;

use common::*;
use data_model::GameData;
use sim_battle::siege_fx::{OIL_PERIOD, RAM_PERIOD};
use sim_battle::{BattleSim, Command, SideId, SiegeFxKind, SiegeSetup, UnitState};

const BESIEGERS: [&str; 4] = [
    "unit_men_at_arms_foot",
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_longbowmen",
];
const GARRISON: [&str; 5] = [
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_crossbowmen",
    "unit_crossbowmen",
    "unit_men_at_arms_foot",
];

fn siege(data: &GameData, extra: &[&str], fortification: u32, seed: u64) -> BattleSim {
    let mut attackers = units(data, &BESIEGERS);
    attackers.extend(units(data, extra));
    let setup = setup(
        attackers,
        units(data, &GARRISON),
        Some(SiegeSetup {
            fortification,
            breach: 0,
        }),
    );
    BattleSim::new(setup, seed).unwrap()
}

fn ram_id(sim: &BattleSim) -> u32 {
    sim.units().iter().position(|u| u.ram).unwrap() as u32
}

/// Outer point `offset` metres in front of the gate.
fn before_gate(sim: &BattleSim, offset: f64) -> (f64, f64) {
    let works = sim.siege().unwrap();
    let gate = &works.pieces[works.gate];
    let (mx, mz) = gate.midpoint();
    let (nx, nz) = gate.outward();
    (mx + nx * offset, mz + nz * offset)
}

/// Sends every defender far to the back of the town (no gate guard).
fn clear_the_gate(sim: &mut BattleSim) {
    let (cx, cz) = sim.siege().unwrap().center;
    for i in 0..sim.units().len() {
        if sim.units()[i].side == SideId::Defender {
            place(sim, i as u32, cx + (i as f64 - 6.0) * 12.0, cz + 100.0, 0.0);
        }
    }
}

#[test]
fn the_ram_strikes_the_gate_in_rhythm() {
    let data = data();
    // SB (ADR 0100): level 5, so that the gate outlasts 30 s of blows.
    let mut sim = siege(&data, &[], 5, 17);
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Defender);
    clear_the_gate(&mut sim);
    let ram = ram_id(&sim);
    let band = sim.siege().unwrap().band();
    let (x, z) = before_gate(&sim, band + 1.0);
    place(&mut sim, ram, x, z, 0.0);
    let gate = sim.siege().unwrap().gate;
    let hp0 = sim.siege().unwrap().pieces[gate].hp;
    let mut last_hp = hp0;
    let mut changes = 0;
    for _ in 0..300 {
        sim.step();
        let hp = sim.siege().unwrap().pieces[gate].hp;
        if hp < last_hp {
            changes += 1;
        }
        last_hp = hp;
    }
    let strikes: Vec<f64> = sim
        .siege_fx()
        .iter()
        .filter(|fx| matches!(fx.kind, SiegeFxKind::RamStrike { unit, .. } if unit == ram))
        .map(|fx| fx.time)
        .collect();
    assert!(
        (9..=10).contains(&strikes.len()),
        "30 s at the gate: {} blows",
        strikes.len()
    );
    assert_eq!(changes, strikes.len(), "the gate only suffers the blows");
    for pair in strikes.windows(2) {
        assert!(((pair[1] - pair[0]) - RAM_PERIOD).abs() < 0.05, "{pair:?}");
    }
    // Same wear per second as a continuous ram (`data/rules/siege_works.json`, full crew).
    let per_s = sim_battle::SiegeWorkRules::bundled().ram.damage_per_s;
    let lost = hp0 - last_hp;
    assert!(
        (lost - per_s * RAM_PERIOD * strikes.len() as f64).abs() < 1.0,
        "{lost}"
    );
}

#[test]
fn an_engine_shot_strikes_a_point_on_its_wall() {
    let data = data();
    // SB (ADR 0100): level 5, so that the piece outlasts a minute of shots.
    let mut sim = siege(&data, &["unit_trebuchet"], 5, 19);
    lab(&mut sim);
    let works = sim.siege().unwrap().clone();
    let piece = works.front_walls()[0];
    let trebuchet = sim.units().iter().position(|u| u.wall_breaker()).unwrap() as u32;
    sim.issue_command(Command::TargetWall {
        units: vec![trebuchet],
        piece,
    })
    .unwrap();
    let (mx, mz) = works.pieces[piece].midpoint();
    let (nx, nz) = works.pieces[piece].outward();
    place(
        &mut sim,
        trebuchet,
        mx + nx * 200.0,
        mz + nz * 200.0,
        (-nx).atan2(-nz),
    );
    run(&mut sim, 60.0);
    let shots: Vec<_> = sim
        .siege_fx()
        .iter()
        .filter_map(|fx| match fx.kind {
            SiegeFxKind::EngineShot {
                unit,
                piece: p,
                x,
                z,
                height,
                ..
            } => Some((unit, p, x, z, height)),
            _ => None,
        })
        .collect();
    assert!(shots.len() >= 3, "{} shots in a minute", shots.len());
    for &(unit, p, x, z, height) in &shots {
        assert_eq!(unit, trebuchet);
        assert_eq!(p, piece);
        assert!(works.pieces[piece].distance(x, z) < 0.5, "on the wall");
        assert!((0.0..=1.0).contains(&height));
    }
    // Not all stones strike the same spot.
    assert!(shots.windows(2).any(|w| (w[0].2 - w[1].2).abs() > 0.5));
}

#[test]
fn ladders_are_raised_then_the_wall_walk_is_gained() {
    let data = data();
    let mut sim = siege(&data, &[], 2, 13);
    lab(&mut sim);
    clear_the_gate(&mut sim);
    let works = sim.siege().unwrap().clone();
    let piece = works.front_walls()[0];
    let p = &works.pieces[piece];
    let (mx, mz) = p.midpoint();
    let (nx, nz) = p.outward();
    place(
        &mut sim,
        1,
        mx + nx * 12.0,
        mz + nz * 12.0,
        (-nx).atan2(-nz),
    );
    sim.issue_command(Command::Move {
        units: vec![1],
        x: mx - nx * 30.0,
        z: mz - nz * 30.0,
        run: false,
        facing: None,
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
    })
    .unwrap();
    // Halfway up: climbers on the ladders, a few men already on top.
    run(&mut sim, 30.0);
    let unit = &sim.units()[1];
    assert!(unit.climb_progress > 0.3 && unit.climb_progress < 0.9);
    let ladders = sim.ladders(unit);
    assert!(ladders.len() >= 2);
    let wall = &works.pieces[piece];
    for l in &ladders {
        assert!(wall.outside_offset(l.foot.0, l.foot.1) > wall.outside_offset(l.top.0, l.top.1));
        assert!(l.top_y > l.foot_y + works.wall_height * 0.8);
        for t in &works.towers {
            let d = ((l.top.0 - t.x).powi(2) + (l.top.1 - t.z).powi(2)).sqrt();
            assert!(d > t.radius, "ladder inside a tower");
        }
    }
    assert!(
        ladders.windows(2).all(|w| w[0].foot != w[1].foot),
        "ladders side by side"
    );
    let shown = sim.climbers_shown(unit, 1.0);
    let poses = sim.soldier_poses(unit, 1.0);
    assert_eq!(poses.len(), unit.soldiers() as usize);
    let ground = sim.field().height(mx, mz);
    let on_rungs = poses[..shown]
        .iter()
        .filter(|p| p[1] > ground + 0.5 && p[1] < ground + works.wall_height + 1.0)
        .count();
    assert!(on_rungs * 2 >= shown, "{on_rungs}/{shown} up the ladders");
    let on_top = poses[shown..]
        .iter()
        .filter(|p| p[1] > ground + works.wall_height - 0.5)
        .count();
    assert!(on_top > 0 && on_top < poses.len() - shown);
    // BV1: with fewer figures than soldiers, one pose per figure.
    let half = sim.soldier_poses(unit, 0.5);
    assert_eq!(half.len(), unit.figure_count(0.5) as usize);
    assert!(sim.climbers_shown(unit, 0.5) <= half.len());
    run(&mut sim, 60.0);
    let fx = sim.siege_fx();
    let raised = fx
        .iter()
        .position(
            |f| matches!(f.kind, SiegeFxKind::LaddersRaised { unit: 1, piece: q } if q == piece),
        )
        .expect("ladders raised");
    let on_wall = fx
        .iter()
        .position(|f| matches!(f.kind, SiegeFxKind::OnWall { unit: 1, piece: q } if q == piece))
        .expect("wall walk gained");
    assert!(raised < on_wall);
    assert!(
        fx[on_wall].time - fx[raised].time > 30.0,
        "ladders are slow"
    );
}

#[test]
fn a_siege_tower_docks_and_lowers_its_bridge() {
    let data = data();
    let mut sim = siege(&data, &["unit_siege_tower"], 2, 13);
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Defender);
    let tower = sim.units().iter().position(|u| u.siege_tower()).unwrap() as u32;
    let works = sim.siege().unwrap().clone();
    let piece = works.front_walls()[0];
    let (mx, mz) = works.pieces[piece].midpoint();
    let (nx, nz) = works.pieces[piece].outward();
    place(
        &mut sim,
        tower,
        mx + nx * 40.0,
        mz + nz * 40.0,
        (-nx).atan2(-nz),
    );
    let band = works.band();
    sim.issue_command(Command::Move {
        units: vec![tower],
        x: mx + nx * (band + 2.0),
        z: mz + nz * (band + 2.0),
        run: false,
        facing: None,
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
    })
    .unwrap();
    run(&mut sim, 120.0);
    assert!(sim.siege_fx().iter().any(
        |f| matches!(f.kind, SiegeFxKind::TowerDocked { unit, piece: q } if unit == tower && q == piece)
    ));
    assert_eq!(sim.siege().unwrap().pieces[piece].docked_tower, Some(tower));
}

#[test]
fn boiling_oil_scalds_the_attackers_at_the_gate_only_when_guarded() {
    let data = data();
    let oil_losses = |guarded: bool| -> (usize, u32, u32) {
        let mut sim = siege(&data, &[], 2, 41);
        lab(&mut sim);
        hold_fire(&mut sim, SideId::Defender);
        clear_the_gate(&mut sim);
        let band = sim.siege().unwrap().band();
        if guarded {
            // A militia just behind the gate.
            let (x, z) = before_gate(&sim, -(band + 12.0));
            let guard = sim
                .units()
                .iter()
                .position(|u| u.side == SideId::Defender)
                .unwrap() as u32;
            place(&mut sim, guard, x, z, 0.0);
        }
        // Militia 2 at the foot of the gate, militia 1 far off.
        let (x, z) = before_gate(&sim, band + 5.0);
        place(&mut sim, 2, x, z, std::f64::consts::PI);
        let (x, z) = before_gate(&sim, 120.0);
        place(&mut sim, 1, x, z, std::f64::consts::PI);
        let before = (sim.units()[2].soldiers(), sim.units()[1].soldiers());
        run(&mut sim, OIL_PERIOD * 2.0 + 1.0);
        let pots = sim
            .siege_fx()
            .iter()
            .filter(|f| matches!(&f.kind, SiegeFxKind::BoilingOil { targets, .. } if targets.contains(&2)))
            .count();
        (
            pots,
            before.0 - sim.units()[2].soldiers(),
            before.1 - sim.units()[1].soldiers(),
        )
    };
    let (pots, near, far) = oil_losses(true);
    assert_eq!(pots, 2, "a pot every {OIL_PERIOD} s");
    let (no_pots, near_unguarded, _) = oil_losses(false);
    assert_eq!(no_pots, 0, "no guard, no oil");
    // The towers shoot at both regiments either way: the oil is the difference.
    assert!(
        near >= near_unguarded + 3,
        "scalded: {near} vs {near_unguarded} without oil"
    );
    assert!(far <= near_unguarded + 1, "far off: {far}");
}

#[test]
fn the_gate_gives_way_and_the_garrison_falls_back_to_the_square() {
    let data = data();
    let mut garrison = units(&data, &GARRISON);
    garrison.extend(units(&data, &GARRISON));
    let setup = setup(
        units(&data, &BESIEGERS),
        garrison,
        Some(SiegeSetup {
            fortification: 1,
            breach: 0,
        }),
    );
    let mut sim = BattleSim::new(setup, 43).unwrap();
    sim.set_ai(SideId::Attacker, false);
    sim.set_end_conditions(false);
    let ram = ram_id(&sim);
    let band = sim.siege().unwrap().band();
    let (x, z) = before_gate(&sim, band + 1.0);
    place(&mut sim, ram, x, z, 0.0);
    let gate = sim.siege().unwrap().gate;
    sim.siege_mut().unwrap().pieces[gate].hp = 10.0;
    run(&mut sim, 10.0);
    let fx = sim.siege_fx();
    assert!(fx
        .iter()
        .any(|f| matches!(f.kind, SiegeFxKind::RamStrike { breached: true, .. })));
    assert!(fx
        .iter()
        .any(|f| f.kind == SiegeFxKind::GateBroken { piece: gate }));
    assert!(fx.iter().any(|f| f.kind == SiegeFxKind::DefendersFallBack));
    assert!(has_event(&sim, "se replie sur la place"));
    // The reserve gathers on the square, a few regiments block the gate.
    run(&mut sim, 90.0);
    let works = sim.siege().unwrap();
    let in_square = sim
        .units()
        .iter()
        .filter(|u| {
            u.side == SideId::Defender && u.able() && !u.on_wall && works.in_square(u.x, u.z)
        })
        .count();
    assert!(in_square >= 1, "nobody on the square");
    let (gx, gz) = works.pieces[gate].midpoint();
    let at_gate = sim
        .units()
        .iter()
        .filter(|u| {
            u.side == SideId::Defender
                && u.able()
                && !u.on_wall
                && ((u.x - gx).powi(2) + (u.z - gz).powi(2)).sqrt() < 45.0
        })
        .count();
    assert!(at_gate <= sim_battle::CaptureRules::bundled().fall_back.gate_blockers);
}

#[test]
fn tower_volleys_are_reported_and_the_events_are_deterministic() {
    let data = data();
    let play = || {
        let mut sim = siege(&data, &["unit_trebuchet", "unit_siege_tower"], 2, 47);
        run(&mut sim, 600.0);
        sim.siege_fx().to_vec()
    };
    let a = play();
    let b = play();
    assert_eq!(a, b);
    assert!(a
        .iter()
        .any(|f| matches!(f.kind, SiegeFxKind::TowerVolley { .. })));
    assert!(a
        .iter()
        .any(|f| matches!(f.kind, SiegeFxKind::EngineShot { .. })));
    assert!(a
        .iter()
        .any(|f| matches!(f.kind, SiegeFxKind::RamStrike { .. })));
    // take_new_siege_fx hands each event over once.
    let mut sim = siege(&data, &[], 2, 47);
    run(&mut sim, 120.0);
    let first = sim.take_new_siege_fx();
    assert_eq!(first.len(), sim.siege_fx().len());
    assert!(sim.take_new_siege_fx().is_empty());
    let _ = UnitState::Idle;
}

/// BR3b: an attacker regiment already inside the walls while no gate or
/// breach is open (it came over by the ladders) makes for the square
/// instead of standing on its ladder point, which it has passed.
#[test]
fn an_attacker_over_the_wall_makes_for_the_square() {
    let data = data();
    let mut sim = siege(&data, &[], 2, 11);
    if sim.is_deploying() {
        sim.start_battle().expect("start");
    }
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, false);
    clear_the_gate(&mut sim);
    let (cx, cz) = sim.siege().unwrap().center;
    let (px, pz) = {
        let works = sim.siege().unwrap();
        let p = works.front_walls()[0];
        let (mx, mz) = works.pieces[p].midpoint();
        let (nx, nz) = works.pieces[p].outward();
        (mx - nx * 25.0, mz - nz * 25.0)
    };
    // The men-at-arms (id 0) stand on the ladder point, inside the walls.
    place(&mut sim, 0, px, pz, 0.0);
    assert!(sim.siege().unwrap().inside(px, pz));
    assert!(sim.siege().unwrap().openings().is_empty());
    let before = (px - cx).hypot(pz - cz);
    run(&mut sim, 40.0);
    let u = &sim.units()[0];
    assert!(sim.siege().unwrap().openings().is_empty());
    let after = (u.x - cx).hypot(u.z - cz);
    assert!(
        after < before - 20.0,
        "towards the square: {before:.0} m -> {after:.0} m"
    );
}
