//! Materials of the province overlays, draped on the dynamic exaggerated relief (`relief.wesl`,
//! see `water/drape.rs`). Meshes carry the altitude in metres in `position.y`; the vertex shader
//! converts it to the displayed height, so the zoom-driven relief needs no rebuild.

use bevy::asset::uuid_handle;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::{Shader, ShaderRef};

use crate::campaign::terrain::relief_gpu::ReliefParams;
use crate::campaign::water::drape::Drape;

const FILL_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0101");
const BORDER_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0102");

/// Registers the embedded shaders (no-op without the render plugin).
pub fn register_shaders(app: &mut App) {
    let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() else {
        return;
    };
    for (handle, source, path) in [
        (
            FILL_SHADER,
            include_str!("shaders/fill.wesl"),
            "provinces/fill.wesl",
        ),
        (
            BORDER_SHADER,
            include_str!("shaders/border.wesl"),
            "provinces/border.wesl",
        ),
    ] {
        if let Err(error) = shaders.insert(handle.id(), Shader::from_wesl(source, path)) {
            error!("provinces: shader {path}: {error:?}");
        }
    }
}

/// Uniform block shared by both overlay shaders.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct OverlayParams {
    /// Multiplied into the texel / vertex colour (alpha = fade with the zoom).
    pub tint: Vec4,
    /// World units above the draped ground, constant and per world unit of eye distance.
    pub lift_base: f32,
    pub lift_per_distance: f32,
    pub pad0: f32,
    pub pad1: f32,
}

impl OverlayParams {
    pub fn new(lift_base: f32, lift_per_distance: f32) -> Self {
        Self {
            tint: Vec4::ONE,
            lift_base,
            lift_per_distance,
            pad0: 0.0,
            pad1: 0.0,
        }
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.tint.w = alpha;
    }
}

/// Province colour wash (one flat colour per province from the fill texture).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ProvinceFillMaterial {
    #[uniform(0)]
    pub params: OverlayParams,
    #[uniform(1)]
    pub relief: ReliefParams,
    #[texture(2, sample_type = "float", filterable = false)]
    pub relief_floor: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub fill: Handle<Image>,
}

/// Border ribbons coloured per vertex.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ProvinceBorderMaterial {
    #[uniform(0)]
    pub params: OverlayParams,
    #[uniform(1)]
    pub relief: ReliefParams,
    #[texture(2, sample_type = "float", filterable = false)]
    pub relief_floor: Handle<Image>,
}

macro_rules! drape_impl {
    ($ty:ty) => {
        impl Drape for $ty {
            fn set_relief(&mut self, params: ReliefParams, floor: &Handle<Image>) {
                self.relief = params;
                self.relief_floor = floor.clone();
            }
            fn relief(&self) -> (ReliefParams, &Handle<Image>) {
                (self.relief, &self.relief_floor)
            }
        }
    };
}
drape_impl!(ProvinceFillMaterial);
drape_impl!(ProvinceBorderMaterial);

macro_rules! overlay_material {
    ($ty:ty, $shader:expr) => {
        impl Material for $ty {
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
        }
    };
}
overlay_material!(ProvinceFillMaterial, FILL_SHADER);
overlay_material!(ProvinceBorderMaterial, BORDER_SHADER);
