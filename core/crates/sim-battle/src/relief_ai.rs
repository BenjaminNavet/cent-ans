//! Tactical reading of the relief for the battle AI (lot R2b).
//!
//! Built once per battle from the [`Battlefield`] (cached by
//! [`crate::sim::BattleSim::relief_map`]) and read by [`crate::ai`]:
//! - **local relative height** (prominence): the ground minus the mean ground
//!   around it; positive on crests and spurs, negative in hollows and valleys;
//! - **slope** and the **climb** along a march (the steepest rise met);
//! - **march cost**: the length of a march weighted like the simulation's
//!   slowdown uphill, to go round a slope by a valley or a shelf;
//! - **line of sight** (does the ground mask the target?);
//! - **reverse slope**: a spot just behind a crest, masked from the enemy;
//! - **water** (EP3): does the river lie between two points, what a crossing
//!   costs (march, a narrow deck filed across, a steep far bank), and how
//!   many enemy shooters cover a crossing's far end.
//!
//! Pure and deterministic (grid order, strict improvements only).

use crate::field::Battlefield;
use crate::hydro::{BankKind, Crossing};

/// Half-side of the square window of the local mean (cells of the field
/// grid, 10 m): prominence compares a point with the ground within ~80 m.
pub const PROMINENCE_RADIUS: usize = 8;
/// Sampling step along a march or a line of sight (metres).
const STEP: f64 = 10.0;
/// Baseline of the grade measured along a march (metres): shorter banks
/// than this are crossed without notice.
const GRADE_BASE: f64 = 20.0;
/// Weight of the rise in the march cost, as in the simulation's speed
/// (`1 / (1 + 6 grade)` uphill).
const UPHILL_WEIGHT: f64 = 6.0;

/// Tactical reading of the relief of one battlefield.
#[derive(Debug, Clone, PartialEq)]
pub struct ReliefMap {
    nx: usize,
    nz: usize,
    resolution: f64,
    /// Ground minus the local mean ground, per cell of the field grid.
    prominence: Vec<f64>,
}

impl ReliefMap {
    /// Reads the relief of `field` (summed-area table: linear in the grid).
    pub fn new(field: &Battlefield) -> Self {
        let (nx, nz) = (field.nx, field.nz);
        // Summed-area table with a zero row and column in front.
        let mut sums = vec![0.0; (nx + 1) * (nz + 1)];
        for j in 0..nz {
            for i in 0..nx {
                sums[(j + 1) * (nx + 1) + i + 1] = field.heights[j * nx + i]
                    + sums[j * (nx + 1) + i + 1]
                    + sums[(j + 1) * (nx + 1) + i]
                    - sums[j * (nx + 1) + i];
            }
        }
        let r = PROMINENCE_RADIUS;
        let mut prominence = vec![0.0; nx * nz];
        for j in 0..nz {
            for i in 0..nx {
                let (i0, i1) = (i.saturating_sub(r), (i + r + 1).min(nx));
                let (j0, j1) = (j.saturating_sub(r), (j + r + 1).min(nz));
                let total =
                    sums[j1 * (nx + 1) + i1] - sums[j0 * (nx + 1) + i1] - sums[j1 * (nx + 1) + i0]
                        + sums[j0 * (nx + 1) + i0];
                let mean = total / ((i1 - i0) * (j1 - j0)) as f64;
                prominence[j * nx + i] = field.heights[j * nx + i] - mean;
            }
        }
        ReliefMap {
            nx,
            nz,
            resolution: field.resolution,
            prominence,
        }
    }

    /// Ground at (x, z) minus the mean ground around it (bilinear).
    pub fn prominence(&self, x: f64, z: f64) -> f64 {
        let fx = (x / self.resolution).clamp(0.0, (self.nx - 1) as f64);
        let fz = (z / self.resolution).clamp(0.0, (self.nz - 1) as f64);
        let (ix, iz) = (fx.floor() as usize, fz.floor() as usize);
        let (ix1, iz1) = ((ix + 1).min(self.nx - 1), (iz + 1).min(self.nz - 1));
        let (tx, tz) = (fx - ix as f64, fz - iz as f64);
        let at = |i: usize, j: usize| self.prominence[j * self.nx + i];
        let top = at(ix, iz) * (1.0 - tx) + at(ix1, iz) * tx;
        let bottom = at(ix, iz1) * (1.0 - tx) + at(ix1, iz1) * tx;
        top * (1.0 - tz) + bottom * tz
    }

    /// Ground gradient at (x, z) (metres of rise per metre, towards +x, +z).
    pub fn gradient(field: &Battlefield, x: f64, z: f64) -> (f64, f64) {
        let h = GRADE_BASE * 0.5;
        (
            (field.height(x + h, z) - field.height(x - h, z)) / (2.0 * h),
            (field.height(x, z + h) - field.height(x, z - h)) / (2.0 * h),
        )
    }

    /// Steepness of the ground at (x, z) (metres per metre).
    pub fn slope(field: &Battlefield, x: f64, z: f64) -> f64 {
        let (gx, gz) = Self::gradient(field, x, z);
        gx.hypot(gz)
    }

    /// Steepest rise (grade over [`GRADE_BASE`]) met marching from `from` to
    /// `to`; 0 when the march only goes down or along the slope.
    pub fn climb(field: &Battlefield, from: (f64, f64), to: (f64, f64)) -> f64 {
        let (dx, dz) = (to.0 - from.0, to.1 - from.1);
        let len = dx.hypot(dz);
        if len < 1e-6 {
            return 0.0;
        }
        let (ux, uz) = (dx / len, dz / len);
        let steps = (len / STEP).ceil().max(1.0) as usize;
        (0..steps)
            .map(|k| {
                let s = (k as f64 * STEP).min(len);
                let (x, z) = (from.0 + ux * s, from.1 + uz * s);
                let rise =
                    field.height(x + ux * GRADE_BASE, z + uz * GRADE_BASE) - field.height(x, z);
                rise / GRADE_BASE
            })
            .fold(0.0, f64::max)
    }

    /// Length of the march from `from` to `to`, each stretch lengthened by
    /// its rise as the simulation slows a regiment uphill.
    pub fn march_cost(field: &Battlefield, from: (f64, f64), to: (f64, f64)) -> f64 {
        let (dx, dz) = (to.0 - from.0, to.1 - from.1);
        let len = dx.hypot(dz);
        let steps = (len / STEP).ceil().max(1.0) as usize;
        let piece = len / steps as f64;
        let mut cost = 0.0;
        let mut h0 = field.height(from.0, from.1);
        for k in 1..=steps {
            let t = k as f64 / steps as f64;
            let h1 = field.height(from.0 + dx * t, from.1 + dz * t);
            let grade = ((h1 - h0) / piece.max(1e-6)).max(0.0);
            cost += piece * (1.0 + UPHILL_WEIGHT * grade);
            h0 = h1;
        }
        cost
    }

    /// Can a regiment at `from` see one at `to` (the ground does not rise
    /// above the line of sight, as the simulation's shooting checks)?
    pub fn sees(field: &Battlefield, from: (f64, f64), to: (f64, f64)) -> bool {
        !field.blocks_sight(from, to)
    }

    /// Reverse slope behind the crest at `crest` for a side facing
    /// `forward` (+1 towards +z): the nearest point up to `reach` metres
    /// behind it from which an enemy standing `watch` metres in front of the
    /// crest can no longer see the regiment. `None` on a flat or a forward
    /// slope.
    pub fn reverse_slope(
        field: &Battlefield,
        crest: (f64, f64),
        forward: f64,
        watch: f64,
        reach: f64,
    ) -> Option<(f64, f64)> {
        let enemy = (crest.0, crest.1 + forward * watch);
        let mut back = STEP;
        while back <= reach {
            let spot = (crest.0, crest.1 - forward * back);
            if field.inside(spot.0, spot.1) && field.blocks_sight(enemy, spot) {
                return Some(spot);
            }
            back += STEP;
        }
        None
    }

    /// EP3: does the main river lie between `a` and `b`?
    pub fn river_between(field: &Battlefield, a: (f64, f64), b: (f64, f64)) -> bool {
        field
            .river
            .as_ref()
            .is_some_and(|r| r.north_of(a.0, a.1) != r.north_of(b.0, b.1))
    }

    /// EP3: cost (metres of march) of going from `from` to `to` by
    /// `crossing`: the march to its near end and from its far end (relief
    /// counted), the crossing itself lengthened when a front of `frontage`
    /// metres must file across a narrower deck or ford, and a steep far
    /// bank.
    pub fn crossing_cost(
        field: &Battlefield,
        from: (f64, f64),
        crossing: &Crossing,
        to: (f64, f64),
        frontage: f64,
    ) -> f64 {
        let north = field
            .river
            .as_ref()
            .is_some_and(|r| r.north_of(from.0, from.1));
        let (near, far) = (crossing.end(north), crossing.end(!north));
        let across = (far.0 - near.0).hypot(far.1 - near.1);
        let squeeze = (frontage / crossing.width.max(1.0)).max(1.0);
        let steep = if field.bank_kind(far.0, far.1 + (far.1 - near.1).signum() * 4.0)
            == Some(BankKind::Steep)
        {
            40.0
        } else {
            0.0
        };
        Self::march_cost(field, from, near)
            + across * squeeze
            + Self::march_cost(field, far, to)
            + steep
    }

    /// EP3: how many of the enemy shooters `foes` (x, z, range) reach the
    /// far end `end` of a crossing.
    pub fn covered(end: (f64, f64), foes: &[(f64, f64, f64)]) -> usize {
        foes.iter()
            .filter(|&&(x, z, r)| (x - end.0).hypot(z - end.1) <= r + 10.0)
            .count()
    }
}
