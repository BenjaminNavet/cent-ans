//! The settlement graph: edge costs and Dijkstra tables.

use std::collections::BTreeMap;

use crate::state::CampaignState;
use data_model::pathfinding::{self, Label, Visit};
use data_model::{FactionId, GameData, ProvinceId, SettlementId, Terrain};

/// True when `from` → `to` is a sea crossing rather than a land step.
pub fn is_sea_crossing(data: &GameData, from: &SettlementId, to: &SettlementId) -> bool {
    data.movement_graph
        .edge(from, to)
        .is_some_and(|edge| edge.sea)
}

/// Cost multiplier of a terrain (v1: 2 for mountains and marshes).
pub fn terrain_cost(terrain: Terrain) -> u32 {
    data_model::terrain_cost(terrain) as u32
}

/// Land neighbours of a province (geometry graph first, entity data otherwise).
pub fn land_neighbors<'a>(data: &'a GameData, id: &ProvinceId) -> &'a [ProvinceId] {
    data.province_land_neighbors(id)
}

/// Provinces reachable by sea from `id` (`sea_neighbors` of the geometry).
pub fn sea_neighbors(data: &GameData, id: &ProvinceId) -> Vec<ProvinceId> {
    data.province_sea_neighbors(id)
}

/// Movement points one v1 province step is worth.
pub fn points_per_step(data: &GameData) -> f64 {
    data.movement_rules().points_per_step
}

/// Movement cost from `from` to the adjacent `to`, or `None` when not connected.
pub fn edge_cost(data: &GameData, from: &SettlementId, to: &SettlementId) -> Option<u32> {
    if from == to {
        return None;
    }
    data.movement_graph
        .edge(from, to)
        .map(|edge| (edge.cost.round() as u32).max(1))
}

/// All edges leaving `from`, with their costs, in id order.
pub fn edges(data: &GameData, from: &SettlementId) -> Vec<(SettlementId, u32)> {
    data.movement_graph
        .edges(from)
        .iter()
        .map(|edge| (edge.to.clone(), (edge.cost.round() as u32).max(1)))
        .collect()
}

/// Dijkstra result: cost to reach each settlement and the settlement before it.
pub type Reach = Label<SettlementId>;

/// Shortest paths from `start` for an army of `faction`.
///
/// Expansion stops at settlements holding an army hostile to `faction` or
/// held by a faction at war with it (an army entering them must fight or
/// besiege, and stops). `budget` bounds the total cost when given; `cap`
/// clamps each edge (an army's full seasonal allowance).
pub fn dijkstra(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    start: &SettlementId,
    budget: Option<u32>,
    cap: Option<u32>,
) -> BTreeMap<SettlementId, Reach> {
    let mut best = BTreeMap::new();
    pathfinding::dijkstra(
        &mut best,
        start.clone(),
        budget,
        |current| {
            let blocked = current != start
                && (state.is_hostile_settlement(faction, current)
                    || !state.hostile_armies_at(faction, current).is_empty());
            if blocked {
                Visit::Skip
            } else {
                Visit::Expand
            }
        },
        |current, out| {
            out.extend(
                edges(data, current)
                    .into_iter()
                    .map(|(next, edge)| (next, cap.map_or(edge, |c| edge.min(c.max(1))))),
            );
        },
    );
    best
}

/// Rebuilds the path (start excluded, target included) from a Dijkstra table.
pub fn path_to(
    table: &BTreeMap<SettlementId, Reach>,
    target: &SettlementId,
) -> Option<Vec<SettlementId>> {
    pathfinding::path_to(table, target)
}
