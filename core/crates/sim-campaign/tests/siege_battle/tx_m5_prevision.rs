//! M5 (ADR 0722): the pre-battle forecast is blurred by the fog of war, for
//! the player only, and never alters the battle itself.

use data_model::test_support::{fac, game_data};
use data_model::AutoResolveRules;
use sim_campaign::test_support::first_army;

fn staged() -> sim_campaign::CampaignState {
    let data = game_data();
    let mut state = sim_campaign::CampaignState::new_1337(data, fac("fac_france"), 7).unwrap();
    state.chronicle.disabled = true;
    let attacker = first_army(&state, "fac_france");
    let defender = first_army(&state, "fac_england");
    state.debug_stage_battle(&attacker, &defender).unwrap();
    state
}

#[test]
fn tx_m5_prevision_fog_is_stable_and_within_spread() {
    let mut data = game_data().clone();
    let state = staged();
    data.auto_resolve.forecast_fog = data_model::ForecastFogRules {
        spread_unseen: 0.0,
        spread_scouted: 0.0,
        spread_spied: 0.0,
        spread_spied_from_place: 0.0,
    };
    let exact = state.battle_forecast(&data, 0).unwrap();
    let spread = 0.15;
    data.auto_resolve.forecast_fog = data_model::ForecastFogRules {
        spread_unseen: spread,
        spread_scouted: spread,
        spread_spied: spread,
        spread_spied_from_place: spread,
    };
    let fogged = state.battle_forecast(&data, 0).unwrap();
    // Stable from one call to the next.
    assert_eq!(fogged, state.battle_forecast(&data, 0).unwrap());
    let ratio = |a: f64, b: f64| if b > 0.0 { a / b } else { 1.0 };
    let r = ratio(fogged.attacker_win_chance, exact.attacker_win_chance);
    assert!(
        (1.0 - spread - 1e-9..=1.0 + spread + 1e-9).contains(&r)
            || fogged.attacker_win_chance == 1.0
    );
    // Forces and head counts are never blurred.
    assert_eq!(fogged.attacker_soldiers, exact.attacker_soldiers);
    assert_eq!(fogged.defender_soldiers, exact.defender_soldiers);
    assert!(fogged.modifiers.iter().any(|m| m.contains("Brouillard")));
    assert!(!exact.modifiers.iter().any(|m| m.contains("Brouillard")));
}

#[test]
fn tx_m5_prevision_rules_are_data() {
    let rules: &AutoResolveRules = &game_data().auto_resolve;
    let fog = &rules.forecast_fog;
    assert!(fog.spread_unseen > fog.spread_scouted);
    assert!(fog.spread_scouted > fog.spread_spied);
    assert!(fog.spread_spied > fog.spread_spied_from_place);
}
