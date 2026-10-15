//! Relief, ground colormap, vegetation and fields of the campaign map.
//!
//! Ports of `game/scripts/map/`: `relief_pyramid.gd` (`pyramid`), `relief_quadtree.gd` and the
//! `relief-lod` crate (used as a dependency: `lod`), `terrain_builder.gd` (`mesh`, `surface`),
//! `relief_floor.gd` / `relief_state.gd` / `relief_exaggeration_profile.gd` (`relief_floor`,
//! `relief_state`, `profile`), the PNG tile decoder of `godot-bridge` (`decode`), the ground
//! colormap of `relief_landcover.gd` (`colormap`) and `terrain.gdshader` +
//! `relief_quadtree.gdshaderinc` (`material` + `terrain.wesl`).
//!
//! Not ported yet (other parts of `terrain.gdshader` / the map scripts): splat and biome
//! textures, procedural parcels, seasons / weather / fog of war, borders and province
//! highlighting, rivers, forests, vegetation and fields (`agri_seasons.gd`, `biome_parents.gd`,
//! `cafv_tile.gd`), geomorphing between levels (skirts hide the cracks) and shadows.
//!
//! Other lots read the ground through [`surface::GroundHeight`] (displayed height at a map
//! point). Dump section `campaign_terrain`. Owned by a single lot of agent (ADR 0357).

pub mod colormap;
pub mod cpu_sampler;
pub mod decode;
pub mod lod;
pub mod material;
pub mod mesh;
pub mod profile;
pub mod pyramid;
pub mod relief_floor;
pub mod relief_gpu;
pub mod relief_state;
pub mod surface;
pub mod tres;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use serde_json::{Value, json};

use self::colormap::{GpuCopy, bc1_image, load_colormap};
use self::lod::{TerrainLod, update_lod};
use self::material::{TerrainMaterial, TerrainParams, sun_direction};
use self::profile::{CloseCameraProfile, ReliefProfile};
use self::pyramid::ReliefPyramid;
use self::relief_floor::{FloorGrid, HeightGrid};
use self::relief_state::ReliefState;
use self::surface::Surface;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::CameraSet;
use crate::paths;

pub struct CampaignTerrainPlugin;

impl Plugin for CampaignTerrainPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "terrain.wesl");
        // Shared relief module: `import cent_ans_client::campaign::terrain::relief::{...}`.
        bevy::shader::load_shader_library!(app, "relief.wesl");
        app.init_resource::<cpu_sampler::CpuSampler>();
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .add_systems(
                OnEnter(AppState::Campaign),
                (
                    start_loading,
                    spawn_sun,
                    create_material,
                    cpu_sampler::install_samplers,
                ),
            )
            .add_systems(OnExit(AppState::Campaign), teardown)
            .add_systems(
                Update,
                (
                    poll_world,
                    poll_colormap,
                    relief_gpu::sync_relief_gpu,
                    ensure_lod,
                    update_lod
                        .after(CameraSet)
                        .run_if(resource_exists::<TerrainLod>)
                        .run_if(resource_exists::<crate::campaign::camera::rig::CampaignRig>),
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            )
            .add_systems(
                PreUpdate,
                cpu_sampler::sync_snapshot.run_if(in_state(AppState::Campaign)),
            )
            .register_dump(dump_terrain);
    }
}

/// Everything computed off the main thread at load time.
struct LoadedWorld {
    heights: HeightGrid,
    pyramid: ReliefPyramid,
    floor: FloorGrid,
    relief_profile: ReliefProfile,
    camera_profile: CloseCameraProfile,
    load_ms: f32,
}

type Shared<T> = Arc<Mutex<Option<Result<T, String>>>>;

#[derive(Resource)]
struct PendingWorld(Shared<LoadedWorld>);

#[derive(Resource)]
struct PendingColormap(Shared<Option<GpuCopy>>);

/// Loaded, persistent between campaign visits.
#[derive(Resource, Clone)]
pub struct TerrainWorld {
    pub heights: Arc<HeightGrid>,
    pub pyramid: Arc<ReliefPyramid>,
    pub load_ms: f32,
}

/// The colormap image once on the GPU side.
#[derive(Resource, Clone)]
pub struct Colormap(pub Handle<Image>);

/// Marks the sun light of the campaign.
#[derive(Component)]
pub struct CampaignSun;

#[derive(Resource, Default)]
struct TerrainMessages {
    errors: Vec<String>,
}

fn start_loading(
    mut commands: Commands,
    world: Option<Res<TerrainWorld>>,
    colormap: Option<Res<Colormap>>,
    pending_world: Option<Res<PendingWorld>>,
    pending_colormap: Option<Res<PendingColormap>>,
) {
    commands.init_resource::<TerrainMessages>();
    let root: PathBuf = match paths::find_root() {
        Ok(root) => root,
        Err(message) => {
            error!("campaign terrain: {message}");
            return;
        }
    };
    let map_dir = root.join("data/map");
    if world.is_none() && pending_world.is_none() {
        let slot: Shared<LoadedWorld> = Arc::new(Mutex::new(None));
        let out = Arc::clone(&slot);
        let root = root.clone();
        std::thread::spawn(move || {
            let result = load_world(&root);
            *out.lock().expect("terrain load slot") = Some(result);
        });
        commands.insert_resource(PendingWorld(slot));
    }
    if colormap.is_none() && pending_colormap.is_none() {
        let slot: Shared<Option<GpuCopy>> = Arc::new(Mutex::new(None));
        let out = Arc::clone(&slot);
        std::thread::spawn(move || {
            *out.lock().expect("colormap slot") = Some(load_colormap(&map_dir));
        });
        commands.insert_resource(PendingColormap(slot));
    }
}

/// Heightmap, pyramid index and valley floor (the heavy part of `TerrainBuilder.build`).
fn load_world(root: &std::path::Path) -> Result<LoadedWorld, String> {
    let started = Instant::now();
    let map_dir = root.join("data/map");
    let pyramid = ReliefPyramid::load(&map_dir)?;
    let meta: Value = serde_json::from_str(
        &std::fs::read_to_string(map_dir.join("map.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // `heightmap_render.png` when `map.json` names it (MapData), else `heightmap.png`.
    let file = meta["render_heightmap"]["file"]
        .as_str()
        .unwrap_or("heightmap.png");
    let image = decode::decode_gray16(&map_dir.join(file))?;
    let h_min = meta["height_min_m"].as_f64().unwrap_or(-200.0) as f32;
    let h_max = meta["height_max_m"].as_f64().unwrap_or(4800.0) as f32;
    let heights = HeightGrid {
        width: image.width,
        height: image.height,
        data: image.data,
        h_min_m: h_min,
        h_range_m: h_max - h_min,
    };
    let relief_profile = ReliefProfile::load(root);
    let camera_profile = CloseCameraProfile::load(root);
    let floor = if relief_profile.enabled {
        relief_floor::compute(&heights, &relief_profile)
    } else {
        FloorGrid::default()
    };
    Ok(LoadedWorld {
        heights,
        pyramid,
        floor,
        relief_profile,
        camera_profile,
        load_ms: started.elapsed().as_secs_f32() * 1000.0,
    })
}

fn poll_world(
    mut commands: Commands,
    pending: Option<Res<PendingWorld>>,
    mut messages: ResMut<TerrainMessages>,
) {
    let Some(pending) = pending else {
        return;
    };
    let Some(result) = pending.0.lock().expect("terrain load slot").take() else {
        return;
    };
    commands.remove_resource::<PendingWorld>();
    match result {
        Ok(loaded) => {
            let heights = Arc::new(loaded.heights);
            let pyramid = Arc::new(loaded.pyramid);
            let mut relief = ReliefState::new(loaded.relief_profile, loaded.camera_profile);
            relief.set_floor(loaded.floor);
            info!(
                "campaign terrain: world {}x{}, pyramid E0-E{} ({} tiles), loaded in {:.0} ms",
                heights.width,
                heights.height,
                pyramid.max_level,
                pyramid.tile_count(),
                loaded.load_ms
            );
            commands.insert_resource(Surface::new(Arc::clone(&heights), Arc::clone(&pyramid)));
            commands.insert_resource(relief);
            commands.insert_resource(TerrainWorld {
                heights,
                pyramid,
                load_ms: loaded.load_ms,
            });
        }
        Err(message) => {
            error!("campaign terrain: {message}");
            messages.errors.push(message);
        }
    }
}

fn poll_colormap(
    mut commands: Commands,
    pending: Option<Res<PendingColormap>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    handle_res: Option<Res<TerrainMaterialHandle>>,
    mut messages: ResMut<TerrainMessages>,
) {
    let Some(pending) = pending else {
        return;
    };
    let Some(result) = pending.0.lock().expect("colormap slot").take() else {
        return;
    };
    commands.remove_resource::<PendingColormap>();
    match result {
        Ok(Some(copy)) => {
            let handle = images.add(bc1_image(copy));
            // The material exists from OnEnter; the LOD state may not (the colormap can arrive
            // before the world): bind through the material handle, never through the LOD.
            if let Some(handle_res) = handle_res
                && let Some(mut material) = materials.get_mut(&handle_res.0)
            {
                material.colormap = Some(handle.clone());
                material.params.map.w = 1.0;
            }
            commands.insert_resource(Colormap(handle));
        }
        Ok(None) => warn!("campaign terrain: map.json has no colormap; procedural fallback colour"),
        Err(message) => {
            error!("campaign terrain: colormap: {message}");
            messages.errors.push(message);
        }
    }
}

fn create_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    colormap: Option<Res<Colormap>>,
    existing: Option<Res<TerrainMaterialHandle>>,
) {
    if existing.is_some() {
        return;
    }
    let mut params = TerrainParams::default();
    if colormap.is_some() {
        params.map.w = 1.0;
    }
    let handle = materials.add(TerrainMaterial {
        params,
        colormap: colormap.map(|c| c.0.clone()),
    });
    commands.insert_resource(TerrainMaterialHandle(handle));
}

#[derive(Resource)]
struct TerrainMaterialHandle(Handle<TerrainMaterial>);

fn spawn_sun(mut commands: Commands) {
    let dir = sun_direction(34.0);
    commands.spawn((
        Name::new("CampaignSun"),
        CampaignSun,
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.86),
            illuminance: 3_500.0,
            shadow_maps_enabled: false,
            ..default()
        },
        // The light travels opposite to the direction towards the sun.
        Transform::from_translation(Vec3::ZERO).looking_to(-dir, Vec3::Y),
        DespawnOnExit(AppState::Campaign),
    ));
}

#[allow(clippy::too_many_arguments)]
/// Creates the LOD state once the world is loaded and the camera rig exists.
fn ensure_lod(
    mut commands: Commands,
    world: Option<Res<TerrainWorld>>,
    lod: Option<Res<TerrainLod>>,
    handle: Option<Res<TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    relief: Option<Res<ReliefState>>,
    surface: Option<Res<Surface>>,
    time: Res<Time>,
) {
    if lod.is_some() {
        return;
    }
    let (Some(world), Some(handle), Some(relief)) = (world, handle, relief) else {
        return;
    };
    if let Some(mut material) = materials.get_mut(&handle.0) {
        material.params.map.x = world.heights.width as f32;
        material.params.map.y = world.heights.height as f32;
        material.params.scale.x = relief.vertical_scale;
        material.params.scale.y = relief.gain;
        material.params.scale.z = relief.squash;
    }
    let mut state = TerrainLod::new(&world.pyramid, &world.heights, handle.0.clone());
    if let Some(surface) = surface {
        state.adopt_pages(&surface, time.elapsed_secs_f64());
    }
    commands.insert_resource(state);
}

fn teardown(mut commands: Commands) {
    // Node entities carry `DespawnOnExit`; the selector must start afresh on the next visit.
    commands.remove_resource::<TerrainLod>();
}

fn dump_terrain(world: &World) -> (String, Value) {
    let mut value = json!({ "ready": false });
    if let Some(messages) = world.get_resource::<TerrainMessages>()
        && !messages.errors.is_empty()
    {
        value["errors"] = json!(messages.errors);
    }
    if let Some(terrain) = world.get_resource::<TerrainWorld>() {
        value["world"] = json!({
            "size_px": [terrain.heights.width, terrain.heights.height],
            "load_ms": terrain.load_ms,
            "pyramid_max_level": terrain.pyramid.max_level,
            "pyramid_tiles": terrain.pyramid.tile_count(),
            "root_tiles": [terrain.pyramid.root_cols, terrain.pyramid.root_rows],
        });
    }
    value["colormap_loaded"] = json!(world.contains_resource::<Colormap>());
    value["colormap_pending"] = json!(world.contains_resource::<PendingColormap>());
    if let Some(handle) = world.get_resource::<TerrainMaterialHandle>()
        && let Some(material) = world.resource::<Assets<TerrainMaterial>>().get(&handle.0)
    {
        value["material"] = json!({
            "colormap_flag": material.params.map.w,
            "colormap_bound": material.colormap.is_some(),
            "scale": material.params.scale.to_array(),
        });
    }
    if let Some(sampler) = world.get_resource::<cpu_sampler::CpuSampler>() {
        value["cpu_sampler"] = json!({
            "ready": sampler.is_ready(),
            "center_height_m": sampler.height_m(4096.0, 3072.0),
            "center_display": sampler.display_height(4096.0, 3072.0),
        });
    }
    if let Some(gpu) = world.get_resource::<relief_gpu::ReliefGpu>() {
        value["relief_gpu"] = json!({
            "scale": gpu.params.scale.to_array(),
            "info": gpu.params.info.to_array(),
        });
    }
    if let Some(relief) = world.get_resource::<ReliefState>() {
        value["relief"] = json!({
            "vertical_scale": relief.vertical_scale,
            "gain": relief.gain,
            "squash": relief.squash,
            "floor_cells": relief.grid.side,
            "has_floor": relief.has_floor(),
        });
    }
    if let Some(surface) = world.get_resource::<Surface>() {
        value["resident_pages"] = json!(surface.page_count());
        value["pages_per_level"] = json!(surface.pages_per_level());
    }
    if let Some(lod) = world.get_resource::<TerrainLod>() {
        value["ready"] = json!(true);
        value["selected_nodes"] = json!(lod.selected_items);
        value["node_entities"] = json!(lod.node_count());
        value["built_nodes"] = json!(lod.built_nodes());
        value["peak_nodes"] = json!(lod.peak_nodes);
        value["triangles"] = json!(lod.triangles());
        value["pending_builds"] = json!(lod.pending_builds);
        value["meshes_built"] = json!(lod.meshes_built);
        value["pages_loaded"] = json!(lod.pages_loaded);
        value["pages_in_flight"] = json!(lod.in_flight());
        value["missing_pages"] = json!(lod.missing_pages);
        value["px_scale"] = json!(lod.px_scale);
        value["build_ms_last"] = json!(lod.build_ms_last);
        value["settled"] = json!(lod.is_settled());
        value["frame"] = json!(lod.frame);
    }
    ("campaign_terrain".into(), value)
}
