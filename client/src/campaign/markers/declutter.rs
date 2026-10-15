//! Greedy screen-rectangle placement by priority (Total War style). Port of
//! `marker_declutter.gd`: rectangles are placed in priority order; one that overlaps an already
//! placed rectangle yields (unless forced). Fixed-cell spatial grid, ~linear cost.

use std::collections::HashMap;

use bevy::math::Rect;

#[derive(Debug, Clone)]
pub struct Declutter {
    cell_px: f32,
    rects: Vec<Rect>,
    owners: Vec<i32>,
    grid: HashMap<(i32, i32), Vec<usize>>,
}

impl Default for Declutter {
    fn default() -> Self {
        Self::new(64.0)
    }
}

/// Strict area overlap (touching edges do not count), like Godot's `Rect2.intersects`.
fn intersects(a: &Rect, b: &Rect) -> bool {
    a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
}

impl Declutter {
    pub fn new(cell: f32) -> Self {
        Self {
            cell_px: cell.max(4.0),
            rects: Vec::new(),
            owners: Vec::new(),
            grid: HashMap::new(),
        }
    }

    pub fn reset(&mut self, cell: f32) {
        *self = Self::new(cell);
    }

    pub fn placed_count(&self) -> usize {
        self.rects.len()
    }

    fn keys(&self, r: &Rect) -> Vec<(i32, i32)> {
        let c = self.cell_px;
        let (x0, y0) = ((r.min.x / c).floor() as i32, (r.min.y / c).floor() as i32);
        let (x1, y1) = ((r.max.x / c).floor() as i32, (r.max.y / c).floor() as i32);
        let mut keys = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                keys.push((x, y));
            }
        }
        keys
    }

    /// True when `rect` overlaps a placed rectangle (those of `ignore_owner` excepted).
    pub fn overlaps(&self, rect: &Rect, ignore_owner: i32) -> bool {
        self.keys(rect).iter().any(|k| {
            self.grid.get(k).is_some_and(|list| {
                list.iter()
                    .any(|&i| self.owners[i] != ignore_owner && intersects(&self.rects[i], rect))
            })
        })
    }

    /// Places `rect` if free (or if `force`); returns true when placed.
    pub fn try_place(&mut self, rect: Rect, owner: i32, force: bool) -> bool {
        if !force && self.overlaps(&rect, owner) {
            return false;
        }
        let index = self.rects.len();
        for k in self.keys(&rect) {
            self.grid.entry(k).or_default().push(index);
        }
        self.rects.push(rect);
        self.owners.push(owner);
        true
    }
}

/// Number of overlapping pairs, ignoring pairs of the same owner (>= 0). Quadratic: tests only.
pub fn count_overlaps(rects: &[Rect], owners: &[i32]) -> usize {
    let mut count = 0;
    for a in 0..rects.len() {
        for b in (a + 1)..rects.len() {
            if owners.len() == rects.len() && owners[a] >= 0 && owners[a] == owners[b] {
                continue;
            }
            if intersects(&rects[a], &rects[b]) {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec2;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_corners(Vec2::new(x, y), Vec2::new(x + w, y + h))
    }

    #[test]
    fn overlap_yields_and_force_wins() {
        let mut d = Declutter::default();
        assert!(d.try_place(r(0.0, 0.0, 50.0, 20.0), 1, false));
        assert!(!d.try_place(r(40.0, 10.0, 50.0, 20.0), 2, false));
        assert!(d.try_place(r(40.0, 10.0, 50.0, 20.0), 2, true));
        assert_eq!(d.placed_count(), 2);
    }

    #[test]
    fn same_owner_and_touching_edges_do_not_block() {
        let mut d = Declutter::default();
        assert!(d.try_place(r(0.0, 0.0, 50.0, 20.0), 1, false));
        assert!(d.try_place(r(10.0, 0.0, 10.0, 10.0), 1, false)); // own marker and name
        assert!(d.try_place(r(50.0, 0.0, 10.0, 10.0), 3, false)); // touching only
    }

    #[test]
    fn rect_spanning_many_cells_and_negative_coordinates() {
        let mut d = Declutter::new(16.0);
        assert!(d.try_place(r(-100.0, -100.0, 300.0, 5.0), 1, false));
        assert!(!d.try_place(r(150.0, -102.0, 10.0, 10.0), 2, false));
        assert!(d.try_place(r(150.0, -50.0, 10.0, 10.0), 2, false));
    }

    #[test]
    fn count_overlaps_ignores_same_owner() {
        let rects = [
            r(0.0, 0.0, 10.0, 10.0),
            r(5.0, 5.0, 10.0, 10.0),
            r(6.0, 6.0, 2.0, 2.0),
        ];
        assert_eq!(count_overlaps(&rects, &[]), 3);
        assert_eq!(count_overlaps(&rects, &[1, 1, 2]), 2);
    }
}
