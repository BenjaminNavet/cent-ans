//! Lot OMR R3 (ADR 0117): the lord's household guard — the cheapest unit of
//! the capital city's garrison — is paid by the domain, and a realm in debt
//! dismisses its agents.

use data_model::{AgentKind, SettlementKind};
use sim_campaign::economy::{garrison_share, garrison_upkeep_percent};
use sim_campaign::test_support::start;
use sim_campaign::{CampaignState, Order};

use data_model::test_support::{fac, game_data};

#[test]
fn the_data_sets_a_one_unit_guard_paid_by_the_domain() {
    let data = game_data();
    let guard = data
        .settlement_rules
        .as_ref()
        .and_then(|r| r.capital_guard.as_ref())
        .expect("capital_guard in data/settlements/rules.json");
    assert_eq!(guard.units, 1);
    assert_eq!(guard.upkeep_percent, 0);
}

#[test]
fn only_the_cheapest_units_of_the_capital_form_the_guard() {
    let data = game_data();
    let percent = garrison_upkeep_percent(data, SettlementKind::City);
    let costs = [100, 40, 60];
    // Elsewhere every unit pays the city's share.
    assert_eq!(
        garrison_share(data, SettlementKind::City, false, costs),
        200 * percent / 100
    );
    // In the capital the cheapest (40) is free.
    assert_eq!(
        garrison_share(data, SettlementKind::City, true, costs),
        160 * percent / 100
    );
    assert_eq!(garrison_share(data, SettlementKind::City, true, []), 0);
}

#[test]
fn a_county_keeping_one_unit_at_home_pays_no_garrison() {
    let data = game_data();
    let mut state = start(data, "fac_papacy", 1);
    let perm = fac("fac_perm");
    let capital = state
        .faction_capital_city(&perm)
        .cloned()
        .expect("Perm holds its capital city");
    // Field armies dismissed, every other garrison emptied, one unit at home.
    state.armies.retain(|_, a| a.faction != perm);
    for (id, settlement) in state.settlements.iter_mut() {
        if settlement.controller == perm && *id != capital {
            settlement.garrison.clear();
        }
    }
    let city = state.settlements.get_mut(&capital).unwrap();
    city.garrison.truncate(1);
    assert_eq!(city.garrison.len(), 1);
    assert_eq!(state.faction_upkeep(data, &perm), 0);
    // A second unit pays its share.
    let unit = state.settlements[&capital].garrison[0].clone();
    state
        .settlements
        .get_mut(&capital)
        .unwrap()
        .garrison
        .push(unit);
    assert!(state.faction_upkeep(data, &perm) > 0);
    // Lost, the capital city is no longer the guard's home.
    state.settlements.get_mut(&capital).unwrap().controller = fac("fac_golden_horde");
    assert!(state.faction_capital_city(&perm).is_none());
}

#[test]
fn a_realm_in_debt_dismisses_its_costliest_agent() {
    let data = game_data();
    let mut state = start(data, "fac_papacy", 1);
    let france = fac("fac_france");
    let paris = state.faction_capital_city(&france).cloned().expect("Paris");
    state.factions.get_mut(&france).unwrap().treasury = 50_000;
    state
        .recruit_agent(data, &france, &paris, AgentKind::Spy)
        .expect("a spy");
    state
        .recruit_agent(data, &france, &paris, AgentKind::Emissary)
        .expect("a herald");
    let dismissals = |state: &CampaignState| {
        sim_campaign::agents::plan_agents(state, data, &france)
            .into_iter()
            .filter(|o| matches!(o, Order::DismissAgent { .. }))
            .count()
    };
    assert_eq!(dismissals(&state), 0, "a solvent realm keeps its agents");
    state.factions.get_mut(&france).unwrap().treasury = -100;
    let orders = sim_campaign::agents::plan_agents(&state, data, &france);
    let dismissed: Vec<_> = orders
        .iter()
        .filter_map(|o| match o {
            Order::DismissAgent { agent } => Some(agent.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(dismissed.len(), 1, "one agent a season");
    let costliest = state
        .agents_of(&france)
        .into_iter()
        .max_by_key(|(_, a)| sim_campaign::agents::rules(data).types[&a.kind].upkeep)
        .map(|(id, _)| id.clone())
        .unwrap();
    assert_eq!(dismissed[0], costliest);
    assert!(
        !orders
            .iter()
            .any(|o| matches!(o, Order::AgentAction { agent, .. } if *agent == costliest)),
        "the dismissed agent does not act"
    );
}
