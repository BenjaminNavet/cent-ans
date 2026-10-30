//! IA-PLAN-1: the division planner, plan "Tenir et recevoir" (defender).
//!
//! Unit tests of the conditions, phases and divisions live next to the code
//! (`ai/planner`); here the planner plays whole battles. Debug builds assert
//! that no regiment of a division is ordered at another enemy than its
//! division's target, so merely running them checks that rule.

use crate::common;
use crate::ep9_decisive::{battle, Mode, Tier};

use common::run;
use data_model::Terrain;
use sim_battle::ai_rules::PlannerMode;
use sim_battle::{BattleEnd, BattleEventKind, BattleScale, BattleSim};

fn planned(mode: Mode, terrain: Terrain, seed: u64, planner: PlannerMode) -> BattleSim {
    let setup = battle(Tier::Standard, terrain, false, mode);
    let scale = BattleScale::named("skirmish").unwrap();
    let mut sim = BattleSim::new_scaled(setup, seed, scale).unwrap();
    sim.set_planner(planner);
    sim
}

fn refusal_time(sim: &BattleSim) -> Option<f64> {
    sim.events()
        .iter()
        .find(|e| e.kind == Some(BattleEventKind::RefusalLikely))
        .map(|e| e.time)
}

#[test]
fn the_planner_defaults_to_legacy() {
    let sim = BattleSim::new_scaled(
        battle(Tier::Standard, Terrain::Plains, false, Mode::AiVsAi),
        0,
        BattleScale::named("skirmish").unwrap(),
    )
    .unwrap();
    assert_eq!(sim.planner_mode(), PlannerMode::Legacy);
}

/// A passive attacker facing a defender that holds (60 % of its strength): the
/// likely refusal is announced within five minutes, well before the battle is
/// refused, and the defender keeps the field.
#[test]
fn a_passive_attacker_gets_an_early_refusal_warning() {
    for seed in 0..3 {
        let mut setup = battle(Tier::Standard, Terrain::Hills, false, Mode::IdleAttacker);
        for unit in &mut setup.defender.units {
            unit.soldiers = unit.soldiers * 6 / 10;
        }
        let scale = BattleScale::named("skirmish").unwrap();
        let mut sim = BattleSim::new_scaled(setup, seed, scale).unwrap();
        sim.set_planner(PlannerMode::Plans);
        while !sim.is_finished() && sim.elapsed() < 1800.0 {
            run(&mut sim, 10.0);
        }
        let warned = refusal_time(&sim).expect("a refusal warning");
        assert!(warned < 300.0, "seed {seed}: warned at {warned:.0} s");
        assert_eq!(sim.end_kind(), Some(BattleEnd::Refused), "seed {seed}");
        assert!(warned < sim.elapsed());
    }
}

/// The division planner plays a whole AI-versus-AI battle without breaking
/// its own rules (debug assertion on the division targets).
#[test]
fn plans_play_a_full_battle() {
    for terrain in [Terrain::Plains, Terrain::Hills] {
        for seed in 0..3 {
            let mut sim = planned(Mode::AiVsAi, terrain, seed, PlannerMode::Plans);
            while !sim.is_finished() && sim.elapsed() < 1800.0 {
                run(&mut sim, 10.0);
            }
            assert!(sim.is_finished(), "{} seed {seed}", terrain.key());
        }
    }
}

/// Same seed, same battle: the planner keeps no hidden state.
#[test]
fn plans_are_deterministic() {
    let play = || {
        let mut sim = planned(Mode::AiVsAi, Terrain::Plains, 5, PlannerMode::Plans);
        run(&mut sim, 400.0);
        sim.units()
            .iter()
            .map(|u| (u.soldiers(), (u.x * 100.0).round() as i64))
            .collect::<Vec<_>>()
    };
    assert_eq!(play(), play());
}
