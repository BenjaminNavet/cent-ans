//! CPU mipmap chains: Bevy loads JPEG/PNG textures with a single mip level, which would alias
//! and cost cache misses compared with an engine that imports mipmaps.

use bevy::{
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
};

/// Box-filters `level0` (RGBA8, `width × height`) down to 1×1; returns all levels concatenated
/// (level 0 included) and the level count.
pub fn build_chain(level0: &[u8], width: usize, height: usize) -> (Vec<u8>, u32) {
    let mut data = level0.to_vec();
    let (mut w, mut h) = (width, height);
    let mut levels = 1;
    let mut previous = level0.to_vec();
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                let (x0, y0) = ((x * 2).min(w - 1), (y * 2).min(h - 1));
                let (x1, y1) = ((x * 2 + 1).min(w - 1), (y * 2 + 1).min(h - 1));
                for channel in 0..4 {
                    let sample =
                        |sx: usize, sy: usize| u32::from(previous[(sy * w + sx) * 4 + channel]);
                    let sum = sample(x0, y0) + sample(x1, y0) + sample(x0, y1) + sample(x1, y1);
                    next[(y * nw + x) * 4 + channel] = ((sum + 2) / 4) as u8;
                }
            }
        }
        data.extend_from_slice(&next);
        previous = next;
        (w, h) = (nw, nh);
        levels += 1;
    }
    (data, levels)
}

/// Adds mip chains to every uncompressed RGBA8 image that has only one level.
pub fn generate_missing_mips(images: &mut Assets<Image>) -> usize {
    let ids: Vec<AssetId<Image>> = images
        .iter()
        .filter(|(_, image)| {
            image.texture_descriptor.mip_level_count == 1
                && !image
                    .texture_descriptor
                    .usage
                    .contains(TextureUsages::RENDER_ATTACHMENT)
                && image.width() > 1
                && image.height() > 1
                && image.texture_descriptor.size.depth_or_array_layers == 1
                && matches!(
                    image.texture_descriptor.format,
                    TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
                )
                && image.data.is_some()
        })
        .map(|(id, _)| id)
        .collect();
    for id in &ids {
        let Some(mut image) = images.get_mut(*id) else {
            continue;
        };
        let (width, height) = (image.width() as usize, image.height() as usize);
        let Some(level0) = image.data.take() else {
            continue;
        };
        let (chain, levels) = build_chain(&level0, width, height);
        image.data = Some(chain);
        image.texture_descriptor.mip_level_count = levels;
        if matches!(image.sampler, ImageSampler::Default) {
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                mipmap_filter: ImageFilterMode::Linear,
                anisotropy_clamp: 8,
                ..default()
            });
        }
    }
    ids.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_has_every_level() {
        let (data, levels) = build_chain(&[255u8; 8 * 4 * 4], 8, 4);
        assert_eq!(levels, 4); // 8x4, 4x2, 2x1, 1x1
        assert_eq!(data.len(), (32 + 8 + 2 + 1) * 4);
        assert!(data.iter().all(|&value| value == 255));
    }

    #[test]
    fn chain_averages() {
        let mut level0 = vec![0u8; 2 * 2 * 4];
        level0[0] = 200; // one bright texel of four
        let (data, levels) = build_chain(&level0, 2, 2);
        assert_eq!(levels, 2);
        assert_eq!(data[16], 50);
    }
}
