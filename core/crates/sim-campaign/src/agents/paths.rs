//! Agent path search over the settlement graph.

use super::*;

/// Shortest paths for an agent: every edge is open (agents slip through
/// enemy places and armies); `cap` clamps each edge to a season's allowance.
pub fn agent_dijkstra(
    data: &GameData,
    start: &SettlementId,
    budget: Option<u32>,
    cap: u32,
) -> BTreeMap<SettlementId, movement::Reach> {
    AgentTable::search(data, start, budget, cap, None).into_reaches()
}

/// [`agent_dijkstra`] with settlement ids in maps: the reference of the
/// equality tests.
#[cfg(feature = "test-support")]
pub fn agent_dijkstra_by_ids(
    data: &GameData,
    start: &SettlementId,
    budget: Option<u32>,
    cap: u32,
) -> BTreeMap<SettlementId, movement::Reach> {
    let mut best = BTreeMap::new();
    pathfinding::dijkstra(
        &mut best,
        start.clone(),
        budget,
        |_| Visit::Expand,
        |current, out| {
            out.extend(
                movement::edges(data, current)
                    .into_iter()
                    .map(|(next, edge)| (next, edge.min(cap.max(1)))),
            );
        },
    );
    best
}

/// No settlement (unreached, or the start's previous).
const NONE: u32 = u32::MAX;

/// Search labels over dense settlement indices.
struct DenseLabels {
    cost: Vec<u32>,
    previous: Vec<u32>,
}

impl Labels<u32> for DenseLabels {
    fn cost(&self, node: &u32) -> Option<u32> {
        let cost = self.cost[*node as usize];
        (cost != NONE).then_some(cost)
    }

    fn set(&mut self, node: &u32, cost: u32, previous: Option<&u32>) {
        self.cost[*node as usize] = cost;
        self.previous[*node as usize] = previous.copied().unwrap_or(NONE);
    }
}

/// OMR R1: [`agent_dijkstra`] over the graph's dense indices
/// ([`data_model::movement_graph::GraphIndex`]; index order = id order, so
/// the heap breaks ties and the paths come out exactly as before).
pub struct AgentTable<'a> {
    data: &'a GameData,
    start: SettlementId,
    /// Index of the start, `None` when it has no edge (the table holds it alone).
    origin: Option<u32>,
    cost: Vec<u32>,
    previous: Vec<u32>,
}

impl<'a> AgentTable<'a> {
    /// Searches from `start`; with `until`, stops once that settlement's
    /// cost is final (its path does not change after).
    pub fn search(
        data: &'a GameData,
        start: &SettlementId,
        budget: Option<u32>,
        cap: u32,
        until: Option<&SettlementId>,
    ) -> Self {
        let index = data.movement_graph.index();
        let origin = index.position(start);
        let mut table = Self {
            data,
            start: start.clone(),
            origin,
            cost: Vec::new(),
            previous: Vec::new(),
        };
        let Some(origin) = origin else {
            return table;
        };
        let until = until.and_then(|t| index.position(t));
        let n = index.ids.len();
        table.cost = vec![NONE; n];
        table.previous = vec![NONE; n];
        let step_cap = cap.max(1);
        let mut labels = DenseLabels {
            cost: std::mem::take(&mut table.cost),
            previous: std::mem::take(&mut table.previous),
        };
        pathfinding::dijkstra(
            &mut labels,
            origin,
            budget,
            |&current| {
                if until == Some(current) {
                    Visit::Stop
                } else {
                    Visit::Expand
                }
            },
            |&current, out| {
                out.extend(
                    index.edges[current as usize]
                        .iter()
                        .map(|&(next, edge)| (next, (edge.round() as u32).max(1).min(step_cap))),
                );
            },
        );
        table.cost = labels.cost;
        table.previous = labels.previous;
        table
    }

    /// Cost of reaching `id`, if reached.
    pub fn cost(&self, id: &SettlementId) -> Option<u32> {
        if self.origin.is_none() {
            return (id == &self.start).then_some(0);
        }
        let i = self.data.movement_graph.index().position(id)? as usize;
        (self.cost[i] != NONE).then_some(self.cost[i])
    }

    /// [`movement::path_to`] on this table (`None` when `target` is unreached).
    pub fn path_to(&self, target: &SettlementId) -> Option<Vec<SettlementId>> {
        if self.origin.is_none() {
            return (target == &self.start).then(Vec::new);
        }
        let index = self.data.movement_graph.index();
        let mut i = index.position(target)?;
        if self.cost[i as usize] == NONE {
            return None;
        }
        let mut path = Vec::new();
        while self.previous[i as usize] != NONE {
            path.push(index.ids[i as usize].clone());
            i = self.previous[i as usize];
        }
        path.reverse();
        Some(path)
    }

    /// The table as [`agent_dijkstra_by_ids`] returns it.
    pub fn into_reaches(self) -> BTreeMap<SettlementId, movement::Reach> {
        let Some(_) = self.origin else {
            return BTreeMap::from([(
                self.start,
                movement::Reach {
                    cost: 0,
                    previous: None,
                },
            )]);
        };
        let index = self.data.movement_graph.index();
        self.cost
            .iter()
            .zip(&self.previous)
            .enumerate()
            .filter(|(_, (cost, _))| **cost != NONE)
            .map(|(i, (cost, previous))| {
                (
                    index.ids[i].clone(),
                    movement::Reach {
                        cost: *cost,
                        previous: (*previous != NONE)
                            .then(|| index.ids[*previous as usize].clone()),
                    },
                )
            })
            .collect()
    }
}
