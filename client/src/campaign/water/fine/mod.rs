//! Close rendering of the fine hydrography, the draped roads and the fine anchors (ADR 0036):
//! port of `fine_geo_layer.gd` (this file), `fine_geo_store.gd` ([`store`]), `fine_ribbon_job.gd`
//! ([`job`]), `cafv_tile.gd` ([`cafv`]), `river_fine.gdshader` / `road_fine.gdshader`
//! ([`material`], `shaders/river_fine.wesl`, `shaders/road_fine.wesl`).
//!
//! Active only when the relief root has the fine tiles (`<root>/pyramid/hydro_fine`, outside git)
//! and under the « county » tier (near weight of `ZoomTiers`): no cost in the strategic view.
//! - E2 tiles (64 units) around the focus: read on the IO pool ([`store`]), meshed on the async
//!   compute pool ([`job`]); one river and one road mesh per tile, LRU cache, re-meshed when the
//!   finest relief page level under the tile changes.
//! - Fade: disc `fine_zone` (centre = focus, radius = tiles ready) x near weight.
//! - Anchors: crossings switch to their fine anchors through
//!   [`crate::campaign::countryside::bridges::FineBridgeMode`].
//! - Heights in metres in the meshes, draped by `relief.wesl`: nothing to rebuild on zoom.
//!
//! Dev hooks: `CENT_ANS_FINE_GEO_ROOT=<dir>` replaces the relief root holding `pyramid/hydro_fine`
//! and `pyramid/roads_fine` (e.g. a folder whose `pyramid` links to the real-map pyramid);
//! `CENT_ANS_FINE_DEBUG=1` draws the fine water magenta and the roads yellow (`--fine-debug`).
//!
//! Not ported (see `docs/wip/bm-finegeo.md`): `fine_bed_carver.gd` (carves the river bed in the
//! terrain pages: terrain lot), the fade of the old ribbons and of the terrain inside the fine disc
//! (`old_materials`, `roads.set_fine_zone`), settlement and hamlet anchors (markers lot), gate
//! bridges (settlement covers are empty since VT, ADR 0138).

pub mod cafv;
pub mod job;
pub mod material;
pub mod store;

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use bevy::tasks::{AsyncComputeTaskPool, Task, TaskPool, block_on};
use serde_json::{Value, json};

use self::cafv::{LAYER_RIVERS, LAYER_ROADS};
use self::job::{FineRibbonJob, FineRibbonOutput, finest_level};
use self::material::{FineRiverMaterial, FineRiverParams, FineRoadMaterial, FineRoadParams};
use self::store::{FineGeoStore, col_of, key_of, rect_distance, row_of, tile_rect};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::camera::zoom::{ZoomTiers, smoothstep};
use crate::campaign::countryside::bridges::{self, FineBridgeMode};
use crate::campaign::terrain::surface::Surface;

/// Radius (units) of the wanted tiles around the focus: camera distance x factor, clamped.
const RADIUS_FACTOR: f32 = 1.3;
const MIN_RADIUS: f32 = 40.0;
const MAX_RADIUS: f32 = 170.0;
/// Meshed tiles kept (visible or not).
const MAX_BUILT_TILES: usize = 40;
const MAX_BUILD_JOBS: usize = 2;
const MAX_INSTALLS_PER_FRAME: usize = 2;
/// Delay without a new page before re-meshing a tile (s).
const SURFACE_SETTLE_S: f32 = 0.6;
/// Near weight above which the crossings switch to their fine anchors, and the half width of the
/// cross-fade (ZF-A).
const BRIDGE_SWITCH_WEIGHT: f32 = 0.5;
const BRIDGE_FADE: f32 = 0.25;

/// One meshed tile.
struct BuiltTile {
    root: Entity,
    river: Option<Entity>,
    road: Option<Entity>,
    used: u64,
    /// Time (s) the surface under the tile changed; `None` when up to date.
    dirty: Option<f32>,
    version: u32,
    finest: i32,
    river_points: usize,
    road_points: usize,
}

/// Counters for the dump.
#[derive(Debug, Default, Clone)]
pub struct FineStats {
    pub builds: u32,
    pub build_ms_total: f32,
    pub build_ms_max: f32,
    pub update_ms_max: f32,
    pub remeshes: u32,
}

/// State of the fine layer.
#[derive(Resource, Default)]
pub struct FineGeo {
    pub enabled: bool,
    /// Why the layer is off (empty when on).
    pub reason: String,
    pub tiles_root: PathBuf,
    store: Option<FineGeoStore>,
    built: HashMap<i32, BuiltTile>,
    jobs: HashMap<i32, Task<FineRibbonOutput>>,
    ready: VecDeque<FineRibbonOutput>,
    wanted: Vec<i32>,
    frame: u64,
    pub weight: f32,
    pub zone: Vec4,
    pub focus: Vec2,
    pub radius: f32,
    river_material: Handle<FineRiverMaterial>,
    road_material: Handle<FineRoadMaterial>,
    zones: Vec<[f32; 3]>,
    towns: Vec<[f32; 3]>,
    towns_ready: bool,
    meters_per_unit: f32,
    /// Minimum display rank by quality (low 5, medium 4, else 3) and radius scale.
    pub min_order: u32,
    pub radius_scale: f32,
    surface_version: u64,
    pub stats: FineStats,
}

impl FineGeo {
    pub fn store(&self) -> Option<&FineGeoStore> {
        self.store.as_ref()
    }
}

pub struct FineGeoPlugin;

impl Plugin for FineGeoPlugin {
    fn build(&self, app: &mut App) {
        material::register_shaders(app);
        app.init_resource::<FineGeo>()
            .init_resource::<FineBridgeMode>()
            .register_dump(dump_fine_geo);
        if app.world().contains_resource::<Assets<Mesh>>() {
            app.add_plugins((
                MaterialPlugin::<FineRiverMaterial>::default(),
                MaterialPlugin::<FineRoadMaterial>::default(),
            ))
            .add_systems(
                PostUpdate,
                (
                    super::drape::sync_drape::<FineRiverMaterial>,
                    super::drape::sync_drape::<FineRoadMaterial>,
                ),
            )
            .add_systems(OnEnter(AppState::Campaign), setup_fine_geo)
            .add_systems(
                Update,
                (
                    update_fine_geo,
                    bridges::apply_fine_bridges
                        .after(bridges::spawn_bridges)
                        .before(bridges::ground_bridges),
                )
                    .run_if(in_state(AppState::Campaign)),
            );
        }
    }
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Relief root holding `pyramid/hydro_fine` (`map_paths.relief_root_for`): the env override, else
/// the map folder.
fn tiles_root(map_dir: &Path) -> PathBuf {
    match std::env::var("CENT_ANS_FINE_GEO_ROOT") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => map_dir.to_path_buf(),
    }
}

/// Minimum rank and radius scale by quality level (`apply_render_quality`).
fn quality_of(level: &str) -> (u32, f32) {
    match level {
        "low" => (5, 0.6),
        "medium" => (4, 0.8),
        _ => (3, 1.0),
    }
}

#[allow(clippy::too_many_arguments)]
fn setup_fine_geo(
    mut commands: Commands,
    mut fine: ResMut<FineGeo>,
    mut bridges: ResMut<FineBridgeMode>,
    mut river_materials: ResMut<Assets<FineRiverMaterial>>,
    mut road_materials: ResMut<Assets<FineRoadMaterial>>,
    quality: Option<Res<crate::settings::RenderQuality>>,
) {
    // Entities of a previous visit died with `DespawnOnExit`; tasks are dropped (cancelled).
    *fine = FineGeo::default();
    *bridges = FineBridgeMode::default();
    let Ok(root) = crate::paths::find_root() else {
        fine.reason = "racine du dépôt introuvable".into();
        return;
    };
    let map_dir = crate::paths::data_dir(&root).join("map");
    let tiles = tiles_root(&map_dir);
    let (store, ok) = FineGeoStore::load_from(&map_dir, &tiles);
    fine.tiles_root = tiles.clone();
    if !ok || !store.available(LAYER_RIVERS) {
        fine.reason = format!(
            "pas de tuiles fines sous {} (pyramid/hydro_fine)",
            tiles.display()
        );
        info!("fine geo: off ({})", fine.reason);
        return;
    }
    let meters = read_json(&map_dir.join("map.json"))
        .and_then(|m| m.get("meters_per_px").and_then(Value::as_f64))
        .unwrap_or(719.0) as f32;
    fine.meters_per_unit = meters;
    let (min_order, radius_scale) = quality
        .as_deref()
        .map_or((3, 1.0), |q| quality_of(&q.level));
    fine.min_order = min_order;
    fine.radius_scale = radius_scale;
    if let Some(bytes) = std::fs::read(map_dir.join("rivers_render.json")).ok()
        && let Ok(rendered) = data_model::map_geo::parse_rendered_rivers(&bytes)
    {
        fine.zones = rendered
            .zones
            .iter()
            .map(|z| [z.px[0], z.px[1], z.radius_px])
            .collect();
    }
    let display = read_json(&map_dir.join("river_display.json")).unwrap_or(Value::Null);
    let debug = std::env::var("CENT_ANS_FINE_DEBUG").is_ok_and(|v| v == "1");
    let mut river = FineRiverParams::from_display(&display, meters);
    let mut road = FineRoadParams {
        meters_per_unit: meters,
        ..Default::default()
    };
    if debug {
        river.debug_flat = 1.0;
        road.debug_flat = 1.0;
    }
    fine.river_material = river_materials.add(FineRiverMaterial {
        params: river,
        ..Default::default()
    });
    fine.road_material = road_materials.add(FineRoadMaterial {
        params: road,
        ..Default::default()
    });
    // Crossings: fine anchors by id (order of `crossings_px.json`, matched by id here).
    bridges.anchors = store
        .crossings
        .iter()
        .filter(|c| c.snapped)
        .map(|c| (c.id.clone(), c.clone()))
        .collect();
    bridges.meters_per_unit = meters;
    bridges.gen_width_scale = river.gen_width_scale;
    bridges.gen_width_ref = river.gen_width_ref;
    info!(
        "fine geo: on, {} river tiles, {} road tiles, {} crossing anchors",
        store.tile_count(LAYER_RIVERS),
        store.tile_count(LAYER_ROADS),
        bridges.anchors.len()
    );
    fine.store = Some(store);
    fine.enabled = true;
    commands.spawn((
        Name::new("FineGeo"),
        FineGeoRoot,
        Transform::IDENTITY,
        Visibility::Hidden,
        DespawnOnExit(AppState::Campaign),
    ));
}

/// Parent of the tile entities (hidden above the near tier).
#[derive(Component)]
pub struct FineGeoRoot;

/// Tiles of `store` crossing the disc, sorted by distance.
fn wanted_tiles(store: &FineGeoStore, focus: Vec2, radius: f32) -> Vec<i32> {
    let t = cafv::TILE_UNITS;
    let mut order: Vec<(f32, i32)> = Vec::new();
    for row in ((focus.y - radius) / t).floor() as i32..=((focus.y + radius) / t).floor() as i32 {
        for col in ((focus.x - radius) / t).floor() as i32..=((focus.x + radius) / t).floor() as i32
        {
            if col < 0 || row < 0 {
                continue;
            }
            if !store.has_tile(LAYER_RIVERS, col, row) && !store.has_tile(LAYER_ROADS, col, row) {
                continue;
            }
            let d = rect_distance(tile_rect(col, row), focus);
            if d < radius {
                order.push((d, key_of(col, row)));
            }
        }
    }
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    order.into_iter().map(|(_, k)| k).collect()
}

/// Copy of the resident pages touching the rectangle (job snapshot).
fn snapshot(surface: &Surface, rect: [f32; 4]) -> Surface {
    use crate::campaign::terrain::pyramid::{
        ReliefPyramid, col_of_key, level_of_key, row_of_key, tile_units,
    };
    let mut out = Surface::new(surface.heights.clone(), surface.pyramid.clone());
    let keys: Vec<i64> = surface.page_keys().collect();
    for key in keys {
        let level = level_of_key(key);
        let (ox, oz) = ReliefPyramid::tile_origin(level, col_of_key(key), row_of_key(key));
        let size = tile_units(level) as f32;
        let (ox, oz) = (ox as f32, oz as f32);
        if ox < rect[2]
            && ox + size > rect[0]
            && oz < rect[3]
            && oz + size > rect[1]
            && let Some(page) = surface.page(key)
        {
            out.insert_page(key, page.clone());
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn update_fine_geo(
    mut commands: Commands,
    mut fine: ResMut<FineGeo>,
    mut bridges: ResMut<FineBridgeMode>,
    rig: Option<Res<CampaignRig>>,
    tiers: Option<Res<ZoomTiers>>,
    surface: Option<Res<Surface>>,
    places: Option<Res<crate::campaign::markers::places::Places>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut river_materials: ResMut<Assets<FineRiverMaterial>>,
    mut road_materials: ResMut<Assets<FineRoadMaterial>>,
    mut roots: Query<(Entity, &mut Visibility), With<FineGeoRoot>>,
    mut tile_vis: Query<&mut Visibility, Without<FineGeoRoot>>,
    time: Res<Time>,
) {
    if !fine.enabled {
        return;
    }
    let (Some(rig), Some(tiers)) = (rig, tiers) else {
        return;
    };
    let started = std::time::Instant::now();
    let fine = &mut *fine;
    fine.frame += 1;
    let now = time.elapsed_secs();
    if !fine.towns_ready
        && let Some(places) = places.as_ref().filter(|p| !p.places.is_empty())
    {
        fine.towns = places
            .places
            .iter()
            .filter(|p| p.kind == "city" || p.kind == "town")
            .map(|p| [p.px.x, p.px.y, p.radius * 1.3 + 0.05])
            .collect();
        fine.towns_ready = true;
    }
    let Some(store) = fine.store.as_mut() else {
        return;
    };
    store.poll(false);
    let weight = tiers.near_weight(rig.distance);
    let Ok((root, mut root_vis)) = roots.single_mut() else {
        return;
    };
    if (weight - fine.weight).abs() > 1e-4 {
        fine.weight = weight;
        let shown = if weight > 0.001 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *root_vis != shown {
            *root_vis = shown;
        }
        if let Some(mut m) = river_materials.get_mut(&fine.river_material) {
            m.params.fine_weight = weight;
        }
        if let Some(mut m) = road_materials.get_mut(&fine.road_material) {
            m.params.fine_weight = weight;
        }
    }
    // ZF-A: cross-fade coarse bridges -> fine ones around the switch weight.
    let fine_alpha = smoothstep(
        BRIDGE_SWITCH_WEIGHT - BRIDGE_FADE,
        BRIDGE_SWITCH_WEIGHT + BRIDGE_FADE,
        weight,
    );
    let fine_mode = fine_alpha >= 0.5;
    if bridges.fine != fine_mode {
        bridges.fine = fine_mode;
    }
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    collect_jobs(&mut fine.jobs, &mut fine.ready, false);
    if weight <= 0.001 {
        set_zone(
            fine,
            Vec4::new(0.0, 0.0, -1.0, 0.0),
            &mut river_materials,
            &mut road_materials,
        );
        note_update(fine, started);
        return;
    }
    let radius = (rig.distance * RADIUS_FACTOR).clamp(MIN_RADIUS, MAX_RADIUS) * fine.radius_scale;
    fine.focus = focus;
    fine.radius = radius;
    let store = fine.store.as_mut().expect("store");
    fine.wanted = wanted_tiles(store, focus, radius);
    for &key in &fine.wanted {
        store.request(LAYER_RIVERS, col_of(key), row_of(key));
        store.request(LAYER_ROADS, col_of(key), row_of(key));
    }
    if let Some(surface) = surface.as_deref() {
        mark_dirty(fine, surface, now);
        start_jobs(fine, surface, now);
    }
    install_ready(fine, root, &mut commands, &mut meshes);
    // Disc entirely covered by meshed tiles.
    let mut covered = radius;
    for &key in &fine.wanted {
        if fine.built.get(&key).is_none_or(|b| b.version == 0) {
            covered = covered.min(rect_distance(tile_rect(col_of(key), row_of(key)), focus));
        }
    }
    set_zone(
        fine,
        Vec4::new(focus.x, focus.y, covered, weight),
        &mut river_materials,
        &mut road_materials,
    );
    let frame = fine.frame;
    let wanted: std::collections::HashSet<i32> = fine.wanted.iter().copied().collect();
    for (key, tile) in &mut fine.built {
        let show = wanted.contains(key);
        if show {
            tile.used = frame;
        }
        if let Ok(mut vis) = tile_vis.get_mut(tile.root) {
            let v = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *vis != v {
                *vis = v;
            }
        }
    }
    evict(fine, &mut commands);
    note_update(fine, started);
}

fn note_update(fine: &mut FineGeo, started: std::time::Instant) {
    let ms = started.elapsed().as_secs_f32() * 1000.0;
    fine.stats.update_ms_max = fine.stats.update_ms_max.max(ms);
}

fn set_zone(
    fine: &mut FineGeo,
    zone: Vec4,
    rivers: &mut Assets<FineRiverMaterial>,
    roads: &mut Assets<FineRoadMaterial>,
) {
    if fine.zone.abs_diff_eq(zone, 1e-4) {
        return;
    }
    fine.zone = zone;
    if let Some(mut m) = rivers.get_mut(&fine.river_material) {
        m.params.fine_zone = zone;
    }
    if let Some(mut m) = roads.get_mut(&fine.road_material) {
        m.params.fine_zone = zone;
    }
}

/// Re-mesh a tile when the finest page level under it changed (only up to E4: the detail zones
/// E5-E7 already have the bed carved in their pages, small surface gap).
fn mark_dirty(fine: &mut FineGeo, surface: &Surface, now: f32) {
    if surface.version == fine.surface_version {
        return;
    }
    fine.surface_version = surface.version;
    for (&key, tile) in &mut fine.built {
        let finest = finest_level(surface, tile_rect(col_of(key), row_of(key)));
        if finest != tile.finest && finest.min(4) != tile.finest.min(4) && tile.dirty.is_none() {
            tile.dirty = Some(now);
        }
    }
}

fn start_jobs(fine: &mut FineGeo, surface: &Surface, now: f32) {
    let wanted = fine.wanted.clone();
    for key in wanted {
        if fine.jobs.len() >= MAX_BUILD_JOBS {
            return;
        }
        if fine.jobs.contains_key(&key) {
            continue;
        }
        let needs = match fine.built.get(&key) {
            None => true,
            Some(t) => t.dirty.is_some_and(|d| now - d >= SURFACE_SETTLE_S),
        };
        if !needs {
            continue;
        }
        let (col, row) = (col_of(key), row_of(key));
        let store = fine.store.as_mut().expect("store");
        if !store.is_ready(LAYER_RIVERS, col, row) || !store.is_ready(LAYER_ROADS, col, row) {
            continue;
        }
        let rect = tile_rect(col, row);
        let grown = [rect[0] - 1.0, rect[1] - 1.0, rect[2] + 1.0, rect[3] + 1.0];
        let near = |c: &[f32; 3]| {
            c[0] > rect[0] - c[2]
                && c[0] < rect[2] + c[2]
                && c[1] > rect[1] - c[2]
                && c[1] < rect[3] + c[2]
        };
        let job = FineRibbonJob {
            key,
            river_tile: store.get(LAYER_RIVERS, col, row),
            road_tile: store.get(LAYER_ROADS, col, row),
            surface: Some(snapshot(surface, grown)),
            zones: fine.zones.iter().filter(|z| near(z)).copied().collect(),
            towns: fine.towns.iter().filter(|t| near(t)).copied().collect(),
            meters_per_unit: fine.meters_per_unit,
            min_order: fine.min_order,
        };
        if let Some(t) = fine.built.get_mut(&key) {
            t.dirty = None;
            fine.stats.remeshes += 1;
        }
        let pool = AsyncComputeTaskPool::get_or_init(TaskPool::default);
        fine.jobs.insert(key, pool.spawn(async move { job.run() }));
    }
}

fn collect_jobs(
    jobs: &mut HashMap<i32, Task<FineRibbonOutput>>,
    ready: &mut VecDeque<FineRibbonOutput>,
    block: bool,
) {
    let done: Vec<i32> = jobs
        .iter()
        .filter(|(_, t)| block || t.is_finished())
        .map(|(k, _)| *k)
        .collect();
    for key in done {
        let task = jobs.remove(&key).expect("job");
        ready.push_back(block_on(task));
    }
}

fn install_ready(
    fine: &mut FineGeo,
    root: Entity,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
) {
    for _ in 0..MAX_INSTALLS_PER_FRAME {
        let Some(out) = fine.ready.pop_front() else {
            return;
        };
        let frame = fine.frame;
        let tile = fine.built.entry(out.key).or_insert_with(|| {
            let entity = commands
                .spawn((
                    Name::new(format!("FineTile_{}_{}", col_of(out.key), row_of(out.key))),
                    Transform::IDENTITY,
                    Visibility::Inherited,
                    ChildOf(root),
                ))
                .id();
            BuiltTile {
                root: entity,
                river: None,
                road: None,
                used: frame,
                dirty: None,
                version: 0,
                finest: -1,
                river_points: 0,
                road_points: 0,
            }
        });
        for (slot, mesh, river) in [
            (&mut tile.river, out.river, true),
            (&mut tile.road, out.road, false),
        ] {
            if let Some(old) = slot.take() {
                commands.entity(old).try_despawn();
            }
            let Some(mesh) = mesh else { continue };
            let mut e = commands.spawn((
                Name::new(if river { "FineRivers" } else { "FineRoads" }),
                Mesh3d(meshes.add(mesh)),
                Transform::IDENTITY,
                NotShadowCaster,
                NoFrustumCulling,
                ChildOf(tile.root),
            ));
            if river {
                e.insert(MeshMaterial3d(fine.river_material.clone()));
            } else {
                e.insert(MeshMaterial3d(fine.road_material.clone()));
            }
            *slot = Some(e.id());
        }
        tile.version += 1;
        tile.finest = out.finest;
        tile.river_points = out.river_points;
        tile.road_points = out.road_points;
        fine.stats.builds += 1;
        fine.stats.build_ms_total += out.build_ms;
        fine.stats.build_ms_max = fine.stats.build_ms_max.max(out.build_ms);
    }
}

fn evict(fine: &mut FineGeo, commands: &mut Commands) {
    if fine.built.len() <= MAX_BUILT_TILES {
        return;
    }
    let mut keys: Vec<(u64, i32)> = fine.built.iter().map(|(k, t)| (t.used, *k)).collect();
    keys.sort();
    for (used, key) in keys {
        if fine.built.len() <= MAX_BUILT_TILES || used >= fine.frame {
            return;
        }
        if let Some(tile) = fine.built.remove(&key) {
            commands.entity(tile.root).try_despawn();
        }
    }
}

fn dump_fine_geo(world: &World) -> (String, Value) {
    let fine = world.resource::<FineGeo>();
    let bridges = world.resource::<FineBridgeMode>();
    let store = fine.store.as_ref();
    let built_visible = fine
        .built
        .keys()
        .filter(|k| fine.wanted.contains(k))
        .count();
    let river_points: usize = fine.built.values().map(|t| t.river_points).sum();
    let road_points: usize = fine.built.values().map(|t| t.road_points).sum();
    (
        "campaign_fine_geo".into(),
        json!({
            "enabled": fine.enabled,
            "reason": fine.reason,
            "tiles_root": fine.tiles_root.display().to_string(),
            "river_tiles_indexed": store.map_or(0, |s| s.tile_count(LAYER_RIVERS)),
            "road_tiles_indexed": store.map_or(0, |s| s.tile_count(LAYER_ROADS)),
            "anchors": {
                "settlements": store.map_or(0, |s| s.settlements.len()),
                "hamlets": store.map_or(0, |s| s.hamlets.len()),
                "crossings": store.map_or(0, |s| s.crossings.len()),
            },
            "tiles_loaded": store.map_or(0, |s| s.loaded_tiles),
            "tiles_cached": [store.map_or(0, |s| s.cached(LAYER_RIVERS)), store.map_or(0, |s| s.cached(LAYER_ROADS))],
            "loads_pending": store.map_or(0, |s| s.pending()),
            "weight": fine.weight,
            "focus": [fine.focus.x, fine.focus.y],
            "radius": fine.radius,
            "zone": [fine.zone.x, fine.zone.y, fine.zone.z, fine.zone.w],
            "wanted": fine.wanted.len(),
            "built": fine.built.len(),
            "built_visible": built_visible,
            "jobs": fine.jobs.len(),
            "ready": fine.ready.len(),
            "river_points": river_points,
            "road_points": road_points,
            "min_order": fine.min_order,
            "zones": fine.zones.len(),
            "towns": fine.towns.len(),
            "builds": fine.stats.builds,
            "remeshes": fine.stats.remeshes,
            "build_ms_avg": if fine.stats.builds > 0 { fine.stats.build_ms_total / fine.stats.builds as f32 } else { 0.0 },
            "build_ms_max": fine.stats.build_ms_max,
            "update_ms_max": fine.stats.update_ms_max,
            "bridges": {
                "fine_mode": bridges.fine,
                "anchors": bridges.anchors.len(),
                "reshaped": bridges.reshaped,
            },
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_levels_follow_godot() {
        assert_eq!(quality_of("low"), (5, 0.6));
        assert_eq!(quality_of("medium"), (4, 0.8));
        assert_eq!(quality_of("ultra"), (3, 1.0));
    }

    #[test]
    fn wanted_tiles_sorted_by_distance() {
        let mut store = FineGeoStore::default();
        for (c, r) in [(10, 10), (11, 10), (14, 10)] {
            store.insert(
                LAYER_RIVERS,
                cafv::CafvTile::single_line(LAYER_RIVERS, c, r, 0, &[[0.0; 4]; 2]),
            );
        }
        let focus = Vec2::new(11.5 * 64.0, 10.5 * 64.0);
        let wanted = wanted_tiles(&store, focus, 80.0);
        assert_eq!(wanted, vec![key_of(11, 10), key_of(10, 10)]);
    }
}
