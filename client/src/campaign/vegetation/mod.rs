//! Campaign forests, trees and wetlands dressing.
//!
//! Port of game/scripts/map/vegetation.gd (generalised tree style, ADR 0161), the tile job
//! `vegetation_tile_job.gd` and `vegetation_mask.gd`, `tree_species.gd`, `forest_stands.gd`,
//! `dn_tree_models.gd` and `ga3_vegetation.gd`. Owned by a single lot of agent (ADR 0357);
//! other lots do not edit this folder.
//!
//! Flow: the terrain world (heightmap) loads, then `data::load` reads the rasters and tables on
//! a thread; around the camera focus, 256 px tiles are scattered by the `vegetation` crate
//! (pool of native threads, `tiles.rs`); the main thread seats the trees on the displayed
//! surface (`GroundHeight`, re-seated when the relief exaggeration changes) and a worker
//! assembles per-species instance records (`render::assemble`) drawn by the instanced pipeline
//! of the battle lot: models near the focus, crossed-card impostors elsewhere.
//!
//! Not ported: dense forest detail by zoom (`forest_detail.gd`, off in the generalised style),
//! seasonal tint / wind / edge thinning of the foliage shaders, `TreeClearance` (lakes, roads),
//! aligned hedges (off in the generalised style), cloud shadows on trees.

pub mod data;
pub mod mask;
pub mod render;
pub mod tiles;

use std::collections::{HashMap, HashSet};
use std::ops::Not;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::units::instancing::InstancedBatch;
use crate::paths;

use self::data::VegetationWorld;
use self::render::{Assembled, TreeAssets};
use self::tiles::{Done, GROUND_SINK, Job, ScatterPool, TileGrid, TreeInst};
use super::camera::rig::CampaignRig;
use super::terrain::TerrainWorld;
use super::terrain::relief_state::ReliefState;
use super::terrain::surface::{GroundHeight, Surface};

/// No tree beyond this camera distance (`generalised_max_distance`).
const MAX_DISTANCE: f32 = 900.0;
/// Radius around the focus of the seeded tiles (`generalised_view_base` + factor x distance),
/// capped (Godot: 900).
const VIEW_BASE: f32 = 120.0;
const VIEW_FACTOR: f32 = 1.6;
const VIEW_MAX: f32 = 600.0;
/// Tiles kept (`generalised_max_cached_tiles`: 96 in Godot).
const MAX_TILES: usize = 48;
const MAX_IN_FLIGHT: usize = 6;
const POOL_THREADS: usize = 3;

type Shared<T> = Arc<Mutex<Option<T>>>;

/// Loaded vegetation inputs (kept between campaign visits).
#[derive(Resource, Clone)]
pub struct VegWorld(pub Arc<VegetationWorld>);

#[derive(Resource)]
struct PendingWorld(Shared<Result<VegetationWorld, String>>);

#[derive(Resource)]
struct Pool(ScatterPool);

#[derive(Default)]
enum TileState {
    #[default]
    Pending,
    Ready {
        trees: Arc<Vec<TreeInst>>,
        /// `(vertical scale bits, surface version)` of the last seating; `None`: never seated.
        seated: Option<(u32, u64)>,
        seated_frame: u64,
    },
}

#[derive(Resource, Default)]
struct VegTiles {
    tiles: HashMap<i64, TileState>,
    grid: Option<TileGrid>,
    scatter_ms_total: f32,
    scatter_ms_max: f32,
    scattered: usize,
}

/// Camera-derived view of the vegetation.
#[derive(Resource, Default)]
struct VegView {
    focus: Vec2,
    distance: f32,
    active: bool,
    radius: f32,
    density: f32,
    model_radius: f32,
}

struct Running {
    slot: Shared<Assembled>,
}

#[derive(Resource, Default)]
struct VegRender {
    assets: TreeAssets,
    entities: HashMap<(u16, bool), Entity>,
    versions: HashMap<(u16, bool), u64>,
    running: Option<Running>,
    dirty: bool,
    last_focus: Vec2,
    last_density: f32,
    last_model_radius: f32,
    last_ready_models: usize,
    stats: RenderStats,
}

#[derive(Default, Clone)]
struct RenderStats {
    assemblies: usize,
    assemble_ms: f32,
    near: usize,
    far: usize,
    thinned: usize,
    batches: usize,
    batches_model: usize,
}

#[derive(Resource, Default)]
struct VegErrors(Vec<String>);

pub struct CampaignVegetationPlugin;

impl Plugin for CampaignVegetationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VegTiles>()
            .init_resource::<VegView>()
            .init_resource::<VegRender>()
            .init_resource::<VegErrors>()
            .register_dump(dump_vegetation)
            .add_systems(OnEnter(AppState::Campaign), reset_campaign)
            .add_systems(OnExit(AppState::Campaign), reset_campaign)
            .add_systems(
                Update,
                (
                    start_loading,
                    poll_loading,
                    update_view,
                    poll_scatter,
                    seat_trees,
                    drive_assets,
                    assemble_records,
                    apply_records,
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

fn reset_campaign(
    mut commands: Commands,
    mut tiles: ResMut<VegTiles>,
    mut render: ResMut<VegRender>,
    mut view: ResMut<VegView>,
) {
    for (_, entity) in render.entities.drain() {
        commands.entity(entity).despawn();
    }
    render.versions.clear();
    render.running = None;
    render.dirty = true;
    render.last_ready_models = 0;
    tiles.tiles.clear();
    *view = VegView::default();
}

fn start_loading(
    mut commands: Commands,
    terrain: Option<Res<TerrainWorld>>,
    world: Option<Res<VegWorld>>,
    pending: Option<Res<PendingWorld>>,
) {
    let (Some(terrain), None, None) = (terrain, world, pending) else {
        return;
    };
    let root: PathBuf = match paths::find_root() {
        Ok(root) => root,
        Err(message) => {
            error!("campaign vegetation: {message}");
            return;
        }
    };
    let heights = terrain.heights.clone();
    let slot: Shared<Result<VegetationWorld, String>> = Arc::new(Mutex::new(None));
    let out = slot.clone();
    std::thread::spawn(move || {
        let result = data::load(&root, heights);
        *out.lock().expect("vegetation load slot") = Some(result);
    });
    commands.insert_resource(PendingWorld(slot));
}

fn poll_loading(
    mut commands: Commands,
    pending: Option<Res<PendingWorld>>,
    mut errors: ResMut<VegErrors>,
    mut tiles: ResMut<VegTiles>,
) {
    let Some(pending) = pending else { return };
    let Some(result) = pending.0.lock().expect("vegetation load slot").take() else {
        return;
    };
    commands.remove_resource::<PendingWorld>();
    match result {
        Ok(world) => {
            info!(
                "campaign vegetation: {} species, {} stands, {} clearings, loaded in {:.0} ms",
                world.species.count,
                world.stand_count,
                world.exclusions.len(),
                world.load_ms
            );
            tiles.grid = Some(TileGrid::new(world.size));
            let world = Arc::new(world);
            commands.insert_resource(Pool(ScatterPool::start(world.clone(), POOL_THREADS)));
            commands.insert_resource(VegWorld(world));
        }
        Err(message) => {
            error!("campaign vegetation: {message}");
            errors.0.push(message);
        }
    }
}

/// Camera focus and distance → view parameters, and the tiles to scatter.
fn update_view(
    rig: Option<Res<CampaignRig>>,
    pool: Option<Res<Pool>>,
    mut view: ResMut<VegView>,
    mut tiles: ResMut<VegTiles>,
    mut render: ResMut<VegRender>,
) {
    let (Some(rig), Some(pool)) = (rig, pool) else {
        return;
    };
    let Some(grid) = tiles.grid else { return };
    view.focus = Vec2::new(rig.focus.x, rig.focus.z);
    view.distance = rig.distance;
    view.active = rig.distance <= MAX_DISTANCE;
    view.radius = (VIEW_BASE + VIEW_FACTOR * rig.distance).min(VIEW_MAX);
    view.density = render::density_at(rig.distance);
    view.model_radius = render::model_radius(rig.distance);
    if !view.active {
        return;
    }
    let mut wanted: Vec<(f32, i64)> = (0..grid.cols * grid.rows)
        .filter_map(|index| {
            let d = grid.distance(index, view.focus.x, view.focus.y);
            (d <= view.radius).then_some((d, index))
        })
        .collect();
    wanted.sort_by(|a, b| a.0.total_cmp(&b.0));
    wanted.truncate(MAX_TILES - 2);
    let keep: HashSet<i64> = wanted.iter().map(|w| w.1).collect();
    // Evict tiles that are no longer wanted once the cache is full.
    if tiles.tiles.len() > MAX_TILES - 2 {
        let mut stale: Vec<(f32, i64)> = tiles
            .tiles
            .keys()
            .filter(|i| !keep.contains(i))
            .map(|&i| (grid.distance(i, view.focus.x, view.focus.y), i))
            .collect();
        stale.sort_by(|a, b| b.0.total_cmp(&a.0));
        let excess = tiles.tiles.len() - (MAX_TILES - 2);
        for (_, index) in stale.into_iter().take(excess) {
            tiles.tiles.remove(&index);
            render.dirty = true;
        }
    }
    let in_flight = tiles
        .tiles
        .values()
        .filter(|t| matches!(t, TileState::Pending))
        .count();
    let mut budget = MAX_IN_FLIGHT.saturating_sub(in_flight);
    for (_, index) in wanted {
        if budget == 0 {
            break;
        }
        if tiles.tiles.contains_key(&index) {
            continue;
        }
        tiles.tiles.insert(index, TileState::Pending);
        pool.0.request(Job {
            index,
            origin: grid.origin(index),
        });
        budget -= 1;
    }
}

fn poll_scatter(pool: Option<Res<Pool>>, mut tiles: ResMut<VegTiles>) {
    let Some(pool) = pool else { return };
    for Done { index, trees, ms } in pool.0.poll(4) {
        tiles.scattered += 1;
        tiles.scatter_ms_total += ms;
        tiles.scatter_ms_max = tiles.scatter_ms_max.max(ms);
        // A tile evicted while in flight is dropped.
        if let Some(state) = tiles.tiles.get_mut(&index) {
            *state = TileState::Ready {
                trees: Arc::new(trees),
                seated: None,
                seated_frame: 0,
            };
        }
    }
}

/// Seats one tile per frame on the displayed ground: unseated tiles first, then tiles whose
/// seating is stale (relief exaggeration changed; finer pages near the focus).
fn seat_trees(
    ground: GroundHeight,
    relief: Option<Res<ReliefState>>,
    surface: Option<Res<Surface>>,
    view: Res<VegView>,
    mut tiles: ResMut<VegTiles>,
    mut render: ResMut<VegRender>,
    mut frame: Local<u64>,
) {
    *frame += 1;
    let (Some(relief), Some(surface), Some(grid)) = (relief, surface, tiles.grid) else {
        return;
    };
    if !ground.is_ready() || !view.active {
        return;
    }
    let scale_bits = relief.vertical_scale.to_bits();
    let version = surface.version;
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
        let d = grid.distance(index, view.focus.x, view.focus.y);
        let priority = match seated {
            None => 0u8,
            Some((bits, _)) if *bits != scale_bits => 1,
            Some((_, v)) if *v != version && d < 160.0 && *frame - seated_frame > 45 => 2,
            _ => continue,
        };
        if best.is_none_or(|(p, bd, _)| (priority, d) < (p, bd)) {
            best = Some((priority, d, index));
        }
    }
    let Some((_, _, index)) = best else { return };
    let Some(TileState::Ready {
        trees,
        seated,
        seated_frame,
    }) = tiles.tiles.get_mut(&index)
    else {
        return;
    };
    let seated_trees: Vec<TreeInst> = trees
        .iter()
        .map(|t| {
            let mut t = *t;
            t.y = ground.at(t.x, t.z).unwrap_or(0.0) - GROUND_SINK * t.height;
            t
        })
        .collect();
    *trees = Arc::new(seated_trees);
    *seated = Some((scale_bits, version));
    *seated_frame = *frame;
    render.dirty = true;
}

/// Loads the atlas, the models of the species near the focus, and finishes the loaded ones.
fn drive_assets(
    world: Option<Res<VegWorld>>,
    server: Res<AssetServer>,
    mut render: ResMut<VegRender>,
    mut meshes: ResMut<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    materials: Res<Assets<bevy::gltf::GltfMaterial>>,
) {
    let Some(world) = world else { return };
    let render = &mut *render;
    if render.assets.atlas.is_none() {
        render.assets.atlas = Some(server.load(render::ATLAS_ALBEDO));
    }
    if render.assets.atlas_rows == 0
        && let Some(atlas) = render.assets.atlas.as_ref().and_then(|h| images.get(h))
    {
        let cell = (atlas.width() / render::ATLAS_VIEWS).max(1);
        render.assets.atlas_rows = atlas.height() / cell;
        render.dirty = true;
    }
    if render::finish_models(
        &mut render.assets,
        &world.0,
        &server,
        &mut meshes,
        &materials,
    ) {
        render.dirty = true;
    }
}

/// Starts an assembly when something changed and none is running.
fn assemble_records(
    view: Res<VegView>,
    tiles: Res<VegTiles>,
    mut render: ResMut<VegRender>,
    world: Option<Res<VegWorld>>,
) {
    let Some(_world) = world else { return };
    if render.running.is_some() || !view.active {
        return;
    }
    let ready_models = render.assets.ready_models();
    let moved = view.focus.distance(render.last_focus) > (view.model_radius * 0.25).max(2.0);
    let density_changed = (view.density - render.last_density).abs() > 0.05;
    let zone_changed = (view.model_radius > 0.0) != (render.last_model_radius > 0.0)
        || (view.model_radius - render.last_model_radius).abs() > 4.0;
    let models_changed = ready_models != render.last_ready_models;
    if !(render.dirty || moved || density_changed || zone_changed || models_changed) {
        return;
    }
    let trees: Vec<Arc<Vec<TreeInst>>> = tiles
        .tiles
        .values()
        .filter_map(|state| match state {
            TileState::Ready {
                trees,
                seated: Some(_),
                ..
            } => Some(trees.clone()),
            _ => None,
        })
        .collect();
    if trees.is_empty() && render.entities.is_empty() {
        render.dirty = false;
        return;
    }
    // Dev hook: `CENT_ANS_VEG_MODELS=0` draws impostors only.
    let models_on = std::env::var("CENT_ANS_VEG_MODELS")
        .is_ok_and(|v| v == "0")
        .not();
    let model_rows: Vec<bool> = (0.._world.0.species.count)
        .map(|row| {
            models_on
                && render
                    .assets
                    .models
                    .get(&(row as u16))
                    .is_some_and(|m| m.ready.is_some())
        })
        .collect();
    let (focus, density, model_radius) = (view.focus, view.density, view.model_radius);
    let slot: Shared<Assembled> = Arc::new(Mutex::new(None));
    let out = slot.clone();
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let mut assembled = render::assemble(&trees, focus, model_radius, density, &model_rows);
        assembled.ms = started.elapsed().as_secs_f32() * 1000.0;
        *out.lock().expect("assembly slot") = Some(assembled);
    });
    render.last_focus = focus;
    render.last_density = density;
    render.last_model_radius = model_radius;
    render.last_ready_models = ready_models;
    render.dirty = false;
    render.running = Some(Running { slot });
}

/// Hands the assembled records to the instanced batches.
fn apply_records(
    mut commands: Commands,
    view: Res<VegView>,
    world: Option<Res<VegWorld>>,
    server: Res<AssetServer>,
    mut render: ResMut<VegRender>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut batches: Query<&mut InstancedBatch>,
) {
    let Some(world) = world else { return };
    let render = &mut *render;
    if !view.active && !render.entities.is_empty() {
        for (_, entity) in render.entities.drain() {
            commands.entity(entity).despawn();
        }
        render.stats.batches = 0;
        render.stats.batches_model = 0;
        render.dirty = true;
        return;
    }
    let Some(running) = render.running.as_ref() else {
        return;
    };
    let Some(assembled) = running.slot.lock().expect("assembly slot").take() else {
        return;
    };
    render.running = None;
    // Models to load for the species of the zone.
    for row in &assembled.wanted_rows {
        render::request_model(&mut render.assets, &world.0, &server, *row);
    }
    let rows = render.assets.atlas_rows;
    let Some(atlas) = render.assets.atlas.clone().filter(|_| rows > 0) else {
        render.dirty = true;
        return;
    };
    let mut seen: HashSet<(u16, bool)> = HashSet::new();
    for (key, bytes) in assembled.batches {
        let (row, model) = key;
        let count = (bytes.len() / render::RECORD_BYTES) as u32;
        let (mesh, texture) = if model {
            match render.assets.models.get(&row).and_then(|m| m.ready.clone()) {
                Some(ready) => ready,
                None => continue,
            }
        } else {
            let ratio = world
                .0
                .width_ratio
                .get(row as usize)
                .copied()
                .unwrap_or(1.0);
            let mesh = render
                .assets
                .cards
                .entry(row)
                .or_insert_with(|| meshes.add(render::card_mesh(row, rows, ratio)))
                .clone();
            (mesh, atlas.clone())
        };
        seen.insert(key);
        let version = render.versions.entry(key).or_insert(0);
        *version += 1;
        let records = Arc::new(bytes);
        match render
            .entities
            .get(&key)
            .and_then(|e| batches.get_mut(*e).ok())
        {
            Some(mut batch) => {
                batch.records = records;
                batch.count = count;
                batch.version = *version;
            }
            None => {
                let entity = commands
                    .spawn((
                        Name::new(format!(
                            "Trees{}{}",
                            world
                                .0
                                .species_ids
                                .get(row as usize)
                                .map_or("", String::as_str),
                            if model { "Model" } else { "Impostor" }
                        )),
                        InstancedBatch {
                            mesh_handle: mesh,
                            records,
                            count,
                            texture_handle: Some(texture),
                            params: render::params(model),
                            version: *version,
                        },
                        DespawnOnExit(AppState::Campaign),
                    ))
                    .id();
                render.entities.insert(key, entity);
            }
        }
    }
    let gone: Vec<(u16, bool)> = render
        .entities
        .keys()
        .filter(|k| !seen.contains(k))
        .copied()
        .collect();
    for key in gone {
        if let Some(entity) = render.entities.remove(&key) {
            commands.entity(entity).despawn();
        }
    }
    render.stats.assemblies += 1;
    render.stats.near = assembled.near;
    render.stats.far = assembled.far;
    render.stats.thinned = assembled.thinned;
    render.stats.assemble_ms = assembled.ms;
    render.stats.batches = render.entities.len();
    render.stats.batches_model = render.entities.keys().filter(|k| k.1).count();
}

fn dump_vegetation(world: &World) -> (String, Value) {
    let mut value = json!({ "ready": false });
    if let Some(errors) = world.get_resource::<VegErrors>()
        && !errors.0.is_empty()
    {
        value["errors"] = json!(errors.0);
    }
    value["loading"] = json!(world.contains_resource::<PendingWorld>());
    let Some(veg) = world.get_resource::<VegWorld>() else {
        return ("vegetation".into(), value);
    };
    let w = &veg.0;
    value["ready"] = json!(true);
    value["world"] = json!({
        "species": w.species.count,
        "stands": w.stand_count,
        "clearings": w.exclusions.len(),
        "load_ms": w.load_ms,
        "map_px": [w.size.0, w.size.1],
    });
    let mut kinds: std::collections::BTreeMap<&str, usize> = Default::default();
    for site in &w.wet_sites {
        *kinds.entry(site.kind.as_str()).or_default() += 1;
    }
    value["wetlands"] = json!({
        "sites": w.wet_sites.len(),
        "by_kind": kinds,
        "raster": !w.wet.is_empty(),
        "cells_cleared": mask::WET_CLEARED.load(Ordering::Relaxed),
    });
    if let Some(tiles) = world.get_resource::<VegTiles>() {
        let (mut pending, mut ready, mut seated, mut trees) = (0, 0, 0, 0usize);
        for state in tiles.tiles.values() {
            match state {
                TileState::Pending => pending += 1,
                TileState::Ready {
                    trees: t,
                    seated: s,
                    ..
                } => {
                    ready += 1;
                    trees += t.len();
                    seated += usize::from(s.is_some());
                }
            }
        }
        value["tiles"] = json!({
            "pending": pending, "ready": ready, "seated": seated, "trees": trees,
            "scattered": tiles.scattered,
            "scatter_ms_avg": if tiles.scattered > 0 { tiles.scatter_ms_total / tiles.scattered as f32 } else { 0.0 },
            "scatter_ms_max": tiles.scatter_ms_max,
        });
    }
    if let Some(view) = world.get_resource::<VegView>() {
        value["view"] = json!({
            "active": view.active, "focus": [view.focus.x, view.focus.y],
            "distance": view.distance, "radius": view.radius,
            "density": view.density, "model_radius": view.model_radius,
        });
    }
    if let Some(render) = world.get_resource::<VegRender>() {
        let models_failed = render.assets.models.values().filter(|m| m.failed).count();
        value["render"] = json!({
            "impostor_instances": render.stats.far,
            "model_instances": render.stats.near,
            "thinned_out": render.stats.thinned,
            "batches": render.stats.batches,
            "model_batches": render.stats.batches_model,
            "assemblies": render.stats.assemblies,
            "assemble_ms": render.stats.assemble_ms,
            "models_requested": render.assets.models.len(),
            "models_ready": render.assets.ready_models(),
            "models_failed": models_failed,
            "cards": render.assets.cards.len(),
            "atlas_rows": render.assets.atlas_rows,
        });
    }
    ("vegetation".into(), value)
}
