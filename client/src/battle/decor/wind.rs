//! Wind sway: an `ExtendedMaterial` whose vertex stage (`wind.wesl`) moves grass cards and tree
//! crowns with the time (two sines and a slow gust, values in `battle_decor_look.json`).
//! Port of the vertex part of game/shaders/battle_grass.gdshader and the wind of
//! battle_vegetation.gd.

use bevy::asset::embedded_asset;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::look::{WindLook, wind_direction};

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct WindParams {
    pub sway: Vec4,
    pub shape: Vec4,
    /// Rectangle (x, z, width, depth) covered by the flatten map; zero width = no map.
    pub flatten_rect: Vec4,
}

impl WindParams {
    /// `mode`: 0 grass card (weight from the card's v), 1 crown (weight from the height).
    pub fn new(wind: &WindLook, mode: f32, height_ref: f32) -> Self {
        let dir = wind_direction(wind);
        WindParams {
            sway: Vec4::new(
                wind.amplitude_m,
                wind.speed,
                wind.gust_amplitude_m,
                wind.gust_speed,
            ),
            shape: Vec4::new(dir.x, dir.y, mode, height_ref),
            flatten_rect: Vec4::ZERO,
        }
    }
}

impl Default for WindParams {
    fn default() -> Self {
        WindParams {
            sway: Vec4::ZERO,
            shape: Vec4::new(1.0, 0.0, 0.0, 1.0),
            flatten_rect: Vec4::ZERO,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct WindExtension {
    #[uniform(100)]
    pub params: WindParams,
    /// RG8 map of the flattened (R) and bloodied (G) grass, see `flatten.rs`.
    #[texture(101)]
    #[sampler(102)]
    pub flatten: Option<Handle<Image>>,
}

impl MaterialExtension for WindExtension {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/wind.wesl".into()
    }
    /// No depth prepass: Bevy's default prepass vertex stage does not sway, so with SSAO on
    /// (which needs the prepass) the still cards' depth hid the ground behind the swaying ones
    /// and the sky showed through. Shadows keep their own (still) pass.
    fn enable_prepass() -> bool {
        false
    }
}

pub type WindMaterial = ExtendedMaterial<StandardMaterial, WindExtension>;

pub struct WindPlugin;

impl Plugin for WindPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "wind.wesl");
        app.add_plugins(MaterialPlugin::<WindMaterial>::default());
    }
}
