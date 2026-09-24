//! B6: the tactical AI uses the campaign site (hedges, ditches, village).

mod common;

use common::*;
use sim_battle::ai::{defensive_cover, CoverKind};
use sim_battle::{
    BattleSetup, BattleSim, Obstacle, ObstacleKind, SideId, Unit, UnitState, Village, Weather,
    Zone, ATTACKER_LINE_Z, DEFENDER_LINE_Z, FIELD_WIDTH,
};

fn demo_setup() -> BattleSetup {
    serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap()
}

fn demo_sim() -> BattleSim {
    let mut sim = BattleSim::new(demo_setup(), 1337).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim
}

fn first_contact(sim: &mut BattleSim, limit: f64) -> Option<f64> {
    while sim.elapsed() < limit && !sim.is_finished() {
        sim.step();
        if sim.units().iter().any(|u| u.state == UnitState::Melee) {
            return Some(sim.elapsed());
        }
    }
    None
}

/// Digest of a whole battle: end time, winner, soldiers left per regiment.
fn digest(sim: &mut BattleSim) -> String {
    run_to_end(sim);
    let hp: Vec<i64> = sim.units().iter().map(|u| u.hp.round() as i64).collect();
    format!("{:.0} {:?} {:?}", sim.elapsed(), sim.winner(), hp)
}

/// A mixed field battle with no site at all (no village, inland plains).
fn no_site_sim(seed: u64) -> BattleSim {
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

#[test]
#[ignore]
fn probe() {
    let mut sim = demo_sim();
    let f = sim.field();
    println!(
        "demo: village {:?} obstacles {} coast {:?} ground {:?}",
        f.village.as_ref().map(|v| (v.zone, v.farm)),
        f.obstacles.len(),
        f.coast,
        f.ground
    );
    for o in &f.obstacles {
        println!("  {o:?}");
    }
    println!("contact {:?}", first_contact(&mut sim, 300.0));
    for seed in [3, 11] {
        let mut s = no_site_sim(seed);
        assert!(s.field().obstacles.is_empty() && s.field().village.is_none());
        println!("no-site {seed}: {}", digest(&mut s));
    }
}

/// B4 rhythm kept: the demo armies still meet after about 70 s.
#[test]
fn demo_contact_stays_near_seventy_seconds() {
    let mut sim = demo_sim();
    let contact = first_contact(&mut sim, 300.0);
    assert!(
        contact.is_some_and(|t| (55.0..=95.0).contains(&t)),
        "contact at {contact:?}"
    );
}

/// Without a site (no village, no hedge), the AI plays exactly as before B6.
#[test]
fn battles_without_a_site_are_unchanged() {
    let expected = [
        (
            3,
            "238 Some(Attacker) [27, 46, 48, 100, 100, 20, 51, 83, 84, 6]",
        ),
        (
            11,
            "228 Some(Attacker) [23, 47, 47, 100, 100, 25, 65, 84, 86, 4]",
        ),
    ];
    for (seed, digest_before) in expected {
        let mut sim = no_site_sim(seed);
        assert!(sim.field().obstacles.is_empty() && sim.field().village.is_none());
        assert!(defensive_cover(sim.field(), SideId::Defender).is_none());
        assert_eq!(digest(&mut sim), digest_before, "seed {seed}");
    }
}

/// A hedge across the defender's front, 60 m ahead of its deployment line.
const HEDGE_Z: f64 = DEFENDER_LINE_Z - 60.0;

fn hedge_across(kind: ObstacleKind) -> Obstacle {
    Obstacle {
        a: (470.0, HEDGE_Z + 3.0),
        b: (730.0, HEDGE_Z - 3.0),
        kind,
    }
}

/// Outnumbered English (defensive posture) against a French host.
fn english_on_the_defensive(seed: u64) -> BattleSim {
    let data = data();
    let french = [
        "unit_knights",
        "unit_knights",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_crossbowmen",
        "unit_crossbowmen",
    ];
    let english = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_longbowmen",
    ];
    let mut battle = setup(units(&data, &french), units(&data, &english), None);
    battle.village = Some(false);
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    sim.set_weather(Weather::Clear);
    sim
}

fn english_archers(sim: &BattleSim) -> Vec<&Unit> {
    sim.units()
        .iter()
        .filter(|u| u.side == SideId::Defender && u.can_shoot())
        .collect()
}

/// Behind the hedge: on the English side of it, close enough to be covered.
fn behind_hedge(u: &Unit) -> bool {
    let hedge = hedge_across(ObstacleKind::Hedge);
    let enemy = (600.0, ATTACKER_LINE_Z);
    hedge.distance(u.x, u.z) <= sim_battle::site::HEDGE_COVER_REACH
        && hedge.crosses(enemy, (u.x, u.z))
}

#[test]
fn defensive_archers_stand_behind_the_hedge() {
    let mut sim = english_on_the_defensive(5);
    sim.field_mut()
        .obstacles
        .push(hedge_across(ObstacleKind::Hedge));
    let cover = defensive_cover(sim.field(), SideId::Defender).expect("the hedge is cover");
    assert!(cover.breaks_charge);
    // March to the hedge before the French close in.
    let mut covered_at = None;
    while sim.elapsed() < 150.0 && covered_at.is_none() {
        sim.step();
        if english_archers(&sim).iter().all(|u| behind_hedge(u)) {
            covered_at = Some(sim.elapsed());
        }
    }
    let t = covered_at.expect("the archers reach the hedge");
    // They are still there when the French come within bowshot.
    run(&mut sim, 20.0);
    for u in english_archers(&sim) {
        assert!(
            behind_hedge(u),
            "{} at ({:.0}, {:.0}) after {t:.0} s",
            u.name,
            u.x,
            u.z
        );
    }
    // The men-at-arms stand in line behind the archers.
    let foot = &sim.units()[7];
    assert!(
        foot.z > HEDGE_Z + 10.0 && foot.z < HEDGE_Z + 80.0,
        "foot at {:.0}",
        foot.z
    );
}

#[test]
fn without_the_hedge_the_archers_keep_their_old_ground() {
    let mut sim = english_on_the_defensive(5);
    run(&mut sim, 60.0);
    // No cover: they stay near their deployment (high ground search only).
    for u in english_archers(&sim) {
        assert!(u.z > HEDGE_Z + 20.0, "{} at z {:.0}", u.name, u.z);
    }
}

#[test]
fn a_village_edge_is_cover_too() {
    let mut sim = english_on_the_defensive(5);
    sim.field_mut().village = Some(Village {
        zone: Zone {
            x: 640.0,
            z: DEFENDER_LINE_Z - 20.0,
            radius: 60.0,
        },
        farm: false,
        houses: Vec::new(),
    });
    let cover = defensive_cover(sim.field(), SideId::Defender).expect("the village is cover");
    assert_eq!(cover.kind, CoverKind::Village);
    assert!(sim.field().in_village(cover.center.0, cover.center.1));
    run(&mut sim, 120.0);
    let inside = english_archers(&sim)
        .iter()
        .filter(|u| sim.field().in_village(u.x, u.z))
        .count();
    assert!(inside >= 1, "no archer in the village");
}

/// Isolated archers behind a hedge, knights of the AI facing them.
fn knights_facing_archers(obstacle: Obstacle) -> BattleSim {
    let data = data();
    let mut battle = setup(
        units(&data, &["unit_knights"]),
        units(&data, &["unit_longbowmen", "unit_men_at_arms_foot"]),
        None,
    );
    battle.village = Some(false);
    let mut sim = BattleSim::new(battle, 9).unwrap();
    lab(&mut sim);
    sim.set_ai(SideId::Attacker, true);
    sim.field_mut().obstacles.push(obstacle);
    place(&mut sim, 1, 600.0, HEDGE_Z + 7.0, std::f64::consts::PI);
    place(&mut sim, 2, 1100.0, 750.0, std::f64::consts::PI);
    place(&mut sim, 0, 600.0, HEDGE_Z - 150.0, 0.0);
    // No stakes: the knights would not charge them head on anyway.
    sim.units_mut()[1]
        .abilities
        .retain(|a| *a != data_model::Ability::Stakes);
    hold_fire(&mut sim, SideId::Defender);
    sim
}

#[test]
#[ignore]
fn trace_b6() {
    let mut sim = english_on_the_defensive(5);
    sim.field_mut()
        .obstacles
        .push(hedge_across(ObstacleKind::Hedge));
    println!("{:?}", defensive_cover(sim.field(), SideId::Defender));
    for step in 0..=8 {
        while sim.elapsed() < f64::from(step) * 15.0 {
            sim.step();
        }
        println!("t={:.0}", sim.elapsed());
        for u in sim.units() {
            println!(
                "  {:?} {:22} {:5.0} {:5.0} {:?} dest {:?} cover {}",
                u.side,
                u.name,
                u.x,
                u.z,
                u.state,
                u.destination.map(|d| (d.0 as i32, d.1 as i32)),
                behind_hedge(u)
            );
        }
    }
    let mut sim = knights_facing_archers(hedge_across(ObstacleKind::Hedge));
    for step in 0..=8 {
        while sim.elapsed() < f64::from(step) * 10.0 {
            sim.step();
        }
        let u = &sim.units()[0];
        println!(
            "t={:.0} knights {:.0} {:.0} {:?} dest {:?} tgt {:?}",
            sim.elapsed(),
            u.x,
            u.z,
            u.state,
            u.destination,
            u.target
        );
    }
}

/// Knights sent at archers behind a hedge ride round it instead of
/// charging through (no broken charge), and still reach them.
#[test]
fn cavalry_rides_round_a_hedge() {
    let mut sim = knights_facing_archers(hedge_across(ObstacleKind::Hedge));
    let mut contact = false;
    while sim.elapsed() < 120.0 && !contact {
        sim.step();
        contact = sim.units()[0].state == UnitState::Melee;
    }
    assert!(contact, "the knights reach the archers");
    assert!(!has_event(&sim, "se brise sur la haie"));
}

/// With no way round (a ditch across the whole field), the knights wait.
#[test]
fn cavalry_waits_rather_than_charge_through_a_ditch() {
    let mut sim = knights_facing_archers(Obstacle {
        a: (5.0, HEDGE_Z),
        b: (FIELD_WIDTH - 5.0, HEDGE_Z),
        kind: ObstacleKind::Ditch,
    });
    run(&mut sim, 60.0);
    assert!(!has_event(&sim, "se brise sur la haie"));
    assert_ne!(sim.units()[0].state, UnitState::Melee);
}
