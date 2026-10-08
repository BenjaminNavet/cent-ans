//! ES (ADR 0176): an engine's shot at a regiment strikes down a set toll of
//! men and shakes it, instead of a crew-sized volley of arrows.

use crate::common;

use common::*;
use sim_battle::BattleSim;

/// Losses of the men-at-arms after `seconds` under fire from `shooter`
/// 150 m away (AIs off, the target does not move).
fn losses_under(shooter: &str, seconds: f64) -> (f64, f64) {
    let data = data();
    let setup = setup(
        units(data, &[shooter]),
        units(data, &["unit_men_at_arms_foot"]),
        None,
    );
    let mut sim = BattleSim::new(setup, 7).unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 500.0, 400.0, 0.0);
    place(&mut sim, 1, 500.0, 550.0, std::f64::consts::PI);
    let (hp0, morale0) = (sim.units()[1].hp, sim.units()[1].morale);
    run(&mut sim, seconds);
    let target = &sim.units()[1];
    (hp0 - target.hp, morale0 - target.morale)
}

#[test]
fn trebuchet_shots_kill_and_shake_men() {
    // 60 s: five stones.
    let (killed, shaken) = losses_under("unit_trebuchet", 60.0);
    assert!(killed >= 10.0, "trebuchet killed only {killed:.1} men");
    assert!(shaken >= 10.0, "morale lost only {shaken:.1}");
}

#[test]
fn engines_rank_by_siege_attack() {
    let (mangonel, _) = losses_under("unit_mangonel", 60.0);
    let (trebuchet, _) = losses_under("unit_trebuchet", 60.0);
    assert!(
        mangonel > 3.0 && mangonel < trebuchet,
        "{mangonel:.1} / {trebuchet:.1}"
    );
}
