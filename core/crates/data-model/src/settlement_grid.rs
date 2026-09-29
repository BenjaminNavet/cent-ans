//! OMR R1: settlements bucketed on a coarse grid of map pixels, for the
//! nearest settlement to a point (every field army's anchor, each faction
//! turn: 3 900 settlements walked per army on the Ural–Mediterranean map).
//!
//! [`GameData::nearest_settlement`] answers exactly like the walk over
//! every settlement (smallest distance, then smallest id): the rings of
//! cells are searched until no unvisited cell can hold a point as near as
//! the best one found.
use std::sync::{Arc, OnceLock};

use crate::ids::SettlementId;
use crate::load::GameData;

/// Cells along the longest side of the grid.
const CELLS: f32 = 96.0;

#[derive(Debug)]
pub struct SettlementGrid {
    /// Every settlement with a position, in id order.
    points: Vec<(SettlementId, [f32; 2])>,
    /// Indices into `points` by cell (row-major), each in id order.
    cells: Vec<Vec<u32>>,
    min: [f32; 2],
    size: f32,
    columns: usize,
    rows: usize,
    /// (`settlements`, `settlement_px`) sizes the grid was built from: a
    /// change falls back to the walk.
    built_from: (usize, usize),
}

/// Distance in map pixels (the same formula as the walks in `sim-campaign`).
pub fn point_distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The walk: every settlement with a position, nearest first, then by id.
pub fn nearest_by_walk(data: &GameData, point: [f32; 2]) -> Option<SettlementId> {
    positions(data)
        .into_iter()
        .map(|(id, p)| (point_distance(p, point), id))
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
        .map(|(_, id)| id)
}

/// Settlements with their position, in id order (`settlement_px` first,
/// else [`GameData::settlement_point`]).
fn positions(data: &GameData) -> Vec<(SettlementId, [f32; 2])> {
    let mut known = data.settlement_px.iter().peekable();
    data.settlements
        .keys()
        .filter_map(|id| {
            while known.next_if(|(k, _)| *k < id).is_some() {}
            let point = match known.peek() {
                Some((k, p)) if *k == id => Some(**p),
                _ => data.settlement_point(id),
            };
            point.map(|p| (id.clone(), p))
        })
        .collect()
}

impl SettlementGrid {
    fn new(data: &GameData) -> Self {
        let points = positions(data);
        let finite = points
            .iter()
            .map(|(_, p)| *p)
            .filter(|p| p[0].is_finite() && p[1].is_finite());
        let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
        for p in finite {
            for k in 0..2 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        if min[0] > max[0] {
            min = [0.0; 2];
            max = [1.0; 2];
        }
        let size = ((max[0] - min[0]).max(max[1] - min[1]) / CELLS).max(1.0);
        let columns = (((max[0] - min[0]) / size) as usize + 1).max(1);
        let rows = (((max[1] - min[1]) / size) as usize + 1).max(1);
        let mut grid = Self {
            points: Vec::new(),
            cells: vec![Vec::new(); columns * rows],
            min,
            size,
            columns,
            rows,
            built_from: (data.settlements.len(), data.settlement_px.len()),
        };
        for (i, (_, p)) in points.iter().enumerate() {
            let (c, r) = grid.cell(*p);
            grid.cells[r * columns + c].push(i as u32);
        }
        grid.points = points;
        grid
    }

    /// Cell of `p`, clamped to the grid (a point off the grid, or not
    /// finite, lands in a border cell: the bound below still holds).
    fn cell(&self, p: [f32; 2]) -> (usize, usize) {
        let axis = |v: f32, min: f32, n: usize| {
            let i = ((v - min) / self.size).floor();
            if i.is_nan() || i < 0.0 {
                0
            } else {
                (i as usize).min(n - 1)
            }
        };
        (
            axis(p[0], self.min[0], self.columns),
            axis(p[1], self.min[1], self.rows),
        )
    }

    fn nearest(&self, point: [f32; 2]) -> Option<SettlementId> {
        let (qc, qr) = self.cell(point);
        let mut best: Option<(f32, u32)> = None;
        let rings = self.columns.max(self.rows);
        for ring in 0..=rings {
            // Every point of a cell at ring `k` lies at least `(k - 1) ×
            // size` away (the query is in, or beyond, its own cell): once
            // the best distance is below that for the next ring, stop. The
            // margin covers the rounding of the distance formula.
            if let Some((d, _)) = best {
                let bound = (ring as f32 - 1.0) * self.size;
                if d.is_finite() && d + 1.0 + self.size * 1e-3 < bound {
                    break;
                }
            }
            let (c0, c1) = (qc.saturating_sub(ring), (qc + ring).min(self.columns - 1));
            let (r0, r1) = (qr.saturating_sub(ring), (qr + ring).min(self.rows - 1));
            for r in r0..=r1 {
                for c in c0..=c1 {
                    // Only the ring's border (inner cells were visited).
                    if r.abs_diff(qr) != ring && c.abs_diff(qc) != ring {
                        continue;
                    }
                    for &i in &self.cells[r * self.columns + c] {
                        let d = point_distance(self.points[i as usize].1, point);
                        let better = best.is_none_or(|(bd, bi)| {
                            d.total_cmp(&bd)
                                .then_with(|| {
                                    self.points[i as usize].0.cmp(&self.points[bi as usize].0)
                                })
                                .is_lt()
                        });
                        if better {
                            best = Some((d, i));
                        }
                    }
                }
            }
        }
        best.map(|(_, i)| self.points[i as usize].0.clone())
    }
}

/// Lazily built grid, shared by the clones of a [`GameData`].
#[derive(Debug, Clone, Default)]
pub struct SettlementGridCell(Arc<OnceLock<SettlementGrid>>);

impl GameData {
    /// OMR R1: the settlement nearest to `point` (map pixels), nearest
    /// first then by id, like [`nearest_by_walk`], read on a grid built on
    /// first use.
    pub fn nearest_settlement(&self, point: [f32; 2]) -> Option<SettlementId> {
        let grid = self
            .settlement_grid
            .0
            .get_or_init(|| SettlementGrid::new(self));
        if grid.built_from != (self.settlements.len(), self.settlement_px.len()) {
            return nearest_by_walk(self, point);
        }
        grid.nearest(point)
    }
}
