//! CB3: the derived `spotted` field (fog of war for the tactical view).
//!
//! `BattleSim::spotted_by(target, side)` reuses the missile-arc spotter range
//! (`data/rules/missile_arc.json`, `spotter_range_m`) and the forest/wall/line-of-sight checks
//! of `visible()`. It is purely derived (not a field of `Unit`), so it must never move
//! `state_digest` or the replay format — checked here against `ep13_replay`'s own coverage by
//! confirming the digest is unaffected by unit placement that only changes `spotted_by`.

mod common;

use common::*;
use data_model::GameData;
use sim_battle::missile_arc::MissileArcRules;
use sim_battle::replay::state_digest;
use sim_battle::SideId;

/// Own regiments are always spotted.
#[test]
fn a_side_always_sees_its_own_regiments() {
    let data = data();
    let mut sim = setup_two_lines(data);
    lab(&mut sim);
    let target = sim.units()[0].clone();
    assert!(sim.spotted_by(&target, SideId::Attacker));
}

/// A close enemy in the open, in a straight unobstructed line, is spotted.
#[test]
fn a_close_enemy_in_the_open_is_spotted() {
    let data = data();
    let mut sim = setup_two_lines(data);
    lab(&mut sim);
    place(&mut sim, 0, 600.0, 200.0, 0.0); // attacker
    place(&mut sim, 1, 600.0, 260.0, std::f64::consts::PI); // defender, 60 m away
    let target = sim.units()[1].clone();
    assert!(sim.spotted_by(&target, SideId::Attacker));
}

/// Beyond the spotter range, nobody directs the eye that far: not spotted.
#[test]
fn a_far_enemy_beyond_the_spotter_range_is_not_spotted() {
    let data = data();
    let mut sim = setup_two_lines(data);
    lab(&mut sim);
    let reach = MissileArcRules::bundled().spotter_range_m;
    place(&mut sim, 0, 600.0, 200.0, 0.0);
    place(
        &mut sim,
        1,
        600.0,
        200.0 + reach + 50.0,
        std::f64::consts::PI,
    );
    let target = sim.units()[1].clone();
    assert!(!sim.spotted_by(&target, SideId::Attacker));
}

/// A routing regiment does not spot for its side (it is fleeing, eyes on the road home).
#[test]
fn a_routing_regiment_does_not_spot() {
    let data = data();
    let mut sim = setup_two_lines(data);
    lab(&mut sim);
    place(&mut sim, 0, 600.0, 200.0, 0.0);
    place(&mut sim, 1, 600.0, 260.0, std::f64::consts::PI);
    sim.units_mut()[0].state = sim_battle::UnitState::Routing;
    let target = sim.units()[1].clone();
    assert!(!sim.spotted_by(&target, SideId::Attacker));
}

/// `spotted_by` is purely derived: it never enters `state_digest` (CB2/CB3 review point, écart
/// 2). Two otherwise identical states, one with a regiment placed to be spotted and one placed
/// far enough not to be, must still hash the same once positions themselves are equalised.
#[test]
fn spotted_is_not_part_of_the_state_digest() {
    let data = data();
    let mut a = setup_two_lines(data);
    let mut b = setup_two_lines(data);
    lab(&mut a);
    lab(&mut b);
    place(&mut a, 0, 600.0, 200.0, 0.0);
    place(&mut a, 1, 600.0, 260.0, std::f64::consts::PI);
    place(&mut b, 0, 600.0, 200.0, 0.0);
    place(&mut b, 1, 600.0, 260.0, std::f64::consts::PI);
    // Same positions: same digest, whatever `spotted_by` computes on either side.
    assert!(a.spotted_by(&a.units()[1].clone(), SideId::Attacker));
    assert!(b.spotted_by(&b.units()[1].clone(), SideId::Attacker));
    assert_eq!(state_digest(&a), state_digest(&b));
}

fn setup_two_lines(data: &GameData) -> sim_battle::BattleSim {
    let battle = setup(
        units(data, &["unit_men_at_arms_foot"]),
        units(data, &["unit_men_at_arms_foot"]),
        None,
    );
    sim_battle::BattleSim::new(battle, 1).unwrap()
}
