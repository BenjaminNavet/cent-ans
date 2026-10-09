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

use crate::relief_lod_bridge::ReliefLod;
use vegetation::{
    reground, scatter_tile, DetailArea, Distribution, Ground, MapRasters, ReliefFloor,
    SpeciesTable, TileRequest, TileResult, PAGE_PX,
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
    /// Lot HB4: species table (`TreeSpecies.table()`); required to scatter (ADR 0204, no V4 scatter).
    species: Option<Arc<SpeciesTable>>,
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
            species: None,
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
        // Lot PB3g: pages already held by the quadtree's native store are shared, not copied
        // (a snapshot holds up to 256 pages of 512 KB).
        let store = grid
            .get("qt_store")
            .and_then(|v| v.try_to::<Gd<ReliefLod>>().ok());
        let store = store.as_ref().map(|s| s.bind());
        let pages = pages
            .iter_shared()
            .filter_map(|(key, bytes)| {
                let key = key.try_to::<i64>().ok()?;
                if let Some(shared) = store.as_ref().and_then(|s| s.page_bytes(key)) {
                    if shared.len() >= PAGE_PX * PAGE_PX * 2 {
                        return Some((key, shared));
                    }
                }
                let bytes = bytes.try_to::<PackedByteArray>().ok()?;
                (bytes.len() >= PAGE_PX * PAGE_PX * 2).then(|| (key, Arc::new(bytes.to_vec())))
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

fn floats_of(dict: &VarDictionary, key: &str) -> Vec<f32> {
    dict.get(key)
        .and_then(|v| v.try_to::<PackedFloat32Array>().ok())
        .map(|a| a.to_vec())
        .unwrap_or_default()
}

fn ints_of(dict: &VarDictionary, key: &str) -> Vec<i32> {
    dict.get(key)
        .and_then(|v| v.try_to::<PackedInt32Array>().ok())
        .map(|a| a.to_vec())
        .unwrap_or_default()
}

/// Lot HB4: `TreeSpecies.table()` → `SpeciesTable` (`None` when malformed).
fn species_of(table: &VarDictionary) -> Option<SpeciesTable> {
    let dist = table
        .get("distribution")
        .and_then(|v| v.try_to::<VarDictionary>().ok())
        .unwrap_or_default();
    let d = Distribution::default();
    let parsed = SpeciesTable {
        count: int_of(table, "count", 0).max(0) as usize,
        base: floats_of(table, "base"),
        alt_lo: floats_of(table, "alt_lo"),
        alt_hi: floats_of(table, "alt_hi"),
        river: floats_of(table, "river"),
        conifer: floats_of(table, "conifer"),
        kind: ints_of(table, "kind"),
        season: ints_of(table, "season"),
        height: floats_of(table, "height"),
        width: floats_of(table, "width"),
        biome_params: floats_of(table, "biome_params"),
        dist: Distribution {
            massif_core: float_of(&dist, "massif_core", d.massif_core),
            massif_fill: float_of(&dist, "massif_fill", d.massif_fill),
            edge_fill: float_of(&dist, "edge_fill", d.edge_fill),
            stand_px: float_of(&dist, "stand_px", d.stand_px),
            stand_share: float_of(&dist, "stand_share", d.stand_share),
            altitude_fade_m: float_of(&dist, "altitude_fade_m", d.altitude_fade_m),
            river_reach_px: float_of(&dist, "river_reach_px", d.river_reach_px),
            conifer_raster: float_of(&dist, "conifer_raster", d.conifer_raster),
            orchard_parcel_px: float_of(&dist, "orchard_parcel_px", d.orchard_parcel_px),
            orchard_fill: float_of(&dist, "orchard_fill", d.orchard_fill),
            hedge_boost: float_of(&dist, "hedge_boost", d.hedge_boost),
            grove_core: float_of(&dist, "grove_core", d.grove_core),
            hedge_tree: float_of(&dist, "hedge_tree", d.hedge_tree),
            village_boost: float_of(&dist, "village_boost", d.village_boost),
            default_biome: int_of(&dist, "default_biome", d.default_biome as i64).clamp(1, 7)
                as usize,
        },
    };
    parsed.is_valid().then_some(parsed)
}

/// Lot SZ4b: optional dense-forest cell of a request (`detail_rect` Rect2, `keep`, `parts_side`,
/// `corridors` PackedFloat32Array of `x0, y0, x1, y1, half_width` segments kept free of trees).
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
        corridors: params
            .get("corridors")
            .and_then(|v| v.try_to::<PackedFloat32Array>().ok())
            .map(|values| {
                values
                    .as_slice()
                    .as_chunks::<5>()
                    .0
                    .iter()
                    .map(|c| c.map(|v| v as f64))
                    .collect()
            })
            .unwrap_or_default(),
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
            ..Default::default()
        });
    }

    /// Lot SZ1: uncapped base and mountain squash factor of the current floor grid
    /// (`MapData.relief_floor_grid` "base" / "squash"); call after `set_floor`. Arrays of another
    /// size are ignored (base = floor, k = 0).
    #[func]
    fn set_relief_fields(&mut self, base: PackedFloat32Array, squash: PackedFloat32Array) {
        let n = self.floor.data.len();
        let base = base.to_vec();
        let squash = squash.to_vec();
        self.floor = Arc::new(ReliefFloor {
            data: self.floor.data.clone(),
            base: if base.len() == n { base } else { Vec::new() },
            squash: if squash.len() == n {
                squash
            } else {
                Vec::new()
            },
            side: self.floor.side,
            cell: self.floor.cell,
        });
    }

    /// Lot HB4: species table for later requests (`TreeSpecies.table()`); an empty or malformed
    /// dictionary clears it and scatter requests are then refused. True when a table is active.
    #[func]
    fn set_species(&mut self, table: VarDictionary) -> bool {
        self.species = species_of(&table).map(Arc::new);
        self.species.is_some()
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
    /// (PackedVector3Array), ground_grid (`TerrainBuilder.surface_grid`), relief_gain (ZG8),
    /// relief_squash (SZ1), biome (HB4, PackedFloat32Array side × side, optional).
    /// False if not started or malformed.
    #[func]
    fn request(&mut self, id: i64, params: VarDictionary) -> bool {
        if self.jobs.is_none() || self.species.is_none() {
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
            // VT3: real-size trees need a ~0.03-unit pitch in dense forest cells (0.05 before).
            spacing: float_of(&params, "spacing", 1.5).max(0.01),
            coarse_step: float_of(&params, "coarse_step", 4.0).max(1.0),
            tree_scale: float_of(&params, "tree_scale", 1.0),
            vertical_scale: float_of(&params, "vertical_scale", 1.0),
            relief_gain: float_of(&params, "relief_gain", 0.0),
            relief_squash: float_of(&params, "relief_squash", 0.0),
            floor: Arc::clone(&self.floor),
            coarse: grids,
            side,
            exclusions,
            ground,
            detail: detail_of(&params),
            biome: floats_of(&params, "biome"),
            species: self.species.clone(),
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
    #[allow(clippy::too_many_arguments)]
    fn request_reground(
        &mut self,
        id: i64,
        buffers: VarArray,
        ground_grid: VarDictionary,
        origin: Vector2,
        vertical_scale: f64,
        relief_gain: f64,
        relief_squash: f64,
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
            relief_squash,
            floor: Arc::clone(&self.floor),
            coarse: Default::default(),
            side: 0,
            exclusions: Vec::new(),
            ground: ground_of(&ground_grid),
            detail: None,
            biome: Vec::new(),
            species: None,
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
