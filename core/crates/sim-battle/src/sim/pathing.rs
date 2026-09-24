//! Siege pathing (F5a § 4): A* on a grid of [`CELL`]-metre cells where the
//! intact wall bands and the houses are obstacles; breaches, the broken gate
//! (or the open gate of a sortie, for the garrison) and the streets are
//! free, and so are burnt houses (S2). Paths are cached per regiment and recomputed when its goal cell
//! or the walls (openings, sortie) change. Deterministic: integer costs,
//! ties broken by cell index.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::BattleSim;
use crate::setup::SideId;
use crate::siege::{PieceKind, SiegeWorks};

/// Grid cell size, in metres.
pub const CELL: f64 = 4.0;
/// Clearance kept around houses (metres).
const HOUSE_MARGIN: f64 = 3.0;

/// A regiment's cached path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CachedPath {
    goal: usize,
    signature: (usize, bool, usize),
    waypoints: Vec<(f64, f64)>,
}

struct Grid<'a> {
    works: &'a SiegeWorks,
    nx: usize,
    nz: usize,
    side: SideId,
}

impl Grid<'_> {
    fn cell(&self, x: f64, z: f64) -> usize {
        let ix = ((x / CELL).floor().max(0.0) as usize).min(self.nx - 1);
        let iz = ((z / CELL).floor().max(0.0) as usize).min(self.nz - 1);
        iz * self.nx + ix
    }

    fn centre(&self, c: usize) -> (f64, f64) {
        let (ix, iz) = (c % self.nx, c / self.nx);
        ((ix as f64 + 0.5) * CELL, (iz as f64 + 0.5) * CELL)
    }

    fn free(&self, c: usize) -> bool {
        let (x, z) = self.centre(c);
        let w = self.works;
        let open = |p: &crate::siege::WallPiece| {
            !p.intact() || (p.kind == PieceKind::Gate && w.sortie && self.side == SideId::Defender)
        };
        // Just out of the blocking band (so that a gate stays passable), and
        // wide enough that no diagonal step straddles a wall.
        let clearance = (w.band() + 0.75).max(CELL * 0.75);
        let wall = w
            .pieces
            .iter()
            .any(|p| !open(p) && p.distance(x, z) < clearance);
        !wall && w.house_at(x, z, HOUSE_MARGIN).is_none()
    }

    /// A* from `start` to `goal` (both treated as free); cell path.
    fn search(&self, start: usize, goal: usize) -> Option<Vec<usize>> {
        let n = self.nx * self.nz;
        let h = |c: usize| {
            let (ax, az) = ((c % self.nx) as i64, (c / self.nx) as i64);
            let (bx, bz) = ((goal % self.nx) as i64, (goal / self.nx) as i64);
            let (dx, dz) = ((ax - bx).abs(), (az - bz).abs());
            10 * dx.max(dz) + 4 * dx.min(dz)
        };
        let mut cost = vec![i64::MAX; n];
        let mut from = vec![usize::MAX; n];
        let mut free: Vec<Option<bool>> = vec![None; n];
        let mut open = BinaryHeap::new();
        cost[start] = 0;
        open.push(Reverse((h(start), start)));
        while let Some(Reverse((_, c))) = open.pop() {
            if c == goal {
                let mut path = vec![c];
                let mut at = c;
                while from[at] != usize::MAX {
                    at = from[at];
                    path.push(at);
                }
                path.reverse();
                return Some(path);
            }
            let (ix, iz) = ((c % self.nx) as i64, (c / self.nx) as i64);
            for (dx, dz) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (x, z) = (ix + dx, iz + dz);
                if x < 0 || z < 0 || x >= self.nx as i64 || z >= self.nz as i64 {
                    continue;
                }
                let next = z as usize * self.nx + x as usize;
                let ok = *free[next].get_or_insert_with(|| next == goal || self.free(next));
                if !ok {
                    continue;
                }
                let step = if dx != 0 && dz != 0 { 14 } else { 10 };
                let new_cost = cost[c] + step;
                if new_cost < cost[next] {
                    cost[next] = new_cost;
                    from[next] = c;
                    open.push(Reverse((new_cost + h(next), next)));
                }
            }
        }
        None
    }
}

/// No house disc crossed by the segment `a`-`b` (and no wall).
pub(crate) fn segment_clear(works: &SiegeWorks, a: (f64, f64), b: (f64, f64)) -> bool {
    if works.path_blocked(a, b) {
        return false;
    }
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = (dx * dx + dz * dz).max(1e-9);
    // Keep out of the wall bands (where a regiment closing in is stopped),
    // except right at the start.
    let len = len2.sqrt();
    let near_wall = |x: f64, z: f64| {
        works
            .pieces
            .iter()
            .any(|p| p.intact() && p.distance(x, z) < works.band() + 1.5)
    };
    let samples = (len / 2.0).ceil() as usize;
    if (1..=samples).any(|k| {
        let t = (k as f64 * 2.0).min(len) / len;
        k as f64 * 2.0 > 3.0 && near_wall(a.0 + dx * t, a.1 + dz * t)
    }) {
        return false;
    }
    works.houses.iter().filter(|h| h.standing()).all(|h| {
        let t = (((h.x - a.0) * dx + (h.z - a.1) * dz) / len2).clamp(0.0, 1.0);
        let (px, pz) = (a.0 + dx * t, a.1 + dz * t);
        (px - h.x).powi(2) + (pz - h.z).powi(2) >= (h.radius + HOUSE_MARGIN).powi(2)
    })
}

impl BattleSim {
    /// A step from `from` to `to` walks into a house (routing regiments slip
    /// through the alleys; leaving a house one stands in is free).
    pub(super) fn house_block(&self, index: usize, from: (f64, f64), to: (f64, f64)) -> bool {
        let Some(works) = &self.siege else {
            return false;
        };
        if self.units[index].state == crate::unit::UnitState::Routing {
            return false;
        }
        works.houses.iter().filter(|h| h.standing()).any(|h| {
            let d = |p: (f64, f64)| ((p.0 - h.x).powi(2) + (p.1 - h.z).powi(2)).sqrt();
            d(to) < h.radius + 1.0 && d(to) < d(from)
        })
    }

    /// A* waypoint towards (tx, tz) for regiment `index`; `None` when no
    /// path exists (the caller falls back to ladders or a straight line).
    pub(super) fn grid_route(&self, index: usize, tx: f64, tz: f64) -> Option<(f64, f64)> {
        let works = self.siege.as_ref()?;
        let unit = &self.units[index];
        let from = (unit.x, unit.z);
        if segment_clear(works, from, (tx, tz)) {
            return Some((tx, tz));
        }
        let grid = Grid {
            works,
            nx: (self.field.width / CELL).ceil() as usize,
            nz: (self.field.depth / CELL).ceil() as usize,
            side: unit.side,
        };
        let goal = grid.cell(tx, tz);
        let signature = (works.openings().len(), works.sortie, works.burnt_houses());
        let mut cache = self.path_cache.borrow_mut();
        if cache.len() < self.units.len() {
            cache.resize(self.units.len(), None);
        }
        let stale = cache[index]
            .as_ref()
            .is_none_or(|c| c.goal != goal || c.signature != signature);
        if stale {
            let waypoints = grid
                .search(grid.cell(from.0, from.1), goal)
                .map(|cells| cells.into_iter().map(|c| grid.centre(c)).collect())
                .unwrap_or_default();
            cache[index] = Some(CachedPath {
                goal,
                signature,
                waypoints,
            });
        }
        let path = &cache[index].as_ref()?.waypoints;
        if path.is_empty() {
            return None;
        }
        // A way exists but we are pressed against a jamb or a wall face:
        // step back off it first.
        if let Some((p, d)) = works.nearest_intact(from.0, from.1) {
            if d < works.band() + 0.5 && !unit.on_wall {
                let (px, pz) = works.pieces[p].closest_point(from.0, from.1);
                let k = (works.band() + 3.0) / d.max(0.1);
                return Some((px + (from.0 - px) * k, pz + (from.1 - pz) * k));
            }
        }
        let d2 = |p: (f64, f64)| (p.0 - from.0).powi(2) + (p.1 - from.1).powi(2);
        let nearest = (0..path.len()).min_by(|&a, &b| d2(path[a]).total_cmp(&d2(path[b])))?;
        // String pulling: the farthest waypoint in plain sight.
        let last = path.len() - 1;
        let far = (nearest + 1..=last.min(nearest + 12))
            .rev()
            .find(|&k| segment_clear(works, from, path[k]));
        Some(path[far.unwrap_or((nearest + 1).min(last))])
    }
}
