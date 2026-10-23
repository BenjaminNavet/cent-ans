//! Cast shadows of the campaign trees (port of `Vegetation._tree_shadows_on` /
//! `tree_shadow_limit`, generalised style): the tree batches cast into the sun's shadow maps while
//! the camera is closer to its focus than `generalised_shadow_distance` (70), capped by the quality
//! preset's `veg_shadow_distance` (`data/fx/render_quality.json`: 0 on the low preset), with an 8 %
//! hysteresis so that the shadows do not flicker at the threshold. The drawing itself is
//! `shadow_pass.rs` (depth-only pass of the instanced pipeline); this module only toggles the
//! [`VegShadowCaster`] marker on the tree batches.
//!
//! The soft ground ellipses that stood in for the shadows (vegvis) are gone.

use std::collections::HashMap;

use bevy::prelude::*;
use serde_json::Value;

use super::billboard::{ShadowOrder, VegBatch};
use super::shadow_pass::{MAX_CASCADES, VegShadowCaster};
use super::{VegRender, VegView};
use crate::settings::quality::RenderQuality;

/// `MapPropScale.generalised_shadow_distance`.
pub const GENERALISED_SHADOW_DISTANCE: f32 = 70.0;
/// `Vegetation.shadow_camera_distance` (preset value when the file has none).
pub const SHADOW_CAMERA_DISTANCE: f32 = 300.0;
/// `Vegetation.SHADOW_HYSTERESIS`.
pub const HYSTERESIS: f32 = 0.08;

/// Shadow state of the trees.
#[derive(Resource, Default)]
pub struct TreeShadows {
    /// Trees cast shadows now.
    pub on: bool,
    /// Camera-distance limit in use.
    pub limit: f32,
    /// Tree batches carrying the caster marker.
    pub casters: usize,
    /// `veg_shadow_distance` per quality level (read once).
    presets: Option<HashMap<String, f32>>,
}

/// `tree_shadow_limit`: the preset's distance capped by the generalised one.
pub fn shadow_limit(preset_distance: f32) -> f32 {
    preset_distance.min(GENERALISED_SHADOW_DISTANCE)
}

/// Next on/off state for a camera at `distance` (hysteresis around `limit`).
pub fn next_state(on: bool, distance: f32, limit: f32) -> bool {
    let threshold = limit
        * if on {
            1.0 + HYSTERESIS
        } else {
            1.0 - HYSTERESIS
        };
    limit > 0.0 && distance < threshold
}

/// TS-T9: colour of the ground after the opacity cap of the cast shadows, mirror of the end of the
/// terrain shader (`terrain.wesl`): the shadowed colour never falls under `1 - max_opacity` of the
/// luminance of the lit ground, with the hue of the sky (ambient) light. `ambient` is the sky
/// colour, `shadowed` the colour shaded by the sun map, `lit` the same ground fully lit.
pub fn capped_shadow(
    shadowed: [f32; 3],
    lit: [f32; 3],
    ambient: [f32; 3],
    max_opacity: f32,
) -> [f32; 3] {
    let lum = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let sky_lum = lum(ambient).max(1e-4);
    let floor = lum(lit) * (1.0 - max_opacity);
    std::array::from_fn(|i| shadowed[i].max(floor * ambient[i] / sky_lum))
}

/// `veg_shadow_distance` of every preset of `render_quality.json`.
pub fn read_presets(text: &str) -> HashMap<String, f32> {
    let mut out = HashMap::new();
    let Ok(root) = serde_json::from_str::<Value>(text) else {
        return out;
    };
    if let Some(map) = root.get("presets").and_then(Value::as_object) {
        for (name, level) in map {
            if let Some(d) = level.get("veg_shadow_distance").and_then(Value::as_f64) {
                out.insert(name.clone(), d as f32);
            }
        }
    }
    out
}

fn load_presets() -> HashMap<String, f32> {
    crate::paths::find_root()
        .ok()
        .and_then(|root| {
            std::fs::read_to_string(crate::paths::data_dir(&root).join("fx/render_quality.json"))
                .ok()
        })
        .map(|text| read_presets(&text))
        .unwrap_or_default()
}

pub(super) fn reset(mut shadows: ResMut<TreeShadows>) {
    shadows.on = false;
    shadows.casters = 0;
}

pub(super) fn sync_shadows(
    mut commands: Commands,
    view: Res<VegView>,
    render: Res<VegRender>,
    quality: Option<Res<RenderQuality>>,
    mut shadows: ResMut<TreeShadows>,
    casters: Query<(), With<VegShadowCaster>>,
) {
    if shadows.presets.is_none() {
        shadows.presets = Some(load_presets());
    }
    let preset = match quality.as_ref() {
        None => SHADOW_CAMERA_DISTANCE,
        Some(q) if !q.shadows => 0.0,
        Some(q) => shadows
            .presets
            .as_ref()
            .and_then(|p| p.get(&q.level).copied())
            .unwrap_or(SHADOW_CAMERA_DISTANCE),
    };
    let limit = shadow_limit(preset);
    shadows.limit = limit;
    shadows.on = view.active && next_state(shadows.on, view.distance, limit);
    let mut count = 0;
    for entity in render.entities.values() {
        let has = casters.contains(*entity);
        if shadows.on {
            count += 1;
            if !has {
                commands
                    .entity(*entity)
                    .try_insert(VegShadowCaster::default());
            }
        } else if has {
            commands.entity(*entity).try_remove::<VegShadowCaster>();
        }
    }
    shadows.casters = count;
}

/// The camera seen by the cascades, for [`cascade_range`].
#[derive(Clone, Copy, Debug)]
pub struct CascadeView {
    /// Ground point (xz) and height of the camera.
    pub ground: Vec2,
    pub height: f32,
    /// `1 / cos` of the widest angle between the view axis and a ray of the frustum: a point at
    /// view depth `z` lies at most `z * slant` from the camera.
    pub slant: f32,
    /// Horizontal shadow length per unit of height (`1 / tan` of the sun's elevation).
    pub shadow_run: f32,
}

/// Records of a sorted batch that may cast into the cascade covering view depths `near..far`:
/// a tree is kept when its ground distance to the camera fits the slice's distances, widened by
/// the reach of its shadow and by how far the camera moved since the sort.
pub fn cascade_range(order: &ShadowOrder, view: &CascadeView, near: f32, far: f32) -> [u32; 2] {
    if order.distances.is_empty() {
        return [0, u32::MAX];
    }
    let reach = order.max_height * 1.5 * view.shadow_run + view.ground.distance(order.origin) + 0.5;
    let drop = (view.height - order.min_y).max(0.0);
    let min = (near * near - drop * drop).max(0.0).sqrt() - reach;
    let max = far * view.slant + reach;
    let start = order.distances.partition_point(|d| *d < min);
    let end = order.distances.partition_point(|d| *d <= max);
    [start as u32, end as u32]
}

/// Per-cascade record runs of the shadow-casting tree batches (after the transforms, so with
/// this frame's camera).
pub(super) fn shadow_ranges(
    cameras: Query<(&GlobalTransform, &Projection), With<crate::campaign::camera::CampaignCamera>>,
    suns: Query<(&GlobalTransform, &bevy::light::CascadeShadowConfig), With<DirectionalLight>>,
    mut batches: Query<(&VegBatch, &mut VegShadowCaster)>,
) {
    let view = cameras.iter().next().and_then(|(camera, projection)| {
        let Projection::Perspective(p) = projection else {
            return None;
        };
        let (sun, config) = suns.iter().next()?;
        let t = (p.fov * 0.5).tan();
        let elevation = (-sun.forward().y).clamp(0.08, 1.0);
        Some((
            CascadeView {
                ground: camera.translation().xz(),
                height: camera.translation().y,
                slant: (1.0 + t * t * (1.0 + p.aspect_ratio * p.aspect_ratio)).sqrt(),
                shadow_run: (1.0 - elevation * elevation).sqrt() / elevation,
            },
            config.clone(),
        ))
    });
    let off = crate::campaign::perf_switch::off("vegcull");
    for (batch, mut caster) in &mut batches {
        let mut ranges = [[0, u32::MAX]; MAX_CASCADES];
        if let (Some((view, config)), false) = (view.as_ref(), off) {
            for (i, range) in ranges.iter_mut().enumerate() {
                let Some(&far) = config.bounds.get(i) else {
                    *range = [0, 0];
                    continue;
                };
                let near = if i == 0 {
                    config.minimum_distance
                } else {
                    config.bounds[i - 1] * (1.0 - config.overlap_proportion)
                };
                *range = cascade_range(&batch.shadow_order, view, near, far);
            }
        }
        if caster.ranges != ranges {
            caster.ranges = ranges;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cascades_keep_the_trees_of_their_slice() {
        let order = ShadowOrder {
            origin: Vec2::ZERO,
            distances: (0..100).map(|d| d as f32).collect(),
            max_height: 1.0,
            min_y: 0.0,
        };
        let view = CascadeView {
            ground: Vec2::ZERO,
            height: 20.0,
            slant: 1.2,
            shadow_run: 1.0,
        };
        // A slice under the camera's height starts at the foot of the camera.
        assert_eq!(cascade_range(&order, &view, 1.0, 10.0), [0, 15]);
        // Farther: the near trees are skipped (sqrt(40² - 20²) - 2 = 32.6).
        assert_eq!(cascade_range(&order, &view, 40.0, 50.0), [33, 63]);
        // The camera moved since the sort: the run widens.
        let moved = CascadeView {
            ground: Vec2::new(5.0, 0.0),
            ..view
        };
        assert_eq!(cascade_range(&order, &moved, 40.0, 50.0), [28, 68]);
        // Unsorted batch: everything.
        assert_eq!(
            cascade_range(&ShadowOrder::default(), &view, 40.0, 50.0),
            [0, u32::MAX]
        );
    }

    #[test]
    fn shadows_switch_with_hysteresis() {
        let limit = shadow_limit(300.0);
        assert_eq!(limit, GENERALISED_SHADOW_DISTANCE);
        assert!(next_state(false, 60.0, limit));
        assert!(!next_state(false, 66.0, limit));
        assert!(next_state(true, 74.0, limit));
        assert!(!next_state(true, 80.0, limit));
        assert!(!next_state(false, 10.0, shadow_limit(0.0)));
    }

    #[test]
    fn cast_shadow_opacity_is_capped_and_sky_tinted() {
        let ambient = [0.5, 0.6, 0.9];
        let lit = [0.8, 0.8, 0.8];
        // Black shadow (no ambient on the ground): lifted to 60 % of the lit luminance, blue-tinted.
        let out = capped_shadow([0.0; 3], lit, ambient, 0.4);
        let lum = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        assert!((lum(out) - 0.8 * 0.6).abs() < 1e-4, "{out:?}");
        assert!(out[2] > out[0], "bluish: {out:?}");
        // A shadow already lighter than the floor is left alone; a lit ground too.
        let light = [0.7, 0.7, 0.75];
        assert_eq!(capped_shadow(light, lit, ambient, 0.4), light);
        // No cap: untouched.
        assert_eq!(capped_shadow([0.0; 3], lit, ambient, 1.0), [0.0; 3]);
    }

    #[test]
    fn presets_are_read_from_the_quality_file() {
        let presets = load_presets();
        assert!(presets.contains_key("low"), "{presets:?}");
        assert!(presets.values().any(|d| *d == 0.0));
    }
}
