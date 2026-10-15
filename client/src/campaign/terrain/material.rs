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
/// Shadow caster (prepass vertex) of the ground: displaced like the main pass.
pub const SHADOW_SHADER_PATH: &str =
    "embedded://cent_ans_client/campaign/terrain/terrain_shadow.wesl";

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
    /// Colour grade of the Godot environment: saturation, contrast, shadow saturation, strength.
    pub grade: Vec4,
    pub lift: Vec4,
    pub highlights: Vec4,
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
            ambient: Vec4::new(0.495, 0.4675, 0.418, 1.0),
            fog: Vec4::new(fog.red, fog.green, fog.blue, 0.5),
            scale: Vec4::new(super::profile::HEIGHT_SCALE, 0.0, 0.0, 14.0),
            map: Vec4::new(1.0, 1.0, 800.0, 0.0),
            edge: Vec4::new(edge.red, edge.green, edge.blue, 6000.0),
            grade: Vec4::new(1.0, 1.0, 1.0, 0.0),
            lift: Vec4::ZERO,
            highlights: Vec4::ONE,
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
    #[uniform(4)]
    pub look: super::look::LookParams,
    /// Fog-of-war coverage (R8, `FogOfWar::mask`); unbound when the fog is off.
    #[texture(5)]
    #[sampler(6)]
    pub fog_mask: Option<Handle<Image>>,
    /// Regional ground tables (`ground.rs`, TX).
    #[uniform(7)]
    pub ground: super::ground::GroundParams,
    #[texture(8, dimension = "2d_array")]
    #[sampler(9)]
    pub bg_albedo: Option<Handle<Image>>,
    #[texture(10, dimension = "2d_array")]
    pub bg_normal: Option<Handle<Image>>,
    /// R: biome A + 16 B, G: distance to the neighbour biome.
    #[texture(11)]
    #[sampler(12)]
    pub ab_dist: Option<Handle<Image>>,
    #[texture(13)]
    #[sampler(14)]
    pub splat: Option<Handle<Image>>,
    #[texture(15, dimension = "2d_array")]
    pub micro_albedo: Option<Handle<Image>>,
    /// Painted fields table (`fields.rs`).
    #[uniform(16)]
    pub fields: super::fields::FieldParams,
    /// Coarse altitudes (m, 16 px blocks) for the drawn mountains / trees of the parchment.
    #[texture(17, sample_type = "float", filterable = false)]
    pub height_lod: Option<Handle<Image>>,
    /// Owner colour per 4 px block (faction wash of the parchment).
    #[texture(18)]
    #[sampler(19)]
    pub wash: Option<Handle<Image>>,
    /// Parcels materials of the painted fields (`hb_albedo` of `hb_ground.gdshaderinc`), sampled
    /// with `ground_sampler`; normal + roughness array (Rg8) beside it.
    #[texture(20, dimension = "2d_array")]
    pub parcels_albedo: Option<Handle<Image>>,
    #[texture(21, dimension = "2d_array")]
    pub parcels_normal: Option<Handle<Image>>,
    /// Land-use raster (`countryside::landuse`): R vine, G dryness, B bocage.
    #[texture(22)]
    #[sampler(23)]
    pub landuse: Option<Handle<Image>>,
    /// Terroir mask (`terroir.rs`): R fields, G vine, B burnt, A pasture.
    #[texture(24)]
    #[sampler(25)]
    pub terroir: Option<Handle<Image>>,
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    // No depth / normal prepass for the camera (the campaign camera has none).
    fn enable_prepass() -> bool {
        false
    }

    // The relief casts sun shadows (Godot `relief_shadow_cascades`): the shadow pass draws the
    // ground displaced by `terrain_shadow.wesl` (the default prepass would draw the flat mesh).
    fn enable_shadows() -> bool {
        true
    }

    fn prepass_vertex_shader() -> ShaderRef {
        SHADOW_SHADER_PATH.into()
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
