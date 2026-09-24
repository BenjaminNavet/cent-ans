//! Path finding on the navigation grid (lot M2, spec § 3.2).
//!
//! Pure algorithms over a [`NavGrid`]: A* on 8 neighbours with the octile
//! heuristic, a line-of-sight smoothing pass (theta*-like string pulling)
//! and a bounded Dijkstra for the reachable area. Diagonal steps may cut
//! corners: the M1 pipeline closes the great rivers diagonally (8-connected
//! barriers), so a narrow diagonal bridge or pass stays open.
//!
//! Costs: entering a cell orthogonally costs its grid value (10 = plain),
//! diagonally the value × √2 (rounded). The searches work on a window of
//! the grid (bounded by the budget, or by the start and goal plus a
//! margin) with dense scratch buffers reused per thread.

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use data_model::{NavGrid, IMPASSABLE};
use serde::{Deserialize, Serialize};

/// A cell of the navigation grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Cell {
    pub x: u16,
    pub y: u16,
}

impl Cell {
    pub fn new(x: u32, y: u32) -> Self {
        Cell {
            x: x.min(u32::from(u16::MAX)) as u16,
            y: y.min(u32::from(u16::MAX)) as u16,
        }
    }

    /// The cell containing map pixel `point`.
    pub fn of_point(grid: &NavGrid, point: [f32; 2]) -> Self {
        let (x, y) = grid.cell_of(point[0], point[1]);
        Cell::new(x, y)
    }

    /// Map-pixel centre of the cell.
    pub fn center(self, grid: &NavGrid) -> [f32; 2] {
        grid.cell_center(u32::from(self.x), u32::from(self.y))
    }

    fn xy(self) -> (i64, i64) {
        (i64::from(self.x), i64::from(self.y))
    }
}

/// A path found by [`find_path`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridPath {
    /// Every cell walked, 8-connected, start excluded, goal included.
    pub cells: Vec<Cell>,
    /// Corners of the smoothed path, start excluded, goal included.
    pub waypoints: Vec<Cell>,
    /// Total cost of `cells`.
    pub cost: u32,
}

/// Cost of a diagonal step into a cell of cost `value`.
#[inline]
pub fn diagonal_cost(value: u8) -> u32 {
    (u32::from(value) * 14_142 + 5_000) / 10_000
}

/// Cost of stepping from `from` to the adjacent `to` (8-neighbourhood), or
/// `None` when `to` is impassable.
#[inline]
pub fn step_cost(grid: &NavGrid, from: Cell, to: Cell) -> Option<u32> {
    let (fx, fy) = from.xy();
    let (tx, ty) = to.xy();
    let value = grid.cost(tx, ty);
    if value == IMPASSABLE {
        return None;
    }
    if fx != tx && fy != ty {
        Some(diagonal_cost(value))
    } else {
        Some(u32::from(value))
    }
}

/// Admissible octile distance between two cells at the grid's cheapest cost.
#[inline]
fn heuristic(min_cost: u8, a: (i64, i64), b: (i64, i64)) -> u32 {
    let dx = (a.0 - b.0).unsigned_abs() as u32;
    let dy = (a.1 - b.1).unsigned_abs() as u32;
    let (long, short) = if dx > dy { (dx, dy) } else { (dy, dx) };
    (long - short) * u32::from(min_cost) + short * diagonal_cost(min_cost)
}

const NEIGHBOURS: [(i64, i64); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// A rectangle of the grid, `[x0, x1) × [y0, y1)`.
#[derive(Debug, Clone, Copy)]
struct Window {
    x0: i64,
    y0: i64,
    width: i64,
    height: i64,
}

impl Window {
    fn around(grid: &NavGrid, points: &[(i64, i64)], margin: i64) -> Self {
        let min_x = points.iter().map(|p| p.0).min().unwrap_or(0) - margin;
        let max_x = points.iter().map(|p| p.0).max().unwrap_or(0) + margin;
        let min_y = points.iter().map(|p| p.1).min().unwrap_or(0) - margin;
        let max_y = points.iter().map(|p| p.1).max().unwrap_or(0) + margin;
        let x0 = min_x.max(0);
        let y0 = min_y.max(0);
        let x1 = (max_x + 1).min(i64::from(grid.width));
        let y1 = (max_y + 1).min(i64::from(grid.height));
        Window {
            x0,
            y0,
            width: (x1 - x0).max(1),
            height: (y1 - y0).max(1),
        }
    }

    fn len(&self) -> usize {
        (self.width * self.height) as usize
    }

    #[inline]
    fn local(&self, x: i64, y: i64) -> Option<usize> {
        let (lx, ly) = (x - self.x0, y - self.y0);
        (lx >= 0 && ly >= 0 && lx < self.width && ly < self.height)
            .then(|| (ly * self.width + lx) as usize)
    }

    #[inline]
    fn cell(&self, local: usize) -> Cell {
        let local = local as i64;
        Cell::new(
            (self.x0 + local % self.width) as u32,
            (self.y0 + local / self.width) as u32,
        )
    }
}

/// Dense per-thread buffers of the searches.
#[derive(Default)]
struct Scratch {
    g: Vec<u32>,
    parent: Vec<u32>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

/// Cells a search must not enter (hostile places...), besides impassable ones.
pub trait Blocker {
    fn blocked(&self, cell: Cell) -> bool;
}

impl<F: Fn(Cell) -> bool> Blocker for F {
    fn blocked(&self, cell: Cell) -> bool {
        self(cell)
    }
}

/// No extra blocked cell.
pub fn nothing_blocked(_: Cell) -> bool {
    false
}

/// A set of cells with a fast negative answer (a coarse bitmap of 32×32
/// blocks in front of a sorted list): the blocker of the searches.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CellSet {
    coarse: Vec<u64>,
    cells: Vec<u32>,
}

/// Blocks per side of the coarse bitmap (32 cells each: 4096 cells).
const COARSE_SIDE: usize = 128;

impl CellSet {
    pub fn new() -> Self {
        CellSet {
            coarse: vec![0; COARSE_SIDE * COARSE_SIDE / 64],
            cells: Vec::new(),
        }
    }

    #[inline]
    fn pack(cell: Cell) -> u32 {
        (u32::from(cell.y) << 16) | u32::from(cell.x)
    }

    #[inline]
    fn block(cell: Cell) -> Option<usize> {
        let (bx, by) = (usize::from(cell.x) >> 5, usize::from(cell.y) >> 5);
        (bx < COARSE_SIDE && by < COARSE_SIDE).then_some(by * COARSE_SIDE + bx)
    }

    pub fn insert(&mut self, cell: Cell) {
        if self.coarse.is_empty() {
            self.coarse = vec![0; COARSE_SIDE * COARSE_SIDE / 64];
        }
        if let Some(bit) = Self::block(cell) {
            self.coarse[bit >> 6] |= 1 << (bit & 63);
        }
        let packed = Self::pack(cell);
        if let Err(at) = self.cells.binary_search(&packed) {
            self.cells.insert(at, packed);
        }
    }

    #[inline]
    pub fn contains(&self, cell: Cell) -> bool {
        if let Some(bit) = Self::block(cell) {
            if self
                .coarse
                .get(bit >> 6)
                .is_none_or(|w| w & (1 << (bit & 63)) == 0)
            {
                return false;
            }
        }
        self.cells.binary_search(&Self::pack(cell)).is_ok()
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

impl FromIterator<Cell> for CellSet {
    fn from_iter<I: IntoIterator<Item = Cell>>(iter: I) -> Self {
        let mut set = CellSet::new();
        for cell in iter {
            set.insert(cell);
        }
        set
    }
}

impl Blocker for CellSet {
    #[inline]
    fn blocked(&self, cell: Cell) -> bool {
        self.contains(cell)
    }
}

/// Cost of entering a neighbour of `(x, y)` at offset `(dx, dy)`, read
/// straight from the cost array (hot loops), or `None` when impassable or
/// off the grid.
#[inline(always)]
fn neighbour_cost(
    costs: &[u8],
    width: i64,
    height: i64,
    nx: i64,
    ny: i64,
    diagonal: bool,
) -> Option<u32> {
    if nx < 0 || ny < 0 || nx >= width || ny >= height {
        return None;
    }
    let value = costs[(ny * width + nx) as usize];
    if value == IMPASSABLE {
        return None;
    }
    let value = value.max(1);
    Some(if diagonal {
        diagonal_cost(value)
    } else {
        u32::from(value)
    })
}

/// A* from `start` to `goal` (8 neighbours, octile heuristic), then
/// smoothed by line of sight. `blocked` cells are never entered, except
/// the goal. `max_cost` bounds the search (a path dearer than it is not
/// looked for). `None` when the goal is unreachable.
pub fn find_path<B: Blocker + ?Sized>(
    grid: &NavGrid,
    start: Cell,
    goal: Cell,
    blocked: &B,
    max_cost: Option<u32>,
) -> Option<GridPath> {
    if start == goal {
        return Some(GridPath {
            cells: Vec::new(),
            waypoints: Vec::new(),
            cost: 0,
        });
    }
    if !grid.passable(goal.xy().0, goal.xy().1) {
        return None;
    }
    let (s, g) = (start.xy(), goal.xy());
    let span = (s.0 - g.0).abs().max((s.1 - g.1).abs());
    // First a window around both ends; the whole grid if the detour is longer.
    let margin = match max_cost {
        Some(cost) => i64::from(cost / u32::from(grid.min_cost.max(1))) + 1,
        None => (span / 2).max(64),
    };
    let window = Window::around(grid, &[s, g], margin);
    if grid.component(s.0, s.1) != grid.component(g.0, g.1) {
        return None;
    }
    let cells = astar(grid, window, start, goal, blocked, max_cost).or_else(|| {
        let wide = Window::around(grid, &[s, g], (span * 2).max(256));
        (max_cost.is_none() && window.len() < wide.len())
            .then(|| astar(grid, wide, start, goal, blocked, None))
            .flatten()
    })?;
    Some(smooth(grid, start, &cells, blocked, goal))
}

fn astar<B: Blocker + ?Sized>(
    grid: &NavGrid,
    window: Window,
    start: Cell,
    goal: Cell,
    blocked: &B,
    max_cost: Option<u32>,
) -> Option<Vec<Cell>> {
    let (gx, gy) = goal.xy();
    let start_local = window.local(start.xy().0, start.xy().1)?;
    let goal_local = window.local(gx, gy)?;
    let costs = grid.costs.as_slice();
    let (width, height) = (i64::from(grid.width), i64::from(grid.height));
    let limit = max_cost.unwrap_or(u32::MAX);
    let min_cost = grid.min_cost;
    SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        let Scratch { g: best, parent } = &mut *scratch;
        best.clear();
        best.resize(window.len(), u32::MAX);
        parent.clear();
        parent.resize(window.len(), u32::MAX);
        let mut heap: BinaryHeap<Reverse<(u32, u32, u32)>> = BinaryHeap::new();
        best[start_local] = 0;
        heap.push(Reverse((
            heuristic(min_cost, start.xy(), (gx, gy)),
            u32::MAX,
            start_local as u32,
        )));
        while let Some(Reverse((_, depth, local))) = heap.pop() {
            let local = local as usize;
            let g = u32::MAX - depth;
            if g > best[local] {
                continue;
            }
            if local == goal_local {
                let mut path = Vec::new();
                let mut current = local;
                while current != start_local {
                    path.push(window.cell(current));
                    current = parent[current] as usize;
                }
                path.reverse();
                return Some(path);
            }
            let lx = local as i64 % window.width;
            let ly = local as i64 / window.width;
            let (x, y) = (window.x0 + lx, window.y0 + ly);
            for (dx, dy) in NEIGHBOURS {
                let (nlx, nly) = (lx + dx, ly + dy);
                if nlx < 0 || nly < 0 || nlx >= window.width || nly >= window.height {
                    continue;
                }
                let (nx, ny) = (x + dx, y + dy);
                let Some(step) = neighbour_cost(costs, width, height, nx, ny, dx != 0 && dy != 0)
                else {
                    continue;
                };
                let total = g + step;
                let next_local = (nly * window.width + nlx) as usize;
                if total > limit || total >= best[next_local] {
                    continue;
                }
                let next = Cell::new(nx as u32, ny as u32);
                if next_local != goal_local && blocked.blocked(next) {
                    continue;
                }
                best[next_local] = total;
                parent[next_local] = local as u32;
                let f = total + heuristic(min_cost, (nx, ny), (gx, gy));
                heap.push(Reverse((f, u32::MAX - total, next_local as u32)));
            }
        }
        None
    })
}
/// Cells of the 8-connected line from `a` (excluded) to `b` (included).
pub fn line_cells(a: Cell, b: Cell) -> Vec<Cell> {
    let (mut x, mut y) = a.xy();
    let (x1, y1) = b.xy();
    let dx = (x1 - x).abs();
    let dy = -(y1 - y).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let sy = if y < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut cells = Vec::with_capacity(dx.max(-dy) as usize);
    while (x, y) != (x1, y1) {
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        cells.push(Cell::new(x as u32, y as u32));
    }
    cells
}

/// Cost of walking `cells` from `from`, or `None` if a step is impossible
/// or enters a blocked cell other than `goal`.
fn chain_cost<B: Blocker + ?Sized>(
    grid: &NavGrid,
    from: Cell,
    cells: &[Cell],
    blocked: &B,
    goal: Cell,
) -> Option<u32> {
    let mut previous = from;
    let mut total = 0;
    for &cell in cells {
        total += step_cost(grid, previous, cell)?;
        if cell != goal && blocked.blocked(cell) {
            return None;
        }
        previous = cell;
    }
    Some(total)
}

/// Longest straight segment tried by the smoothing (cells).
const MAX_SEGMENT: usize = 256;

/// String pulling: from each corner, jump to the farthest cell of the A*
/// chain in line of sight whose straight line costs no more than the chain.
fn smooth<B: Blocker + ?Sized>(
    grid: &NavGrid,
    start: Cell,
    chain: &[Cell],
    blocked: &B,
    goal: Cell,
) -> GridPath {
    // prefix[k]: cost from start to chain[k - 1] (prefix[0] = start).
    let mut prefix = Vec::with_capacity(chain.len() + 1);
    prefix.push(0u32);
    let mut previous = start;
    for &cell in chain {
        let step = step_cost(grid, previous, cell).unwrap_or(u32::from(IMPASSABLE));
        prefix.push(prefix.last().copied().unwrap_or(0) + step);
        previous = cell;
    }
    let point = |k: usize| if k == 0 { start } else { chain[k - 1] };
    let last = chain.len();
    let mut anchor = 0;
    let mut waypoints = Vec::new();
    let mut cells = Vec::with_capacity(chain.len());
    let mut cost = 0;
    while anchor < last {
        let mut best = anchor + 1;
        let mut best_line: Option<(Vec<Cell>, u32)> = None;
        let limit = (anchor + MAX_SEGMENT).min(last);
        for k in anchor + 2..=limit {
            let line = line_cells(point(anchor), point(k));
            match chain_cost(grid, point(anchor), &line, blocked, goal) {
                Some(c) if c <= prefix[k] - prefix[anchor] => {
                    best = k;
                    best_line = Some((line, c));
                }
                _ => break,
            }
        }
        let (line, c) =
            best_line.unwrap_or_else(|| (vec![point(best)], prefix[best] - prefix[anchor]));
        cells.extend(line);
        cost += c;
        waypoints.push(point(best));
        anchor = best;
    }
    GridPath {
        cells,
        waypoints,
        cost,
    }
}

/// Every cell reachable from `start` for at most `budget`, with its cost
/// (bounded Dijkstra). `blocked` cells are never entered. Sorted by cell.
pub fn reachable_area<B: Blocker + ?Sized>(
    grid: &NavGrid,
    start: Cell,
    budget: u32,
    blocked: &B,
) -> Vec<(Cell, u32)> {
    let mut result = Vec::new();
    bounded_dijkstra(grid, start, budget, blocked, |cell, cost| {
        result.push((cell, cost));
        false
    });
    result.sort_unstable();
    result
}

/// Bounded Dijkstra from `start`, calling `visit(cell, cost)` on every
/// settled cell in order of cost (ties by position in the search window,
/// row-major: deterministic); stops early when `visit` returns `true` and
/// returns that cell with its cost. `visit` must not start another search.
pub fn bounded_dijkstra<B: Blocker + ?Sized>(
    grid: &NavGrid,
    start: Cell,
    budget: u32,
    blocked: &B,
    mut visit: impl FnMut(Cell, u32) -> bool,
) -> Option<(Cell, u32)> {
    let s = start.xy();
    let margin = i64::from(budget / u32::from(grid.min_cost.max(1))) + 1;
    let window = Window::around(grid, &[s], margin);
    let start_local = window.local(s.0, s.1)?;
    let costs = grid.costs.as_slice();
    let (width, height) = (i64::from(grid.width), i64::from(grid.height));
    SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        let Scratch {
            g: best,
            parent: settled,
        } = &mut *scratch;
        best.clear();
        best.resize(window.len(), u32::MAX);
        settled.clear();
        settled.resize(window.len(), 0);
        let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
        best[start_local] = 0;
        heap.push(Reverse((0, start_local as u32)));
        while let Some(Reverse((g, local))) = heap.pop() {
            let local = local as usize;
            if g > best[local] || settled[local] != 0 {
                continue;
            }
            settled[local] = 1;
            let lx = local as i64 % window.width;
            let ly = local as i64 / window.width;
            let (x, y) = (window.x0 + lx, window.y0 + ly);
            let cell = Cell::new(x as u32, y as u32);
            if visit(cell, g) {
                return Some((cell, g));
            }
            for (dx, dy) in NEIGHBOURS {
                let (nlx, nly) = (lx + dx, ly + dy);
                if nlx < 0 || nly < 0 || nlx >= window.width || nly >= window.height {
                    continue;
                }
                let (nx, ny) = (x + dx, y + dy);
                let Some(step) = neighbour_cost(costs, width, height, nx, ny, dx != 0 && dy != 0)
                else {
                    continue;
                };
                let total = g + step;
                let next_local = (nly * window.width + nlx) as usize;
                if total > budget || total >= best[next_local] {
                    continue;
                }
                if blocked.blocked(Cell::new(nx as u32, ny as u32)) {
                    continue;
                }
                best[next_local] = total;
                heap.push(Reverse((total, next_local as u32)));
            }
        }
        None
    })
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use data_model::PLAIN_COST;

    fn grid(width: u32, height: u32) -> NavGrid {
        NavGrid::uniform(width, height, 2, 1.438, PLAIN_COST)
    }

    #[test]
    fn straight_line_on_a_plain() {
        let g = grid(50, 50);
        let path = find_path(
            &g,
            Cell::new(5, 5),
            Cell::new(25, 5),
            &nothing_blocked,
            None,
        )
        .expect("path");
        assert_eq!(path.cost, 200);
        assert_eq!(path.cells.len(), 20);
        assert_eq!(path.waypoints, vec![Cell::new(25, 5)]);
    }

    #[test]
    fn smoothing_straightens_diagonals() {
        let g = grid(60, 60);
        let path = find_path(
            &g,
            Cell::new(0, 0),
            Cell::new(40, 13),
            &nothing_blocked,
            None,
        )
        .expect("path");
        // Octile: 27 orthogonal + 13 diagonal steps.
        assert_eq!(path.cost, 27 * 10 + 13 * 14);
        assert_eq!(path.waypoints, vec![Cell::new(40, 13)]);
    }

    #[test]
    fn a_closed_wall_cannot_be_crossed() {
        let mut g = grid(20, 20);
        for i in 0..20 {
            g.set(i, 19 - i, IMPASSABLE);
            g.set(i, (18 - i).max(0), IMPASSABLE);
        }
        g.refresh_min_cost();
        assert!(find_path(
            &g,
            Cell::new(0, 0),
            Cell::new(19, 19),
            &nothing_blocked,
            None
        )
        .is_none());
    }

    #[test]
    fn blocked_cells_are_avoided_but_the_goal_is_entered() {
        let g = grid(30, 30);
        let blocked = |c: Cell| c.x == 10 && c.y < 20;
        let path = find_path(&g, Cell::new(5, 5), Cell::new(10, 5), &blocked, None).expect("goal");
        assert_eq!(path.cells.last(), Some(&Cell::new(10, 5)));
        let path =
            find_path(&g, Cell::new(5, 5), Cell::new(15, 5), &blocked, None).expect("around");
        assert!(path.cells.iter().all(|c| !blocked(*c)));
        assert!(path.cells.iter().any(|c| c.y >= 20));
    }

    #[test]
    fn reachable_area_is_bounded() {
        let g = grid(100, 100);
        let area = reachable_area(&g, Cell::new(50, 50), 30, &nothing_blocked);
        assert!(area.iter().all(|(_, c)| *c <= 30));
        assert!(area.contains(&(Cell::new(53, 50), 30)));
        assert!(!area.iter().any(|(c, _)| *c == Cell::new(54, 50)));
        assert!(area.contains(&(Cell::new(52, 52), 28)));
    }

    #[test]
    fn max_cost_bounds_find_path() {
        let g = grid(100, 100);
        assert!(find_path(
            &g,
            Cell::new(0, 0),
            Cell::new(50, 0),
            &nothing_blocked,
            Some(400)
        )
        .is_none());
        assert!(find_path(
            &g,
            Cell::new(0, 0),
            Cell::new(40, 0),
            &nothing_blocked,
            Some(400)
        )
        .is_some());
    }

    #[test]
    fn line_cells_are_connected() {
        let cells = line_cells(Cell::new(0, 0), Cell::new(7, 3));
        assert_eq!(cells.len(), 7);
        assert_eq!(cells.last(), Some(&Cell::new(7, 3)));
    }
}
