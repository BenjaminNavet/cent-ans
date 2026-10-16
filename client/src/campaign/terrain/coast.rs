//! Coast of the campaign ground (lot coastterr): port of `coast_look.gd` (data side of TB5),
//! `coast_common.gdshaderinc` + `coast_band.gdshaderinc` (cliffs and beaches drawn by
//! `terrain.wesl`, hook `coast_band`) and the glaciers / névés of `freshwater_ground.gdshaderinc`
//! (DN-ME4). Purely visual: no game rule.
//!
//! Data: `data/map/coast_types.json` (schema `data/schemas/coast_types.schema.json`): geology by
//! region (rock: chalk, granite or rock; beach: sand or shingle), slope rule (altitude read
//! `cliff.probe_px` inland of the shore: beach below `cliff.min_m`, cliff above `cliff.max_m`),
//! band widths, colours, swash. `data/map/coast_dist.png` (R8, map px): signed distance to the
//! coast, `(v - 128) / 2` px, > 0 on land. `data/map/map_freshwater.json` `glacier`.
//!
//! Textures bound to [`TerrainMaterial`](super::material::TerrainMaterial):
//! * `coast_dist` (R8, full map resolution, linear);
//! * `coast_probe` (Rg8, half resolution): R altitude (m, 0-255) at the slope-rule probe of the
//!   nearest shore, G 1 on a lake shore. Godot reads the heightmap at the probe in the fragment
//!   shader (`height_m_lod(coast_probe_uv(...))`); here it is baked once on the CPU (same rule,
//!   same sample points), which spares the full-resolution heightmap on the GPU;
//! * `coast_types` (Rgba8, one texel per `cell_px`): R chalk, G granite, B shingle, A beach width.

use std::path::Path;
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, ShaderType, TextureDimension, TextureFormat};
use serde_json::Value;

use super::relief_floor::HeightGrid;

/// Order of the types in the shader arrays (`coast_color`, `coast_shade`, `coast_streak`).
pub const TYPES: [&str; 5] = ["rock", "chalk", "granite", "sand", "shingle"];
/// Map px per texel of the probe texture.
pub const PROBE_DOWNSAMPLE: usize = 2;
/// Coast distance (px) beyond which no probe is baked (band reach + filtering margin).
pub const PROBE_REACH_PX: f32 = 5.0;
/// `lake_level_m`, `lake_bank_px`, `pond_max_px`, `pond_probe_px`, `lake_sea_probe_px` of
/// terrain.gdshader (HB7): water whose ground is above `lake_level_m` is a lake unless the sea
/// is near.
const LAKE_LEVEL_M: f32 = 2.0;
const LAKE_BANK_PX: f32 = 1.6;
const POND_MAX_PX: f32 = 3.0;
const POND_PROBE_PX: f32 = 6.0;
const LAKE_SEA_PROBE_PX: f32 = 4.0;

/// Coast block of the ground uniforms (`CoastParams` of `terrain.wesl`).
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct CoastParams {
    /// 1 when the data is loaded and the textures bound, probe px, cliff min m, cliff max m.
    pub cliff: Vec4,
    /// Cliff px, cliff widen, sea reach px, beach px.
    pub band_a: Vec4,
    /// Wet px, strength, relief, far keep.
    pub band_b: Vec4,
    /// Swash amount, period s, out px, in px.
    pub swash_a: Vec4,
    /// Swash edge px, film px, cliff share, 0.
    pub swash_b: Vec4,
    /// Swash foam colour (linear), 0.
    pub foam: Vec4,
    /// Per type (rock, chalk, granite, sand, shingle): rgb colour, a streak.
    pub color: [Vec4; 5],
    /// Per type: rgb shade.
    pub shade: [Vec4; 5],
    /// Coast distance texture size (px), 1 / scale of the stored distance, 0.
    pub texel: Vec4,
}

impl Default for CoastParams {
    fn default() -> Self {
        // Shader defaults of coast_band / coast_common (`coast_enabled` false).
        CoastParams {
            cliff: Vec4::new(0.0, 1.6, 36.0, 52.0),
            band_a: Vec4::new(0.85, 2.6, 3.0, 0.55),
            band_b: Vec4::new(0.16, 1.0, 0.9, 0.55),
            swash_a: Vec4::new(0.0, 7.0, 0.45, 0.22),
            swash_b: Vec4::new(0.07, 0.3, 0.35, 0.0),
            foam: Vec4::new(0.62, 0.66, 0.66, 0.0),
            color: [Vec4::new(0.4, 0.37, 0.32, 0.3); 5],
            shade: [Vec4::new(0.2, 0.18, 0.16, 0.0); 5],
            texel: Vec4::new(1.0, 1.0, 2.0, 0.0),
        }
    }
}

/// Glacier block (`fw_glacier`, `fw_ice_color`, `fw_crevasse_color`, `fw_neve_color` of
/// freshwater_ground.gdshaderinc).
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct GlacierParams {
    /// Low limit (m), full cover (m), max slope (m/m), strength (0 = off).
    pub line: Vec4,
    pub ice: Vec4,
    pub crevasse: Vec4,
    pub neve: Vec4,
}

impl Default for GlacierParams {
    fn default() -> Self {
        GlacierParams {
            line: Vec4::new(2600.0, 3100.0, 0.62, 1.0),
            ice: Vec4::new(0.70, 0.80, 0.88, 0.0),
            crevasse: Vec4::new(0.38, 0.50, 0.62, 0.0),
            neve: Vec4::new(0.90, 0.93, 0.97, 0.0),
        }
    }
}

impl GlacierParams {
    /// `glacier` block of `map_freshwater.json` (`FreshwaterLayer._apply_glacier`).
    pub fn from_json(data: &Value) -> GlacierParams {
        let mut out = GlacierParams::default();
        let glacier = &data["glacier"];
        if !glacier.is_object() {
            return out;
        }
        let num = |key: &str, fallback: f32| glacier[key].as_f64().map_or(fallback, |v| v as f32);
        out.line = Vec4::new(
            num("line_m", out.line.x),
            num("full_m", out.line.y),
            num("max_slope", out.line.z),
            1.0,
        );
        out.ice = vec3_of(&glacier["ice_color"], out.ice.truncate()).extend(0.0);
        out.crevasse = vec3_of(&glacier["crevasse_color"], out.crevasse.truncate()).extend(0.0);
        out
    }
}

fn vec3_of(value: &Value, fallback: Vec3) -> Vec3 {
    match value.as_array() {
        Some(a) if a.len() >= 3 => Vec3::new(
            a[0].as_f64().unwrap_or(0.0) as f32,
            a[1].as_f64().unwrap_or(0.0) as f32,
            a[2].as_f64().unwrap_or(0.0) as f32,
        ),
        _ => fallback,
    }
}

/// One region of `coast_types.json` (or the default): rock, beach, beach width, polygon (map px).
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub id: String,
    pub rock: String,
    pub beach: String,
    pub beach_width: f32,
    pub polygon: Vec<Vec2>,
}

impl Region {
    fn from_json(entry: &Value) -> Region {
        Region {
            id: entry["id"].as_str().unwrap_or("").to_string(),
            rock: entry["rock"].as_str().unwrap_or("rock").to_string(),
            beach: entry["beach"].as_str().unwrap_or("sand").to_string(),
            beach_width: entry["beach_width"].as_f64().unwrap_or(1.0) as f32,
            polygon: entry["polygon_px"]
                .as_array()
                .map(|points| {
                    points
                        .iter()
                        .filter_map(|p| {
                            let p = p.as_array()?;
                            Some(Vec2::new(
                                p.first()?.as_f64()? as f32,
                                p.get(1)?.as_f64()? as f32,
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Geology texel (`CoastLook._geology_color`): R chalk, G granite, B shingle, A beach width.
    pub fn geology(&self) -> [u8; 4] {
        let unit = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        [
            if self.rock == "chalk" { 255 } else { 0 },
            if self.rock == "granite" { 255 } else { 0 },
            if self.beach == "shingle" { 255 } else { 0 },
            unit((self.beach_width - 0.5) / 1.5),
        ]
    }
}

/// Even-odd point in polygon (`Geometry2D.is_point_in_polygon`).
pub fn point_in_polygon(p: Vec2, polygon: &[Vec2]) -> bool {
    let mut inside = false;
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// `coast_types.json` read.
#[derive(Clone, Debug)]
pub struct CoastData {
    pub cell_px: u32,
    pub default: Region,
    pub regions: Vec<Region>,
    pub params: CoastParams,
}

impl CoastData {
    pub fn from_json(data: &Value) -> Option<CoastData> {
        if !data.is_object() {
            return None;
        }
        let mut params = CoastParams::default();
        let cliff = &data["cliff"];
        let num = |block: &Value, key: &str, fallback: f32| {
            block[key].as_f64().map_or(fallback, |v| v as f32)
        };
        params.cliff = Vec4::new(
            1.0,
            num(cliff, "probe_px", params.cliff.y),
            num(cliff, "min_m", params.cliff.z),
            num(cliff, "max_m", params.cliff.w),
        );
        let band = &data["band"];
        params.band_a = Vec4::new(
            num(band, "cliff_px", params.band_a.x),
            num(band, "cliff_widen", params.band_a.y),
            num(band, "sea_reach_px", params.band_a.z),
            num(band, "beach_px", params.band_a.w),
        );
        params.band_b = Vec4::new(
            num(band, "wet_px", params.band_b.x),
            num(band, "strength", params.band_b.y),
            num(band, "relief", params.band_b.z),
            num(band, "far_keep", params.band_b.w),
        );
        let swash = &data["swash"];
        params.swash_a = Vec4::new(
            num(swash, "amount", params.swash_a.x),
            num(swash, "period_s", params.swash_a.y),
            num(swash, "out_px", params.swash_a.z),
            num(swash, "in_px", params.swash_a.w),
        );
        params.swash_b = Vec4::new(
            num(swash, "edge_px", params.swash_b.x),
            num(swash, "film_px", params.swash_b.y),
            num(swash, "cliff", params.swash_b.z),
            0.0,
        );
        for (i, name) in TYPES.iter().enumerate() {
            let entry = &data["types"][*name];
            let color = vec3_of(&entry["color"], Vec3::new(0.4, 0.37, 0.32));
            let shade = vec3_of(&entry["shade"], Vec3::new(0.2, 0.18, 0.16));
            params.color[i] = color.extend(num(entry, "streak", 0.3));
            params.shade[i] = shade.extend(0.0);
        }
        let mut default = Region::from_json(&data["default"]);
        default.polygon.clear();
        Some(CoastData {
            cell_px: data["cell_px"].as_u64().unwrap_or(16).max(1) as u32,
            default,
            regions: data["regions"]
                .as_array()
                .map(|r| r.iter().map(Region::from_json).collect())
                .unwrap_or_default(),
            params,
        })
    }

    /// Region at the map point `p` (`CoastLook.region_at`): the last region containing it.
    pub fn region_at(&self, p: Vec2) -> &Region {
        self.regions
            .iter()
            .rev()
            .find(|r| point_in_polygon(p, &r.polygon))
            .unwrap_or(&self.default)
    }

    /// Geology raster (`CoastLook.texture`): one texel per `cell_px`, centre inside the polygon.
    pub fn geology_texels(&self, map_w: u32, map_h: u32) -> (UVec2, Vec<u8>) {
        let size = UVec2::new(map_w.div_ceil(self.cell_px), map_h.div_ceil(self.cell_px));
        let mut data = Vec::with_capacity((size.x * size.y * 4) as usize);
        let cell = self.cell_px as f32;
        for row in 0..size.y {
            for col in 0..size.x {
                let centre = Vec2::new((col as f32 + 0.5) * cell, (row as f32 + 0.5) * cell);
                data.extend_from_slice(&self.region_at(centre).geology());
            }
        }
        (size, data)
    }
}

/// Signed distance raster (`coast_dist.png`), sampled like the GPU (texel i centred on i + 0.5).
pub struct CoastDist {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl CoastDist {
    fn texel(&self, x: i64, y: i64) -> f32 {
        let x = x.clamp(0, self.width as i64 - 1) as usize;
        let y = y.clamp(0, self.height as i64 - 1) as usize;
        (self.data[y * self.width + x] as f32 - 128.0) / 2.0
    }

    /// Signed distance (px, > 0 on land) at map point (x, y), bilinear.
    pub fn at(&self, x: f32, y: f32) -> f32 {
        let fx = x - 0.5;
        let fy = y - 0.5;
        let (i, j) = (fx.floor() as i64, fy.floor() as i64);
        let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
        let top = self.texel(i, j) * (1.0 - tx) + self.texel(i + 1, j) * tx;
        let bottom = self.texel(i, j + 1) * (1.0 - tx) + self.texel(i + 1, j + 1) * tx;
        top * (1.0 - ty) + bottom * ty
    }

    /// Unit direction towards the land (`coast_to_land`), zero on a flat patch.
    pub fn to_land(&self, x: f32, y: f32) -> Vec2 {
        let g = Vec2::new(
            self.at(x + 1.0, y) - self.at(x - 1.0, y),
            self.at(x, y + 1.0) - self.at(x, y - 1.0),
        );
        if g.length() < 1e-5 {
            Vec2::ZERO
        } else {
            g.normalize()
        }
    }
}

/// Decodes an 8-bit grayscale PNG.
pub fn decode_gray8(path: &Path) -> Result<CoastDist, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let info = reader.info();
    if info.color_type != png::ColorType::Grayscale || info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{}: not an 8-bit grayscale PNG", path.display()));
    }
    let (width, height) = (info.width as usize, info.height as usize);
    let mut data = vec![0u8; reader.output_buffer_size().unwrap_or(width * height)];
    reader
        .next_frame(&mut data)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    data.truncate(width * height);
    Ok(CoastDist {
        width,
        height,
        data,
    })
}

/// Altitude (m) at map point (x, y) read like a GPU texture of the heightmap.
fn height_at(heights: &HeightGrid, x: f32, y: f32) -> f32 {
    heights.bilinear_m(x - 0.5, y - 0.5)
}

/// `sea_nearby` of terrain.gdshader: 1 when the ground dips below sea level `LAKE_SEA_PROBE_PX`
/// around `p` (8 directions).
fn sea_nearby(heights: &HeightGrid, p: Vec2) -> bool {
    (0..8).any(|k| {
        let a = k as f32 * std::f32::consts::FRAC_PI_4;
        let q = p + Vec2::new(a.cos(), a.sin()) * LAKE_SEA_PROBE_PX;
        height_at(heights, q.x, q.y) < 0.0
    })
}

/// Probe of the slope rule at `p` (`coast_probe_uv` + `height_m_lod`) and the lake-shore test of
/// terrain.gdshader (HB7): (altitude m at the probe, lake shore). `None` far from any coast.
pub fn probe_at(
    coast: &CoastDist,
    heights: &HeightGrid,
    probe_px: f32,
    p: Vec2,
) -> Option<(f32, bool)> {
    let c = coast.at(p.x, p.y);
    if c.abs() > PROBE_REACH_PX {
        return None;
    }
    let to_land = coast.to_land(p.x, p.y);
    let probe = p + to_land * (probe_px - c);
    let h = height_at(heights, probe.x, probe.y);
    let mut lake = false;
    if c < LAKE_BANK_PX + 1.5 && c > -POND_MAX_PX - 1.0 {
        let to_water = -to_land;
        let near = p + to_water * (c.max(0.0) + 1.5);
        let far = p + to_water * (c.max(0.0) + POND_PROBE_PX);
        let far_coast = coast.at(far.x, far.y);
        lake = (height_at(heights, near.x, near.y) >= LAKE_LEVEL_M && !sea_nearby(heights, near))
            || far_coast >= -POND_MAX_PX;
    }
    Some((h, lake))
}

/// Probe texture data (Rg8, one texel per `PROBE_DOWNSAMPLE` map px): R altitude (m, clamped
/// 0-255), G 255 on a lake shore. Returns (size, data, texels near a coast).
pub fn bake_probe(coast: &CoastDist, heights: &HeightGrid, probe_px: f32) -> (UVec2, Vec<u8>, u32) {
    let size = UVec2::new(
        coast.width.div_ceil(PROBE_DOWNSAMPLE) as u32,
        coast.height.div_ceil(PROBE_DOWNSAMPLE) as u32,
    );
    let mut data = vec![0u8; (size.x * size.y * 2) as usize];
    let step = PROBE_DOWNSAMPLE as f32;
    let mut near = 0;
    for row in 0..size.y {
        for col in 0..size.x {
            let p = Vec2::new((col as f32 + 0.5) * step, (row as f32 + 0.5) * step);
            if let Some((h, lake)) = probe_at(coast, heights, probe_px, p) {
                let i = ((row * size.x + col) * 2) as usize;
                data[i] = h.clamp(0.0, 255.0).round() as u8;
                data[i + 1] = if lake { 255 } else { 0 };
                near += 1;
            }
        }
    }
    (size, data, near)
}

/// Radius (px) of the shore search of `kind_at` (`CoastLook.SHORE_SEARCH_PX`).
pub const SHORE_SEARCH_PX: i64 = 6;

/// Coast kind near `p` (`CoastLook.kind_at`): `<rock>_cliff` or `<beach>_beach`, `None` without a
/// shore within `SHORE_SEARCH_PX`. Nearest first land pixel, then the altitude at
/// `cliff.probe_px` inland. Returns (kind, region id).
pub fn kind_at(
    coast: &CoastDist,
    heights: &HeightGrid,
    data: &CoastData,
    p: Vec2,
) -> Option<(String, String)> {
    let origin = (p.x.floor() as i64, p.y.floor() as i64);
    let mut best: Option<(i64, i64)> = None;
    let mut best_d = i64::MAX;
    for dy in -SHORE_SEARCH_PX..=SHORE_SEARCH_PX {
        for dx in -SHORE_SEARCH_PX..=SHORE_SEARCH_PX {
            let cell = (origin.0 + dx, origin.1 + dy);
            let c = coast.texel(cell.0, cell.1);
            let d = dx * dx + dy * dy;
            if c > 0.0 && c <= 1.01 && d < best_d {
                best = Some(cell);
                best_d = d;
            }
        }
    }
    let (bx, by) = best?;
    let shore = Vec2::new(bx as f32 + 0.5, by as f32 + 0.5);
    Some((
        shore_kind(coast, heights, data, bx, by),
        data.region_at(shore).id.clone(),
    ))
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Reference shores reported in the dump (name, longitude, latitude).
pub const REFERENCE_SHORES: [(&str, f64, f64); 6] = [
    ("etretat", 0.205, 49.708),
    ("pointe_du_raz", -4.735, 48.039),
    ("douvres", 1.335, 51.125),
    ("cabourg", -0.115, 49.292),
    ("baie_mont_saint_michel", -1.53, 48.66),
    ("berck", 1.565, 50.405),
];

/// Radius (px) around a reference point within which its shore pixels are classified (the
/// composed map is deformed: a real longitude / latitude can fall tens of px off the shore).
pub const REFERENCE_RADIUS_PX: i64 = 30;

/// Kind of a shore pixel (`0 < coast <= 1.01`) by the slope rule, with its region.
fn shore_kind(coast: &CoastDist, heights: &HeightGrid, data: &CoastData, x: i64, y: i64) -> String {
    let g = Vec2::new(
        coast.texel(x + 1, y) - coast.texel(x - 1, y),
        coast.texel(x, y + 1) - coast.texel(x, y - 1),
    );
    let shore = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
    let inland = if g.length() < 1e-5 {
        shore
    } else {
        shore + g.normalize() * (data.params.cliff.y - coast.texel(x, y))
    };
    let region = data.region_at(shore);
    let h = heights.height_m_at(inland.x, inland.y);
    if smoothstep(data.params.cliff.z, data.params.cliff.w, h) >= 0.5 {
        format!("{}_cliff", region.rock)
    } else {
        format!("{}_beach", region.beach)
    }
}

/// Count of each coast kind over the shore pixels within `radius` of `p`.
pub fn shore_kinds(
    coast: &CoastDist,
    heights: &HeightGrid,
    data: &CoastData,
    p: Vec2,
    radius: i64,
) -> std::collections::BTreeMap<String, u32> {
    let mut counts = std::collections::BTreeMap::new();
    let (ox, oy) = (p.x.floor() as i64, p.y.floor() as i64);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let (x, y) = (ox + dx, oy + dy);
            let c = coast.texel(x, y);
            if c > 0.0 && c <= 1.01 {
                *counts
                    .entry(shore_kind(coast, heights, data, x, y))
                    .or_insert(0) += 1;
            }
        }
    }
    counts
}

/// Coast kinds around each reference shore (map px from the map projection).
fn reference_kinds(
    map_dir: &Path,
    coast: &CoastDist,
    heights: &HeightGrid,
    data: &CoastData,
) -> Vec<Value> {
    let Ok(projection) = crate::campaign::provinces::projection::MapProjection::load(map_dir)
    else {
        return Vec::new();
    };
    REFERENCE_SHORES
        .iter()
        .filter_map(|(name, lon, lat)| {
            let px = projection.lonlat_to_px(*lon, *lat)?;
            let kinds = shore_kinds(coast, heights, data, px, REFERENCE_RADIUS_PX);
            let main = kinds
                .iter()
                .max_by_key(|(_, n)| **n)
                .map(|(k, _)| k.clone())
                .unwrap_or_default();
            Some(serde_json::json!({
                "name": name, "px": [px.x.round(), px.y.round()],
                "kind": main, "kinds": kinds, "region": data.region_at(px).id,
            }))
        })
        .collect()
}

/// Everything the coast bake produces (off the main thread).
pub struct CoastBake {
    pub params: CoastParams,
    pub dist: Image,
    pub probe: Image,
    pub geology: Image,
    pub probe_texels: u32,
    pub regions: usize,
    pub references: Vec<Value>,
    pub ms: f32,
}

fn image(size: UVec2, format: TextureFormat, data: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    image
}

/// Reads the coast data and bakes the three textures; `None` without data (rendering unchanged).
pub fn bake(map_dir: &Path, heights: &HeightGrid) -> Result<Option<CoastBake>, String> {
    let started = std::time::Instant::now();
    let Ok(text) = std::fs::read_to_string(map_dir.join("coast_types.json")) else {
        return Ok(None);
    };
    let json: Value = serde_json::from_str(&text).map_err(|e| format!("coast_types.json: {e}"))?;
    let Some(data) = CoastData::from_json(&json) else {
        return Ok(None);
    };
    let dist_path = map_dir.join("coast_dist.png");
    if !dist_path.exists() {
        return Ok(None);
    }
    let coast = decode_gray8(&dist_path)?;
    let (probe_size, probe_data, probe_texels) = bake_probe(&coast, heights, data.params.cliff.y);
    let (geo_size, geo_data) = data.geology_texels(coast.width as u32, coast.height as u32);
    let references = reference_kinds(map_dir, &coast, heights, &data);
    let mut params = data.params;
    params.texel = Vec4::new(coast.width as f32, coast.height as f32, 2.0, 0.0);
    let dist_size = UVec2::new(coast.width as u32, coast.height as u32);
    Ok(Some(CoastBake {
        params,
        dist: image(dist_size, TextureFormat::R8Unorm, coast.data),
        probe: image(probe_size, TextureFormat::Rg8Unorm, probe_data),
        geology: image(geo_size, TextureFormat::Rgba8Unorm, geo_data),
        probe_texels,
        regions: data.regions.len(),
        references,
        ms: started.elapsed().as_secs_f32() * 1000.0,
    }))
}

/// Coast textures once baked (persistent between campaign visits).
#[derive(Resource, Clone)]
pub struct CoastTextures {
    pub params: CoastParams,
    pub dist: Handle<Image>,
    pub probe: Handle<Image>,
    pub geology: Handle<Image>,
    pub probe_texels: u32,
    pub regions: usize,
    pub references: Vec<Value>,
    pub ms: f32,
}

type BakeSlot = Arc<Mutex<Option<Result<Option<CoastBake>, String>>>>;

/// Bake running on a worker thread.
#[derive(Resource)]
pub struct PendingCoast(BakeSlot);

/// Bake outcome for the dump: "none" (no data), "pending", "ready" or the error.
#[derive(Resource, Default, Clone)]
pub struct CoastStatus(pub String);

/// Starts the bake once the heightmap is loaded (needs `TerrainWorld::heights`).
pub fn start_bake(
    mut commands: Commands,
    world: Option<Res<super::TerrainWorld>>,
    textures: Option<Res<CoastTextures>>,
    pending: Option<Res<PendingCoast>>,
    status: Option<Res<CoastStatus>>,
) {
    let Some(world) = world else {
        return;
    };
    if textures.is_some() || pending.is_some() || status.is_some_and(|s| !s.0.is_empty()) {
        return;
    }
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    let slot: BakeSlot = Arc::new(Mutex::new(None));
    let out = Arc::clone(&slot);
    let heights = Arc::clone(&world.heights);
    std::thread::spawn(move || {
        let result = bake(&root.join("data/map"), &heights);
        *out.lock().expect("coast slot") = Some(result);
    });
    commands.insert_resource(PendingCoast(slot));
    commands.insert_resource(CoastStatus("pending".into()));
}

/// Collects the bake and uploads its textures.
pub fn poll_bake(
    mut commands: Commands,
    pending: Option<Res<PendingCoast>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(pending) = pending else {
        return;
    };
    let Some(result) = pending.0.lock().expect("coast slot").take() else {
        return;
    };
    commands.remove_resource::<PendingCoast>();
    match result {
        Ok(Some(baked)) => {
            info!(
                "campaign terrain: coast baked in {:.0} ms ({} probe texels, {} regions)",
                baked.ms, baked.probe_texels, baked.regions
            );
            commands.insert_resource(CoastTextures {
                params: baked.params,
                dist: images.add(baked.dist),
                probe: images.add(baked.probe),
                geology: images.add(baked.geology),
                probe_texels: baked.probe_texels,
                regions: baked.regions,
                references: baked.references,
                ms: baked.ms,
            });
            commands.insert_resource(CoastStatus("ready".into()));
        }
        Ok(None) => commands.insert_resource(CoastStatus("none".into())),
        Err(message) => {
            error!("campaign terrain: coast: {message}");
            commands.insert_resource(CoastStatus(message));
        }
    }
}

/// Binds the coast textures to the ground material (again after a material rebuild).
pub fn bind_coast(
    textures: Option<Res<CoastTextures>>,
    handle: Option<Res<super::TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<super::material::TerrainMaterial>>,
) {
    let (Some(textures), Some(handle)) = (textures, handle) else {
        return;
    };
    let bound = materials
        .get(&handle.0)
        .is_none_or(|m| m.coast_dist.as_ref() == Some(&textures.dist));
    if bound {
        return;
    }
    if let Some(mut material) = materials.get_mut(&handle.0) {
        material.extras.coast = textures.params;
        material.coast_dist = Some(textures.dist.clone());
        material.coast_probe = Some(textures.probe.clone());
        material.coast_types = Some(textures.geology.clone());
    }
}

/// Dump section `terrain_coast`.
pub fn dump(world: &World) -> (String, Value) {
    let mut value = serde_json::json!({
        "status": world.get_resource::<CoastStatus>().map(|s| s.0.clone()).unwrap_or_default(),
    });
    if let Some(t) = world.get_resource::<CoastTextures>() {
        let images = world.resource::<Assets<Image>>();
        let size = |h: &Handle<Image>| images.get(h).map(|i| [i.width(), i.height()]);
        value["textures"] = serde_json::json!({
            "dist": size(&t.dist), "probe": size(&t.probe), "geology": size(&t.geology),
            "probe_texels": t.probe_texels, "regions": t.regions, "bake_ms": t.ms,
            "reference_shores": t.references,
        });
    }
    if let Some(handle) = world.get_resource::<super::TerrainMaterialHandle>()
        && let Some(m) = world
            .resource::<Assets<super::material::TerrainMaterial>>()
            .get(&handle.0)
    {
        let e = &m.extras;
        value["material"] = serde_json::json!({
            "coast_enabled": e.coast.cliff.x,
            "coast_bound": m.coast_dist.is_some() && m.coast_probe.is_some() && m.coast_types.is_some(),
            "cliff_m": [e.coast.cliff.z, e.coast.cliff.w],
            "band_a": e.coast.band_a.to_array(),
            "swash_amount": e.coast.swash_a.x,
            "glacier": e.glacier.line.to_array(),
            "post_contrast_saturation": [e.post.x, e.post.y, e.post.z],
            "weather": {
                "any": e.wx.x,
                "selection": e.wx.y,
                "field_bound": m.wx_field.is_some(),
                "clear_bound": m.wx_clear.is_some(),
                "morning": m.look.mist.y,
            },
        });
    }
    ("terrain_coast".into(), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_json() -> Option<Value> {
        let root = crate::paths::find_root().ok()?;
        let text = std::fs::read_to_string(root.join("data/map/coast_types.json")).ok()?;
        serde_json::from_str(&text).ok()
    }

    #[test]
    fn reads_the_real_coast_types() {
        let Some(json) = real_json() else {
            return;
        };
        let data = CoastData::from_json(&json).expect("coast data");
        assert!(data.regions.len() > 3);
        assert_eq!(data.cell_px, 16);
        assert_eq!(data.params.cliff, Vec4::new(1.0, 1.6, 36.0, 52.0));
        assert!(
            (data.params.band_a.x - 1.0).abs() < 1e-6,
            "cliff_px from the data"
        );
        assert!((data.params.swash_a.x - 1.0).abs() < 1e-6);
        // Chalk is light, granite dark.
        assert!(data.params.color[1].x > 0.8 && data.params.color[2].x < 0.3);
        // Dover / Seven Sisters: chalk with shingle.
        let caux = data
            .regions
            .iter()
            .find(|r| r.id == "chalk_caux")
            .expect("Caux");
        assert_eq!(caux.geology()[0], 255);
        assert_eq!(caux.geology()[2], 255);
    }

    #[test]
    fn last_region_wins_and_default_elsewhere() {
        let square = |id: &str, rock: &str, size: f32| Region {
            id: id.into(),
            rock: rock.into(),
            beach: "sand".into(),
            beach_width: 1.0,
            polygon: vec![
                Vec2::ZERO,
                Vec2::new(size, 0.0),
                Vec2::new(size, size),
                Vec2::new(0.0, size),
            ],
        };
        let data = CoastData {
            cell_px: 16,
            default: square("", "rock", 0.0),
            regions: vec![square("a", "chalk", 100.0), square("b", "granite", 50.0)],
            params: CoastParams::default(),
        };
        assert_eq!(data.region_at(Vec2::new(10.0, 10.0)).id, "b");
        assert_eq!(data.region_at(Vec2::new(80.0, 80.0)).id, "a");
        assert_eq!(data.region_at(Vec2::new(500.0, 80.0)).rock, "rock");
        let (size, texels) = data.geology_texels(160, 160);
        assert_eq!(size, UVec2::new(10, 10));
        assert_eq!(texels[1], 255, "first cell is granite");
        assert_eq!(texels[(4 * 10 + 4) * 4], 255, "cell (4, 4) is chalk");
    }

    #[test]
    fn beach_width_is_encoded_like_godot() {
        let r = Region {
            id: String::new(),
            rock: "rock".into(),
            beach: "shingle".into(),
            beach_width: 2.0,
            polygon: vec![],
        };
        assert_eq!(r.geology(), [0, 0, 255, 255]);
    }

    /// A straight coast along x = 50 (land to the east) rising to a cliff of 80 m.
    fn synthetic(cliff_m: f32) -> (CoastDist, HeightGrid) {
        let (w, h) = (100usize, 40usize);
        let mut dist = Vec::with_capacity(w * h);
        let mut heights = Vec::with_capacity(w * h);
        for _ in 0..h {
            for x in 0..w {
                let d = x as f32 + 0.5 - 50.0;
                dist.push((d * 2.0 + 128.0).clamp(0.0, 255.0) as u8);
                let m = if d < 0.0 { -10.0 } else { cliff_m };
                heights.push(((m + 200.0) / 5000.0 * 65535.0) as u16);
            }
        }
        (
            CoastDist {
                width: w,
                height: h,
                data: dist,
            },
            HeightGrid {
                width: w,
                height: h,
                data: heights,
                h_min_m: -200.0,
                h_range_m: 5000.0,
            },
        )
    }

    #[test]
    fn probe_reads_the_altitude_inland_of_the_shore() {
        let (coast, heights) = synthetic(80.0);
        assert!((coast.at(50.0, 20.0)).abs() < 1e-4);
        assert!((coast.to_land(50.0, 20.0) - Vec2::X).length() < 1e-4);
        // From the sea side and from the land side, the same probe (1.6 px inland).
        let (sea_h, sea_lake) = probe_at(&coast, &heights, 1.6, Vec2::new(48.0, 20.0)).unwrap();
        let (land_h, _) = probe_at(&coast, &heights, 1.6, Vec2::new(52.0, 20.0)).unwrap();
        assert!((sea_h - 80.0).abs() < 1.0, "{sea_h}");
        assert!((land_h - 80.0).abs() < 1.0, "{land_h}");
        assert!(!sea_lake, "water below sea level is the sea");
        assert!(probe_at(&coast, &heights, 1.6, Vec2::new(20.0, 20.0)).is_none());
        let (size, data, near) = bake_probe(&coast, &heights, 1.6);
        assert_eq!(size, UVec2::new(50, 20));
        assert!(near > 0);
        assert_eq!(data[((10 * 50 + 25) * 2) as usize], 80);
    }

    #[test]
    fn water_above_sea_level_is_a_lake() {
        let (coast, mut heights) = synthetic(80.0);
        for v in heights.data.iter_mut() {
            if *v < ((0.0 + 200.0) / 5000.0 * 65535.0) as u16 {
                *v = ((150.0 + 200.0) / 5000.0 * 65535.0) as u16;
            }
        }
        let (_, lake) = probe_at(&coast, &heights, 1.6, Vec2::new(50.5, 20.0)).unwrap();
        assert!(lake);
    }

    #[test]
    fn glacier_reads_the_freshwater_data() {
        let json: Value = serde_json::json!({"glacier": {"line_m": 2500.0, "full_m": 3000.0,
            "max_slope": 0.5, "ice_color": [0.1, 0.2, 0.3], "crevasse_color": [0.4, 0.5, 0.6]}});
        let g = GlacierParams::from_json(&json);
        assert_eq!(g.line, Vec4::new(2500.0, 3000.0, 0.5, 1.0));
        assert_eq!(g.ice.truncate(), Vec3::new(0.1, 0.2, 0.3));
        let none = GlacierParams::from_json(&Value::Null);
        assert_eq!(none.line.x, 2600.0);
    }

    #[test]
    fn real_shores_are_chalk_cliffs_in_caux_and_granite_at_the_raz() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let map_dir = root.join("data/map");
        let (Some(json), Ok(coast), Ok(image)) = (
            real_json(),
            decode_gray8(&map_dir.join("coast_dist.png")),
            crate::campaign::terrain::decode::decode_gray16(&map_dir.join("heightmap_render.png")),
        ) else {
            return;
        };
        let heights = HeightGrid {
            width: image.width,
            height: image.height,
            data: image.data,
            h_min_m: -200.0,
            h_range_m: 5000.0,
        };
        let data = CoastData::from_json(&json).unwrap();
        let kinds = reference_kinds(&map_dir, &coast, &heights, &data);
        let kind = |name: &str| {
            kinds
                .iter()
                .find(|k| k["name"] == name)
                .and_then(|k| k["kind"].as_str())
                .unwrap_or("")
                .to_string()
        };
        assert_eq!(kind("etretat"), "chalk_cliff", "{kinds:?}");
        assert!(kind("cabourg").ends_with("_beach"), "{kinds:?}");
        assert!(kind("berck").ends_with("_beach"), "{kinds:?}");
    }
}
