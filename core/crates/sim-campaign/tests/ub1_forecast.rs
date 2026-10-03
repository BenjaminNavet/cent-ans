//! UB1 pre-battle screen: balance forecast and withdrawal from a pending
//! battle (`battle_forecast.rs`).

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::battle_forecast::WITHDRAW_MORALE_LOSS;
use sim_campaign::{ArmyId, CampaignState};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn first_army(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction))
        .map(|(id, _)| id.clone())
        .unwrap()
}

fn at_war(state: &mut CampaignState) {
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
}

fn staged(
    data: &GameData,
    attacker_faction: &str,
    defender_faction: &str,
) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).expect("1337 start");
    state.chronicle.disabled = true;
    at_war(&mut state);
    let attacker = first_army(&state, attacker_faction);
    let defender = first_army(&state, defender_faction);
    state.debug_stage_battle(&attacker, &defender).unwrap();
    (state, attacker)
}

#[test]
fn forecast_is_consistent_and_does_not_consume_randomness() {
    let data = data();
    let (state, _) = staged(&data, "fac_france", "fac_england");
    let forecast = state.battle_forecast(&data, 0).expect("forecast");
    assert!(forecast.attacker_power > 0.0 && forecast.defender_power > 0.0);
    // The bar and the verdict share one probability (A6-L1, ADR 0181).
    assert!((forecast.attacker_share - forecast.attacker_win_chance).abs() < 1e-9);
    assert!((0.0..=1.0).contains(&forecast.attacker_win_chance));
    assert!(forecast.attacker_soldiers > 0 && forecast.defender_soldiers > 0);
    assert!(
        forecast.can_withdraw,
        "the player attacks: he may call it off"
    );
    // Pure: forecasting then auto-resolving gives the same result as auto-resolving.
    let mut plain = state.clone();
    let mut forecasted = state.clone();
    forecasted.battle_forecast(&data, 0).unwrap();
    let a = plain.auto_resolve_pending(&data, 0).unwrap();
    let b = forecasted.auto_resolve_pending(&data, 0).unwrap();
    assert_eq!(a, b);
    assert!(state.battle_forecast(&data, 5).is_err());
}

#[test]
fn an_attacker_may_withdraw_at_a_morale_cost() {
    let data = data();
    let (mut state, attacker) = staged(&data, "fac_france", "fac_england");
    let before: Vec<u8> = state.armies[&attacker]
        .units
        .iter()
        .map(|u| u.morale)
        .collect();
    let events = state.withdraw_pending_battle(&data, 0).expect("withdraw");
    assert_eq!(events.len(), 1);
    assert!(state.pending_battles.is_empty());
    let after: Vec<u8> = state.armies[&attacker]
        .units
        .iter()
        .map(|u| u.morale)
        .collect();
    for (b, a) in before.iter().zip(&after) {
        assert_eq!(*a, b.saturating_sub(WITHDRAW_MORALE_LOSS));
    }
}

#[test]
fn a_defender_cannot_slip_away() {
    let data = data();
    let (mut state, _) = staged(&data, "fac_england", "fac_france");
    let forecast = state.battle_forecast(&data, 0).expect("forecast");
    assert!(!forecast.can_withdraw);
    assert!(state.withdraw_pending_battle(&data, 0).is_err());
    assert_eq!(state.pending_battles.len(), 1);
}

#[test]
fn calling_off_an_assault_keeps_the_siege() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 9).expect("1337 start");
    state.chronicle.disabled = true;
    let lead = first_army(&state, "fac_france");
    let guyenne = ProvinceId::new("prov_guyenne").unwrap();
    let index = state.debug_stage_siege(&data, &lead, &guyenne).unwrap();
    let forecast = state.battle_forecast(&data, index).expect("forecast");
    assert!(forecast.siege && forecast.can_withdraw);
    assert!(forecast.defender_soldiers > 0);
    let settlement = state.armies[&lead].settlement().cloned().unwrap();
    let morale: Vec<u8> = state.armies[&lead].units.iter().map(|u| u.morale).collect();
    state
        .withdraw_pending_battle(&data, index)
        .expect("withdraw");
    assert!(state.pending_battles.is_empty());
    assert!(
        state.settlements[&settlement].siege.is_some(),
        "the siege goes on"
    );
    let after: Vec<u8> = state.armies[&lead].units.iter().map(|u| u.morale).collect();
    assert_eq!(morale, after, "no morale cost for a postponed assault");
}
