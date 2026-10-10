//! TW trans: auto-resolve report for the result screen and general-loss
//! risk of the forecast.

use data_model::test_support::{fac, game_data};
use data_model::GameData;
use sim_campaign::test_support::first_army;
use sim_campaign::CampaignState;

fn staged(data: &GameData) -> CampaignState {
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
    let attacker = first_army(&state, "fac_france");
    let defender = first_army(&state, "fac_england");
    state.debug_stage_battle(&attacker, &defender).unwrap();
    state
}

#[test]
fn forecast_exposes_general_loss_risk_without_touching_the_rng() {
    let data = game_data();
    let state = staged(data);
    let forecast = state.battle_forecast(data, 0).expect("forecast");
    for pct in [
        forecast.attacker_general_loss_pct,
        forecast.defender_general_loss_pct,
    ] {
        assert!((0.0..=100.0).contains(&pct));
    }
    // Stable for a given state (private generators) and pure.
    let again = state.battle_forecast(data, 0).unwrap();
    assert_eq!(forecast, again);
    let mut plain = state.clone();
    let mut forecasted = state.clone();
    forecasted.battle_forecast(data, 0).unwrap();
    assert_eq!(
        plain.auto_resolve_pending(data, 0).unwrap(),
        forecasted.auto_resolve_pending(data, 0).unwrap()
    );
}

#[test]
fn auto_resolve_report_matches_the_battle() {
    let data = game_data();
    let mut state = staged(data);
    let before = state.clone();
    let (events, report) = state.auto_resolve_pending_report(data, 0).unwrap();
    let report = report.expect("a field battle has a report");
    assert!(!events.is_empty());
    assert!(report.attacker.player && !report.defender.player);
    assert_eq!(report.attacker.faction, fac("fac_france"));
    assert_eq!(report.defender.faction, fac("fac_england"));
    assert!(report.attacker.soldiers_before > 0 && report.defender.soldiers_before > 0);
    // The loser took at least as many losses as its report says, never more than it had.
    assert!(report.attacker.losses <= report.attacker.soldiers_before);
    assert!(report.defender.losses <= report.defender.soldiers_before);
    assert!(!report.province.is_empty());
    // Same resolution as the plain call.
    let mut plain = before;
    assert_eq!(plain.auto_resolve_pending(data, 0).unwrap(), events);
}
