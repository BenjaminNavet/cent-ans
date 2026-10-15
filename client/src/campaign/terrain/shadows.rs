//! Sun shadows of the campaign (port of the shadow part of `campaign_atmosphere.gd`
//! `apply_distance`): the shadow range follows the camera distance (`range_factor` x distance, at
//! least `far_min`, down to `close_min` in the close view), the shadows are cut above
//! `max_camera_distance` (the relief shading is enough from that high). Tuning:
//! `data/fx/campaign_lighting.json` § `shadows`.
//!
//! The settings lot owns the final light state (`settings::quality::apply_shadows` multiplies the
//! cascade bounds by the quality preset's `map_shadow_range` and ands the shadows with the user
//! toggle): this module writes the *base* of the sun ([`SunBase`]) that it scales.

use bevy::light::{CascadeShadowConfig, CascadeShadowConfigBuilder};
use bevy::prelude::*;
use serde_json::{Value, json};

use super::CampaignSun;
use crate::campaign::camera::rig::CampaignRig;
use crate::settings::quality::SunBase;

#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ShadowTuning {
    pub range_factor: f32,
    pub max_camera_distance: f32,
    pub far_min: f32,
    pub close_min: f32,
    pub close_begin_distance: f32,
    pub close_full_distance: f32,
    pub cascades: usize,
    pub first_split: f32,
    pub overlap: f32,
}

impl Default for ShadowTuning {
    /// The `@export` defaults of `campaign_atmosphere.gd` (Godot 4 splits 0.1 / 0.2 / 0.5).
    fn default() -> Self {
        ShadowTuning {
            range_factor: 3.5,
            max_camera_distance: 650.0,
            far_min: 50.0,
            close_min: 12.0,
            close_begin_distance: 30.0,
            close_full_distance: 6.0,
            cascades: 4,
            first_split: 0.1,
            overlap: 0.2,
        }
    }
}

impl ShadowTuning {
    pub fn from_json(block: &Value) -> ShadowTuning {
        let d = ShadowTuning::default();
        let num = |key: &str, fallback: f32| block[key].as_f64().map_or(fallback, |v| v as f32);
        ShadowTuning {
            range_factor: num("range_factor", d.range_factor),
            max_camera_distance: num("max_camera_distance", d.max_camera_distance),
            far_min: num("far_min", d.far_min),
            close_min: num("close_min", d.close_min),
            close_begin_distance: num("close_begin_distance", d.close_begin_distance),
            close_full_distance: num("close_full_distance", d.close_full_distance),
            cascades: block["cascades"]
                .as_u64()
                .map_or(d.cascades, |v| v.clamp(1, 4) as usize),
            first_split: num("first_split", d.first_split).clamp(0.01, 1.0),
            overlap: num("overlap", d.overlap).clamp(0.0, 1.0),
        }
    }

    pub fn load(root: &std::path::Path) -> ShadowTuning {
        std::fs::read_to_string(root.join("data/fx/campaign_lighting.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map_or_else(ShadowTuning::default, |v| {
                ShadowTuning::from_json(&v["shadows"])
            })
    }

    /// Whether the sun casts shadows with the camera at `distance`.
    pub fn enabled(&self, distance: f32) -> bool {
        distance < self.max_camera_distance
    }

    /// Shadow range (world units) with the camera at `distance`.
    pub fn range(&self, distance: f32) -> f32 {
        let close = 1.0
            - smoothstep(
                self.close_full_distance,
                self.close_begin_distance,
                distance,
            );
        let floor = self.far_min + (self.close_min - self.far_min) * close;
        (distance * self.range_factor).max(floor)
    }

    /// Far bounds of the cascades with the camera at `distance`.
    pub fn bounds(&self, distance: f32, near: f32) -> Vec<f32> {
        let range = self.range(distance);
        CascadeShadowConfigBuilder {
            num_cascades: self.cascades,
            minimum_distance: near.min(range * 0.5),
            maximum_distance: range,
            first_cascade_far_bound: (range * self.first_split).max(near * 1.01),
            overlap_proportion: self.overlap,
        }
        .build()
        .bounds
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// What the last frame wrote, for the dump.
#[derive(Resource, Debug, Default, Clone)]
pub struct ShadowState {
    pub enabled: bool,
    pub range: f32,
    pub bounds: Vec<f32>,
}

pub fn load_tuning(mut commands: Commands) {
    let tuning = crate::paths::find_root()
        .map(|root| ShadowTuning::load(&root))
        .unwrap_or_default();
    commands.insert_resource(tuning);
    commands.insert_resource(ShadowState::default());
}

/// Sizes the sun's shadow cascades to the camera distance (writes the sun's [`SunBase`]).
pub fn size_shadows(
    tuning: Res<ShadowTuning>,
    rig: Option<Res<CampaignRig>>,
    mut state: ResMut<ShadowState>,
    mut suns: Query<(&mut SunBase, &mut CascadeShadowConfig), With<CampaignSun>>,
) {
    let Some(rig) = rig else {
        return;
    };
    let distance = rig.distance;
    let enabled = tuning.enabled(distance);
    // Same near plane as the camera rig (`CloseCameraProfile::near_plane`, 0.02 x distance).
    let near = (distance * 0.02).clamp(0.002, 1.0);
    let bounds = tuning.bounds(distance, near);
    for (mut base, mut config) in &mut suns {
        if base.shadows != enabled {
            base.shadows = enabled;
        }
        if base.bounds.as_ref() != Some(&bounds) {
            base.bounds = Some(bounds.clone());
        }
        if (config.minimum_distance - near).abs() > 1e-4
            || config.overlap_proportion != tuning.overlap
        {
            config.minimum_distance = near;
            config.overlap_proportion = tuning.overlap;
        }
    }
    state.enabled = enabled;
    state.range = tuning.range(distance);
    state.bounds = bounds;
}

pub fn dump(world: &World) -> Value {
    let Some(state) = world.get_resource::<ShadowState>() else {
        return Value::Null;
    };
    let lights: Vec<Value> = world
        .iter_entities()
        .filter(|e| e.contains::<CampaignSun>())
        .filter_map(|e| {
            let light = e.get::<DirectionalLight>()?;
            let config = e.get::<CascadeShadowConfig>()?;
            Some(json!({
                "shadow_maps_enabled": light.shadow_maps_enabled,
                "bounds": config.bounds,
                "minimum_distance": config.minimum_distance,
            }))
        })
        .collect();
    json!({
        "enabled": state.enabled,
        "range": state.range,
        "base_bounds": state.bounds,
        "sun": lights,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_follows_the_camera_and_stops_high_up() {
        let t = ShadowTuning::default();
        assert!((t.range(400.0) - 1400.0).abs() < 1e-3);
        assert!((t.range(3.0) - 12.0).abs() < 1e-3);
        assert!((t.range(40.0) - 140.0).abs() < 1e-3);
        assert!(t.enabled(400.0));
        assert!(!t.enabled(1200.0));
        let b = t.bounds(400.0, 8.0);
        assert_eq!(b.len(), 4);
        assert!((b[3] - 1400.0).abs() < 1e-2);
        assert!((b[0] - 140.0).abs() < 1e-2);
    }

    #[test]
    fn real_data_parses() {
        let root = crate::paths::find_root().expect("repo root");
        let t = ShadowTuning::load(&root);
        assert_eq!(t, ShadowTuning::default());
    }
}
