//! UB1: kills credited per unit for the result screen.

mod common;

use common::*;
use sim_battle::{BattleSim, SideId};

const ARMY: [&str; 4] = [
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
];

#[test]
fn kills_match_the_enemy_losses_they_caused() {
    let data = data();
    let mut sim = BattleSim::new(setup(units(data, &ARMY), units(data, &ARMY), None), 3).unwrap();
    run_to_end(&mut sim);
    for side in SideId::BOTH {
        let kills: f64 = sim
            .units()
            .iter()
            .filter(|u| u.side == side)
            .map(|u| u.kills)
            .sum();
        let enemy_losses: f64 = sim
            .units()
            .iter()
            .filter(|u| u.side == side.other())
            .map(|u| f64::from(u.initial_soldiers) - u.hp.max(0.0))
            .sum();
        assert!(kills > 0.0, "{side:?} struck nobody");
        // Charges on stakes, walls and fire are not credited: kills never
        // exceed the enemy's losses.
        assert!(
            kills <= enemy_losses + 1e-6,
            "{side:?}: {kills} kills for {enemy_losses} enemy losses"
        );
        assert!(
            kills >= enemy_losses * 0.8,
            "{side:?}: {kills} kills credited for {enemy_losses} enemy losses"
        );
    }
}
