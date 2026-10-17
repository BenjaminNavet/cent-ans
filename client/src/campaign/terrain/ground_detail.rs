//! Close-range ground detail (lot VQ cground): shader module `ground_detail.wesl`, imported by
//! `terrain.wesl` (macro tone drift of the open ground, per-parcel tone / saturation, strips with
//! balks, soil mottling). Tunables: `data/fx/ground_detail.json`; level: knob `ground_detail` of
//! `data/fx/render_quality.json` (0 off, 1 tones, 2 + strips and mottling), overridable by
//! `CENT_ANS_GROUND_DETAIL=0|1|2` (A/B benches).

use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use serde_json::Value;

use super::TerrainMaterialHandle;
use super::material::TerrainMaterial;
use crate::paths;
use crate::settings::RenderQuality;

/// Uniform block `GdParams` of `ground_detail.wesl` (binding 40 of the terrain material).
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct GdParams {
    /// Level, macro tone amp, macro hue amp, macro fade end footprint.
    pub a: Vec4,
    /// Parcel luminance contrast, saturation, jitter, structure fade end footprint.
    pub b: Vec4,
    /// Strip tone amp, balk amp, width min (m), width max (m).
    pub c: Vec4,
    /// Mottle amp, mottle period (m), structure fade start footprint, mottle fade end footprint.
    pub d: Vec4,
    /// Macro periods (map px), plough share of the dark strips.
    pub e: Vec4,
}

impl Default for GdParams {
    /// Off: the shader returns the colours untouched.
    fn default() -> Self {
        Self::from_json(&Value::Null, 0)
    }
}

fn number(block: &Value, key: &str, fallback: f32) -> f32 {
    block
        .get(key)
        .and_then(Value::as_f64)
        .map_or(fallback, |v| v as f32)
}

impl GdParams {
    /// Parameters of `ground_detail.json` at `level`.
    pub fn from_json(spec: &Value, level: u32) -> Self {
        let (m, p, s, t) = (
            &spec["macro"],
            &spec["parcel"],
            &spec["strips"],
            &spec["mottle"],
        );
        let periods = m["periods_px"].as_array();
        let period = |i: usize, fallback: f32| {
            periods
                .and_then(|a| a.get(i))
                .and_then(Value::as_f64)
                .map_or(fallback, |v| v as f32)
        };
        Self {
            a: Vec4::new(
                level as f32,
                number(m, "tone_amp", 0.16),
                number(m, "hue_amp", 1.0),
                number(m, "fade_footprint", 1.2),
            ),
            b: Vec4::new(
                number(p, "luma_contrast", 0.55),
                number(p, "saturation", 0.35),
                number(p, "jitter", 0.22),
                number(p, "fade_end_footprint", 0.45),
            ),
            c: Vec4::new(
                number(s, "tone_amp", 0.14),
                number(s, "balk_amp", 0.28),
                number(s, "width_min_m", 38.0),
                number(s, "width_max_m", 95.0),
            ),
            d: Vec4::new(
                number(t, "amp", 0.10),
                number(t, "period_m", 160.0),
                number(p, "fade_start_footprint", 0.12),
                number(t, "fade_end_footprint", 0.3),
            ),
            e: Vec4::new(
                period(0, 4.0),
                period(1, 11.0),
                period(2, 29.0),
                number(s, "plough_share", 0.8),
            ),
        }
    }
}

/// Level of the `ground_detail` knob for a quality `level` (`render_quality.json`).
pub fn knob_for(presets: &Value, level: &str) -> u32 {
    if let Some(forced) = std::env::var("CENT_ANS_GROUND_DETAIL")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
    {
        return forced.min(2);
    }
    presets["presets"][level]["ground_detail"]
        .as_u64()
        .map_or(2, |v| v.min(2) as u32)
}

fn read_json(rel: &str) -> Value {
    paths::find_root()
        .ok()
        .and_then(|root| std::fs::read_to_string(root.join(rel)).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}

/// Pushes the parameters to the ground material when the quality level changes.
pub fn sync(
    quality: Option<Res<RenderQuality>>,
    handle: Option<Res<TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut done: Local<Option<String>>,
) {
    let (Some(handle), level) = (
        handle,
        quality.map_or("high".to_owned(), |q| q.level.clone()),
    ) else {
        return;
    };
    if done.as_ref() == Some(&level) {
        return;
    }
    let knob = knob_for(&read_json("data/fx/render_quality.json"), &level);
    let params = GdParams::from_json(&read_json("data/fx/ground_detail.json"), knob);
    if let Some(mut material) = materials.get_mut(&handle.0) {
        material.gd = params;
        *done = Some(level);
    }
}

pub fn add_plugin(app: &mut App) {
    bevy::shader::load_shader_library!(app, "ground_detail.wesl");
    app.add_systems(
        Update,
        sync.run_if(in_state(crate::app::AppState::Campaign)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_data_file_and_the_level() {
        let spec = json!({"macro": {"tone_amp": 0.2, "periods_px": [1.0, 2.0, 3.0]},
            "strips": {"width_min_m": 40.0}});
        let p = GdParams::from_json(&spec, 2);
        assert_eq!(p.a.x, 2.0);
        assert_eq!(p.a.y, 0.2);
        assert_eq!(p.e.xyz(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.c.z, 40.0);
    }

    #[test]
    fn default_is_off() {
        assert_eq!(GdParams::default().a.x, 0.0);
    }

    #[test]
    fn knob_reads_the_preset() {
        let presets =
            json!({"presets": {"low": {"ground_detail": 0}, "high": {"ground_detail": 2}}});
        assert_eq!(knob_for(&presets, "low"), 0);
        assert_eq!(knob_for(&presets, "high"), 2);
    }

    #[test]
    fn shipped_data_has_a_knob_per_preset() {
        let root = paths::find_root().expect("repo root");
        let presets: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("data/fx/render_quality.json")).unwrap(),
        )
        .unwrap();
        for level in ["low", "medium", "high", "ultra"] {
            assert!(
                presets["presets"][level]["ground_detail"].is_u64(),
                "{level}"
            );
        }
    }
}
