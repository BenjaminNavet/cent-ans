//! Movement graph over the settlements (lot C4, spec § 4.4).
//!
//! [`GameData::build_movement_graph`] indexes the edges of
//! `data/map/settlement_graph.json` (lot C3). When that file is absent or
//! empty, it builds a deterministic fallback graph instead:
//!
//! - the settlements of a province are all linked together, cost = distance
//!   × the province's terrain cost;
//! - the cities of land-neighbouring provinces are linked, plus the
//!   `fallback_links_per_neighbor` closest settlement pairs of the two
//!   provinces, cost = distance × the mean terrain cost of both provinces;
//! - the `port` settlements of provinces linked by `sea_neighbors` are
//!   linked by sea, cost = `sea_crossing_steps × points_per_step`.
//!
//! Costs are in kilometres of plain, the unit of movement points (see
//! [`MovementRules`]).

use std::collections::{BTreeMap, BTreeSet};

use crate::entities::province::Terrain;
use crate::entities::settlement::{MovementRules, RetreatRules, SettlementEdge};
use crate::ids::{ProvinceId, SettlementId};
use crate::load::GameData;

/// Mean Earth radius in kilometres (great-circle distances).
pub const EARTH_RADIUS_KM: f64 = 6371.0;

/// One directed half of an undirected graph edge.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    pub to: SettlementId,
    /// Movement cost (kilometres of plain).
    pub cost: f64,
    pub road: bool,
    pub sea: bool,
}

/// Adjacency lists of the settlement graph, sorted by target id.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MovementGraph {
    pub adjacency: BTreeMap<SettlementId, Vec<GraphEdge>>,
    /// `true` when built by the fallback rules (no `settlement_graph.json`).
    pub fallback: bool,
}

impl MovementGraph {
    /// Edges leaving `from` (empty for an unknown or isolated settlement).
    pub fn edges(&self, from: &SettlementId) -> &[GraphEdge] {
        self.adjacency
            .get(from)
            .map_or(&[], |edges| edges.as_slice())
    }

    /// The edge `from` → `to`, if any.
    pub fn edge(&self, from: &SettlementId, to: &SettlementId) -> Option<&GraphEdge> {
        self.edges(from).iter().find(|e| &e.to == to)
    }

    fn add(&mut self, edge: &SettlementEdge) {
        if edge.from == edge.to {
            return;
        }
        for (a, b) in [(&edge.from, &edge.to), (&edge.to, &edge.from)] {
            let list = self.adjacency.entry(a.clone()).or_default();
            match list.iter_mut().find(|e| &e.to == b) {
                // Duplicate pair: keep the cheapest.
                Some(existing) if existing.cost <= edge.cost => {}
                Some(existing) => {
                    existing.cost = edge.cost;
                    existing.road = edge.road;
                    existing.sea = edge.sea;
                }
                None => list.push(GraphEdge {
                    to: b.clone(),
                    cost: edge.cost.max(0.0),
                    road: edge.road,
                    sea: edge.sea,
                }),
            }
        }
    }

    fn sort(&mut self) {
        for list in self.adjacency.values_mut() {
            list.sort_by(|a, b| a.to.cmp(&b.to));
        }
    }
}

/// Movement cost multiplier of a terrain (v1: 2 for mountains and marshes).
pub fn terrain_cost(terrain: Terrain) -> f64 {
    match terrain {
        Terrain::Mountains | Terrain::Marsh => 2.0,
        _ => 1.0,
    }
}

/// Great-circle distance in kilometres between two `[lon, lat]` points.
pub fn distance_km(a: [f64; 2], b: [f64; 2]) -> f64 {
    let (lat1, lat2) = (a[1].to_radians(), b[1].to_radians());
    let dlat = lat2 - lat1;
    let dlon = (b[0] - a[0]).to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * h.sqrt().min(1.0).asin()
}

impl GameData {
    /// Movement tuning (`rules.json`, defaults when absent).
    pub fn movement_rules(&self) -> MovementRules {
        self.settlement_rules
            .as_ref()
            .map(|r| r.movement.clone())
            .unwrap_or_default()
    }

    /// Free movement rules (`data/movement/rules.json`, defaults when
    /// absent, lot M2).
    pub fn free_movement_rules(&self) -> &crate::entities::movement::FreeMovementRules {
        static DEFAULT: std::sync::OnceLock<crate::entities::movement::FreeMovementRules> =
            std::sync::OnceLock::new();
        self.free_movement
            .as_ref()
            .unwrap_or_else(|| DEFAULT.get_or_init(Default::default))
    }

    /// Retreat tuning (`rules.json`, defaults when absent, lot C7a).
    pub fn retreat_rules(&self) -> RetreatRules {
        self.settlement_rules
            .as_ref()
            .map(|r| r.retreat.clone())
            .unwrap_or_default()
    }

    /// Land neighbours of a province (geometry graph first, entity data otherwise).
    pub fn province_land_neighbors(&self, id: &ProvinceId) -> &[ProvinceId] {
        if let Some(geometry) = self.province_geometry.get(id) {
            return &geometry.neighbors;
        }
        self.provinces
            .get(id)
            .map_or(&[], |province| province.neighbors.as_slice())
    }

    /// Provinces reachable by sea from `id` (`sea_neighbors` of the geometry).
    pub fn province_sea_neighbors(&self, id: &ProvinceId) -> Vec<ProvinceId> {
        self.province_geometry
            .get(id)
            .and_then(|geometry| geometry.extra.get("sea_neighbors"))
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .filter_map(|raw| ProvinceId::new(raw).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// (Re)builds [`GameData::movement_graph`] from `settlement_graph`, or
    /// the fallback graph when it is empty. Called by [`GameData::load`].
    pub fn build_movement_graph(&mut self) {
        let mut graph = MovementGraph::default();
        if self.settlement_graph.is_empty() {
            graph.fallback = true;
            for edge in self.fallback_edges() {
                graph.add(&edge);
            }
        } else {
            // Lot C7a: road edges carry the factor baked by C3; rescale
            // them to the one of `rules.json`.
            let road_scale = self.movement_rules().road_cost_factor
                / crate::entities::settlement::BAKED_ROAD_COST_FACTOR;
            for edge in &self.settlement_graph {
                if edge.road && !edge.sea && (road_scale - 1.0).abs() > f64::EPSILON {
                    let mut edge = edge.clone();
                    edge.cost *= road_scale;
                    graph.add(&edge);
                } else {
                    graph.add(edge);
                }
            }
        }
        graph.sort();
        self.movement_graph = graph;
    }

    fn fallback_edges(&self) -> Vec<SettlementEdge> {
        let rules = self.movement_rules();
        let mut edges = Vec::new();
        let terrain = |p: &ProvinceId| {
            self.provinces
                .get(p)
                .map_or(1.0, |province| terrain_cost(province.terrain))
        };
        let lonlat = |id: &SettlementId| self.settlements.get(id).map(|s| s.lonlat);
        let edge = |from: &SettlementId, to: &SettlementId, cost: f64, sea: bool| SettlementEdge {
            from: from.clone(),
            to: to.clone(),
            cost: (cost * 100.0).round() / 100.0,
            road: false,
            sea,
        };
        // Inside each province.
        for (province, ids) in &self.settlements_by_province {
            let factor = terrain(province);
            for (i, a) in ids.iter().enumerate() {
                for b in &ids[i + 1..] {
                    if let (Some(pa), Some(pb)) = (lonlat(a), lonlat(b)) {
                        edges.push(edge(a, b, distance_km(pa, pb) * factor, false));
                    }
                }
            }
        }
        // Between land neighbours.
        let mut seen: BTreeSet<(ProvinceId, ProvinceId)> = BTreeSet::new();
        for province in self.settlements_by_province.keys() {
            for other in self.province_land_neighbors(province) {
                let pair = if province < other {
                    (province.clone(), other.clone())
                } else {
                    (other.clone(), province.clone())
                };
                if !self.settlements_by_province.contains_key(other) || !seen.insert(pair.clone()) {
                    continue;
                }
                let factor = (terrain(&pair.0) + terrain(&pair.1)) / 2.0;
                let (Some(city_a), Some(city_b)) =
                    (self.province_city(&pair.0), self.province_city(&pair.1))
                else {
                    continue;
                };
                let mut linked: BTreeSet<(SettlementId, SettlementId)> = BTreeSet::new();
                edges.push(edge(
                    &city_a.id,
                    &city_b.id,
                    distance_km(city_a.lonlat, city_b.lonlat) * factor,
                    false,
                ));
                linked.insert((city_a.id.clone(), city_b.id.clone()));
                let mut pairs: Vec<(f64, &SettlementId, &SettlementId)> = Vec::new();
                for a in &self.settlements_by_province[&pair.0] {
                    for b in &self.settlements_by_province[&pair.1] {
                        if let (Some(pa), Some(pb)) = (lonlat(a), lonlat(b)) {
                            pairs.push((distance_km(pa, pb), a, b));
                        }
                    }
                }
                pairs.sort_by(|x, y| {
                    x.0.total_cmp(&y.0)
                        .then_with(|| x.1.cmp(y.1))
                        .then_with(|| x.2.cmp(y.2))
                });
                let mut extra = 0;
                for (distance, a, b) in pairs {
                    if extra >= rules.fallback_links_per_neighbor {
                        break;
                    }
                    if linked.insert((a.clone(), b.clone())) {
                        edges.push(edge(a, b, distance * factor, false));
                        extra += 1;
                    }
                }
            }
        }
        // Across the sea, port to port.
        let sea_cost = rules.sea_crossing_steps * rules.points_per_step;
        let ports = |p: &ProvinceId| -> Vec<SettlementId> {
            self.settlements_by_province
                .get(p)
                .map(|ids| {
                    ids.iter()
                        .filter(|id| self.settlements.get(*id).is_some_and(|s| s.port))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut seen: BTreeSet<(ProvinceId, ProvinceId)> = BTreeSet::new();
        for province in self.settlements_by_province.keys() {
            for other in self.province_sea_neighbors(province) {
                let pair = if province < &other {
                    (province.clone(), other.clone())
                } else {
                    (other.clone(), province.clone())
                };
                if !seen.insert(pair.clone()) {
                    continue;
                }
                for a in ports(&pair.0) {
                    for b in ports(&pair.1) {
                        edges.push(edge(&a, &b, sea_cost, true));
                    }
                }
            }
        }
        edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_paris_rouen() {
        let paris = [2.3522, 48.8566];
        let rouen = [1.0993, 49.4431];
        let d = distance_km(paris, rouen);
        assert!((100.0..120.0).contains(&d), "{d}");
    }

    #[test]
    fn duplicate_edges_keep_the_cheapest() {
        let id = |s: &str| SettlementId::new(s).unwrap();
        let mut graph = MovementGraph::default();
        for cost in [5.0, 3.0, 4.0] {
            graph.add(&SettlementEdge {
                from: id("set_a"),
                to: id("set_b"),
                cost,
                road: false,
                sea: false,
            });
        }
        assert_eq!(graph.edges(&id("set_a")).len(), 1);
        assert_eq!(graph.edge(&id("set_b"), &id("set_a")).unwrap().cost, 3.0);
    }
}
