//! Lot RJ-b: figure and regiment poses blended between the two latest
//! simulation steps.
//!
//! The battle moves by fixed 0.1 s steps; drawn as they are, the figures
//! stand still for five or six frames and then jump. The renderer keeps the
//! poses of the two latest steps built (`prev` at tick `N - k`, `cur` at tick
//! `N`) and draws them blended for the time `N - 1 + alpha` (one step behind
//! the simulation, `alpha` = `BattleSim::step_fraction`): continuous, even
//! when several steps fall in one frame (speed ×N, catch-up).
//!
//! No blend for a figure without a previous pose (new regiment, extra
//! figure), nor for one that jumps more than [`TELEPORT_M_PER_STEP`] per step
//! (legitimate teleport: deployment, replay jump).
//!
//! No Godot type here: plain Rust, testable with `cargo test`.

use std::f64::consts::{PI, TAU};

/// A figure pose from `soldier_poses`: `(x, y, z, angle)`.
pub(crate) type Pose = [f64; 4];

/// Beyond this distance per simulation step (50 m/s, five times a charging
/// horse), a figure or a regiment is drawn where it is, not slid there.
pub(crate) const TELEPORT_M_PER_STEP: f64 = 5.0;

/// Steps between two built poses beyond which they are not blended (replay
/// jump, long pause of the renderer): the latest stand.
pub(crate) const MAX_SPAN: u64 = 10;

/// Blend factor (0: `prev`, 1: `cur`) of the poses built at `prev_ticks` and
/// `cur_ticks` for a frame `alpha` (`step_fraction`) into the step after
/// `cur_ticks`: time shown `cur_ticks - 1 + alpha`.
pub(crate) fn blend_factor(prev_ticks: u64, cur_ticks: u64, alpha: f64) -> f64 {
    if cur_ticks <= prev_ticks || cur_ticks - prev_ticks > MAX_SPAN {
        return 1.0;
    }
    let span = (cur_ticks - prev_ticks) as f64;
    ((span - 1.0 + alpha.clamp(0.0, 1.0)) / span).clamp(0.0, 1.0)
}

/// `a` towards `b` by `t`, the short way round (radians).
pub(crate) fn lerp_angle(a: f64, b: f64, t: f64) -> f64 {
    let delta = (b - a + PI).rem_euclid(TAU) - PI;
    a + delta * t
}

/// `prev` towards `cur` by `t` (angle the short way round); `cur` itself when
/// the figure moved more than `teleport` metres on the ground.
pub(crate) fn blend_pose(prev: &Pose, cur: &Pose, t: f64, teleport: f64) -> Pose {
    let (dx, dz) = (cur[0] - prev[0], cur[2] - prev[2]);
    if t >= 1.0 || dx * dx + dz * dz > teleport * teleport {
        return *cur;
    }
    [
        prev[0] + dx * t,
        prev[1] + (cur[1] - prev[1]) * t,
        prev[2] + dz * t,
        lerp_angle(prev[3], cur[3], t),
    ]
}

/// PO4 "loose ranks": each figure drawn slightly off its place (position and
/// yaw), stable per (regiment, rank in the buffer). Rendering only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LooseRanks {
    /// Largest offset along x and along z (metres).
    pub(crate) offset_m: f64,
    /// Largest yaw offset (radians).
    pub(crate) yaw_rad: f64,
}

impl LooseRanks {
    /// `(dx, dz, dyaw)` of figure `index` of regiment `unit`, uniform in
    /// `[-offset, offset]² × [-yaw, yaw]`.
    pub(crate) fn jitter(&self, unit: u32, index: usize) -> (f64, f64, f64) {
        let mut state = (u64::from(unit) << 32) ^ index as u64 ^ 0x9E37_79B9_7F4A_7C15;
        let mut unit_float = || {
            state = splitmix64(state);
            (state >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
        };
        let dx = unit_float() * self.offset_m;
        let dz = unit_float() * self.offset_m;
        let dyaw = unit_float() * self.yaw_rad;
        (dx, dz, dyaw)
    }
}

fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Appends the `MultiMesh` transform (12 floats, rotation about Y) of a pose.
pub(crate) fn push_pose(buffer: &mut Vec<f32>, [x, y, z, angle]: Pose) {
    let (s, c) = (angle.sin() as f32, angle.cos() as f32);
    buffer.extend_from_slice(&[
        c, 0.0, s, x as f32, 0.0, 1.0, 0.0, y as f32, -s, 0.0, c, z as f32,
    ]);
}

/// The `MultiMesh` transforms of regiment `unit`'s figures: `cur` blended
/// from `prev` by `t` (figure by figure, by rank; ranks `prev` lacks are
/// drawn at `cur`), loosened when `loose` is set.
pub(crate) fn write_figures(
    out: &mut Vec<f32>,
    unit: u32,
    prev: Option<&[Pose]>,
    cur: &[Pose],
    t: f64,
    teleport: f64,
    loose: Option<LooseRanks>,
) {
    out.clear();
    out.reserve(cur.len() * 12);
    let prev = if t < 1.0 { prev.unwrap_or(&[]) } else { &[] };
    for (index, pose) in cur.iter().enumerate() {
        let mut pose = match prev.get(index) {
            Some(before) => blend_pose(before, pose, t, teleport),
            None => *pose,
        };
        if let Some(loose) = loose {
            let (dx, dz, dyaw) = loose.jitter(unit, index);
            pose[0] += dx;
            pose[2] += dz;
            pose[3] += dyaw;
        }
        push_pose(out, pose);
    }
}

/// Ground speed (m/s) of a regiment centre moved from `prev` to `cur` in
/// `span` steps of `dt` seconds; 0 without a previous position or after a
/// teleport.
pub(crate) fn ground_speed(prev: Option<&Pose>, cur: &Pose, span: u64, dt: f64) -> f64 {
    let Some(prev) = prev else {
        return 0.0;
    };
    if span == 0 || span > MAX_SPAN {
        return 0.0;
    }
    let distance = (cur[0] - prev[0]).hypot(cur[2] - prev[2]);
    if distance > TELEPORT_M_PER_STEP * span as f64 {
        return 0.0;
    }
    distance / (span as f64 * dt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_factor_shows_one_step_behind() {
        assert_eq!(blend_factor(4, 5, 0.0), 0.0);
        assert!((blend_factor(4, 5, 0.25) - 0.25).abs() < 1e-12);
        assert_eq!(blend_factor(4, 5, 1.0), 1.0);
        // Two steps in one frame: time shown 6 + alpha between ticks 5 and 7.
        assert!((blend_factor(5, 7, 0.5) - 0.75).abs() < 1e-12);
        // Nothing to blend from, or too far back.
        assert_eq!(blend_factor(5, 5, 0.3), 1.0);
        assert_eq!(blend_factor(7, 5, 0.3), 1.0);
        assert_eq!(blend_factor(0, MAX_SPAN + 1, 0.3), 1.0);
    }

    #[test]
    fn angles_blend_the_short_way() {
        let a = 179f64.to_radians();
        let b = -179f64.to_radians();
        let mid = lerp_angle(a, b, 0.5);
        assert!((mid.sin()).abs() < 1e-9 && mid.cos() < -0.999, "{mid}");
        assert!((lerp_angle(0.0, 1.0, 0.25) - 0.25).abs() < 1e-12);
        assert!((lerp_angle(0.1, 0.1 + TAU, 0.5) - 0.1).abs() < 1e-12);
    }

    #[test]
    fn poses_blend_unless_teleported() {
        let prev = [0.0, 1.0, 0.0, 0.0];
        let cur = [1.0, 3.0, 2.0, 0.5];
        let mid = blend_pose(&prev, &cur, 0.5, 5.0);
        assert_eq!(mid, [0.5, 2.0, 1.0, 0.25]);
        let far = [10.0, 0.0, 0.0, 0.0];
        assert_eq!(blend_pose(&prev, &far, 0.5, 5.0), far);
        assert_eq!(blend_pose(&prev, &cur, 1.0, 5.0), cur);
    }

    fn origin(buffer: &[f32], index: usize) -> (f32, f32) {
        (buffer[index * 12 + 3], buffer[index * 12 + 11])
    }

    #[test]
    fn new_figures_stand_at_their_current_pose() {
        let prev = [[0.0, 0.0, 0.0, 0.0]];
        let cur = [[1.0, 0.0, 0.0, 0.0], [5.0, 0.0, 5.0, 0.0]];
        let mut out = Vec::new();
        write_figures(&mut out, 3, Some(&prev), &cur, 0.5, 5.0, None);
        assert_eq!(out.len(), 24);
        assert_eq!(origin(&out, 0), (0.5, 0.0));
        assert_eq!(origin(&out, 1), (5.0, 5.0));
        // A new regiment: no previous poses at all.
        write_figures(&mut out, 3, None, &cur, 0.5, 5.0, None);
        assert_eq!(origin(&out, 0), (1.0, 0.0));
        // Fewer figures than before: the remaining ranks still blend.
        write_figures(&mut out, 3, Some(&cur), &prev, 0.5, 5.0, None);
        assert_eq!(out.len(), 12);
        assert_eq!(origin(&out, 0), (0.5, 0.0));
    }

    #[test]
    fn unblended_buffer_matches_the_plain_transforms() {
        let cur = [[1.0, 2.0, 3.0, 0.7], [4.0, 5.0, 6.0, -2.0]];
        let mut plain = Vec::new();
        for pose in cur {
            push_pose(&mut plain, pose);
        }
        let mut out = Vec::new();
        write_figures(&mut out, 0, None, &cur, 0.3, 5.0, None);
        assert_eq!(out, plain);
        write_figures(&mut out, 0, Some(&[[0.0; 4]; 2]), &cur, 1.0, 5.0, None);
        assert_eq!(out, plain);
    }

    #[test]
    fn loose_ranks_are_stable_and_bounded() {
        let loose = LooseRanks {
            offset_m: 0.15,
            yaw_rad: 4f64.to_radians(),
        };
        let a = loose.jitter(7, 12);
        assert_eq!(a, loose.jitter(7, 12));
        assert_ne!(a, loose.jitter(7, 13));
        assert_ne!(a, loose.jitter(8, 12));
        for i in 0..500 {
            let (dx, dz, dyaw) = loose.jitter(i % 9, i as usize);
            assert!(dx.abs() <= 0.15 && dz.abs() <= 0.15 && dyaw.abs() <= loose.yaw_rad);
        }
    }

    #[test]
    fn ground_speed_from_the_last_steps() {
        let prev = [0.0, 0.0, 0.0, 0.0];
        let cur = [0.3, 0.0, 0.4, 0.0];
        assert!((ground_speed(Some(&prev), &cur, 1, 0.1) - 5.0).abs() < 1e-9);
        assert!((ground_speed(Some(&prev), &cur, 2, 0.1) - 2.5).abs() < 1e-9);
        assert_eq!(ground_speed(None, &cur, 1, 0.1), 0.0);
        assert_eq!(
            ground_speed(Some(&prev), &[50.0, 0.0, 0.0, 0.0], 1, 0.1),
            0.0
        );
    }
}
