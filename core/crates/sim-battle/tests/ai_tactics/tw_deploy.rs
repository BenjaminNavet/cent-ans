//! TW ai-deploy (ADR 0331): the AI answers the player's deployment, and the
//! player can order a pursuit of routing regiments.

use crate::common::*;
use data_model::UnitCategory;
use sim_battle::{BattleSim, Command, CommandError, SideId, UnitState};

/// Player attacks with horse; the AI defender fields spears and foot.
fn deployed(seed: u64, cavalry_x: f64) -> BattleSim {
    let data = data();
    let player = units(
        data,
        &[
            "unit_ordonnance_gendarmes",
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
        ],
    );
    let ai = units(
        data,
        &[
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_coutiliers",
            "unit_men_at_arms_foot",
        ],
    );
    let mut setup = setup(player, ai, None);
    setup.player_side = Some(SideId::Attacker);
    let mut sim = BattleSim::new(setup, seed).unwrap();
    sim.set_ai(SideId::Attacker, false);
    assert!(sim.begin_deployment());
    let zone = sim.deployment_zone(SideId::Attacker);
    let z = (zone.z0 + zone.z1) * 0.5;
    let id = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Attacker && u.category == UnitCategory::Cavalry)
        .map(|u| u.id)
        .expect("a cavalry regiment");
    sim.deploy_unit(id, cavalry_x, z, None).unwrap();
    sim.start_battle().unwrap();
    sim
}

fn spear_x(sim: &BattleSim) -> f64 {
    sim.units()
        .iter()
        .find(|u| u.side == SideId::Defender && u.name.to_lowercase().contains("coutil"))
        .map(|u| u.x)
        .expect("the spear regiment")
}

/// The spearmen come to stand opposite the enemy horse, wherever it is.
#[test]
fn spears_face_enemy_cavalry() {
    let mut spots = Vec::new();
    for &cx in &[300.0, 600.0, 900.0] {
        let sim = deployed(1, cx);
        let foot: Vec<f64> = sim
            .units()
            .iter()
            .filter(|u| u.side == SideId::Defender)
            .map(|u| u.x)
            .collect();
        let (lo, hi) = foot
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
        let wanted = cx.clamp(lo, hi);
        let dx = (spear_x(&sim) - wanted).abs();
        assert!(
            dx < 60.0,
            "spears at {:.0}, horse at {cx} (line {lo:.0}..{hi:.0})",
            spear_x(&sim)
        );
        spots.push(spear_x(&sim));
    }
    assert!(
        spots[0] < spots[2],
        "the spears follow the horse: {spots:?}"
    );
}

#[test]
fn counter_deployment_is_deterministic_and_inside_the_zone() {
    let a = deployed(7, 500.0);
    let b = deployed(7, 500.0);
    let pos = |s: &BattleSim| -> Vec<(f64, f64)> { s.units().iter().map(|u| (u.x, u.z)).collect() };
    assert_eq!(pos(&a), pos(&b));
    let zone = a.deployment_zone(SideId::Defender);
    for u in a.units().iter().filter(|u| u.side == SideId::Defender) {
        assert!(zone.contains(u.x, u.z), "{} outside the zone", u.name);
    }
}

/// Without a player there is no reaction (two AIs place simultaneously).
#[test]
fn no_counter_deployment_without_a_player() {
    let data = data();
    let a = units(
        data,
        &["unit_ordonnance_gendarmes", "unit_men_at_arms_foot"],
    );
    let d = units(data, &["unit_men_at_arms_foot", "unit_coutiliers"]);
    let mut sim = BattleSim::new(setup(a, d, None), 3).unwrap();
    assert!(sim.begin_deployment());
    let before: Vec<(f64, f64)> = sim.units().iter().map(|u| (u.x, u.z)).collect();
    sim.start_battle().unwrap();
    let after: Vec<(f64, f64)> = sim.units().iter().map(|u| (u.x, u.z)).collect();
    assert_eq!(before, after);
}

/// `Pursue` is refused on a regiment that is not routing and sends the
/// horse after one that is.
#[test]
fn pursue_orders_a_chase_of_a_routing_regiment() {
    let data = data();
    let a = units(
        data,
        &["unit_ordonnance_gendarmes", "unit_men_at_arms_foot"],
    );
    let d = units(data, &["unit_men_at_arms_foot", "unit_men_at_arms_foot"]);
    let mut sim = BattleSim::new(setup(a, d, None), 5).unwrap();
    sim.set_ai(SideId::Attacker, false);
    sim.set_ai(SideId::Defender, false);
    let horse = 0u32;
    let foe = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender)
        .map(|u| u.id)
        .unwrap();
    let order = Command::Pursue {
        units: vec![horse],
        target: foe,
        queue: false,
    };
    assert_eq!(
        sim.apply_command(order.clone(), Some(SideId::Attacker)),
        Err(CommandError::NotRouting(foe))
    );
    sim.units_mut()[foe as usize].state = UnitState::Routing;
    sim.apply_command(order, Some(SideId::Attacker)).unwrap();
    assert_eq!(sim.units()[horse as usize].target, Some(foe));
    let gap = |sim: &BattleSim| {
        let (h, f) = (&sim.units()[horse as usize], &sim.units()[foe as usize]);
        (h.x - f.x).hypot(h.z - f.z)
    };
    let before = gap(&sim);
    for _ in 0..300 {
        sim.step();
    }
    assert!(gap(&sim) < before, "the horse closed in");
    let friend = Command::Pursue {
        units: vec![horse],
        target: 1,
        queue: false,
    };
    assert!(sim.apply_command(friend, Some(SideId::Attacker)).is_err());
}

fn layout(seed: u64, player: &[&str], ai: &[&str], react: bool, foot_x: f64) -> BattleSim {
    let data = data();
    let mut setup = setup(units(data, player), units(data, ai), None);
    setup.player_side = react.then_some(SideId::Attacker);
    let mut sim = BattleSim::new(setup, seed).unwrap();
    sim.set_ai(SideId::Attacker, false);
    assert!(sim.begin_deployment());
    if react {
        let zone = sim.deployment_zone(SideId::Attacker);
        let ids: Vec<u32> = sim
            .units()
            .iter()
            .filter(|u| u.side == SideId::Attacker)
            .map(|u| u.id)
            .collect();
        for (k, id) in ids.into_iter().enumerate() {
            sim.deploy_unit(
                id,
                foot_x + 40.0 * k as f64,
                (zone.z0 + zone.z1) * 0.5,
                None,
            )
            .unwrap();
        }
    }
    sim.start_battle().unwrap();
    sim
}

fn shooter_centre(sim: &BattleSim) -> f64 {
    let xs: Vec<f64> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender && u.name.to_lowercase().contains("arc"))
        .map(|u| u.x)
        .collect();
    assert!(!xs.is_empty(), "the AI has archers");
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// The archers shift towards the enemy's slow foot (within the shift cap).
#[test]
fn shooters_shift_towards_slow_foot() {
    let foot = ["unit_men_at_arms_foot"; 3];
    let ai = [
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_longbowmen",
    ];
    let base = shooter_centre(&layout(2, &foot, &ai, false, 0.0));
    let left = shooter_centre(&layout(2, &foot, &ai, true, 150.0));
    let right = shooter_centre(&layout(2, &foot, &ai, true, 1050.0));
    assert!(
        left < base && base < right,
        "{left:.0} < {base:.0} < {right:.0}"
    );
    assert!((right - left) <= 2.0 * 120.0 + 1.0);
}

/// Facing a front twice as wide as its own, the AI refuses a flank: some
/// regiment steps back compared with the unprovoked layout.
#[test]
fn wide_enemy_front_refuses_a_flank() {
    let wide = ["unit_men_at_arms_foot"; 10];
    let ai = ["unit_men_at_arms_foot"; 4];
    let base = layout(4, &wide, &ai, false, 0.0);
    let reacted = layout(4, &wide, &ai, true, 300.0);
    let z = |s: &BattleSim| -> Vec<f64> {
        s.units()
            .iter()
            .filter(|u| u.side == SideId::Defender)
            .map(|u| u.z)
            .collect()
    };
    let (b, r) = (z(&base), z(&reacted));
    assert!(
        b.iter().zip(&r).any(|(b, r)| r > b),
        "a flank steps back: {b:?} -> {r:?}"
    );
}
