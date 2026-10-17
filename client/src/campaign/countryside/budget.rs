//! Per-frame spawn budget of the countryside streaming (`render.spawn_roots_per_frame`).
//!
//! Spawning a cell (props) or a ring of outcrops in one frame adds thousands of meshes at once;
//! Bevy then pays `calculate_bounds`, the pipeline specialisation checks and the render-world
//! extraction for all of them in that single frame (94-190 ms in the descent bench). The budget
//! is shared by every spawner of the frame so the work is spread over several frames.

use bevy::prelude::*;

/// Roots still allowed this frame, shared by the spawners (refilled on the first use of a frame).
#[derive(Resource, Default)]
pub struct SpawnBudget {
    left: usize,
    stamp: Option<u32>,
}

impl SpawnBudget {
    /// Remaining budget of frame `frame` (refilled to `per_frame` when the frame changed).
    pub fn left(&mut self, frame: u32, per_frame: usize) -> usize {
        if self.stamp != Some(frame) {
            self.stamp = Some(frame);
            self.left = per_frame;
        }
        self.left
    }

    /// Takes up to `pending` roots from the frame's budget.
    pub fn take(&mut self, frame: u32, per_frame: usize, pending: usize) -> usize {
        self.left(frame, per_frame);
        grant(pending, &mut self.left)
    }
}

/// Configured per-frame budget; `CENT_ANS_PERF_OFF=spawnbudget` lifts it (A/B of the old path).
pub fn per_frame(configured: usize) -> usize {
    if crate::campaign::perf_switch::off("spawnbudget") {
        usize::MAX
    } else {
        configured
    }
}

/// Grants `min(pending, *left)` spawns and deducts them from the frame's remaining budget.
pub fn grant(pending: usize, left: &mut usize) -> usize {
    let n = pending.min(*left);
    *left -= n;
    n
}

/// Number of frames needed to spawn `total` roots at `per_frame` (a zero budget counts as 1).
pub fn frames_needed(total: usize, per_frame: usize) -> usize {
    total.div_ceil(per_frame.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_never_exceeds_the_remaining_budget() {
        let mut left = 100;
        assert_eq!(grant(60, &mut left), 60);
        assert_eq!(left, 40);
        assert_eq!(grant(300, &mut left), 40);
        assert_eq!(left, 0);
        assert_eq!(grant(5, &mut left), 0);
    }

    #[test]
    fn a_burst_is_spread_over_frames_without_losing_roots() {
        let (mut pending, per_frame) = (1500, 150);
        let mut frames = 0;
        while pending > 0 {
            let mut left = per_frame;
            let n = grant(pending, &mut left);
            assert!(n <= per_frame);
            pending -= n;
            frames += 1;
        }
        assert_eq!(frames, frames_needed(1500, per_frame));
        assert_eq!(frames, 10);
    }

    #[test]
    fn the_budget_refills_each_frame() {
        let mut b = SpawnBudget::default();
        assert_eq!(b.take(1, 100, 80), 80);
        assert_eq!(b.take(1, 100, 80), 20);
        assert_eq!(b.take(1, 100, 80), 0);
        assert_eq!(b.take(2, 100, 80), 80);
    }

    #[test]
    fn two_spawners_share_one_budget() {
        let mut left = 150;
        let cell = grant(300, &mut left);
        let rocks = grant(4000, &mut left);
        assert_eq!(cell + rocks, 150);
    }
}
