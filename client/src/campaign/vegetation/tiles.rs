//! Tile scatter: a small pool of native threads calling `vegetation::scatter_tile` (the same
//! Rust code Godot used through `VegetationScatter`), and the tile grid.
//!
//! The scatter runs without ground (`Ground::None`, heightmap only); the main thread then seats
//! the trees on the displayed surface (`GroundHeight`), because that surface moves with the
//! camera zoom (exaggerated relief).

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use vegetation::species::CUSTOM_STRIDE;
use vegetation::{
    FLOATS_PER_INSTANCE, Ground, KIND_COUNT, KIND_HEDGE, ReliefFloor, TileRequest, scatter_tile,
};

use super::data::{VegetationWorld, style};

/// Side of a tile (map px): the root tile of the relief pyramid.
pub const TILE_PX: f64 = 256.0;
/// Coarse mask step (px), `VegetationTileJob.coarse_step`.
pub const COARSE_STEP: f64 = 4.0;
/// Sink of the trunk foot (share of the height), `GROUND_SINK`.
pub const GROUND_SINK: f32 = 0.08;

/// A scattered tree (hedges are not drawn in the generalised style).
#[derive(Clone, Copy, Debug)]
pub struct TreeInst {
    pub x: f32,
    pub z: f32,
    /// Displayed ground at the foot minus the sink; 0 until seated.
    pub y: f32,
    pub yaw_deg: f32,
    /// Height in map units.
    pub height: f32,
    /// Catalogue row of the species (impostor atlas row).
    pub row: u16,
    /// Thinning seed in [0, 1]: kept when `seed >= 1 - density`.
    pub seed: f32,
}

pub struct Job {
    pub index: i64,
    pub origin: (f64, f64),
}

pub struct Done {
    pub index: i64,
    pub trees: Vec<TreeInst>,
    pub ms: f32,
    /// Trees removed by the clearings (lakes, sea, main roads).
    pub cleared: usize,
}

/// Grid of tiles over the map.
#[derive(Clone, Copy, Debug)]
pub struct TileGrid {
    pub cols: i64,
    pub rows: i64,
}

impl TileGrid {
    pub fn new(size: (f64, f64)) -> TileGrid {
        TileGrid {
            cols: (size.0 / TILE_PX).ceil() as i64,
            rows: (size.1 / TILE_PX).ceil() as i64,
        }
    }

    pub fn index(&self, col: i64, row: i64) -> i64 {
        row * self.cols + col
    }

    pub fn origin(&self, index: i64) -> (f64, f64) {
        (
            (index % self.cols) as f64 * TILE_PX,
            (index / self.cols) as f64 * TILE_PX,
        )
    }

    /// Distance from (x, z) to the nearest point of the tile.
    pub fn distance(&self, index: i64, x: f32, z: f32) -> f32 {
        let (ox, oz) = self.origin(index);
        let dx = (ox as f32 - x).max(x - (ox + TILE_PX) as f32).max(0.0);
        let dz = (oz as f32 - z).max(z - (oz + TILE_PX) as f32).max(0.0);
        (dx * dx + dz * dz).sqrt()
    }
}

/// Worker threads and their queues.
pub struct ScatterPool {
    jobs: Sender<Job>,
    done: Mutex<Receiver<Done>>,
}

impl ScatterPool {
    pub fn start(world: Arc<VegetationWorld>, threads: usize) -> ScatterPool {
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done_rx) = mpsc::channel::<Done>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        for n in 0..threads.max(1) {
            let world = world.clone();
            let jobs = job_rx.clone();
            let done = done_tx.clone();
            std::thread::Builder::new()
                .name(format!("vegetation-scatter-{n}"))
                .spawn(move || {
                    loop {
                        let job = match jobs.lock() {
                            Ok(rx) => rx.recv(),
                            Err(_) => return,
                        };
                        let Ok(job) = job else { return };
                        let started = std::time::Instant::now();
                        let (trees, cleared) = scatter(&world, job.index, job.origin);
                        let ms = started.elapsed().as_secs_f32() * 1000.0;
                        if done
                            .send(Done {
                                index: job.index,
                                trees,
                                ms,
                                cleared,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                })
                .ok();
        }
        ScatterPool {
            jobs: job_tx,
            done: Mutex::new(done_rx),
        }
    }

    pub fn request(&self, job: Job) {
        let _ = self.jobs.send(job);
    }

    pub fn poll(&self, max: usize) -> Vec<Done> {
        let Ok(rx) = self.done.lock() else {
            return Vec::new();
        };
        rx.try_iter().take(max).collect()
    }
}

/// Scatters one tile (`VegetationTileJob.run` + `VegetationScatter.request`).
pub fn scatter(world: &VegetationWorld, index: i64, origin: (f64, f64)) -> (Vec<TreeInst>, usize) {
    let coarse = world.coarse(origin, TILE_PX, COARSE_STEP);
    let margin = 12.0;
    let exclusions = world
        .exclusions
        .iter()
        .copied()
        .filter(|&(x, y, r)| {
            x + r + margin >= origin.0
                && x - r - margin <= origin.0 + TILE_PX
                && y + r + margin >= origin.1
                && y - r - margin <= origin.1 + TILE_PX
        })
        .collect();
    let request = TileRequest {
        tile_index: index,
        origin,
        size_px: TILE_PX,
        spacing: style::SPACING,
        coarse_step: COARSE_STEP,
        tree_scale: style::scale(),
        vertical_scale: 1.0,
        relief_gain: 0.0,
        relief_squash: 0.0,
        floor: Arc::new(ReliefFloor::default()),
        coarse: coarse.grids,
        side: coarse.side,
        exclusions,
        ground: Ground::None,
        detail: None,
        biome: coarse.biome,
        species: Some(world.species.clone()),
    };
    let result = scatter_tile(&request, &world.map);
    let mut trees = Vec::new();
    for (slot, buffer) in result.buffers.iter().enumerate() {
        if slot % KIND_COUNT == KIND_HEDGE {
            continue;
        }
        for record in buffer.as_chunks::<FLOATS_PER_INSTANCE>().0.iter() {
            let height =
                (record[1] * record[1] + record[5] * record[5] + record[9] * record[9]).sqrt();
            let row = (record[12] / CUSTOM_STRIDE).floor() as i64 - 1;
            trees.push(TreeInst {
                x: record[3],
                z: record[11],
                y: 0.0,
                yaw_deg: (-record[8]).atan2(record[0]).to_degrees(),
                height,
                row: row.clamp(0, world.species.count as i64 - 1) as u16,
                seed: record[15],
            });
        }
    }
    let cleared = world.clearance.filter_tile(&mut trees, origin, TILE_PX);
    (trees, cleared)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_indexes_and_measures_tiles() {
        let grid = TileGrid::new((7080.0, 6104.0));
        assert_eq!((grid.cols, grid.rows), (28, 24));
        let index = grid.index(3, 2);
        assert_eq!(grid.origin(index), (768.0, 512.0));
        assert_eq!(grid.distance(index, 800.0, 600.0), 0.0);
        assert_eq!(grid.distance(index, 760.0, 600.0), 8.0);
    }
}
