//! TW retreat: the beaten AI sounds the retreat in open country, and an
//! ordered general retreat ends the battle as a `Withdrawal`, not a rout.

use crate::common::*;
use sim_battle::{BattleEnd, BattleSim, Command, GeneralSetup, SideId};

fn general() -> GeneralSetup {
    GeneralSetup {
        character: "chr_test".to_owned(),
        name: "Test".to_owned(),
        command: 5,
        unit_index: 0,
        morale_bonus: 0.0,
        charge_percent: 0.0,
        ranged_percent: 0.0,
        defense_percent: 0.0,
        sovereign: false,
    }
}

fn lopsided(seed: u64) -> BattleSim {
    let data = data();
    let strong = units(
        data,
        &["unit_men_at_arms_foot"; 8]
            .iter()
            .chain(["unit_longbowmen"; 4].iter())
            .copied()
            .collect::<Vec<_>>(),
    );
    let weak = units(data, &["unit_men_at_arms_foot"; 6]);
    let mut setup = setup(weak, strong, None);
    setup.attacker.general = Some(general());
    setup.defender.general = Some(general());
    BattleSim::new(setup, seed).unwrap()
}

/// A weak army sounds the retreat before it breaks: it leaves the field in
/// good order for most seeds (and never routs on the order alone).
#[test]
fn weak_ai_army_withdraws_before_broken() {
    let mut withdrawals = 0;
    for seed in 0..6 {
        let mut sim = lopsided(seed);
        let mut guard = 0;
        while !sim.is_finished() && guard < 120_000 {
            sim.step();
            guard += 1;
        }
        let outcome = sim.outcome().expect("finished");
        assert_eq!(outcome.winner, SideId::Defender, "seed {seed}");
        if outcome.end == BattleEnd::Withdrawal {
            withdrawals += 1;
            assert!(outcome.attacker.withdrew, "seed {seed}");
            assert!(!outcome.attacker.routed, "seed {seed}");
        }
    }
    assert!(withdrawals >= 3, "only {withdrawals} orderly retreats");
}

/// The general leaves last: no regiment of the side is left on the field
/// when his order is given.
#[test]
fn general_withdraws_after_the_regiments() {
    let mut sim = lopsided(1);
    let mut first_general = None;
    let mut first_regiment = None;
    while !sim.is_finished() && sim.elapsed() < 1500.0 {
        sim.step();
        for unit in sim.units().iter().filter(|u| u.side == SideId::Attacker) {
            if unit.withdrawing {
                let slot = if unit.is_general {
                    &mut first_general
                } else {
                    &mut first_regiment
                };
                slot.get_or_insert(sim.elapsed());
            }
        }
    }
    if let (Some(g), Some(r)) = (first_general, first_regiment) {
        assert!(g >= r, "general {g} before regiments {r}");
    }
}

/// A player's general retreat while the army is still in order is a
/// `Withdrawal`: the order's side loses without routing.
#[test]
fn ordered_retreat_is_a_withdrawal_not_a_rout() {
    let data = data();
    let setup = setup(
        units(data, &["unit_men_at_arms_foot"; 3]),
        units(data, &["unit_men_at_arms_foot"; 3]),
        None,
    );
    let mut sim = BattleSim::new(setup, 7).unwrap();
    sim.set_ai(SideId::Attacker, false);
    sim.set_ai(SideId::Defender, false);
    run(&mut sim, 5.0);
    let ids: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker)
        .map(|u| u.id)
        .collect();
    sim.apply_command(Command::Withdraw { units: ids }, None)
        .unwrap();
    let mut guard = 0;
    while !sim.is_finished() && guard < 60_000 {
        sim.step();
        guard += 1;
    }
    let outcome = sim.outcome().expect("finished");
    assert_eq!(outcome.end, BattleEnd::Withdrawal);
    assert_eq!(outcome.winner, SideId::Defender);
    assert!(!outcome.attacker.routed);
    assert!(outcome.attacker.withdrew);
    assert!(outcome.attacker.morale_delta > -20);
    assert!(has_event(&sim, "retraite générale"));
}
