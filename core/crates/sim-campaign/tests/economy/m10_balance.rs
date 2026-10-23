//! M10 balance rules: disbanding a whole army, opulent courts, vanished
//! factions leave no war behind.

use sim_campaign::test_support::idle;
use sim_campaign::{CampaignState, Order};

use data_model::test_support::{fac, game_data};

#[test]
fn dismissing_the_last_unit_disbands_the_army() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let (army_id, size, general) = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == france)
        .map(|(id, a)| (id.clone(), a.units.len(), a.general.clone()))
        .unwrap();
    for _ in 0..size {
        state
            .submit_order(
                data,
                Order::DisbandUnit {
                    army: Some(army_id.clone()),
                    settlement: None,
                    unit_index: 0,
                },
            )
            .unwrap();
    }
    assert!(!state.armies.contains_key(&army_id));
    if let Some(general) = general {
        assert!(state.characters[&general].army.is_none());
    }
}

#[test]
fn a_hoarded_treasury_feeds_the_court() {
    let data = game_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    let base = state.faction_administration_upkeep(data, &france);
    let income = state.faction_income(data, &france);
    state.factions.get_mut(&france).unwrap().treasury =
        data.economy_rules.opulence_seasons * income + 1_000_000;
    let rich = state.faction_administration_upkeep(data, &france);
    let rules = &data.economy_rules;
    assert_eq!(
        rich - base,
        rules.opulence(state.factions[&france].treasury, income)
    );
    assert!(rich - base >= 1_000_000 * rules.opulence_percent / 100);
}

/// ECO (ADR 0749): the idle-hoard drain is nil within the reserve and grows
/// faster than the hoard (each tier adds its share), so no balance is stable
/// above the point where the drain equals the surplus.
#[test]
fn the_hoard_drain_is_progressive() {
    let rules = &game_data().economy_rules;
    let income = 10_000;
    let reserve = rules.opulence_seasons * income;
    assert_eq!(rules.opulence(reserve, income), 0);
    assert_eq!(rules.opulence(0, income), 0);
    let drain = |seasons: i64| rules.opulence(seasons * income, income);
    let mut previous_rate = 0.0;
    for seasons in 3..=20 {
        let rate = drain(seasons) as f64 / (seasons * income) as f64;
        assert!(
            rate >= previous_rate,
            "drain share must not fall: {seasons} seasons -> {rate}"
        );
        previous_rate = rate;
    }
    // A hoard of twelve seasons loses at least a quarter of its income value
    // per season: the realm cannot sit on it.
    assert!(drain(12) >= 3 * income);
}

#[test]
fn a_vanished_faction_leaves_no_war_or_alliance() {
    let data = game_data();
    let france = fac("fac_france");
    let navarre = fac("fac_navarre");
    let mut state = CampaignState::new_1337(data, france.clone(), 1).unwrap();
    state
        .submit_order(
            data,
            Order::DeclareWar {
                target: navarre.clone(),
            },
        )
        .unwrap();
    assert!(state.factions[&france].at_war_with.contains(&navarre));
    // Navarre loses everything to France.
    for settlement in state.settlements.values_mut() {
        if settlement.controller == navarre {
            settlement.controller = france.clone();
            settlement.owner = france.clone();
        }
    }
    state.armies.retain(|_, a| a.faction != navarre);
    state.end_turn_with(data, idle);
    assert!(!state.factions[&navarre].alive);
    for faction in state.factions.values() {
        assert!(!faction.at_war_with.contains(&navarre));
        assert!(!faction.allies.contains(&navarre));
    }
}

#[test]
fn landing_on_a_hostile_shore_costs_men_and_movement() {
    use sim_campaign::movement::{edge_cost, edges, is_sea_crossing};
    let mut data = game_data().clone();
    // Landing cost only: no interception at sea, whatever the RNG stream.
    std::sync::Arc::make_mut(&mut data.naval.rules).intercept_base = 0.0;
    std::sync::Arc::make_mut(&mut data.naval.rules).intercept_per_control = 0.0;
    let england = fac("fac_england");
    let mut state = CampaignState::new_1337(&data, england.clone(), 1).unwrap();
    // An English port facing a hostile (French) port across the sea.
    let (from, to) = state
        .settlements
        .iter()
        .filter(|(_, s)| s.controller == england)
        .flat_map(|(id, _)| {
            edges(&data, id)
                .into_iter()
                .map(move |(n, _)| (id.clone(), n))
        })
        .find(|(from, to)| {
            is_sea_crossing(&data, from, to)
                && edge_cost(&data, from, to).is_some()
                && state
                    .settlement_province(to)
                    .is_some_and(|p| state.is_hostile_territory(&england, p))
                && state.hostile_armies_at(&england, to).is_empty()
        })
        .expect("an English port faces a hostile one");
    let army_id = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == england)
        .map(|(id, _)| id.clone())
        .unwrap();
    {
        let army = state.armies.get_mut(&army_id).unwrap();
        army.position = sim_campaign::ArmyPosition::Settlement(from.clone());
        army.clear_plan();
    }
    let before: u32 = state.armies[&army_id]
        .units
        .iter()
        .map(|u| u.strength)
        .sum();
    // Lot M2: a crossing is an `embark` order, resolved at once.
    state
        .submit_order(
            &data,
            Order::Embark {
                army: army_id.clone(),
                to_port: to.clone(),
            },
        )
        .unwrap();
    let events = std::mem::take(&mut state.pending_events);
    assert_eq!(state.armies[&army_id].settlement().cloned().unwrap(), to);
    let after: u32 = state.armies[&army_id]
        .units
        .iter()
        .map(|u| u.strength)
        .sum();
    assert!(after < before, "landing losses: {before} -> {after}");
    assert!(events.iter().any(|e| e.text_fr.contains("Débarquement")));
}
