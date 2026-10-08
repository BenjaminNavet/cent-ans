//! Precomputed paths of the trade routes (review point 18c,
//! `docs/archive/chantiers.md`).
//!
//! A trade route's path only depends on the static movement graph and on
//! its two hubs, so it is computed once, when [`GameData::build_movement_graph`]
//! runs (at load, and whenever a test rebuilds the graph), instead of at
//! every seasonal resolution. `sim-campaign::trade` reads the cache through
//! [`GameData::trade_path`], which falls back to a fresh search for a pair
//! of settlements the cache does not know (a catalogue edited after load).

use std::collections::BTreeMap;

use crate::pathfinding::{dijkstra, path_to, Label, Visit};

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
    let mut labels: BTreeMap<SettlementId, Label<SettlementId>> = BTreeMap::new();
    dijkstra(
        &mut labels,
        from.clone(),
        None,
        |node| {
            if node == to {
                Visit::Stop
            } else {
                Visit::Expand
            }
        },
        |node, out| {
            out.extend(
                graph
                    .edges(node)
                    .iter()
                    .map(|edge| (edge.to.clone(), edge_cost(edge.cost))),
            );
        },
    );
    let cost = labels.get(to)?.cost;
    let mut path = vec![from.clone()];
    path.extend(path_to(&labels, to)?);
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
