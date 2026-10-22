//! Image preprocessing (ADR 0724): images that carry a `.meta` naming the compressor become KTX2
//! (BC7 colour, BC5 normals) in `imported_assets/`. Only built with `processed-assets`.
//!
//! The compressor is deliberately NOT the default processor of any extension: most atlases are read or
//! rebuilt on the CPU as RGBA8 (ground strips, mip chains, UI icons, ...) and would break once block
//! compressed. A texture opts in through its `.meta` file.

use bevy::asset::processor::{LoadTransformAndSave, SetDefaultProcessor};
use bevy::asset::transformer::IdentityAssetTransformer;
use bevy::image::{CompressedImageSaver, Image, ImageLoader};
use bevy::prelude::*;

type ImageCompressor =
    LoadTransformAndSave<ImageLoader, IdentityAssetTransformer<Image>, CompressedImageSaver>;

/// Registers the image compressor so that `.meta` files can select it.
pub struct AssetProcessingPlugin;

impl Plugin for AssetProcessingPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_processor::<ImageCompressor>(ImageCompressor::from(
            CompressedImageSaver::default(),
        ));
    }
}
