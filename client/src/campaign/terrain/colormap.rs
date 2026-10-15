//! Ground colour map (`map.json` `colormap.bc1`, ADR 0142): BC1 image with mipmaps stored as zlib
//! parts, 2 texels per map unit. Port of `ReliefLandcover.load_colormap` / `load_gpu_copy`.

use std::io::Read;
use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use flate2::read::ZlibDecoder;
use serde_json::Value;

/// A GPU copy described by `map.json`: size, mip chain and raw (decompressed) bytes.
pub struct GpuCopy {
    pub width: u32,
    pub height: u32,
    pub mips: bool,
    pub data: Vec<u8>,
}

/// Bytes of a BC1 image of the given size, with the full mip chain if `mips`.
pub fn bc1_bytes(width: u32, height: u32, mips: bool) -> usize {
    let (mut w, mut h) = (width.max(1), height.max(1));
    let mut total = 0usize;
    loop {
        total += (w.div_ceil(4) * h.div_ceil(4)) as usize * 8;
        if !mips || (w == 1 && h == 1) {
            break;
        }
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    total
}

/// Number of mip levels of a full chain.
pub fn mip_levels(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

/// Reads `colormap.bc1` of `map.json` under `map_dir`; `Ok(None)` when the map has no colormap.
pub fn load_colormap(map_dir: &Path) -> Result<Option<GpuCopy>, String> {
    let text = std::fs::read_to_string(map_dir.join("map.json")).map_err(|e| e.to_string())?;
    let meta: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let Some(entry) = meta.get("colormap").and_then(|c| c.get("bc1")) else {
        return Ok(None);
    };
    if entry["format"].as_str() != Some("dxt1") || entry["compression"].as_str() != Some("deflate")
    {
        return Err("colormap.bc1: unsupported format".into());
    }
    let size = entry["size_px"].as_array().ok_or("colormap.bc1: size_px")?;
    let (width, height) = (
        size[0].as_u64().unwrap_or(0) as u32,
        size[1].as_u64().unwrap_or(0) as u32,
    );
    let mips = entry["mipmaps"].as_bool().unwrap_or(false);
    let pattern = entry["pattern"].as_str().ok_or("colormap.bc1: pattern")?;
    let parts = entry["part_bytes"]
        .as_array()
        .ok_or("colormap.bc1: part_bytes")?;
    let mut data = Vec::with_capacity(bc1_bytes(width, height, mips));
    for (index, expected) in parts.iter().enumerate() {
        let expected = expected.as_u64().unwrap_or(0) as usize;
        let path = map_dir.join(pattern.replace("{part}", &index.to_string()));
        let file = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let before = data.len();
        ZlibDecoder::new(std::io::BufReader::new(file))
            .read_to_end(&mut data)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if data.len() - before != expected {
            return Err(format!(
                "{}: {} bytes, expected {expected}",
                path.display(),
                data.len() - before
            ));
        }
    }
    if data.len() != bc1_bytes(width, height, mips) {
        return Err(format!(
            "colormap.bc1: {} bytes do not match {width}x{height}",
            data.len()
        ));
    }
    Ok(Some(GpuCopy {
        width,
        height,
        mips,
        data,
    }))
}

/// Bevy image of a BC1 copy (sRGB, trilinear, anisotropic, clamped).
pub fn bc1_image(copy: GpuCopy) -> Image {
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: copy.width,
        height: copy.height,
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.format = TextureFormat::Bc1RgbaUnormSrgb;
    image.texture_descriptor.mip_level_count = if copy.mips {
        mip_levels(copy.width, copy.height)
    } else {
        1
    };
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.data = Some(copy.data);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_chain_sizes_match_map_json() {
        // map.json: colormap 14160 x 12208 with mipmaps = 115 249 816 bytes (4 parts).
        assert_eq!(bc1_bytes(14160, 12208, true), 115_249_816);
        assert_eq!(bc1_bytes(14160, 12208, false), 3540 * 3052 * 8);
        assert_eq!(mip_levels(14160, 12208), 14);
        assert_eq!(bc1_bytes(4, 4, true), 24);
        assert_eq!(mip_levels(1, 1), 1);
    }

    #[test]
    fn image_descriptor_is_bc1_srgb_with_mips() {
        let image = bc1_image(GpuCopy {
            width: 8,
            height: 8,
            mips: true,
            data: vec![0; bc1_bytes(8, 8, true)],
        });
        assert_eq!(image.texture_descriptor.mip_level_count, 4);
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Bc1RgbaUnormSrgb
        );
    }
}
