//! Stone of the siege walls and towers: the player's `castle_wall_varriation` texture
//! (`game/assets/textures/battle/castle_wall_varriation_{diff,nor}.jpg`, full 2048 px) projected in
//! world triplanar. The mapping (repetitions per metre, blend sharpness, normal strength,
//! roughness) and the tints (`STONE`, `STONE_DARK.lerp(STONE, ratio)` as a wall is battered) come
//! from `data/art/siege_stone.json` (lot B7: 0.33 repetitions per metre, i.e. courses of ~0.21 m
//! and blocks of 0.3-0.7 m; Godot's 0.22 gave blocks twice too big for a rampart). The generic
//! building atlas (512 px layers, 3 m tile) keeps the houses, the rubble and the roofs.

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler};
use bevy::image::{ImageSamplerDescriptor, TextureFormatPixelInfo};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use crate::battle::decor::ground::{mip_chain, mip_count};

/// Data file of the stone look, under `data/`.
pub const LOOK_FILE: &str = "art/siege_stone.json";
/// Defaults when the data file is missing or unreadable (same values as the shipped file).
pub const STONE: [f32; 3] = [0.95, 0.9, 0.8];
pub const STONE_DARK: [f32; 3] = [0.55, 0.52, 0.47];

/// Triplanar mapping and tints of the siege stone (`data/art/siege_stone.json`).
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct StoneLook {
    pub repetitions_per_metre: f32,
    pub blend_sharpness: f32,
    pub normal_strength: f32,
    pub roughness: f32,
    pub reflectance: f32,
    pub stone: [f32; 3],
    pub stone_dark: [f32; 3],
}

impl Default for StoneLook {
    fn default() -> Self {
        Self {
            repetitions_per_metre: 0.33,
            blend_sharpness: 4.0,
            normal_strength: 1.0,
            roughness: 0.92,
            reflectance: 0.3,
            stone: STONE,
            stone_dark: STONE_DARK,
        }
    }
}

impl StoneLook {
    pub fn load() -> StoneLook {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read(crate::paths::data_dir(&root).join(LOOK_FILE)).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Colour of a wall holding `ratio` of its hit points.
    pub fn tint(&self, ratio: f32) -> [f32; 3] {
        let k = ratio.clamp(0.0, 1.0);
        std::array::from_fn(|i| self.stone_dark[i] + (self.stone[i] - self.stone_dark[i]) * k)
    }
}

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

/// The two textures (loaded once) and the cached materials by damage step.
#[derive(Resource)]
pub struct StoneTextures {
    diffuse: Handle<Image>,
    normal: Handle<Image>,
    pub converted: bool,
    pub look: StoneLook,
    /// False when the texture files are missing (the atlas masonry stays the fallback).
    pub present: bool,
}

impl StoneTextures {
    /// One material tinted by `ratio` (1 = intact).
    pub fn material(&self, ratio: f32) -> SiegeStoneMaterial {
        let tint = self.look.tint(ratio);
        ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::linear_rgb(tint[0], tint[1], tint[2]),
                perceptual_roughness: self.look.roughness,
                metallic: 0.0,
                reflectance: self.look.reflectance,
                ..default()
            },
            extension: StoneExt {
                params: Vec4::new(
                    self.look.repetitions_per_metre,
                    self.look.blend_sharpness,
                    self.look.normal_strength,
                    0.0,
                ),
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
        look: StoneLook::load(),
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
        let look = StoneLook::default();
        assert_eq!(look.tint(1.0), STONE);
        assert_eq!(look.tint(0.0), STONE_DARK);
        let half = look.tint(0.5);
        assert!((half[0] - 0.75).abs() < 1e-6);
        assert_eq!(look.tint(2.0), STONE);
    }

    #[test]
    fn shipped_file_parses_to_the_defaults() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let bytes = std::fs::read(root.join("data").join(LOOK_FILE)).expect("look file");
        let look: StoneLook = serde_json::from_slice(&bytes).expect("parses");
        assert_eq!(look, StoneLook::default());
    }

    #[test]
    fn blocks_stay_at_rampart_scale() {
        // The texture holds ~14 courses per tile: a course must measure 0.15-0.3 m.
        let course = 1.0 / StoneLook::default().repetitions_per_metre / 14.0;
        assert!((0.15..=0.3).contains(&course), "course {course} m");
    }
}
