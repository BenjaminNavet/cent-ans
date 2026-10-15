//! Land-use raster of the ground shader (`VegetationFields.landuse`, `terrain.gdshader` uniform
//! `landuse`): R vine share, G dryness, B bocage, A 1, `LANDUSE_SIZE`² RGBA8 in the map frame
//! (u = x / map width, v = y / map height), blurred across province borders.
//!
//! Built once on a thread from `data/map/province_ids.png` + `data/provinces/*.json`
//! (`field_plan::landuse_of`, `field_plan::landuse_image`) and published as the resource
//! [`LanduseTexture`] for the terrain lot (bind it with a linear, clamped sampler; Godot samples
//! it at the map uv and passes it through `terroir_landuse`). Visual only, no rule.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde_json::Value;

use super::field_plan::{LANDUSE_SIZE, landuse_image, landuse_of};
use crate::campaign::provinces::data::ProvinceMap;

/// The land-use texture, ready for the ground shader.
#[derive(Resource, Clone)]
pub struct LanduseTexture {
    pub image: Handle<Image>,
    /// Provinces read (with a land use), and build time.
    pub provinces: usize,
    pub build_ms: f32,
}

type Slot = Arc<Mutex<Option<Result<(Vec<u8>, usize, f32), String>>>>;

#[derive(Resource)]
pub(super) struct PendingLanduse(Slot);

/// The raster could not be built (logged once, not retried).
#[derive(Resource)]
pub(super) struct LanduseFailed;

/// RGBA8 bytes of the raster from the province map and a land use per province index.
pub fn raster_bytes(
    size: (f32, f32),
    index_at: impl Fn(f32, f32) -> u16,
    values: &HashMap<u16, [f32; 4]>,
) -> Vec<u8> {
    let pixels = landuse_image(|i, j, n| {
        let x = (i as f32 + 0.5) * size.0 / n as f32;
        let y = (j as f32 + 0.5) * size.1 / n as f32;
        values.get(&index_at(x, y)).copied()
    });
    pixels
        .iter()
        .flat_map(|p| p.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8))
        .collect()
}

fn build(root: std::path::PathBuf) -> Result<(Vec<u8>, usize, f32), String> {
    let started = std::time::Instant::now();
    let data = crate::paths::data_dir(&root);
    let map = ProvinceMap::load(&data.join("map"))?;
    let mut values = HashMap::new();
    for feature in map.provinces() {
        let path = data.join("provinces").join(format!("{}.json", feature.id));
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(province) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let index = map.index_of_id(&feature.id);
        if index != 0 {
            values.insert(index, landuse_of(&province));
        }
    }
    let size = (map.width as f32, map.height as f32);
    let bytes = raster_bytes(size, |x, y| map.index_at(x, y), &values);
    Ok((
        bytes,
        values.len(),
        started.elapsed().as_secs_f32() * 1000.0,
    ))
}

pub(super) fn start(mut commands: Commands) {
    let Ok(root) = crate::paths::find_root() else {
        commands.insert_resource(LanduseFailed);
        return;
    };
    let slot: Slot = Arc::default();
    let out = slot.clone();
    let spawned = std::thread::Builder::new()
        .name("landuse".into())
        .spawn(move || {
            *out.lock().expect("landuse slot") = Some(build(root));
        });
    if spawned.is_ok() {
        commands.insert_resource(PendingLanduse(slot));
    } else {
        commands.insert_resource(LanduseFailed);
    }
}

pub(super) fn finish(
    mut commands: Commands,
    pending: Res<PendingLanduse>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(result) = pending.0.lock().expect("landuse slot").take() else {
        return;
    };
    commands.remove_resource::<PendingLanduse>();
    match result {
        Ok((bytes, provinces, build_ms)) => {
            let mut image = Image::new(
                Extent3d {
                    width: LANDUSE_SIZE as u32,
                    height: LANDUSE_SIZE as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                bytes,
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            );
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::ClampToEdge,
                address_mode_v: ImageAddressMode::ClampToEdge,
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..default()
            });
            commands.insert_resource(LanduseTexture {
                image: images.add(image),
                provinces,
                build_ms,
            });
        }
        Err(error) => {
            warn!("landuse raster: {error}");
            commands.insert_resource(LanduseFailed);
        }
    }
}

/// Dump section `landuse`.
pub fn dump(world: &World) -> (String, Value) {
    let value = world
        .get_resource::<LanduseTexture>()
        .map_or(Value::Null, |t| {
            let image = world
                .resource::<Assets<Image>>()
                .get(&t.image)
                .and_then(|i| i.data.as_ref());
            let mean = image.map(|data| {
                let n = (data.len() / 4).max(1) as f32;
                (0..3)
                    .map(|c| {
                        data.iter()
                            .skip(c)
                            .step_by(4)
                            .map(|&v| v as f32)
                            .sum::<f32>()
                            / n
                            / 255.0
                    })
                    .collect::<Vec<f32>>()
            });
            serde_json::json!({
                "size": LANDUSE_SIZE, "provinces": t.provinces, "build_ms": t.build_ms,
                "mean_vine_dry_bocage": mean,
            })
        });
    ("landuse".into(), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_takes_the_province_values_and_fills_the_sea() {
        let values = HashMap::from([(1u16, [1.0, 0.0, 0.5, 1.0])]);
        // Land on the left half, sea on the right.
        let bytes = raster_bytes((100.0, 100.0), |x, _| u16::from(x < 50.0), &values);
        assert_eq!(bytes.len(), LANDUSE_SIZE * LANDUSE_SIZE * 4);
        assert_eq!(&bytes[..4], &[255, 0, 128, 255]);
        // The sea near the coast takes the neighbouring land (dilation).
        let row = LANDUSE_SIZE / 2;
        let coast = (row * LANDUSE_SIZE + LANDUSE_SIZE / 2 + 2) * 4;
        assert!(bytes[coast] > 200);
    }
}
