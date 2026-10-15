//! Relief exaggeration and close-camera profiles: ports of `relief_exaggeration_profile.gd` and
//! `close_camera_profile.gd`, read from `game/resources/*.tres`. Rendering only.

use super::tres::Tres;
use std::path::Path;

/// Vertical scale of the strategic view: `MapData.HEIGHT_SCALE` (world units per metre).
pub const HEIGHT_SCALE: f32 = 0.006;

/// `ReliefExaggerationProfile` (RE, ADR 0353).
#[derive(Debug, Clone)]
pub struct ReliefProfile {
    pub enabled: bool,
    pub near_exaggeration: f32,
    pub gain_far: f32,
    pub gain_near: f32,
    pub floor_cell_px: i32,
    pub floor_sample_step: i32,
    pub floor_min_radius: i32,
    pub floor_blur_radius: i32,
    pub floor_blur_passes: i32,
    pub local_relief_cap_m: f32,
    pub mountain_knee_m: f32,
    pub mountain_ratio: f32,
    pub mountain_squash_far: f32,
    pub mountain_squash_full_exaggeration: f32,
    pub mountain_squash_max: f32,
    pub mountain_lift: f32,
    pub mountain_lift_start_m: f32,
    pub mountain_lift_full_m: f32,
    pub mountain_lift_power: f32,
    pub mountain_lift_max: f32,
    pub massif_lift: f32,
    pub massif_smooth_radius: i32,
    pub massif_coast_fade_cells: i32,
    pub plain_flatten: f32,
    pub plain_flatten_full_m: f32,
    pub plain_flatten_end_m: f32,
    pub sun_elevation_deg: f32,
}

impl ReliefProfile {
    /// Script defaults overridden by the `.tres` values.
    pub fn from_tres(t: &Tres) -> ReliefProfile {
        ReliefProfile {
            enabled: t.bool("enabled", true),
            near_exaggeration: t.f32("near_exaggeration", 2.5),
            gain_far: t.f32("gain_far", 0.3),
            gain_near: t.f32("gain_near", 0.8),
            floor_cell_px: t.i32("floor_cell_px", 8),
            floor_sample_step: t.i32("floor_sample_step", 2),
            floor_min_radius: t.i32("floor_min_radius", 1),
            floor_blur_radius: t.i32("floor_blur_radius", 2),
            floor_blur_passes: t.i32("floor_blur_passes", 2),
            local_relief_cap_m: t.f32("local_relief_cap_m", 350.0),
            mountain_knee_m: t.f32("mountain_knee_m", 350.0),
            mountain_ratio: t.f32("mountain_ratio", 0.3),
            mountain_squash_far: t.f32("mountain_squash_far", 0.0),
            mountain_squash_full_exaggeration: t.f32("mountain_squash_full_exaggeration", 3.5),
            mountain_squash_max: t.f32("mountain_squash_max", 0.55),
            mountain_lift: t.f32("mountain_lift", 0.0),
            mountain_lift_start_m: t.f32("mountain_lift_start_m", 500.0),
            mountain_lift_full_m: t.f32("mountain_lift_full_m", 1600.0),
            mountain_lift_power: t.f32("mountain_lift_power", 0.0),
            mountain_lift_max: t.f32("mountain_lift_max", 6.0),
            massif_lift: t.f32("massif_lift", 0.0),
            massif_smooth_radius: t.i32("massif_smooth_radius", 1),
            massif_coast_fade_cells: t.i32("massif_coast_fade_cells", 2),
            plain_flatten: t.f32("plain_flatten", 0.0),
            plain_flatten_full_m: t.f32("plain_flatten_full_m", 120.0),
            plain_flatten_end_m: t.f32("plain_flatten_end_m", 350.0),
            sun_elevation_deg: t.f32("sun_elevation_deg", 34.0),
        }
    }

    /// `game/resources/relief_exaggeration.tres` under `root`.
    pub fn load(root: &Path) -> ReliefProfile {
        Self::from_tres(&Tres::load(
            &root.join("game/resources/relief_exaggeration.tres"),
        ))
    }

    /// `near_floor`: effective near exaggeration.
    pub fn near_floor(&self, zg4_near: f32) -> f32 {
        if self.enabled {
            self.near_exaggeration
        } else {
            zg4_near
        }
    }

    /// `gain_for_scale`: local relief gain for a vertical scale.
    pub fn gain_for_scale(&self, scale: f32, s_near: f32) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        let far = HEIGHT_SCALE;
        if s_near >= far * 0.999 {
            return self.gain_far;
        }
        let t = ((far / scale.max(1e-9)).ln() / (far / s_near).ln()).clamp(0.0, 1.0);
        self.gain_far + (self.gain_near - self.gain_far) * t
    }

    pub fn shapes_relief(&self) -> bool {
        self.enabled
            && (self.mountain_knee_m > 0.0
                || self.mountain_lift > 0.0
                || self.massif_lift > 0.0
                || self.plain_flatten > 0.0)
    }

    /// `squash_weight_for_scale`.
    pub fn squash_weight_for_scale(&self, scale: f32, s_full: f32) -> f32 {
        if !self.shapes_relief() {
            return 0.0;
        }
        let far = HEIGHT_SCALE;
        if s_full >= far * 0.999 {
            return 1.0;
        }
        let t = ((far / scale.max(1e-9)).ln() / (far / s_full).ln()).clamp(0.0, 1.0);
        self.mountain_squash_far + (1.0 - self.mountain_squash_far) * t
    }

    /// `mountain_squash_of`: factor k of a regional amplitude A (m).
    pub fn mountain_squash_of(&self, amplitude_m: f32) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        let mut k = 0.0;
        if self.mountain_knee_m > 0.0 && amplitude_m > self.mountain_knee_m {
            let shown =
                self.mountain_knee_m + (amplitude_m - self.mountain_knee_m) * self.mountain_ratio;
            k = (1.0 - shown / amplitude_m).clamp(0.0, self.mountain_squash_max);
        }
        k - self.mountain_lift_of(amplitude_m) + self.plain_flatten_of(amplitude_m)
    }

    pub fn mountain_lift_of(&self, amplitude_m: f32) -> f32 {
        if !self.enabled || self.mountain_lift <= 0.0 || amplitude_m <= self.mountain_lift_start_m {
            return 0.0;
        }
        let span = (self.mountain_lift_full_m - self.mountain_lift_start_m).max(1.0);
        let t = ((amplitude_m - self.mountain_lift_start_m) / span).clamp(0.0, 1.0);
        let mut lift = self.mountain_lift * t * t * (3.0 - 2.0 * t);
        if self.mountain_lift_power > 0.0 && amplitude_m > self.mountain_lift_full_m {
            lift = (1.0 + lift)
                * (amplitude_m / self.mountain_lift_full_m).powf(self.mountain_lift_power)
                - 1.0;
        }
        lift.min(self.mountain_lift_max)
    }

    pub fn massif_lift_of(&self, amplitude_m: f32) -> f32 {
        if !self.enabled || self.massif_lift <= 0.0 || amplitude_m <= self.mountain_lift_start_m {
            return 0.0;
        }
        let span = (self.mountain_lift_full_m - self.mountain_lift_start_m).max(1.0);
        let t = ((amplitude_m - self.mountain_lift_start_m) / span).clamp(0.0, 1.0);
        let mut lift = self.massif_lift * t * t * (3.0 - 2.0 * t);
        if self.mountain_lift_power > 0.0 && amplitude_m > self.mountain_lift_full_m {
            lift *= (amplitude_m / self.mountain_lift_full_m).powf(self.mountain_lift_power);
        }
        lift.min(self.mountain_lift_max)
    }

    pub fn plain_flatten_of(&self, amplitude_m: f32) -> f32 {
        if !self.enabled || self.plain_flatten <= 0.0 || amplitude_m >= self.plain_flatten_end_m {
            return 0.0;
        }
        let span = (self.plain_flatten_end_m - self.plain_flatten_full_m).max(1.0);
        let t = ((self.plain_flatten_end_m - amplitude_m) / span).clamp(0.0, 1.0);
        self.plain_flatten * t * t * (3.0 - 2.0 * t)
    }
}

/// `CloseCameraProfile` (ZG4): close camera, dynamic exaggeration, clip planes.
#[derive(Debug, Clone)]
pub struct CloseCameraProfile {
    pub level_min_distance: Vec<f32>,
    pub min_distance_slope: f32,
    pub landmark_min_distance: f32,
    pub exaggeration_far: f32,
    pub exaggeration_near: f32,
    pub exaggeration_far_distance: f32,
    pub exaggeration_near_distance: f32,
    pub exaggeration_step: f32,
    pub exaggeration_hysteresis: f32,
    pub pitch_reference_distance: f32,
    pub pitch_closest_deg: f32,
    pub pitch_closest_distance: f32,
    pub look_up_factor: f32,
    pub near_factor: f32,
    pub near_min: f32,
    pub near_max: f32,
    pub far_factor: f32,
    pub far_margin: f32,
    pub far_max: f32,
    pub clearance_factor: f32,
    pub clearance_min: f32,
    pub occlusion_samples: i32,
    pub occlusion_min_t: f32,
    pub crest_samples: i32,
    pub crest_radius_factor: f32,
    pub crest_focus_weight: f32,
}

impl CloseCameraProfile {
    pub fn from_tres(t: &Tres) -> CloseCameraProfile {
        CloseCameraProfile {
            level_min_distance: t
                .array("level_min_distance")
                .unwrap_or_else(|| vec![22.0, 9.0, 5.0, 2.6, 1.5, 0.8, 0.5, 0.3]),
            min_distance_slope: t.f32("min_distance_slope", 0.45),
            landmark_min_distance: t.f32("landmark_min_distance", 2.6),
            exaggeration_far: t.f32("exaggeration_far", 4.31),
            exaggeration_near: t.f32("exaggeration_near", 1.5),
            exaggeration_far_distance: t.f32("exaggeration_far_distance", 45.0),
            exaggeration_near_distance: t.f32("exaggeration_near_distance", 0.45),
            exaggeration_step: t.f32("exaggeration_step", 0.04),
            exaggeration_hysteresis: t.f32("exaggeration_hysteresis", 0.15),
            pitch_reference_distance: t.f32("pitch_reference_distance", 22.0),
            pitch_closest_deg: t.f32("pitch_closest_deg", 11.0),
            pitch_closest_distance: t.f32("pitch_closest_distance", 0.3),
            look_up_factor: t.f32("look_up_factor", 0.3),
            near_factor: t.f32("near_factor", 0.02),
            near_min: t.f32("near_min", 0.002),
            near_max: t.f32("near_max", 1.0),
            far_factor: t.f32("far_factor", 4.0),
            far_margin: t.f32("far_margin", 700.0),
            far_max: t.f32("far_max", 6000.0),
            clearance_factor: t.f32("clearance_factor", 0.06),
            clearance_min: t.f32("clearance_min", 0.004),
            occlusion_samples: t.i32("occlusion_samples", 6),
            occlusion_min_t: t.f32("occlusion_min_t", 0.25),
            crest_samples: t.i32("crest_samples", 8),
            crest_radius_factor: t.f32("crest_radius_factor", 0.5),
            crest_focus_weight: t.f32("crest_focus_weight", 1.0),
        }
    }

    /// `game/resources/close_camera.tres` under `root`.
    pub fn load(root: &Path) -> CloseCameraProfile {
        Self::from_tres(&Tres::load(&root.join("game/resources/close_camera.tres")))
    }

    pub fn min_distance_for_level(&self, level: i32) -> f32 {
        if self.level_min_distance.is_empty() {
            return 22.0;
        }
        let last = self.level_min_distance.len() as i32 - 1;
        self.level_min_distance[level.clamp(0, last) as usize]
    }

    /// Effective near exaggeration floor, raised by the relief profile when enabled.
    pub fn near_exaggeration(&self, relief: &ReliefProfile) -> f32 {
        relief.near_floor(self.exaggeration_near)
    }

    /// Continuous vertical exaggeration (x) for a camera distance.
    pub fn exaggeration_at(&self, distance: f32, relief: &ReliefProfile) -> f32 {
        let near = self.near_exaggeration(relief);
        let lo = self.exaggeration_near_distance.max(1e-4).ln();
        let hi = self
            .exaggeration_far_distance
            .max(self.exaggeration_near_distance * 1.01)
            .ln();
        let t = smoothstep(lo, hi, distance.max(1e-4).ln());
        near * (self.exaggeration_far / near).powf(t)
    }

    /// Continuous vertical scale (world units per metre).
    pub fn vertical_scale_at(&self, distance: f32, relief: &ReliefProfile) -> f32 {
        HEIGHT_SCALE * self.exaggeration_at(distance, relief) / self.exaggeration_far
    }

    /// Scale quantised in geometric steps; `current` (> 0) gives the hysteresis.
    pub fn quantized_scale(&self, distance: f32, current: f32, relief: &ReliefProfile) -> f32 {
        let step = (1.0 + self.exaggeration_step.max(1e-3)).ln();
        let x = (self.vertical_scale_at(distance, relief) / HEIGHT_SCALE).ln() / step;
        let mut index = x.round();
        if current > 0.0 {
            let current_index = (current / HEIGHT_SCALE).ln() / step;
            let current_index = current_index.round();
            if (x - current_index).abs() < 0.5 + self.exaggeration_hysteresis {
                index = current_index;
            }
        }
        let lowest =
            ((self.near_exaggeration(relief) / self.exaggeration_far).ln() / step - 0.5).ceil();
        index = index.clamp(lowest, 0.0);
        HEIGHT_SCALE * (index * step).exp()
    }

    /// Pitch (degrees) below the reference distance.
    pub fn close_pitch_deg(&self, distance: f32, reference_deg: f32) -> f32 {
        let t = self.log_t(distance);
        self.pitch_closest_deg + (reference_deg - self.pitch_closest_deg) * t
    }

    /// Height by which the aim is raised above the focus point.
    pub fn look_up(&self, distance: f32) -> f32 {
        self.look_up_factor * (1.0 - self.log_t(distance)) * distance
    }

    fn log_t(&self, distance: f32) -> f32 {
        let lo = self.pitch_closest_distance.max(1e-4).ln();
        let hi = self
            .pitch_reference_distance
            .max(self.pitch_closest_distance * 1.01)
            .ln();
        ((distance.max(1e-4).ln() - lo) / (hi - lo)).clamp(0.0, 1.0)
    }

    pub fn near_plane(&self, distance: f32) -> f32 {
        (distance * self.near_factor).clamp(self.near_min, self.near_max)
    }

    pub fn far_plane(&self, distance: f32) -> f32 {
        (distance * self.far_factor + self.far_margin).min(self.far_max)
    }

    pub fn clearance(&self, distance: f32) -> f32 {
        (distance * self.clearance_factor).max(self.clearance_min)
    }
}

/// GLSL/Godot `smoothstep`.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profiles() -> (CloseCameraProfile, ReliefProfile) {
        (
            CloseCameraProfile::from_tres(&Tres::default()),
            ReliefProfile::from_tres(&Tres::parse("[resource]\nnear_exaggeration = 2.5\n")),
        )
    }

    #[test]
    fn exaggeration_goes_from_near_floor_to_far_value() {
        let (camera, relief) = profiles();
        assert!((camera.exaggeration_at(0.1, &relief) - 2.5).abs() < 1e-3);
        assert!((camera.exaggeration_at(100.0, &relief) - 4.31).abs() < 1e-3);
        let mid = camera.exaggeration_at(4.5, &relief);
        assert!(mid > 2.5 && mid < 4.31);
    }

    #[test]
    fn quantised_scale_is_capped_by_the_strategic_scale_and_sticky() {
        let (camera, relief) = profiles();
        assert!((camera.quantized_scale(1000.0, -1.0, &relief) - HEIGHT_SCALE).abs() < 1e-9);
        let near = camera.quantized_scale(0.3, -1.0, &relief);
        assert!(near < HEIGHT_SCALE);
        // A tiny move of the distance stays on the same step.
        let again = camera.quantized_scale(0.3001, near, &relief);
        assert_eq!(near, again);
    }

    #[test]
    fn close_pitch_and_planes() {
        let (camera, _) = profiles();
        assert!((camera.close_pitch_deg(0.3, 30.0) - 11.0).abs() < 1e-3);
        assert!((camera.close_pitch_deg(22.0, 30.0) - 30.0).abs() < 1e-3);
        assert!((camera.far_plane(10_000.0) - 6000.0).abs() < 1e-3);
        assert!((camera.near_plane(1000.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn relief_amplitude_factors() {
        let (_, relief) = profiles();
        assert_eq!(relief.plain_flatten_of(1000.0), 0.0);
        let strong = ReliefProfile::from_tres(&Tres::parse(
            "[resource]\nmountain_lift = 0.2\nmountain_lift_start_m = 350\nmountain_lift_full_m = 1400\nplain_flatten = 0.6\nmountain_knee_m = 0\n",
        ));
        assert!(strong.mountain_squash_of(2000.0) < 0.0);
        assert!(strong.mountain_squash_of(50.0) > 0.5);
    }
}
