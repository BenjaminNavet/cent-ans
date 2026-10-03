//! Sea voyages (lot EM, ADR 0167).
//!
//! An army standing in a port sails, in one season, to any port reached by
//! a chain of at most `max_voyage_legs` sea edges of the settlement graph
//! (lanes of SL1 and short passages of C3). It puts in on the way only at
//! ports not held by an enemy; the destination itself may be hostile (a
//! landing). Each leg risks interception and gales like a single crossing
//! (`march::order_embark`).

use std::collections::BTreeMap;

use data_model::{FactionId, GameData, SettlementId};

use crate::state::CampaignState;

/// Every port `faction` can sail to from `from` this season, with the
/// cheapest voyage to it (`from` excluded, destination last).
pub fn sea_voyages(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    from: &SettlementId,
) -> BTreeMap<SettlementId, Vec<SettlementId>> {
    let max_legs = data.free_movement_rules().max_voyage_legs.max(1);
    // Bellman-Ford by number of legs: `best` holds the cheapest voyage found
    // so far to each port, `frontier` the ports improved by the last leg.
    let mut best: BTreeMap<SettlementId, (f64, Vec<SettlementId>)> = BTreeMap::new();
    let mut frontier: BTreeMap<SettlementId, (f64, Vec<SettlementId>)> = BTreeMap::new();
    frontier.insert(from.clone(), (0.0, Vec::new()));
    for _ in 0..max_legs {
        let mut next = BTreeMap::new();
        for (port, (cost, path)) in &frontier {
            if port != from && state.is_hostile_settlement(faction, port) {
                continue;
            }
            for edge in data.movement_graph.edges(port) {
                if !edge.sea || &edge.to == from || path.contains(&edge.to) {
                    continue;
                }
                let total = cost + edge.cost;
                if best.get(&edge.to).is_some_and(|(known, _)| *known <= total) {
                    continue;
                }
                let mut voyage = path.clone();
                voyage.push(edge.to.clone());
                best.insert(edge.to.clone(), (total, voyage.clone()));
                next.insert(edge.to.clone(), (total, voyage));
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    best.into_iter()
        .map(|(port, (_, voyage))| (port, voyage))
        .collect()
}

/// The cheapest voyage of `faction` from `from` to `to` this season, if any.
pub fn sea_voyage(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    from: &SettlementId,
    to: &SettlementId,
) -> Option<Vec<SettlementId>> {
    sea_voyages(state, data, faction, from).remove(to)
}

/// The port reachable by sea from `from` nearest map pixel `point` (a click
/// on the water), if any.
pub fn port_near(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    from: &SettlementId,
    point: [f32; 2],
) -> Option<SettlementId> {
    let distance = |port: &SettlementId| {
        data.settlement_point(port)
            .map_or(f32::INFINITY, |p| (p[0] - point[0]).hypot(p[1] - point[1]))
    };
    sea_voyages(state, data, faction, from)
        .into_keys()
        .filter(|port| distance(port).is_finite())
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
}
