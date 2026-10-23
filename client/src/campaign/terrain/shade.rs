//! Relief shade of the campaign ground: cast shadows of the relief and horizon occlusion
//! (valley AO), baked into the ground cache (`ground_cache.rs`, ADR 0745).
//!
//! The marching of the block-averaged heightfield (`height_lod`, 16 px blocks) runs in the vertex
//! stage of the bake pass (`terrain.wesl`, `relief_shade`), once per node vertex, never per pixel
//! and per frame; the result is interpolated over the node and stored in a `Rg8Unorm` atlas layer
//! (R sun visibility, G horizon occlusion). The sun enters the cache signature quantised, so a
//! slow sun glide rebakes in steps (at most once a second, `ground_cache::invalidate`).
//! Settings: `data/fx/campaign_relief_shade.json`.

use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use serde::Deserialize;

use crate::paths;

const DATA_FILE: &str = "fx/campaign_relief_shade.json";

/// `data/fx/campaign_relief_shade.json`.
#[derive(Resource, Debug, Clone, Deserialize, PartialEq)]
pub struct ReliefShadeSettings {
    pub enabled: bool,
    /// Darkening of the ground in the relief shadow (0 none, 1 down to the sky light).
    pub shadow_strength: f32,
    /// Share of the horizon occlusion applied to the ambient light.
    pub ao_strength: f32,
    /// Reach of the sun ray march (map px).
    pub max_distance_px: f32,
    /// Samples along the sun ray.
    pub steps: u32,
    /// Penumbra width (m of altitude at the ray).
    pub penumbra_m: f32,
    /// Lift of the ray origin above the ground (m), against self-shadowing.
    pub bias_m: f32,
    /// Reach of the horizon search (map px).
    pub ao_radius_px: f32,
    /// Quantisation of the sun direction in the cache signature (degrees).
    pub sun_quantize_deg: f32,
}

impl Default for ReliefShadeSettings {
    fn default() -> Self {
        ReliefShadeSettings {
            enabled: false,
            shadow_strength: 0.0,
            ao_strength: 0.0,
            max_distance_px: 512.0,
            steps: 24,
            penumbra_m: 80.0,
            bias_m: 6.0,
            ao_radius_px: 160.0,
            sun_quantize_deg: 1.0,
        }
    }
}

impl ReliefShadeSettings {
    pub fn load() -> ReliefShadeSettings {
        let Some(root) = paths::find_root().ok() else {
            return Self::default();
        };
        match std::fs::read(paths::data_dir(&root).join(DATA_FILE)) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                error!("{DATA_FILE}: {error}");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }
}

/// Uniform block `ShadeParams` of `terrain.wesl` (binding 43).
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct ShadeParams {
    /// Quantised direction towards the sun (xyz), 1 in w when the shade is baked.
    pub sun: Vec4,
    /// Reach (map px), steps, penumbra (m), bias (m).
    pub march: Vec4,
    /// Shadow strength, AO strength, AO reach (map px), displayed units per metre of altitude.
    pub mix: Vec4,
}

/// Direction towards the sun snapped to a grid of `step_deg` degrees (azimuth and elevation).
pub fn quantize_sun(dir: Vec3, step_deg: f32) -> Vec3 {
    let dir = dir.normalize_or(Vec3::Y);
    if step_deg <= 0.0 {
        return dir;
    }
    let step = step_deg.to_radians();
    let elevation = (dir.y.clamp(-1.0, 1.0).asin() / step).round() * step;
    let azimuth = (dir.z.atan2(dir.x) / step).round() * step;
    Vec3::new(
        elevation.cos() * azimuth.cos(),
        elevation.sin(),
        elevation.cos() * azimuth.sin(),
    )
}

/// Displayed units per metre of altitude of the exaggerated relief, as used to turn the sun
/// elevation into a rise in metres per map px. Quantised to 1/16 of the far scale so a scale
/// glide rebakes in steps.
pub fn quantize_scale(scale: f32, gain: f32) -> f32 {
    let value = (scale * (1.0 + gain.max(0.0))).max(1e-5);
    let grid = super::profile::HEIGHT_SCALE / 16.0;
    (value / grid).round().max(1.0) * grid
}

/// Uniform for the current settings; `bound` is false while the heightfield is not on the GPU.
pub fn params(
    settings: &ReliefShadeSettings,
    sun: Vec3,
    scale: f32,
    gain: f32,
    bound: bool,
) -> ShadeParams {
    if !(settings.enabled && bound) {
        // Off: a constant uniform, so the sun gliding never rebakes the cache for nothing.
        return ShadeParams::default();
    }
    let sun = quantize_sun(sun, settings.sun_quantize_deg);
    ShadeParams {
        sun: sun.extend(1.0),
        march: Vec4::new(
            settings.max_distance_px.max(16.0),
            settings.steps.clamp(1, 64) as f32,
            settings.penumbra_m.max(1.0),
            settings.bias_m.max(0.0),
        ),
        mix: Vec4::new(
            settings.shadow_strength.clamp(0.0, 1.0),
            settings.ao_strength.clamp(0.0, 1.0),
            settings.ao_radius_px.max(16.0),
            quantize_scale(scale, gain),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantised_sun_is_stable_within_a_step() {
        let a = quantize_sun(Vec3::new(0.5, 0.35, 0.8).normalize(), 1.0);
        let b = quantize_sun(Vec3::new(0.5, 0.35 + 0.001, 0.8).normalize(), 1.0);
        assert_eq!(a, b);
        assert!((a.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn quantised_sun_changes_across_steps() {
        let a = quantize_sun(Vec3::new(0.5, 0.30, 0.8).normalize(), 1.0);
        let b = quantize_sun(Vec3::new(0.5, 0.40, 0.8).normalize(), 1.0);
        assert_ne!(a, b);
    }

    #[test]
    fn disabled_or_unbound_params_flag_off() {
        let mut settings = ReliefShadeSettings {
            enabled: true,
            shadow_strength: 3.0,
            steps: 1000,
            ..default()
        };
        let sun = Vec3::new(0.5, 0.4, 0.7).normalize();
        assert_eq!(params(&settings, sun, 0.006, 0.0, false).sun.w, 0.0);
        let on = params(&settings, sun, 0.006, 0.0, true);
        assert_eq!(on.sun.w, 1.0);
        assert_eq!(on.mix.x, 1.0, "strength clamped");
        assert_eq!(on.march.y, 64.0, "steps clamped");
        settings.enabled = false;
        assert_eq!(params(&settings, sun, 0.006, 0.0, true).sun.w, 0.0);
    }

    #[test]
    fn scale_quantisation_is_steady() {
        let a = quantize_scale(0.006, 0.0);
        assert_eq!(a, quantize_scale(0.006 * 1.01, 0.0));
        assert!(quantize_scale(0.0, 0.0) > 0.0);
    }

    #[test]
    fn shipped_settings_parse() {
        let text = include_str!("../../../../data/fx/campaign_relief_shade.json");
        let settings: ReliefShadeSettings = serde_json::from_str(text).unwrap();
        assert!(settings.enabled);
        assert!(settings.steps >= 1);
    }
}
