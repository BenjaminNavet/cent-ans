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
//! Impostors use their own instanced pipeline (`billboard.rs`, `billboard.wesl`): azimuth view,
//! seasonal tint, wind, atlas normals. `clearance.rs` ports `TreeClearance` (lakes, sea, roads);
//! `reeds.rs` the reed clumps of the wetlands (their ponds are drawn by `water/fresh.rs`).
//!
//! `edges.rs`: forest-cover edges (lower, sparser trees); `mips.rs`: mip chains of the atlases;
//! rivers clear the trees (`clearance.rs`, crown radius exact incl. `crown_widen`); the reeds have
//! a near level and dithered level fades.
//!
//! Not ported: dense forest detail by zoom (`forest_detail.gd`, off in the generalised style),
//! aligned hedges (off in the generalised style), cloud shadows on trees.
//!
//! Tree shadows: real cast shadows in the sun's shadow maps (`shadow_pass.rs`, toggled by
//! `shadows.rs` with the camera distance).

pub mod billboard;
pub mod clearance;
pub mod data;
pub mod detail_profile;
pub mod edges;
pub mod ground_job;
pub mod ground_materials;
pub mod light;
pub mod look;
pub mod mask;
pub mod meshes;
pub mod mips;
pub mod reeds;
pub mod render;
pub mod shadow_pass;
pub mod shadows;
pub mod textures;
pub mod tiles;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::terrain::season::SeasonWeights;

use self::billboard::{BillboardPlugin, Mode, VegBatch};
use crate::paths;

use self::data::VegetationWorld;
use self::render::{Assembled, TreeAssets};
use self::tiles::{Done, Job, ScatterPool, TileGrid, TreeInst};
use super::camera::rig::CampaignRig;
use super::terrain::TerrainWorld;
use super::terrain::relief_state::ReliefState;
use super::terrain::surface::GroundHeight;

/// No tree beyond this camera distance (`generalised_max_distance`).
const MAX_DISTANCE: f32 = 900.0;
/// Radius around the focus of the seeded tiles (`generalised_view_base` + factor x distance),
/// capped (Godot: 900; measured 52 tiles at 14 ms each, 167 k impostors, assembly 11 ms off thread).
const VIEW_BASE: f32 = 120.0;
const VIEW_FACTOR: f32 = 1.6;
const VIEW_MAX: f32 = 900.0;
/// Tiles kept (`generalised_max_cached_tiles`: 96 in Godot).
const MAX_TILES: usize = 96;
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
    cleared: usize,
    edge_thinned: usize,
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

/// A seeded block of the wetland decor.
struct WetBlock {
    seeded: reeds::Seeded,
    /// Vertical scale bits of the last seating; `None`: not seated.
    seated: Option<u32>,
    reed_bytes: Arc<Vec<u8>>,
}

/// Blocks and batches of one level of the wetland decor.
#[derive(Default)]
struct WetLevel {
    blocks: HashMap<(i32, i32), WetBlock>,
    /// Reed batch (the ponds are drawn by `water/fresh.rs`).
    entities: [Option<Entity>; 1],
    versions: [u64; 1],
    last_focus: Vec2,
    last_radius: f32,
    dirty: bool,
    /// Opacity pushed to the batches (dithered), `fw_alpha`.
    alpha: f32,
    reeds: usize,
    rebuilds: usize,
}

/// Reed clumps around the focus (`reeds.rs`), near and patch levels.
#[derive(Resource, Default)]
struct WetDecor {
    levels: [WetLevel; 2],
    meshes: Option<Handle<Mesh>>,
    card: Option<Handle<Image>>,
    seeded_blocks: usize,
    seed_ms_total: f32,
    seed_ms_max: f32,
}

pub struct CampaignVegetationPlugin;

impl Plugin for CampaignVegetationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BillboardPlugin, shadow_pass::VegShadowPassPlugin))
            .init_resource::<VegTiles>()
            .init_resource::<VegView>()
            .init_resource::<VegRender>()
            .init_resource::<ground_job::GroundJobs>()
            .init_resource::<VegErrors>()
            .init_resource::<WetDecor>()
            .init_resource::<mips::AtlasMips>()
            .init_resource::<shadows::TreeShadows>()
            .init_resource::<light::VegLight>()
            .register_dump(dump_vegetation)
            .register_dump(dump_ported)
            .add_systems(
                OnEnter(AppState::Campaign),
                (reset_campaign, shadows::reset),
            )
            .add_systems(OnExit(AppState::Campaign), (reset_campaign, shadows::reset))
            .add_systems(
                Update,
                (
                    start_loading,
                    poll_loading,
                    update_view,
                    poll_scatter,
                    ground_job::launch_ground_jobs,
                    ground_job::poll_ground_jobs,
                    drive_assets,
                    mips::build_atlas_mips,
                    assemble_records,
                    apply_records,
                    shadows::sync_shadows,
                    update_wetlands,
                    sync_season,
                    light::sync_light,
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
    mut wet: ResMut<WetDecor>,
    mut ground_jobs: ResMut<ground_job::GroundJobs>,
) {
    ground_job::clear(&mut ground_jobs);
    for level in &mut wet.levels {
        for entity in level.entities.iter_mut().filter_map(Option::take) {
            commands.entity(entity).despawn();
        }
    }
    *wet = WetDecor::default();
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
    for Done {
        index,
        trees,
        ms,
        cleared,
        edge_thinned,
    } in pool.0.poll(4)
    {
        tiles.scattered += 1;
        tiles.cleared += cleared;
        tiles.edge_thinned += edge_thinned;
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
    if render.assets.normal.is_none() {
        // Normal data: linear, not sRGB.
        render.assets.normal = Some(
            server
                .load_builder()
                .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| {
                    settings.is_srgb = false
                })
                .load(render::ATLAS_NORMAL),
        );
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
    // Near lod1 models with the Godot `foliage_model` treatment (vegvis: chroma, gains, season,
    // terrain lighting). Dev hook: `CENT_ANS_VEG_MODELS=0` draws impostors only.
    let models_on = std::env::var("CENT_ANS_VEG_MODELS").map_or(true, |v| v != "0");
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
#[allow(clippy::too_many_arguments)]
fn apply_records(
    mut commands: Commands,
    view: Res<VegView>,
    world: Option<Res<VegWorld>>,
    server: Res<AssetServer>,
    mut render: ResMut<VegRender>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut veg_batches: Query<&mut VegBatch>,
    season: Option<Res<SeasonWeights>>,
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
    let season_weights = season.map_or(Vec4::new(1.0, 0.0, 0.0, 0.0), |s| s.weights);
    for (key, bytes) in assembled.batches {
        let (row, model) = key;
        let species_id = world
            .0
            .species_ids
            .get(row as usize)
            .map_or("", String::as_str);
        let version = render.versions.entry(key).or_insert(0);
        *version += 1;
        let version = *version;
        let records = Arc::new(bytes);
        let class = world
            .0
            .species
            .season
            .get(row as usize)
            .copied()
            .unwrap_or(0);
        if model {
            let count = (records.len() / billboard::RECORD_BYTES) as u32;
            let shadow_mesh = render
                .assets
                .models
                .get(&row)
                .and_then(|m| m.shadow_ready.clone());
            let Some((mesh, texture)) =
                render.assets.models.get(&row).and_then(|m| m.ready.clone())
            else {
                continue;
            };
            seen.insert(key);
            match render
                .entities
                .get(&key)
                .and_then(|e| veg_batches.get_mut(*e).ok())
            {
                Some(mut batch) => {
                    batch.records = records;
                    batch.count = count;
                    batch.shadow_mesh = shadow_mesh;
                    batch.version = version;
                }
                None => {
                    let gains = render
                        .assets
                        .gains
                        .get_or_insert_with(render::ModelGains::load);
                    let mut params = render::model_params(gains, species_id, class);
                    params.season = season_weights;
                    let entity = commands
                        .spawn((
                            Name::new(format!("Trees{species_id}Model")),
                            VegBatch {
                                mesh_handle: mesh,
                                records,
                                count,
                                shadow_mesh,
                                albedo: texture,
                                normal: None,
                                params,
                                version,
                                params_version: 0,
                            },
                            DespawnOnExit(AppState::Campaign),
                        ))
                        .id();
                    render.entities.insert(key, entity);
                }
            }
            continue;
        }
        let count = (records.len() / billboard::RECORD_BYTES) as u32;
        seen.insert(key);
        match render
            .entities
            .get(&key)
            .and_then(|e| veg_batches.get_mut(*e).ok())
        {
            Some(mut batch) => {
                batch.records = records;
                batch.count = count;
                batch.version = version;
            }
            None => {
                let ratio = world
                    .0
                    .width_ratio
                    .get(row as usize)
                    .copied()
                    .unwrap_or(1.0);
                let mesh = render
                    .assets
                    .quad
                    .get_or_insert_with(|| meshes.add(render::quad_mesh()))
                    .clone();
                let gain = render
                    .assets
                    .gains
                    .get_or_insert_with(render::ModelGains::load)
                    .impostor_gain;
                let mut params = render::impostor_params(row, rows, ratio, class, gain);
                params.season = season_weights;
                let entity = commands
                    .spawn((
                        Name::new(format!("Trees{species_id}Impostor")),
                        VegBatch {
                            mesh_handle: mesh,
                            records,
                            count,
                            shadow_mesh: None,
                            albedo: atlas.clone(),
                            normal: render.assets.normal.clone(),
                            params,
                            version,
                            params_version: 0,
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

/// Pushes the smoothed season weights into the parameter block of every vegetation batch.
fn sync_season(season: Option<Res<SeasonWeights>>, mut batches: Query<&mut VegBatch>) {
    let Some(season) = season else { return };
    for mut batch in &mut batches {
        if batch.params.season != season.weights {
            batch.params.season = season.weights;
            batch.params_version += 1;
        }
    }
}

/// Seeds, seats and draws the reed clumps near the focus (both levels of
/// `FreshwaterLayer`: near under a few units, patch up to `patch_max_distance`).
#[allow(clippy::too_many_arguments)]
fn update_wetlands(
    mut commands: Commands,
    world: Option<Res<VegWorld>>,
    view: Res<VegView>,
    ground: GroundHeight,
    relief: Option<Res<ReliefState>>,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut wet: ResMut<WetDecor>,
    mut batches: Query<&mut VegBatch>,
) {
    let (Some(world), Some(relief)) = (world, relief) else {
        return;
    };
    let config = &world.0.wetlands;
    if config.sites.is_empty() || !ground.is_ready() {
        return;
    }
    let wet = &mut *wet;
    let scale_bits = relief.vertical_scale.to_bits();
    let height = |x: f32, z: f32| ground.at(x, z).unwrap_or(0.0);
    let mut seed_budget = 3usize;
    for level in [reeds::Level::Near, reeds::Level::Patch] {
        let alpha = if view.active && view.distance < config.patch_max_distance {
            config.alpha_of(level, view.distance)
        } else {
            0.0
        };
        let slot = &mut wet.levels[level as usize];
        if alpha <= 0.01 {
            // Hidden level: the batches empty out once, the blocks stay cached.
            if slot.alpha > 0.01 || slot.reeds > 0 {
                for entity in slot.entities.iter().flatten() {
                    if let Ok(mut batch) = batches.get_mut(*entity) {
                        batch.count = 0;
                        batch.version += 1;
                    }
                }
                slot.reeds = 0;
                slot.dirty = true;
            }
            slot.alpha = 0.0;
            continue;
        }
        let size = config.block_of(level);
        let radius = config.radius_of(level, view.distance);
        let focus = view.focus;
        let (low, high) = (
            ((focus - Vec2::splat(radius)) / size).floor().as_ivec2(),
            ((focus + Vec2::splat(radius)) / size).floor().as_ivec2(),
        );
        // Distance of the focus to the nearest point of a block.
        let block_distance = |key: (i32, i32)| {
            let origin = Vec2::new(key.0 as f32, key.1 as f32) * size;
            focus.distance(focus.clamp(origin, origin + Vec2::splat(size)))
        };
        let mut wanted: Vec<((i32, i32), f32)> = Vec::new();
        for bx in low.x..=high.x {
            for bz in low.y..=high.y {
                let d = block_distance((bx, bz));
                if d <= radius {
                    wanted.push(((bx, bz), d));
                }
            }
        }
        wanted.sort_by(|a, b| a.1.total_cmp(&b.1));
        for &(key, _) in &wanted {
            if slot.blocks.contains_key(&key) || seed_budget == 0 {
                continue;
            }
            seed_budget -= 1;
            let started = std::time::Instant::now();
            let seeded = reeds::seed_block(
                config,
                &world.0.land,
                world.0.size,
                &world.0.exclusions,
                level,
                key,
            );
            let mut reed_bytes = Vec::new();
            reeds::block_records(&seeded, &mut reed_bytes, height);
            let ms = started.elapsed().as_secs_f32() * 1000.0;
            wet.seeded_blocks += 1;
            wet.seed_ms_total += ms;
            wet.seed_ms_max = wet.seed_ms_max.max(ms);
            slot.blocks.insert(
                key,
                WetBlock {
                    seeded,
                    seated: Some(scale_bits),
                    reed_bytes: Arc::new(reed_bytes),
                },
            );
            slot.dirty = true;
        }
        // Reseat the blocks whose relief exaggeration changed (two per frame).
        let mut reseated = 0;
        for &(key, _) in &wanted {
            if reseated >= 2 {
                break;
            }
            let Some(block) = slot.blocks.get_mut(&key) else {
                continue;
            };
            if block.seated == Some(scale_bits) {
                continue;
            }
            let mut reed_bytes = Vec::new();
            reeds::block_records(&block.seeded, &mut reed_bytes, height);
            block.reed_bytes = Arc::new(reed_bytes);
            block.seated = Some(scale_bits);
            reseated += 1;
            slot.dirty = true;
        }
        // Evict the far blocks.
        let cache = match level {
            reeds::Level::Near => 400,
            reeds::Level::Patch => 220,
        };
        if slot.blocks.len() > cache {
            let mut far: Vec<((i32, i32), f32)> = slot
                .blocks
                .keys()
                .map(|&k| (k, block_distance(k)))
                .filter(|(_, d)| *d > radius)
                .collect();
            far.sort_by(|a, b| b.1.total_cmp(&a.1));
            let excess = slot.blocks.len() - cache;
            for (key, _) in far.into_iter().take(excess) {
                slot.blocks.remove(&key);
            }
        }
        // Opacity of the level (dithered in the shader).
        if (slot.alpha - alpha).abs() > 0.01 {
            slot.alpha = alpha;
            for entity in slot.entities.iter().flatten() {
                if let Ok(mut batch) = batches.get_mut(*entity) {
                    batch.params.range.w = alpha;
                    batch.params_version += 1;
                }
            }
        }
        let moved = focus.distance(slot.last_focus) > size * 0.5
            || (radius - slot.last_radius).abs() > size * 0.5;
        if !(slot.dirty || moved) {
            continue;
        }
        // Records of the wanted blocks, nearest first, up to `max_instances`.
        let mut reed_all = Vec::new();
        let mut total = 0usize;
        for (key, _) in &wanted {
            let Some(block) = slot.blocks.get(key) else {
                continue;
            };
            let count = block.reed_bytes.len() / billboard::RECORD_BYTES;
            if total + count > config.max_instances {
                break;
            }
            total += count;
            reed_all.extend_from_slice(&block.reed_bytes);
        }
        slot.last_focus = focus;
        slot.last_radius = radius;
        slot.dirty = false;
        slot.rebuilds += 1;
        slot.reeds = reed_all.len() / billboard::RECORD_BYTES;
        let reed_mesh = wet
            .meshes
            .get_or_insert_with(|| meshes.add(reeds::reed_mesh()))
            .clone();
        let card = wet
            .card
            .get_or_insert_with(|| server.load(reeds::CARD_TEXTURE))
            .clone();
        for (n, (bytes, mesh, mode, name)) in [(reed_all, reed_mesh, Mode::Reeds, "WetlandReeds")]
            .into_iter()
            .enumerate()
        {
            slot.versions[n] += 1;
            let count = (bytes.len() / billboard::RECORD_BYTES) as u32;
            let records = Arc::new(bytes);
            match slot.entities[n].and_then(|e| batches.get_mut(e).ok()) {
                Some(mut batch) => {
                    batch.records = records;
                    batch.count = count;
                    batch.version = slot.versions[n];
                }
                None => {
                    let mut params = reeds::params(config, mode, level);
                    params.range.w = alpha;
                    let entity = commands
                        .spawn((
                            Name::new(format!("{name}{level:?}")),
                            VegBatch {
                                mesh_handle: mesh,
                                records,
                                count,
                                shadow_mesh: None,
                                albedo: card.clone(),
                                normal: None,
                                params,
                                version: slot.versions[n],
                                params_version: 0,
                            },
                            DespawnOnExit(AppState::Campaign),
                        ))
                        .id();
                    slot.entities[n] = Some(entity);
                }
            }
        }
    }
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
        "lakes": w.clearance.lakes.len(),
        "main_road_segments": w.clearance.road_segments.len(),
        "coast_raster": !w.clearance.coast.is_empty(),
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
    if let Some(wet) = world.get_resource::<WetDecor>() {
        let level = |l: &WetLevel| {
            json!({
                "blocks_cached": l.blocks.len(),
                "reed_clumps": l.reeds,
                "alpha": l.alpha,
                "rebuilds": l.rebuilds,
                "batches": l.entities.iter().flatten().count(),
            })
        };
        value["wetland_decor"] = json!({
            "sites": w.wetlands.sites.len(),
            "blocks_seeded": wet.seeded_blocks,
            "seed_ms_avg": if wet.seeded_blocks > 0 { wet.seed_ms_total / wet.seeded_blocks as f32 } else { 0.0 },
            "seed_ms_max": wet.seed_ms_max,
            "near": level(&wet.levels[0]),
            "patch": level(&wet.levels[1]),
        });
    }
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
            "cleared_trees": tiles.cleared,
            "edge_thinned_trees": tiles.edge_thinned,
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
    if let Some(shadows) = world.get_resource::<shadows::TreeShadows>() {
        if let Some(light) = world.get_resource::<light::VegLight>() {
            value["light"] = json!({
                "from_terrain": light.sun_dir.w > 0.5,
                "sun_dir": light.sun_dir.truncate().to_array(),
                "ambient": light.ambient.truncate().to_array(),
                "fog": light.fog.to_array(),
            });
        }
        value["shadows"] = json!({
            "cast": shadows.on, "limit": shadows.limit, "caster_batches": shadows.casters,
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
            "quad": render.assets.quad.is_some(),
            "atlas_rows": render.assets.atlas_rows,
            "atlas_mip_levels": world.get_resource::<mips::AtlasMips>().map_or(0, |m| m.levels),
            "atlas_mips_done": world.get_resource::<mips::AtlasMips>().map_or(0, |m| m.finished()),
        });
    }
    ("vegetation".into(), value)
}

/// Dump section `vegetation_ported` (veg4 lot): ground jobs, dense-forest profile per zoom,
/// procedural meshes, species / ground texture packs.
fn dump_ported(world: &World) -> (String, Value) {
    use self::detail_profile::{ForestDetailProfile, TreeRange};
    let profile = ForestDetailProfile::default();
    let range = TreeRange::default();
    let zoom: Vec<Value> = [1.0f32, 4.0, 8.0, 15.0, 22.0, 29.0, 31.0]
        .iter()
        .map(|d| {
            json!({
                "distance": d,
                "fraction": profile.fraction_at(*d, &range),
                "active": profile.active_at(*d, &range, 1.0),
                "radius": profile.radius_at(*d, &range, 1.0),
                "keep": profile.keep_for(profile.fraction_at(*d, &range)),
            })
        })
        .collect();
    let jobs = world.get_resource::<ground_job::GroundJobs>().map(|j| {
        json!({
            "running": j.running.len(), "launched": j.launched, "installed": j.installed,
            "discarded": j.discarded, "trees_seated": j.trees_seated,
            "ms_avg": if j.installed > 0 { j.ms_total / j.installed as f32 } else { 0.0 },
            "ms_max": j.ms_max,
        })
    });
    let meshes: Vec<Value> = meshes::Shape::ALL
        .iter()
        .map(
            |s| json!({"shape": format!("{s:?}"), "triangles": meshes::build(*s).triangle_count()}),
        )
        .collect();
    let packs = crate::paths::find_root().ok().map(|root| {
        let (data, assets) = (crate::paths::data_dir(&root), crate::paths::asset_root(&root));
        let trees = textures::TreeTextures::load(&data, &assets, textures::Quality::High, true);
        let ground = |source| {
            ground_materials::GroundMaterials::load(
                &data,
                &assets,
                source,
                textures::Quality::High,
            )
            .map_or(Value::Null, |m| json!({"layers": m.pack.layer_count(), "grid": [m.pack.grid.0, m.pack.grid.1]}))
        };
        json!({
            "bark_layers": trees.bark.layer_count(), "leaves_layers": trees.leaves.layer_count(),
            "textured_species": trees.textured_species(),
            "oak_bark_layer": trees.bark_layer("oak"), "fir_leaves_layer": trees.leaves_layer("fir"),
            "ground_hb": ground(ground_materials::Source::Hb),
            "ground_tx": ground(ground_materials::Source::Tx),
        })
    });
    (
        "vegetation_ported".into(),
        json!({
            "ground_jobs": jobs, "detail_zoom": zoom, "meshes": meshes, "packs": packs,
            "detail_budget": [profile.instance_budget, profile.max_cells, profile.max_ground_jobs],
        }),
    )
}

/// A scattered tree seen by the decor picking of the Codex (ADR 0219).
#[derive(Debug, Clone, PartialEq)]
pub struct TreeHit {
    /// Species id of the data (`oak`, `beech`…), the key of `CodexStore::entry_for_decor("tree", …)`.
    pub species: String,
    /// Foot of the trunk, displayed world position.
    pub foot: Vec3,
    pub height: f32,
    /// Crown half width.
    pub radius: f32,
}

/// Read access to the trees on show, for the decor bubble of the Codex (`DecorHover`). Trees
/// thinned out by the current density are not returned.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TreeDecor<'w> {
    tiles: Option<Res<'w, VegTiles>>,
    view: Option<Res<'w, VegView>>,
    world: Option<Res<'w, VegWorld>>,
}

impl TreeDecor<'_> {
    /// (tiles ready, trees held) for the dumps.
    pub fn stats(&self) -> (usize, usize) {
        let Some(tiles) = &self.tiles else {
            return (0, 0);
        };
        tiles
            .tiles
            .values()
            .filter_map(|state| match state {
                TileState::Ready { trees, .. } => Some(trees.len()),
                TileState::Pending => None,
            })
            .fold((0, 0), |(count, total), len| (count + 1, total + len))
    }

    /// Trees within `reach` map units of (x, z).
    pub fn near(&self, x: f32, z: f32, reach: f32) -> Vec<TreeHit> {
        let (Some(tiles), Some(world)) = (&self.tiles, &self.world) else {
            return Vec::new();
        };
        let (Some(grid), density) = (&tiles.grid, self.view.as_ref().map_or(1.0, |v| v.density))
        else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for (index, state) in &tiles.tiles {
            let TileState::Ready { trees, .. } = state else {
                continue;
            };
            if grid.distance(*index, x, z) > reach {
                continue;
            }
            for tree in trees.iter() {
                if tree.seed < 1.0 - density
                    || (tree.x - x).powi(2) + (tree.z - z).powi(2) > reach * reach
                {
                    continue;
                }
                let Some(species) = world.0.species_ids.get(tree.row as usize) else {
                    continue;
                };
                found.push(TreeHit {
                    species: species.clone(),
                    foot: Vec3::new(tree.x, tree.y + tree.height * tiles::GROUND_SINK, tree.z),
                    height: tree.height,
                    radius: tree.crown,
                });
            }
        }
        found
    }
}
