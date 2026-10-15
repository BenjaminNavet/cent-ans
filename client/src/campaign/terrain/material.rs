//! Ground material: [`TerrainMaterial`] and its WGSL (`terrain.wesl`). Replaces
//! `terrain.gdshader` + `relief_quadtree.gdshaderinc` for the colormap look (ADR 0142).

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub const SHADER_PATH: &str = "embedded://cent_ans_client/campaign/terrain/terrain.wesl";

/// Uniform block of `terrain.wesl` (`TerrainParams`).
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TerrainParams {
    pub sun_dir: Vec4,
    pub sun_color: Vec4,
    pub ambient: Vec4,
    pub fog: Vec4,
    /// Vertical scale s, local gain g, squash weight c, edge haze width.
    pub scale: Vec4,
    /// World width, height, fog begin, colormap flag.
    pub map: Vec4,
    /// Edge haze colour, fog end.
    pub edge: Vec4,
}

impl Default for TerrainParams {
    fn default() -> Self {
        let sun = sun_direction(34.0);
        let fog = Color::srgb(0.8, 0.79, 0.74).to_linear();
        let edge = Color::srgb(0.74, 0.76, 0.76).to_linear();
        // Light (1, 0.95, 0.86) x 1.1 and ambient (0.9, 0.85, 0.76) x 0.55 of campaign_map.tscn.
        TerrainParams {
            sun_dir: sun.extend(0.0),
            sun_color: Vec4::new(1.1, 1.045, 0.946, 0.0),
            ambient: Vec4::new(0.495, 0.4675, 0.418, 0.0),
            fog: Vec4::new(fog.red, fog.green, fog.blue, 0.5),
            scale: Vec4::new(super::profile::HEIGHT_SCALE, 0.0, 0.0, 14.0),
            map: Vec4::new(1.0, 1.0, 800.0, 0.0),
            edge: Vec4::new(edge.red, edge.green, edge.blue, 6000.0),
        }
    }
}

/// Unit vector towards the sun at `elevation_deg`, azimuth of the Godot `Sun` node
/// (campaign_map.tscn: light travels towards -x, -z; horizontal direction (0.5, 0.866)).
pub fn sun_direction(elevation_deg: f32) -> Vec3 {
    let elevation = elevation_deg.to_radians();
    let horizontal = Vec2::new(0.353_553, 0.612_372).normalize();
    Vec3::new(
        horizontal.x * elevation.cos(),
        elevation.sin(),
        horizontal.y * elevation.cos(),
    )
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct TerrainMaterial {
    #[uniform(0)]
    pub params: TerrainParams,
    #[texture(1)]
    #[sampler(2)]
    pub colormap: Option<Handle<Image>>,
    #[uniform(3)]
    pub season: super::season::SeasonParams,
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    // The default prepass / shadow pass would draw the undisplaced mesh.
    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Skirts are two-sided quads.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_points_up_towards_the_scene_azimuth() {
        let dir = sun_direction(34.0);
        assert!((dir.length() - 1.0).abs() < 1e-5);
        assert!((dir.y - 34f32.to_radians().sin()).abs() < 1e-5);
        assert!(dir.x > 0.0 && dir.z > 0.0 && dir.z > dir.x);
    }
}
