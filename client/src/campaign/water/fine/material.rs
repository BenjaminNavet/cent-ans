//! Materials of the fine rivers and roads (`river_fine.gdshader`, `road_fine.gdshader`), draped
//! with the shared relief module (`terrain/relief.wesl`, binding pattern of `water/drape.rs`).

use bevy::asset::uuid_handle;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::{Shader, ShaderRef};
use serde_json::Value;

use super::super::drape::Drape;
use super::super::materials::linear;
use crate::campaign::terrain::relief_gpu::ReliefParams;

const RIVER_FINE_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0f01");
const ROAD_FINE_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0f02");

/// Registers the embedded shaders (no-op without the render plugin, e.g. in tests).
pub fn register_shaders(app: &mut App) {
    let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() else {
        return;
    };
    for (handle, source, path) in [
        (
            RIVER_FINE_SHADER,
            include_str!("../shaders/river_fine.wesl"),
            "water/river_fine.wesl",
        ),
        (
            ROAD_FINE_SHADER,
            include_str!("../shaders/road_fine.wesl"),
            "water/road_fine.wesl",
        ),
    ] {
        let source = super::super::drape::with_relief(source);
        if let Err(error) = shaders.insert(handle.id(), Shader::from_wesl(source, path)) {
            error!("water: shader {path}: {error:?}");
        }
    }
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct FineRiverParams {
    /// Disc covered by the fine rendering (centre x, z; radius; near-tier weight).
    pub fine_zone: Vec4,
    pub order_px_lo: Vec4,
    pub order_px_hi: Vec4,
    pub shallow_color: Vec4,
    pub deep_color: Vec4,
    pub brackish_color: Vec4,
    pub sky_color: Vec4,
    pub foam_color: Vec4,
    pub sand_color: Vec4,
    pub gravel_color: Vec4,
    pub mud_color: Vec4,
    pub reed_color: Vec4,
    pub reed_dry_color: Vec4,
    pub fine_weight: f32,
    pub min_px: f32,
    pub thin_fade: f32,
    pub depth_bias: f32,
    /// Small rivers: rank 3 visible under `order_distance`, x2 per rank above.
    pub order_distance: f32,
    pub tidal_band: f32,
    pub wetland_band: f32,
    pub divagating_band: f32,
    pub band_min_m: f32,
    pub meters_per_unit: f32,
    pub tide_period: f32,
    pub tide_amount: f32,
    pub flow_speed: f32,
    pub ripple_scale: f32,
    pub ripple_strength: f32,
    pub sky_reflect: f32,
    pub shallow_alpha: f32,
    pub gen_width_scale: f32,
    pub gen_width_ref: f32,
    /// Width (world units) at which the water takes the deep colour (replaces the depth buffer).
    pub deep_width: f32,
    pub debug_flat: f32,
}

impl Default for FineRiverParams {
    fn default() -> Self {
        Self {
            fine_zone: Vec4::new(0.0, 0.0, -1.0, 0.0),
            order_px_lo: Vec4::ONE,
            order_px_hi: Vec4::ONE,
            shallow_color: linear([0.22, 0.33, 0.34], 1.0),
            deep_color: linear([0.07, 0.16, 0.21], 1.0),
            brackish_color: linear([0.33, 0.37, 0.30], 1.0),
            sky_color: linear([0.62, 0.72, 0.80], 1.0),
            foam_color: linear([0.80, 0.82, 0.78], 1.0),
            sand_color: linear([0.62, 0.55, 0.42], 1.0),
            gravel_color: linear([0.50, 0.47, 0.42], 1.0),
            mud_color: linear([0.26, 0.22, 0.16], 1.0),
            reed_color: linear([0.36, 0.38, 0.18], 1.0),
            reed_dry_color: linear([0.55, 0.48, 0.30], 1.0),
            fine_weight: 1.0,
            min_px: 1.2,
            thin_fade: 0.7,
            depth_bias: 0.0015,
            order_distance: 30.0,
            tidal_band: 0.8,
            wetland_band: 0.9,
            divagating_band: 0.35,
            band_min_m: 25.0,
            meters_per_unit: 719.0,
            tide_period: 90.0,
            tide_amount: 0.35,
            flow_speed: 0.35,
            ripple_scale: 90.0,
            ripple_strength: 0.08,
            sky_reflect: 0.75,
            shallow_alpha: 0.9,
            gen_width_scale: 0.0,
            gen_width_ref: 0.14,
            deep_width: 0.25,
            debug_flat: 0.0,
        }
    }
}

impl FineRiverParams {
    /// `RiversRenderer.apply_fine_display`: generalised width, then the `fine` block of
    /// `river_display.json` over its top-level keys.
    pub fn from_display(display: &Value, meters_per_unit: f32) -> Self {
        let mut p = Self {
            meters_per_unit,
            ..Self::default()
        };
        if let Some(g) = display.get("generalised") {
            p.gen_width_scale = g.get("width_scale").and_then(Value::as_f64).unwrap_or(0.0) as f32;
            let ref_m = g
                .get("width_ref_m")
                .and_then(Value::as_f64)
                .unwrap_or(100.0) as f32;
            p.gen_width_ref = ref_m / meters_per_unit.max(1e-3);
        }
        let fine = display.get("fine").cloned().unwrap_or(Value::Null);
        let pick = |key: &str| fine.get(key).or_else(|| display.get(key));
        let rgb = |key: &str| -> Option<Vec4> {
            let a = pick(key)?.as_array()?;
            (a.len() == 3).then(|| {
                let f = |i: usize| a[i].as_f64().unwrap_or(0.0) as f32;
                linear([f(0), f(1), f(2)], 1.0)
            })
        };
        if let Some(c) = rgb("shallow_color") {
            p.shallow_color = c;
        }
        if let Some(c) = rgb("deep_color") {
            p.deep_color = c;
        }
        if let Some(c) = rgb("sky_color") {
            p.sky_color = c;
        }
        let num = |key: &str| pick(key).and_then(Value::as_f64).map(|v| v as f32);
        if let Some(v) = num("thin_fade") {
            p.thin_fade = v;
        }
        if let Some(v) = num("sky_reflect") {
            p.sky_reflect = v;
        }
        if let Some(v) = num("min_px") {
            p.min_px = v;
        }
        if let Some(v) = num("shallow_alpha") {
            p.shallow_alpha = v;
        }
        if let Some(s) = fine.get("order_px_scale").and_then(Value::as_array)
            && s.len() == 8
        {
            let f = |i: usize| s[i].as_f64().unwrap_or(1.0) as f32;
            p.order_px_lo = Vec4::new(f(0), f(1), f(2), f(3));
            p.order_px_hi = Vec4::new(f(4), f(5), f(6), f(7));
        }
        p
    }
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct FineRoadParams {
    pub fine_zone: Vec4,
    pub dirt_color: Vec4,
    pub main_color: Vec4,
    pub rut_color: Vec4,
    pub paved_color: Vec4,
    pub joint_color: Vec4,
    pub grass_edge: Vec4,
    pub bank_color: Vec4,
    pub fine_weight: f32,
    pub min_px: f32,
    pub main_min_px: f32,
    pub depth_bias: f32,
    pub meters_per_unit: f32,
    pub debug_flat: f32,
}

impl Default for FineRoadParams {
    fn default() -> Self {
        Self {
            fine_zone: Vec4::new(0.0, 0.0, -1.0, 0.0),
            dirt_color: linear([0.47, 0.38, 0.25], 1.0),
            main_color: linear([0.60, 0.51, 0.36], 1.0),
            rut_color: linear([0.32, 0.25, 0.16], 1.0),
            paved_color: linear([0.52, 0.50, 0.46], 1.0),
            joint_color: linear([0.30, 0.28, 0.25], 1.0),
            grass_edge: linear([0.30, 0.31, 0.16], 1.0),
            bank_color: linear([0.20, 0.22, 0.12], 1.0),
            fine_weight: 1.0,
            min_px: 2.2,
            main_min_px: 3.2,
            depth_bias: 0.0012,
            meters_per_unit: 719.0,
            debug_flat: 0.0,
        }
    }
}

macro_rules! fine_material {
    ($name:ident, $params:ty, $shader:expr) => {
        #[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
        pub struct $name {
            #[uniform(0)]
            pub params: $params,
            #[uniform(1)]
            pub relief: ReliefParams,
            #[texture(2, sample_type = "float", filterable = false)]
            pub relief_floor: Handle<Image>,
        }

        impl Drape for $name {
            fn set_relief(&mut self, params: ReliefParams, floor: &Handle<Image>) {
                self.relief = params;
                self.relief_floor = floor.clone();
            }
            fn relief(&self) -> (ReliefParams, &Handle<Image>) {
                (self.relief, &self.relief_floor)
            }
        }

        impl Material for $name {
            fn vertex_shader() -> ShaderRef {
                $shader.into()
            }
            fn fragment_shader() -> ShaderRef {
                $shader.into()
            }
            fn alpha_mode(&self) -> AlphaMode {
                AlphaMode::Blend
            }
            fn enable_prepass() -> bool {
                false
            }
            fn enable_shadows() -> bool {
                false
            }
            fn depth_bias(&self) -> f32 {
                // `render_priority = 1` of the Godot river material.
                1.0
            }
            fn specialize(
                _pipeline: &bevy::pbr::MaterialPipeline,
                descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
                _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
                _key: bevy::pbr::MaterialPipelineKey<Self>,
            ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
                // Ribbons have no fixed winding after draping: two-sided (`cull_disabled`).
                descriptor.primitive.cull_mode = None;
                Ok(())
            }
        }
    };
}

fine_material!(FineRiverMaterial, FineRiverParams, RIVER_FINE_SHADER);
fine_material!(FineRoadMaterial, FineRoadParams, ROAD_FINE_SHADER);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_fine_block_overrides_the_defaults() {
        let display: Value = serde_json::json!({
            "min_px": 1.6,
            "generalised": {"width_scale": 5.0, "width_ref_m": 100.0},
            "fine": {"min_px": 4.0, "order_px_scale": [0.45, 0.55, 0.65, 0.8, 1.0, 1.2, 1.5, 1.75], "thin_fade": 0.35}
        });
        let p = FineRiverParams::from_display(&display, 200.0);
        assert_eq!(p.min_px, 4.0);
        assert_eq!(p.thin_fade, 0.35);
        assert_eq!(p.order_px_hi.w, 1.75);
        assert_eq!(p.gen_width_scale, 5.0);
        assert!((p.gen_width_ref - 0.5).abs() < 1e-6);
    }

    #[test]
    fn repository_river_display_parses() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let text = std::fs::read_to_string(root.join("data/map/river_display.json")).unwrap();
        let display: Value = serde_json::from_str(&text).unwrap();
        let p = FineRiverParams::from_display(&display, 719.0);
        assert!(p.min_px > 1.0 && p.gen_width_scale > 0.0);
    }
}
