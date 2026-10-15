//! Dense-forest layer settings (`forest_detail_profile.gd`, `resources/forest_detail.tres`).
//! Purely visual.
//!
//! Map trees are scattered at `Vegetation.spacing` (1.35 u, ~970 m); at 1:1 scale that is one tree
//! per km2. The dense layer adds, around the focus, trees on a fine step
//! (`spacing x full_scale`) closing the canopy of forests within the tree range
//! (`TreeRange::max_distance`); beyond it the terrain canopy carries the forest. This module holds
//! the profile and the per-zoom decisions (share shown, seeding level, radius, budget gain);
//! the layer is switched off in the generalised style (not built here).

use serde::Deserialize;

/// Range of individual trees (`map_prop_scale.tres`: `tree_max_distance`, `visibility_fade`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeRange {
    pub max_distance: f32,
    pub fade: f32,
}

impl Default for TreeRange {
    fn default() -> Self {
        TreeRange {
            max_distance: 30.0,
            fade: 0.2,
        }
    }
}

impl TreeRange {
    /// `range_weight`: 1 below, 0 beyond `max_distance`, smooth over the last `fade` share.
    pub fn weight(&self, distance: f32) -> f32 {
        let fade = (self.max_distance * self.fade).max(1e-4);
        1.0 - smoothstep(self.max_distance - fade, self.max_distance, distance)
    }
}

/// GLSL / GDScript `smoothstep(from, to, x)`.
pub fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    if (to - from).abs() < f32::EPSILON {
        return if x < from { 0.0 } else { 1.0 };
    }
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Settings of the dense layer; defaults are the values of `forest_detail.tres`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct ForestDetailProfile {
    /// Fine step = map step x this ratio (0.022 -> ~21 m between two trees).
    pub full_scale: f32,
    /// Share shown: 1 up to `dense_full_distance`, then `far_share` at the tree range.
    pub dense_full_distance: f32,
    pub far_share: f32,
    /// Below this share (trees still big) the layer is off.
    pub min_fraction: f32,
    /// Cell side (world units, divisor of the 256 tile) and parts per side.
    pub cell_size: f32,
    pub parts_side: u32,
    /// Radius around the focus = `radius_factor` x rig distance, bounded, x budget gain.
    pub radius_factor: f32,
    pub radius_min: f32,
    pub radius_max: f32,
    pub fade_from: f32,
    /// Displayed instance budget; beyond it the radius shrinks (gain >= `min_gain`).
    pub instance_budget: u32,
    pub min_gain: f32,
    /// Seeding levels (kept seeds): a cell is reseeded at a higher level when the wanted share
    /// exceeds the seeded one; margin `keep_margin`.
    pub keep_levels: Vec<f32>,
    pub keep_margin: f32,
    /// Shadows of parts within `detail_factor` x rig distance.
    pub detail_factor: f32,
    /// Margin (map units) added to the card -> impostor fade reach.
    pub near_margin: f32,
    /// Corridors without trees beside thin rivers and draped roads (metres beyond half width).
    pub river_clearance_m: f32,
    pub road_clearance_m: f32,
    /// Per-frame limits: part rewrites, simultaneous scatters and regroundings, cached cells.
    pub max_part_updates: u32,
    pub max_jobs: u32,
    pub max_ground_jobs: u32,
    pub max_cells: u32,
    /// Stored instances (memory cap): older out-of-view cells are freed beyond it.
    pub max_stored_instances: u32,
}

impl Default for ForestDetailProfile {
    fn default() -> Self {
        ForestDetailProfile {
            full_scale: 0.022,
            dense_full_distance: 8.0,
            far_share: 0.35,
            min_fraction: 0.003,
            cell_size: 8.0,
            parts_side: 2,
            radius_factor: 3.5,
            radius_min: 6.0,
            radius_max: 50.0,
            fade_from: 0.4,
            instance_budget: 300_000,
            min_gain: 0.2,
            keep_levels: vec![0.015625, 0.0625, 0.25, 1.0],
            keep_margin: 1.25,
            detail_factor: 1.2,
            near_margin: 4.0,
            river_clearance_m: 25.0,
            road_clearance_m: 10.0,
            max_part_updates: 96,
            max_jobs: 3,
            max_ground_jobs: 1,
            max_cells: 96,
            max_stored_instances: 800_000,
        }
    }
}

impl ForestDetailProfile {
    /// Share [0, 1] of the dense layer to show at rig `distance` (before the quality preset),
    /// zero beyond the tree range.
    pub fn fraction_at(&self, distance: f32, range: &TreeRange) -> f32 {
        let t = smoothstep(self.dense_full_distance, range.max_distance, distance);
        range.weight(distance) * (1.0 + (self.far_share - 1.0) * t)
    }

    /// Whether the layer is on at `distance` for a quality `preset` multiplier.
    pub fn active_at(&self, distance: f32, range: &TreeRange, preset: f32) -> bool {
        self.fraction_at(distance, range) * preset >= self.min_fraction
    }

    /// Smallest seeding level covering `need` (with the margin), 1 at most.
    pub fn keep_for(&self, need: f32) -> f32 {
        let wanted = (need * self.keep_margin).min(1.0);
        self.keep_levels
            .iter()
            .copied()
            .find(|level| *level >= wanted)
            .unwrap_or(1.0)
    }

    /// Radius (world units) around the focus at rig `distance`: bounded, limited by the tree
    /// range, scaled by the budget `gain`.
    pub fn radius_at(&self, distance: f32, range: &TreeRange, gain: f32) -> f32 {
        (self.radius_factor * distance)
            .clamp(self.radius_min, self.radius_max)
            .min(range.max_distance)
            * gain
    }

    /// Budget gain for `shown` instances currently displayed (smoothed by the caller): shrinks the
    /// radius by the square root of the over-budget ratio, never under `min_gain`.
    pub fn budget_gain(&self, shown: u32) -> f32 {
        if shown <= self.instance_budget {
            return 1.0;
        }
        (self.instance_budget as f32 / shown as f32)
            .sqrt()
            .max(self.min_gain)
    }

    /// Fine step between two dense trees for a map step `spacing`.
    pub fn fine_spacing(&self, spacing: f32) -> f32 {
        spacing * self.full_scale
    }

    /// Cell side in px of the 256 px scatter tile grid, parts included.
    pub fn part_size(&self) -> f32 {
        self.cell_size / self.parts_side.max(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_share_up_close_and_nothing_beyond_range() {
        let profile = ForestDetailProfile::default();
        let range = TreeRange::default();
        assert_eq!(profile.fraction_at(3.0, &range), 1.0);
        assert_eq!(profile.fraction_at(31.0, &range), 0.0);
        let mid = profile.fraction_at(20.0, &range);
        assert!(mid < 1.0 && mid > profile.far_share * 0.9, "{mid}");
    }

    #[test]
    fn share_decreases_monotonically_with_distance() {
        let profile = ForestDetailProfile::default();
        let range = TreeRange::default();
        let mut previous = 2.0;
        for step in 0..=60 {
            let value = profile.fraction_at(step as f32 * 0.5, &range);
            assert!(value <= previous + 1e-6);
            previous = value;
        }
    }

    #[test]
    fn keep_levels_pick_the_smallest_covering_level() {
        let profile = ForestDetailProfile::default();
        assert_eq!(profile.keep_for(0.01), 0.015625);
        assert_eq!(profile.keep_for(0.05), 0.0625);
        assert_eq!(profile.keep_for(0.2), 0.25);
        assert_eq!(profile.keep_for(0.9), 1.0);
        assert_eq!(profile.keep_for(5.0), 1.0);
    }

    #[test]
    fn radius_is_bounded_and_budget_shrinks_it() {
        let profile = ForestDetailProfile::default();
        let range = TreeRange::default();
        assert_eq!(profile.radius_at(0.5, &range, 1.0), profile.radius_min);
        assert_eq!(profile.radius_at(20.0, &range, 1.0), range.max_distance);
        assert_eq!(profile.budget_gain(100), 1.0);
        let gain = profile.budget_gain(1_200_000);
        assert!((gain - 0.5).abs() < 1e-6);
        assert_eq!(profile.budget_gain(u32::MAX), profile.min_gain);
    }

    #[test]
    fn profile_parses_partial_json() {
        let parsed: ForestDetailProfile = serde_json::from_str(r#"{"cell_size": 4.0}"#).unwrap();
        assert_eq!(parsed.cell_size, 4.0);
        assert_eq!(parsed.max_cells, 96);
    }
}
