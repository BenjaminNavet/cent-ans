//! Soft separation of friendly regiments (F5a § 1).
//!
//! Two regiments of the same side whose formation rectangles overlap are
//! nudged apart a little every tick: deterministic (pairs in index order,
//! displacements summed then applied), capped so that nobody teleports, and
//! charging regiments are never slowed (the other one gives way).

use super::{BattleSim, DT};
use crate::unit::{Unit, UnitState};

/// Gap kept between two friendly regiments (metres).
pub const FRIEND_GAP: f64 = 1.0;
/// Fastest sideways drift caused by the separation (m/s).
const MAX_PUSH_SPEED: f64 = 3.0;

/// Can this regiment be pushed aside?
fn pushable(unit: &Unit) -> bool {
    unit.present()
        && !unit.on_wall
        && unit.climbing.is_none()
        && matches!(unit.state, UnitState::Idle | UnitState::Rallied)
        && unit.target.is_none()
        && unit.destination.is_none()
        && !unit.withdrawing
}

/// Marching, charging or closing with a target.
fn moving(unit: &Unit) -> bool {
    matches!(unit.state, UnitState::Marching | UnitState::Charging)
        || unit.destination.is_some()
        || (unit.target.is_some() && !matches!(unit.state, UnitState::Shooting | UnitState::Melee))
}

/// Does this regiment take part in the separation at all?
fn solid(unit: &Unit) -> bool {
    unit.present() && unit.state != UnitState::Routing && !unit.on_wall && !unit.withdrawing
}

impl BattleSim {
    /// One step of friendly separation (called after movement).
    pub(super) fn separate_friends(&mut self) {
        let n = self.units.len();
        let mut push = vec![(0.0_f64, 0.0_f64); n];
        for i in 0..n {
            let a = &self.units[i];
            if !solid(a) {
                continue;
            }
            for j in (i + 1)..n {
                let b = &self.units[j];
                if b.side != a.side || !solid(b) {
                    continue;
                }
                let (dx, dz) = (b.x - a.x, b.z - a.z);
                let dist = (dx * dx + dz * dz).sqrt();
                if dist > 200.0 {
                    continue;
                }
                let dir = if dist > 1e-6 {
                    (dx / dist, dz / dist)
                } else {
                    (1.0, 0.0)
                };
                // Same separating test as the enemy contacts (sim::contacts).
                let gap_ab = a.distance_to_rect(b.x, b.z) - b.support(dir);
                let gap_ba = b.distance_to_rect(a.x, a.z) - a.support(dir);
                let overlap = FRIEND_GAP - gap_ab.max(gap_ba);
                if overlap <= 0.0 {
                    continue;
                }
                // Moving regiments pass through their friends (passage of
                // lines): pushing the one in the way would snowplough it.
                if moving(a) || moving(b) {
                    continue;
                }
                let (pa, pb) = (pushable(a), pushable(b));
                let (sa, sb) = match (pa, pb) {
                    (true, true) => (0.5, 0.5),
                    (true, false) => (1.0, 0.0),
                    (false, true) => (0.0, 1.0),
                    (false, false) => continue,
                };
                push[i].0 -= dir.0 * overlap * sa;
                push[i].1 -= dir.1 * overlap * sa;
                push[j].0 += dir.0 * overlap * sb;
                push[j].1 += dir.1 * overlap * sb;
            }
        }
        let cap = MAX_PUSH_SPEED * DT;
        let (width, depth) = (self.field.width, self.field.depth);
        for (i, (px, pz)) in push.into_iter().enumerate() {
            let len = (px * px + pz * pz).sqrt();
            if len < 1e-9 {
                continue;
            }
            let k = len.min(cap) / len;
            let from = (self.units[i].x, self.units[i].z);
            let to = (
                (from.0 + px * k).clamp(1.0, width - 1.0),
                (from.1 + pz * k).clamp(1.0, depth - 1.0),
            );
            if self.wall_block(i, from, to).is_some() || self.water_blocks(i, from, to) {
                continue;
            }
            self.units[i].x = to.0;
            self.units[i].z = to.1;
        }
    }
}
