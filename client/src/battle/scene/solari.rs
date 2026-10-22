//! Trial of `bevy_solari` (raytraced lighting) on the battle scene. Compiled only with the cargo
//! feature `solari`; inert unless `data/fx/solari.json` has `enabled: true` AND the GPU exposes
//! the ray-query features (checked on a throw-away adapter before the app is built, because
//! `DefaultOpaqueRendererMethod::deferred()` is inserted by the plugin even on unsupported GPUs).
//!
//! Scene geometry reaches the ray-tracing acceleration structure through SUBSTITUTE entities: the
//! raster meshes have left the main world (`RenderAssetUsages` without `MAIN_WORLD`), so a
//! `RaytracingMesh3d` cannot reuse their handle. A substitute is a child of the original (same
//! transform, no `Mesh3d`, hence invisible to the raster) carrying a copy of the mesh reduced to
//! position/normal/uv/tangent, plus a `StandardMaterial` of the mean colour of the original.
//! The ground gets a coarse grid built from the battlefield (rays only; the visible ground is
//! rendered deferred, with its own G-buffer shader path, and lit by Solari). Instanced soldiers are left out.
//! See docs/wip/bevy-solari.md. Msaa must be off, shadow maps are replaced by ray queries.

use bevy::asset::RenderAssetUsages;
use bevy::camera::CameraMainTextureUsages;
use bevy::camera::visibility::VisibilityRange;
use bevy::material::OpaqueRendererMethod;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::TextureUsages;
use bevy::solari::SolariPlugins;
use bevy::solari::prelude::{RaytracingMesh3d, SolariLighting};
use serde::Deserialize;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::decor::ground::GroundMaterial;
use crate::battle::decor::wind::WindMaterial;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::battle::scene::terrain::BattleTerrain;
use crate::battle::siege::stone::SiegeStoneMaterial;
use crate::campaign::towns::atlas::BuildingAtlasMaterial;
use crate::paths;

const DATA_FILE: &str = "fx/solari.json";

/// `data/fx/solari.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
pub struct SolariSettings {
    pub enabled: bool,
    #[serde(default)]
    pub restir: bool,
    #[serde(default = "default_bounces")]
    pub max_bounces: u32,
    /// Vertex spacing (m) of the coarse ground substitute.
    #[serde(default = "default_ground_step")]
    pub ground_step_m: f32,
    /// Substitute budget: triangles over all distinct substitute meshes.
    #[serde(default = "default_max_triangles")]
    pub max_triangles: usize,
    /// Substitute budget: traced instances.
    #[serde(default = "default_max_instances")]
    pub max_instances: usize,
}

fn default_bounces() -> u32 {
    2
}

fn default_ground_step() -> f32 {
    12.0
}

fn default_max_triangles() -> usize {
    4_000_000
}

fn default_max_instances() -> usize {
    40_000
}

impl SolariSettings {
    fn load() -> Option<Self> {
        let root = paths::find_root().ok()?;
        let bytes = std::fs::read(paths::data_dir(&root).join(DATA_FILE)).ok()?;
        match serde_json::from_slice(&bytes) {
            Ok(settings) => Some(settings),
            Err(error) => {
                error!("{DATA_FILE}: {error}");
                None
            }
        }
    }
}

/// Wgpu features Solari needs that the default high-performance adapter lacks (empty = supported).
pub fn missing_features() -> Result<String, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = bevy::tasks::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..default()
    }))
    .map_err(|error| format!("no adapter: {error}"))?;
    let missing = SolariPlugins::required_wgpu_features().difference(adapter.features());
    Ok(if missing.is_empty() {
        String::new()
    } else {
        format!("{missing:?}")
    })
}

pub struct SolariTrialPlugin;

impl Plugin for SolariTrialPlugin {
    fn build(&self, app: &mut App) {
        app.register_dump(dump_solari);
        let Some(settings) = SolariSettings::load().filter(|settings| settings.enabled) else {
            return;
        };
        match missing_features() {
            Ok(missing) if missing.is_empty() => {}
            Ok(missing) => {
                info!("solari: GPU lacks {missing}, falling back to the raster pipeline");
                return;
            }
            Err(error) => {
                info!("solari: {error}, falling back to the raster pipeline");
                return;
            }
        }
        info!("solari: ray-query GPU found, raytraced lighting on in battles");
        app.add_plugins(SolariPlugins)
            .insert_resource(settings)
            .init_resource::<Substitutes>()
            .add_systems(
                Update,
                (
                    light_camera,
                    no_shadow_maps,
                    substitute_ground,
                    substitute_standard,
                    substitute_decor::<SiegeStoneMaterial>,
                    substitute_decor::<BuildingAtlasMaterial>,
                    substitute_decor::<WindMaterial>,
                    force_forward::<SiegeStoneMaterial>,
                    force_forward::<BuildingAtlasMaterial>,
                    force_forward::<WindMaterial>,
                )
                    .run_if(in_state(AppState::Battle)),
            )
            .add_systems(OnExit(AppState::Battle), reset_substitutes);
    }
}

/// Dump section `solari`: whether the plugin is active and whether the battle ground goes through
/// the deferred G-buffer path (its material no longer forced to forward rendering).
fn dump_solari(world: &World) -> (String, serde_json::Value) {
    let active = world.contains_resource::<SolariSettings>();
    let ground_deferred = active
        && world
            .get_resource::<Assets<GroundMaterial>>()
            .is_some_and(|materials| {
                materials
                    .iter()
                    .any(|(_, m)| m.base.opaque_render_method != OpaqueRendererMethod::Forward)
            });
    (
        "solari".to_owned(),
        serde_json::json!({ "active": active, "ground_deferred": ground_deferred }),
    )
}

fn light_camera(
    mut commands: Commands,
    settings: Res<SolariSettings>,
    cameras: Query<Entity, (With<BattleCamera>, Without<SolariLighting>)>,
) {
    for entity in &cameras {
        commands.entity(entity).insert((
            SolariLighting {
                restir: settings.restir,
                max_bounces: settings.max_bounces,
                ..default()
            },
            CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
            Msaa::Off,
        ));
        commands
            .entity(entity)
            .remove::<bevy::pbr::ScreenSpaceAmbientOcclusion>();
    }
}

/// Base colour of a material (the `StandardMaterial` itself or the base of an extended one).
trait BaseColor {
    fn base_color(&self) -> Color;
    fn base_mut(&mut self) -> &mut StandardMaterial;
}

impl BaseColor for StandardMaterial {
    fn base_color(&self) -> Color {
        self.base_color
    }

    fn base_mut(&mut self) -> &mut StandardMaterial {
        self
    }
}

impl<E: MaterialExtension> BaseColor for ExtendedMaterial<StandardMaterial, E> {
    fn base_color(&self) -> Color {
        self.base.base_color
    }

    fn base_mut(&mut self) -> &mut StandardMaterial {
        &mut self.base
    }
}

/// Marks an original entity whose substitute was created (or deliberately skipped).
#[derive(Component)]
struct Proxied;

/// Marks the ground substitute.
#[derive(Component)]
struct GroundSubstitute;

/// Cached reduced mesh of one source mesh and its mean vertex colour.
type CachedMesh = Option<(Handle<Mesh>, Vec4)>;

/// Caches shared by all substitutions, plus the counters logged for the budget.
#[derive(Resource, Default)]
struct Substitutes {
    meshes: HashMap<AssetId<Mesh>, CachedMesh>,
    materials: HashMap<(bevy::asset::UntypedAssetId, AssetId<Mesh>), Handle<StandardMaterial>>,
    instances: usize,
    triangles: usize,
    skipped_budget: usize,
    last_logged: usize,
}

fn reset_substitutes(mut substitutes: ResMut<Substitutes>) {
    *substitutes = Substitutes::default();
}

/// Copy of `source` reduced to what Solari reads (position, normal, uv0, tangent, `u32` indices),
/// with the mean vertex colour. `None` when the data left the main world or is not a triangle list.
fn reduced_copy(source: &Mesh) -> Option<(Mesh, Vec4)> {
    if source.primitive_topology() != PrimitiveTopology::TriangleList {
        return None;
    }
    let Ok(Some(VertexAttributeValues::Float32x3(positions))) =
        source.try_attribute_option(Mesh::ATTRIBUTE_POSITION)
    else {
        return None;
    };
    let indices: Vec<u32> = match source.try_indices_option().ok()? {
        Some(indices) => indices.iter().map(|index| index as u32).collect(),
        None => (0..positions.len() as u32).collect(),
    };
    if indices.len() < 3 || indices.iter().any(|&i| i as usize >= positions.len()) {
        return None;
    }
    let normals = match source.try_attribute_option(Mesh::ATTRIBUTE_NORMAL) {
        Ok(Some(VertexAttributeValues::Float32x3(normals))) if normals.len() == positions.len() => {
            normals.clone()
        }
        _ => smooth_normals(positions, &indices),
    };
    let mean_color = match source.try_attribute_option(Mesh::ATTRIBUTE_COLOR) {
        Ok(Some(VertexAttributeValues::Float32x4(colors))) if !colors.is_empty() => {
            let sum = colors
                .iter()
                .fold(Vec4::ZERO, |sum, c| sum + Vec4::from(*c));
            sum / colors.len() as f32
        }
        _ => Vec4::ONE,
    };
    let mesh = raytracing_mesh(positions.clone(), normals, indices);
    Some((mesh, mean_color))
}

/// Mesh with exactly the attributes Solari accepts (position, normal, uv0, tangent) and `u32`
/// indices, kept in both worlds.
fn raytracing_mesh(positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
    let count = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; count])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_TANGENT,
        vec![[1.0f32, 0.0, 0.0, 1.0]; count],
    )
    .with_inserted_indices(Indices::U32(indices))
}

/// Area-weighted vertex normals.
fn smooth_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut sums = vec![Vec3::ZERO; positions.len()];
    for triangle in indices.as_chunks::<3>().0 {
        let [a, b, c] = [triangle[0], triangle[1], triangle[2]]
            .map(|index| Vec3::from(positions[index as usize]));
        let normal = (b - a).cross(c - a);
        for &index in triangle {
            sums[index as usize] += normal;
        }
    }
    sums.into_iter()
        .map(|sum| sum.try_normalize().unwrap_or(Vec3::Y).to_array())
        .collect()
}

fn substitute_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.9,
        reflectance: 0.04,
        ..default()
    }
}

type Original<'a, M> = (
    Entity,
    &'a Mesh3d,
    &'a MeshMaterial3d<M>,
    Option<&'a VisibilityRange>,
);

/// Entities handled per frame by [`substitute_decor`] (keeps the first frames smooth).
const PER_FRAME: usize = 1500;

/// Extended-material variant of [`substitute_core`] (the base colour comes from `Assets<M>`).
fn substitute_decor<M: Material + BaseColor>(
    commands: Commands,
    settings: Res<SolariSettings>,
    substitutes: ResMut<Substitutes>,
    meshes: ResMut<Assets<Mesh>>,
    materials: Res<Assets<M>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    originals: Query<Original<M>, Without<Proxied>>,
) {
    substitute_core(
        commands,
        &settings,
        substitutes,
        meshes,
        &mut standard,
        &originals,
        |handle| materials.get(handle).map(BaseColor::base_color),
    );
}

/// `StandardMaterial` variant: the same asset store is read and written, so one borrow only.
fn substitute_standard(
    commands: Commands,
    settings: Res<SolariSettings>,
    substitutes: ResMut<Substitutes>,
    meshes: ResMut<Assets<Mesh>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    originals: Query<Original<StandardMaterial>, Without<Proxied>>,
) {
    let colors: HashMap<_, _> = originals
        .iter()
        .filter_map(|(_, _, MeshMaterial3d(handle), _)| {
            standard
                .get(handle)
                .map(|material| (handle.id(), material.base_color))
        })
        .collect();
    substitute_core(
        commands,
        &settings,
        substitutes,
        meshes,
        &mut standard,
        &originals,
        |handle| colors.get(&handle.id()).copied(),
    );
}

/// Gives every static decor entity of material `M` a ray-tracing substitute child. Far LODs
/// (visibility range that starts later than 0) are skipped: the near one stands for the object.
fn substitute_core<M: Material>(
    mut commands: Commands,
    settings: &SolariSettings,
    mut substitutes: ResMut<Substitutes>,
    mut meshes: ResMut<Assets<Mesh>>,
    standard: &mut Assets<StandardMaterial>,
    originals: &Query<Original<M>, Without<Proxied>>,
    color_of: impl Fn(&Handle<M>) -> Option<Color>,
) {
    for (entity, Mesh3d(mesh_handle), MeshMaterial3d(material_handle), range) in
        originals.iter().take(PER_FRAME)
    {
        let Some(base_color) = color_of(material_handle) else {
            continue; // not loaded yet, retried next frame
        };
        commands.entity(entity).insert(Proxied);
        if range.is_some_and(|range| range.start_margin.end > 0.0) {
            continue;
        }
        let mesh_id = mesh_handle.id();
        if !substitutes.meshes.contains_key(&mesh_id) {
            // `None` is cached for meshes that already left the main world: never retried.
            let entry = meshes
                .get(mesh_id)
                .and_then(reduced_copy)
                .and_then(|(copy, color)| {
                    let triangles = copy.indices().map_or(0, |indices| indices.len() / 3);
                    if substitutes.triangles + triangles > settings.max_triangles {
                        substitutes.skipped_budget += 1;
                        return None;
                    }
                    substitutes.triangles += triangles;
                    Some((meshes.add(copy), color))
                });
            substitutes.meshes.insert(mesh_id, entry);
        }
        let Some(Some((substitute_mesh, vertex_color))) = substitutes.meshes.get(&mesh_id).cloned()
        else {
            continue;
        };
        if substitutes.instances >= settings.max_instances {
            substitutes.skipped_budget += 1;
            continue;
        }
        let key = (material_handle.id().untyped(), mesh_id);
        let standard_handle = substitutes
            .materials
            .entry(key)
            .or_insert_with(|| {
                let base = base_color.to_linear().to_vec4();
                let mixed = base * vertex_color;
                standard.add(substitute_material(Color::linear_rgb(
                    mixed.x, mixed.y, mixed.z,
                )))
            })
            .clone();
        substitutes.instances += 1;
        commands.entity(entity).with_child((
            Name::new("SolariSubstitute"),
            RaytracingMesh3d(substitute_mesh),
            MeshMaterial3d(standard_handle),
            Transform::default(),
        ));
    }
    if originals.is_empty() && substitutes.instances != substitutes.last_logged {
        substitutes.last_logged = substitutes.instances;
        log_totals(&substitutes);
    }
}

fn log_totals(substitutes: &Substitutes) {
    info!(
        "solari: {} substitute instances, {} distinct meshes, {} triangles, {} skipped (budget)",
        substitutes.instances,
        substitutes.meshes.values().flatten().count(),
        substitutes.triangles,
        substitutes.skipped_budget
    );
}

#[allow(clippy::too_many_arguments)]
/// Coarse ground grid (spacing `ground_step_m`) in the season's ground colour.
fn substitute_ground(
    mut commands: Commands,
    settings: Res<SolariSettings>,
    mut substitutes: ResMut<Substitutes>,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    terrain: Query<Entity, With<BattleTerrain>>,
    existing: Query<(), With<GroundSubstitute>>,
) {
    if terrain.is_empty() || !existing.is_empty() {
        return;
    }
    let field = session.sim_mut().field();
    let step = f64::from(settings.ground_step_m.max(1.0));
    let columns = ((field.width / step).round() as usize).max(1) + 1;
    let rows = ((field.depth / step).round() as usize).max(1) + 1;
    let height = |x: f64, z: f64| field.height(x, z) as f32;
    let mut positions = Vec::with_capacity(columns * rows);
    let mut normals = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        let z = (row as f64 * step).min(field.depth);
        for column in 0..columns {
            let x = (column as f64 * step).min(field.width);
            positions.push([x as f32, height(x, z), z as f32]);
            let dx = height((x + step).min(field.width), z) - height((x - step).max(0.0), z);
            let dz = height(x, (z + step).min(field.depth)) - height(x, (z - step).max(0.0));
            normals.push(
                Vec3::new(-dx, 2.0 * step as f32, -dz)
                    .normalize()
                    .to_array(),
            );
        }
    }
    let mut indices = Vec::with_capacity((columns - 1) * (rows - 1) * 6);
    for row in 0..rows - 1 {
        for column in 0..columns - 1 {
            let a = (row * columns + column) as u32;
            let b = a + 1;
            let c = a + columns as u32;
            let d = c + 1;
            indices.extend([a, c, b, b, c, d]);
        }
    }
    let triangles = indices.len() / 3;
    let vertices = positions.len();
    let [r, g, b] = crate::battle::scene::terrain::season_ground(field.season);
    substitutes.triangles += triangles;
    commands.spawn((
        Name::new("SolariGround"),
        GroundSubstitute,
        RaytracingMesh3d(meshes.add(raytracing_mesh(positions, normals, indices))),
        MeshMaterial3d(standard.add(substitute_material(Color::srgb(r, g, b)))),
        Transform::default(),
        DespawnOnExit(AppState::Battle),
    ));
    info!("solari: ground substitute, {triangles} triangles ({vertices} vertices)");
}

/// Solari makes the deferred renderer the default, and a deferred-resolved material is dropped
/// from the main pass. The custom-shader materials (wind foliage/grass, stone, building atlas) have
/// no G-buffer output, so they are forced back to forward rendering: still visible, lit by the
/// raster lights but not by Solari. The ground is NOT in this list: `ground.wesl` has a deferred
/// path (`deferred_fragment_shader`), so Solari lights it.
fn force_forward<M: Material + BaseColor>(
    mut events: MessageReader<AssetEvent<M>>,
    mut materials: ResMut<Assets<M>>,
) {
    for event in events.read() {
        let (AssetEvent::Added { id } | AssetEvent::Modified { id }) = event else {
            continue;
        };
        let Some(mut material) = materials.get_mut(*id) else {
            continue;
        };
        if material.base_mut().opaque_render_method != OpaqueRendererMethod::Forward {
            material.base_mut().opaque_render_method = OpaqueRendererMethod::Forward;
        }
    }
}

fn no_shadow_maps(mut lights: Query<&mut DirectionalLight, Changed<DirectionalLight>>) {
    for mut light in &mut lights {
        if light.shadow_maps_enabled {
            light.shadow_maps_enabled = false;
        }
    }
}
