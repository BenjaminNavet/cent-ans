//! Battle water: the river and the brooks as real water meshes, the sea of coastal fields, and the
//! foam at the feet of bridge piers (port of game/shaders/battle_water.gdshader, battle_sea.gdshader,
//! battle_foam.gdshader and `_build_river` / `_build_streams` of battle_terrain_mesh.gd,
//! `_build_foam` of battle_bridges.gd).
//!
//! Differences from Godot (the shaders live in `water.wesl`):
//! - the water depth is baked in the mesh (vertex colour red = water level minus ground, per vertex
//!   across the ribbon) instead of being read from the depth buffer, so the shallows, the murk and
//!   the soft bank work without a depth prepass and in offscreen renders; the screen refraction
//!   is dropped (the bed is a fixed pebble tone);
//! - the water is lit by Bevy's PBR (sun, fog, night) instead of being emissive;
//! - the wave normals are procedural noise gradients (no noise texture to bake).
//!
//! The sea replaces the plain plane of `decor::features` (which still counts it in its stats): one
//! quad from the waterline outwards, swell in world coordinates, Fresnel sky reflection fading in
//! the distance, lighter water and a broken foam fringe along the beach.
//!
//! Dump section `battle_water`.

use bevy::asset::RenderAssetUsages;
use bevy::asset::embedded_asset;
use bevy::camera::visibility::VisibilityRange;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use sim_battle::{Battlefield, Flank, Weather};

use super::BattleSession;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::decor::features::{SEA_LEVEL, sea_rect};
use crate::battle::decor::look::DecorLook;

/// How far the river runs beyond the field on both sides (m).
const RIVER_PAD: f32 = 1400.0;
/// Distance between two cross-sections of the river (m).
const RIVER_STEP: f32 = 8.0;
/// Vertices across the river ribbon.
const RIVER_COLUMNS: usize = 9;
/// Vertices across a brook ribbon.
const STREAM_COLUMNS: usize = 5;
/// Water above the bed at the river centre (m), over a ford.
const RIVER_DEPTH: f32 = 0.5;
const FORD_DEPTH: f32 = 0.2;
/// Pier foam: quad size (across, along) and visibility range.
const FOAM_SIZE: Vec2 = Vec2::new(3.2, 9.0);
const FOAM_RANGE: f32 = 900.0;
/// Bay lengths of the bridges (m), as in `battle_bridges.gd`.
const STONE_BAY: f32 = 9.0;
const WOOD_BAY: f32 = 4.0;

/// Which water a material draws (`kind` in the shader).
const KIND_RIVER: f32 = 0.0;
const KIND_SEA: f32 = 1.0;
const KIND_FOAM: f32 = 2.0;

/// Uniform block of `water.wesl` (`WaterParams`).
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct WaterParams {
    /// Shallows colour (linear), turbidity in a.
    pub shallow: Vec4,
    /// Deep colour (linear), clarity (m) in a.
    pub deep: Vec4,
    /// Sky colour reflected (linear), rain ripples 0..1 in a.
    pub sky: Vec4,
    /// x: kind, y: flow speed, z: shore x (sea), w: flank (sea, -1 west, +1 east).
    pub misc: Vec4,
    /// x: ribbon width (m), y: shore colour weight, zw: unused.
    pub misc2: Vec4,
    /// Colour of the shore water of the sea (linear).
    pub shore: Vec4,
}

impl Default for WaterParams {
    fn default() -> Self {
        WaterParams {
            shallow: Vec4::new(0.34, 0.37, 0.28, 0.35).map_rgb_to_linear(),
            deep: Vec4::new(0.05, 0.1, 0.1, 0.9).map_rgb_to_linear(),
            sky: Vec4::new(0.62, 0.72, 0.82, 0.0).map_rgb_to_linear(),
            misc: Vec4::new(KIND_RIVER, 0.55, 0.0, -1.0),
            misc2: Vec4::new(38.0, 0.0, 0.0, 0.0),
            shore: Vec4::new(0.2, 0.33, 0.32, 1.0).map_rgb_to_linear(),
        }
    }
}

/// sRGB triplet (as written in the Godot shaders) to a linear vec4 keeping the alpha channel.
trait MapRgb {
    fn map_rgb_to_linear(self) -> Vec4;
}

impl MapRgb for Vec4 {
    fn map_rgb_to_linear(self) -> Vec4 {
        let linear = Color::srgb(self.x, self.y, self.z).to_linear();
        Vec4::new(linear.red, linear.green, linear.blue, self.w)
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct WaterExtension {
    #[uniform(100)]
    pub params: WaterParams,
}

impl MaterialExtension for WaterExtension {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/scene/water.wesl".into()
    }
}

pub type WaterMaterial = ExtendedMaterial<StandardMaterial, WaterExtension>;

/// What the water of the battle holds (dump).
#[derive(Resource, Debug, Clone, Default)]
pub struct WaterStats {
    pub river: bool,
    pub river_rows: usize,
    pub river_vertices: usize,
    pub river_max_width_m: f32,
    pub river_level_min: f32,
    pub river_level_max: f32,
    pub river_max_depth_m: f32,
    pub streams: usize,
    pub stream_vertices: usize,
    pub sea: bool,
    pub sea_area_m2: f32,
    pub sea_flank: f32,
    pub sea_shore_x: f32,
    pub pier_foam: usize,
    pub rain_ripples: bool,
}

/// Mesh buffers of a water ribbon, built without any rendering.
#[derive(Debug, Default, Clone)]
pub struct RibbonData {
    pub positions: Vec<[f32; 3]>,
    /// x across 0..1, y metres along the course.
    pub uvs: Vec<[f32; 2]>,
    /// Red: water depth at the vertex (m).
    pub depths: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub rows: usize,
    pub max_width: f32,
    pub max_depth: f32,
}

impl RibbonData {
    pub fn into_mesh(self) -> Mesh {
        let normals = vec![[0.0, 1.0, 0.0]; self.positions.len()];
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.depths)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Unit normal (in x, z) of the course at `i`.
fn course_normal(points: &[Vec2], i: usize) -> Vec2 {
    let a = points[i.saturating_sub(1)];
    let b = points[(i + 1).min(points.len() - 1)];
    let dir = (b - a).normalize_or_zero();
    Vec2::new(-dir.y, dir.x)
}

/// Smooths `levels` along the course (`passes` passes of a 1-2-1 kernel); `keep_min` never raises a
/// level above its original value (brooks stay under their banks).
fn smooth_levels(levels: &mut [f32], passes: usize, keep_min: bool) {
    for _ in 0..passes {
        let before = levels.to_vec();
        for i in 1..levels.len().saturating_sub(1) {
            let smooth = (before[i - 1] + before[i] * 2.0 + before[i + 1]) * 0.25;
            levels[i] = if keep_min {
                smooth.min(before[i])
            } else {
                smooth
            };
        }
    }
}

/// Ribbon along `points` at `levels`, `left[i]` / `right[i]` metres to each side of the course.
/// The depth of a vertex is the water level minus `ground` there (0 on the banks).
pub fn build_ribbon(
    points: &[Vec2],
    levels: &[f32],
    left: &[f32],
    right: &[f32],
    columns: usize,
    ground: &impl Fn(f32, f32) -> f32,
) -> RibbonData {
    let mut data = RibbonData::default();
    if points.len() < 2 {
        return data;
    }
    let columns = columns.max(2);
    let mut along = 0.0;
    for i in 0..points.len() {
        let normal = course_normal(points, i);
        if i > 0 {
            along += points[i].distance(points[i - 1]);
        }
        let reach_left = left[i];
        let reach_right = right[i];
        data.max_width = data.max_width.max(reach_left + reach_right);
        for c in 0..columns {
            let u = c as f32 / (columns - 1) as f32;
            let offset = reach_left - u * (reach_left + reach_right);
            let p = points[i] + normal * offset;
            let depth = (levels[i] - ground(p.x, p.y)).max(0.0);
            // The outermost vertices are the bank line: no water there.
            let depth = if c == 0 || c == columns - 1 {
                0.0
            } else {
                depth
            };
            data.max_depth = data.max_depth.max(depth);
            data.positions.push([p.x, levels[i], p.y]);
            data.uvs.push([u, along]);
            data.depths.push([depth, 0.0, 0.0, 1.0]);
        }
        if i > 0 {
            let row = (i * columns) as u32;
            let prev = row - columns as u32;
            for c in 0..(columns as u32 - 1) {
                data.indices.extend_from_slice(&[
                    prev + c,
                    prev + c + 1,
                    row + c,
                    prev + c + 1,
                    row + c + 1,
                    row + c,
                ]);
            }
        }
    }
    data.rows = points.len();
    data
}

/// The river of a field: course points beyond both field edges, water level per point and ribbon.
pub struct RiverGeometry {
    pub ribbon: RibbonData,
    pub points: Vec<Vec2>,
    pub levels: Vec<f32>,
}

impl RiverGeometry {
    /// Water level at `x` (nearest course point).
    pub fn level_at(&self, x: f32) -> f32 {
        if self.points.is_empty() {
            return f32::NEG_INFINITY;
        }
        let first = self.points[0].x;
        let index = ((x - first) / RIVER_STEP)
            .round()
            .clamp(0.0, (self.points.len() - 1) as f32) as usize;
        self.levels[index]
    }
}

/// Builds the river ribbon (`_build_river`): the level sits 0.5 m (0.2 m over a ford) above the
/// bed at the course, smoothed along it; each bank stops where the ground rises above the level.
pub fn river_geometry(field: &Battlefield) -> Option<RiverGeometry> {
    let river = field.river.as_ref()?;
    let width = field.width as f32;
    let count = ((width + 2.0 * RIVER_PAD) / RIVER_STEP).ceil() as usize + 1;
    let ground = |x: f32, z: f32| field.height(f64::from(x), f64::from(z)) as f32;
    let mut points = Vec::with_capacity(count);
    let mut widths = Vec::with_capacity(count);
    for k in 0..count {
        let x = -RIVER_PAD + k as f32 * RIVER_STEP;
        points.push(Vec2::new(x, river.center_z(f64::from(x)) as f32));
        widths.push(river.width_at(f64::from(x)) as f32);
    }
    let mut levels: Vec<f32> = points
        .iter()
        .map(|p| {
            let inside = p.x >= 0.0 && p.x <= width;
            let depth = if inside && river.in_ford(f64::from(p.x)) {
                FORD_DEPTH
            } else {
                RIVER_DEPTH
            };
            ground(p.x, p.y) + depth
        })
        .collect();
    smooth_levels(&mut levels, 8, false);
    let mut left = Vec::with_capacity(count);
    let mut right = Vec::with_capacity(count);
    for i in 0..count {
        let normal = course_normal(&points, i);
        let reach = widths[i] * 0.5 + 8.0;
        let mut extents = [0.0f32; 2];
        for (side, sign) in [1.0f32, -1.0].into_iter().enumerate() {
            let mut d = 1.0f32;
            while d < reach {
                let q = points[i] + normal * sign * d;
                if ground(q.x, q.y) > levels[i] + 0.08 {
                    break;
                }
                d += 1.0;
            }
            extents[side] = d + 2.5;
        }
        left.push(extents[0]);
        right.push(extents[1]);
    }
    let ribbon = build_ribbon(&points, &levels, &left, &right, RIVER_COLUMNS, &ground);
    Some(RiverGeometry {
        ribbon,
        points,
        levels,
    })
}

/// Ribbon of a brook or the tributary (`_build_streams`): resampled every 4 m, level 0.22 m (0.14 m
/// for brooks) above the bed, smoothed and kept under the banks.
pub fn stream_ribbon(stream: &sim_battle::Stream, field: &Battlefield) -> RibbonData {
    let ground = |x: f32, z: f32| field.height(f64::from(x), f64::from(z)) as f32;
    let raw: Vec<Vec2> = stream
        .points
        .iter()
        .map(|&(x, z)| Vec2::new(x as f32, z as f32))
        .collect();
    if raw.len() < 2 {
        return RibbonData::default();
    }
    let mut points = vec![raw[0]];
    for pair in raw.windows(2) {
        let steps = ((pair[0].distance(pair[1]) / 4.0) as usize).max(1);
        for s in 1..=steps {
            points.push(pair[0].lerp(pair[1], s as f32 / steps as f32));
        }
    }
    let depth = match stream.kind {
        sim_battle::StreamKind::Tributary => 0.22,
        sim_battle::StreamKind::Brook => 0.14,
    };
    let mut levels: Vec<f32> = points.iter().map(|p| ground(p.x, p.y) + depth).collect();
    smooth_levels(&mut levels, 6, true);
    let half = stream.width as f32 * 0.5 + 0.8;
    let sides = vec![half; points.len()];
    build_ribbon(&points, &levels, &sides, &sides, STREAM_COLUMNS, &ground)
}

/// One foam quad at the foot of a bridge pier: centre, yaw, level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PierFoam {
    pub position: Vec3,
    /// Rotation about Y turning local +z towards the current.
    pub yaw: f32,
}

/// Foam quads of the bridges of the main river (`battle_bridges.gd`): one per pier, the two
/// abutments included, downstream of the deck.
pub fn pier_foams(field: &Battlefield, level_at: &impl Fn(f32) -> f32) -> Vec<PierFoam> {
    let mut foams = Vec::new();
    for bridge in &field.bridges {
        if bridge.stream.is_some() {
            continue;
        }
        let level = level_at(bridge.x as f32);
        if !level.is_finite() {
            continue;
        }
        let span = (bridge.span as f32).max(2.0);
        let bay = if bridge.stone { STONE_BAY } else { WOOD_BAY };
        let count = ((span / bay).round() as i32).max(1);
        let dir = Vec2::new(bridge.dir.0 as f32, bridge.dir.1 as f32);
        let stream_dir = Vec2::new(dir.y, -dir.x);
        let centre = Vec2::new(bridge.x as f32, bridge.z as f32);
        let shift = bridge.width as f32 * 0.5 + 3.5;
        for k in 0..=count {
            let along = -span * 0.5 + k as f32 * span / count as f32;
            let p = centre + dir * along + stream_dir * shift;
            foams.push(PierFoam {
                position: Vec3::new(p.x, level + 0.04, p.y),
                yaw: stream_dir.x.atan2(stream_dir.y),
            });
        }
    }
    foams
}

/// Quad lying on the water, long axis (uv.y 0..1) downstream along +z.
fn foam_quad() -> Mesh {
    let (hx, hz) = (FOAM_SIZE.x * 0.5, FOAM_SIZE.y * 0.5);
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-hx, 0.0, -hz],
            [hx, 0.0, -hz],
            [hx, 0.0, hz],
            [-hx, 0.0, hz],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0, 0.0, 0.0, 1.0]; 4])
    .with_inserted_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]))
}

/// The sea as one quad (the swell is in the shader, in world coordinates).
fn sea_quad(rect: [f32; 4], y: f32) -> Mesh {
    let [x0, x1, z0, z1] = rect;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0, 0.0, 0.0, 1.0]; 4])
    .with_inserted_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]))
}

/// Parameters of the river water for a weather (`_build_river`): rain muddies and ripples it,
/// snow darkens the deep.
pub fn river_params(weather: Weather, sky: Vec3, flow: f32, width: f32) -> WaterParams {
    let mut params = WaterParams {
        sky: sky.extend(0.0),
        misc: Vec4::new(KIND_RIVER, 0.55 * flow, 0.0, -1.0),
        misc2: Vec4::new(width, 0.0, 0.0, 0.0),
        ..default()
    };
    match weather {
        Weather::Rain => {
            params.shallow.w = 0.75;
            params.sky.w = 1.0;
        }
        Weather::Snow => {
            params.deep = Vec4::new(0.05, 0.09, 0.11, 0.9).map_rgb_to_linear();
        }
        _ => {}
    }
    params
}

fn sky_tint(look: Option<&DecorLook>, weather: Weather) -> Vec3 {
    let Some(look) = look else {
        return Vec3::new(0.62, 0.72, 0.82);
    };
    let sky = look.sky.clone();
    let colors = match weather {
        Weather::Clear => &sky.clear,
        Weather::Fog => &sky.fog,
        Weather::Rain => &sky.rain,
        Weather::Snow => &sky.snow,
    };
    Vec3::from(colors.horizon).lerp(Vec3::from(colors.zenith), 0.3)
}

fn water_material(params: WaterParams, blend: bool, unlit: bool) -> WaterMaterial {
    ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.2,
            reflectance: 0.25,
            cull_mode: None,
            unlit,
            alpha_mode: if blend {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            ..default()
        },
        extension: WaterExtension { params },
    }
}

pub struct BattleWaterPlugin;

impl Plugin for BattleWaterPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "water.wesl");
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .register_dump(dump_water)
            .add_systems(Update, build_water.run_if(resource_added::<BattleSession>))
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<WaterStats>();
            });
    }
}

fn build_water(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    look: Option<Res<DecorLook>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
) {
    let sim = session.sim_mut();
    let weather = sim.weather();
    let field: &Battlefield = sim.field();
    let scope = DespawnOnExit(AppState::Battle);
    let sky = sky_tint(look.as_deref(), weather);
    let mut stats = WaterStats {
        rain_ripples: matches!(weather, Weather::Rain),
        ..default()
    };

    // River and its foam at the piers.
    if let Some(geometry) = river_geometry(field) {
        let ribbon = &geometry.ribbon;
        stats.river = true;
        stats.river_rows = ribbon.rows;
        stats.river_vertices = ribbon.positions.len();
        stats.river_max_width_m = ribbon.max_width;
        stats.river_max_depth_m = ribbon.max_depth;
        stats.river_level_min = geometry
            .levels
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);
        stats.river_level_max = geometry
            .levels
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        let params = river_params(weather, sky, 1.0, ribbon.max_width);
        let material = materials.add(water_material(params, true, false));
        let foams = pier_foams(field, &|x| geometry.level_at(x));
        commands.spawn((
            Name::new("River"),
            Mesh3d(meshes.add(ribbon.clone().into_mesh())),
            MeshMaterial3d(material),
            Transform::default(),
            NotShadowCaster,
            scope.clone(),
        ));
        if !foams.is_empty() {
            let mesh = meshes.add(foam_quad());
            let foam_params = WaterParams {
                misc: Vec4::new(KIND_FOAM, 0.6, 0.0, -1.0),
                ..default()
            };
            let material = materials.add(water_material(foam_params, true, true));
            for foam in &foams {
                commands.spawn((
                    Name::new("PierFoam"),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(foam.position)
                        .with_rotation(Quat::from_rotation_y(foam.yaw)),
                    NotShadowCaster,
                    VisibilityRange::abrupt(0.0, FOAM_RANGE),
                    scope.clone(),
                ));
            }
            stats.pier_foam = foams.len();
        }
    }

    // Tributary and brooks: the river shader, narrower and clearer.
    if !field.streams.is_empty() {
        for stream in &field.streams {
            let ribbon = stream_ribbon(stream, field);
            if ribbon.rows < 2 {
                continue;
            }
            let mut params = river_params(weather, sky, 0.8 / 0.55, stream.width as f32 + 1.6);
            params.deep.w = 1.6;
            params.shallow.w = 0.15;
            stats.stream_vertices += ribbon.positions.len();
            stats.streams += 1;
            commands.spawn((
                Name::new("Stream"),
                Mesh3d(meshes.add(ribbon.into_mesh())),
                MeshMaterial3d(materials.add(water_material(params, true, false))),
                Transform::default(),
                NotShadowCaster,
                scope.clone(),
            ));
        }
    }

    // Sea of a coastal field.
    if let (Some(rect), Some(coast)) = (sea_rect(field), field.coast.as_ref()) {
        let flank = match coast.flank {
            Flank::West => -1.0,
            Flank::East => 1.0,
        };
        stats.sea = true;
        stats.sea_area_m2 = (rect[1] - rect[0]) * (rect[3] - rect[2]);
        stats.sea_flank = flank;
        stats.sea_shore_x = coast.shore_x as f32;
        let mut params = WaterParams {
            sky: sky.extend(if matches!(weather, Weather::Rain) {
                1.0
            } else {
                0.0
            }),
            misc: Vec4::new(KIND_SEA, 1.0, coast.shore_x as f32, flank),
            ..default()
        };
        params.deep = Vec4::new(0.05, 0.13, 0.17, 1.0).map_rgb_to_linear();
        commands.spawn((
            Name::new("Sea"),
            Mesh3d(meshes.add(sea_quad(rect, SEA_LEVEL))),
            MeshMaterial3d(materials.add(water_material(params, false, false))),
            Transform::default(),
            NotShadowCaster,
            scope.clone(),
        ));
    }
    commands.insert_resource(stats);
}

fn dump_water(world: &World) -> (String, serde_json::Value) {
    let value = world.get_resource::<WaterStats>().map(|s| {
        serde_json::json!({
            "river": s.river, "river_rows": s.river_rows, "river_vertices": s.river_vertices,
            "river_max_width_m": s.river_max_width_m, "river_max_depth_m": s.river_max_depth_m,
            "river_level_min": s.river_level_min, "river_level_max": s.river_level_max,
            "streams": s.streams, "stream_vertices": s.stream_vertices,
            "sea": s.sea, "sea_area_m2": s.sea_area_m2, "sea_flank": s.sea_flank,
            "sea_shore_x": s.sea_shore_x, "pier_foam": s.pier_foam,
            "rain_ripples": s.rain_ripples,
        })
    });
    ("battle_water".to_owned(), value.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ribbon_has_rows_times_columns_vertices() {
        let points: Vec<Vec2> = (0..5).map(|i| Vec2::new(i as f32 * 8.0, 10.0)).collect();
        let levels = vec![1.0; 5];
        let sides = vec![6.0; 5];
        let ground = |_x: f32, z: f32| 1.0 - 0.5 + (z - 10.0).abs() * 0.1;
        let data = build_ribbon(&points, &levels, &sides, &sides, 9, &ground);
        assert_eq!(data.positions.len(), 45);
        assert_eq!(data.indices.len(), 4 * 8 * 6);
        assert_eq!(data.rows, 5);
        assert!((data.max_width - 12.0).abs() < 1e-4);
    }

    #[test]
    fn depth_is_zero_on_the_bank_lines_and_positive_in_the_middle() {
        let points: Vec<Vec2> = (0..3).map(|i| Vec2::new(i as f32 * 8.0, 0.0)).collect();
        let levels = vec![1.0; 3];
        let sides = vec![4.0; 3];
        let ground = |_x: f32, z: f32| 0.5 + (z.abs() * 0.25).min(2.0);
        let data = build_ribbon(&points, &levels, &sides, &sides, 9, &ground);
        let row = &data.depths[0..9];
        assert_eq!(row[0][0], 0.0);
        assert_eq!(row[8][0], 0.0);
        assert!((row[4][0] - 0.5).abs() < 1e-4);
        assert!(row[2][0] < row[4][0]);
    }

    #[test]
    fn levels_smooth_towards_their_neighbours() {
        let mut levels = vec![0.0, 0.0, 4.0, 0.0, 0.0];
        smooth_levels(&mut levels, 1, false);
        assert!((levels[2] - 2.0).abs() < 1e-5);
        assert!((levels[1] - 1.0).abs() < 1e-5);
        let mut kept = vec![0.0, 0.0, 4.0, 0.0, 0.0];
        smooth_levels(&mut kept, 3, true);
        assert!(kept.iter().all(|l| *l <= 4.0));
    }

    #[test]
    fn rain_makes_the_river_ripple_and_murky() {
        let clear = river_params(Weather::Clear, Vec3::ONE, 1.0, 30.0);
        let rain = river_params(Weather::Rain, Vec3::ONE, 1.0, 30.0);
        assert_eq!(clear.sky.w, 0.0);
        assert_eq!(rain.sky.w, 1.0);
        assert!(rain.shallow.w > clear.shallow.w);
    }

    #[test]
    fn foam_quad_is_a_rectangle() {
        assert_eq!(foam_quad().count_vertices(), 4);
        assert_eq!(sea_quad([0.0, 1.0, 0.0, 1.0], 0.0).count_vertices(), 4);
    }
}
