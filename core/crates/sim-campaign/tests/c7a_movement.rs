//! Lot C7a: an army covers one or two neighbouring provinces per season
//! (`rules.json` `movement.season_scale` and `road_cost_factor`).

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementId};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

#[test]
fn road_edges_are_rescaled_to_the_rules_factor() {
    let data = data();
    let rules = data.movement_rules();
    let raw = data
        .settlement_graph
        .iter()
        .find(|e| e.road && !e.sea && e.cost > 10.0)
        .expect("a road edge");
    let edge = data.movement_graph.edge(&raw.from, &raw.to).unwrap();
    let expected = raw.cost * rules.road_cost_factor / 0.5;
    assert!((edge.cost - expected).abs() < 1e-6 || edge.cost < expected);
}

#[test]
fn a_season_covers_one_or_two_provinces_from_paris() {
    let data = data();
    let france = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let paris = SettlementId::new("set_paris").unwrap();
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == france)
        .map(|(id, _)| id.clone())
        .unwrap();
    let a = state.armies.get_mut(&army).unwrap();
    a.location = paris.clone();
    a.general = None;
    a.units.truncate(1);
    let allowance = state.army_movement_allowance(&data, &state.armies[&army]);
    assert_eq!(allowance, state.season_movement_points(&data));
    // 1.5 v1 province steps at most outside winter.
    assert!(f64::from(allowance) <= 1.5 * data.movement_rules().points_per_step + 1.0);
    state.armies.get_mut(&army).unwrap().movement_points = allowance;
    let reach = state.reachable(&data, &army);
    assert!(
        reach.len() < 100,
        "{} settlements in one season from Paris",
        reach.len()
    );
    assert!(!reach.contains_key(&SettlementId::new("set_villeneuve_sur_lot").unwrap()));
}
