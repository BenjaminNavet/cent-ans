//! A reusable spatial index (AD12): points bucketed in square cells, so a
//! "near" query reads the cells around the point instead of every point.

use std::collections::BTreeMap;

use data_model::util::dist;

/// Points (map pixels) bucketed in square cells of `cell` pixels.
pub struct SpatialIndex {
    cell: f32,
    points: Vec<[f32; 2]>,
    buckets: BTreeMap<(i32, i32), Vec<usize>>,
}

impl SpatialIndex {
    /// Indexes `points`; a query keeps their order (index into `points`).
    pub fn new(points: Vec<[f32; 2]>, cell: f32) -> Self {
        let cell = cell.max(1.0);
        let mut buckets: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
        for (i, p) in points.iter().enumerate() {
            buckets.entry(Self::cell_of(cell, *p)).or_default().push(i);
        }
        SpatialIndex {
            cell,
            points,
            buckets,
        }
    }

    fn cell_of(cell: f32, p: [f32; 2]) -> (i32, i32) {
        ((p[0] / cell).floor() as i32, (p[1] / cell).floor() as i32)
    }

    /// Indices of the points within `radius` of `point` (`dist <= radius`),
    /// ascending.
    pub fn within(&self, point: [f32; 2], radius: f32) -> Vec<usize> {
        let span = (radius / self.cell).ceil().max(1.0) as i32;
        let (cx, cy) = Self::cell_of(self.cell, point);
        let mut found: Vec<usize> = Vec::new();
        for x in cx - span..=cx + span {
            for y in cy - span..=cy + span {
                if let Some(bucket) = self.buckets.get(&(x, y)) {
                    found.extend(
                        bucket
                            .iter()
                            .copied()
                            .filter(|i| dist(self.points[*i], point) <= radius),
                    );
                }
            }
        }
        found.sort_unstable();
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_matches_linear_scan() {
        let points: Vec<[f32; 2]> = (0..200)
            .map(|i| [(i * 37 % 101) as f32 * 3.7, (i * 53 % 97) as f32 * 4.1])
            .collect();
        for cell in [5.0, 20.0, 80.0] {
            let index = SpatialIndex::new(points.clone(), cell);
            for q in [[0.0, 0.0], [150.0, 200.0], [370.0, 390.0]] {
                for radius in [10.0, 40.0, 120.0] {
                    let want: Vec<usize> = (0..points.len())
                        .filter(|i| dist(points[*i], q) <= radius)
                        .collect();
                    assert_eq!(index.within(q, radius), want);
                }
            }
        }
    }
}
