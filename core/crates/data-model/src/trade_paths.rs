//! Precomputed paths of the trade routes (review point 18c,
//! `docs/archive/chantiers.md`).
//!
//! A trade route's path only depends on the static movement graph and on
//! its two hubs, so it is computed once, when [`GameData::build_movement_graph`]
//! runs (at load, and whenever a test rebuilds the graph), instead of at
//! every seasonal resolution. `sim-campaign::trade` reads the cache through
//! [`GameData::trade_path`], which falls back to a fresh search for a pair
//! of settlements the cache does not know (a catalogue edited after load).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use crate::ids::SettlementId;
use crate::load::GameData;
use crate::movement_graph::MovementGraph;

/// Shortest path between two settlements: `(total cost, path incl. both ends)`.
pub type TradePath = (u32, Vec<SettlementId>);

/// Cached [`TradePath`] of every hub pair of the trade catalogue, keyed by
/// `(from settlement, to settlement)`; `None` when no path exists.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TradePaths {
    pub paths: BTreeMap<(SettlementId, SettlementId), Option<TradePath>>,
}

/// Integer cost of a graph edge, as armies pay it (`sim-campaign::movement::edges`).
fn edge_cost(cost: f64) -> u32 {
    (cost.round() as u32).max(1)
}

/// Shortest path between two settlements on the static movement graph
/// (Dijkstra, ties resolved by settlement id order). Returns
/// `(total cost, path incl. both ends)`, `None` when `to` is unreachable.
pub fn shortest_path(
    graph: &MovementGraph,
    from: &SettlementId,
    to: &SettlementId,
) -> Option<TradePath> {
    if from == to {
        return Some((0, vec![from.clone()]));
    }
    let mut dist: BTreeMap<SettlementId, u32> = BTreeMap::new();
    let mut prev: BTreeMap<SettlementId, SettlementId> = BTreeMap::new();
    let mut heap = BinaryHeap::new();
    dist.insert(from.clone(), 0);
    heap.push(Reverse((0u32, from.clone())));
    while let Some(Reverse((cost, current))) = heap.pop() {
        if &current == to {
            break;
        }
        if cost > *dist.get(&current).unwrap_or(&u32::MAX) {
            continue;
        }
        for edge in graph.edges(&current) {
            let next_cost = cost + edge_cost(edge.cost);
            if next_cost < dist.get(&edge.to).copied().unwrap_or(u32::MAX) {
                dist.insert(edge.to.clone(), next_cost);
                prev.insert(edge.to.clone(), current.clone());
                heap.push(Reverse((next_cost, edge.to.clone())));
            }
        }
    }
    let cost = *dist.get(to)?;
    let mut path = vec![to.clone()];
    let mut current = to.clone();
    while &current != from {
        let previous = prev.get(&current)?;
        path.push(previous.clone());
        current = previous.clone();
    }
    path.reverse();
    Some((cost, path))
}

impl GameData {
    /// (Re)computes [`GameData::trade_paths`] for every route of the trade
    /// catalogue. Called by [`GameData::build_movement_graph`].
    pub fn build_trade_paths(&mut self) {
        let mut cache = TradePaths::default();
        if let Some(catalog) = &self.trade {
            for route in &catalog.routes {
                let (Some(from), Some(to)) =
                    (catalog.hub(&route.from_hub), catalog.hub(&route.to_hub))
                else {
                    continue;
                };
                let key = (from.settlement.clone(), to.settlement.clone());
                if cache.paths.contains_key(&key) {
                    continue;
                }
                let path = shortest_path(&self.movement_graph, &key.0, &key.1);
                cache.paths.insert(key, path);
            }
        }
        self.trade_paths = cache;
    }

    /// Path of a trade route between two settlements: the precomputed one
    /// when known, a fresh search otherwise.
    pub fn trade_path(&self, from: &SettlementId, to: &SettlementId) -> Option<TradePath> {
        match self.trade_paths.paths.get(&(from.clone(), to.clone())) {
            Some(cached) => cached.clone(),
            None => shortest_path(&self.movement_graph, from, to),
        }
    }
}
