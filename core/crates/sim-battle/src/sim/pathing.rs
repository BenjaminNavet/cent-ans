//! Siege pathing (F5a § 4): A* on a grid of [`CELL`]-metre cells where the
//! intact wall bands and the houses are obstacles (BR3: oriented blocks
//! and the stalls of the market square); breaches, the broken gate
//! (or the open gate of a sortie, for the garrison) and the streets are
//! free, and so are burnt houses (S2). Paths are cached per regiment and recomputed when its goal cell
//! or the walls (openings, sortie) change. Deterministic: integer costs,
//! ties broken by cell index.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::BattleSim;
use crate::setup::SideId;
use crate::siege::{PieceKind, SiegeWorks};
use crate::unit::UnitState;

/// Grid cell size, in metres.
pub const CELL: f64 = 4.0;
/// Clearance kept around house blocks (metres, BR3: from
/// `data/rules/siege_town.json`).
fn house_margin() -> f64 {
    crate::town::TownRules::bundled().block.clearance_m
}

/// Clearance kept around the props of the market square (metres).
fn prop_margin() -> f64 {
    crate::town::TownRules::bundled().props.path_margin_m
}

/// A regiment's cached path.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CachedPath {
    goal: usize,
    signature: (usize, bool, usize),
    waypoints: Vec<(f64, f64)>,
}

/// Walls, houses and props do not move between two A* searches: the
/// verdict of each cell is kept, per side (the gate of a sortie opens for
/// the garrison only), until the walls or the houses change (same
/// signature as the paths). Filled lazily: 0 unknown, 1 free, 2 blocked.
#[derive(Debug, Clone, Default)]
pub(crate) struct ObstacleCache {
    signature: Option<(usize, bool, usize)>,
    cells: [Vec<u8>; 2],
}

struct Grid<'a> {
    works: &'a SiegeWorks,
    nx: usize,
    nz: usize,
    side: SideId,
    /// This side's cells of the [`ObstacleCache`] (lent for the search).
    known: std::cell::RefCell<Vec<u8>>,
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
        !wall && w.house_at(x, z, house_margin()).is_none() && !w.prop_at(x, z, prop_margin())
    }

    /// [`Self::free`], remembered in the obstacle cache.
    fn free_cached(&self, c: usize) -> bool {
        let known = self.known.borrow()[c];
        if known != 0 {
            return known == 1;
        }
        let free = self.free(c);
        self.known.borrow_mut()[c] = if free { 1 } else { 2 };
        free
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
                let ok = next == goal || self.free_cached(next);
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
    // BR3: the blocks are rectangles; a quick disc test first.
    let clear = |f: crate::town::Footprint, margin: f64| {
        let t = (((f.x - a.0) * dx + (f.z - a.1) * dz) / len2).clamp(0.0, 1.0);
        let (px, pz) = (a.0 + dx * t, a.1 + dz * t);
        (px - f.x).hypot(pz - f.z) >= f.bounding_radius() + margin
            || f.distance_to_segment(a, b) >= margin
    };
    let (house, prop) = (house_margin(), prop_margin());
    works
        .houses
        .iter()
        .filter(|h| h.standing())
        .all(|h| clear(h.footprint(), house))
        && works.path_props().all(|p| clear(p.footprint(), prop))
}

/// What the cached paths and obstacles depend on: openings in the walls,
/// the sortie gate, houses burnt down.
fn path_signature(works: &SiegeWorks) -> (usize, bool, usize) {
    (works.openings().len(), works.sortie, works.burnt_houses())
}

impl BattleSim {
    /// The pathing grid of `side`, lent this side's obstacle cache (reset
    /// when the walls or the houses changed).
    fn grid<'a>(&self, works: &'a SiegeWorks, side: SideId) -> Grid<'a> {
        let (nx, nz) = (
            (self.field.width / CELL).ceil() as usize,
            (self.field.depth / CELL).ceil() as usize,
        );
        let signature = path_signature(works);
        let mut cache = self.obstacle_cache.borrow_mut();
        if cache.signature != Some(signature) {
            *cache = ObstacleCache {
                signature: Some(signature),
                ..Default::default()
            };
        }
        let mut known = std::mem::take(&mut cache.cells[side.index()]);
        known.resize(nx * nz, 0);
        Grid {
            works,
            nx,
            nz,
            side,
            known: std::cell::RefCell::new(known),
        }
    }

    /// Returns the grid's cells to the obstacle cache.
    fn give_back(&self, grid: Grid<'_>, side: SideId) {
        self.obstacle_cache.borrow_mut().cells[side.index()] = grid.known.into_inner();
    }

    /// BR3 (tests, probes): the A* way on the siege grid from `from` to `to`
    /// for a regiment of `side`, as cell centres; `None` when the streets
    /// give no way (or in a field battle).
    pub fn siege_route(
        &self,
        side: SideId,
        from: (f64, f64),
        to: (f64, f64),
    ) -> Option<Vec<(f64, f64)>> {
        let works = self.siege.as_ref()?;
        let grid = self.grid(works, side);
        let way = grid
            .search(grid.cell(from.0, from.1), grid.cell(to.0, to.1))
            .map(|cells| cells.into_iter().map(|c| grid.centre(c)).collect());
        self.give_back(grid, side);
        way
    }

    /// A step from `from` to `to` walks into a house (routing regiments slip
    /// through the alleys; leaving a house one stands in is free).
    pub(super) fn house_block(&self, index: usize, from: (f64, f64), to: (f64, f64)) -> bool {
        let Some(works) = &self.siege else {
            return false;
        };
        if self.units[index].state == crate::unit::UnitState::Routing {
            return false;
        }
        // BR3: the blocks and the market props are rectangles.
        let into = |f: crate::town::Footprint| {
            let d = |p: (f64, f64)| f.signed_distance(p.0, p.1);
            d(to) < 1.0 && d(to) < d(from)
        };
        works
            .houses
            .iter()
            .filter(|h| h.standing())
            .any(|h| into(h.footprint()))
            || works.path_props().any(|p| into(p.footprint()))
    }

    /// A* waypoint towards (tx, tz) for regiment `index`; `None` when no
    /// path exists (the caller falls back to ladders or a straight line).
    pub(super) fn grid_route(&self, index: usize, tx: f64, tz: f64) -> Option<(f64, f64)> {
        let works = self.siege.as_ref()?;
        let unit = &self.units[index];
        self.grid_step(works, index, (unit.x, unit.z), (tx, tz))
    }

    /// First waypoint of regiment `index` from `from` towards `to` on the
    /// siege grid: straight when in plain sight, else along its A* path
    /// (cached per regiment, recomputed when the goal cell or the walls
    /// change); `None` when the streets give no way.
    fn grid_step(
        &self,
        works: &SiegeWorks,
        index: usize,
        from: (f64, f64),
        to: (f64, f64),
    ) -> Option<(f64, f64)> {
        if segment_clear(works, from, to) {
            return Some(to);
        }
        let unit = &self.units[index];
        let grid = self.grid(works, unit.side);
        let goal = grid.cell(to.0, to.1);
        let signature = path_signature(works);
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
        self.give_back(grid, unit.side);
        let path = &cache[index].as_ref()?.waypoints;
        match leg_on_path(works, unit.on_wall, from, path) {
            Leg::Via(p) => Some(p),
            Leg::NoWay => None,
        }
    }

    /// Climbers go straight at the wall (ladders) when it bars the way from
    /// `from` and no opening is near enough to be worth the detour.
    fn climbs_straight(
        &self,
        works: &SiegeWorks,
        index: usize,
        from: (f64, f64),
        to: (f64, f64),
    ) -> bool {
        let unit = &self.units[index];
        let climber = unit.side == SideId::Attacker && unit.can_climb();
        if !climber || !works.path_blocked(from, to) {
            return false;
        }
        let Some(opening) = works.best_opening(from, to) else {
            return true;
        };
        let dist =
            |a: (f64, f64), b: (f64, f64)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let mid = works.pieces[opening].midpoint();
        let detour = dist(from, mid) + dist(mid, to);
        detour > dist(from, to) * 1.6 + 40.0
    }

    /// Regiments that ignore the walls and the streets: on the wall walk
    /// (outside a sortie) or routing.
    fn walks_straight_in_siege(&self, works: &SiegeWorks, index: usize) -> bool {
        let unit = &self.units[index];
        let sallying = works.sortie && unit.side == SideId::Defender;
        (unit.on_wall && !sallying) || unit.state == UnitState::Routing
    }

    /// Where `index` should head to reach (tx, tz): straight, or through the
    /// best opening in the walls when an intact wall is in the way (climbers
    /// keep going straight unless the detour is short); across the river by
    /// a bridge or a ford in the field (EP3). The first waypoint of
    /// [`Self::plan_route`], computed alone (no chain, no allocation).
    pub(super) fn route(&self, index: usize, tx: f64, tz: f64) -> (f64, f64) {
        let unit = &self.units[index];
        let (from, to) = ((unit.x, unit.z), (tx, tz));
        let Some(works) = &self.siege else {
            // EP3: across the river by a bridge or a ford.
            return self.water_step(unit, from, to).unwrap_or(to);
        };
        if self.walks_straight_in_siege(works, index)
            || self.climbs_straight(works, index, from, to)
        {
            return to;
        }
        // F5a: A* through breaches, gate and streets (houses are obstacles).
        self.grid_step(works, index, from, to).unwrap_or(to)
    }

    /// CB-M2 (tests, probes): the waypoint regiment `id` heads for this tick
    /// towards its target or destination (what [`Self::route`] gives the
    /// step); `None` without either. The path cache is written only when
    /// the step itself would write it (a new goal).
    pub fn heading_of(&self, id: u32) -> Option<(f64, f64)> {
        let index = id as usize;
        let unit = self.units.get(index)?;
        let (tx, tz) = match unit.target {
            Some(t) => {
                let t = self.units.get(t as usize)?;
                (t.x, t.z)
            }
            None => unit.destination?,
        };
        Some(self.route(index, tx, tz))
    }

    /// CB-M2: the whole way regiment `index` would walk from `from` to `to`,
    /// the destination last: the chain of the waypoints [`Self::route`]
    /// heads for one after the other (same decisions, taken from each
    /// waypoint in turn; in a siege along the same A* path, read from the
    /// regiment's cache when it is still valid). `None` when the
    /// destination cannot be reached (deep water without a crossing for
    /// horsemen and engines, no way through the streets); the order then
    /// falls back to a straight line as before. Read-only: the path cache is
    /// neither read for another start than the regiment's nor written.
    pub(crate) fn plan_route(
        &self,
        index: usize,
        from: (f64, f64),
        to: (f64, f64),
    ) -> Option<Vec<(f64, f64)>> {
        let unit = &self.units[index];
        let mut legs = Vec::new();
        let Some(works) = &self.siege else {
            let mut p = from;
            for _ in 0..MAX_LEGS {
                let next = self.water_step(unit, p, to)?;
                legs.push(next);
                if next == to {
                    return Some(legs);
                }
                if next == p {
                    break;
                }
                p = next;
            }
            legs.push(to);
            return Some(legs);
        };
        if self.walks_straight_in_siege(works, index) {
            return Some(vec![to]);
        }
        let own_start = from == (unit.x, unit.z);
        let mut path: Option<Vec<(f64, f64)>> = None;
        let mut p = from;
        for leg in 0..MAX_LEGS {
            if self.climbs_straight(works, index, p, to) || segment_clear(works, p, to) {
                legs.push(to);
                return Some(legs);
            }
            let path = path.get_or_insert_with(|| self.grid_path(works, index, p, to, own_start));
            match leg_on_path(works, unit.on_wall, p, path) {
                Leg::NoWay if leg == 0 => return None,
                Leg::Via(next) if next != p => {
                    legs.push(next);
                    p = next;
                }
                _ => break,
            }
        }
        legs.push(to);
        Some(legs)
    }

    /// The A* path of regiment `index` towards `to`: its cached one when
    /// still valid for this goal and these walls (and `use_cache`), else
    /// searched from `from`. Never written to the cache.
    fn grid_path(
        &self,
        works: &SiegeWorks,
        index: usize,
        from: (f64, f64),
        to: (f64, f64),
        use_cache: bool,
    ) -> Vec<(f64, f64)> {
        let side = self.units[index].side;
        let grid = self.grid(works, side);
        let goal = grid.cell(to.0, to.1);
        let signature = path_signature(works);
        let cached = use_cache
            .then(|| {
                self.path_cache
                    .borrow()
                    .get(index)
                    .and_then(|c| c.as_ref())
                    .filter(|c| c.goal == goal && c.signature == signature)
                    .map(|c| c.waypoints.clone())
            })
            .flatten();
        let path = cached.unwrap_or_else(|| {
            grid.search(grid.cell(from.0, from.1), goal)
                .map(|cells| cells.into_iter().map(|c| grid.centre(c)).collect())
                .unwrap_or_default()
        });
        self.give_back(grid, side);
        path
    }
}

/// Most waypoints of a planned route (the chain then heads straight for
/// the destination).
const MAX_LEGS: usize = 64;

/// Next leg along an A* path.
enum Leg {
    Via((f64, f64)),
    NoWay,
}

/// The waypoint a regiment at `from` heads for along `path` (cell centres)
/// when the destination is not in plain sight: off the wall face first when
/// pressed against it, else the farthest waypoint in plain sight (string
/// pulling).
fn leg_on_path(works: &SiegeWorks, on_wall: bool, from: (f64, f64), path: &[(f64, f64)]) -> Leg {
    if path.is_empty() {
        return Leg::NoWay;
    }
    // A way exists but we are pressed against a jamb or a wall face:
    // step back off it first.
    if let Some((p, d)) = works.nearest_intact(from.0, from.1) {
        if d < works.band() + 0.5 && !on_wall {
            let (px, pz) = works.pieces[p].closest_point(from.0, from.1);
            let k = (works.band() + 3.0) / d.max(0.1);
            return Leg::Via((px + (from.0 - px) * k, pz + (from.1 - pz) * k));
        }
    }
    let d2 = |p: (f64, f64)| (p.0 - from.0).powi(2) + (p.1 - from.1).powi(2);
    let Some(nearest) = (0..path.len()).min_by(|&a, &b| d2(path[a]).total_cmp(&d2(path[b]))) else {
        return Leg::NoWay;
    };
    // String pulling: the farthest waypoint in plain sight.
    let last = path.len() - 1;
    let far = (nearest + 1..=last.min(nearest + 12))
        .rev()
        .find(|&k| segment_clear(works, from, path[k]));
    Leg::Via(path[far.unwrap_or((nearest + 1).min(last))])
}
