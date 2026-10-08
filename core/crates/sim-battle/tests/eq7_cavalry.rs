//! EQ7: the attacking horse waits for its foot before riding into the
//! range of archers who still have arrows.

mod common;

use common::*;
use sim_battle::{BattleSim, SideId};

/// The small mixed battle of `b6` (no site, AI on both sides).
fn small_mixed_battle(seed: u64) -> BattleSim {
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
    // CB4: the regiments' abilities, as in the game.
    battle.abilities = data.battle_abilities.values().cloned().collect();
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    sim
}

/// The richer French army (knights, men-at-arms, crossbows, 5 800) wins
/// the mixed battle against English longbows screened by their knights
/// (3 750) most of the time again: its knights no longer ride alone through
/// the arrows ahead of their foot. Main before EQ7: 31/64 (about 8 of these
/// 16); with EQ7: 15/16, 57/64.
#[test]
fn french_knights_wait_for_their_foot_and_win_the_mixed_battle() {
    let french = (0..16)
        .filter(|&seed| {
            let mut sim = small_mixed_battle(seed);
            run_to_end(&mut sim);
            sim.winner() == Some(SideId::Attacker)
        })
        .count();
    assert!(french >= 12, "French win {french}/16");
}
