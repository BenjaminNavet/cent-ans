//! M10 balance rules: disbanding a whole army, opulent courts, vanished
//! factions leave no war behind.

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::economy::{OPULENCE_PERCENT, OPULENCE_SEASONS};
use sim_campaign::{CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

#[test]
fn dismissing_the_last_unit_disbands_the_army() {
    let data = data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let (army_id, size, general) = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == france)
        .map(|(id, a)| (id.clone(), a.units.len(), a.general.clone()))
        .unwrap();
    for _ in 0..size {
        state
            .submit_order(
                &data,
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
    let data = data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let base = state.faction_administration_upkeep(&data, &france);
    let income = state.faction_income_effective(&data, &france);
    state.factions.get_mut(&france).unwrap().treasury = OPULENCE_SEASONS * income + 1_000_000;
    let rich = state.faction_administration_upkeep(&data, &france);
    assert_eq!(rich - base, 1_000_000 * OPULENCE_PERCENT / 100);
}

#[test]
fn a_vanished_faction_leaves_no_war_or_alliance() {
    let data = data();
    let france = fac("fac_france");
    let navarre = fac("fac_navarre");
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    state
        .submit_order(
            &data,
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
    state.end_turn_with(&data, idle);
    assert!(!state.factions[&navarre].alive);
    for faction in state.factions.values() {
        assert!(!faction.at_war_with.contains(&navarre));
        assert!(!faction.allies.contains(&navarre));
    }
}

#[test]
fn landing_on_a_hostile_shore_costs_men_and_movement() {
    use sim_campaign::movement::{edge_cost, edges, is_sea_crossing};
    let data = data();
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
        army.location = from.clone();
        army.path.clear();
    }
    let before: u32 = state.armies[&army_id]
        .units
        .iter()
        .map(|u| u.strength)
        .sum();
    state
        .submit_order(&data, Order::move_along(army_id.clone(), vec![to.clone()]))
        .unwrap();
    let events = state.end_turn_with(&data, idle);
    assert_eq!(state.armies[&army_id].location, to);
    let after: u32 = state.armies[&army_id]
        .units
        .iter()
        .map(|u| u.strength)
        .sum();
    assert!(after < before, "landing losses: {before} -> {after}");
    assert!(events.iter().any(|e| e.text_fr.contains("Débarquement")));
}
