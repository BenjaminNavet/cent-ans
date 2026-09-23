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
                    province: None,
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
    for province in state.provinces.values_mut() {
        if province.controller == navarre {
            province.controller = france.clone();
            province.owner = france.clone();
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
