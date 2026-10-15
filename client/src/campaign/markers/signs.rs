//! Constant-screen-size signs (ruins, worksites). Port of `screen_signs.gd`, `ruin_markers.gd`,
//! `construction_markers.gd`: a sign keeps its on-screen size, so its world scale follows the
//! distance to the camera.

use bevy::prelude::*;

/// Ruin icon width as a fraction of the camera distance (`RuinMarkers.screen_fraction`).
pub const RUIN_SCREEN_FRACTION: f32 = 0.035;
/// Height above the ground of a ruin sign (`RuinMarkers.lift`).
pub const RUIN_LIFT: f32 = 5.0;
/// Worksite width as a fraction of the camera distance (`ConstructionMarkers.screen_fraction`).
pub const WORKSITE_SCREEN_FRACTION: f32 = 0.04;
/// Native width (m) of the worksite model brought back to `screen_fraction`.
pub const WORKSITE_SPAN_M: f32 = 34.0;
/// Presentation yaw of the worksite scaffold (rad).
pub const WORKSITE_YAW: f32 = 0.6;
pub const WORKSITE_MODEL: &str = "models/outbuildings/worksite_1.glb";

/// World width of a ruin sign seen from `distance`.
pub fn ruin_width(distance: f32) -> f32 {
    distance.max(0.01) * RUIN_SCREEN_FRACTION
}

/// Scale (world units per model metre) of a worksite seen from `distance`.
pub fn worksite_scale(distance: f32) -> f32 {
    distance.max(0.01) * WORKSITE_SCREEN_FRACTION / WORKSITE_SPAN_M
}

#[derive(Component)]
pub struct RuinSign;

#[derive(Component)]
pub struct WorksiteSign;

/// Rescales every sign from its distance to the camera; ruins also face the camera.
pub fn rescale_signs(
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut ruins: Query<(&mut Transform, &GlobalTransform), (With<RuinSign>, Without<Camera3d>)>,
    mut worksites: Query<
        (&mut Transform, &GlobalTransform),
        (With<WorksiteSign>, Without<RuinSign>, Without<Camera3d>),
    >,
) {
    let Some(cam) = camera.iter().next() else {
        return;
    };
    let eye = cam.translation();
    let rotation = cam.rotation();
    for (mut t, g) in &mut ruins {
        t.scale = Vec3::splat(ruin_width(eye.distance(g.translation())));
        t.rotation = rotation;
    }
    for (mut t, g) in &mut worksites {
        t.scale = Vec3::splat(worksite_scale(eye.distance(g.translation())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_is_proportional_to_distance() {
        assert!((ruin_width(100.0) - 3.5).abs() < 1e-6);
        assert!((ruin_width(200.0) - 2.0 * ruin_width(100.0)).abs() < 1e-6);
        assert!(ruin_width(0.0) > 0.0);
        let s = worksite_scale(340.0);
        assert!((s - 0.4).abs() < 1e-6);
        // The projected width (scale * span) over distance is constant.
        assert!(
            (worksite_scale(50.0) * WORKSITE_SPAN_M / 50.0 - WORKSITE_SCREEN_FRACTION).abs() < 1e-6
        );
    }
}
