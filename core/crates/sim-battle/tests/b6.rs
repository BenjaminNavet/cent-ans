//! B6: the tactical AI uses the campaign site (hedges, ditches, village).

mod common;

use common::*;
use sim_battle::{BattleSetup, BattleSim, SideId, UnitState};

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
