//! BV1 tests: shot events for the renderer, visual unit-size figures.

use crate::common;

use common::*;
use sim_battle::{BattleSim, Formation, MissileKind, ShotCover};

fn duel() -> BattleSim {
    let data = data();
    let battle = setup(
        units(data, &["unit_longbowmen"]),
        units(data, &["unit_genoese_crossbowmen"]),
        None,
    );
    let mut sim = BattleSim::new(battle, 3).unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 500.0, 300.0, 0.0);
    place(&mut sim, 1, 500.0, 420.0, std::f64::consts::PI);
    sim
}

#[test]
fn volleys_are_reported_with_kind_cover_and_kills() {
    let mut sim = duel();
    let before: u32 = sim.units().iter().map(|u| u.soldiers()).sum();
    run(&mut sim, 30.0);
    let shots = sim.take_shots();
    assert!(!shots.is_empty(), "both sides shoot within 30 s");
    let bows: Vec<_> = shots.iter().filter(|s| s.shooter == 0).collect();
    let crossbows: Vec<_> = shots.iter().filter(|s| s.shooter == 1).collect();
    assert!(!bows.is_empty() && !crossbows.is_empty());
    assert!(bows.iter().all(|s| s.kind == MissileKind::Arrow));
    assert!(crossbows.iter().all(|s| s.kind == MissileKind::Bolt));
    assert!(bows.iter().all(|s| s.target == Some(1)));
    // The Genoese shoot from behind their pavises.
    assert!(bows.iter().all(|s| s.cover == ShotCover::Pavise));
    assert!(shots.iter().all(|s| s.missiles > 0 && !s.incendiary));
    let first = bows[0];
    assert!((first.aim.1 - 420.0).abs() < 1e-6 && (first.from.1 - 300.0).abs() < 1e-6);
    // The recorded kills are the losses of the duel (the hp pool is fractional).
    let after: f64 = sim.units().iter().map(|u| u.hp).sum();
    let kills: f64 = shots.iter().map(|s| s.kills).sum();
    assert!((f64::from(before) - after - kills).abs() < 1e-6);
    // Read once: the next call only returns new volleys.
    assert!(sim.take_shots().is_empty());
}

#[test]
fn unread_shots_are_bounded() {
    let mut sim = duel();
    run(&mut sim, 600.0);
    assert!(sim.take_shots().len() <= sim_battle::shot::MAX_PENDING_SHOTS);
}

#[test]
fn figures_fill_the_simulated_rectangle() {
    let data = data();
    let battle = setup(
        units(data, &["unit_men_at_arms_foot", "unit_knights"]),
        units(data, &["unit_longbowmen"]),
        None,
    );
    let mut sim = BattleSim::new(battle, 1).unwrap();
    lab(&mut sim);
    for formation in [
        Formation::of("line"),
        Formation::of("column"),
        Formation::of("square"),
        Formation::of("wedge"),
    ] {
        for id in 0..2u32 {
            sim.units_mut()[id as usize].formation = formation;
            let unit = &sim.units()[id as usize];
            assert_eq!(unit.figure_positions(1.0), unit.soldier_positions());
            assert_eq!(unit.figure_count(1.0), unit.soldiers());
            let (w, d) = unit.extent();
            for scale in [0.5, 1.5, 2.5] {
                let figures = unit.figure_positions(scale);
                let expected = (f64::from(unit.soldiers()) * scale).round() as usize;
                assert_eq!(figures.len(), expected, "{formation:?} × {scale}");
                assert_eq!(unit.figure_count(scale) as usize, expected);
                let (fx, fz) = unit.forward();
                let (rx, rz) = unit.right();
                for &(x, z, _) in &figures {
                    let (dx, dz) = (x - unit.x, z - unit.z);
                    let lateral = (dx * rx + dz * rz).abs();
                    let along = (dx * fx + dz * fz).abs();
                    assert!(
                        lateral <= w * 0.5 + 1.0,
                        "{formation:?} × {scale}: {lateral} > {w}/2"
                    );
                    assert!(
                        along <= d * 0.5 + 1.0,
                        "{formation:?} × {scale}: {along} > {d}/2"
                    );
                }
            }
        }
    }
    // Ultra line of foot: extra ranks rather than men packed shoulder to
    // shoulder (mean nearest-neighbour distance stays above half a metre).
    sim.units_mut()[0].formation = Formation::of("line");
    let figures = sim.units()[0].figure_positions(2.5);
    let mean: f64 = figures
        .iter()
        .map(|&(x, z, _)| {
            figures
                .iter()
                .filter(|&&(ox, oz, _)| (ox, oz) != (x, z))
                .map(|&(ox, oz, _)| ((ox - x).powi(2) + (oz - z).powi(2)).sqrt())
                .fold(f64::INFINITY, f64::min)
        })
        .sum::<f64>()
        / figures.len() as f64;
    assert!(mean > 0.5, "mean spacing {mean}");
}
