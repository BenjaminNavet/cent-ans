//! Path preview geometry: `PathPlan` (map pixels) to a world polyline (port of `path_preview.gd`).

use bevy::prelude::*;
use sim_campaign::PathPlan;

use super::projection::{Ground, px_to_world};

pub const SUBDIVISION_PX: f32 = 12.0;
pub const STEP_PER_DISTANCE: f32 = 0.04;
pub const MIN_STEP: f32 = 0.04;
pub const MAX_POINTS: usize = 2000;
pub const LIFT_PER_DISTANCE: f32 = 0.004;
pub const MIN_LIFT: f32 = 0.002;
pub const MAX_LIFT: f32 = 0.6;

/// A previewed march, ready to draw.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PathView {
    /// Subdivided polyline in world space, lifted above the ground.
    pub points: Vec<Vec3>,
    /// Corners of the plan (not subdivided), world space.
    pub corners: Vec<Vec3>,
    /// Indices into `corners` where each turn ends.
    pub turn_ends: Vec<usize>,
    pub turns: usize,
    pub cost: u32,
    pub cost_this_turn: u32,
    pub reachable_this_turn: bool,
}

/// Lift above the ground for a camera distance.
pub fn lift_at(camera_distance: f32) -> f32 {
    (camera_distance * LIFT_PER_DISTANCE).clamp(MIN_LIFT, MAX_LIFT)
}

/// Subdivides `waypoints` (map pixels) so that the ribbon follows the ground.
pub fn subdivide(waypoints: &[Vec2], camera_distance: f32) -> Vec<Vec2> {
    let Some(first) = waypoints.first() else {
        return Vec::new();
    };
    let length: f32 = waypoints.windows(2).map(|w| w[0].distance(w[1])).sum();
    let step = (camera_distance * STEP_PER_DISTANCE)
        .clamp(MIN_STEP, SUBDIVISION_PX)
        .max(length / MAX_POINTS as f32);
    let mut out = vec![*first];
    for pair in waypoints.windows(2) {
        let steps = ((pair[0].distance(pair[1]) / step) as usize).max(1);
        for k in 1..=steps {
            out.push(pair[0].lerp(pair[1], k as f32 / steps as f32));
        }
    }
    out
}

/// A drawable polyline of map-pixel waypoints (a sea route has no turn ends).
pub fn view_from_waypoints(
    waypoints: &[Vec2],
    turn_ends: Vec<usize>,
    ground: &impl Ground,
    camera_distance: f32,
) -> PathView {
    let lift = lift_at(camera_distance);
    let lifted = |p: Vec2| px_to_world(p, ground) + Vec3::Y * lift;
    PathView {
        points: subdivide(waypoints, camera_distance)
            .into_iter()
            .map(lifted)
            .collect(),
        corners: waypoints.iter().copied().map(lifted).collect(),
        turn_ends,
        ..PathView::default()
    }
}

/// Converts a plan to a drawable view. `None` when the plan has nothing to walk.
pub fn preview_from_plan(
    plan: &PathPlan,
    ground: &impl Ground,
    camera_distance: f32,
) -> Option<PathView> {
    if plan.points.len() < 2 {
        return None;
    }
    let waypoints: Vec<Vec2> = plan.points.iter().map(|p| Vec2::new(p[0], p[1])).collect();
    Some(PathView {
        turns: plan.turns(),
        cost: plan.cost,
        cost_this_turn: plan.cost_this_turn,
        reachable_this_turn: plan.reachable_this_turn(),
        ..view_from_waypoints(&waypoints, plan.turn_ends.clone(), ground, camera_distance)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::armies::projection::HeightSampler;
    use sim_campaign::terrain_mods::CostBreakdown;

    fn plan(points: Vec<[f32; 2]>, turn_ends: Vec<usize>) -> PathPlan {
        PathPlan {
            points,
            turn_ends,
            cost: 40,
            cost_this_turn: 20,
            breakdown: CostBreakdown::default(),
            general_percent: 0.0,
        }
    }

    #[test]
    fn plan_converts_to_world_polyline() {
        let ground = HeightSampler::flat();
        let p = plan(vec![[10.0, 20.0], [10.0, 80.0], [50.0, 80.0]], vec![1, 2]);
        let view = preview_from_plan(&p, &ground, 100.0).unwrap();
        assert_eq!(view.corners.len(), 3);
        // map pixel (x, y) -> world (x, lift, y)
        assert!((view.corners[0].x - 10.0).abs() < 1e-5);
        assert!((view.corners[0].z - 20.0).abs() < 1e-5);
        assert!((view.corners[0].y - lift_at(100.0)).abs() < 1e-5);
        assert_eq!(view.turns, 2);
        assert!(!view.reachable_this_turn);
        // subdivided polyline keeps both ends and is denser than the corners
        assert!(view.points.len() > view.corners.len());
        assert_eq!(view.points.first(), view.corners.first());
        assert_eq!(view.points.last(), view.corners.last());
    }

    #[test]
    fn single_point_plan_is_nothing() {
        let p = plan(vec![[1.0, 1.0]], vec![]);
        assert!(preview_from_plan(&p, &HeightSampler::flat(), 10.0).is_none());
    }

    #[test]
    fn height_sampler_lifts_the_polyline() {
        let ground = HeightSampler::new(|x, _| x * 0.1);
        let p = plan(vec![[0.0, 0.0], [100.0, 0.0]], vec![1]);
        let view = preview_from_plan(&p, &ground, 100.0).unwrap();
        let last = view.points.last().unwrap();
        assert!((last.y - (10.0 + lift_at(100.0))).abs() < 1e-4);
        assert!(view.reachable_this_turn);
    }
}
