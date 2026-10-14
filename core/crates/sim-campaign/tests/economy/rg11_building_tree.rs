//! Lot RG11 (ADR 0345): the construction tree getter agrees with
//! `buildable` and `Order::Build`, and gives plain-French lock reasons.

use data_model::test_support::game_data;
use data_model::{FactionId, GameData, SettlementId, SettlementKind};
use sim_campaign::building_tree::{BuildingTree, NodeState};
use sim_campaign::{CampaignState, Order, Place};

fn france() -> FactionId {
    FactionId::new("fac_france").unwrap()
}

fn setup(data: &GameData, kind: SettlementKind) -> (CampaignState, SettlementId) {
    let mut state = CampaignState::new_1337(data, france(), 7).expect("1337 start");
    let id = state
        .settlements
        .iter()
        .find(|(_, s)| s.kind == kind && s.owner == france() && s.controller == france())
        .map(|(id, _)| id.clone())
        // CC C3: the five villages left are not French; fall back to any held one.
        .or_else(|| {
            state
                .settlements
                .iter()
                .find(|(_, s)| s.kind == kind && s.owner == s.controller)
                .map(|(id, _)| id.clone())
        })
        .expect("a settlement of that kind");
    let place = state.settlements.get_mut(&id).unwrap();
    place.construction = None;
    place.build_queue.clear();
    state.factions.get_mut(&france()).unwrap().treasury = 1_000_000;
    (state, id)
}

fn tree(state: &CampaignState, data: &GameData, id: &SettlementId) -> BuildingTree {
    state.building_tree(data, id).expect("known settlement")
}

#[test]
fn unknown_settlement_has_no_tree() {
    let data = game_data();
    let (state, _) = setup(data, SettlementKind::Town);
    assert!(state
        .building_tree(data, &SettlementId::new("set_nowhere").unwrap())
        .is_none());
}

#[test]
fn tree_covers_every_building_and_matches_buildable() {
    let data = game_data();
    let (state, id) = setup(data, SettlementKind::City);
    let tree = tree(&state, data, &id);
    assert_eq!(tree.nodes.len(), data.buildings.len());
    assert_eq!(tree.kind, SettlementKind::City);
    assert_eq!(tree.queue_size, 3);
    let options = state.buildable(data, &id);
    let standing = &state.settlement_state(&id).unwrap().buildings;
    for node in &tree.nodes {
        assert!(!node.requirements.is_empty());
        assert_eq!(node.current, standing.contains(&node.building));
        match node.state {
            NodeState::Built => assert!(data.has_building(standing, &node.building)),
            NodeState::Locked => {
                assert!(
                    !node.reason.is_empty(),
                    "{} locked without reason",
                    node.building
                );
                assert!(node.reason.starts_with("Requiert"), "{}", node.reason);
                assert!(!node.lock.is_empty());
                assert!(!node.can_build);
                if let Some(option) = options.iter().find(|o| o.building == node.building) {
                    assert!(!option.available, "{} locked but buildable", node.building);
                }
            }
            NodeState::Available => {
                assert!(node.reason.is_empty());
                let option = options
                    .iter()
                    .find(|o| o.building == node.building)
                    .unwrap();
                assert_eq!(node.can_build, option.available, "{}", node.building);
                assert_eq!(node.cost, option.cost);
                assert_eq!(node.turns, option.turns);
            }
            NodeState::Building | NodeState::Queued => assert!(node.queue_index >= 0),
        }
    }
    // Every building `buildable` accepts is a buildable node of the tree.
    for option in options.iter().filter(|o| o.available) {
        let node = tree
            .nodes
            .iter()
            .find(|n| n.building == option.building)
            .unwrap();
        assert_eq!(node.state, NodeState::Available, "{}", option.building);
        assert!(node.can_build);
    }
}

#[test]
fn lock_reasons_are_plain_french() {
    let data = game_data();
    let (state, id) = setup(data, SettlementKind::Village);
    let village_tree = tree(&state, data, &id);
    // A village cannot hold a market: rank.
    let market = village_tree
        .nodes
        .iter()
        .find(|n| n.building.as_str() == "bld_market")
        .unwrap();
    assert_eq!(market.state, NodeState::Locked);
    assert_eq!(market.lock, "rank");
    assert_eq!(market.reason, "Requiert le rang Cité ou Ville");
    assert!(!market.allowed_here);

    let (state, id) = setup(data, SettlementKind::City);
    let city_tree = tree(&state, data, &id);
    let by_lock = |lock: &str| city_tree.nodes.iter().find(|n| n.lock == lock);
    let tech = by_lock("technology").expect("a technology-locked building in a city");
    assert!(
        tech.reason.starts_with("Requiert la technologie "),
        "{}",
        tech.reason
    );
    let tech_req = tech
        .requirements
        .iter()
        .find(|r| r.kind == "technology")
        .unwrap();
    assert!(!tech_req.met);
    let chain = by_lock("chain").expect("an upgrade whose previous step is missing");
    assert!(chain.parent.is_some());
    assert!(chain.reason.starts_with("Requiert "), "{}", chain.reason);
}

#[test]
fn build_order_moves_node_to_building_then_queue() {
    let data = game_data();
    let (mut state, id) = setup(data, SettlementKind::Town);
    let first = tree(&state, data, &id)
        .nodes
        .iter()
        .filter(|n| n.can_build && n.cost > 0)
        .map(|n| n.building.clone())
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(first.len(), 2, "two buildable buildings in a town");
    for building in &first {
        state
            .submit_order(
                data,
                Order::Build {
                    settlement: Place::Settlement(id.clone()),
                    building: building.clone(),
                },
            )
            .expect("build order");
    }
    let tree = tree(&state, data, &id);
    let node = |b: &data_model::BuildingId| tree.nodes.iter().find(|n| &n.building == b).unwrap();
    assert_eq!(node(&first[0]).state, NodeState::Building);
    assert_eq!(node(&first[0]).queue_index, 0);
    assert!(node(&first[0]).turns_left > 0);
    assert!(node(&first[0]).total_turns >= node(&first[0]).turns_left);
    assert_eq!(node(&first[1]).state, NodeState::Queued);
    assert_eq!(node(&first[1]).queue_index, 1);
    assert_eq!(tree.queue.len(), 2);
    assert_eq!(tree.queue[0].building, first[0]);
    assert!(!node(&first[0]).can_build);
}

#[test]
fn money_refusal_is_not_a_lock() {
    let data = game_data();
    let (mut state, id) = setup(data, SettlementKind::Town);
    state.factions.get_mut(&france()).unwrap().treasury = 0;
    let tree = tree(&state, data, &id);
    let poor = tree
        .nodes
        .iter()
        .find(|n| n.state == NodeState::Available && n.cost > 0)
        .expect("still available, only too dear");
    assert!(!poor.can_build);
    assert!(
        poor.blocked_now.starts_with("trésor insuffisant"),
        "{}",
        poor.blocked_now
    );
    assert!(poor.reason.is_empty());
}
