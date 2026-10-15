//! Tiling water detail textures (`water_detail.gd`, `water_detail.gdshaderinc`, ADR 0141 / 0241).
//!
//! `data/fx/water_detail.json` maps a water material id (`sea`, `ocean`, `river_large`,
//! `river_small`, per-basin sea surfaces) to a normal + albedo pair in
//! `game/assets/textures/water/`: the generated TX surface (`<tx>_normal.jpg` / `_albedo.jpg`,
//! 2k) when its files exist, else the older `<id>_normal.png` / `_albedo.png` (512 px).
//! The shaders add the normal to the procedural waves, tint the colour by the albedo relative to
//! its mean (`mean` is computed here instead of a `textureLod(.., 9)` read) and, on rivers, use
//! the albedo alpha as a foam streak mask.
//!
//! Pure parts (spec parsing, path choice, basin slots, mean colour) are unit tested; the image
//! loading builds mip chains (Godot imports them) on the CPU.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::asset::RenderAssetUsages;
use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::Deserialize;

use crate::campaign::vegetation::mips;

/// Largest side (texels) kept for the detail textures: 2k sources are halved (Godot keeps the
/// array layers at 1024 as well, `SEA_ARRAY_SIZE`).
pub const MAX_SIZE: usize = 1024;
/// Slots of the basin arrays: the default sea plus [`super::data::MAX_BASINS`] basins.
pub const SLOTS: usize = 5;

/// One material of `water_detail.json`.
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct DetailEntry {
    /// Weight of the detail in the shader (0 = procedural only).
    pub strength: f32,
    /// Tile repetitions per world unit.
    pub scale: f32,
    /// Normal drift speed (tiles per second); 0 for the sea.
    pub flow_speed: f32,
    /// 0 clear .. 1 silty (rivers).
    pub turbidity: f32,
    /// Generated surface id replacing the 512 px texture.
    pub tx: String,
}

/// `data/fx/water_detail.json`.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct DetailSpec {
    pub materials: HashMap<String, DetailEntry>,
    /// Sea-basin slot ("default", then the basin ids) -> material id.
    pub basins: HashMap<String, String>,
}

impl DetailSpec {
    pub fn parse(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    pub fn load(root: &Path) -> Self {
        std::fs::read_to_string(root.join("data/fx/water_detail.json"))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    /// Entry of the material `id`, scale defaulting to 1 like `float(entry.get("scale", 1.0))`.
    pub fn entry(&self, id: &str) -> Option<DetailEntry> {
        self.materials.get(id).map(|entry| {
            let mut entry = entry.clone();
            if entry.scale == 0.0 {
                entry.scale = 1.0;
            }
            entry
        })
    }
}

/// File names `(normal, albedo)` of a material: the generated surface when `exists` finds it,
/// else the 512 px PNGs named after the material (`WaterDetail._paths`).
pub fn texture_files(id: &str, entry: &DetailEntry, exists: impl Fn(&str) -> bool) -> [String; 2] {
    if !entry.tx.is_empty() {
        let normal = format!("{}_normal.jpg", entry.tx);
        if exists(&normal) {
            return [normal, format!("{}_albedo.jpg", entry.tx)];
        }
    }
    [format!("{id}_normal.png"), format!("{id}_albedo.png")]
}

/// Basin slots (`WaterDetail.apply_sea_basins`): distinct generated surfaces in order of first use
/// (the array layers) and per slot `(layer or -1, strength, scale, 0)`. `slot_ids` = the basin ids
/// in file order; slot 0 is "default".
pub fn basin_slots(spec: &DetailSpec, slot_ids: &[String]) -> (Vec<String>, [Vec4; SLOTS]) {
    let mut layers: Vec<String> = Vec::new();
    let mut params = [Vec4::new(-1.0, 0.0, 1.0, 0.0); SLOTS];
    let mut slots: Vec<&str> = vec!["default"];
    slots.extend(slot_ids.iter().map(String::as_str));
    for (index, slot) in slots.iter().take(SLOTS).enumerate() {
        let wanted = spec
            .basins
            .get(*slot)
            .or_else(|| spec.basins.get("default"));
        let Some(id) = wanted else { continue };
        let Some(entry) = spec.materials.get(id) else {
            continue;
        };
        if entry.tx.is_empty() {
            continue;
        }
        let layer = match layers.iter().position(|l| l == &entry.tx) {
            Some(layer) => layer,
            None => {
                layers.push(entry.tx.clone());
                layers.len() - 1
            }
        };
        let strength = if entry.strength == 0.0 {
            0.3
        } else {
            entry.strength
        };
        let scale = if entry.scale == 0.0 { 2.0 } else { entry.scale };
        params[index] = Vec4::new(layer as f32, strength, scale, 0.0);
    }
    (layers, params)
}

fn srgb_to_linear(byte: u8) -> f32 {
    let c = byte as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Mean colour (linear) of an RGBA8 sRGB image, floored at 0.02 like `max(mean, vec3(0.02))`.
pub fn linear_mean(rgba: &[u8]) -> Vec3 {
    let pixels = (rgba.len() / 4).max(1) as f32;
    let mut sum = Vec3::ZERO;
    for px in rgba.as_chunks::<4>().0 {
        sum += Vec3::new(
            srgb_to_linear(px[0]),
            srgb_to_linear(px[1]),
            srgb_to_linear(px[2]),
        );
    }
    (sum / pixels).max(Vec3::splat(0.02))
}

/// Number of mip levels down to 1x1 for a square image of `size` texels.
pub fn mip_levels(size: usize) -> u32 {
    usize::BITS - size.max(1).leading_zeros()
}

/// One decoded texture, mips built, RGBA8, side <= [`MAX_SIZE`].
pub struct Decoded {
    pub size: usize,
    pub data: Vec<u8>,
}

/// Decodes a PNG / JPEG file to RGBA8 and halves it down to [`MAX_SIZE`].
pub fn decode(path: &Path) -> Option<Decoded> {
    let bytes = std::fs::read(path).ok()?;
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let image = Image::from_buffer(
        &bytes,
        ImageType::Extension(&extension),
        CompressedImageFormats::NONE,
        false,
        ImageSampler::linear(),
        RenderAssetUsages::MAIN_WORLD,
    )
    .map_err(|error| warn!("water detail: {}: {error}", path.display()))
    .ok()?;
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut data = image.data?;
    if w != h || !w.is_power_of_two() || data.len() != w * h * 4 {
        warn!(
            "water detail: {} is not a square RGBA8 power of two",
            path.display()
        );
        return None;
    }
    let mut size = w;
    while size > MAX_SIZE {
        data = mips::downsample(&data, size, size, false);
        size /= 2;
    }
    Some(Decoded { size, data })
}

fn repeat_sampler() -> ImageSampler {
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    })
}

/// A mip-mapped, repeating GPU image of one or several layers (layer-major data).
fn gpu_image(layers: &[&Decoded], srgb: bool) -> Image {
    let size = layers[0].size;
    let levels = mip_levels(size);
    let format = if srgb {
        TextureFormat::Rgba8UnormSrgb
    } else {
        TextureFormat::Rgba8Unorm
    };
    let mut image = Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: layers.len() as u32,
        },
        TextureDimension::D2,
        vec![0u8; size * size * 4 * layers.len()],
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    let mut data = Vec::new();
    for layer in layers {
        data.extend_from_slice(&mips::chain(&layer.data, size, size, levels, false));
    }
    image.data = Some(data);
    image.texture_descriptor.mip_level_count = levels;
    image.sampler = repeat_sampler();
    image
}

/// A material's loaded pair, ready to bind.
#[derive(Debug, Clone)]
pub struct Layer {
    pub id: String,
    pub normal: Handle<Image>,
    pub albedo: Handle<Image>,
    pub strength: f32,
    pub scale: f32,
    pub flow_speed: f32,
    pub turbidity: f32,
    /// Linear mean colour of the albedo.
    pub mean: Vec3,
    pub files: [String; 2],
    pub size: usize,
}

/// Basin arrays (one layer per distinct generated surface) and the per-slot parameters.
#[derive(Debug, Clone)]
pub struct BasinArrays {
    pub normals: Handle<Image>,
    pub albedos: Handle<Image>,
    pub layers: Vec<String>,
    pub params: [Vec4; SLOTS],
    /// Linear mean albedo per slot.
    pub means: [Vec4; SLOTS],
}

/// Loads detail textures once per build and caches them by material id.
pub struct DetailLoader {
    pub spec: DetailSpec,
    dir: PathBuf,
    cache: HashMap<String, Option<Layer>>,
}

impl DetailLoader {
    pub fn new(root: &Path) -> Self {
        Self {
            spec: DetailSpec::load(root),
            dir: crate::paths::asset_root(root).join("textures/water"),
            cache: HashMap::new(),
        }
    }

    /// `WaterDetail.apply`: the material's pair, or `None` (strength stays 0) when the data or
    /// the normal texture is missing. The albedo defaults to white when its file is missing.
    pub fn layer(&mut self, id: &str, images: &mut Assets<Image>) -> Option<Layer> {
        if let Some(hit) = self.cache.get(id) {
            return hit.clone();
        }
        let layer = self.build_layer(id, images);
        self.cache.insert(id.to_string(), layer.clone());
        layer
    }

    fn build_layer(&self, id: &str, images: &mut Assets<Image>) -> Option<Layer> {
        let entry = self.spec.entry(id)?;
        let dir = self.dir.clone();
        let files = texture_files(id, &entry, |name| dir.join(name).is_file());
        let normal = decode(&dir.join(&files[0]))?;
        let albedo = decode(&dir.join(&files[1])).unwrap_or_else(|| Decoded {
            size: 1,
            data: vec![255; 4],
        });
        let mean = linear_mean(&albedo.data);
        let size = normal.size;
        let normal = images.add(gpu_image(&[&normal], false));
        let albedo = images.add(gpu_image(&[&albedo], true));
        Some(Layer {
            id: id.to_string(),
            normal,
            albedo,
            strength: entry.strength,
            scale: entry.scale,
            flow_speed: entry.flow_speed,
            turbidity: entry.turbidity,
            mean,
            files,
            size,
        })
    }

    /// `WaterDetail.apply_sea_basins`: arrays of the generated surfaces named by `basins`.
    pub fn basin_arrays(
        &self,
        basin_ids: &[String],
        images: &mut Assets<Image>,
    ) -> Option<BasinArrays> {
        let (wanted, mut params) = basin_slots(&self.spec, basin_ids);
        let mut normals: Vec<Decoded> = Vec::new();
        let mut albedos: Vec<Decoded> = Vec::new();
        let mut kept: Vec<String> = Vec::new();
        let mut remap: Vec<Option<usize>> = Vec::new();
        for tx in &wanted {
            let n = decode(&self.dir.join(format!("{tx}_normal.jpg")));
            let a = decode(&self.dir.join(format!("{tx}_albedo.jpg")));
            match (n, a) {
                (Some(n), Some(a)) if n.size == a.size => {
                    remap.push(Some(normals.len()));
                    normals.push(n);
                    albedos.push(a);
                    kept.push(tx.clone());
                }
                _ => remap.push(None),
            }
        }
        if normals.is_empty() {
            return None;
        }
        let mut means = [Vec4::ONE; SLOTS];
        for (slot, p) in params.iter_mut().enumerate() {
            if p.x < 0.0 {
                continue;
            }
            match remap.get(p.x as usize).copied().flatten() {
                Some(layer) => {
                    p.x = layer as f32;
                    means[slot] = linear_mean(&albedos[layer].data).extend(0.0);
                }
                None => *p = Vec4::new(-1.0, 0.0, 1.0, 0.0),
            }
        }
        let normal_refs: Vec<&Decoded> = normals.iter().collect();
        let albedo_refs: Vec<&Decoded> = albedos.iter().collect();
        Some(BasinArrays {
            normals: images.add(gpu_image(&normal_refs, false)),
            albedos: images.add(gpu_image(&albedo_refs, true)),
            layers: kept,
            params,
            means,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"{
      "materials": {
        "sea": {"strength": 0.35, "scale": 2.0, "flow_speed": 0.0},
        "ocean": {"strength": 0.3, "scale": 1.0, "tx": "water_ocean_open"},
        "river_small": {"strength": 0.45, "scale": 6.0, "flow_speed": 0.08, "turbidity": 0.1, "tx": "water_river_small_clear"},
        "water_sea_atlantic": {"tx": "water_sea_atlantic", "strength": 0.35, "scale": 2.0},
        "water_sea_black": {"tx": "water_sea_black", "strength": 0.35, "scale": 2.0}
      },
      "basins": {"default": "water_sea_black", "atlantic": "water_sea_atlantic", "baltic": "water_sea_black"}
    }"#;

    #[test]
    fn spec_parses_and_defaults_scale() {
        let spec = DetailSpec::parse(SPEC);
        assert_eq!(spec.materials.len(), 5);
        let river = spec.entry("river_small").unwrap();
        assert_eq!(
            (river.strength, river.scale, river.turbidity),
            (0.45, 6.0, 0.1)
        );
        assert!(spec.entry("nothing").is_none());
        let bare = DetailSpec::parse(r#"{"materials": {"x": {"strength": 1.0}}}"#);
        assert_eq!(bare.entry("x").unwrap().scale, 1.0);
        assert!(DetailSpec::parse("not json").materials.is_empty());
    }

    #[test]
    fn generated_surface_wins_when_present_else_png() {
        let spec = DetailSpec::parse(SPEC);
        let ocean = spec.entry("ocean").unwrap();
        assert_eq!(
            texture_files("ocean", &ocean, |_| true),
            ["water_ocean_open_normal.jpg", "water_ocean_open_albedo.jpg"]
        );
        assert_eq!(
            texture_files("ocean", &ocean, |_| false),
            ["ocean_normal.png", "ocean_albedo.png"]
        );
        let sea = spec.entry("sea").unwrap();
        assert_eq!(
            texture_files("sea", &sea, |_| true),
            ["sea_normal.png", "sea_albedo.png"]
        );
    }

    #[test]
    fn basin_slots_share_layers_and_fall_back_to_default() {
        let spec = DetailSpec::parse(SPEC);
        let ids = vec![
            "atlantic".to_string(),
            "north_sea".to_string(),
            "baltic".to_string(),
        ];
        let (layers, params) = basin_slots(&spec, &ids);
        // default -> black (layer 0); atlantic -> atlantic (1); north_sea unknown -> default (0);
        // baltic -> black (0); 5th slot unused (-1).
        assert_eq!(layers, ["water_sea_black", "water_sea_atlantic"]);
        assert_eq!(params[0].x, 0.0);
        assert_eq!(params[1].x, 1.0);
        assert_eq!(params[2].x, 0.0);
        assert_eq!(params[3].x, 0.0);
        assert_eq!(params[4].x, -1.0);
        assert_eq!((params[1].y, params[1].z), (0.35, 2.0));
    }

    #[test]
    fn basin_slots_without_generated_surface_stay_off() {
        let spec = DetailSpec::parse(
            r#"{"materials": {"sea": {"strength": 0.3}}, "basins": {"default": "sea"}}"#,
        );
        let (layers, params) = basin_slots(&spec, &[]);
        assert!(layers.is_empty());
        assert!(params.iter().all(|p| p.x < 0.0));
    }

    #[test]
    fn mean_is_linear_and_floored() {
        let grey = [188u8, 188, 188, 255].repeat(4); // sRGB 188 ~ 0.5 linear
        let mean = linear_mean(&grey);
        assert!((mean.x - 0.5).abs() < 0.01, "{mean:?}");
        let black = linear_mean(&[0, 0, 0, 255]);
        assert_eq!(black, Vec3::splat(0.02));
    }

    #[test]
    fn mip_levels_reach_one_texel() {
        assert_eq!(mip_levels(1), 1);
        assert_eq!(mip_levels(512), 10);
        assert_eq!(mip_levels(1024), 11);
    }

    #[test]
    fn gpu_image_layers_are_layer_major_with_mips() {
        let layer = Decoded {
            size: 4,
            data: vec![100; 4 * 4 * 4],
        };
        let image = gpu_image(&[&layer, &layer], false);
        assert_eq!(image.texture_descriptor.mip_level_count, 3);
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 2);
        let per_layer = (16 + 4 + 1) * 4;
        assert_eq!(image.data.as_ref().unwrap().len(), per_layer * 2);
    }

    #[test]
    fn real_textures_decode_when_present() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let spec = DetailSpec::load(&root);
        let dir = crate::paths::asset_root(&root).join("textures/water");
        let sea = spec.entry("sea").expect("sea entry");
        let files = texture_files("sea", &sea, |n| dir.join(n).is_file());
        let decoded = decode(&dir.join(&files[0])).expect("sea normal decodes");
        assert_eq!(decoded.size, 512);
        let ocean = spec.entry("ocean").unwrap();
        let files = texture_files("ocean", &ocean, |n| dir.join(n).is_file());
        assert!(files[0].ends_with(".jpg"), "{files:?}");
        assert_eq!(decode(&dir.join(&files[0])).unwrap().size, MAX_SIZE);
    }
}
