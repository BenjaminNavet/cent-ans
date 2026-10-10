//! TW retreat: an ordered general retreat is not a rout, and the defender
//! of a field battle may fall back before it (not in a siege or an ambush).

use data_model::test_support::{fac, game_data};
use data_model::GameData;
use sim_battle::{BattleEnd, BattleOpening, BattleOutcome, SideId, SideResult};
use sim_campaign::test_support::first_army;
use sim_campaign::{ArmyId, CampaignState};

fn staged(data: &GameData, attacker: &str, defender: &str) -> (CampaignState, ArmyId, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).expect("1337 start");
    state.chronicle.disabled = true;
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    let attacker = first_army(&state, attacker);
    let defender = first_army(&state, defender);
    state.debug_stage_battle(&attacker, &defender).unwrap();
    (state, attacker, defender)
}

fn side(count: usize, morale_delta: i32, routed: bool, withdrew: bool) -> SideResult {
    SideResult {
        losses: vec![1; count],
        total_losses: count as u32,
        morale_delta,
        routed,
        general_killed: false,
        general_captured: false,
        no_quarter: false,
        withdrew,
        standards_taken: Vec::new(),
        standards_lost: 0,
        baggage_lost: false,
    }
}

/// England (defender) wins; France (attacker) leaves by `end`.
fn resolve(end: BattleEnd) -> (CampaignState, ArmyId, Vec<sim_campaign::GameEvent>) {
    let data = game_data();
    let (mut state, attacker, _) = staged(data, "fac_france", "fac_england");
    let setup = state.battle_setup(data, 0).unwrap();
    let withdrawal = end == BattleEnd::Withdrawal;
    let outcome = BattleOutcome {
        winner: SideId::Defender,
        attacker: side(
            setup.attacker.units.len(),
            if withdrawal { -8 } else { -20 },
            !withdrawal,
            withdrawal,
        ),
        defender: side(setup.defender.units.len(), 3, false, false),
        duration: 300.0,
        end,
    };
    let events = state.resolve_pending_battle(data, 0, &outcome).unwrap();
    (state, attacker, events)
}

#[test]
fn withdrawal_is_told_apart_from_a_rout() {
    let (_, _, withdrawal) = resolve(BattleEnd::Withdrawal);
    let (_, _, rout) = resolve(BattleEnd::Rout);
    assert!(withdrawal
        .iter()
        .any(|e| e.text_fr.contains("Retraite ordonnée")));
    assert!(!rout.iter().any(|e| e.text_fr.contains("Retraite ordonnée")));
}

#[test]
fn withdrawal_costs_less_morale_than_a_rout() {
    let morale = |end| {
        let (state, attacker, _) = resolve(end);
        state
            .armies
            .get(&attacker)
            .map(|a| a.units.iter().map(|u| u32::from(u.morale)).sum::<u32>())
    };
    let (withdrawn, routed) = (morale(BattleEnd::Withdrawal), morale(BattleEnd::Rout));
    if let (Some(w), Some(r)) = (withdrawn, routed) {
        assert!(w > r, "withdrawal {w} vs rout {r}");
    }
}

#[test]
fn a_defender_may_fall_back_before_a_field_battle() {
    let data = game_data();
    let (mut state, _, defender) = staged(data, "fac_england", "fac_france");
    let before = state.armies[&defender].total_strength();
    let forecast = state.battle_forecast(data, 0).expect("forecast");
    assert!(forecast.can_withdraw);
    let events = state.withdraw_pending_battle(data, 0).expect("withdraw");
    assert!(state.pending_battles.is_empty());
    assert!(!events.is_empty());
    if let Some(army) = state.armies.get(&defender) {
        assert!(army.total_strength() < before, "stragglers are lost");
    }
}

#[test]
fn a_defender_cannot_fall_back_from_an_ambush_or_a_siege() {
    let data = game_data();
    let (mut state, _, _) = staged(data, "fac_england", "fac_france");
    state.pending_battles[0].opening = BattleOpening::Ambush {
        victim: SideId::Defender,
    };
    assert!(!state.battle_forecast(data, 0).unwrap().can_withdraw);
    assert!(state.withdraw_pending_battle(data, 0).is_err());
    assert_eq!(state.pending_battles.len(), 1);

    let (mut state, _, _) = staged(data, "fac_england", "fac_france");
    state.pending_battles[0].siege = true;
    assert!(state
        .battle_forecast(data, 0)
        .map_or(true, |f| !f.can_withdraw));
}
