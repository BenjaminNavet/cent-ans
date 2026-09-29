//! OMR R1: the agents' search over dense indices gives the tables and paths
//! of the search over settlement ids, on the real map.

use std::path::PathBuf;

use data_model::GameData;
use sim_campaign::agents::{agent_dijkstra, agent_dijkstra_by_ids, AgentTable};
use sim_campaign::movement;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

#[test]
fn indexed_search_matches_the_search_by_ids() {
    let data = data();
    let ids: Vec<_> = data.settlements.keys().cloned().collect();
    assert!(ids.len() > 1000);
    let mut checked = 0;
    for (n, start) in ids.iter().enumerate().step_by(97) {
        let cap = [40, 90, 150, 400][n % 4];
        for budget in [None, Some(0), Some(120), Some(600)] {
            let slow = agent_dijkstra_by_ids(&data, start, budget, cap);
            let fast = agent_dijkstra(&data, start, budget, cap);
            assert_eq!(fast, slow, "{start} {budget:?} {cap}");
            checked += 1;
        }
        let full = agent_dijkstra_by_ids(&data, start, None, cap);
        let table = AgentTable::search(&data, start, None, cap, None);
        for target in ids.iter().step_by(31) {
            assert_eq!(
                table.cost(target),
                full.get(target).map(|r| r.cost),
                "{start} -> {target}"
            );
            let path = movement::path_to(&full, target);
            assert_eq!(table.path_to(target), path, "{start} -> {target}");
            // Stopped at the target: the same path.
            let early = AgentTable::search(&data, start, None, cap, Some(target));
            assert_eq!(early.path_to(target), path, "{start} -> {target} (early)");
        }
    }
    assert!(checked > 40);
}

#[test]
fn unknown_start_holds_itself_alone() {
    let data = data();
    let nowhere = data_model::SettlementId::new("set_nowhere").unwrap();
    let slow = agent_dijkstra_by_ids(&data, &nowhere, None, 100);
    assert_eq!(agent_dijkstra(&data, &nowhere, None, 100), slow);
    let table = AgentTable::search(&data, &nowhere, None, 100, None);
    assert_eq!(table.cost(&nowhere), Some(0));
    assert_eq!(table.path_to(&nowhere), Some(Vec::new()));
    let elsewhere = data.settlements.keys().next().unwrap();
    assert_eq!(table.path_to(elsewhere), None);
}
