//! Plane geometry shared by the battle code: points are `(x, z)` in metres.
//! The formulas are the ones the battle rules were tuned with: change their
//! rounding and the replays change.

/// Parameter in `[0, 1]` of the point of the segment `a`-`b` nearest `p`
/// (0 for a degenerate segment).
pub fn segment_param(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = (dx * dx + dz * dz).max(1e-9);
    (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0)
}

/// Point of the segment `a`-`b` nearest `p`.
pub fn closest_on_segment(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> (f64, f64) {
    let t = segment_param(a, b, p);
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Distance from `p` to the segment `a`-`b`, through the closest point.
pub fn distance_to_segment(a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> f64 {
    let (cx, cz) = closest_on_segment(a, b, p);
    ((p.0 - cx).powi(2) + (p.1 - cz).powi(2)).sqrt()
}

/// Distance from `p` to the segment `a`-`b` (`hypot` form of the river and
/// AI code).
pub fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dz * dz;
    let t = if len2 < 1e-9 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - dx * t).hypot(p.1 - a.1 - dz * t)
}

/// Is `p` inside the closed polygon `ring` (even-odd rule)?
pub fn point_in_polygon(ring: &[(f64, f64)], p: (f64, f64)) -> bool {
    let mut inside = false;
    let mut j = ring.len() - 1;
    for (i, &(xi, zi)) in ring.iter().enumerate() {
        let (xj, zj) = ring[j];
        if (zi > p.1) != (zj > p.1) && p.0 < (xj - xi) * (p.1 - zi) / (zj - zi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Twice the signed area of the triangle `o`, `a`, `b`.
fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Proper intersection of the segments `p1`-`p2` and `q1`-`q2`.
pub fn segments_intersect(p1: (f64, f64), p2: (f64, f64), q1: (f64, f64), q2: (f64, f64)) -> bool {
    let d1 = cross(q1, q2, p1);
    let d2 = cross(q1, q2, p2);
    let d3 = cross(p1, p2, q1);
    let d4 = cross(p1, p2, q2);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// A uniform grid over thick polylines: answers "which polylines pass within
/// their reach of `p`" by testing only the segments of `p`'s cell, instead of
/// every segment of the field. Built once from the polylines, read-only after.
#[derive(Debug, Clone, Default)]
pub struct SegmentGrid {
    origin: (f64, f64),
    cols: usize,
    rows: usize,
    cells: Vec<Vec<u32>>,
    segments: Vec<GridSegment>,
}

#[derive(Debug, Clone, Copy)]
struct GridSegment {
    a: (f64, f64),
    b: (f64, f64),
    reach: f64,
    tag: u32,
}

/// Side of a cell of a [`SegmentGrid`], in metres.
const GRID_CELL: f64 = 32.0;

impl SegmentGrid {
    /// Grid of the polylines `(points, reach, tag)`: a point is "hit" by a
    /// polyline when it lies within `reach` of one of its segments (a single
    /// point counts as a degenerate segment).
    pub fn build<'a>(lines: impl IntoIterator<Item = (&'a [(f64, f64)], f64, u32)>) -> Self {
        let mut segments = Vec::new();
        for (points, reach, tag) in lines {
            if let [only] = points {
                segments.push(GridSegment {
                    a: *only,
                    b: *only,
                    reach,
                    tag,
                });
            }
            for w in points.windows(2) {
                segments.push(GridSegment {
                    a: w[0],
                    b: w[1],
                    reach,
                    tag,
                });
            }
        }
        if segments.is_empty() {
            return Self::default();
        }
        let (mut x0, mut z0, mut x1, mut z1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for s in &segments {
            x0 = x0.min(s.a.0.min(s.b.0) - s.reach);
            z0 = z0.min(s.a.1.min(s.b.1) - s.reach);
            x1 = x1.max(s.a.0.max(s.b.0) + s.reach);
            z1 = z1.max(s.a.1.max(s.b.1) + s.reach);
        }
        let cols = ((x1 - x0) / GRID_CELL).floor() as usize + 1;
        let rows = ((z1 - z0) / GRID_CELL).floor() as usize + 1;
        let mut grid = SegmentGrid {
            origin: (x0, z0),
            cols,
            rows,
            cells: vec![Vec::new(); cols * rows],
            segments,
        };
        for (i, s) in grid.segments.clone().iter().enumerate() {
            let (cx0, cz0) = grid.cell_of(s.a.0.min(s.b.0) - s.reach, s.a.1.min(s.b.1) - s.reach);
            let (cx1, cz1) = grid.cell_of(s.a.0.max(s.b.0) + s.reach, s.a.1.max(s.b.1) + s.reach);
            for cz in cz0..=cz1 {
                for cx in cx0..=cx1 {
                    grid.cells[cz * cols + cx].push(i as u32);
                }
            }
        }
        grid
    }

    fn cell_of(&self, x: f64, z: f64) -> (usize, usize) {
        let cx = (((x - self.origin.0) / GRID_CELL) as usize).min(self.cols - 1);
        let cz = (((z - self.origin.1) / GRID_CELL) as usize).min(self.rows - 1);
        (cx, cz)
    }

    /// Tags of the segments within reach of `p` (a tag repeats once per
    /// segment hit).
    pub fn hits(&self, x: f64, z: f64) -> impl Iterator<Item = u32> + '_ {
        let inside = !self.cells.is_empty()
            && x >= self.origin.0
            && z >= self.origin.1
            && x < self.origin.0 + self.cols as f64 * GRID_CELL
            && z < self.origin.1 + self.rows as f64 * GRID_CELL;
        let cell: &[u32] = if inside {
            let (cx, cz) = self.cell_of(x, z);
            &self.cells[cz * self.cols + cx]
        } else {
            &[]
        };
        cell.iter().filter_map(move |&i| {
            let s = &self.segments[i as usize];
            (segment_distance((x, z), s.a, s.b) <= s.reach).then_some(s.tag)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_clamp_to_the_ends() {
        let (a, b) = ((0.0, 0.0), (10.0, 0.0));
        assert_eq!(distance_to_segment(a, b, (5.0, 3.0)), 3.0);
        assert_eq!(distance_to_segment(a, b, (13.0, 4.0)), 5.0);
        assert_eq!(segment_distance((13.0, 4.0), a, b), 5.0);
        assert_eq!(closest_on_segment(a, b, (-4.0, 1.0)), a);
        assert_eq!(distance_to_segment(a, a, (3.0, 4.0)), 5.0);
    }

    #[test]
    fn grid_matches_the_exhaustive_scan() {
        let lines: [(Vec<(f64, f64)>, f64, u32); 3] = [
            (vec![(10.0, 10.0), (200.0, 40.0), (260.0, 300.0)], 5.0, 0),
            (vec![(0.0, 150.0), (300.0, 160.0)], 3.0, 1),
            (vec![(120.0, 120.0)], 6.0, 2),
        ];
        let grid = SegmentGrid::build(lines.iter().map(|(p, r, t)| (p.as_slice(), *r, *t)));
        for ix in -10..40 {
            for iz in -10..40 {
                let (x, z) = (f64::from(ix) * 8.0, f64::from(iz) * 8.0);
                let mut want: Vec<u32> = Vec::new();
                for (p, r, t) in &lines {
                    let hit = if p.len() == 1 {
                        (x - p[0].0).hypot(z - p[0].1) <= *r
                    } else {
                        p.windows(2)
                            .any(|w| segment_distance((x, z), w[0], w[1]) <= *r)
                    };
                    if hit {
                        want.push(*t);
                    }
                }
                let mut got: Vec<u32> = grid.hits(x, z).collect();
                got.sort_unstable();
                got.dedup();
                assert_eq!(got, want, "at ({x}, {z})");
            }
        }
    }

    #[test]
    fn polygon_and_segment_tests() {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_polygon(&square, (5.0, 5.0)));
        assert!(!point_in_polygon(&square, (15.0, 5.0)));
        assert!(segments_intersect(
            (0.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (10.0, 0.0)
        ));
        assert!(!segments_intersect(
            (0.0, 0.0),
            (1.0, 1.0),
            (5.0, 0.0),
            (5.0, 9.0)
        ));
    }
}
