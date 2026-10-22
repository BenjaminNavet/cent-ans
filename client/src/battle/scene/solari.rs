//! Trial of `bevy_solari` (raytraced lighting) on the battle scene. Compiled only with the cargo
//! feature `solari`; inert unless `data/fx/solari.json` has `enabled: true` AND the GPU exposes
//! the ray-query features (checked on a throw-away adapter before the app is built, because
//! `DefaultOpaqueRendererMethod::deferred()` is inserted by the plugin even on unsupported GPUs).
//!
//! Limits of the prototype (see docs/wip/bevy-solari.md): Solari only traces entities with
//! `RaytracingMesh3d` + `MeshMaterial3d<StandardMaterial>`. The ground (`ExtendedMaterial`), the
//! instanced soldiers and the plants use custom materials, so they are neither in the TLAS nor
//! lit by the deferred path. Msaa must be off, shadow maps are replaced by ray queries.

use bevy::camera::CameraMainTextureUsages;
use bevy::mesh::{Indices, Mesh};
use bevy::prelude::*;
use bevy::render::render_resource::TextureUsages;
use bevy::solari::SolariPlugins;
use bevy::solari::prelude::{RaytracingMesh3d, SolariLighting};
use serde::Deserialize;

use crate::app::AppState;
use crate::battle::scene::camera::BattleCamera;
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
}

fn default_bounces() -> u32 {
    2
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
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
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
            .add_systems(
                Update,
                (light_camera, trace_standard_meshes, no_shadow_maps)
                    .run_if(in_state(AppState::Battle)),
            );
    }
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

/// Solari needs UV0, tangents and `u32` indices on every traced mesh (same fix-ups as Bevy's
/// `solari` example).
fn trace_standard_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    entities: Query<
        (Entity, &Mesh3d),
        (
            With<MeshMaterial3d<StandardMaterial>>,
            Without<RaytracingMesh3d>,
        ),
    >,
) {
    for (entity, Mesh3d(handle)) in &entities {
        let Some(mut mesh) = meshes.get_mut(handle) else {
            continue;
        };
        if !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_0) {
            let count = mesh.count_vertices();
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; count]);
            mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[0.0f32; 4]; count]);
        }
        if !mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT) && mesh.generate_tangents().is_err() {
            continue;
        }
        if mesh.contains_attribute(Mesh::ATTRIBUTE_UV_1) {
            mesh.remove_attribute(Mesh::ATTRIBUTE_UV_1);
        }
        if let Some(indices) = mesh.indices_mut()
            && let Indices::U16(_) = indices
        {
            *indices = Indices::U32(indices.iter().map(|index| index as u32).collect());
        }
        commands
            .entity(entity)
            .insert(RaytracingMesh3d(handle.clone()));
    }
}

fn no_shadow_maps(mut lights: Query<&mut DirectionalLight, Changed<DirectionalLight>>) {
    for mut light in &mut lights {
        if light.shadow_maps_enabled {
            light.shadow_maps_enabled = false;
        }
    }
}
