//! Dijkstra shortest paths over a graph of ordered nodes.
//!
//! One search serves the settlement graph everywhere (armies, agents, the AI
//! planner, trade routes). Ties on cost are broken by node order, so the
//! result is the same whatever the storage of the labels.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// Cost to reach a node and the node before it on the cheapest path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label<N> {
    pub cost: u32,
    pub previous: Option<N>,
}

/// Where a search keeps the best label of each node it has reached.
pub trait Labels<N> {
    /// Best cost found so far for `node`.
    fn cost(&self, node: &N) -> Option<u32>;
    /// Records a better `cost` for `node`, reached from `previous`.
    fn set(&mut self, node: &N, cost: u32, previous: Option<&N>);
}

impl<N: Ord + Clone> Labels<N> for BTreeMap<N, Label<N>> {
    fn cost(&self, node: &N) -> Option<u32> {
        self.get(node).map(|label| label.cost)
    }

    fn set(&mut self, node: &N, cost: u32, previous: Option<&N>) {
        self.insert(
            node.clone(),
            Label {
                cost,
                previous: previous.cloned(),
            },
        );
    }
}

/// What a search does with a node it has just settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visit {
    /// Relax the node's edges.
    Expand,
    /// Keep the node's label but do not leave it.
    Skip,
    /// End the search here.
    Stop,
}

/// Dijkstra from `start` (cost 0) into `labels`.
///
/// `visit` rules on every settled node; `edges` pushes the `(neighbour,
/// cost)` pairs leaving a node into the buffer it is given (emptied before
/// each call). Paths costing more than `budget` are not explored.
pub fn dijkstra<N: Ord + Clone, L: Labels<N>>(
    labels: &mut L,
    start: N,
    budget: Option<u32>,
    mut visit: impl FnMut(&N) -> Visit,
    mut edges: impl FnMut(&N, &mut Vec<(N, u32)>),
) {
    let mut heap = BinaryHeap::new();
    let mut out = Vec::new();
    labels.set(&start, 0, None);
    heap.push(Reverse((0u32, start)));
    while let Some(Reverse((cost, current))) = heap.pop() {
        if labels.cost(&current).is_some_and(|best| best < cost) {
            continue;
        }
        match visit(&current) {
            Visit::Expand => {}
            Visit::Skip => continue,
            Visit::Stop => break,
        }
        out.clear();
        edges(&current, &mut out);
        for (next, step) in out.drain(..) {
            let total = cost + step;
            if budget.is_some_and(|limit| total > limit) {
                continue;
            }
            if labels.cost(&next).is_none_or(|known| total < known) {
                labels.set(&next, total, Some(&current));
                heap.push(Reverse((total, next)));
            }
        }
    }
}

/// Path from the search start (excluded) to `target` (included).
pub fn path_to<N: Ord + Clone>(labels: &BTreeMap<N, Label<N>>, target: &N) -> Option<Vec<N>> {
    let mut path = vec![target.clone()];
    let mut current = labels.get(target)?;
    while let Some(previous) = &current.previous {
        path.push(previous.clone());
        current = labels.get(previous)?;
    }
    path.pop();
    path.reverse();
    Some(path)
}
