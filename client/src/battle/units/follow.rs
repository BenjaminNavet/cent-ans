//! Per-figure followers: each drawn man walks to his place in the regiment instead of being glued
//! to it, so a regiment that turns bends and trails, men face the way they walk and the block
//! never looks like one rigid rectangle. Render only: the simulation (rectangle, facing, slots)
//! is untouched and the followers snap onto the slots whenever a figure is far from its place.

use std::collections::HashMap;

use super::Pose;

/// Fastest walk of a figure towards its place (m/s): a run, so a marching line keeps up.
const MAX_SPEED: f32 = 5.5;
/// Stiffness of the pull towards the place (1/s): the lag of a figure is its speed over this.
const PULL: f32 = 5.0;
/// Further than this from its place a figure is placed there (deployment, teleport, new count).
const SNAP_DISTANCE: f32 = 14.0;
/// Height difference that snaps a figure (stairs, ladders, wall walks).
const SNAP_HEIGHT: f32 = 1.2;
/// Below this distance to the place a figure keeps the facing of its regiment (m).
const WALK_FROM: f32 = 0.25;
/// Beyond `WALK_FROM` plus this, a figure faces its heading entirely (m).
const WALK_BLEND: f32 = 0.9;
/// Fastest turn of a figure (rad/s).
const TURN_RATE: f32 = 6.0;

/// Smoothed poses by regiment id.
#[derive(Default)]
pub struct Followers {
    poses: HashMap<u32, Vec<Pose>>,
}

fn wrap(angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (angle + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}

impl Followers {
    /// Moves the followers of regiment `id` by `dt` seconds towards `target` and returns where
    /// they stand. `rigid` regiments (on a wall, up a ladder) are shown on their places.
    pub fn follow(&mut self, id: u32, target: &[Pose], dt: f32, rigid: bool) -> Vec<Pose> {
        if rigid || dt <= 0.0 {
            self.poses.insert(id, target.to_vec());
            return target.to_vec();
        }
        let old = self.poses.remove(&id);
        let shown: Vec<Pose> = target
            .iter()
            .enumerate()
            .map(|(i, goal)| match old.as_ref().and_then(|o| o.get(i)) {
                Some(here) => step(here, goal, dt),
                None => *goal,
            })
            .collect();
        self.poses.insert(id, shown.clone());
        shown
    }

    pub fn clear(&mut self) {
        self.poses.clear();
    }
}

fn step(here: &Pose, goal: &Pose, dt: f32) -> Pose {
    let (dx, dz) = (goal[0] - here[0], goal[2] - here[2]);
    let distance = dx.hypot(dz);
    if distance > SNAP_DISTANCE || (goal[1] - here[1]).abs() > SNAP_HEIGHT {
        return *goal;
    }
    let travel = (distance * (1.0 - (-PULL * dt).exp())).min(MAX_SPEED * dt);
    let (x, z) = if distance > 1e-4 {
        (
            here[0] + dx / distance * travel,
            here[2] + dz / distance * travel,
        )
    } else {
        (goal[0], goal[2])
    };
    // Facing: the regiment's, turned towards the heading the further the figure is from his place.
    let walking = ((distance - WALK_FROM) / WALK_BLEND).clamp(0.0, 1.0);
    let heading = dx.atan2(dz);
    let wanted = goal[3] + wrap(heading - goal[3]) * walking;
    let turn = (TURN_RATE * dt).max(0.0);
    let facing = here[3] + wrap(wanted - here[3]).clamp(-turn, turn);
    [x, goal[1], z, facing]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_figure_far_from_its_place_snaps() {
        let mut followers = Followers::default();
        followers.follow(1, &[[0.0, 0.0, 0.0, 0.0]], 0.1, false);
        let shown = followers.follow(1, &[[50.0, 0.0, 0.0, 0.0]], 0.1, false);
        assert_eq!(shown[0], [50.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_figure_trails_then_catches_up() {
        let mut followers = Followers::default();
        followers.follow(1, &[[0.0, 0.0, 0.0, 0.0]], 0.1, false);
        let first = followers.follow(1, &[[3.0, 0.0, 0.0, 0.0]], 0.1, false)[0];
        assert!(first[0] > 0.0 && first[0] < 3.0, "trails: {first:?}");
        let mut last = first;
        for _ in 0..60 {
            last = followers.follow(1, &[[3.0, 0.0, 0.0, 0.0]], 0.1, false)[0];
        }
        assert!((last[0] - 3.0).abs() < 0.05, "arrives: {last:?}");
    }

    #[test]
    fn a_walking_figure_faces_its_heading_and_a_resting_one_its_regiment() {
        let mut followers = Followers::default();
        followers.follow(1, &[[0.0, 0.0, 0.0, 0.0]], 0.1, false);
        let mut walking = [0.0; 4];
        for _ in 0..30 {
            walking = followers.follow(1, &[[0.0, 0.0, 6.0, 0.0]], 0.1, false)[0];
        }
        assert!(
            walking[3].abs() < 0.2,
            "heading +z is facing 0: {walking:?}"
        );
        let mut turned = [0.0; 4];
        for _ in 0..50 {
            turned = followers.follow(2, &[[5.0, 0.0, 5.0, 1.0]], 0.1, false)[0];
        }
        assert!(
            (turned[3] - 1.0).abs() < 0.1,
            "rests on regiment facing: {turned:?}"
        );
    }

    #[test]
    fn rigid_regiments_are_shown_on_their_places() {
        let mut followers = Followers::default();
        followers.follow(1, &[[0.0, 0.0, 0.0, 0.0]], 0.1, true);
        let shown = followers.follow(1, &[[2.0, 1.0, 0.0, 0.5]], 0.1, true);
        assert_eq!(shown[0], [2.0, 1.0, 0.0, 0.5]);
    }
}
