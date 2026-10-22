//! Texture census (lot `texcensus`): two dump sections, read-only measurements.
//!
//! - `textures`: every image of `Assets<Image>`, estimated CPU bytes (`Image.data` still held) and GPU
//!   bytes (format, size, layers, mips), grouped by asset path prefix, the 30 biggest, and the images
//!   whose CPU copy stays in RAM although they are (or can be) on the GPU.
//! - `texture_loads`: decode / mip / atlas constructions timed by [`timed`] / [`Timer`] at their call
//!   sites (any thread; `main` says whether it ran on the main thread, i.e. blocked a frame).
//!
//! Costs: one `Instant` pair per instrumented call, the census itself runs only when a dump is taken.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Instant;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat};
use serde_json::{Value, json};

use super::dump::DumpAppExt;

/// Images listed in the "biggest" table.
const TOP_IMAGES: usize = 30;
/// Loads listed in the dump, longest first.
const TOP_LOADS: usize = 60;

struct LoadRecord {
    name: String,
    ms: f64,
    bytes: usize,
    main: bool,
}

static LOADS: Mutex<Vec<LoadRecord>> = Mutex::new(Vec::new());

fn is_main_thread() -> bool {
    std::thread::current().name() == Some("main")
}

/// Records one finished load / construction.
pub fn record(name: impl Into<String>, started: Instant, bytes: usize) {
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    if let Ok(mut loads) = LOADS.lock() {
        loads.push(LoadRecord {
            name: name.into(),
            ms,
            bytes,
            main: is_main_thread(),
        });
    }
}

/// Runs `work` and records its duration under `name` (`bytes` measured on the result).
pub fn timed<T>(name: impl Into<String>, bytes: impl FnOnce(&T) -> usize, work: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let out = work();
    record(name, started, bytes(&out));
    out
}

/// Start of a span that only counts if it lasts at least `min_ms` (systems that run every frame).
pub struct Timer(Instant);

impl Timer {
    pub fn start() -> Self {
        Self(Instant::now())
    }

    pub fn finish_if(self, min_ms: f64, name: impl Into<String>, bytes: usize) {
        if self.0.elapsed().as_secs_f64() * 1000.0 >= min_ms {
            record(name, self.0, bytes);
        }
    }
}

pub struct TextureCensusPlugin;

impl Plugin for TextureCensusPlugin {
    fn build(&self, app: &mut App) {
        app.register_dump(dump_textures).register_dump(dump_loads);
    }
}

/// Bytes of the whole mip chain (all layers) of a texture descriptor.
pub fn gpu_bytes(image: &Image) -> usize {
    let descriptor = &image.texture_descriptor;
    let format: TextureFormat = descriptor.format;
    let (block_w, block_h) = format.block_dimensions();
    let block_bytes = format.block_copy_size(None).unwrap_or(4) as usize;
    let (w, h, d) = (
        descriptor.size.width,
        descriptor.size.height,
        descriptor.size.depth_or_array_layers,
    );
    let mut total = 0usize;
    for level in 0..descriptor.mip_level_count.max(1) {
        let lw = (w >> level).max(1).div_ceil(block_w) as usize;
        let lh = (h >> level).max(1).div_ceil(block_h) as usize;
        let ld = if descriptor.dimension == TextureDimension::D3 {
            (d >> level).max(1) as usize
        } else {
            d as usize
        };
        total += lw * lh * ld * block_bytes;
    }
    total * descriptor.sample_count.max(1) as usize
}

fn category(path: Option<String>) -> String {
    let Some(path) = path else {
        return "(procedural)".to_owned();
    };
    let path = path.split('#').next().unwrap_or(&path).to_owned();
    let parts: Vec<&str> = path.split('/').collect();
    match parts.as_slice() {
        ["textures", a, b, ..] if parts.len() > 3 => format!("textures/{a}/{b}"),
        ["textures", a, ..] if parts.len() > 2 => format!("textures/{a}"),
        [a, b, ..] if parts.len() > 2 => format!("{a}/{b}"),
        [a, ..] => (*a).to_owned(),
        [] => "(empty)".to_owned(),
    }
}

#[derive(Default)]
struct Totals {
    count: usize,
    gpu: usize,
    cpu: usize,
    /// CPU copy kept although the image has a render-world copy.
    wasted: usize,
    wasted_count: usize,
}

fn mib(bytes: usize) -> f64 {
    (bytes as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0
}

fn dump_textures(world: &World) -> (String, Value) {
    let Some(images) = world.get_resource::<Assets<Image>>() else {
        return ("textures".into(), Value::Null);
    };
    let server = world.get_resource::<AssetServer>();
    let mut groups: BTreeMap<String, Totals> = BTreeMap::new();
    let mut rows: Vec<(usize, Value)> = Vec::new();
    let mut retained: Vec<(usize, Value)> = Vec::new();
    let mut all = Totals::default();
    for (id, image) in images.iter() {
        let path = server.and_then(|s| s.get_path(id)).map(|p| p.to_string());
        let key = category(path.clone());
        let gpu = gpu_bytes(image);
        let cpu = image.data.as_ref().map_or(0, Vec::len);
        let on_gpu = image.asset_usage.contains(RenderAssetUsages::RENDER_WORLD);
        let wasted = if on_gpu { cpu } else { 0 };
        for totals in [groups.entry(key.clone()).or_default(), &mut all] {
            totals.count += 1;
            totals.gpu += gpu;
            totals.cpu += cpu;
            totals.wasted += wasted;
            totals.wasted_count += usize::from(wasted > 0);
        }
        let d = &image.texture_descriptor;
        let row = json!({
            "path": path.unwrap_or_else(|| format!("(procedural {id})")),
            "format": format!("{:?}", d.format),
            "size": [d.size.width, d.size.height, d.size.depth_or_array_layers],
            "mips": d.mip_level_count,
            "gpu_mib": mib(gpu),
            "cpu_mib": mib(cpu),
            "usage": format!("{:?}", image.asset_usage),
            "cpu_retained": cpu > 0,
        });
        if wasted > 0 {
            retained.push((wasted, row.clone()));
        }
        rows.push((gpu.max(cpu), row));
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    retained.sort_by(|a, b| b.0.cmp(&a.0));
    let by_prefix: Vec<Value> = {
        let mut list: Vec<_> = groups.iter().collect();
        list.sort_by(|a, b| b.1.gpu.cmp(&a.1.gpu));
        list.into_iter()
            .map(|(name, t)| {
                json!({
                    "prefix": name, "count": t.count, "gpu_mib": mib(t.gpu), "cpu_mib": mib(t.cpu),
                    "cpu_wasted_mib": mib(t.wasted), "cpu_wasted_count": t.wasted_count,
                })
            })
            .collect()
    };
    (
        "textures".into(),
        json!({
            "count": all.count,
            "gpu_mib": mib(all.gpu),
            "cpu_mib": mib(all.cpu),
            "cpu_wasted_mib": mib(all.wasted),
            "cpu_wasted_count": all.wasted_count,
            "by_prefix": by_prefix,
            "top": rows.into_iter().take(TOP_IMAGES).map(|r| r.1).collect::<Vec<_>>(),
            "top_wasted": retained.into_iter().take(TOP_IMAGES).map(|r| r.1).collect::<Vec<_>>(),
        }),
    )
}

fn dump_loads(_world: &World) -> (String, Value) {
    let Ok(loads) = LOADS.lock() else {
        return ("texture_loads".into(), Value::Null);
    };
    let total_ms: f64 = loads.iter().map(|l| l.ms).sum();
    let main_ms: f64 = loads.iter().filter(|l| l.main).map(|l| l.ms).sum();
    let mut order: Vec<&LoadRecord> = loads.iter().collect();
    order.sort_by(|a, b| b.ms.total_cmp(&a.ms));
    (
        "texture_loads".into(),
        json!({
            "count": loads.len(),
            "total_ms": total_ms.round(),
            "main_thread_ms": main_ms.round(),
            "longest": order.into_iter().take(TOP_LOADS).map(|l| json!({
                "name": l.name, "ms": (l.ms * 10.0).round() / 10.0, "mib": mib(l.bytes), "main": l.main,
            })).collect::<Vec<_>>(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::render_resource::Extent3d;

    #[test]
    fn rgba8_mip_chain_costs_a_third_more() {
        let mut image = Image::new_fill(
            Extent3d { width: 4, height: 4, depth_or_array_layers: 2 },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::all(),
        );
        assert_eq!(gpu_bytes(&image), 4 * 4 * 4 * 2);
        image.texture_descriptor.mip_level_count = 3;
        assert_eq!(gpu_bytes(&image), (16 + 4 + 1) * 4 * 2);
    }

    #[test]
    fn categories_group_by_two_or_three_segments() {
        assert_eq!(category(None), "(procedural)");
        assert_eq!(category(Some("textures/battle/tx/a.jpg".into())), "textures/battle/tx");
        assert_eq!(category(Some("heraldry/banners/x.png".into())), "heraldry/banners");
        assert_eq!(category(Some("a.png".into())), "a.png");
    }
}
