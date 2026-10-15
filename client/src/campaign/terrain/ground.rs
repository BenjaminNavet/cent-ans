//! Regional ground of the campaign map (TX, ADR 0243): the 45-layer background pack
//! (`tx_campaign_bg_*`, role x biome), the biome blend rasters (`biomes_blend_ab/dist.png`), the
//! soil-material splat (`splat.png`) and the micro grain pack (`tx_micro_ground_*`). Port of
//! `campaign_textures.gd` + `biome_parents.gd` (layer table, hues) and of the splat / sample
//! parts of `terrain.gdshader`; the shading is in `terrain.wesl` (`ground_proc`).
//!
//! Loading runs off the main thread (JPEG sheets are cut into texture arrays at the size Godot
//! uses: the local 2048 px packs in high texture quality, albedo BC1-encoded on the CPU like the
//! Godot VRAM import, normals Rg8 up to 1024 px, mip chains built on the CPU) and the result is bound to [`TerrainMaterial`] by
//! `poll_ground` in `mod.rs`. Settings: `data/fx/campaign_terrain_textures.json`.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler,
    ImageSamplerDescriptor, ImageType,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, ShaderType, TextureDimension, TextureFormat, TextureViewDescriptor,
    TextureViewDimension,
};
use serde_json::Value;

/// Roles of the shader, in the order of `terrain.wesl` (`ROLE_*`).
pub const ROLES: [&str; 7] = [
    "grass", "farmland", "forest", "rock", "heath", "snow", "sand",
];
/// Biome classes of `biomes.yaml` (0 = sea).
pub const BIOME_COUNT: usize = 15;
/// Entries of the per-layer tables of the shader.
pub const MEAN_SLOTS: usize = 48;
/// Largest side of one layer of the normal arrays on the GPU (the `normal_size` of the 2048
/// packs); albedo layers keep the size of their sheet (BC1).
pub const NORMAL_MAX_PX: usize = 1024;

/// Uniform block `GroundParams` of `terrain.wesl`.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct GroundParams {
    /// Layer of each (biome, role): index `biome * 7 + role`, padded to 27 vec4.
    pub layers: [UVec4; 27],
    /// Linear mean colour of each layer.
    pub means: [Vec4; MEAN_SLOTS],
    /// Hue of each layer relative to its role.
    pub hues: [Vec4; MEAN_SLOTS],
    /// Micro layer of each role (7, padded to 2 vec4).
    pub micro_layers: [UVec4; 2],
    /// Mean luminance of each micro layer (8, 2 vec4).
    pub micro_mean: [Vec4; 2],
    /// 1 when the regional pack is bound, blend width (texels), min blend weight, hue keep.
    pub bg: Vec4,
    /// Tile screen px, base tile (map px), detail strength, detail normal strength.
    pub tiling: Vec4,
    /// Micro on, micro tile (m), micro strength, metres per map px.
    pub micro: Vec4,
    /// Detail fade footprints (xy), micro fade footprints (zw).
    pub fades: Vec4,
    /// Splat bound (x), rock line (m) (y), land-use raster bound (z), terroir mask bound (w; both
    /// set by `terroir.rs`).
    pub splat: Vec4,
    /// GA4 macro variation: noise periods (map px) and octave weights.
    pub macro_periods: Vec4,
    pub macro_weights: Vec4,
    /// GA4 tint amplitude, luminance amplitude (0 = off); TB6 canopy relief of the dark masses.
    pub macro_amp: Vec4,
    /// TB6 dark forest masses of the colour map: floor (linear luminance, 0 = off), knee,
    /// feather (mip levels), kept contrast.
    pub dark: Vec4,
    /// RV-D valley occlusion (`relief_occlusion.gdshaderinc`): bound, valley darkening, crest
    /// lift, ambient occlusion of the valley floors.
    pub occlusion: Vec4,
}

impl Default for GroundParams {
    fn default() -> Self {
        GroundParams {
            layers: [UVec4::ZERO; 27],
            means: [Vec4::splat(0.1); MEAN_SLOTS],
            hues: [Vec4::ONE; MEAN_SLOTS],
            micro_layers: [UVec4::ZERO; 2],
            micro_mean: [Vec4::splat(0.18); 2],
            bg: Vec4::new(0.0, 7.0, 0.03, 0.0),
            tiling: Vec4::new(96.0, 3.0, 0.85, 0.55),
            micro: Vec4::new(0.0, 10.0, 0.55, 719.0),
            fades: Vec4::new(0.45, 1.3, 0.004, 0.02),
            splat: Vec4::new(0.0, 1700.0, 0.0, 0.0),
            macro_periods: Vec4::new(9.0, 24.0, 60.0, 150.0),
            macro_weights: Vec4::new(0.4, 0.3, 0.2, 0.1),
            macro_amp: Vec4::new(0.0, 0.0, 0.0, 0.0),
            dark: Vec4::new(0.0, 2.0, 2.5, 0.25),
            occlusion: Vec4::new(0.0, 0.40, 0.14, 0.6),
        }
    }
}

/// Everything the loading thread produces.
pub struct GroundData {
    pub bg_albedo: Image,
    pub bg_normal: Image,
    /// See [`load_blend`].
    pub ab_dist: Image,
    pub splat: Image,
    pub micro_albedo: Option<Image>,
    pub params: GroundParams,
    pub fields: super::fields::FieldParams,
    /// Parcels materials of the painted fields (`tx_campaign_parcels_pack`, `hb_albedo`): albedo
    /// (sRGB) and normal (Rg8) arrays; `None` when the pack is missing.
    pub parcels_albedo: Option<Image>,
    pub parcels_normal: Option<Image>,
    /// RV-D valley occlusion (`data/map/relief_occlusion.png`, L8 on the half grid, 128 neutral).
    pub occlusion: Option<Image>,
}

/// The parcels pack of the painted fields as two texture arrays (`GroundMaterials.load_arrays`).
fn load_parcels(root: &Path, high: bool) -> Result<(Image, Image), String> {
    let pack = load_pack(root, "art/tx_campaign_parcels_pack.json", high, true)?;
    Ok((pack.albedo, pack.normal))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Parent of each biome (`data/map/biome_parents.json`, `BiomeParents.parent_of`).
pub fn load_parents(root: &Path) -> Vec<usize> {
    read_json(&root.join("data/map/biome_parents.json"))
        .ok()
        .and_then(|v| {
            v["parents"]
                .as_array()
                .map(|a| a.iter().map(|x| x.as_u64().unwrap_or(0) as usize).collect())
        })
        .unwrap_or_else(|| (0..BIOME_COUNT).collect())
}

/// Layer of the pack for (biome, pack role), climbing the parents; `None` if absent.
fn find_layer(layers: &[Value], parents: &[usize], biome: usize, pack_role: &str) -> Option<u32> {
    let mut current = biome;
    for _ in 0..BIOME_COUNT {
        for layer in layers {
            if layer["role"].as_str() != Some(pack_role) {
                continue;
            }
            let listed = layer["biomes"]
                .as_array()
                .is_some_and(|b| b.iter().any(|x| x.as_u64() == Some(current as u64)));
            if listed {
                return layer["layer"].as_u64().map(|l| l as u32);
            }
        }
        let parent = parents.get(current).copied().unwrap_or(current);
        if parent == current {
            break;
        }
        current = parent;
    }
    None
}

/// `bg_layers` (`CampaignTextures.build_layer_table`): `BIOME_COUNT x 7`, index
/// `biome * 7 + role`; biome 0 (sea) copies biome 1.
pub fn build_layer_table(layers: &[Value], regional: &Value, parents: &[usize]) -> Vec<u32> {
    let mut table = vec![0u32; BIOME_COUNT * ROLES.len()];
    for biome in 1..BIOME_COUNT {
        let local = &regional["overrides"][biome.to_string()];
        for (role_index, role) in ROLES.iter().enumerate() {
            let wanted = local[role]
                .as_str()
                .or_else(|| regional["roles"][role].as_str())
                .unwrap_or("");
            let layer = find_layer(layers, parents, biome, wanted).unwrap_or(0);
            table[biome * ROLES.len() + role_index] = layer;
        }
    }
    for role_index in 0..ROLES.len() {
        table[role_index] = table[ROLES.len() + role_index];
    }
    table
}

fn mean_of(layer: &Value) -> Vec3 {
    let m = layer["mean_linear"].as_array();
    let get = |i: usize| {
        m.and_then(|a| a.get(i))
            .and_then(Value::as_f64)
            .unwrap_or(0.1) as f32
    };
    Vec3::new(get(0), get(1), get(2))
}

/// Linear mean colours padded to [`MEAN_SLOTS`].
pub fn padded_means(layers: &[Value]) -> Vec<Vec4> {
    let mut means = vec![Vec4::splat(0.1); MEAN_SLOTS];
    for layer in layers {
        let index = layer["layer"].as_u64().unwrap_or(0) as usize;
        if index < MEAN_SLOTS {
            means[index] = mean_of(layer).extend(0.0);
        }
    }
    means
}

/// Hue of each layer relative to the mean of the layers of its role, scaled to luminance 1
/// (`CampaignTextures.layer_hues`): a regional soil tints the ground without changing the
/// average colour of all biomes.
pub fn layer_hues(layers: &[Value]) -> Vec<Vec4> {
    let mut hues = vec![Vec4::ONE; MEAN_SLOTS];
    let mut sums: std::collections::HashMap<String, (Vec3, f32)> = Default::default();
    for layer in layers {
        let role = layer["role"].as_str().unwrap_or("").to_string();
        let entry = sums.entry(role).or_insert((Vec3::ZERO, 0.0));
        entry.0 += mean_of(layer);
        entry.1 += 1.0;
    }
    for layer in layers {
        let role = layer["role"].as_str().unwrap_or("");
        let (sum, count) = sums[role];
        let reference = sum / count;
        let m = mean_of(layer);
        let ratio = m / reference.max(Vec3::splat(0.01));
        let luminance = ratio.dot(Vec3::new(0.2126, 0.7152, 0.0722));
        let index = layer["layer"].as_u64().unwrap_or(0) as usize;
        if index < MEAN_SLOTS {
            hues[index] = (ratio / luminance.max(0.01)).extend(1.0);
        }
    }
    hues
}

fn luminance(m: Vec3) -> f32 {
    m.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

fn pack_u32(values: &[u32], vec4s: usize) -> Vec<UVec4> {
    let mut out = vec![UVec4::ZERO; vec4s];
    for (i, v) in values.iter().enumerate() {
        if i / 4 < vec4s {
            out[i / 4][i % 4] = *v;
        }
    }
    out
}

/// Box-filters an interleaved image by an integer factor.
fn downsample(data: &[u8], width: usize, height: usize, channels: usize, factor: usize) -> Vec<u8> {
    if factor <= 1 {
        return data.to_vec();
    }
    let (ow, oh) = (width / factor, height / factor);
    let mut out = vec![0u8; ow * oh * channels];
    let area = (factor * factor) as u32;
    for y in 0..oh {
        for x in 0..ow {
            for c in 0..channels {
                let mut sum = 0u32;
                for dy in 0..factor {
                    for dx in 0..factor {
                        sum += data[((y * factor + dy) * width + x * factor + dx) * channels + c]
                            as u32;
                    }
                }
                out[(y * ow + x) * channels + c] = ((sum + area / 2) / area) as u8;
            }
        }
    }
    out
}

/// Texel format of a ground texture array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayKind {
    /// Albedo, BC1 sRGB (as the Godot VRAM-compressed import, 0.5 byte per texel).
    Bc1Srgb,
    /// Albedo or data, RGBA8 (`true`: sRGB).
    Rgba(bool),
    /// Normal map xy, Rg8.
    Rg8,
}

/// The layers of a sheet of `cols x rows` square layers (row-major, as the Godot 2D-array
/// importer slices it).
struct Sheet<'a> {
    data: &'a [u8],
    width: usize,
    cols: usize,
    /// Side of one layer (texels).
    src: usize,
}

impl<'a> Sheet<'a> {
    fn new(sheet: &'a Image, grid: (u32, u32)) -> Result<Sheet<'a>, String> {
        let data = sheet.data.as_ref().ok_or("sheet without data")?;
        let (width, height) = (sheet.width() as usize, sheet.height() as usize);
        let (cols, rows) = (grid.0 as usize, grid.1 as usize);
        if cols == 0
            || rows == 0
            || width % cols != 0
            || height % rows != 0
            || width / cols != height / rows
        {
            return Err(format!(
                "sheet {width}x{height} does not fit grid {cols}x{rows}"
            ));
        }
        Ok(Sheet {
            data,
            width,
            cols,
            src: width / cols,
        })
    }

    /// RGBA8 texels of one layer at `size` (box-filtered down, or repeated up by an integer
    /// factor when the sheet is smaller).
    fn layer(&self, layer: usize, size: usize) -> Vec<u8> {
        let src = self.src;
        let (col, row) = (layer % self.cols, layer / self.cols);
        let mut rgba = Vec::with_capacity(src * src * 4);
        for y in 0..src {
            let start = ((row * src + y) * self.width + col * src) * 4;
            rgba.extend_from_slice(&self.data[start..start + src * 4]);
        }
        if size <= src {
            return downsample(&rgba, src, src, 4, (src / size.max(1)).max(1));
        }
        let up = size / src;
        let mut out = Vec::with_capacity(size * size * 4);
        for y in 0..src * up {
            for x in 0..src * up {
                let o = ((y / up) * src + x / up) * 4;
                out.extend_from_slice(&rgba[o..o + 4]);
            }
        }
        out
    }
}

/// One layer and its mip chain in the texel format of `kind`.
fn encode_layer(rgba: Vec<u8>, size: usize, kind: ArrayKind) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut current, mut s) = (rgba, size);
    loop {
        match kind {
            ArrayKind::Bc1Srgb => out.extend(super::bc1::encode_level(&current, s, s)),
            ArrayKind::Rgba(_) => out.extend_from_slice(&current),
            ArrayKind::Rg8 => {
                out.extend(current.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1]]))
            }
        }
        if s <= 1 {
            break;
        }
        current = downsample(&current, s, s, 4, 2);
        s /= 2;
    }
    out
}

/// A texture array of `layer_count` layers of `size` texels, `produce(i)` giving the RGBA8
/// texels of layer `i`; layers are cut and encoded on a few threads, layer-major with mips.
fn build_array(
    layer_count: usize,
    size: usize,
    kind: ArrayKind,
    produce: &(dyn Fn(usize) -> Vec<u8> + Sync),
) -> Image {
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(1, 8);
    let chunk = layer_count.div_ceil(threads).max(1);
    let mut parts: Vec<Vec<u8>> = vec![Vec::new(); layer_count.div_ceil(chunk)];
    std::thread::scope(|scope| {
        for (t, part) in parts.iter_mut().enumerate() {
            scope.spawn(move || {
                for layer in t * chunk..((t + 1) * chunk).min(layer_count) {
                    part.extend(encode_layer(produce(layer), size, kind));
                }
            });
        }
    });
    let format = match kind {
        ArrayKind::Bc1Srgb => TextureFormat::Bc1RgbaUnormSrgb,
        ArrayKind::Rgba(true) => TextureFormat::Rgba8UnormSrgb,
        ArrayKind::Rgba(false) => TextureFormat::Rgba8Unorm,
        ArrayKind::Rg8 => TextureFormat::Rg8Unorm,
    };
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: size as u32,
        height: size as u32,
        depth_or_array_layers: layer_count as u32,
    };
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.format = format;
    image.texture_descriptor.mip_level_count = size.ilog2() + 1;
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.data = Some(parts.concat());
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    image
}

/// A texture array from a whole sheet, layers reduced to `max_px` at most.
#[cfg(test)]
fn sheet_to_array(
    sheet: &Image,
    grid: (u32, u32),
    layer_count: usize,
    kind: ArrayKind,
    max_px: usize,
) -> Result<Image, String> {
    let sheet = Sheet::new(sheet, grid)?;
    if layer_count > (grid.0 * grid.1) as usize {
        return Err(format!("{layer_count} layers > grid {}x{}", grid.0, grid.1));
    }
    let size = sheet.src.min(max_px.max(1));
    Ok(build_array(layer_count, size, kind, &|i| {
        sheet.layer(i, size)
    }))
}

/// GPU arrays of a layered pack (albedo BC1 + normal Rg8) and the manifest whose layer table the
/// shader uses (always the base one: the 2048 variant has the same indices).
pub struct PackArrays {
    pub albedo: Image,
    pub normal: Image,
    pub manifest: Value,
    /// Side of the albedo layers.
    pub layer_px: u32,
}

/// `CampaignTextures.pack_manifest` + `_load_pack`: the base pack `data/<rel>` or, in high
/// texture quality, its local 2048 variant (`<rel>_2048.json`, textures in `.../hi/`) when its
/// files are present (they are not in the repository). A layer missing from the variant (e.g.
/// `chestnut_grove`) comes from the base sheet, enlarged.
pub fn load_pack(
    root: &Path,
    rel: &str,
    high: bool,
    with_normal: bool,
) -> Result<PackArrays, String> {
    let base = read_json(&root.join("data").join(rel))?;
    let hi_rel = rel
        .rsplit_once('.')
        .map_or(format!("{rel}_2048"), |(b, e)| format!("{b}_2048.{e}"));
    let hi = if high {
        read_json(&root.join("data").join(&hi_rel))
            .ok()
            .filter(|m| {
                ["albedo", "normal"].iter().all(|k| {
                    let p = m[*k].as_str().unwrap_or("");
                    root.join("game")
                        .join(p.strip_prefix("res://").unwrap_or(p))
                        .is_file()
                })
            })
    } else {
        None
    };
    let grid = |m: &Value| {
        (
            m["grid"][0].as_u64().unwrap_or(1) as u32,
            m["grid"][1].as_u64().unwrap_or(1) as u32,
        )
    };
    let ids: Vec<(usize, String)> = base["layers"]
        .as_array()
        .map(|l| {
            l.iter()
                .map(|e| {
                    (
                        e["layer"].as_u64().unwrap_or(0) as usize,
                        e["id"].as_str().unwrap_or("").to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let count = ids.iter().map(|(l, _)| l + 1).max().unwrap_or(0);
    // Source of each base layer: Some(hi layer) or None (base sheet).
    let mut source: Vec<Option<usize>> = vec![None; count];
    if let Some(hi) = &hi {
        let by_id: std::collections::HashMap<&str, usize> = hi["layers"]
            .as_array()
            .map(|l| {
                l.iter()
                    .map(|e| {
                        (
                            e["id"].as_str().unwrap_or(""),
                            e["layer"].as_u64().unwrap_or(0) as usize,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        for (layer, id) in &ids {
            source[*layer] = by_id.get(id.as_str()).copied();
        }
    }
    let build = |key: &str, kind: ArrayKind, max_px: usize| -> Result<(Image, usize), String> {
        let base_img = if hi.is_none() || source.iter().any(Option::is_none) {
            Some(load_sheet(root, base[key].as_str().unwrap_or(""))?)
        } else {
            None
        };
        let hi_img = match &hi {
            Some(m) => Some(load_sheet(root, m[key].as_str().unwrap_or(""))?),
            None => None,
        };
        let base_sheet = base_img
            .as_ref()
            .map(|i| Sheet::new(i, grid(&base)))
            .transpose()?;
        let hi_sheet = match (&hi, &hi_img) {
            (Some(m), Some(i)) => Some(Sheet::new(i, grid(m))?),
            _ => None,
        };
        let src = hi_sheet
            .as_ref()
            .or(base_sheet.as_ref())
            .map_or(1, |s| s.src);
        let size = src.min(max_px);
        let produce = |i: usize| match (source[i], &hi_sheet, &base_sheet) {
            (Some(h), Some(s), _) => s.layer(h, size),
            (_, _, Some(b)) => b.layer(i, size),
            _ => vec![128; size * size * 4],
        };
        Ok((build_array(count, size, kind, &produce), size))
    };
    let (albedo, layer_px) = build("albedo", ArrayKind::Bc1Srgb, usize::MAX)?;
    let normal = if with_normal {
        build("normal", ArrayKind::Rg8, NORMAL_MAX_PX)?.0
    } else {
        Image::default()
    };
    Ok(PackArrays {
        albedo,
        normal,
        manifest: base,
        layer_px: layer_px as u32,
    })
}

fn load_sheet(root: &Path, res_path: &str) -> Result<Image, String> {
    let rel = res_path.strip_prefix("res://").unwrap_or(res_path);
    let path = root.join("game").join(rel);
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Image::from_buffer(
        &bytes,
        ImageType::Extension("jpg"),
        CompressedImageFormats::NONE,
        false,
        ImageSampler::Default,
        RenderAssetUsages::MAIN_WORLD,
    )
    .map_err(|e| format!("{}: {e}", path.display()))
}

/// Decodes an 8-bit PNG into `(width, height, channels, bytes)` (grayscale 1, RGBA 4).
fn decode_png8(path: &Path) -> Result<(usize, usize, usize, Vec<u8>), String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(BufReader::new(file))
        .read_info()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let info = reader.info();
    if info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{}: not 8-bit", path.display()));
    }
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => return Err(format!("{}: unsupported {other:?}", path.display())),
    };
    let (width, height) = (info.width as usize, info.height as usize);
    let mut bytes = vec![
        0u8;
        reader
            .output_buffer_size()
            .unwrap_or(width * height * channels)
    ];
    reader
        .next_frame(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((width, height, channels, bytes))
}

fn flat_image(
    width: usize,
    height: usize,
    format: TextureFormat,
    data: Vec<u8>,
    linear_clamp: bool,
) -> Image {
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: width as u32,
        height: height as u32,
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.format = format;
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.data = Some(data);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: if linear_clamp {
            ImageFilterMode::Linear
        } else {
            ImageFilterMode::Nearest
        },
        min_filter: if linear_clamp {
            ImageFilterMode::Linear
        } else {
            ImageFilterMode::Nearest
        },
        ..default()
    });
    image
}

/// Biome blend + landscape + river rasters merged into one half-resolution Rgba8 image:
/// R biome A + 16 B (`biomes_blend_ab.png`, nearest), G distance to the neighbour biome
/// (`biomes_blend_dist.png`), B agricultural landscape mask (`agri_regions.png`, 4 px per texel,
/// nearest), A signed distance to the river bed (`river_bed.png`, 128 + 16 per px, linear).
fn load_blend(map_dir: &Path) -> Result<Image, String> {
    let (w, h, c, ab) = decode_png8(&map_dir.join("biomes_blend_ab.png"))?;
    let (w2, h2, c2, dist) = decode_png8(&map_dir.join("biomes_blend_dist.png"))?;
    if (w, h, c) != (w2, h2, c2) || c != 1 {
        return Err("biome blend rasters differ in size".into());
    }
    let (aw, ah, ac, agri) = decode_png8(&map_dir.join("agri_regions.png"))?;
    if ac != 1 {
        return Err("agri_regions.png is not grayscale".into());
    }
    let (rw, rh, rc, river) = decode_png8(&map_dir.join("river_bed.png"))?;
    if rc != 1 {
        return Err("river_bed.png is not grayscale".into());
    }
    let mut data = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let agri_value = agri[(y * ah / h).min(ah - 1) * aw + (x * aw / w).min(aw - 1)];
            let river_value = river[(y * rh / h).min(rh - 1) * rw + (x * rw / w).min(rw - 1)];
            data.extend_from_slice(&[ab[i], dist[i], agri_value, river_value]);
        }
    }
    Ok(flat_image(w, h, TextureFormat::Rgba8Unorm, data, true))
}

/// `splat.png` (R grass, G farmland, B forest, A rock/heath) at half resolution (one texel is
/// 2 x 719 m; the shader filters it bilinearly).
fn load_splat(map_dir: &Path) -> Result<Image, String> {
    let (w, h, c, data) = decode_png8(&map_dir.join("splat.png"))?;
    if c != 4 {
        return Err("splat.png is not RGBA".into());
    }
    let half = downsample(&data, w, h, 4, 2);
    Ok(flat_image(
        w / 2,
        h / 2,
        TextureFormat::Rgba8Unorm,
        half,
        true,
    ))
}

fn vec4_of(value: &Value, fallback: Vec4) -> Vec4 {
    match value.as_array() {
        Some(a) if a.len() == 4 => Vec4::from_array(std::array::from_fn(|i| {
            a[i].as_f64().unwrap_or(fallback[i] as f64) as f32
        })),
        _ => fallback,
    }
}

/// GA4 macro variation (`macro` of `campaign_terrain_textures.json`, `CampaignTextures.apply_terrain`).
pub fn apply_macro(block: &Value, params: &mut GroundParams) {
    if block.is_null() {
        return;
    }
    params.macro_periods = vec4_of(&block["periods_px"], params.macro_periods);
    params.macro_weights = vec4_of(&block["weights"], params.macro_weights);
    params.macro_amp.x = block["tint_amp"].as_f64().unwrap_or(1.0) as f32;
    params.macro_amp.y = block["lum_amp"].as_f64().unwrap_or(0.0) as f32;
}

/// TB6 dark masses (`forest_masses` of `data/ui/campaign_map.json`, `sg_dark_*`).
pub fn apply_dark_masses(block: &Value, params: &mut GroundParams) {
    if block.is_null() {
        return;
    }
    let num = |key: &str, fallback: f32| block[key].as_f64().map_or(fallback, |v| v as f32);
    params.dark = Vec4::new(
        num("floor", 0.0),
        num("knee", 2.0),
        num("feather", 2.5),
        num("keep", 0.25),
    );
    params.macro_amp.w = num("canopy", 0.12);
}

/// Loads the whole regional ground; `Err` when the settings or a file is missing.
pub fn load_ground(root: &Path, high: bool) -> Result<GroundData, String> {
    let spec = read_json(&root.join("data/fx/campaign_terrain_textures.json"))?;
    let regional = &spec["regional"];
    if regional.is_null() {
        return Err("campaign_terrain_textures.json has no `regional` block".into());
    }
    let map_dir = root.join("data/map");
    let parents = load_parents(root);
    let pack = load_pack(root, regional["pack"].as_str().unwrap_or(""), high, true)?;
    let manifest = pack.manifest;
    let layers: Vec<Value> = manifest["layers"].as_array().cloned().unwrap_or_default();
    let (bg_albedo, bg_normal) = (pack.albedo, pack.normal);

    let mut params = GroundParams::default();
    let table = build_layer_table(&layers, regional, &parents);
    params.layers = pack_u32(&table, 27).try_into().expect("27 vec4");
    params.means = padded_means(&layers).try_into().expect("48 means");
    params.hues = layer_hues(&layers).try_into().expect("48 hues");
    let ab_dist = load_blend(&map_dir)?;
    let splat = load_splat(&map_dir)?;
    let meta = read_json(&map_dir.join("map.json"))?;
    let meters_per_px = meta["meters_per_px"].as_f64().unwrap_or(719.0) as f32;
    let blend_texel_px = regional["blend_texel_px"].as_f64().unwrap_or(2.0) as f32;
    let texel_km = blend_texel_px * meters_per_px / 1000.0;
    params.bg = Vec4::new(
        1.0,
        regional["blend_km"].as_f64().unwrap_or(10.0) as f32 / texel_km.max(1e-3),
        regional["min_blend_weight"].as_f64().unwrap_or(0.03) as f32,
        regional["hue_keep"].as_f64().unwrap_or(0.0) as f32,
    );
    let tile_screen = spec["tile_screen_px"].as_f64().unwrap_or(72.0) as f32;
    params.tiling.x = tile_screen;
    params.micro.w = meters_per_px;
    params.splat = Vec4::new(1.0, 1700.0, 0.0, 0.0);
    apply_macro(&spec["macro"], &mut params);
    if let Ok(ui) = read_json(&root.join("data/ui/campaign_map.json")) {
        apply_dark_masses(&ui["forest_masses"], &mut params);
    }

    // Micro grain.
    let mut micro_albedo = None;
    let micro = &spec["micro"];
    if !micro.is_null() {
        let micro_manifest =
            read_json(&root.join("data").join(micro["pack"].as_str().unwrap_or("")))?;
        let micro_layers: Vec<Value> = micro_manifest["layers"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        micro_albedo =
            Some(load_pack(root, micro["pack"].as_str().unwrap_or(""), high, false)?.albedo);
        let mut means = [0.18f32; 8];
        let mut by_id = std::collections::HashMap::new();
        for layer in &micro_layers {
            let index = layer["layer"].as_u64().unwrap_or(0) as usize;
            by_id.insert(layer["id"].as_str().unwrap_or("").to_string(), index as u32);
            if index < 8 {
                means[index] = luminance(mean_of(layer));
            }
        }
        let roles: Vec<u32> = ROLES
            .iter()
            .map(|role| {
                micro["roles"][role]
                    .as_str()
                    .and_then(|id| by_id.get(id).copied())
                    .unwrap_or(0)
            })
            .collect();
        params.micro_layers = pack_u32(&roles, 2).try_into().expect("2 vec4");
        params.micro_mean = [
            Vec4::new(means[0], means[1], means[2], means[3]),
            Vec4::new(means[4], means[5], means[6], means[7]),
        ];
        let fade = &micro["fade_footprint"];
        params.micro = Vec4::new(
            1.0,
            micro["tile_m"].as_f64().unwrap_or(10.0) as f32,
            micro["strength"].as_f64().unwrap_or(0.55) as f32,
            meters_per_px,
        );
        params.fades.z = fade[0].as_f64().unwrap_or(0.004) as f32;
        params.fades.w = fade[1].as_f64().unwrap_or(0.02) as f32;
    }
    let mut fields = super::fields::load_fields(root)?;
    let (parcels_albedo, parcels_normal) = match load_parcels(root, high) {
        Ok((albedo, normal)) => {
            fields.tex_flags.z = 1.0;
            (Some(albedo), Some(normal))
        }
        Err(message) => {
            warn!("campaign terrain: parcels pack: {message}");
            (None, None)
        }
    };
    // RV-D valley occlusion (`MapData` mask `relief_occlusion.png`, `TerrainBuilder` mipmapped).
    let occlusion = match decode_png8(&map_dir.join("relief_occlusion.png")) {
        Ok((w, h, 1, bytes)) => {
            params.occlusion.x = 1.0;
            let mut image = flat_image(w, h, TextureFormat::R8Unorm, bytes, true);
            image.sampler = ImageSampler::linear();
            Some(image)
        }
        Ok(_) | Err(_) => {
            warn!("campaign terrain: relief_occlusion.png missing or not L8");
            None
        }
    };
    Ok(GroundData {
        occlusion,
        bg_albedo,
        bg_normal,
        ab_dist,
        splat,
        micro_albedo,
        params,
        fields,
        parcels_albedo,
        parcels_normal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        crate::paths::find_root().expect("repository root")
    }

    #[test]
    fn layer_table_maps_every_biome_and_role_to_a_real_layer() {
        let root = root();
        let spec = read_json(&root.join("data/fx/campaign_terrain_textures.json")).unwrap();
        let manifest = read_json(&root.join("data/art/tx_campaign_bg_pack.json")).unwrap();
        let layers = manifest["layers"].as_array().unwrap();
        let table = build_layer_table(layers, &spec["regional"], &load_parents(&root));
        assert_eq!(table.len(), BIOME_COUNT * 7);
        assert!(table.iter().all(|l| (*l as usize) < layers.len()));
        // Biome 1 grass is the `grass_bare_b01` layer 0; the sea row copies biome 1.
        assert_eq!(table[7], 0);
        assert_eq!(table[0], table[7]);
        // Biome 1 heath is overridden to `understory` (layer 1).
        assert_eq!(table[7 + 4], 1);
        // A biome whose own grass layer exists in the pack uses it.
        for layer in layers {
            if layer["role"].as_str() == Some("grass_bare") {
                for biome in layer["biomes"].as_array().unwrap() {
                    let biome = biome.as_u64().unwrap() as usize;
                    assert_eq!(table[biome * 7], layer["layer"].as_u64().unwrap() as u32);
                }
            }
        }
    }

    #[test]
    fn hues_have_unit_luminance() {
        let manifest = read_json(&root().join("data/art/tx_campaign_bg_pack.json")).unwrap();
        let layers = manifest["layers"].as_array().unwrap();
        let hues = layer_hues(layers);
        for h in hues.iter().take(layers.len()) {
            let l = luminance(h.truncate());
            assert!((l - 1.0).abs() < 1e-3, "{l}");
        }
    }

    #[test]
    fn array_slicing_builds_a_layer_major_chain_with_mips() {
        // 2 x 2 sheet of 4 x 4 layers, solid colours 10, 20, 30, 40.
        let mut data = Vec::new();
        for y in 0..8usize {
            for x in 0..8usize {
                let v = ((y / 4) * 2 + x / 4 + 1) as u8 * 10;
                data.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let mut sheet = Image::default();
        sheet.texture_descriptor.size = Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        };
        sheet.texture_descriptor.format = TextureFormat::Rgba8UnormSrgb;
        sheet.data = Some(data);
        let array = sheet_to_array(&sheet, (2, 2), 4, ArrayKind::Rgba(true), 4).unwrap();
        let bytes = array.data.as_ref().unwrap();
        // 4x4 + 2x2 + 1x1 texels of 4 bytes per layer.
        assert_eq!(bytes.len(), 4 * (16 + 4 + 1) * 4);
        assert_eq!(array.texture_descriptor.mip_level_count, 3);
        let per_layer = (16 + 4 + 1) * 4;
        assert_eq!(bytes[0], 10);
        assert_eq!(bytes[per_layer], 20);
        assert_eq!(bytes[3 * per_layer], 40);
        // Last mip of layer 2 keeps the solid colour.
        assert_eq!(bytes[3 * per_layer - 4], 30);
    }

    #[test]
    fn real_ground_loads_and_fits_the_shader_tables() {
        let data = load_ground(&root(), false).expect("regional ground");
        assert_eq!(
            data.bg_albedo.texture_descriptor.size.depth_or_array_layers,
            45
        );
        assert_eq!(
            data.bg_normal.texture_descriptor.size.depth_or_array_layers,
            45
        );
        assert_eq!(data.params.bg.x, 1.0);
        assert!(data.params.bg.y > 1.0);
        assert!(data.micro_albedo.is_some());
        assert_eq!(data.splat.width(), 3540);
        assert_eq!(data.ab_dist.width(), 3540);
    }
}
