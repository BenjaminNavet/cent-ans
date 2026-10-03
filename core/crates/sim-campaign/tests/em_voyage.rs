//! EM (ADR 0167): voyages of several legs from a port, landing at any port
//! reached this season, click on the water.

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementId};
use sim_campaign::orders::OrderError;
use sim_campaign::voyage::{port_near, sea_voyage, sea_voyages};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn set(id: &str) -> SettlementId {
    SettlementId::new(id).unwrap()
}

/// England's strongest army in `port`, full movement, no foreign fleet.
fn english_army_at(data: &GameData, port: &str) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 7).unwrap();
    let army = state
        .armies()
        .iter()
        .filter(|(_, a)| a.faction == fac("fac_england"))
        .max_by_key(|(id, a)| (a.total_strength(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .unwrap();
    let entry = state.armies.get_mut(&army).unwrap();
    entry.position = ArmyPosition::Settlement(set(port));
    entry.movement_left = u32::MAX / 2;
    state.naval.ensure(data);
    for (faction, pool) in state.naval.fleets.iter_mut() {
        if faction != &fac("fac_england") {
            pool.clear();
        }
    }
    (state, army)
}

#[test]
fn a_port_reaches_ports_beyond_its_direct_lanes() {
    let data = data();
    let (state, _) = english_army_at(&data, "set_southampton");
    let england = fac("fac_england");
    let voyages = sea_voyages(&state, &data, &england, &set("set_southampton"));
    let max_legs = data.free_movement_rules().max_voyage_legs as usize;
    assert!(max_legs > 1, "data allows multi-leg voyages");
    assert!(voyages
        .values()
        .all(|v| !v.is_empty() && v.len() <= max_legs));
    assert!(
        voyages.values().any(|v| v.len() >= 2),
        "some port needs a port of call: {voyages:?}"
    );
    // Each leg is a sea edge of the graph.
    for (port, voyage) in &voyages {
        assert_eq!(voyage.last(), Some(port));
        let mut from = set("set_southampton");
        for leg in voyage {
            assert!(data.movement_graph.edge(&from, leg).is_some_and(|e| e.sea));
            from = leg.clone();
        }
    }
}

#[test]
fn an_army_sails_through_ports_of_call_in_one_season() {
    let data = data();
    let (mut state, army) = english_army_at(&data, "set_southampton");
    let england = fac("fac_england");
    let (target, voyage) = sea_voyages(&state, &data, &england, &set("set_southampton"))
        .into_iter()
        .find(|(port, v)| v.len() >= 2 && !state.is_hostile_settlement(&england, port))
        .expect("a friendly or neutral port two legs away");
    state
        .submit_order(
            &data,
            Order::Embark {
                army: army.clone(),
                to_port: target.clone(),
            },
        )
        .unwrap();
    let entry = &state.armies[&army];
    assert_eq!(entry.settlement(), Some(&target), "voyage {voyage:?}");
    assert_eq!(entry.movement_left, 0);
}

#[test]
fn embarking_needs_the_whole_season() {
    let data = data();
    let (mut state, army) = english_army_at(&data, "set_southampton");
    state.armies.get_mut(&army).unwrap().movement_left = 1;
    let err = state
        .submit_order(
            &data,
            Order::Embark {
                army: army.clone(),
                to_port: set("set_bordeaux"),
            },
        )
        .unwrap_err();
    assert!(matches!(err, OrderError::EmbarkNeedsFullTurn), "{err:?}");
}

#[test]
fn an_inland_town_has_no_sea_route() {
    let data = data();
    let (mut state, army) = english_army_at(&data, "set_southampton");
    let england = fac("fac_england");
    assert!(sea_voyage(
        &state,
        &data,
        &england,
        &set("set_southampton"),
        &set("set_paris")
    )
    .is_none());
    let err = state
        .submit_order(
            &data,
            Order::Embark {
                army,
                to_port: set("set_paris"),
            },
        )
        .unwrap_err();
    assert!(matches!(err, OrderError::NoSeaRoute { .. }), "{err:?}");
}

#[test]
fn a_click_on_the_water_picks_the_nearest_reachable_port() {
    let data = data();
    let (state, _) = english_army_at(&data, "set_southampton");
    let england = fac("fac_england");
    let bordeaux = data.settlement_point(&set("set_bordeaux")).unwrap();
    let near = port_near(
        &state,
        &data,
        &england,
        &set("set_southampton"),
        [bordeaux[0] - 3.0, bordeaux[1] + 2.0],
    );
    assert_eq!(near, Some(set("set_bordeaux")));
}
