//! Lot C5 « Commerce » integration tests: route income, cuts by war/embargo,
//! trade agreement bonus, determinism, old saves.

use data_model::GameData;
use sim_campaign::negotiation::{self, Article};
use sim_campaign::{trade, CampaignState, SiegeState};

use data_model::test_support::{fac, game_data};

fn france(data: &GameData, seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start");
    state.chronicle.disabled = true;
    // Test scenarios control war/peace/embargo explicitly; the 1337 data has
    // its own historical embargo (England on Flanders' wool), which would
    // otherwise cut the very route these tests probe.
    for faction in state.factions.values_mut() {
        faction.at_war_with.clear();
        faction.embargoes.clear();
    }
    state
}

/// Lot DP1 treaty with the single trade-agreement article: the only way an
/// agreement is formed (ADR 0012, unification C5/DP1).
fn sign_trade_treaty(state: &mut CampaignState, data: &GameData, a: &str, b: &str) {
    negotiation::apply_treaty(state, data, &fac(a), &fac(b), &[Article::TradeAgreement])
        .expect("trade treaty applies");
}

fn bruges_londres(routes: &[trade::TradeRouteView]) -> &trade::TradeRouteView {
    routes
        .iter()
        .find(|r| r.id == "route_bruges_londres")
        .expect("route_bruges_londres exists")
}

#[test]
fn route_at_peace_yields_income_to_both_controllers() {
    let data = game_data();
    let state = france(data, 1);
    let routes = trade::trade_routes(&state, data);
    let route = bruges_londres(&routes);
    assert!(!route.cut, "{:?}", route.cut_reason);
    assert_eq!(route.from_faction, Some(fac("fac_flanders")));
    assert_eq!(route.to_faction, Some(fac("fac_england")));
    assert!(route.value_from > 0);
    assert!(route.value_to > 0);

    let flanders_income = trade::faction_trade_income(&state, data, &fac("fac_flanders"));
    let england_income = trade::faction_trade_income(&state, data, &fac("fac_england"));
    assert!(flanders_income > 0);
    assert!(england_income > 0);
}

#[test]
fn war_between_the_two_ends_cuts_the_route() {
    let data = game_data();
    let mut state = france(data, 2);
    state
        .factions
        .get_mut(&fac("fac_flanders"))
        .unwrap()
        .at_war_with
        .insert(fac("fac_england"));
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .at_war_with
        .insert(fac("fac_flanders"));

    let routes = trade::trade_routes(&state, data);
    let route = bruges_londres(&routes);
    assert!(route.cut);
    assert_eq!(route.cut_reason.as_deref(), Some("guerre"));
    assert_eq!(route.value_from, 0);
    assert_eq!(route.value_to, 0);
}

#[test]
fn embargo_cuts_the_route() {
    let data = game_data();
    let mut state = france(data, 3);
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .embargoes
        .insert(fac("fac_flanders"));

    let routes = trade::trade_routes(&state, data);
    let route = bruges_londres(&routes);
    assert!(route.cut);
    assert_eq!(route.cut_reason.as_deref(), Some("embargo"));
}

#[test]
fn siege_on_a_hub_cuts_the_route() {
    let data = game_data();
    let mut state = france(data, 4);
    let bruges = data
        .settlements
        .values()
        .find(|s| s.id.as_str() == "set_bruges")
        .unwrap()
        .id
        .clone();
    let settlement = state.settlements.get_mut(&bruges).unwrap();
    settlement.siege = Some(SiegeState {
        attacker: fac("fac_england"),
        turns_left: 4,
        turns_elapsed: 0,
        supplies: 100,
        breach: 0,
        started_turn: 0,
        engine_work: 0,
    });

    let routes = trade::trade_routes(&state, data);
    let route = bruges_londres(&routes);
    assert!(route.cut);
    assert!(route.cut_reason.as_deref().unwrap_or("").contains("siège"));
}

#[test]
fn trade_agreement_raises_the_route_value() {
    let data = game_data();
    let mut state = france(data, 5);
    let routes_before = trade::trade_routes(&state, data);
    let base = bruges_londres(&routes_before).total_value();

    sign_trade_treaty(&mut state, data, "fac_flanders", "fac_england");

    assert!(state.factions[&fac("fac_flanders")]
        .ledger
        .trade_agreements
        .contains(&fac("fac_england")));
    let routes_after = trade::trade_routes(&state, data);
    let route = bruges_londres(&routes_after);
    assert!(route.agreement);
    assert!(
        route.total_value() > base,
        "agreement should raise the route's value: {base} -> {}",
        route.total_value()
    );
}

#[test]
fn end_turn_credits_trade_income_into_the_treasury() {
    let data = game_data();
    let mut state = france(data, 6);
    // No war/embargo touches Flanders in the fresh 1337 start, and armies
    // start far from the Bruges-Londres crossing, so the route survives the
    // first turn's AI moves.
    state.end_turn(data);
    let after = &state.factions[&fac("fac_flanders")];
    assert!(after.last_budget.trade_income > 0);
}

#[test]
fn campaign_with_trade_is_deterministic() {
    let data = game_data();
    let mut a = france(data, 7);
    let mut b = a.clone();
    for _ in 0..8 {
        a.end_turn(data);
        b.end_turn(data);
    }
    assert_eq!(a.save_json(), b.save_json());
}

#[test]
fn trade_state_survives_saves_and_old_saves_load() {
    let data = game_data();
    let mut state = france(data, 8);
    sign_trade_treaty(&mut state, data, "fac_flanders", "fac_england");
    state.end_turn(data);
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded, state);

    // A save written before C5 lacks the new fields entirely.
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    for faction in value["factions"].as_object_mut().unwrap().values_mut() {
        let f = faction.as_object_mut().unwrap();
        f.remove("last_budget");
    }
    let old = CampaignState::load_json(&value.to_string()).expect("old save loads");
    let f = &old.factions[&fac("fac_flanders")];
    assert_eq!(f.last_budget.trade_income, 0);
    assert!(old.has_trade_agreement(&fac("fac_flanders"), &fac("fac_england")));
}

#[test]
fn trade_article_counts_common_routes_in_the_evaluation() {
    let data = game_data();
    let state = france(data, 9);
    let (flanders, england) = (fac("fac_flanders"), fac("fac_england"));
    assert!(trade::common_routes(&state, data, &flanders, &england) > 0);
    let verdict = negotiation::evaluate_treaty(
        &state,
        data,
        &flanders,
        &england,
        &[Article::TradeAgreement],
    );
    assert!(verdict.articles[0]
        .reasons
        .iter()
        .any(|(text, value)| text == "Routes commerciales communes" && *value > 0));
}
