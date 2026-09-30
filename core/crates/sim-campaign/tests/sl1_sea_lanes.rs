//! SL1 (ADR 0117): sea lanes — graph edges, crossings, gales, interception
//! seas, trade cut by enemy squadrons, map view.

use std::path::PathBuf;

use data_model::{FactionId, GameData, SeaLaneKind, SeaZoneId, SettlementId};
use sim_campaign::naval::{crossing_sea, SeaControl};
use sim_campaign::sea_lanes::{sea_lanes, storm_loss_percent, trade_sea_legs};
use sim_campaign::state::Season;
use sim_campaign::trade::trade_routes;
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

fn biscay() -> SeaZoneId {
    SeaZoneId::new("sea_bay_of_biscay").unwrap()
}

/// England's strongest army standing in Southampton, no enemy fleet at sea.
fn english_army_at(data: &GameData, port: &str, season: Season) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 7).unwrap();
    state.season = season;
    let army = state
        .armies()
        .iter()
        .filter(|(_, a)| a.faction == fac("fac_england"))
        .max_by_key(|(id, a)| (a.total_strength(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .unwrap();
    let entry = state.armies.get_mut(&army).unwrap();
    entry.position = ArmyPosition::Settlement(set(port));
    state.naval.ensure(data);
    for (faction, pool) in state.naval.fleets.iter_mut() {
        if faction != &fac("fac_england") {
            pool.clear();
        }
    }
    (state, army)
}

fn full_movement(state: &mut CampaignState, data: &GameData, army: &ArmyId) {
    let _ = data;
    let entry = state.armies.get_mut(army).unwrap();
    entry.movement_left = u32::MAX / 2;
}

#[test]
fn every_lane_is_a_sea_edge_priced_by_its_length() {
    let data = data();
    assert!(data.naval.sea_lanes.lanes.len() >= 20);
    for lane in &data.naval.sea_lanes.lanes {
        let edge = data
            .movement_graph
            .edge(&lane.from, &lane.to)
            .unwrap_or_else(|| panic!("{} missing from the graph", lane.id));
        assert!(edge.sea, "{}", lane.id);
        assert!(
            data.naval.lane_lengths_km.contains_key(&lane.id),
            "{} not routed (cent-ans geo sea-lanes)",
            lane.id
        );
        assert!(edge.cost >= 140.0 - 1e-6, "{} costs {}", lane.id, edge.cost);
    }
    // The Gascon wine route: ~1 000 km of sea, half the price of a march.
    let gascony = data
        .movement_graph
        .edge(&set("set_southampton"), &set("set_bordeaux"))
        .unwrap();
    assert!((400.0..650.0).contains(&gascony.cost), "{}", gascony.cost);
}

#[test]
fn an_english_army_sails_from_southampton_to_bordeaux_in_one_season() {
    let data = data();
    let (mut state, army) = english_army_at(&data, "set_southampton", Season::Summer);
    full_movement(&mut state, &data, &army);
    let before = state.armies[&army].total_strength();
    state
        .submit_order(
            &data,
            Order::Embark {
                army: army.clone(),
                to_port: set("set_bordeaux"),
            },
        )
        .unwrap();
    let entry = &state.armies[&army];
    assert_eq!(entry.settlement(), Some(&set("set_bordeaux")));
    assert_eq!(entry.movement_left, 0);
    // Summer, open sea: no gale.
    assert_eq!(entry.total_strength(), before);
}

#[test]
fn winter_gales_cost_men_on_the_open_sea_more_than_along_the_coast() {
    let data = data();
    let (state, _) = english_army_at(&data, "set_southampton", Season::Winter);
    let open = storm_loss_percent(&state, &data, &set("set_southampton"), &set("set_bordeaux"));
    let coast = storm_loss_percent(&state, &data, &set("set_bordeaux"), &set("set_bayonne"));
    assert!(open >= 8.0, "{open}");
    assert!(coast > 0.0 && coast < open, "{coast} vs {open}");

    let (mut state, army) = english_army_at(&data, "set_southampton", Season::Winter);
    full_movement(&mut state, &data, &army);
    let before = state.armies[&army].total_strength();
    state
        .submit_order(
            &data,
            Order::Embark {
                army: army.clone(),
                to_port: set("set_bordeaux"),
            },
        )
        .unwrap();
    let after = state.armies[&army].total_strength();
    let lost = f64::from(before - after) / f64::from(before) * 100.0;
    assert!(
        lost > open * 0.7 && lost <= open + 0.01,
        "{lost}% for {open}%"
    );
    assert!(state
        .pending_events
        .iter()
        .any(|e| e.text_fr.contains("Gros temps")));
}

#[test]
fn a_lane_crossing_is_fought_on_the_lane_sea() {
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_england"), 1).unwrap();
    assert_eq!(
        crossing_sea(&state, &data, &set("set_southampton"), &set("set_bordeaux")),
        Some(biscay())
    );
    let lane = data
        .naval
        .lane_between(&set("set_sandwich"), &set("set_sluis"))
        .unwrap();
    assert_eq!(lane.kind, SeaLaneKind::OpenSea);
}

#[test]
fn french_squadrons_in_biscay_cut_the_gascon_wine_trade() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 3).unwrap();
    state.season = Season::Summer;
    let route = |state: &CampaignState| {
        trade_routes(state, &data)
            .into_iter()
            .find(|r| r.id == "route_bordeaux_londres")
            .unwrap()
    };
    let free = route(&state);
    assert!(
        free.path.windows(2).any(|w| data
            .movement_graph
            .edge(&w[0], &w[1])
            .is_some_and(|e| e.sea)),
        "the wine goes by sea: {:?}",
        free.path
    );
    assert!(!free.cut, "{:?}", free.cut_reason);

    state.naval.ensure(&data);
    let sea = crossing_sea(&state, &data, &free.path[0], &free.path[1])
        .or_else(|| {
            free.path.windows(2).find_map(|w| {
                data.movement_graph
                    .edge(&w[0], &w[1])
                    .filter(|e| e.sea)
                    .and_then(|_| crossing_sea(&state, &data, &w[0], &w[1]))
            })
        })
        .unwrap();
    assert!(state.is_at_war(&fac("fac_england"), &fac("fac_france")));
    state.naval.control.insert(
        sea.clone(),
        SeaControl {
            faction: fac("fac_france"),
            level: 30,
        },
    );
    let harried = route(&state);
    assert!(!harried.cut);
    assert!(
        harried.security < free.security,
        "{} vs {}",
        harried.security,
        free.security
    );
    assert!(harried.total_value() < free.total_value());

    state.naval.control.insert(
        sea,
        SeaControl {
            faction: fac("fac_france"),
            level: 100,
        },
    );
    let held = route(&state);
    assert!(held.cut);
    assert!(
        held.cut_reason
            .as_deref()
            .is_some_and(|r| r.contains("tenue")),
        "{:?}",
        held.cut_reason
    );
}

#[test]
fn winter_lowers_sea_trade_only() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 3).unwrap();
    let path: Vec<SettlementId> = vec![set("set_southampton"), set("set_bordeaux")];
    state.season = Season::Summer;
    let summer = trade_sea_legs(
        &state,
        &data,
        &path,
        &fac("fac_england"),
        &fac("fac_england"),
    );
    state.season = Season::Winter;
    let winter = trade_sea_legs(
        &state,
        &data,
        &path,
        &fac("fac_england"),
        &fac("fac_england"),
    );
    assert_eq!(summer.season_factor, 1.0);
    assert!(winter.season_factor < 0.8);
    let land = vec![set("set_bordeaux"), set("set_libourne")];
    let dry = trade_sea_legs(
        &state,
        &data,
        &land,
        &fac("fac_england"),
        &fac("fac_england"),
    );
    assert_eq!(dry.season_factor, 1.0);
}

#[test]
fn the_map_view_names_lanes_their_sea_and_trade() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 3).unwrap();
    state.naval.ensure(&data);
    state.naval.control.insert(
        biscay(),
        SeaControl {
            faction: fac("fac_france"),
            level: 70,
        },
    );
    let views = sea_lanes(&state, &data, &fac("fac_england"));
    assert_eq!(views.len(), data.naval.sea_lanes.lanes.len());
    let wine = views
        .iter()
        .find(|v| v.id == "lane_southampton_bordeaux")
        .unwrap();
    assert_eq!(wine.sea_name, "le golfe de Gascogne");
    assert!(wine.hostile && wine.blockade);
    assert!(wine.length_km > 800.0);
    assert!(views.iter().any(|v| !v.trade_routes.is_empty()));
}
