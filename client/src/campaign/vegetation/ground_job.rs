//! Re-seating of the trees of a vegetation tile on the displayed terrain surface
//! (`vegetation_ground_job.gd`), run off the main thread on Bevy's `AsyncComputeTaskPool`
//! instead of Godot's `WorkerThreadPool`.
//!
//! Input: the tile's trees (shared, never mutated) and a snapshot of the surface (resident relief
//! pages + heightmap, `Surface`) and of the relief exaggeration (`ReliefState`); output: a new tree
//! list installed in one go on the main thread. A tile freed or rescattered meanwhile (its tree
//! list is no longer the one the job started from) ignores the result, like Godot's `generation`.
//! No access to the ECS from the task.

use std::sync::Arc;
use std::time::Instant;

use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future};

use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::Surface;

use super::detail_profile::ForestDetailProfile;
use super::tiles::{GROUND_SINK, TreeInst};
use super::{TileState, VegRender, VegTiles, VegView};

/// Surface snapshot moved into a task.
#[derive(Clone)]
pub struct GroundSnapshot {
    pub surface: Surface,
    pub relief: ReliefState,
}

impl GroundSnapshot {
    /// Displayed ground height at (x, z), same function as `GroundHeight::at`.
    pub fn at(&self, x: f32, z: f32) -> f32 {
        self.surface.display_height_at(&self.relief, x, z)
    }
}

/// `VegetationTileJob.reground`: seats every tree on the snapshot (foot sunk by `GROUND_SINK`).
pub fn reseat(trees: &[TreeInst], ground: &GroundSnapshot) -> Vec<TreeInst> {
    trees
        .iter()
        .map(|tree| {
            let mut seated = *tree;
            seated.y = ground.at(tree.x, tree.z) - GROUND_SINK * tree.height;
            seated
        })
        .collect()
}

/// A running job.
pub struct GroundJob {
    pub tile: i64,
    /// Tree list the job started from (identity of the tile generation).
    source: Arc<Vec<TreeInst>>,
    scale_bits: u32,
    version: u64,
    frame: u64,
    task: Task<(Vec<TreeInst>, f32)>,
}

impl GroundJob {
    /// Starts the job on the async compute pool.
    pub fn spawn(
        tile: i64,
        source: Arc<Vec<TreeInst>>,
        snapshot: GroundSnapshot,
        scale_bits: u32,
        version: u64,
        frame: u64,
    ) -> GroundJob {
        let input = source.clone();
        let task = AsyncComputeTaskPool::get_or_init(Default::default).spawn(async move {
            let started = Instant::now();
            let out = reseat(&input, &snapshot);
            (out, started.elapsed().as_secs_f32() * 1000.0)
        });
        GroundJob {
            tile,
            source,
            scale_bits,
            version,
            frame,
            task,
        }
    }

    /// Result when finished.
    pub fn poll(&mut self) -> Option<(Vec<TreeInst>, f32)> {
        block_on(future::poll_once(&mut self.task))
    }
}

/// Running jobs and counters (dump).
#[derive(Resource, Default)]
pub struct GroundJobs {
    pub running: Vec<GroundJob>,
    pub frame: u64,
    pub launched: usize,
    pub installed: usize,
    pub discarded: usize,
    pub trees_seated: usize,
    pub ms_total: f32,
    pub ms_max: f32,
}

/// Priority of a tile: unseated first, then a changed relief scale, then (nearby, at most every
/// 45 frames) a new surface version (finer pages).
fn priority(
    seated: Option<(u32, u64)>,
    seated_frame: u64,
    frame: u64,
    scale_bits: u32,
    version: u64,
    distance: f32,
) -> Option<u8> {
    match seated {
        None => Some(0),
        Some((bits, _)) if bits != scale_bits => Some(1),
        Some((_, v)) if v != version && distance < 160.0 && frame - seated_frame > 45 => Some(2),
        _ => None,
    }
}

/// Starts jobs for the most urgent tiles (at most `max_ground_jobs` at once).
pub(super) fn launch_ground_jobs(
    surface: Option<Res<Surface>>,
    relief: Option<Res<ReliefState>>,
    view: Res<VegView>,
    tiles: Res<VegTiles>,
    mut jobs: ResMut<GroundJobs>,
    mut cache: Local<Option<(u64, u32, GroundSnapshot)>>,
) {
    jobs.frame += 1;
    let (Some(surface), Some(relief), Some(grid)) = (surface, relief, tiles.grid) else {
        return;
    };
    if !view.active {
        return;
    }
    let limit = ForestDetailProfile::default().max_ground_jobs.max(1) as usize + 1;
    if jobs.running.len() >= limit {
        return;
    }
    let scale_bits = relief.vertical_scale.to_bits();
    let version = surface.version;
    let busy: HashSet<i64> = jobs.running.iter().map(|j| j.tile).collect();
    let mut best: Option<(u8, f32, i64)> = None;
    for (&index, state) in &tiles.tiles {
        let TileState::Ready {
            seated,
            seated_frame,
            ..
        } = state
        else {
            continue;
        };
        if busy.contains(&index) {
            continue;
        }
        let d = grid.distance(index, view.focus.x, view.focus.y);
        let Some(p) = priority(*seated, *seated_frame, jobs.frame, scale_bits, version, d) else {
            continue;
        };
        if best.is_none_or(|(bp, bd, _)| (p, d) < (bp, bd)) {
            best = Some((p, d, index));
        }
    }
    let Some((_, _, index)) = best else { return };
    let Some(TileState::Ready { trees, .. }) = tiles.tiles.get(&index) else {
        return;
    };
    // One snapshot per (surface version, scale): clones of Arcs, shared by the jobs of a frame run.
    let stale = cache
        .as_ref()
        .is_none_or(|(v, b, _)| *v != version || *b != scale_bits);
    if stale {
        *cache = Some((
            version,
            scale_bits,
            GroundSnapshot {
                surface: (*surface).clone(),
                relief: (*relief).clone(),
            },
        ));
    }
    let snapshot = cache.as_ref().expect("snapshot").2.clone();
    let frame = jobs.frame;
    jobs.running.push(GroundJob::spawn(
        index,
        trees.clone(),
        snapshot,
        scale_bits,
        version,
        frame,
    ));
    jobs.launched += 1;
}

/// Installs the finished jobs on their tiles.
pub(super) fn poll_ground_jobs(
    mut jobs: ResMut<GroundJobs>,
    mut tiles: ResMut<VegTiles>,
    mut render: ResMut<VegRender>,
) {
    let mut index = 0;
    while index < jobs.running.len() {
        let Some((seated_trees, ms)) = jobs.running[index].poll() else {
            index += 1;
            continue;
        };
        let job = jobs.running.swap_remove(index);
        jobs.ms_total += ms;
        jobs.ms_max = jobs.ms_max.max(ms);
        match tiles.tiles.get_mut(&job.tile) {
            Some(TileState::Ready {
                trees,
                seated,
                seated_frame,
            }) if Arc::ptr_eq(trees, &job.source) => {
                jobs.trees_seated += seated_trees.len();
                *trees = Arc::new(seated_trees);
                *seated = Some((job.scale_bits, job.version));
                *seated_frame = job.frame;
                render.dirty = true;
                jobs.installed += 1;
            }
            _ => jobs.discarded += 1,
        }
    }
}

/// Forgets the running jobs (tasks are cancelled on drop).
pub fn clear(jobs: &mut GroundJobs) {
    jobs.running.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priorities_order_unseated_then_scale_then_version() {
        assert_eq!(priority(None, 0, 100, 1, 1, 500.0), Some(0));
        assert_eq!(priority(Some((2, 1)), 0, 100, 1, 1, 500.0), Some(1));
        assert_eq!(priority(Some((1, 0)), 10, 100, 1, 1, 50.0), Some(2));
        // Too far, too recent or up to date: nothing to do.
        assert_eq!(priority(Some((1, 0)), 10, 100, 1, 1, 500.0), None);
        assert_eq!(priority(Some((1, 0)), 90, 100, 1, 1, 50.0), None);
        assert_eq!(priority(Some((1, 1)), 10, 100, 1, 1, 50.0), None);
    }
}
