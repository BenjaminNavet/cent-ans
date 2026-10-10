//! TW pursuit (ADR 0321): the victors chase the routed of a 3D field
//! battle (men killed and taken alive), modulated by their living cavalry
//! and the "no quarter" order; regiments earn experience.

use crate::common;

use common::*;
use data_model::GameData;
use sim_battle::{BattleOutcome, BattleSim, Command, GeneralSetup, SideId, UnitState};

fn general() -> GeneralSetup {
    GeneralSetup {
        character: "chr_test".to_owned(),
        name: "Le connétable".to_owned(),
        command: 6,
        unit_index: 0,
        morale_bonus: 0.0,
        charge_percent: 0.0,
        ranged_percent: 0.0,
        defense_percent: 0.0,
        sovereign: false,
    }
}

/// `attackers` against three regiments of men-at-arms whose every
/// regiment is routing: the battle ends at the next step.
fn rout_sim(data: &GameData, attackers: &[&str], no_quarter: bool) -> BattleSim {
    let mut s = setup(
        units(data, attackers),
        units(data, &["unit_men_at_arms_foot"; 3]),
        None,
    );
    s.attacker.general = Some(general());
    s.orders = data.battle_orders.values().cloned().collect();
    let mut sim = BattleSim::new(s, 7).unwrap();
    lab(&mut sim);
    sim.set_end_conditions(true);
    if no_quarter {
        sim.apply_command(
            Command::LeaderOrder {
                side: None,
                order: "order_no_quarter".to_owned(),
                units: vec![],
            },
            Some(SideId::Attacker),
        )
        .unwrap();
    }
    for unit in sim
        .units_mut()
        .iter_mut()
        .filter(|u| u.side == SideId::Defender)
    {
        unit.state = UnitState::Routing;
        unit.morale = 0.0;
    }
    run_to_end(&mut sim);
    sim
}

fn outcome(attackers: &[&str], no_quarter: bool) -> BattleOutcome {
    rout_sim(data(), attackers, no_quarter).outcome().unwrap()
}

const FOOT: [&str; 2] = ["unit_men_at_arms_foot", "unit_men_at_arms_foot"];
const HORSE: [&str; 3] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_knights",
];

#[test]
fn the_pursuit_costs_the_routed_men_and_never_more_than_they_number() {
    let result = outcome(&FOOT, false);
    assert_eq!(result.winner, SideId::Attacker);
    let loser = &result.defender;
    let killed: u32 = loser.pursuit_losses.iter().sum();
    assert!(
        killed > 0 && loser.captured > 0,
        "{killed} {}",
        loser.captured
    );
    assert_eq!(loser.total_losses, killed + loser.captured);
    let strength: Vec<u32> = data()
        .unit_types
        .get(&data_model::UnitTypeId::new("unit_men_at_arms_foot").unwrap())
        .map(|t| vec![t.soldiers; 3])
        .unwrap();
    for (lost, men) in loser.losses.iter().zip(strength) {
        assert!(*lost <= men);
    }
    // The winner is not chased.
    assert!(result.attacker.pursuit_losses.iter().all(|n| *n == 0));
    assert_eq!(result.attacker.captured, 0);
}

#[test]
fn the_pursuit_is_deterministic() {
    assert_eq!(outcome(&FOOT, false), outcome(&FOOT, false));
}

#[test]
fn living_cavalry_widens_the_chase() {
    let foot = outcome(&FOOT, false).defender.total_losses;
    let horse = outcome(&HORSE, false).defender.total_losses;
    assert!(horse > foot, "{horse} <= {foot}");
}

#[test]
fn no_quarter_takes_no_prisoner() {
    let spared = outcome(&FOOT, false).defender;
    let slain = outcome(&FOOT, true).defender;
    assert!(spared.captured > 0);
    assert_eq!(slain.captured, 0);
    assert_eq!(slain.total_losses, spared.total_losses);
    assert!(slain.pursuit_losses.iter().sum::<u32>() > spared.pursuit_losses.iter().sum::<u32>());
}

#[test]
fn a_refused_battle_has_no_pursuit_and_no_experience() {
    let data = data();
    let mut sim = BattleSim::new(
        setup(
            units(data, &FOOT),
            units(data, &["unit_men_at_arms_foot"; 2]),
            None,
        ),
        3,
    )
    .unwrap();
    lab(&mut sim);
    sim.set_end_conditions(true);
    run_to_end(&mut sim);
    let result = sim.outcome().unwrap();
    assert_eq!(result.end, sim_battle::BattleEnd::Refused);
    for side in [&result.attacker, &result.defender] {
        assert_eq!(side.captured, 0);
        assert!(side.pursuit_losses.iter().all(|n| *n == 0));
        assert!(side.unit_xp_milli.iter().all(|n| *n == 0));
        assert_eq!(side.total_losses, 0);
    }
}

#[test]
fn survivors_and_victors_earn_experience() {
    let result = outcome(&FOOT, false);
    let rules = data_model::BattleOutcomeRules::bundled();
    for xp in &result.attacker.unit_xp_milli {
        assert!(*xp >= rules.unit_xp.victory_milli && *xp <= rules.unit_xp.max_milli);
    }
    assert!(result.defender.unit_xp_milli.iter().all(|n| *n == 0));
}
