//! « Quitter la bataille »: `Command::Concede` ends the battle at once.

mod common;

use common::*;
use sim_battle::{BattleSim, Command, SideId};

fn battle(player: SideId) -> BattleSim {
    let data = data();
    let attacker = units(data, &["unit_men_at_arms_foot", "unit_urban_militia"]);
    let defender = units(data, &["unit_urban_militia"]);
    let mut setup = setup(attacker, defender, None);
    setup.player_side = Some(player);
    BattleSim::new(setup, 1).unwrap()
}

#[test]
fn conceding_ends_the_battle_lost() {
    for player in SideId::BOTH {
        let mut sim = battle(player);
        run(&mut sim, 2.0);
        sim.issue_command(Command::Concede { side: None }).unwrap();
        assert!(sim.is_finished(), "{player:?}: finished at once");
        assert_eq!(sim.winner(), Some(player.other()));
        assert!(sim.outcome().is_some());
    }
}

#[test]
fn conceding_works_during_deployment() {
    let mut sim = battle(SideId::Attacker);
    assert!(sim.begin_deployment());
    sim.issue_command(Command::Concede { side: None }).unwrap();
    assert!(sim.is_finished());
    assert!(!sim.is_deploying());
    assert_eq!(sim.winner(), Some(SideId::Defender));
}

#[test]
fn conceding_for_the_enemy_is_refused() {
    let mut sim = battle(SideId::Attacker);
    let refused = sim.issue_command(Command::Concede {
        side: Some(SideId::Defender),
    });
    assert!(refused.is_err());
    assert!(!sim.is_finished());
}
