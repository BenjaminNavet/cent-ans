//! Far layer of the 1:1 towns (port of `town_far_layer.gd`, `town_far_mask.gd`, the soot of
//! `town_soot.gd`; ADR 0138), merged by map tiles.
//!
//! Godot drew it in the `real` style only (every town 1:1, no maquette). In Bevy the 1:1 layer
//! (`layer.rs`) streams only the towns around the rig and the markers lot keeps the maquettes
//! (ADR 0158) beyond: without a far layer, a 1:1 town stood beside giant maquettes on the
//! horizon. The far layer fills the band in between: while the 1:1 layer is active (rig below
//! `max_rig_distance`), every town of `towns_1340.json` that is not streamed is drawn at 1:1 by
//! its far mesh and its maquette sinks (`TownsState::is_shown`); past the threshold the far mesh
//! sinks with the layer weight as the maquettes rise. `CENT_ANS_TOWN_FAR=0` turns it off.
//!
//! - Town index (mask texel): order of the ids (sorted), landmark cities left to their maquette.
//! - Generation on load in the async compute pool, one task per tile (F1: `f1_tile` units, F2:
//!   `f2_tile`); the main thread only builds the meshes (a few per frame).
//! - F1 below `f1_range` of the camera, F2 beyond; no shadows (the vertex stage is custom).
//! - Mask (Rgba8 64 x 64): R = the 1:1 town is built (its far mesh is sunk), G = soot.

use std::collections::{HashMap, HashSet};

use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::camera::primitives::Aabb;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use serde_json::{Value, json};

use super::layer::{TownsState, TownsWorld};
use super::soot::{SootConfig, SootInput, TownSoot};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::CampaignCamera;
use crate::campaign::terrain::relief_gpu::{ReliefGpu, ReliefParams};
use crate::campaign::terrain::surface::Surface;
use crate::campaign::water::drape::{Drape, add_drape, with_relief};
use crate::paths;
use crate::sim::Sim;

pub mod builder;

use builder::{FarMesh, Ground};

const FAR_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e7a01");
const MASK_SIDE: u32 = 64;
const CAPACITY: usize = (MASK_SIDE * MASK_SIDE) as usize;

/// Settings of the far layer (`town_far_layer.gd` exports).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FarProfile {
    pub f1_tile: f32,
    pub f2_tile: f32,
    /// End of F1 (camera distance, units).
    pub f1_range: f32,
    pub sink_depth_m: f32,
    /// Tiles turned into meshes per frame.
    pub builds_per_frame: usize,
}

impl Default for FarProfile {
    fn default() -> Self {
        FarProfile {
            f1_tile: 128.0,
            f2_tile: 512.0,
            f1_range: 300.0,
            sink_depth_m: 30.0,
            builds_per_frame: 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct FarParams {
    /// Metres per unit, layer weight, sink depth (m), soot on roofs (max).
    pub a: Vec4,
    /// Soot colour (linear rgb), soot on walls (max).
    pub soot: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct TownFarExt {
    #[uniform(100)]
    pub params: FarParams,
    #[uniform(101)]
    pub relief: ReliefParams,
    #[texture(102, sample_type = "float", filterable = false)]
    pub relief_floor: Handle<Image>,
    #[texture(103, sample_type = "float", filterable = false)]
    pub mask: Handle<Image>,
}

impl MaterialExtension for TownFarExt {
    fn vertex_shader() -> ShaderRef {
        FAR_SHADER.into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}

pub type TownFarMaterial = ExtendedMaterial<StandardMaterial, TownFarExt>;

impl Drape for TownFarMaterial {
    fn set_relief(&mut self, params: ReliefParams, floor: &Handle<Image>) {
        self.extension.relief = params;
        self.extension.relief_floor = floor.clone();
    }
    fn relief(&self) -> (ReliefParams, &Handle<Image>) {
        (self.extension.relief, &self.extension.relief_floor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    F1,
    F2,
}

struct Tile {
    level: Level,
    center: Vec2,
    towns: Vec<u32>,
    entity: Option<Entity>,
    triangles: usize,
    visible: bool,
}

/// State of the far layer.
#[derive(Resource)]
pub struct FarWorld {
    profile: FarProfile,
    ids: Vec<String>,
    index_of: HashMap<String, u32>,
    tiles: Vec<Tile>,
    tasks: Vec<(usize, Task<FarMesh>)>,
    material: Handle<TownFarMaterial>,
    mask: Handle<Image>,
    /// Mask bytes (R built, G soot).
    mask_data: Vec<u8>,
    mask_uploads: usize,
    soot: TownSoot,
    soot_turn: i64,
    forced_soot: Vec<(String, f32)>,
    gen_start: f32,
    gen_ms: f32,
    covered: usize,
}

/// `CENT_ANS_TOWN_FAR=0` disables the layer.
#[derive(Resource, Clone, Copy)]
struct FarEnabled(bool);

pub fn add(app: &mut App) {
    // Godot draws the far meshes in the `real` style only (`SettlementLayer._setup_towns`).
    let real = app
        .world()
        .get_resource::<super::style::TownStyle>()
        .is_some_and(|s| s.real());
    app.insert_resource(FarEnabled(
        real && std::env::var("CENT_ANS_TOWN_FAR").map_or(true, |v| v != "0"),
    ))
    .add_plugins(MaterialPlugin::<TownFarMaterial>::default())
    .register_dump(dump_far)
    .add_systems(OnExit(AppState::Campaign), reset_far)
    .add_systems(
        Update,
        (start_far, collect_far, view_far, soot_far, upload_mask)
            .chain()
            .after(super::layer::stream_towns)
            .run_if(in_state(AppState::Campaign))
            .run_if(resource_exists::<TownsWorld>),
    );
    add_drape::<TownFarMaterial>(app);
    if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() {
        let source = with_relief(include_str!("town_far.wesl"));
        if let Err(error) = shaders.insert(
            FAR_SHADER.id(),
            Shader::from_wesl(source, "towns/far/town_far.wesl"),
        ) {
            error!("towns: far shader: {error:?}");
        }
    }
}

fn reset_far(mut commands: Commands) {
    commands.remove_resource::<FarWorld>();
}

fn mask_image(data: &[u8]) -> Image {
    Image::new(
        Extent3d {
            width: MASK_SIDE,
            height: MASK_SIDE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data.to_vec(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

fn parse_forced_soot(text: &str) -> Vec<(String, f32)> {
    text.split(',')
        .filter_map(|p| {
            let (id, v) = p.split_once(':')?;
            Some((id.trim().to_owned(), v.trim().parse().ok()?))
        })
        .collect()
}

/// Groups the towns by tile of `side` units (keys sorted: stable order).
fn tiles_of(px: &[(u32, Vec2)], side: f32, level: Level) -> Vec<Tile> {
    let mut by: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
    for (i, p) in px {
        by.entry(((p.x / side).floor() as i32, (p.y / side).floor() as i32))
            .or_default()
            .push(*i);
    }
    let mut keys: Vec<_> = by.keys().copied().collect();
    keys.sort();
    keys.into_iter()
        .map(|k| Tile {
            level,
            center: Vec2::new((k.0 as f32 + 0.5) * side, (k.1 as f32 + 0.5) * side),
            towns: by.remove(&k).unwrap_or_default(),
            entity: None,
            triangles: 0,
            visible: false,
        })
        .collect()
}

/// Plans the tiles, samples the grounds and starts the generation tasks (once the terrain is
/// there).
fn start_far(
    mut commands: Commands,
    enabled: Res<FarEnabled>,
    world: Res<TownsWorld>,
    far: Option<Res<FarWorld>>,
    surface: Option<Res<Surface>>,
    gpu: Option<Res<ReliefGpu>>,
    time: Res<Time>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<TownFarMaterial>>,
) {
    if !enabled.0 || far.is_some() {
        return;
    }
    let (Some(surface), Some(gpu)) = (surface, gpu) else {
        return;
    };
    let profile = FarProfile::default();
    let db = world.db.clone();
    let mut ids: Vec<String> = db
        .towns
        .keys()
        .filter(|id| !world.skip.contains(*id))
        .cloned()
        .collect();
    ids.sort();
    ids.truncate(CAPACITY);
    let index_of: HashMap<String, u32> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i as u32))
        .collect();
    let mpu = db.meters_per_unit;
    // Grounds sampled on the main thread (polar grid, resident relief).
    let mut grounds: Vec<Ground> = Vec::with_capacity(ids.len());
    let mut px = Vec::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
        let town = &db.towns[id];
        let radii = builder::radii_of(town);
        let anchor = town.px;
        grounds.push(Ground::sample(&radii, |l| {
            let w = anchor + l / mpu;
            surface.height_m_at(w.x, w.y).max(0.0)
        }));
        px.push((i as u32, town.px));
    }
    let mut tiles = tiles_of(&px, profile.f1_tile, Level::F1);
    tiles.extend(tiles_of(&px, profile.f2_tile, Level::F2));
    let grounds = std::sync::Arc::new(grounds);
    let ids_arc = std::sync::Arc::new(ids.clone());
    let pool = AsyncComputeTaskPool::get();
    let mut tasks = Vec::with_capacity(tiles.len());
    for (t, tile) in tiles.iter().enumerate() {
        let (db, grounds, ids, towns, level) = (
            db.clone(),
            grounds.clone(),
            ids_arc.clone(),
            tile.towns.clone(),
            tile.level,
        );
        tasks.push((
            t,
            pool.spawn(async move {
                let mut out = FarMesh::default();
                for i in towns {
                    let town = &db.towns[&ids[i as usize]];
                    let ground = &grounds[i as usize];
                    let part = match level {
                        Level::F1 => {
                            let spec = builder::wall_params(&town.walls, db.walls.get(&town.walls));
                            builder::build_f1(town, i, mpu, ground, &spec)
                        }
                        Level::F2 => builder::build_f2(town, i, mpu, ground),
                    };
                    out.append(&part);
                }
                out
            }),
        ));
    }
    let mask_data = vec![0u8; CAPACITY * 4];
    let mask = images.add(mask_image(&mask_data));
    let soot_config = paths::find_root()
        .map(|r| SootConfig::load(&paths::data_dir(&r)))
        .unwrap_or_default();
    let srgb = |c: f32| Color::srgb(c, c, c).to_linear().red;
    let material = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        },
        extension: TownFarExt {
            params: FarParams {
                a: Vec4::new(mpu, 1.0, profile.sink_depth_m, 0.6),
                soot: Vec4::new(srgb(0.11), srgb(0.075), srgb(0.055), 0.15),
            },
            relief: gpu.params,
            relief_floor: gpu.floor.clone(),
            mask: mask.clone(),
        },
    });
    commands.insert_resource(FarWorld {
        profile,
        ids,
        index_of,
        tiles,
        tasks,
        material,
        mask,
        mask_data,
        mask_uploads: 1,
        soot: TownSoot::new(soot_config),
        soot_turn: -1,
        forced_soot: std::env::var("CENT_ANS_TOWN_SOOT")
            .map(|t| parse_forced_soot(&t))
            .unwrap_or_default(),
        gen_start: time.elapsed_secs(),
        gen_ms: 0.0,
        covered: 0,
    });
}

fn to_mesh(m: FarMesh) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, m.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, m.normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, m.colors);
    // StandardMaterial's fragment reads `uv` whenever `uv_b` exists: a zero UV0.
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; m.uv1.len()]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, m.uv1);
    mesh.insert_indices(Indices::U32(m.indices));
    mesh
}

/// Turns finished tasks into meshes (a few per frame).
fn collect_far(
    mut commands: Commands,
    time: Res<Time>,
    far: Option<ResMut<FarWorld>>,
    mut state: ResMut<TownsState>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(mut far) = far else { return };
    if far.tasks.is_empty() {
        return;
    }
    let mut built = 0;
    let mut k = 0;
    while k < far.tasks.len() && built < far.profile.builds_per_frame {
        let Some(mesh) = block_on(poll_once(&mut far.tasks[k].1)) else {
            k += 1;
            continue;
        };
        let (t, _) = far.tasks.swap_remove(k);
        built += 1;
        let tris = mesh.triangles();
        let tile_level = far.tiles[t].level;
        if tris > 0 {
            // Positions carry metres in y: generous box (displayed heights stay well inside).
            let (lo, hi) = mesh.positions.iter().fold(
                (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
                |(lo, hi), p| {
                    let q = Vec2::new(p[0], p[2]);
                    (lo.min(q), hi.max(q))
                },
            );
            let aabb =
                Aabb::from_min_max(Vec3::new(lo.x, -50.0, lo.y), Vec3::new(hi.x, 500.0, hi.y));
            let handle = meshes.add(to_mesh(mesh));
            let entity = commands
                .spawn((
                    Name::new(format!(
                        "TownFar_{}_{t}",
                        if tile_level == Level::F1 { "f1" } else { "f2" }
                    )),
                    Mesh3d(handle),
                    MeshMaterial3d(far.material.clone()),
                    Transform::IDENTITY,
                    aabb,
                    Visibility::Hidden,
                    NotShadowCaster,
                    DespawnOnExit(AppState::Campaign),
                ))
                .id();
            far.tiles[t].entity = Some(entity);
        }
        far.tiles[t].triangles = tris;
    }
    if far.tasks.is_empty() {
        far.gen_ms = (time.elapsed_secs() - far.gen_start) * 1000.0;
        // Every town has its far mesh: the maquettes of the markers lot can sink.
        let ids = far.ids.clone();
        far.covered = ids.len();
        state.set_far(ids);
    }
}

/// F1 / F2 by camera distance, layer weight, mask of the built 1:1 towns.
fn view_far(
    far: Option<ResMut<FarWorld>>,
    state: Res<TownsState>,
    camera: Query<&GlobalTransform, With<CampaignCamera>>,
    mut materials: ResMut<Assets<TownFarMaterial>>,
    mut vis: Query<&mut Visibility>,
) {
    let Some(mut far) = far else { return };
    let eye = camera.iter().next().map(|t| t.translation());
    let active = state.active && state.weight > 0.001;
    let f1 = far.profile.f1_range;
    let mut tiles = std::mem::take(&mut far.tiles);
    for tile in &mut tiles {
        let want = active
            && eye.is_some_and(|e| {
                let d = Vec3::new(tile.center.x - e.x, e.y, tile.center.y - e.z).length();
                match tile.level {
                    Level::F1 => d < f1,
                    Level::F2 => d >= f1,
                }
            });
        if want != tile.visible
            && let Some(e) = tile.entity
            && let Ok(mut v) = vis.get_mut(e)
        {
            *v = if want {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            tile.visible = want;
        }
    }
    far.tiles = tiles;
    if let Some(m) = materials.get(&far.material)
        && (m.extension.params.a.y - state.weight).abs() > 1e-3
        && let Some(mut m) = materials.get_mut(&far.material)
    {
        m.extension.params.a.y = state.weight;
    }
    // R: the 1:1 town is built.
    let built: HashSet<u32> = state
        .shown_ids()
        .into_iter()
        .filter_map(|id| far.index_of.get(id).copied())
        .collect();
    for i in 0..far.ids.len() {
        far.mask_data[i * 4] = if built.contains(&(i as u32)) { 255 } else { 0 };
    }
}

/// Soot from the simulation, re-read when the turn changes (or the sim state changes).
fn soot_far(far: Option<ResMut<FarWorld>>, sim: Res<Sim>, world: Res<TownsWorld>) {
    let Some(mut far) = far else { return };
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    let turn = i64::from(state.turn);
    if turn == far.soot_turn && !sim.is_changed() {
        return;
    }
    far.soot_turn = turn;
    let mut inputs = Vec::with_capacity(far.ids.len());
    for id in &far.ids {
        let Ok(sid) = data_model::SettlementId::new(id) else {
            continue;
        };
        let (Some(def), Some(live)) = (data.settlements.get(&sid), state.settlement_state(&sid))
        else {
            continue;
        };
        let devastation = state
            .province_state(&def.province)
            .map_or(0.0, |p| f32::from(p.devastation));
        inputs.push(SootInput {
            id: id.clone(),
            province: def.province.to_string(),
            controller: live.controller.to_string(),
            kind: world
                .db
                .towns
                .get(id)
                .map_or_else(String::new, |t| t.kind.clone()),
            devastation,
            besieged: live.siege.is_some(),
        });
    }
    let forced = std::mem::take(&mut far.forced_soot);
    for (id, v) in &forced {
        far.soot.force(id, *v, turn);
    }
    far.soot.update(&inputs, turn);
    for i in 0..far.ids.len() {
        let v = far.soot.amount_of(&far.ids[i]);
        far.mask_data[i * 4 + 1] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

fn upload_mask(far: Option<ResMut<FarWorld>>, mut images: ResMut<Assets<Image>>) {
    let Some(mut far) = far else { return };
    let changed = images
        .get(&far.mask)
        .is_some_and(|img| img.data.as_deref() != Some(far.mask_data.as_slice()));
    if changed && let Some(mut img) = images.get_mut(&far.mask) {
        img.data = Some(far.mask_data.clone());
        far.mask_uploads += 1;
    }
}

fn dump_far(world: &World) -> (String, Value) {
    let Some(far) = world.get_resource::<FarWorld>() else {
        return ("campaign_town_far".into(), json!({"ready": false}));
    };
    let mut bands = json!({});
    for (level, name) in [(Level::F1, "f1"), (Level::F2, "f2")] {
        let tiles: Vec<&Tile> = far.tiles.iter().filter(|t| t.level == level).collect();
        bands[name] = json!({
            "tiles": tiles.len(),
            "built": tiles.iter().filter(|t| t.entity.is_some()).count(),
            "visible": tiles.iter().filter(|t| t.visible).count(),
            "triangles": tiles.iter().map(|t| t.triangles).sum::<usize>(),
            "visible_towns": tiles.iter().filter(|t| t.visible).map(|t| t.towns.len()).sum::<usize>(),
            "visible_triangles": tiles.iter().filter(|t| t.visible).map(|t| t.triangles).sum::<usize>(),
        });
    }
    let built_1to1 = far
        .mask_data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 127)
        .count();
    let state = world.get_resource::<TownsState>();
    (
        "campaign_town_far".into(),
        json!({
            "ready": true,
            "towns": far.ids.len(),
            "covered": far.covered,
            "pending_tasks": far.tasks.len(),
            "generation_ms": far.gen_ms,
            "bands": bands,
            "f1_range": far.profile.f1_range,
            "mask_built": built_1to1,
            "mask_uploads": far.mask_uploads,
            "sooty_towns": far.soot.sooty(),
            "soot_turn": far.soot_turn,
            "layer_active": state.is_some_and(|s| s.active),
            "layer_weight": state.map_or(0.0, |s| s.weight),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_group_and_forced_soot_parse() {
        let px = vec![
            (0, Vec2::new(10.0, 10.0)),
            (1, Vec2::new(20.0, 100.0)),
            (2, Vec2::new(300.0, 10.0)),
        ];
        let f1 = tiles_of(&px, 128.0, Level::F1);
        assert_eq!(f1.len(), 2);
        assert_eq!(f1.iter().map(|t| t.towns.len()).sum::<usize>(), 3);
        assert_eq!(tiles_of(&px, 512.0, Level::F2).len(), 1);
        assert_eq!(
            parse_forced_soot("set_a:0.8, set_b:0.3"),
            vec![("set_a".into(), 0.8), ("set_b".into(), 0.3)]
        );
    }
}
