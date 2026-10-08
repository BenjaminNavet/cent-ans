//! Neighbourhood of the regiments: the pairs of centres close enough to
//! interact, found with a uniform grid instead of testing every pair.

use super::BattleSim;
use crate::unit::Unit;

impl BattleSim {
    /// The pairs `(i, j)`, `i < j`, of regiments accepted by `keep` whose
    /// centres are at most `radius` metres apart, sorted by `(i, j)`: the
    /// order in which a double loop over the regiments meets them, so a
    /// caller summing floats over the pairs gets the same bits.
    pub(super) fn near_pairs(
        &self,
        radius: f64,
        keep: impl Fn(&Unit) -> bool,
    ) -> Vec<(usize, usize)> {
        let cell_of =
            |v: f64, cells: usize| ((v / radius).floor().max(0.0) as usize).min(cells - 1);
        let cols = (self.field.width / radius) as usize + 1;
        let rows = (self.field.depth / radius) as usize + 1;
        // Counting sort of the kept regiments by cell (units keep id order
        // inside a cell). Regiments off the field fall in the edge cells,
        // which only adds candidates.
        let mut start = vec![0usize; cols * rows + 1];
        let cells: Vec<Option<(usize, usize)>> = self
            .units
            .iter()
            .map(|u| keep(u).then(|| (cell_of(u.x, cols), cell_of(u.z, rows))))
            .collect();
        for &(cx, cz) in cells.iter().flatten() {
            start[cz * cols + cx + 1] += 1;
        }
        for c in 0..cols * rows {
            start[c + 1] += start[c];
        }
        let mut fill = start.clone();
        let mut members = vec![0usize; start[cols * rows]];
        for (i, cell) in cells.iter().enumerate() {
            if let Some((cx, cz)) = cell {
                let slot = &mut fill[cz * cols + cx];
                members[*slot] = i;
                *slot += 1;
            }
        }
        let limit = radius * radius;
        let mut pairs = Vec::new();
        let mut row = Vec::new();
        for (i, cell) in cells.iter().enumerate() {
            let Some(&(cx, cz)) = cell.as_ref() else {
                continue;
            };
            let (a_x, a_z) = (self.units[i].x, self.units[i].z);
            row.clear();
            for nz in cz.saturating_sub(1)..=(cz + 1).min(rows - 1) {
                for nx in cx.saturating_sub(1)..=(cx + 1).min(cols - 1) {
                    let c = nz * cols + nx;
                    for &j in &members[start[c]..start[c + 1]] {
                        let b = &self.units[j];
                        let (dx, dz) = (b.x - a_x, b.z - a_z);
                        if j > i && dx * dx + dz * dz <= limit {
                            row.push(j);
                        }
                    }
                }
            }
            row.sort_unstable();
            pairs.extend(row.iter().map(|&j| (i, j)));
        }
        pairs
    }
}
