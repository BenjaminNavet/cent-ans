//! EP1 tests (ADR 0031): battle scale tiers, field size by head count,
//! regiments per side, AI against AI with 40+ regiments.

mod common;

use common::*;
use data_model::Terrain;
use sim_battle::{
    BattleRng, BattleScale, BattleSim, Battlefield, FieldSize, SideId, UnitState, Weather,
};

const KINDS: [&str; 5] = [
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
    "unit_crossbowmen",
];

fn army(count: usize) -> Vec<&'static str> {
    (0..count).map(|i| KINDS[i % KINDS.len()]).collect()
}

fn big_setup(regiments: usize) -> sim_battle::BattleSetup {
    let data = data();
    let mut battle = setup(
        units(&data, &army(regiments)),
        units(&data, &army(regiments)),
        None,
    );
    for unit in battle
        .attacker
        .units
        .iter_mut()
        .chain(battle.defender.units.iter_mut())
    {
        unit.soldiers = 120;
        unit.max_soldiers = 120;
    }
    battle
}

#[test]
fn standard_field_is_unchanged() {
    for terrain in [
        Terrain::Plains,
        Terrain::Hills,
        Terrain::Bocage,
        Terrain::Marsh,
    ] {
        let a = Battlefield::generate(terrain, true, Weather::Clear, &mut BattleRng::from_seed(9));
        let b = Battlefield::generate_sized(
            FieldSize::STANDARD,
            terrain,
            true,
            Weather::Clear,
            &mut BattleRng::from_seed(9),
        );
        assert_eq!(a, b);
        assert_eq!((a.width, a.depth), (1200.0, 800.0));
    }
}

#[test]
fn head_count_picks_the_field() {
    let small = BattleSim::new(big_setup(10), 3).unwrap();
    assert_eq!(small.scale().key, "skirmish");
    assert_eq!((small.field().width, small.field().depth), (1200.0, 800.0));
    assert_eq!(small.max_on_field(), 20);

    // 63 × 120 × 2 = 15 120 soldiers.
    let epic = BattleSim::new(big_setup(63), 3).unwrap();
    assert_eq!(epic.scale().key, "epic");
    let field = epic.field();
    assert_eq!((field.width, field.depth), (2400.0, 1600.0));
    assert_eq!(field.nx, 241);
    assert_eq!(field.heights.len(), 241 * 161);
    assert!(epic.max_on_field() >= 40);
    // Every regiment is on the field, inside it, within its deployment zone.
    assert_eq!(epic.reserves(SideId::Attacker), 0);
    for unit in epic.units() {
        assert!(
            field.inside(unit.x, unit.z),
            "{} at ({}, {})",
            unit.id,
            unit.x,
            unit.z
        );
        let zone = epic.deployment_zone(unit.side);
        assert!(
            zone.contains(unit.x, unit.z) || unit.category == data_model::UnitCategory::Siege,
            "{} at ({:.0}, {:.0}) outside {:?}",
            unit.id,
            unit.x,
            unit.z,
            zone
        );
    }
}

#[test]
fn forced_tier_and_sieges() {
    let forced =
        BattleSim::new_scaled(big_setup(10), 3, BattleScale::named("large").unwrap()).unwrap();
    assert_eq!(forced.field().width, 1800.0);
    let mut siege = big_setup(63);
    siege.siege = Some(sim_battle::SiegeSetup {
        fortification: 1,
        breach: 0,
    });
    let sim = BattleSim::new(siege, 3).unwrap();
    assert_eq!(sim.field().width, 1200.0, "sieges keep the standard field");
}

#[test]
fn reinforcements_beyond_the_epic_cap() {
    let sim = BattleSim::new(big_setup(90), 5).unwrap();
    let cap = sim.max_on_field();
    assert_eq!(sim.reserves(SideId::Defender), 90 - cap);
}

/// AI against AI with 60 regiments a side: both sides close, fight and the
/// battle ends in time (probe of the tactical AI at scale).
///
/// R4 (ADR 0046, merged after EP1): an evenly matched defender receives the
/// enemy on its ground and its archers thin the attack before contact, so
/// fewer regiments are locked in melee at once (15-22 over seeds 3, 5, 11
/// instead of 20+); the melee is counted at every step.
///
/// EP9 (ADR 0056): the army that gives way breaks as a whole once it has
/// lost most of its fighting strength, so the battle ends in 5-6 minutes
/// instead of 14-18, before the whole line is locked in melee (11-12
/// regiments at most over seeds 3, 5, 11): the threshold of 20 cannot come
/// back; the battle must end within 12 minutes.
#[test]
fn ai_handles_sixty_regiments_a_side() {
    let mut sim = BattleSim::new(big_setup(60), 11).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    let mut melee_seen = 0usize;
    while !sim.is_finished() && sim.elapsed() < 1800.0 {
        sim.step();
        melee_seen = melee_seen.max(
            sim.units()
                .iter()
                .filter(|u| u.state == UnitState::Melee)
                .count(),
        );
    }
    assert!(melee_seen >= 10, "at most {melee_seen} regiments in melee");
    assert!(sim.is_finished(), "still running at {:.0} s", sim.elapsed());
    assert!(sim.elapsed() <= 720.0, "over at {:.0} s", sim.elapsed());
    println!(
        "60/side: {melee_seen} regiments in melee at most, over at {:.0} s, winner {:?}",
        sim.elapsed(),
        sim.winner()
    );
}

/// Probe (ignored): simulation cost per tick at 63 and 105 regiments a side.
#[test]
#[ignore]
fn probe_tick_cost() {
    for regiments in [20, 63, 105] {
        let mut sim = BattleSim::new(big_setup(regiments), 11).unwrap();
        sim.set_ai(SideId::Attacker, true);
        sim.set_ai(SideId::Defender, true);
        while sim.elapsed() < 90.0 {
            sim.step();
        }
        let start = std::time::Instant::now();
        let ticks = 300;
        for _ in 0..ticks {
            sim.step();
        }
        let per_tick = start.elapsed().as_secs_f64() * 1000.0 / f64::from(ticks);
        let present = sim.units().iter().filter(|u| u.present()).count();
        println!("{regiments}/side: {present} present, {per_tick:.2} ms per tick");
    }
}
