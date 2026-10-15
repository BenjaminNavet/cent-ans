//! Stone of the siege walls and towers: the player's `castle_wall_varriation` texture
//! (`game/assets/textures/battle/castle_wall_varriation_{diff,nor}.jpg`, full 2048 px) projected in
//! world triplanar exactly as `BattleSiege._textured("stone", ...)` of Godot maps it: 0.22
//! repetitions per metre, blend sharpness 4, normal strength 0.8, roughness 0.92, tint `STONE`
//! (`STONE_DARK.lerp(STONE, ratio)` as a wall is battered). The generic building atlas (512 px
//! layers, 3 m tile) gave a coarser, lighter stone; it keeps the houses, the rubble and the roofs.

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler};
use bevy::image::{ImageSamplerDescriptor, TextureFormatPixelInfo};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use crate::battle::decor::ground::{mip_chain, mip_count};

/// Godot `STONE` / `STONE_DARK` of `battle_siege.gd`.
pub const STONE: [f32; 3] = [0.95, 0.9, 0.8];
pub const STONE_DARK: [f32; 3] = [0.55, 0.52, 0.47];
/// Repetitions per metre (`TEXTURES["stone"]`).
pub const STONE_SCALE: f32 = 0.22;
pub const TRIPLANAR_SHARPNESS: f32 = 4.0;
pub const NORMAL_SCALE: f32 = 0.8;
pub const ROUGHNESS: f32 = 0.92;
const DIFFUSE: &str = "textures/battle/castle_wall_varriation_diff.jpg";
const NORMAL: &str = "textures/battle/castle_wall_varriation_nor.jpg";

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct StoneExt {
    /// x: repetitions per metre, y: blend sharpness, z: normal strength.
    #[uniform(100)]
    pub params: Vec4,
    #[texture(101)]
    #[sampler(102)]
    pub diffuse: Handle<Image>,
    #[texture(103)]
    #[sampler(104)]
    pub normal: Handle<Image>,
}

impl MaterialExtension for StoneExt {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/siege/stone.wesl".into()
    }
}

pub type SiegeStoneMaterial = ExtendedMaterial<StandardMaterial, StoneExt>;

/// Colour of a wall holding `ratio` of its hit points (Godot `STONE_DARK.lerp(STONE, ratio)`).
pub fn wall_tint(ratio: f32) -> [f32; 3] {
    let k = ratio.clamp(0.0, 1.0);
    std::array::from_fn(|i| STONE_DARK[i] + (STONE[i] - STONE_DARK[i]) * k)
}

/// The two textures (loaded once) and the cached materials by damage step.
#[derive(Resource)]
pub struct StoneTextures {
    diffuse: Handle<Image>,
    normal: Handle<Image>,
    pub converted: bool,
    /// False when the texture files are missing (the atlas masonry stays the fallback).
    pub present: bool,
}

impl StoneTextures {
    /// One material tinted by `ratio` (1 = intact).
    pub fn material(&self, ratio: f32) -> SiegeStoneMaterial {
        let tint = wall_tint(ratio);
        ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::linear_rgb(tint[0], tint[1], tint[2]),
                perceptual_roughness: ROUGHNESS,
                metallic: 0.0,
                reflectance: 0.3,
                ..default()
            },
            extension: StoneExt {
                params: Vec4::new(STONE_SCALE, TRIPLANAR_SHARPNESS, NORMAL_SCALE, 0.0),
                diffuse: self.diffuse.clone(),
                normal: self.normal.clone(),
            },
        }
    }
}

/// Repeating, mip-mapped, anisotropic version of a loaded square RGBA8 image.
fn mipmap_repeat(image: &mut Image) -> bool {
    let Some(data) = image.data.take() else {
        return false;
    };
    let size = image.width();
    let bytes = image
        .texture_descriptor
        .format
        .pixel_size()
        .unwrap_or_default();
    if size == 0 || !size.is_power_of_two() || image.height() != size || bytes != 4 {
        image.data = Some(data);
        return false;
    }
    image.data = Some(mip_chain(&data, size as usize));
    image.texture_descriptor.mip_level_count = mip_count(size);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    true
}

fn load_textures(mut commands: Commands, assets: Res<AssetServer>) {
    let present = crate::paths::find_root()
        .map(|root| {
            let dir = crate::paths::asset_root(&root);
            dir.join(DIFFUSE).is_file() && dir.join(NORMAL).is_file()
        })
        .unwrap_or(false);
    commands.insert_resource(StoneTextures {
        diffuse: if present {
            assets.load(DIFFUSE)
        } else {
            Handle::default()
        },
        normal: if present {
            assets
                .load_builder()
                .with_settings(|s: &mut ImageLoaderSettings| s.is_srgb = false)
                .load(NORMAL)
        } else {
            Handle::default()
        },
        converted: false,
        present,
    });
}

/// Once loaded, gives both textures their repeat sampler and mip chain.
fn convert_textures(mut textures: ResMut<StoneTextures>, mut images: ResMut<Assets<Image>>) {
    if textures.converted || !textures.present {
        return;
    }
    for handle in [textures.diffuse.clone(), textures.normal.clone()] {
        if images.get(&handle).is_none() {
            return;
        }
    }
    for handle in [textures.diffuse.clone(), textures.normal.clone()] {
        if let Some(mut image) = images.get_mut(&handle) {
            let _ = mipmap_repeat(&mut image);
        }
    }
    let _ = RenderAssetUsages::default();
    textures.converted = true;
}

pub struct StonePlugin;

impl Plugin for StonePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "stone.wesl");
        app.add_plugins(MaterialPlugin::<SiegeStoneMaterial>::default())
            .add_systems(Startup, load_textures)
            .add_systems(Update, convert_textures);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_battered_wall_darkens_from_stone_to_stone_dark() {
        assert_eq!(wall_tint(1.0), STONE);
        assert_eq!(wall_tint(0.0), STONE_DARK);
        let half = wall_tint(0.5);
        assert!((half[0] - 0.75).abs() < 1e-6);
        assert_eq!(wall_tint(2.0), STONE);
    }

    #[test]
    fn godot_mapping_constants() {
        assert_eq!(STONE_SCALE, 0.22);
        assert_eq!(TRIPLANAR_SHARPNESS, 4.0);
        assert_eq!(NORMAL_SCALE, 0.8);
    }
}
