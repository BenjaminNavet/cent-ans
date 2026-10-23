//! Ground material: [`TerrainMaterial`] and its WGSL (`terrain.wesl`). Replaces
//! `terrain.gdshader` + `relief_quadtree.gdshaderinc` for the colormap look (ADR 0142).

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::{ShaderDefVal, ShaderRef};

pub const SHADER_PATH: &str = "embedded://cent_ans_client/campaign/terrain/terrain.wesl";
/// Shadow caster (prepass vertex) of the ground: displaced like the main pass.
pub const SHADOW_SHADER_PATH: &str =
    "embedded://cent_ans_client/campaign/terrain/terrain_shadow.wesl";

/// Dev bisection switches of the ground shader (`CENT_ANS_PERF_OFF`, `campaign/perf_switch.rs`):
/// each one compiles a part of the fragment out (`PERF_<NAME>` shader def).
const PERF_DEFS: [(&str, &str); 16] = [
    ("tflat", "PERF_TFLAT"),
    ("tnoproc", "PERF_TNOPROC"),
    ("tnofields", "PERF_TNOFIELDS"),
    ("tnocoast", "PERF_TNOCOAST"),
    ("tnowx", "PERF_TNOWX"),
    ("tnocloud", "PERF_TNOCLOUD"),
    ("tnofp", "PERF_TNOFP"),
    ("tnofpsplat", "PERF_TNOFPSPLAT"),
    ("tonerole", "PERF_TONEROLE"),
    ("tnomacro", "PERF_TNOMACRO"),
    ("tnomicro", "PERF_TNOMICRO"),
    ("tnofar", "PERF_TNOFAR"),
    ("tnogdk", "PERF_TNOGDK"),
    ("tnohbtex", "PERF_TNOHBTEX"),
    ("tnosun", "PERF_TNOSUN"),
    ("tnogdmacro", "PERF_TNOGDMACRO"),
];

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

/// Pipeline key of the ground: texels per side of the ground cache layers (0: live ground).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TerrainMaterialKey {
    pub cache_texels: u32,
}

impl From<&TerrainMaterial> for TerrainMaterialKey {
    fn from(material: &TerrainMaterial) -> Self {
        TerrainMaterialKey {
            cache_texels: material.cache_texels,
        }
    }
}

/// `PERF_*` defs of the bisection switches turned off (`CENT_ANS_PERF_OFF`).
pub fn perf_defs() -> Vec<ShaderDefVal> {
    PERF_DEFS
        .iter()
        .filter(|(switch, _)| crate::campaign::perf_switch::off(switch))
        .map(|(_, def)| (*def).into())
        .collect()
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(TerrainMaterialKey)]
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
    // Metal allows 16 samplers per fragment stage (material + view). fog_mask, wash and landuse
    // have the same linear clamp sampler as terroir, so the shader reads them with `terroir_sampler`.
    // Likewise ab_dist, splat, coast_dist and wx_field (all linear, clamp-to-edge, read at mip 0):
    // only colormap, ground (bg_albedo) and terroir keep a sampler (3 owned, 13 in total).
    /// Fog-of-war coverage (R8, `FogOfWar::mask`); unbound when the fog is off.
    #[texture(5)]
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
    pub ab_dist: Option<Handle<Image>>,
    #[texture(13)]
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
    pub wash: Option<Handle<Image>>,
    /// Parcels materials of the painted fields (`hb_albedo` of `hb_ground.gdshaderinc`), sampled
    /// with `ground_sampler`; normal + roughness array (Rg8) beside it.
    #[texture(20, dimension = "2d_array")]
    pub parcels_albedo: Option<Handle<Image>>,
    #[texture(21, dimension = "2d_array")]
    pub parcels_normal: Option<Handle<Image>>,
    /// Land-use raster (`countryside::landuse`): R vine, G dryness, B bocage.
    #[texture(22)]
    pub landuse: Option<Handle<Image>>,
    /// Terroir mask (`terroir.rs`): R fields, G vine, B burnt, A pasture.
    #[texture(24)]
    #[sampler(25)]
    pub terroir: Option<Handle<Image>>,
    /// RV-D valley occlusion (R8, half grid), sampled with `terroir_sampler` (linear, clamp).
    #[texture(26)]
    pub occlusion: Option<Handle<Image>>,
    /// Coast band, glaciers, post adjustment, per-province weather flags (lot coastterr).
    #[uniform(27)]
    pub extras: TerrainExtras,
    /// Close-range ground detail (lot VQ cground, `ground_detail.wesl`).
    #[uniform(40)]
    pub gd: super::ground_detail::GdParams,
    /// Signed distance to the coast (R8, `coast_dist.png`, `(v - 128) / 2` px, > 0 on land).
    #[texture(28)]
    pub coast_dist: Option<Handle<Image>>,
    /// Slope-rule probe (`coast::bake_probe`, Rg8, half resolution), sampled with `coast_sampler`.
    #[texture(30)]
    pub coast_probe: Option<Handle<Image>>,
    /// Coast geology (`CoastLook.texture`, Rgba8, one texel per 16 px).
    #[texture(31)]
    pub coast_types: Option<Handle<Image>>,
    /// Per-province weather (`WeatherGround::field`, Rgba8: rain, snow, fog, storm).
    #[texture(32)]
    pub wx_field: Option<Handle<Image>>,
    /// Selected province (`WeatherGround::clear`, R8), sampled with `wx_sampler`.
    #[texture(34)]
    pub wx_clear: Option<Handle<Image>>,
    /// Ground cache atlas (ADR 0742, `ground_cache.rs`): baked regional albedo + forest share,
    /// normal offset; one layer per relief node.
    #[texture(41, dimension = "2d_array")]
    pub cache_albedo: Option<Handle<Image>>,
    #[texture(42, dimension = "2d_array")]
    pub cache_dn: Option<Handle<Image>>,
    /// Texels per side of a cache layer, 0 when the ground is computed live.
    pub cache_texels: u32,
}

/// Uniform block `TerrainExtras` of `terrain.wesl` (binding 27).
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TerrainExtras {
    pub coast: super::coast::CoastParams,
    pub glacier: super::coast::GlacierParams,
    /// Contrast / saturation of the Godot campaign environment (`adjustment_contrast` 1.06,
    /// `adjustment_saturation` 1.08), 0 in z: the adjustment now runs as a post-tonemap pass
    /// (`post.rs`), not in this shader (the fields stay for the dump).
    pub post: Vec4,
    /// Per-province weather: 1 when the field / selection textures are bound and some province
    /// has weather (`WeatherGround::any`), 1 when a selection mask is bound.
    pub wx: Vec4,
}

impl Default for TerrainExtras {
    fn default() -> Self {
        TerrainExtras {
            coast: Default::default(),
            glacier: Default::default(),
            post: Vec4::new(super::post::CONTRAST, super::post::SATURATION, 0.0, 0.0),
            wx: Vec4::ZERO,
        }
    }
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
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Skirts are two-sided quads.
        descriptor.primitive.cull_mode = None;
        // `terrain.wesl` also holds the ground-cache bake entry points: name the main ones.
        descriptor.vertex.entry_point = Some("vertex".into());
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.entry_point = Some("fragment".into());
        }
        let texels = key.bind_group_data.cache_texels;
        let texels_def = ShaderDefVal::UInt("GC_TEXELS".into(), texels.max(1));
        descriptor.vertex.shader_defs.push(texels_def.clone());
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.extend(perf_defs());
            fragment.shader_defs.push(texels_def);
            if texels > 0 {
                fragment.shader_defs.push("GROUND_CACHE".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Metal allows 16 samplers per fragment stage and the forward view takes 10: the ground
    /// material may own 4 at most (a 17th binding crashed the campaign at first render).
    #[test]
    fn terrain_shader_owns_few_samplers() {
        let source = include_str!("terrain.wesl");
        let declared = source.matches(": sampler;").count();
        assert!(declared <= 4, "{declared} samplers in terrain.wesl");
        let bound = include_str!("material.rs")
            .lines()
            .filter(|l| l.trim_start().starts_with("#[sampler("))
            .count();
        assert_eq!(bound, declared);
    }

    #[test]
    fn sun_points_up_towards_the_scene_azimuth() {
        let dir = sun_direction(34.0);
        assert!((dir.length() - 1.0).abs() < 1e-5);
        assert!((dir.y - 34f32.to_radians().sin()).abs() < 1e-5);
        assert!(dir.x > 0.0 && dir.z > 0.0 && dir.z > dir.x);
    }
}
