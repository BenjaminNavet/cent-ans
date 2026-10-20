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
    /// Index in `points` where the march stops this turn (the last point when it all fits).
    pub reach_split: usize,
    /// No route: a short dull stub towards the pointer instead of a march.
    pub blocked: bool,
}

/// Lift above the ground for a camera distance.
pub fn lift_at(camera_distance: f32) -> f32 {
    (camera_distance * LIFT_PER_DISTANCE).clamp(MIN_LIFT, MAX_LIFT)
}

/// Subdivides `waypoints` (map pixels) so that the ribbon follows the ground.
pub fn subdivide(waypoints: &[Vec2], camera_distance: f32) -> Vec<Vec2> {
    subdivide_indexed(waypoints, camera_distance).0
}

/// `subdivide` plus, for each waypoint, its index in the subdivided polyline.
pub fn subdivide_indexed(waypoints: &[Vec2], camera_distance: f32) -> (Vec<Vec2>, Vec<usize>) {
    let Some(first) = waypoints.first() else {
        return (Vec::new(), Vec::new());
    };
    let length: f32 = waypoints.windows(2).map(|w| w[0].distance(w[1])).sum();
    let step = (camera_distance * STEP_PER_DISTANCE)
        .clamp(MIN_STEP, SUBDIVISION_PX)
        .max(length / MAX_POINTS as f32);
    let mut out = vec![*first];
    let mut corner_at = vec![0];
    for pair in waypoints.windows(2) {
        let steps = ((pair[0].distance(pair[1]) / step) as usize).max(1);
        for k in 1..=steps {
            out.push(pair[0].lerp(pair[1], k as f32 / steps as f32));
        }
        corner_at.push(out.len() - 1);
    }
    (out, corner_at)
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
    let (subdivided, corner_at) = subdivide_indexed(waypoints, camera_distance);
    let points: Vec<Vec3> = subdivided.into_iter().map(lifted).collect();
    let reach_split = turn_ends
        .first()
        .and_then(|i| corner_at.get(*i))
        .copied()
        .unwrap_or(points.len().saturating_sub(1));
    PathView {
        reach_split,
        points,
        corners: waypoints.iter().copied().map(lifted).collect(),
        turn_ends,
        ..PathView::default()
    }
}

/// Dull stub of at most `max_px` from `from` towards `to` (no route to the pointer).
pub fn blocked_view(
    from: Vec2,
    to: Vec2,
    max_px: f32,
    ground: &impl Ground,
    camera_distance: f32,
) -> PathView {
    let reach = (to - from).clamp_length_max(max_px);
    PathView {
        blocked: true,
        ..view_from_waypoints(&[from, from + reach], Vec::new(), ground, camera_distance)
    }
}

/// Style of the march line, `path` block of `data/map/stance_cues.json`.
#[derive(Resource, Debug, Clone)]
pub struct PathStyle {
    pub color: [f32; 3],
    pub blocked_color: [f32; 3],
    /// Alpha of the dashes and of the continuous glow under them, this turn / beyond.
    pub reach_alpha: f32,
    pub beyond_alpha: f32,
    pub reach_glow: f32,
    pub beyond_glow: f32,
    pub blocked_alpha: f32,
    /// Dash length, gap (in dash lengths) and speed, per unit of camera distance.
    pub dash_per_distance: f32,
    pub gap_ratio: f32,
    pub speed_per_distance: f32,
    pub min_dash: f32,
    pub max_dash: f32,
    pub blocked_max_px: f32,
}

impl Default for PathStyle {
    fn default() -> Self {
        Self::parse("{}")
    }
}

impl PathStyle {
    pub fn parse(text: &str) -> Self {
        let value: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
        let path = &value["path"];
        let f = |key: &str, old: f32| path[key].as_f64().map_or(old, |v| v as f32);
        let color =
            |key: &str, old: &str| super::views::parse_hex_color(path[key].as_str().unwrap_or(old));
        Self {
            color: color("color", "#f08a2a"),
            blocked_color: color("blocked_color", "#8a2a22"),
            reach_alpha: f("reach_alpha", 1.0),
            beyond_alpha: f("beyond_alpha", 0.35),
            reach_glow: f("reach_glow", 0.4),
            beyond_glow: f("beyond_glow", 0.12),
            blocked_alpha: f("blocked_alpha", 0.55),
            dash_per_distance: f("dash_per_distance", 0.03),
            gap_ratio: f("gap_ratio", 0.7).max(0.0),
            speed_per_distance: f("speed_per_distance", 0.05),
            min_dash: f("min_dash", 0.4),
            max_dash: f("max_dash", 14.0),
            blocked_max_px: f("blocked_max_px", 90.0),
        }
    }

    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/map/stance_cues.json")).ok())
            .map_or_else(Self::default, |text| Self::parse(&text))
    }

    /// Dash length and period for a camera distance.
    pub fn dash(&self, camera_distance: f32) -> (f32, f32) {
        let dash = (camera_distance * self.dash_per_distance).clamp(self.min_dash, self.max_dash);
        (dash, dash * (1.0 + self.gap_ratio))
    }
}

/// A piece of the dashed march line.
#[derive(Debug, Clone, PartialEq)]
pub struct DashPiece {
    pub points: Vec<Vec3>,
    /// Lies in the part reachable this turn.
    pub reachable: bool,
}

/// Point of the polyline at arc length `s` (clamped), with the index of the segment it lies on.
fn point_at(points: &[Vec3], cum: &[f32], s: f32) -> (Vec3, usize) {
    let last = points.len() - 1;
    if s <= 0.0 {
        return (points[0], 0);
    }
    if s >= cum[last] {
        return (points[last], last.saturating_sub(1));
    }
    let seg = cum
        .partition_point(|c| *c <= s)
        .saturating_sub(1)
        .min(last - 1);
    let span = (cum[seg + 1] - cum[seg]).max(1e-9);
    (
        points[seg].lerp(points[seg + 1], (s - cum[seg]) / span),
        seg,
    )
}

/// Splits the polyline into moving dashes of length `dash` every `period`, shifted by `phase`
/// (the dashes advance towards the end as `phase` grows). Each dash keeps the polyline's own
/// vertices so that it hugs the ground, and is cut at `reach_split` so that it belongs to one
/// part only.
pub fn dash_pieces(
    points: &[Vec3],
    reach_split: usize,
    dash: f32,
    period: f32,
    phase: f32,
) -> Vec<DashPiece> {
    if points.len() < 2 || dash <= 0.0 || period < dash {
        return Vec::new();
    }
    let mut cum = vec![0.0f32];
    for pair in points.windows(2) {
        cum.push(cum.last().copied().unwrap_or(0.0) + pair[0].distance(pair[1]));
    }
    let total = cum[cum.len() - 1];
    let split = cum[reach_split.min(points.len() - 1)];
    let mut out = Vec::new();
    let mut start = phase.rem_euclid(period) - period;
    while start < total {
        let (a, b) = (start.max(0.0), (start + dash).min(total));
        start += period;
        if b - a < dash * 0.05 {
            continue;
        }
        for (lo, hi, reachable) in [(a, b.min(split), true), (a.max(split), b, false)] {
            if hi - lo < dash * 0.05 {
                continue;
            }
            let (first, seg_a) = point_at(points, &cum, lo);
            let (end, seg_b) = point_at(points, &cum, hi);
            let mut piece = vec![first];
            piece.extend(
                points
                    .iter()
                    .enumerate()
                    .skip(seg_a + 1)
                    .take_while(|(i, _)| *i <= seg_b)
                    .filter(|(i, _)| cum[*i] > lo && cum[*i] < hi)
                    .map(|(_, p)| *p),
            );
            piece.push(end);
            out.push(DashPiece {
                points: piece,
                reachable,
            });
        }
    }
    out
}

/// Draws the march line: a faint continuous glow and moving dashes, vivid this turn and pale
/// beyond; dull and red when there is no route.
pub fn draw_path(
    gizmos: &mut Gizmos,
    view: &PathView,
    style: &PathStyle,
    override_color: Option<Color>,
    time_s: f32,
    camera_distance: f32,
) -> usize {
    let (dash, period) = style.dash(camera_distance);
    let phase = time_s * style.speed_per_distance * camera_distance.max(1.0);
    let rgb = if view.blocked {
        style.blocked_color
    } else {
        style.color
    };
    let tint = |alpha: f32| match override_color {
        Some(c) => c.with_alpha(alpha),
        None => Color::srgba(rgb[0], rgb[1], rgb[2], alpha),
    };
    let split = view.reach_split.min(view.points.len().saturating_sub(1));
    if view.blocked {
        gizmos.linestrip(view.points.iter().copied(), tint(style.blocked_alpha * 0.5));
    } else {
        gizmos.linestrip(
            view.points[..=split].iter().copied(),
            tint(style.reach_glow),
        );
        gizmos.linestrip(
            view.points[split..].iter().copied(),
            tint(style.beyond_glow),
        );
    }
    let pieces = dash_pieces(&view.points, split, dash, period, phase);
    for piece in &pieces {
        let alpha = if view.blocked {
            style.blocked_alpha
        } else if piece.reachable {
            style.reach_alpha
        } else {
            style.beyond_alpha
        };
        gizmos.linestrip(piece.points.iter().copied(), tint(alpha));
    }
    pieces.len()
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

    fn line(n: usize) -> Vec<Vec3> {
        (0..=n).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect()
    }

    #[test]
    fn reach_split_follows_the_first_turn_end() {
        let ground = HeightSampler::flat();
        let p = plan(vec![[0.0, 0.0], [100.0, 0.0], [200.0, 0.0]], vec![1, 2]);
        let view = preview_from_plan(&p, &ground, 100.0).unwrap();
        // The split sits on the first corner, the polyline goes on beyond it.
        assert!((view.points[view.reach_split].x - 100.0).abs() < 1e-3);
        assert!(view.reach_split < view.points.len() - 1);
        // When all fits this turn, the whole line is reachable.
        let p = plan(vec![[0.0, 0.0], [100.0, 0.0]], vec![1]);
        let view = preview_from_plan(&p, &ground, 100.0).unwrap();
        assert_eq!(view.reach_split, view.points.len() - 1);
    }

    #[test]
    fn dashes_are_cut_at_the_split_and_advance() {
        let pts = line(20);
        let pieces = dash_pieces(&pts, 10, 2.0, 4.0, 0.0);
        assert!(pieces.iter().all(|p| p.points.len() >= 2));
        for piece in &pieces {
            let (lo, hi) = (piece.points[0].x, piece.points.last().unwrap().x);
            if piece.reachable {
                assert!(hi <= 10.0 + 1e-4, "{lo}..{hi}");
            } else {
                assert!(lo >= 10.0 - 1e-4, "{lo}..{hi}");
            }
        }
        assert!(pieces.iter().any(|p| p.reachable) && pieces.iter().any(|p| !p.reachable));
        // A dash that straddles the split gives one piece on each side.
        let straddle = dash_pieces(&pts, 10, 4.0, 40.0, 8.0);
        assert_eq!(straddle.len(), 2);
        // Shifting the phase moves the dashes towards the destination.
        let a = dash_pieces(&pts, 20, 2.0, 4.0, 0.5);
        let b = dash_pieces(&pts, 20, 2.0, 4.0, 1.0);
        let first = |v: &[DashPiece]| v.iter().map(|p| p.points[0].x).fold(f32::MAX, f32::min);
        let _ = (first(&a), first(&b));
        assert!(a[1].points[0].x < b[1].points[0].x);
    }

    #[test]
    fn dashes_hug_the_ground() {
        // A hill: the line is lifted per vertex; a dash keeps the vertices inside it.
        let pts: Vec<Vec3> = (0..=20)
            .map(|i| Vec3::new(i as f32, (i as f32 * 0.5).sin() * 3.0, 0.0))
            .collect();
        let pieces = dash_pieces(&pts, 20, 6.0, 8.0, 0.0);
        for piece in &pieces {
            for p in &piece.points {
                let expected = (p.x * 0.5).sin() * 3.0;
                // Between vertices the chord stays within the curve's sag.
                assert!((p.y - expected).abs() < 0.25, "{p:?} vs {expected}");
            }
            assert!(piece.points.len() >= 3, "interior vertices kept");
        }
    }

    #[test]
    fn blocked_stub_is_short_and_flagged() {
        let ground = HeightSampler::new(|x, _| x * 0.1);
        let view = blocked_view(Vec2::ZERO, Vec2::new(1000.0, 0.0), 90.0, &ground, 100.0);
        assert!(view.blocked);
        let end = view.points.last().unwrap();
        assert!((end.x - 90.0).abs() < 1e-3);
        assert!((end.y - (9.0 + lift_at(100.0))).abs() < 1e-3);
    }

    #[test]
    fn style_reads_the_cues_block() {
        let s = PathStyle::parse(r##"{"path":{"color":"#ff0000","beyond_alpha":0.2}}"##);
        assert_eq!(s.color, [1.0, 0.0, 0.0]);
        assert!((s.beyond_alpha - 0.2).abs() < 1e-6);
        let (dash, period) = PathStyle::default().dash(100.0);
        assert!(period > dash);
    }
}
