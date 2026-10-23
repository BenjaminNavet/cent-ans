//! Dump section `memory` (ADR 0748): process RSS and the footprint of the image and mesh
//! collections, split into CPU copy (`Image::data` kept in the main world) and estimated GPU size
//! (format x extent x layers x mip chain), with the ten heaviest images. Computed only when a
//! dump is taken.

use bevy::asset::AssetId;
use bevy::mesh::Mesh;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use serde_json::{Value, json};

use super::dump::DumpAppExt;

/// Seconds after start when the allocator is asked to give its idle pages back (experiment,
/// `CENT_ANS_RELIEF=<seconds>`).
fn relief_at() -> Option<f32> {
    std::env::var("CENT_ANS_RELIEF").ok()?.parse().ok()
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
}

fn relieve(time: Res<Time<Real>>, mut done: Local<bool>) {
    let Some(at) = relief_at() else { return };
    if *done || time.elapsed_secs() < at {
        return;
    }
    *done = true;
    let before = footprint_mib();
    #[cfg(target_os = "macos")]
    // SAFETY: documented libmalloc call; a null zone means every zone.
    let freed = unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
    #[cfg(not(target_os = "macos"))]
    let freed = 0usize;
    info!(
        "memory relief: footprint {before:.0} -> {:.0} MiB ({} MiB returned)",
        footprint_mib(),
        freed >> 20
    );
}

/// Images listed in the dump.
const TOP_IMAGES: usize = 10;

pub struct MemoryProbePlugin;

impl Plugin for MemoryProbePlugin {
    fn build(&self, app: &mut App) {
        app.register_dump(dump).add_systems(Update, relieve);
    }
}

/// Resident set size of this process in MiB, read from `ps` (macOS and Linux).
fn rss_mib() -> f64 {
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|text| text.trim().parse::<f64>().ok())
        .map_or(-1.0, |kib| kib / 1024.0)
}

/// Peak resident set size of this process in MiB (`getrusage`: bytes on macOS, KiB elsewhere).
pub fn peak_rss_mib() -> f64 {
    // SAFETY: `getrusage` only writes into the zeroed struct we hand it.
    let max_rss = unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) != 0 {
            return -1.0;
        }
        usage.ru_maxrss as f64
    };
    if cfg!(target_os = "macos") {
        max_rss / (1024.0 * 1024.0)
    } else {
        max_rss / 1024.0
    }
}

/// Physical footprint of this process in MiB (what Activity Monitor and the memory budget of
/// ADR 0748 count: dirty and compressed pages, unified GPU memory included; `-1` off macOS).
/// The RSS misleads on macOS (the compressor moves pages out of it without freeing them).
pub fn footprint_mib() -> f64 {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: `proc_pid_rusage` only writes into the zeroed struct we hand it.
        unsafe {
            let mut info: libc::rusage_info_v4 = std::mem::zeroed();
            let status = libc::proc_pid_rusage(
                std::process::id() as libc::c_int,
                libc::RUSAGE_INFO_V4,
                (&raw mut info).cast::<libc::rusage_info_t>(),
            );
            if status == 0 {
                return info.ri_phys_footprint as f64 / (1024.0 * 1024.0);
            }
        }
    }
    -1.0
}

/// `memory` block of the bench reports (checked by `tools/bevy_bench/perf_guard.sh`).
pub fn report() -> Value {
    json!({"rss_mib": rss_mib(), "peak_rss_mib": peak_rss_mib(), "footprint_mib": footprint_mib()})
}

/// Estimated bytes of the texture once uploaded (full mip chain as declared by the descriptor).
fn gpu_bytes(image: &Image) -> u64 {
    let descriptor = &image.texture_descriptor;
    let format: TextureFormat = descriptor.format;
    let (block_w, block_h) = format.block_dimensions();
    let block_bytes = u64::from(format.block_copy_size(None).unwrap_or(4));
    let samples = u64::from(descriptor.sample_count.max(1));
    let mut total = 0u64;
    for level in 0..descriptor.mip_level_count.max(1) {
        let w = (descriptor.size.width >> level).max(1);
        let h = (descriptor.size.height >> level).max(1);
        let blocks = u64::from(w.div_ceil(block_w)) * u64::from(h.div_ceil(block_h));
        total += blocks * block_bytes * u64::from(descriptor.size.depth_or_array_layers) * samples;
    }
    total
}

fn mib(bytes: u64) -> f64 {
    (bytes as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0
}

fn dump(world: &World) -> (String, Value) {
    let mut cpu_images = 0u64;
    let mut gpu_images = 0u64;
    let mut kept_cpu_count = 0usize;
    let mut rows: Vec<(u64, u64, AssetId<Image>, String)> = Vec::new();
    if let Some(images) = world.get_resource::<Assets<Image>>() {
        let server = world.get_resource::<AssetServer>();
        for (id, image) in images.iter() {
            let cpu = image.data.as_ref().map_or(0, |d| d.len() as u64);
            let gpu = gpu_bytes(image);
            cpu_images += cpu;
            gpu_images += gpu;
            kept_cpu_count += usize::from(cpu > 0);
            let label = server
                .and_then(|s| s.get_path(id))
                .map_or_else(|| "<procedural>".to_owned(), |p| p.to_string());
            let d = &image.texture_descriptor;
            let label = format!(
                "{label} {}x{}x{} {:?} mips={} usage={:?}",
                d.size.width,
                d.size.height,
                d.size.depth_or_array_layers,
                d.format,
                d.mip_level_count,
                image.asset_usage
            );
            rows.push((gpu, cpu, id, label));
        }
    }
    rows.sort_by_key(|row| std::cmp::Reverse(row.0.max(row.1)));
    let mut by_cpu: Vec<&(u64, u64, AssetId<Image>, String)> = rows.iter().collect();
    by_cpu.sort_by_key(|row| std::cmp::Reverse(row.1));
    let top_cpu: Vec<Value> = by_cpu
        .iter()
        .take(TOP_IMAGES)
        .map(|(gpu, cpu, _, label)| json!({"image": label, "gpu_mib": mib(*gpu), "cpu_mib": mib(*cpu)}))
        .collect();
    let top: Vec<Value> = rows
        .iter()
        .take(TOP_IMAGES)
        .map(|(gpu, cpu, _, label)| {
            json!({"image": label, "gpu_mib": mib(*gpu), "cpu_mib": mib(*cpu)})
        })
        .collect();
    let (mut mesh_cpu, mut mesh_count, mut kept_meshes) = (0u64, 0usize, 0usize);
    if let Some(meshes) = world.get_resource::<Assets<Mesh>>() {
        for (_, mesh) in meshes.iter() {
            mesh_count += 1;
            if mesh.try_attributes().is_err() {
                continue; // extracted to the render world: no CPU copy left
            }
            kept_meshes += 1;
            mesh_cpu += mesh.get_vertex_buffer_size() as u64
                + mesh.get_index_buffer_bytes().map_or(0, |b| b.len() as u64);
        }
    }
    (
        "memory".into(),
        json!({
            "rss_mib": rss_mib(),
            "peak_rss_mib": peak_rss_mib(),
            "footprint_mib": footprint_mib(),
            "images": {
                "count": rows.len(),
                "gpu_estimate_mib": mib(gpu_images),
                "cpu_copy_mib": mib(cpu_images),
                "cpu_copy_count": kept_cpu_count,
                "top_gpu": top,
                "top_cpu_copy": top_cpu,
            },
            "meshes": {"count": mesh_count, "cpu_copy_count": kept_meshes, "cpu_mib": mib(mesh_cpu)},
        }),
    )
}
