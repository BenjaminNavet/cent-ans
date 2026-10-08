//! SG2: animated siege engines. The renderer winds a trebuchet back over
//! its reload and lets the stone fly on the shot: the reload it reads
//! (`Unit::reload`, `Unit::reload_period`) must match the time between two
//! shots of the engine.

mod common;

use common::*;
use sim_battle::shot::ENGINE_RELOAD;
use sim_battle::{BattleSim, Command, SiegeFxKind, SiegeSetup};

fn siege_with_trebuchet(seed: u64) -> (BattleSim, u32, usize) {
    let data = data();
    let attackers = units(data, &["unit_men_at_arms_foot", "unit_trebuchet"]);
    let defenders = units(data, &["unit_urban_militia", "unit_crossbowmen"]);
    let setup = setup(
        attackers,
        defenders,
        Some(SiegeSetup {
            fortification: 1,
            breach: 0,
            ..Default::default()
        }),
    );
    let mut sim = BattleSim::new(setup, seed).unwrap();
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
        mx + nx * 180.0,
        mz + nz * 180.0,
        (-nx).atan2(-nz),
    );
    (sim, trebuchet, piece)
}

#[test]
fn engine_reload_matches_the_time_between_shots() {
    let (mut sim, trebuchet, _) = siege_with_trebuchet(23);
    let unit = &sim.units()[trebuchet as usize];
    assert_eq!(unit.reload_period(), ENGINE_RELOAD);
    let mut shots = Vec::new();
    let mut reload_after_shot = Vec::new();
    let engine_shots = |sim: &BattleSim| {
        sim.siege_fx()
            .iter()
            .filter(
                |fx| matches!(fx.kind, SiegeFxKind::EngineShot { unit, .. } if unit == trebuchet),
            )
            .count()
    };
    for _ in 0..600 {
        let before = engine_shots(&sim);
        sim.step();
        if engine_shots(&sim) > before {
            shots.push(sim.elapsed());
            reload_after_shot.push(sim.units()[trebuchet as usize].reload);
        }
    }
    assert!(shots.len() >= 3, "{} shots", shots.len());
    for pair in shots.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(
            (gap - ENGINE_RELOAD).abs() < 0.25,
            "a trebuchet shoots every {ENGINE_RELOAD} s: {gap}"
        );
    }
    for reload in reload_after_shot {
        assert!((reload - ENGINE_RELOAD).abs() < 1e-9, "{reload}");
    }
}

#[test]
fn every_engine_shot_is_also_a_core_shot_event() {
    let (mut sim, trebuchet, piece) = siege_with_trebuchet(29);
    run(&mut sim, 40.0);
    let fx_times: Vec<f64> = sim
        .siege_fx()
        .iter()
        .filter_map(|fx| match fx.kind {
            SiegeFxKind::EngineShot { unit, piece: p, .. } if unit == trebuchet => {
                assert_eq!(p, piece);
                Some(fx.time)
            }
            _ => None,
        })
        .collect();
    let shot_times: Vec<f64> = sim
        .take_shots()
        .iter()
        .filter(|s| s.shooter == trebuchet)
        .map(|s| s.time)
        .collect();
    assert!(!fx_times.is_empty());
    assert_eq!(fx_times, shot_times, "the sling lets go on the core's shot");
}
