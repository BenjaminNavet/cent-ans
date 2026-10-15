//! Data of the full parchment look (strategic view, ADR 0124; port of `parchment_map.gdshaderinc`,
//! lot CM2): the coarse height raster the drawn mountains and trees read (the Godot shader samples
//! the heightmap at mip 4, 16 px blocks) and the faction wash raster (owner heraldic colour per
//! province, same blocks as the province fill). The drawing itself is in `terrain.wesl`.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::TerrainMaterialHandle;
use super::material::TerrainMaterial;
use super::relief_floor::HeightGrid;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::provinces::overlay::{FILL_DOWNSCALE, OverlayState, paint_fill};

/// Side (map px) of a block of the coarse height raster (mip 4 of the Godot heightmap).
pub const HEIGHT_BLOCK: usize = 16;

/// Block-averaged altitudes (m) as an R32Float image (read with `textureLoad`).
pub fn height_lod_image(heights: &HeightGrid) -> Image {
    let (w, h) = (
        heights.width.div_ceil(HEIGHT_BLOCK),
        heights.height.div_ceil(HEIGHT_BLOCK),
    );
    let mut out = Vec::with_capacity(w * h * 4);
    for by in 0..h {
        for bx in 0..w {
            let (x0, y0) = (bx * HEIGHT_BLOCK, by * HEIGHT_BLOCK);
            let (x1, y1) = (
                (x0 + HEIGHT_BLOCK).min(heights.width),
                (y0 + HEIGHT_BLOCK).min(heights.height),
            );
            let mut sum = 0u64;
            for y in y0..y1 {
                let row = &heights.data[y * heights.width + x0..y * heights.width + x1];
                sum += row.iter().map(|v| u64::from(*v)).sum::<u64>();
            }
            let count = ((x1 - x0) * (y1 - y0)) as f32;
            let raw = sum as f32 / count;
            let meters = heights.h_min_m + raw / 65535.0 * heights.h_range_m;
            out.extend_from_slice(&meters.to_le_bytes());
        }
    }
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: w as u32,
        height: h as u32,
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.format = TextureFormat::R32Float;
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.data = Some(out);
    image
}

/// Owner colours of the faction wash and its image; rebuilt when the owners change.
#[derive(Resource)]
pub struct WashRaster {
    pub image: Handle<Image>,
    colors: Vec<[u8; 4]>,
    pub rebuilds: u32,
}

fn wash_image(width: u32, height: u32) -> Image {
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.format = TextureFormat::Rgba8UnormSrgb;
    image.asset_usage = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
    image.data = Some(vec![0; (width * height * 4) as usize]);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// Paints the wash raster from the owners of the provinces (`OverlayState::view`) and binds it.
pub fn sync_wash(
    mut commands: Commands,
    overlay: Option<Res<OverlayState>>,
    map: Option<Res<ProvinceMap>>,
    wash: Option<ResMut<WashRaster>>,
    handle: Option<Res<TerrainMaterialHandle>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let (Some(overlay), Some(map), Some(handle)) = (overlay, map, handle) else {
        return;
    };
    let max_index = overlay.colors.len().saturating_sub(1) as u16;
    let colors: Vec<[u8; 4]> = (0..=max_index)
        .map(|index| {
            if index == 0 {
                return [0; 4];
            }
            overlay
                .heraldic
                .get(overlay.view.owner_of(index))
                .map_or([0; 4], |c| c.with_alpha(1.0).to_u8_array())
        })
        .collect();
    let (width, height) = (map.width / FILL_DOWNSCALE, map.height / FILL_DOWNSCALE);
    let mut bind = false;
    let image_handle = match wash {
        Some(mut state) => {
            if state.colors == colors {
                return;
            }
            state.colors.clone_from(&colors);
            state.rebuilds += 1;
            state.image.clone()
        }
        None => {
            let image = images.add(wash_image(width, height));
            commands.insert_resource(WashRaster {
                image: image.clone(),
                colors: colors.clone(),
                rebuilds: 1,
            });
            bind = true;
            image
        }
    };
    let table: Vec<Srgba> = colors
        .iter()
        .map(|c| Srgba::from_u8_array(*c))
        .collect::<Vec<_>>();
    if let Some(mut image) = images.get_mut(&image_handle)
        && let Some(data) = image.data.as_mut()
    {
        paint_fill(&map, &table, width, height, data);
    }
    if bind && let Some(mut material) = materials.get_mut(&handle.0) {
        material.wash = Some(image_handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_lod_averages_blocks_in_metres() {
        let grid = HeightGrid {
            width: 40,
            height: 20,
            data: vec![65535; 40 * 20],
            h_min_m: -200.0,
            h_range_m: 5000.0,
        };
        let image = height_lod_image(&grid);
        assert_eq!(image.width(), 3);
        assert_eq!(image.height(), 2);
        let bytes = image.data.as_ref().unwrap();
        let first = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert!((first - 4800.0).abs() < 0.5, "{first}");
    }
}
