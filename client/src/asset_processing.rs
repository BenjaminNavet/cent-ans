//! Image preprocessing (ADR 0724): JPG/PNG sources are compressed to KTX2 (BC7 colour, BC5 normals) by
//! Bevy's asset processor and cached in `imported_assets/`. Only built with `processed-assets`.

use bevy::asset::processor::{LoadTransformAndSave, SetDefaultProcessor};
use bevy::asset::transformer::IdentityAssetTransformer;
use bevy::image::{CompressedImageSaver, Image, ImageLoader};
use bevy::prelude::*;

/// Extensions of the raw images that the processor compresses.
const IMAGE_EXTENSIONS: [&str; 3] = ["jpg", "png", "webp"];

type ImageCompressor =
    LoadTransformAndSave<ImageLoader, IdentityAssetTransformer<Image>, CompressedImageSaver>;

/// Registers the image compressor as the default processor of every raw image extension.
pub struct AssetProcessingPlugin;

impl Plugin for AssetProcessingPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_processor::<ImageCompressor>(ImageCompressor::from(
            CompressedImageSaver::default(),
        ));
        for extension in IMAGE_EXTENSIONS {
            app.set_default_asset_processor::<ImageCompressor>(extension);
        }
    }
}
