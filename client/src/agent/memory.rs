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

/// Bytes the Metal device reports as allocated by this process (`MTLDevice.currentAllocatedSize`:
/// textures, buffers, heaps, i.e. everything wgpu created; the share of the footprint the image and
/// mesh counts of the dump cannot see). `None` off macOS.
pub fn metal_allocated_mib() -> Option<f64> {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::{c_char, c_void};
        use std::sync::OnceLock;
        #[link(name = "Metal", kind = "framework")]
        unsafe extern "C" {
            fn MTLCreateSystemDefaultDevice() -> *mut c_void;
        }
        #[link(name = "objc")]
        unsafe extern "C" {
            fn sel_registerName(name: *const c_char) -> *mut c_void;
            fn objc_msgSend();
        }
        static DEVICE: OnceLock<usize> = OnceLock::new();
        let device = *DEVICE.get_or_init(|| {
            // SAFETY: plain Metal call, the device lives for the whole process.
            unsafe { MTLCreateSystemDefaultDevice() as usize }
        }) as *mut c_void;
        if device.is_null() {
            return None;
        }
        // SAFETY: `currentAllocatedSize` is a `NSUInteger` getter of MTLDevice; the transmuted
        // `objc_msgSend` has exactly that signature.
        let bytes = unsafe {
            let selector = sel_registerName(c"currentAllocatedSize".as_ptr());
            let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(device, selector)
        };
        Some(bytes as f64 / (1024.0 * 1024.0))
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Seconds between two `memlog` lines (`CENT_ANS_MEMLOG=<seconds>`, off by default).
fn memlog_every() -> Option<f32> {
    std::env::var("CENT_ANS_MEMLOG").ok()?.parse().ok()
}

/// Periodic one-line memory log: footprint, Metal allocation, image / mesh counts, frame number.
/// Lets a footprint jump be dated against the other log lines (protocol: `docs/performance-memoire.md`).
fn memlog(
    time: Res<Time<Real>>,
    frames: Res<bevy::diagnostic::FrameCount>,
    images: Option<Res<Assets<Image>>>,
    meshes: Option<Res<Assets<Mesh>>>,
    mut next: Local<f32>,
) {
    let Some(every) = memlog_every() else { return };
    if time.elapsed_secs() < *next {
        return;
    }
    *next = time.elapsed_secs() + every;
    let (mut gpu, mut count) = (0u64, 0usize);
    if let Some(images) = images {
        for (_, image) in images.iter() {
            gpu += gpu_bytes(image);
            count += 1;
        }
    }
    info!(
        "memlog t={:.0}s frame={} footprint={:.0} metal={:.0} images={count} gpu_est={:.0} meshes={}",
        time.elapsed_secs(),
        frames.0,
        footprint_mib(),
        metal_allocated_mib().unwrap_or(-1.0),
        gpu as f64 / (1024.0 * 1024.0),
        meshes.map_or(0, |m| m.len())
    );
}

/// Images listed in the dump.
const TOP_IMAGES: usize = 10;

pub struct MemoryProbePlugin;

impl Plugin for MemoryProbePlugin {
    fn build(&self, app: &mut App) {
        app.register_dump(dump)
            .add_systems(Update, (relieve, memlog));
        if memlog_every().is_some()
            && let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp)
        {
            render_app.add_systems(
                bevy::render::Render,
                gpu_census.in_set(bevy::render::RenderSystems::Cleanup),
            );
        }
    }
}

/// Render-world census (`CENT_ANS_MEMLOG`): what the GPU really holds, by texture format and by
/// size class, as opposed to the main-world estimate of the dump (the `Assets<Image>` view misses
/// the images whose asset was dropped or whose bind group keeps an old texture alive).
fn gpu_census(
    gpu_images: Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    mut next: Local<Option<std::time::Instant>>,
) {
    let Some(every) = memlog_every() else { return };
    let now = std::time::Instant::now();
    if next.is_some_and(|at| now < at) {
        return;
    }
    *next = Some(now + std::time::Duration::from_secs_f32(every));
    let mut classes: std::collections::BTreeMap<String, (usize, u64)> = Default::default();
    let mut total = 0u64;
    for (_, gpu) in gpu_images.iter() {
        let descriptor = &gpu.texture_descriptor;
        let bytes = descriptor_bytes(descriptor);
        total += bytes;
        let side = descriptor.size.width.max(descriptor.size.height);
        let class = format!(
            "{:?} {} x{}",
            descriptor.format,
            match side {
                0..=16 => "<=16",
                17..=512 => "<=512",
                513..=1024 => "<=1024",
                1025..=2048 => "<=2048",
                _ => ">2048",
            },
            if descriptor.size.depth_or_array_layers > 1 {
                "layers"
            } else {
                "1"
            }
        );
        let entry = classes.entry(class).or_default();
        entry.0 += 1;
        entry.1 += bytes;
    }
    let mut rows: Vec<_> = classes.into_iter().collect();
    rows.sort_by_key(|(_, (_, bytes))| std::cmp::Reverse(*bytes));
    let top: Vec<String> = rows
        .iter()
        .take(8)
        .map(|(class, (count, bytes))| {
            format!("{class}: {count} / {:.0} MiB", *bytes as f64 / 1048576.0)
        })
        .collect();
    info!(
        "gpucensus images={} total={:.0} MiB | {}",
        gpu_images.iter().count(),
        total as f64 / 1048576.0,
        top.join(" | ")
    );
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
    descriptor_bytes(&image.texture_descriptor)
}

fn descriptor_bytes(descriptor: &bevy::render::render_resource::TextureDescriptor) -> u64 {
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
            "metal_allocated_mib": metal_allocated_mib(),
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
