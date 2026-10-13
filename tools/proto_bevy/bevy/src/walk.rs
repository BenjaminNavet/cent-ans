//! `StandardMaterial` extended with the contract's procedural walk (vertex shader only).

use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

/// Shader-side parameters, see `shaders/walk.wesl`.
#[derive(ShaderType, Debug, Clone, Copy)]
pub struct WalkParams {
    /// `Time::elapsed_secs()` at the end of warm-up (the shader reads `globals.time`); a
    /// per-frame material write would re-queue every soldier each frame.
    pub t0: f32,
    pub hz: f32,
    pub bob_m: f32,
    pub sway_rad: f32,
}

/// Material extension: binding 100 keeps clear of the 0..99 range of `StandardMaterial`.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct WalkExt {
    #[uniform(100)]
    pub params: WalkParams,
}

pub type WalkMaterial = ExtendedMaterial<StandardMaterial, WalkExt>;

impl MaterialExtension for WalkExt {
    fn vertex_shader() -> ShaderRef {
        "bevy/shaders/walk.wesl".into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        "bevy/shaders/walk_prepass.wesl".into()
    }

    fn deferred_vertex_shader() -> ShaderRef {
        "bevy/shaders/walk_prepass.wesl".into()
    }
}
