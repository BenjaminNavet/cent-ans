//! Native thread pool for vegetation tile scattering (lot PB2).
//!
//! godot-rust bindings are single-threaded: the main thread converts the
//! GDScript job fields into plain data, native threads run
//! `vegetation::scatter_tile` (optimised even in dev builds), and the main
//! thread polls finished tiles (same pattern as `ReliefDecoder`). Rendering
//! support only, no game rule.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use godot::classes::RefCounted;
use godot::prelude::*;
use vegetation::{
    reground, scatter_tile, DetailArea, Ground, MapRasters, ReliefFloor, TileRequest, TileResult,
    PAGE_PX,
};

struct Job {
    id: i64,
    request: TileRequest,
    /// Buffers to re-seat (`request_reground`); `None` for a scatter.
    reground: Option<Vec<Vec<f32>>>,
    map: Arc<MapRasters>,
}

struct Done {
    id: i64,
    result: TileResult,
    micros: u64,
}

/// Pool of native threads scattering vegetation tiles.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct VegetationScatter {
    map: Arc<MapRasters>,
    floor: Arc<ReliefFloor>,
    jobs: Option<Sender<Job>>,
    done: Option<Receiver<Done>>,
    workers: Vec<JoinHandle<()>>,
    pending: i64,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for VegetationScatter {
    fn init(base: Base<RefCounted>) -> Self {
        VegetationScatter {
            map: Arc::new(MapRasters::default()),
            floor: Arc::new(ReliefFloor::default()),
            jobs: None,
            done: None,
            workers: Vec::new(),
            pending: 0,
            base,
        }
    }
}

fn float_of(dict: &VarDictionary, key: &str, default: f64) -> f64 {
    dict.get(key)
        .and_then(|v| v.try_to::<f64>().ok())
        .unwrap_or(default)
}

fn int_of(dict: &VarDictionary, key: &str, default: i64) -> i64 {
    dict.get(key)
        .and_then(|v| v.try_to::<i64>().ok())
        .unwrap_or(default)
}

fn ground_of(grid: &VarDictionary) -> Ground {
    if let Some(pages) = grid
        .get("qt_pages")
        .and_then(|v| v.try_to::<VarDictionary>().ok())
    {
        let pages = pages
            .iter_shared()
            .filter_map(|(key, bytes)| {
                let key = key.try_to::<i64>().ok()?;
                let bytes = bytes.try_to::<PackedByteArray>().ok()?;
                (bytes.len() >= PAGE_PX * PAGE_PX * 2).then(|| (key, bytes.to_vec()))
            })
            .collect();
        return Ground::Pages {
            pages,
            max_level: int_of(grid, "max_level", 0),
            h_min: float_of(grid, "h_min", 0.0),
            h_range: float_of(grid, "h_range", 0.0),
        };
    }
    let heights = grid
        .get("heights")
        .and_then(|v| v.try_to::<PackedFloat32Array>().ok());
    let side = int_of(grid, "side", 0).max(0) as usize;
    match heights {
        Some(heights) if side >= 2 && heights.len() >= side * side => Ground::Grid {
            heights: heights.to_vec(),
            side,
            unit: float_of(grid, "unit", 1.0),
        },
        _ => Ground::None,
    }
}

/// Lot SZ4b: optional dense-forest cell of a request (`detail_rect` Rect2, `keep`, `parts_side`).
fn detail_of(params: &VarDictionary) -> Option<DetailArea> {
    let rect = params.get("detail_rect")?.try_to::<Rect2>().ok()?;
    Some(DetailArea {
        rect: (
            rect.position.x as f64,
            rect.position.y as f64,
            (rect.position.x + rect.size.x) as f64,
            (rect.position.y + rect.size.y) as f64,
        ),
        keep: float_of(params, "keep", 1.0),
        parts_side: int_of(params, "parts_side", 4).max(1) as usize,
    })
}

#[godot_api]
impl VegetationScatter {
    /// Shares the map rasters with later requests: heightmap bytes (`bpp` 1 or 2) and the river
    /// bed distance (one byte per pixel, may be empty).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn set_map(
        &mut self,
        height: PackedByteArray,
        width: i64,
        height_px: i64,
        bpp: i64,
        little_endian: bool,
        height_min_m: f64,
        height_max_m: f64,
        river: PackedByteArray,
        river_width: i64,
        river_height: i64,
    ) {
        let width = width.max(0) as usize;
        let height_px = height_px.max(0) as usize;
        let bpp = bpp.clamp(1, 2) as usize;
        let height = height.to_vec();
        let valid = height.len() >= width * height_px * bpp;
        let river = river.to_vec();
        let river_ok = river.len() >= (river_width.max(0) * river_height.max(0)) as usize;
        self.map = Arc::new(MapRasters {
            height: if valid { height } else { Vec::new() },
            width: if valid { width } else { 0 },
            height_px: if valid { height_px } else { 0 },
            bpp,
            little_endian,
            height_min_m,
            height_max_m,
            river: if river_ok { river } else { Vec::new() },
            river_width: river_width.max(0) as usize,
            river_height: river_height.max(0) as usize,
        });
    }

    /// Shares the valley floor of the local relief exaggeration (`MapData.relief_floor_grid`,
    /// lot ZG8) with later requests; an empty grid disables it.
    #[func]
    fn set_floor(&mut self, data: PackedFloat32Array, side_x: i64, side_y: i64, cell: f64) {
        let side = (side_x.max(0) as usize, side_y.max(0) as usize);
        let data = data.to_vec();
        let valid = data.len() == side.0 * side.1;
        self.floor = Arc::new(ReliefFloor {
            data: if valid { data } else { Vec::new() },
            side: if valid { side } else { (0, 0) },
            cell: cell.max(1.0),
        });
    }

    /// Starts `threads` scattering threads (clamped to 1..=16); later calls are ignored.
    #[func]
    fn start(&mut self, threads: i64) {
        if self.jobs.is_some() {
            return;
        }
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done_rx) = mpsc::channel::<Done>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        for _ in 0..threads.clamp(1, 16) {
            let job_rx = Arc::clone(&job_rx);
            let done_tx = done_tx.clone();
            self.workers
                .push(std::thread::spawn(move || worker_loop(&job_rx, &done_tx)));
        }
        self.jobs = Some(job_tx);
        self.done = Some(done_rx);
    }

    /// Queues a tile under the caller's `id`. `params`: tile_index, origin_x, origin_y, size_px,
    /// spacing, coarse_step, tree_scale, vertical_scale, side, coarse (Array of 7
    /// PackedFloat32Array: forest, crops, conifer, beech, hedge, grove, region), exclusions
    /// (PackedVector3Array), ground_grid (`TerrainBuilder.surface_grid`), relief_gain (ZG8).
    /// False if not started or malformed.
    #[func]
    fn request(&mut self, id: i64, params: VarDictionary) -> bool {
        if self.jobs.is_none() {
            return false;
        }
        let side = int_of(&params, "side", 0).max(0) as usize;
        let Some(coarse) = params
            .get("coarse")
            .and_then(|v| v.try_to::<VarArray>().ok())
        else {
            return false;
        };
        let mut grids: [Vec<f32>; 7] = Default::default();
        for (index, grid) in grids.iter_mut().enumerate() {
            let Some(values) = coarse
                .get(index)
                .and_then(|v| v.try_to::<PackedFloat32Array>().ok())
            else {
                return false;
            };
            if values.len() < side * side {
                return false;
            }
            *grid = values.to_vec();
        }
        let exclusions = params
            .get("exclusions")
            .and_then(|v| v.try_to::<PackedVector3Array>().ok())
            .map(|list| {
                list.as_slice()
                    .iter()
                    .map(|e| (e.x as f64, e.y as f64, e.z as f64))
                    .collect()
            })
            .unwrap_or_default();
        let ground = params
            .get("ground_grid")
            .and_then(|v| v.try_to::<VarDictionary>().ok())
            .map(|grid| ground_of(&grid))
            .unwrap_or(Ground::None);
        let request = TileRequest {
            tile_index: int_of(&params, "tile_index", 0),
            origin: (
                float_of(&params, "origin_x", 0.0),
                float_of(&params, "origin_y", 0.0),
            ),
            size_px: float_of(&params, "size_px", 256.0),
            spacing: float_of(&params, "spacing", 1.5).max(0.05),
            coarse_step: float_of(&params, "coarse_step", 4.0).max(1.0),
            tree_scale: float_of(&params, "tree_scale", 1.0),
            vertical_scale: float_of(&params, "vertical_scale", 1.0),
            relief_gain: float_of(&params, "relief_gain", 0.0),
            floor: Arc::clone(&self.floor),
            coarse: grids,
            side,
            exclusions,
            ground,
            detail: detail_of(&params),
        };
        self.send(Job {
            id,
            request,
            reground: None,
            map: Arc::clone(&self.map),
        })
    }

    /// Queues the re-seating of a tile's packed buffers on `ground_grid`
    /// (`TerrainBuilder.surface_grid`, local to `origin`); the result comes back from `poll`
    /// like a scatter (`counts` unchanged).
    #[func]
    fn request_reground(
        &mut self,
        id: i64,
        buffers: VarArray,
        ground_grid: VarDictionary,
        origin: Vector2,
        vertical_scale: f64,
        relief_gain: f64,
    ) -> bool {
        let buffers: Vec<Vec<f32>> = buffers
            .iter_shared()
            .map(|v| {
                v.try_to::<PackedFloat32Array>()
                    .map(|b| b.to_vec())
                    .unwrap_or_default()
            })
            .collect();
        let request = TileRequest {
            tile_index: 0,
            origin: (origin.x as f64, origin.y as f64),
            size_px: 0.0,
            spacing: 1.0,
            coarse_step: 1.0,
            tree_scale: 1.0,
            vertical_scale,
            relief_gain,
            floor: Arc::clone(&self.floor),
            coarse: Default::default(),
            side: 0,
            exclusions: Vec::new(),
            ground: ground_of(&ground_grid),
            detail: None,
        };
        self.send(Job {
            id,
            request,
            reground: Some(buffers),
            map: Arc::clone(&self.map),
        })
    }

    /// Up to `max` finished tiles, each `{id, buffers (Array of PackedFloat32Array), counts
    /// (PackedInt32Array), ms}`.
    #[func]
    fn poll(&mut self, max: i64) -> VarArray {
        let mut out = VarArray::new();
        let Some(done) = &self.done else {
            return out;
        };
        while (out.len() as i64) < max {
            let Ok(item) = done.try_recv() else {
                break;
            };
            self.pending -= 1;
            let mut buffers = VarArray::new();
            for buffer in item.result.buffers {
                buffers.push(&PackedFloat32Array::from(buffer).to_variant());
            }
            let mut dict = VarDictionary::new();
            dict.set("id", &item.id.to_variant());
            dict.set("ms", &(item.micros as f64 / 1000.0).to_variant());
            dict.set("buffers", &buffers.to_variant());
            dict.set(
                "counts",
                &PackedInt32Array::from(item.result.counts).to_variant(),
            );
            out.push(&dict.to_variant());
        }
        out
    }

    /// Requests not yet returned by `poll`.
    #[func]
    fn pending(&self) -> i64 {
        self.pending
    }
}

impl VegetationScatter {
    fn send(&mut self, job: Job) -> bool {
        let Some(jobs) = &self.jobs else {
            return false;
        };
        if jobs.send(job).is_err() {
            return false;
        }
        self.pending += 1;
        true
    }
}

impl Drop for VegetationScatter {
    fn drop(&mut self) {
        self.jobs = None;
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn worker_loop(jobs: &Mutex<Receiver<Job>>, done: &Sender<Done>) {
    loop {
        let job = match jobs.lock() {
            Ok(receiver) => receiver.recv(),
            Err(_) => return,
        };
        let Ok(job) = job else {
            return;
        };
        let start = Instant::now();
        // A panicking tile must still answer, or the GDScript side waits forever.
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match job.reground {
                Some(mut buffers) => {
                    reground(&mut buffers, &job.request, &job.map);
                    let counts = buffers.iter().map(|b| (b.len() / 16) as i32).collect();
                    TileResult { buffers, counts }
                }
                None => scatter_tile(&job.request, &job.map),
            }))
            .unwrap_or_else(|_| TileResult {
                buffers: vec![Vec::new(); 16],
                counts: vec![0; 16],
            });
        let item = Done {
            id: job.id,
            result,
            micros: start.elapsed().as_micros() as u64,
        };
        if done.send(item).is_err() {
            return;
        }
    }
}
