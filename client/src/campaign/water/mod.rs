//! Sea, coast, lakes, rivers, roads and river names of the campaign map (ADR 0357).
//!
//! Replaces game/scripts/map/: `sea.gd`, `sea_basins.gd`, `coast_renderer.gd`, `coast_look.gd`
//! (coast line and foam band only), `lakes_renderer.gd`, `rivers_renderer.gd`,
//! `river_crossings.gd` (placeholder bridge decks), `road_renderer.gd`, `river_labels.gd`
//! (placement only), `polyline_mesh.gd`; shaders `water`, `river_water`, `terrain_line`,
//! `road_line`, `road`.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.
//!
//! World convention (as in Godot): `x` = map pixel x, `z` = map pixel y, `y` = metres x 0.006.
//! The sea is at `y = 0`; lakes at their level; rivers, roads and coast hug the ground given by
//! [`GroundHeight`] (flat by default: another lot plugs the terrain sampler).
//!
//! Not ported yet (see the lot report): `freshwater_layer.gd` rendering (ponds, reeds, torrents,
//! waterfalls: only counted), `sea_lane_layer.gd`, carved river bed and fords, Nano Banana water
//! detail textures, parchment-view blends, 3D text of river names, `coast_look` swash/geology.

pub mod basins;
pub mod data;
pub mod drape;
pub mod height;
pub mod label_ui;
pub mod labels;
pub mod materials;
pub mod polygon;
pub mod ribbon;
pub mod sea_life;

use std::path::Path;
use std::time::Instant;

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::paths;

use self::data::WaterData;
use self::height::GroundHeight;
use self::materials::{
    LineMaterial, RiverMaterial, RiverParams, RoadRibbonMaterial, SeaMaterial, SeaParams,
};
use self::ribbon::{RibbonBuffers, RibbonLine};

/// The sea extends 4 maps beyond each border (`Sea.margin_factor`).
const SEA_MARGIN_FACTOR: f32 = 4.0;
/// Opaque floor under the transparent sea (`Sea.abyss_depth`).
const ABYSS_DEPTH: f32 = -4.5;
/// Width above which a river is major whatever its importance (`RiversRenderer.MAJOR_WIDTH`).
const MAJOR_WIDTH: f32 = 0.5;
const MAJOR_IMPORTANCE: i64 = 3;
/// Sample step (map px) of the dirt-road ribbons; the ground is smooth enough at this step.
const RIBBON_STEP: f32 = 1.0;
const RIBBON_MAIN_WIDTH: f32 = 0.55;
const RIBBON_WIDTH: f32 = 0.38;
const MAIN_ROAD_WIDTH: f32 = 0.5;
/// Lake outline growth under the shore (`LakesRenderer.shore_overlap_px`).
const LAKE_SHORE_OVERLAP: f32 = 3.0;

/// Which part of the water lot an entity belongs to (dump counters).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaterPart {
    Sea,
    Abyss,
    Coast,
    Lakes,
    MajorRivers,
    MinorRivers,
    MainRoads,
    MinorRoads,
    RoadRibbons,
    Bridge,
}

/// View weights driven by the camera lot (`update_view` of the Godot renderers).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WaterView {
    /// Weight of the medium tier (road lines).
    pub medium: f32,
    /// Weight of the near tier (road ribbons).
    pub near: f32,
    /// Parchment weight (`ZoomTiers.strategic_weight`): lakes hide above 0.5.
    pub strategic_weight: f32,
    /// Camera distance in world units (minor rivers hide beyond `minor_max_distance`).
    pub camera_distance: f32,
}

impl Default for WaterView {
    fn default() -> Self {
        Self {
            medium: 1.0,
            near: 0.0,
            strategic_weight: 0.0,
            camera_distance: 0.0,
        }
    }
}

/// Ask the plugin to rebuild every mesh (e.g. after the terrain replaced [`GroundHeight`]).
#[derive(Message, Debug, Clone, Copy)]
pub struct RebuildWater;

/// Counters and samples of the last build, exposed as the `campaign_water` dump section.
#[derive(Resource, Debug, Clone, Default)]
pub struct WaterStats {
    pub built: bool,
    pub build_ms: u128,
    pub map_size: Vec2,
    pub sea_vertices: usize,
    pub sea_extent: Vec2,
    pub sea_basins: Vec<String>,
    pub coast_lines: usize,
    pub coast_vertices: usize,
    pub lakes: usize,
    pub lakes_meshed: usize,
    pub lake_vertices: usize,
    pub lake_triangles: usize,
    pub lake_samples: Vec<(String, f32, usize)>,
    pub rivers: usize,
    pub major_pieces: usize,
    pub minor_pieces: usize,
    pub river_vertices: usize,
    pub river_triangles: usize,
    pub roads: usize,
    pub main_roads: usize,
    pub road_line_vertices: usize,
    pub road_ribbon_vertices: usize,
    pub road_ribbon_triangles: usize,
    pub crossings: usize,
    pub bridges: usize,
    pub labels: Vec<labels::RiverLabel>,
    pub freshwater: Vec<(String, usize)>,
    pub has_coast_dist: bool,
    /// `data/fx/sea_life.json` applied to the sea (shallows, crests, tidal flats).
    pub sea_life: bool,
}

pub struct CampaignWaterPlugin;

impl Plugin for CampaignWaterPlugin {
    fn build(&self, app: &mut App) {
        materials::register_shaders(app);
        label_ui::build(app);
        app.init_resource::<GroundHeight>()
            .init_resource::<WaterView>()
            .init_resource::<WaterStats>()
            .add_message::<RebuildWater>()
            .register_dump(dump_water);
        if app.world().contains_resource::<Assets<Mesh>>() {
            app.add_plugins((
                MaterialPlugin::<SeaMaterial>::default(),
                MaterialPlugin::<RiverMaterial>::default(),
                MaterialPlugin::<LineMaterial>::default(),
                MaterialPlugin::<RoadRibbonMaterial>::default(),
            ))
            .add_systems(
                PostUpdate,
                (
                    drape::sync_drape::<RiverMaterial>,
                    drape::sync_drape::<LineMaterial>,
                    drape::sync_drape::<RoadRibbonMaterial>,
                ),
            )
            .add_systems(OnEnter(AppState::Campaign), (build_water, debug_camera))
            .add_systems(
                Update,
                (rebuild_water, apply_view)
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
        }
    }
}

/// Dev aid (`CENT_ANS_WATER_DEBUG_CAMERA=x,y,height`, map pixels): a camera above the map so the
/// shaders compile and render when the camera lot is not merged yet. Off by default.
fn debug_camera(mut commands: Commands) {
    let Ok(value) = std::env::var("CENT_ANS_WATER_DEBUG_CAMERA") else {
        return;
    };
    let parts: Vec<f32> = value
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    let [x, z, height] = parts[..] else { return };
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(x, height, z + height * 0.6).looking_at(Vec3::new(x, 0.0, z), Vec3::Y),
        DespawnOnExit(AppState::Campaign),
    ));
}

/// Mutable asset stores and the command queue handed to the builders.
struct Ctx<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    sea_materials: &'a mut Assets<SeaMaterial>,
    river_materials: &'a mut Assets<RiverMaterial>,
    line_materials: &'a mut Assets<LineMaterial>,
    ribbon_materials: &'a mut Assets<RoadRibbonMaterial>,
    standard_materials: &'a mut Assets<StandardMaterial>,
    images: &'a mut Assets<Image>,
}

impl Ctx<'_, '_, '_> {
    /// Spawns a mesh with a material under a [`WaterPart`] tag; shadows off, screen-scoped.
    fn spawn<M: Material>(
        &mut self,
        part: WaterPart,
        name: &str,
        mesh: Mesh,
        material: Handle<M>,
        transform: Transform,
    ) -> Entity {
        let handle = self.meshes.add(mesh);
        self.commands
            .spawn((
                Name::new(name.to_string()),
                part,
                Mesh3d(handle),
                MeshMaterial3d(material),
                transform,
                NotShadowCaster,
                DespawnOnExit(AppState::Campaign),
            ))
            .id()
    }
}

/// Reads a PNG as a GPU-only image (`None` if missing or unreadable).
fn load_png(path: &Path, srgb: bool) -> Option<Image> {
    let bytes = std::fs::read(path).ok()?;
    match Image::from_buffer(
        &bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        srgb,
        ImageSampler::linear(),
        RenderAssetUsages::RENDER_WORLD,
    ) {
        Ok(image) => Some(image),
        Err(error) => {
            warn!("water: {}: {error}", path.display());
            None
        }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct BuildAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    sea_materials: ResMut<'w, Assets<SeaMaterial>>,
    river_materials: ResMut<'w, Assets<RiverMaterial>>,
    line_materials: ResMut<'w, Assets<LineMaterial>>,
    ribbon_materials: ResMut<'w, Assets<RoadRibbonMaterial>>,
    standard_materials: ResMut<'w, Assets<StandardMaterial>>,
    images: ResMut<'w, Assets<Image>>,
}

fn build_water(
    mut commands: Commands,
    mut assets: BuildAssets,
    ground: Res<GroundHeight>,
    mut stats: ResMut<WaterStats>,
) {
    let Ok(root) = paths::find_root() else {
        warn!("water: repository root not found");
        return;
    };
    let started = Instant::now();
    let map_dir = paths::data_dir(&root).join("map");
    let data = WaterData::load(&map_dir);
    let mut ctx = Ctx {
        commands: &mut commands,
        meshes: &mut assets.meshes,
        sea_materials: &mut assets.sea_materials,
        river_materials: &mut assets.river_materials,
        line_materials: &mut assets.line_materials,
        ribbon_materials: &mut assets.ribbon_materials,
        standard_materials: &mut assets.standard_materials,
        images: &mut assets.images,
    };
    let mut out = WaterStats {
        built: true,
        map_size: data.map_size,
        ..default()
    };
    let coast_dist = load_png(&map_dir.join("coast_dist.png"), false).map(|i| ctx.images.add(i));
    out.has_coast_dist = coast_dist.is_some();
    build_sea(&mut ctx, &data, coast_dist.clone(), &mut out);
    build_coast(&mut ctx, &data, ground.0.as_ref(), &mut out);
    build_lakes(&mut ctx, &data, coast_dist, &mut out);
    build_rivers(&mut ctx, &data, ground.0.as_ref(), &mut out);
    build_roads(&mut ctx, &data, ground.0.as_ref(), &mut out);
    build_crossings(&mut ctx, &data, ground.0.as_ref(), &mut out);
    out.labels = labels::place(&data);
    out.freshwater = {
        let mut counts: Vec<_> = data
            .freshwater_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        counts.sort();
        counts
    };
    out.build_ms = started.elapsed().as_millis();
    info!(
        "water: built in {} ms ({} river verts, {} road ribbon verts, {} lakes, {} labels)",
        out.build_ms,
        out.river_vertices,
        out.road_ribbon_vertices,
        out.lakes_meshed,
        out.labels.len()
    );
    *stats = out;
}

fn rebuild_water(
    mut requests: MessageReader<RebuildWater>,
    parts: Query<Entity, With<WaterPart>>,
    mut commands: Commands,
    mut pending: Local<bool>,
) {
    // Two steps: despawn this frame, rebuild next frame (commands are deferred).
    if *pending {
        *pending = false;
        commands.run_system_cached(build_water);
        return;
    }
    if requests.read().count() > 0 {
        for entity in &parts {
            commands.entity(entity).despawn();
        }
        *pending = true;
    }
}

type PartQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static WaterPart,
        &'static mut Visibility,
        Option<&'static MeshMaterial3d<LineMaterial>>,
        Option<&'static MeshMaterial3d<RoadRibbonMaterial>>,
    ),
>;

/// Applies the view weights to materials and visibility.
fn apply_view(
    view: Res<WaterView>,
    stats: Res<WaterStats>,
    mut line_materials: ResMut<Assets<LineMaterial>>,
    mut ribbon_materials: ResMut<Assets<RoadRibbonMaterial>>,
    mut parts: PartQuery,
) {
    if !view.is_changed() || !stats.built {
        return;
    }
    for (part, mut visibility, line, ribbon) in &mut parts {
        let show = match part {
            WaterPart::MainRoads | WaterPart::MinorRoads => view.medium > 0.01,
            WaterPart::RoadRibbons => view.near > 0.01,
            WaterPart::Lakes => view.strategic_weight < 0.5,
            _ => true,
        };
        *visibility = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(handle) = line
            && let Some(mut material) = line_materials.get_mut(&handle.0)
            && matches!(part, WaterPart::MainRoads | WaterPart::MinorRoads)
        {
            material.params.alpha = view.medium;
        }
        if let Some(handle) = ribbon
            && let Some(mut material) = ribbon_materials.get_mut(&handle.0)
        {
            material.params.alpha = view.near;
        }
    }
}

// --- Sea ---------------------------------------------------------------------------------

fn build_sea(
    ctx: &mut Ctx,
    data: &WaterData,
    coast_dist: Option<Handle<Image>>,
    out: &mut WaterStats,
) {
    let extent = data.map_size * (1.0 + SEA_MARGIN_FACTOR * 2.0);
    let centre = Vec3::new(data.map_size.x * 0.5, 0.0, data.map_size.y * 0.5);
    let plane = Mesh::from(Plane3d::default().mesh().size(extent.x, extent.y));
    out.sea_vertices = plane.count_vertices();
    out.sea_extent = extent;
    out.sea_basins = data.basins.basins.iter().map(|b| b.id.clone()).collect();

    let weights = if data.basins.basins.is_empty() {
        None
    } else {
        let (w, h, bytes) = basins::weight_grid(&data.basins, data.map_size);
        let mut image = Image::new(
            Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            bytes,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::linear();
        Some(ctx.images.add(image))
    };
    let mut params = SeaParams::new(data.map_size, &data.basins);
    let life = crate::paths::find_root()
        .ok()
        .and_then(|root| sea_life::SeaLife::load(&root));
    let tide_zones = life.as_ref().map(|life| {
        life.apply(&mut params);
        ctx.images.add(life.tide_image(data.map_size))
    });
    out.sea_life = life.is_some();
    let material = ctx.sea_materials.add(SeaMaterial {
        params,
        coast_dist,
        basin_weights: weights,
        tide_zones,
    });
    ctx.spawn(
        WaterPart::Sea,
        "Sea",
        plane,
        material,
        Transform::from_translation(centre),
    );

    let floor = Mesh::from(Plane3d::default().mesh().size(extent.x, extent.y));
    let floor_material = ctx.standard_materials.add(StandardMaterial {
        base_color: Color::srgb(0.03, 0.10, 0.15),
        perceptual_roughness: 0.92,
        ..default()
    });
    ctx.spawn(
        WaterPart::Abyss,
        "Abyss",
        floor,
        floor_material,
        Transform::from_translation(Vec3::new(centre.x, ABYSS_DEPTH, centre.z)),
    );
}

// --- Coast -------------------------------------------------------------------------------

fn build_coast(
    ctx: &mut Ctx,
    data: &WaterData,
    ground: &dyn height::HeightSampler,
    out: &mut WaterStats,
) {
    let widths: Vec<Vec<f32>> = data.coastlines.iter().map(|l| vec![0.0; l.len()]).collect();
    let lines: Vec<RibbonLine> = data
        .coastlines
        .iter()
        .zip(&widths)
        .map(|(points, widths)| RibbonLine {
            points,
            widths,
            importance: 0.0,
        })
        .collect();
    let buffers = ribbon::screen_ribbon(&lines, ground);
    out.coast_lines = data.coastlines.len();
    out.coast_vertices = buffers.positions.len();
    if let Some(mesh) = buffers.into_mesh() {
        let material = ctx.line_materials.add(LineMaterial::coast());
        ctx.spawn(
            WaterPart::Coast,
            "Coast",
            mesh,
            material,
            Transform::IDENTITY,
        );
    }
}

// --- Lakes -------------------------------------------------------------------------------

/// Triangulated lake sheets at their level (`LakesRenderer.build`); vertex attributes follow the
/// river layout (normal up, zero UVs) so one shader serves both.
fn lake_mesh(lakes: &[data::Lake], out: &mut WaterStats) -> Option<Mesh> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for lake in lakes {
        let mut outline = polygon::offset(&lake.polygon, LAKE_SHORE_OVERLAP);
        let mut triangles = polygon::triangulate(&outline);
        if triangles.is_empty() {
            outline = lake.polygon.clone();
            triangles = polygon::triangulate(&outline);
        }
        if triangles.is_empty() {
            continue;
        }
        let base = positions.len() as u32;
        let y = lake.level_m;
        positions.extend(outline.iter().map(|p| [p.x, y, p.y]));
        indices.extend(triangles.iter().map(|i| i + base));
        out.lakes_meshed += 1;
        if out.lake_samples.len() < 5 {
            out.lake_samples
                .push((lake.name.clone(), lake.level_m, triangles.len() / 3));
        }
    }
    out.lake_vertices = positions.len();
    out.lake_triangles = indices.len() / 3;
    if positions.is_empty() {
        return None;
    }
    let count = positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, vec![[0.0, 0.0]; count]);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

fn build_lakes(
    ctx: &mut Ctx,
    data: &WaterData,
    coast_dist: Option<Handle<Image>>,
    out: &mut WaterStats,
) {
    out.lakes = data.lakes.len();
    let Some(mesh) = lake_mesh(&data.lakes, out) else {
        return;
    };
    let mut params = RiverParams::lake(data.map_size);
    params.has_coast_dist = if coast_dist.is_some() { 1.0 } else { 0.0 };
    let material = ctx.river_materials.add(RiverMaterial {
        params,
        coast_dist,
        relief: Default::default(),
        relief_floor: Handle::default(),
    });
    ctx.spawn(
        WaterPart::Lakes,
        "Lakes",
        mesh,
        material,
        Transform::IDENTITY,
    );
}

// --- Rivers ------------------------------------------------------------------------------

fn build_rivers(
    ctx: &mut Ctx,
    data: &WaterData,
    ground: &dyn height::HeightSampler,
    out: &mut WaterStats,
) {
    out.rivers = data.rivers.len();
    let extent = data.map_size.x.max(data.map_size.y);
    let display = &data.display;
    let mut major: Vec<RibbonLine> = Vec::new();
    let mut minor: Vec<RibbonLine> = Vec::new();
    for river in &data.rivers {
        if river.points.len() < 2 {
            continue;
        }
        let widest = river.widths.iter().copied().fold(0.0, f32::max);
        let line = RibbonLine {
            points: &river.points,
            widths: &river.widths,
            importance: river.importance as f32,
        };
        if river.importance >= MAJOR_IMPORTANCE || widest >= MAJOR_WIDTH {
            major.push(line);
        } else {
            minor.push(line);
        }
    }
    out.major_pieces = major.len();
    out.minor_pieces = minor.len();
    for (part, name, lines, min_px, is_minor) in [
        (
            WaterPart::MajorRivers,
            "Rivers",
            &major,
            display.major_min_px,
            false,
        ),
        (
            WaterPart::MinorRivers,
            "MinorRivers",
            &minor,
            display.minor_min_px,
            true,
        ),
    ] {
        let buffers = ribbon::screen_ribbon(lines, ground);
        out.river_vertices += buffers.positions.len();
        out.river_triangles += buffers.triangle_count();
        if let Some(mesh) = buffers.into_mesh() {
            let params = RiverParams::river(display, min_px, data.map_size, extent, is_minor);
            let material = ctx.river_materials.add(RiverMaterial {
                params,
                coast_dist: None,
                relief: Default::default(),
                relief_floor: Handle::default(),
            });
            ctx.spawn(part, name, mesh, material, Transform::IDENTITY);
        }
    }
}

// --- Roads -------------------------------------------------------------------------------

fn build_roads(
    ctx: &mut Ctx,
    data: &WaterData,
    ground: &dyn height::HeightSampler,
    out: &mut WaterStats,
) {
    out.roads = data.roads.len();
    out.main_roads = data.roads.iter().filter(|r| r.main).count();

    for (part, name, main, width, material) in [
        (
            WaterPart::MinorRoads,
            "MinorRoads",
            false,
            0.0,
            LineMaterial::minor_road(),
        ),
        (
            WaterPart::MainRoads,
            "MainRoads",
            true,
            MAIN_ROAD_WIDTH,
            LineMaterial::main_road(),
        ),
    ] {
        let roads: Vec<&data::Road> = data.roads.iter().filter(|r| r.main == main).collect();
        let widths: Vec<Vec<f32>> = roads.iter().map(|r| vec![width; r.points.len()]).collect();
        let lines: Vec<RibbonLine> = roads
            .iter()
            .zip(&widths)
            .map(|(r, w)| RibbonLine {
                points: &r.points,
                widths: w,
                importance: 0.0,
            })
            .collect();
        let buffers = ribbon::screen_ribbon(&lines, ground);
        out.road_line_vertices += buffers.positions.len();
        if let Some(mesh) = buffers.into_mesh() {
            let handle = ctx.line_materials.add(material);
            ctx.spawn(part, name, mesh, handle, Transform::IDENTITY);
        }
    }

    // Near tier: every road as a dirt ribbon draped on the ground (hidden until `WaterView::near`).
    let runs: Vec<(Vec<Vec2>, f32)> = data
        .roads
        .iter()
        .map(|r| {
            (
                r.points.clone(),
                if r.main {
                    RIBBON_MAIN_WIDTH
                } else {
                    RIBBON_WIDTH
                },
            )
        })
        .collect();
    let buffers: RibbonBuffers = ribbon::flat_ribbon(&runs, RIBBON_STEP, ground);
    out.road_ribbon_vertices = buffers.positions.len();
    out.road_ribbon_triangles = buffers.triangle_count();
    if let Some(mesh) = buffers.into_mesh() {
        let mut material = RoadRibbonMaterial::default();
        material.params.alpha = 0.0;
        let handle = ctx.ribbon_materials.add(material);
        let entity = ctx.spawn(
            WaterPart::RoadRibbons,
            "RoadRibbons",
            mesh,
            handle,
            Transform::IDENTITY,
        );
        ctx.commands.entity(entity).insert(Visibility::Hidden);
    }
}

// --- Crossings ---------------------------------------------------------------------------

/// Placeholder bridge decks (stone / wood) at the snapped crossings; fords and ferries are left
/// to the river shader (not ported). `BridgeMeshes` of Godot is another lot.
fn build_crossings(
    ctx: &mut Ctx,
    data: &WaterData,
    ground: &dyn height::HeightSampler,
    out: &mut WaterStats,
) {
    out.crossings = data.crossings.len();
    let stone = ctx.standard_materials.add(StandardMaterial {
        base_color: Color::srgb(0.58, 0.54, 0.47),
        perceptual_roughness: 0.9,
        ..default()
    });
    let wood = ctx.standard_materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.28, 0.18),
        perceptual_roughness: 0.95,
        ..default()
    });
    for crossing in &data.crossings {
        let material = match crossing.structure.as_str() {
            "stone" | "gate" => stone.clone(),
            "wood" => wood.clone(),
            _ => continue,
        };
        // Deck: long axis across the river (the crossing `dir` is along the bridge).
        let length = (crossing.width * 1.4).max(0.3);
        let deck = Mesh::from(Cuboid::new(length, 0.04, 0.16));
        let y = ground.surface_world(crossing.px.x, crossing.px.y) + 0.03;
        let angle = -crossing.dir.y.atan2(crossing.dir.x);
        let handle = ctx.meshes.add(deck);
        ctx.commands.spawn((
            Name::new(crossing.id.clone()),
            WaterPart::Bridge,
            Mesh3d(handle),
            MeshMaterial3d(material),
            Transform::from_xyz(crossing.px.x, y, crossing.px.y)
                .with_rotation(Quat::from_rotation_y(angle)),
            NotShadowCaster,
            DespawnOnExit(AppState::Campaign),
        ));
        out.bridges += 1;
    }
}

// --- Dump --------------------------------------------------------------------------------

fn dump_water(world: &World) -> (String, Value) {
    let stats = world.resource::<WaterStats>();
    let view = world.resource::<WaterView>();
    let mut per_part: std::collections::BTreeMap<String, usize> = Default::default();
    if let Some(mut query) = world.try_query::<&WaterPart>() {
        for part in query.iter(world) {
            *per_part.entry(format!("{part:?}")).or_default() += 1;
        }
    }
    let labels: Vec<Value> = stats
        .labels
        .iter()
        .take(12)
        .map(|l| {
            json!({"text": l.text, "px": [l.px.x, l.px.y], "dir": [l.dir.x, l.dir.y],
                   "importance": l.importance, "visible_to": l.visible_to})
        })
        .collect();
    let lake_samples: Vec<Value> = stats
        .lake_samples
        .iter()
        .map(|(name, level, tris)| json!({"name": name, "level_m": level, "triangles": tris}))
        .collect();
    (
        "campaign_water".into(),
        json!({
            "built": stats.built,
            "build_ms": stats.build_ms,
            "map_size": [stats.map_size.x, stats.map_size.y],
            "entities": per_part,
            "view": {"medium": view.medium, "near": view.near,
                     "strategic_weight": view.strategic_weight, "camera_distance": view.camera_distance},
            "sea": {"vertices": stats.sea_vertices, "extent": [stats.sea_extent.x, stats.sea_extent.y],
                    "basins": stats.sea_basins, "has_coast_dist": stats.has_coast_dist, "sea_life": stats.sea_life, "y": 0.0},
            "coast": {"lines": stats.coast_lines, "vertices": stats.coast_vertices},
            "lakes": {"count": stats.lakes, "meshed": stats.lakes_meshed, "vertices": stats.lake_vertices,
                      "triangles": stats.lake_triangles, "samples": lake_samples},
            "rivers": {"count": stats.rivers, "major_pieces": stats.major_pieces,
                       "minor_pieces": stats.minor_pieces, "vertices": stats.river_vertices,
                       "triangles": stats.river_triangles},
            "roads": {"count": stats.roads, "main": stats.main_roads,
                      "line_vertices": stats.road_line_vertices,
                      "ribbon_vertices": stats.road_ribbon_vertices,
                      "ribbon_triangles": stats.road_ribbon_triangles},
            "crossings": {"snapped": stats.crossings, "bridges": stats.bridges},
            "labels": {"count": stats.labels.len(), "first": labels,
                       "on_screen": world.get_resource::<label_ui::RiverLabelStats>().map(|s| s.shown.clone()),
                       "suppressed": world.get_resource::<label_ui::RiverLabelStats>().map(|s| s.suppressed)},
            "river_legibility": legibility(world),
            "freshwater_sites": stats.freshwater,
        }),
    )
}

/// River ribbons at [`materials::LEGIBILITY_DISTANCE`] units: pixel floors per importance tier and
/// far-fade opacity of each river material (major rivers, then minor ones).
fn legibility(world: &World) -> Value {
    let Some(materials) = world.get_resource::<Assets<RiverMaterial>>() else {
        return Value::Null;
    };
    let rows: Vec<Value> = materials
        .iter()
        .filter(|(_, m)| m.params.sheet < 0.5)
        .map(|(_, m)| {
            let (floors, alpha) = m.params.legibility(materials::LEGIBILITY_DISTANCE);
            json!({"min_px": m.params.min_px, "tier_floor_px": floors,
                   "alpha_at_260": alpha, "thin_fade": m.params.thin_fade,
                   "fade_range": [m.params.distance_fade.x, m.params.distance_fade.y]})
        })
        .collect();
    json!({"distance": materials::LEGIBILITY_DISTANCE, "materials": rows})
}
