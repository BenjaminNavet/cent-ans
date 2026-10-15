//! Portulan sea plane of the parchment view (`sea.wesl`, port of `parchment_sea.gdshaderinc`):
//! a plane at `SEA_Y` (just above the 3D sea, so it sorts after it) covering the sea plane's
//! extent, blended with the parchment weight. Wind roses come from the decor placement.

use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat,
};
use bevy::shader::{Shader, ShaderRef};

use super::decor::CoastRaster;

pub const SEA_SHADER: Handle<Shader> = uuid_handle!("2b7d4a1c-58e3-4f0e-9c61-7a0d3e5f0001");
/// Height of the plane (world units): above the sea at 0, below any coast worth drawing.
pub const SEA_Y: f32 = 0.6;
/// Plane extent around the map, as the 3D sea (`water::SEA_MARGIN_FACTOR`).
pub const MARGIN_FACTOR: f32 = 4.0;
/// Strength of the rhumb lines (`pm_rhumb_alpha`).
pub const RHUMB_ALPHA: f32 = 0.4;

/// Registers the embedded shader (no-op without the render plugin, e.g. in tests).
pub fn register_shader(app: &mut App) {
    let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() else {
        return;
    };
    let source = include_str!("sea.wesl");
    if let Err(error) = shaders.insert(
        SEA_SHADER.id(),
        Shader::from_wesl(source, "parchment/sea.wesl"),
    ) {
        error!("parchment: shader sea.wesl: {error:?}");
    }
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct SeaParams {
    pub roses: [Vec4; 4],
    /// Map size, parchment weight, rose count.
    pub map: Vec4,
    /// Coast scale, rhumb alpha, painted rose, has coast raster.
    pub misc: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct ParchmentSeaMaterial {
    #[uniform(0)]
    pub params: SeaParams,
    #[texture(1)]
    #[sampler(2)]
    pub coast_dist: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub rose: Option<Handle<Image>>,
}

impl Material for ParchmentSeaMaterial {
    fn vertex_shader() -> ShaderRef {
        SEA_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        SEA_SHADER.into()
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

/// Uniform block for the given roses (at most 4) and weight.
pub fn params(
    map_size: Vec2,
    roses: &[(Vec2, f32)],
    weight: f32,
    painted: bool,
    coast: bool,
    coast_scale: f32,
) -> SeaParams {
    let mut slots = [Vec4::ZERO; 4];
    for (slot, (p, r)) in slots.iter_mut().zip(roses) {
        *slot = Vec4::new(p.x, p.y, *r, 1.0);
    }
    SeaParams {
        roses: slots,
        map: Vec4::new(map_size.x, map_size.y, weight, roses.len().min(4) as f32),
        misc: Vec4::new(
            coast_scale,
            RHUMB_ALPHA,
            if painted { 1.0 } else { 0.0 },
            if coast { 1.0 } else { 0.0 },
        ),
    }
}

/// GPU copy of the coast raster (R8, linear filtering, clamped).
pub fn coast_image(coast: &CoastRaster) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: coast.width,
            height: coast.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        coast.data.clone(),
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// The sea plane entity.
#[derive(Component)]
pub struct ParchmentSea;

/// Spawns the plane; returns its material.
pub fn spawn_plane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ParchmentSeaMaterial>,
    map_size: Vec2,
    material: ParchmentSeaMaterial,
) -> Handle<ParchmentSeaMaterial> {
    let extent = map_size * (1.0 + MARGIN_FACTOR * 2.0);
    let mesh = meshes.add(Mesh::from(
        Plane3d::default().mesh().size(extent.x, extent.y),
    ));
    let handle = materials.add(material);
    commands.spawn((
        Name::new("ParchmentSea"),
        ParchmentSea,
        Mesh3d(mesh),
        MeshMaterial3d(handle.clone()),
        Transform::from_xyz(map_size.x * 0.5, SEA_Y, map_size.y * 0.5),
        NotShadowCaster,
        Visibility::Hidden,
        DespawnOnExit(crate::app::AppState::Campaign),
    ));
    handle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_pack_roses_and_weight() {
        let p = params(
            Vec2::new(100.0, 50.0),
            &[(Vec2::new(10.0, 20.0), 62.0)],
            0.5,
            true,
            true,
            2.0,
        );
        assert_eq!(p.roses[0], Vec4::new(10.0, 20.0, 62.0, 1.0));
        assert_eq!(p.roses[1], Vec4::ZERO);
        assert_eq!(p.map, Vec4::new(100.0, 50.0, 0.5, 1.0));
        assert_eq!(p.misc.z, 1.0);
    }
}
