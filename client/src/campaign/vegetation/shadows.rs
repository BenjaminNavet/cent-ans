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

use super::shadow_pass::VegShadowCaster;
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
                commands.entity(*entity).try_insert(VegShadowCaster);
            }
        } else if has {
            commands.entity(*entity).try_remove::<VegShadowCaster>();
        }
    }
    shadows.casters = count;
}

#[cfg(test)]
mod tests {
    use super::*;

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
