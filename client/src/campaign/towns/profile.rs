//! Render settings of the 1:1 towns (`game/resources/town_render.tres`, `TownRenderProfile`).
//! Distances are map units (1 unit = 719 m) measured from the camera rig.

use std::path::Path;

use crate::campaign::terrain::tres::Tres;

#[derive(Debug, Clone, PartialEq, bevy::prelude::Resource)]
pub struct TownRenderProfile {
    /// Kit houses up to this rig distance, simple blocks beyond.
    pub detail_range: f32,
    /// Blocks up to this distance.
    pub block_range: f32,
    pub stream_min: f32,
    pub stream_factor: f32,
    pub stream_max: f32,
    pub unload_factor: f32,
    /// The 1:1 layer is active below this rig distance.
    pub max_rig_distance: f32,
    pub rig_hysteresis: f32,
    /// Share of `max_rig_distance` over which the layer fades out (0 = hard switch).
    pub layer_fade_band: f32,
    /// Plans built per frame.
    pub max_plan_jobs: usize,
    pub plan_cache: usize,
}

impl Default for TownRenderProfile {
    fn default() -> Self {
        Self {
            detail_range: 1.6,
            block_range: 42.0,
            stream_min: 4.0,
            stream_factor: 2.5,
            stream_max: 48.0,
            unload_factor: 1.3,
            max_rig_distance: 45.0,
            rig_hysteresis: 0.1,
            layer_fade_band: 0.15,
            max_plan_jobs: 2,
            plan_cache: 24,
        }
    }
}

impl TownRenderProfile {
    pub fn from_tres(text: &str) -> Self {
        let t = Tres::parse(text);
        let d = Self::default();
        let f = |key: &str, default: f32| t.f32(key, default);
        Self {
            detail_range: f("detail_range", d.detail_range),
            block_range: f("block_range", d.block_range),
            stream_min: f("stream_min", d.stream_min),
            stream_factor: f("stream_factor", d.stream_factor),
            stream_max: f("stream_max", d.stream_max),
            unload_factor: f("unload_factor", d.unload_factor),
            max_rig_distance: f("max_rig_distance", d.max_rig_distance),
            rig_hysteresis: f("rig_hysteresis", d.rig_hysteresis),
            layer_fade_band: f("layer_fade_band", d.layer_fade_band),
            max_plan_jobs: f("max_plan_jobs", d.max_plan_jobs as f32) as usize,
            plan_cache: f("plan_cache", d.plan_cache as f32) as usize,
        }
    }

    pub fn load(root: &Path) -> Self {
        std::fs::read_to_string(root.join("game/resources/town_render.tres"))
            .map(|t| Self::from_tres(&t))
            .unwrap_or_default()
    }

    /// Weight [0, 1] of the 1:1 layer at rig distance `d`: 1 up to `max_rig_distance` x
    /// (1 - band), 0 at `max_rig_distance` (`layer_weight`, ZF-C).
    pub fn layer_weight(&self, d: f32) -> f32 {
        if self.layer_fade_band <= 0.0 {
            return if d < self.max_rig_distance { 1.0 } else { 0.0 };
        }
        1.0 - crate::campaign::camera::zoom::smoothstep(
            self.max_rig_distance * (1.0 - self.layer_fade_band),
            self.max_rig_distance,
            d,
        )
    }

    /// Streaming radius around the rig focus for a rig distance.
    pub fn stream_radius(&self, rig_distance: f32) -> f32 {
        (self.stream_factor * rig_distance).clamp(self.stream_min, self.stream_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_profile_and_weights() {
        let root = crate::paths::find_root().unwrap();
        let p = TownRenderProfile::load(&root);
        assert_eq!(p.max_rig_distance, 45.0);
        assert_eq!(p.layer_weight(10.0), 1.0);
        assert_eq!(p.layer_weight(45.0), 0.0);
        let mid = p.layer_weight(42.0);
        assert!(mid > 0.0 && mid < 1.0);
        assert_eq!(p.stream_radius(1.0), p.stream_min);
        assert_eq!(p.stream_radius(100.0), p.stream_max);
    }
}
