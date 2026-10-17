//! Playtest 17/10 (battlesim): time to the first melee contact of the AI sides.

use data_model::Terrain;
use sim_battle::UnitState;

use crate::ep9_decisive::{sim, Mode, Tier};

/// First melee time (simulated seconds) and the end of the battle, up to `cap`.
fn first_contact(terrain: Terrain, mode: Mode, seed: u64, cap: f64) -> (Option<f64>, f64) {
    let mut sim = sim(Tier::Standard, terrain, false, mode, seed);
    let mut first = None;
    while !sim.is_finished() && sim.elapsed() < cap {
        sim.step();
        if first.is_none() && sim.units().iter().any(|u| u.state == UnitState::Melee) {
            first = Some(sim.elapsed());
        }
    }
    (first, sim.elapsed())
}

#[test]
#[ignore]
fn contact_survey() {
    for terrain in [Terrain::Plains, Terrain::Hills] {
        for mode in [Mode::AiVsAi, Mode::IdleAttacker, Mode::IdleDefender] {
            let rows: Vec<String> = (0..4)
                .map(|seed| {
                    let (c, end) = first_contact(terrain, mode, seed, 1800.0);
                    format!("{:?}/{end:.0}", c.map(f64::round))
                })
                .collect();
            println!("{} {mode:?}: {}", terrain.key(), rows.join("  "));
        }
    }
}

/// Playtest 17/10 (defect 3): the AI sides meet within a reasonable time. Before the fix the
/// first melee came at ~690-790 s against a passive player (never on hills: the battle was
/// refused / died of a lull); now the defender sallies (`defender_sally_time`) and the winning
/// attacker's archery duel is shorter (`winning_duel_max_seconds`).
#[test]
fn the_ai_engages_a_passive_player_in_time() {
    for terrain in [Terrain::Plains, Terrain::Hills] {
        for (mode, limit) in [(Mode::IdleAttacker, 800.0), (Mode::IdleDefender, 720.0)] {
            for seed in 0..4 {
                let (contact, _) = first_contact(terrain, mode, seed, 1800.0);
                let contact = contact.unwrap_or(f64::INFINITY);
                assert!(
                    contact <= limit,
                    "{} {mode:?} seed {seed}: first melee at {contact:.0} s (limit {limit})",
                    terrain.key()
                );
            }
        }
    }
}
