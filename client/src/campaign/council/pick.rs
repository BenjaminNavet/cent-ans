//! Seat layout and picking of the council room (ports the constants and `pick_ray` of
//! `game/scripts/ui/council_room.gd`). Pure geometry, no ECS: unit-tested.

use bevy::prelude::*;

/// Offices in seating order, left to right behind the table (`SEAT_OFFICES`).
pub const SEAT_OFFICES: [&str; 5] = [
    "chancellor",
    "constable",
    "treasurer",
    "chaplain",
    "chamberlain",
];
/// Seat of each office, in the order of [`SEAT_OFFICES`] (`SEAT_POSITIONS`).
pub const SEAT_POSITIONS: [Vec3; 5] = [
    Vec3::new(-0.85, 0.0, -0.8),
    Vec3::new(0.85, 0.0, -0.8),
    Vec3::new(-1.7, 0.0, -0.8),
    Vec3::new(1.7, 0.0, -0.8),
    Vec3::new(-2.55, 0.0, -0.8),
];
pub const RULER_POSITION: Vec3 = Vec3::new(0.0, 0.0, -1.0);
/// Centre of the table and of the camera (the row is longer on the left).
pub const ROW_CENTER_X: f32 = -0.4;
pub const CAMERA_POSITION: Vec3 = Vec3::new(ROW_CENTER_X, 1.5, 3.6);
pub const CAMERA_TARGET: Vec3 = Vec3::new(ROW_CENTER_X, 1.0, -0.8);
/// Vertical field of view of the room camera (degrees).
pub const CAMERA_FOV_DEG: f32 = 52.0;
pub const HIT_RADIUS: f32 = 0.35;

/// Seat keys: `ruler` then the offices.
pub fn seat_keys() -> impl Iterator<Item = &'static str> {
    std::iter::once("ruler").chain(SEAT_OFFICES)
}

pub fn seat_position(key: &str) -> Vec3 {
    if key == "ruler" {
        return RULER_POSITION;
    }
    SEAT_OFFICES
        .iter()
        .position(|office| *office == key)
        .map_or(Vec3::ZERO, |index| SEAT_POSITIONS[index])
}

/// Yaw so that the front (-Z) of a figure at `from` looks at `to` (models face -Z).
pub fn facing(from: Vec3, to: Vec3) -> f32 {
    (-(to.x - from.x)).atan2(-(to.z - from.z))
}

/// World-space direction of the ray through `uv` (0..1 from the top-left of the stage image).
pub fn ray_direction(uv: Vec2, fov_y_deg: f32, aspect: f32, rotation: Quat) -> Vec3 {
    let half = (fov_y_deg.to_radians() * 0.5).tan();
    let ndc = Vec2::new(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    (rotation * Vec3::new(ndc.x * half * aspect, ndc.y * half, -1.0)).normalize()
}

/// Seat hit by the ray (vacant seats included), the one closest to the ray axis; `None` if none.
pub fn pick_ray(origin: Vec3, direction: Vec3) -> Option<&'static str> {
    let ray = direction.normalize();
    let mut best: Option<&'static str> = None;
    let mut best_miss = f32::INFINITY;
    for key in seat_keys() {
        let base = seat_position(key);
        for height in [0.5_f32, 0.9, 1.2] {
            let to_center = base + Vec3::Y * height - origin;
            let along = to_center.dot(ray);
            if along <= 0.0 {
                continue;
            }
            let miss = (to_center - ray * along).length();
            let radius = HIT_RADIUS * if height > 1.2 { 0.7 } else { 1.0 };
            if miss < radius && miss < best_miss {
                best_miss = miss;
                best = Some(key);
            }
        }
    }
    best
}

/// Projects a world point to stage uv (0..1, top-left origin); `None` behind the camera.
pub fn project(point: Vec3, eye: Vec3, rotation: Quat, fov_y_deg: f32, aspect: f32) -> Option<Vec2> {
    let local = rotation.inverse() * (point - eye);
    if local.z >= -0.01 {
        return None;
    }
    let half = (fov_y_deg.to_radians() * 0.5).tan();
    let ndc = Vec2::new(local.x / (-local.z * half * aspect), local.y / (-local.z * half));
    Some(Vec2::new((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera_rotation() -> Quat {
        Transform::from_translation(CAMERA_POSITION)
            .looking_at(CAMERA_TARGET, Vec3::Y)
            .rotation
    }

    #[test]
    fn every_seat_is_picked_through_its_own_projection() {
        let rotation = camera_rotation();
        let aspect = 760.0 / 480.0;
        for key in seat_keys() {
            let point = seat_position(key) + Vec3::Y * 0.9;
            let uv = project(point, CAMERA_POSITION, rotation, CAMERA_FOV_DEG, aspect).unwrap();
            assert!((0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y), "{key} {uv}");
            let direction = ray_direction(uv, CAMERA_FOV_DEG, aspect, rotation);
            assert_eq!(pick_ray(CAMERA_POSITION, direction), Some(key));
        }
    }

    #[test]
    fn empty_wall_picks_nothing() {
        let direction = ray_direction(Vec2::new(0.02, 0.02), CAMERA_FOV_DEG, 1.58, camera_rotation());
        assert_eq!(pick_ray(CAMERA_POSITION, direction), None);
    }

    #[test]
    fn facing_matches_godot_convention() {
        // A seat looking at the camera (+Z) turns half a turn from the default -Z front.
        let yaw = facing(Vec3::new(0.0, 0.0, -0.8), Vec3::new(0.0, 0.0, 4.0));
        assert!((yaw.abs() - std::f32::consts::PI).abs() < 1e-5);
    }
}
