//! Cloud layer of the sky dome (`sky_clouds.wesl`): an `ExtendedMaterial` over the unlit gradient
//! dome. Godot's sky had HDRI clouds; here procedural cumulus drift with the time.

use bevy::asset::embedded_asset;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use sim_battle::Weather;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct CloudExtension {
    /// x: cover, y: drift speed, z: darkness of the underside.
    #[uniform(100)]
    pub shape: Vec4,
    /// xyz: direction towards the sun.
    #[uniform(100)]
    pub sun: Vec4,
}

impl CloudExtension {
    /// Cloud set of a weather and the direction towards the sun.
    pub fn new(weather: Weather, to_sun: Vec3) -> Self {
        let shape = match weather {
            Weather::Clear => Vec4::new(0.45, 0.012, 0.28, 0.0),
            Weather::Fog => Vec4::new(0.0, 0.0, 0.0, 0.0),
            Weather::Rain => Vec4::new(1.0, 0.03, 0.5, 0.0),
            Weather::Snow => Vec4::new(0.85, 0.02, 0.3, 0.0),
        };
        CloudExtension {
            shape,
            sun: to_sun.extend(0.0),
        }
    }
}

impl MaterialExtension for CloudExtension {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/sky_clouds.wesl".into()
    }
}

pub type CloudMaterial = ExtendedMaterial<StandardMaterial, CloudExtension>;

pub struct SkyCloudsPlugin;

impl Plugin for SkyCloudsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "sky_clouds.wesl");
        app.add_plugins(MaterialPlugin::<CloudMaterial>::default());
    }
}
