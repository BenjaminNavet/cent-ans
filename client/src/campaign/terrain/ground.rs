//! Regional ground of the campaign map (TX, ADR 0243): the 45-layer background pack
//! (`tx_campaign_bg_*`, role x biome), the biome blend rasters (`biomes_blend_ab/dist.png`), the
//! soil-material splat (`splat.png`) and the micro grain pack (`tx_micro_ground_*`). Port of
//! `campaign_textures.gd` + `biome_parents.gd` (layer table, hues) and of the splat / sample
//! parts of `terrain.gdshader`; the shading is in `terrain.wesl` (`ground_proc`).
//!
//! Loading runs off the main thread (JPEG sheets are cut into texture arrays, halved to 512 px
//! per layer, mip chains built on the CPU) and the result is bound to [`TerrainMaterial`] by
//! `poll_ground` in `mod.rs`. Settings: `data/fx/campaign_terrain_textures.json`.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::{
    CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor,
    ImageType,
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
/// Side of one layer of the albedo / normal arrays on the GPU.
pub const LAYER_PX: u32 = 512;

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
    /// Splat bound (x), rock line (m), blend texel size in map px (y of the next vec is unused).
    pub splat: Vec4,
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
            splat: Vec4::new(0.0, 1700.0, 2.0, 0.0),
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

/// Appends the mip chain (down to 1 x 1) of a square layer after its base level.
fn with_mips(base: Vec<u8>, size: usize, channels: usize) -> Vec<u8> {
    let mut out = base.clone();
    let (mut current, mut s) = (base, size);
    while s > 1 {
        current = downsample(&current, s, s, channels, 2);
        s /= 2;
        out.extend_from_slice(&current);
    }
    out
}

/// A texture array from a sheet of `cols x rows` layers (row-major, as the Godot 2D-array
/// importer slices it): layers halved to [`LAYER_PX`] at most, `out_channels` 4 (RGBA8 sRGB or
/// linear) or 2 (Rg8), layer-major data with mips.
fn sheet_to_array(
    sheet: &Image,
    grid: (u32, u32),
    layer_count: usize,
    out_channels: usize,
    srgb: bool,
) -> Result<Image, String> {
    let data = sheet.data.as_ref().ok_or("sheet without data")?;
    let (width, height) = (sheet.width() as usize, sheet.height() as usize);
    let (cols, rows) = (grid.0 as usize, grid.1 as usize);
    if cols == 0 || rows == 0 || width % cols != 0 || height % rows != 0 || width / cols != height / rows
    {
        return Err(format!("sheet {width}x{height} does not fit grid {cols}x{rows}"));
    }
    if layer_count > cols * rows {
        return Err(format!("{layer_count} layers > grid {cols}x{rows}"));
    }
    let src = width / cols;
    let factor = (src / LAYER_PX as usize).max(1);
    let size = src / factor;
    let mut bytes = Vec::new();
    for layer in 0..layer_count {
        let (col, row) = (layer % cols, layer / cols);
        let mut rgba = Vec::with_capacity(src * src * 4);
        for y in 0..src {
            let start = ((row * src + y) * width + col * src) * 4;
            rgba.extend_from_slice(&data[start..start + src * 4]);
        }
        let reduced = downsample(&rgba, src, src, 4, factor);
        let packed = if out_channels == 2 {
            reduced
                .chunks_exact(4)
                .flat_map(|p| [p[0], p[1]])
                .collect()
        } else {
            reduced
        };
        bytes.extend(with_mips(packed, size, out_channels));
    }
    let format = match (out_channels, srgb) {
        (2, _) => TextureFormat::Rg8Unorm,
        (_, true) => TextureFormat::Rgba8UnormSrgb,
        _ => TextureFormat::Rgba8Unorm,
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
    image.data = Some(bytes);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    Ok(image)
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
    let mut bytes = vec![0u8; reader.output_buffer_size().unwrap_or(width * height * channels)];
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

/// Loads the whole regional ground; `Err` when the settings or a file is missing.
pub fn load_ground(root: &Path) -> Result<GroundData, String> {
    let spec = read_json(&root.join("data/fx/campaign_terrain_textures.json"))?;
    let regional = &spec["regional"];
    if regional.is_null() {
        return Err("campaign_terrain_textures.json has no `regional` block".into());
    }
    let map_dir = root.join("data/map");
    let parents = load_parents(root);
    let manifest = read_json(&root.join("data").join(regional["pack"].as_str().unwrap_or("")))?;
    let layers: Vec<Value> = manifest["layers"].as_array().cloned().unwrap_or_default();
    let grid = |m: &Value| {
        (
            m["grid"][0].as_u64().unwrap_or(1) as u32,
            m["grid"][1].as_u64().unwrap_or(1) as u32,
        )
    };
    let albedo_sheet = load_sheet(root, manifest["albedo"].as_str().unwrap_or(""))?;
    let normal_sheet = load_sheet(root, manifest["normal"].as_str().unwrap_or(""))?;
    let bg_albedo = sheet_to_array(&albedo_sheet, grid(&manifest), layers.len(), 4, true)?;
    drop(albedo_sheet);
    let bg_normal = sheet_to_array(&normal_sheet, grid(&manifest), layers.len(), 2, false)?;
    drop(normal_sheet);

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

    // Micro grain.
    let mut micro_albedo = None;
    let micro = &spec["micro"];
    if !micro.is_null() {
        let micro_manifest = read_json(&root.join("data").join(micro["pack"].as_str().unwrap_or("")))?;
        let micro_layers: Vec<Value> = micro_manifest["layers"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let sheet = load_sheet(root, micro_manifest["albedo"].as_str().unwrap_or(""))?;
        micro_albedo = Some(sheet_to_array(
            &sheet,
            grid(&micro_manifest),
            micro_layers.len(),
            4,
            true,
        )?);
        let mut means = vec![0.18f32; 8];
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
    Ok(GroundData {
        bg_albedo,
        bg_normal,
        ab_dist,
        splat,
        micro_albedo,
        params,
        fields: super::fields::load_fields(root)?,
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
        // Biome 8 (desert) has no layer of its own for grass: climbs to semi-arid (7).
        let b7_grass = table[7 * 7];
        assert_eq!(table[8 * 7], b7_grass);
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
        let array = sheet_to_array(&sheet, (2, 2), 4, 4, true).unwrap();
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
        let data = load_ground(&root()).expect("regional ground");
        assert_eq!(data.bg_albedo.texture_descriptor.size.depth_or_array_layers, 45);
        assert_eq!(data.bg_normal.texture_descriptor.size.depth_or_array_layers, 45);
        assert_eq!(data.params.bg.x, 1.0);
        assert!(data.params.bg.y > 1.0);
        assert!(data.micro_albedo.is_some());
        assert_eq!(data.splat.width(), 3540);
        assert_eq!(data.ab_dist.width(), 3540);
    }
}
