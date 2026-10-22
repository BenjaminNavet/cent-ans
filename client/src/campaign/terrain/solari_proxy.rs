//! Prototype of `bevy_solari` on the campaign map (docs/wip/bevy-solari-campagne-proto.md). Compiled
//! only with the cargo feature `solari`; inert unless `data/fx/solari_campaign.json` has
//! `enabled: true` AND the GPU exposes the ray-query features (probe shared with the battle trial).
//!
//! The campaign terrain (`TerrainMaterial`, house forward shader, relief displaced in the vertex
//! shader) cannot be lit by Solari as is: the geometry the rays would hit is not the displayed one
//! and the fragment does not write the G-buffer. So, while Solari is on:
//! - a COARSE PROXY of the terrain (regular grid of ~`terrain_proxy_triangles` triangles, heights
//!   taken from `Surface::display_height_at`, the CPU twin of `displayed_y`, plus the edge sink of
//!   the vertex shader) is drawn with a deferred `StandardMaterial` and added to the acceleration
//!   structure (`RaytracingMesh3d`, same mesh, kept in both worlds);
//! - the quadtree nodes of the real terrain are hidden (`Visibility::Hidden`, re-applied after
//!   `update_lod`; their logic is untouched);
//! - the campaign camera gets `SolariLighting` (+ HDR, depth/deferred/motion prepasses, MSAA off)
//!   and a sky gradient `EnvironmentMapLight` (Solari ignores `AmbientLight`);
//! - the sun keeps lighting (its shadow maps are replaced by ray queries and switched off).
//!
//! The proxy albedo is the composed colormap read again from disk at a coarse mip (one texel per
//! BC1 block) or a flat colour. Rivers, sea, roads, vegetation, towns and armies keep their forward
//! materials (the plugin default is switched back to forward) and stay out of the TLAS.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy::asset::RenderAssetUsages;
use bevy::camera::CameraMainTextureUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::EnvironmentMapLight;
use bevy::material::OpaqueRendererMethod;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::DefaultOpaqueRendererMethod;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::solari::SolariPlugins;
use bevy::solari::prelude::{RaytracingMesh3d, SolariLighting};
use serde::Deserialize;
use serde_json::{Value, json};

use super::TerrainMaterialHandle;
use super::colormap::load_colormap;
use super::material::TerrainMaterial;
use super::relief_state::ReliefState;
use super::surface::Surface;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::CampaignCamera;
use crate::paths;
use crate::settings::quality::RenderQuality;

const DATA_FILE: &str = "fx/solari_campaign.json";
/// Relative change of the vertical exaggeration that triggers a rebuild of the proxy.
const RESCALE_THRESHOLD: f32 = 0.05;
/// Minimum seconds between two rebuilds.
const REBUILD_MIN_SECONDS: f32 = 1.5;
/// Widest albedo texture (texels) taken from the colormap mips.
const ALBEDO_MAX_WIDTH: usize = 1024;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AlbedoFrom {
    ComposedMap,
    Flat,
}

/// `data/fx/solari_campaign.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
struct CampaignSolariSettings {
    enabled: bool,
    #[serde(default = "default_triangles")]
    terrain_proxy_triangles: usize,
    #[serde(default = "default_albedo")]
    albedo_from: AlbedoFrom,
    #[serde(default = "default_flat")]
    flat_color: [f32; 3],
    #[serde(default)]
    restir: bool,
    #[serde(default = "default_bounces")]
    max_bounces: u32,
    #[serde(default = "default_sky")]
    sky_intensity: f32,
}

fn default_triangles() -> usize {
    500_000
}
fn default_albedo() -> AlbedoFrom {
    AlbedoFrom::ComposedMap
}
fn default_flat() -> [f32; 3] {
    [0.28, 0.32, 0.18]
}
fn default_bounces() -> u32 {
    2
}
fn default_sky() -> f32 {
    3000.0
}

impl CampaignSolariSettings {
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

/// What the dump section `solari_campaign` reports.
#[derive(Resource, Default, Debug, Clone)]
struct ProxyStats {
    enabled: bool,
    reason: String,
    proxy_triangles: usize,
    build_ms: f32,
    rebuilds: u32,
    camera_deferred: bool,
    albedo_texels: [usize; 2],
}

/// Decoded coarse colormap (RGBA8 sRGB), delivered by a thread.
struct AlbedoPixels {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

type AlbedoSlot = Arc<Mutex<Option<Result<AlbedoPixels, String>>>>;

#[derive(Resource, Default)]
struct ProxyState {
    entity: Option<Entity>,
    built_scale: f32,
    last_build: Option<Instant>,
    albedo: Option<Handle<Image>>,
    pending_albedo: Option<AlbedoSlot>,
    /// `true` once the albedo is settled (decoded, failed or not wanted).
    albedo_settled: bool,
}

#[derive(Component)]
struct TerrainProxy;

pub fn add_plugin(app: &mut App) {
    let mut stats = ProxyStats::default();
    let settings = CampaignSolariSettings::load();
    let usable = match &settings {
        None => {
            stats.reason = "no settings file".into();
            None
        }
        Some(settings) if !settings.enabled => {
            stats.reason = "disabled in solari_campaign.json".into();
            None
        }
        Some(settings) => match crate::battle::scene::solari::missing_features() {
            Ok(missing) if missing.is_empty() => Some(settings.clone()),
            Ok(missing) => {
                stats.reason = format!("GPU lacks {missing}");
                None
            }
            Err(error) => {
                stats.reason = error;
                None
            }
        },
    };
    app.register_dump(dump);
    let Some(settings) = usable else {
        info!("solari campaign: {}, raster pipeline", stats.reason);
        app.insert_resource(stats);
        return;
    };
    stats.enabled = true;
    stats.reason = "active".into();
    if !app.is_plugin_added::<bevy::solari::realtime::SolariLightingPlugin>() {
        app.add_plugins(SolariPlugins);
    }
    info!(
        "solari campaign: ray-query GPU found, terrain proxy of ~{} triangles ({:?} albedo)",
        settings.terrain_proxy_triangles, settings.albedo_from
    );
    app.insert_resource(stats)
        .insert_resource(settings)
        .init_resource::<ProxyState>()
        .add_systems(Startup, default_to_forward_without_battle_trial)
        .add_systems(
            OnEnter(AppState::Campaign),
            (start_albedo, default_to_forward),
        )
        .add_systems(
            OnExit(AppState::Campaign),
            (reset_proxy, default_to_deferred_for_battle),
        )
        .add_systems(
            Update,
            (
                poll_albedo,
                light_camera,
                build_proxy,
                hide_terrain_nodes.after(super::update_lod),
                force_msaa_off
                    .after(crate::settings::quality::resolve_quality)
                    .before(crate::settings::quality::apply_msaa),
                sun_without_shadow_maps.after(crate::settings::quality::apply_shadows),
            )
                .run_if(in_state(AppState::Campaign)),
        );
}

/// `SolariLightingPlugin` makes deferred the default of every material, which would hide all the
/// house forward materials of the map; only the proxy asks for deferred explicitly. (The battle
/// trial relies on the default and forces its own custom materials to forward.)
fn default_to_forward(mut commands: Commands) {
    commands.insert_resource(DefaultOpaqueRendererMethod::forward());
}

/// At startup: forward unless the battle trial (inserting its settings resource) relies on deferred.
fn default_to_forward_without_battle_trial(
    battle_trial: Option<Res<crate::battle::scene::solari::SolariSettings>>,
    mut commands: Commands,
) {
    if battle_trial.is_none() {
        commands.insert_resource(DefaultOpaqueRendererMethod::forward());
    }
}

/// Back to the plugin default when the battle trial is active (it inserts its settings resource).
fn default_to_deferred_for_battle(
    battle_trial: Option<Res<crate::battle::scene::solari::SolariSettings>>,
    mut commands: Commands,
) {
    if battle_trial.is_some() {
        commands.insert_resource(DefaultOpaqueRendererMethod::deferred());
    }
}

fn reset_proxy(mut state: ResMut<ProxyState>, mut stats: ResMut<ProxyStats>) {
    state.entity = None;
    state.built_scale = 0.0;
    state.last_build = None;
    stats.camera_deferred = false;
}

// --- albedo -----------------------------------------------------------------------------------

/// Starts the thread that re-reads `colormap.bc1` and keeps one texel per block of a coarse mip.
fn start_albedo(settings: Res<CampaignSolariSettings>, mut state: ResMut<ProxyState>) {
    if state.albedo.is_some() || state.pending_albedo.is_some() {
        return;
    }
    if settings.albedo_from != AlbedoFrom::ComposedMap {
        state.albedo_settled = true;
        return;
    }
    let Ok(root) = paths::find_root() else {
        state.albedo_settled = true;
        return;
    };
    let slot: AlbedoSlot = Arc::new(Mutex::new(None));
    let out = Arc::clone(&slot);
    std::thread::spawn(move || {
        let started = Instant::now();
        let result = load_colormap(&root.join("data/map"))
            .and_then(|copy| copy.ok_or_else(|| "no colormap in map.json".to_string()))
            .and_then(|copy| {
                coarse_albedo(
                    copy.width as usize,
                    copy.height as usize,
                    copy.mips,
                    &copy.data,
                )
            });
        info!(
            "solari campaign: coarse albedo decoded in {:.1} s",
            started.elapsed().as_secs_f32()
        );
        *out.lock().expect("albedo slot") = Some(result);
    });
    state.pending_albedo = Some(slot);
}

fn poll_albedo(
    mut state: ResMut<ProxyState>,
    mut stats: ResMut<ProxyStats>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(slot) = state.pending_albedo.clone() else {
        return;
    };
    let Some(result) = slot.lock().expect("albedo slot").take() else {
        return;
    };
    state.pending_albedo = None;
    state.albedo_settled = true;
    match result {
        Ok(pixels) => {
            stats.albedo_texels = [pixels.width, pixels.height];
            let mut image = Image::new(
                Extent3d {
                    width: pixels.width as u32,
                    height: pixels.height as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels.data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::ClampToEdge,
                address_mode_v: ImageAddressMode::ClampToEdge,
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..default()
            });
            state.albedo = Some(images.add(image));
        }
        Err(error) => warn!("solari campaign: albedo unavailable ({error}), flat colour"),
    }
}

/// Bytes of one BC1 level.
fn level_size(width: usize, height: usize) -> usize {
    width.max(1).div_ceil(4) * height.max(1).div_ceil(4) * 8
}

fn rgb565(value: u16) -> [f32; 3] {
    [
        f32::from((value >> 11) & 31) * 255.0 / 31.0,
        f32::from((value >> 5) & 63) * 255.0 / 63.0,
        f32::from(value & 31) * 255.0 / 31.0,
    ]
}

/// One RGBA texel (mean of the two block end colours) per BC1 block of the first mip level whose
/// block grid is at most [`ALBEDO_MAX_WIDTH`] wide.
fn coarse_albedo(
    width: usize,
    height: usize,
    mips: bool,
    data: &[u8],
) -> Result<AlbedoPixels, String> {
    let (mut level_w, mut level_h, mut offset) = (width, height, 0usize);
    while level_w.div_ceil(4) > ALBEDO_MAX_WIDTH {
        if !mips {
            return Err("colormap without mip chain is too large".into());
        }
        offset += level_size(level_w, level_h);
        level_w = (level_w / 2).max(1);
        level_h = (level_h / 2).max(1);
    }
    let (blocks_x, blocks_y) = (level_w.div_ceil(4), level_h.div_ceil(4));
    let end = offset + blocks_x * blocks_y * 8;
    let blocks = data.get(offset..end).ok_or("colormap mip out of range")?;
    let mut pixels = Vec::with_capacity(blocks_x * blocks_y * 4);
    for block in blocks.as_chunks::<8>().0 {
        let c0 = rgb565(u16::from_le_bytes([block[0], block[1]]));
        let c1 = rgb565(u16::from_le_bytes([block[2], block[3]]));
        for channel in 0..3 {
            pixels.push(((c0[channel] + c1[channel]) * 0.5).round() as u8);
        }
        pixels.push(255);
    }
    Ok(AlbedoPixels {
        width: blocks_x,
        height: blocks_y,
        data: pixels,
    })
}

// --- proxy mesh -------------------------------------------------------------------------------

/// Grid of the proxy: quads along x and z for about `triangles` triangles over a `width` x `depth`
/// map (square-ish cells).
fn grid_size(width: f32, depth: f32, triangles: usize) -> (usize, usize) {
    let quads = (triangles / 2).max(1) as f32;
    let nx = (quads * width / depth.max(1.0)).sqrt().round().max(1.0);
    let nz = (quads / nx).round().max(1.0);
    (nx as usize, nz as usize)
}

/// Terrain grid mesh: heights from `height_at(x, z)` (displayed units), normals by central
/// differences, UV0 over the whole map (for the colormap), tangents constant. Counter-clockwise
/// seen from above. Returns the mesh and its triangle count.
fn build_proxy_mesh(
    width: f32,
    depth: f32,
    triangles: usize,
    height_at: impl Fn(f32, f32) -> f32,
) -> (Mesh, usize) {
    let (nx, nz) = grid_size(width, depth, triangles);
    let (cols, rows) = (nx + 1, nz + 1);
    let (step_x, step_z) = (width / nx as f32, depth / nz as f32);
    let mut heights = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            heights.push(height_at(col as f32 * step_x, row as f32 * step_z));
        }
    }
    let at = |col: usize, row: usize| heights[row * cols + col];
    let mut positions = Vec::with_capacity(cols * rows);
    let mut normals = Vec::with_capacity(cols * rows);
    let mut uvs = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            positions.push([col as f32 * step_x, at(col, row), row as f32 * step_z]);
            let (left, right) = (col.saturating_sub(1), (col + 1).min(nx));
            let (up, down) = (row.saturating_sub(1), (row + 1).min(nz));
            let dx = (at(right, row) - at(left, row)) / ((right - left) as f32 * step_x);
            let dz = (at(col, down) - at(col, up)) / ((down - up) as f32 * step_z);
            normals.push(Vec3::new(-dx, 1.0, -dz).normalize().to_array());
            uvs.push([col as f32 / nx as f32, row as f32 / nz as f32]);
        }
    }
    let mut indices = Vec::with_capacity(nx * nz * 6);
    for row in 0..nz {
        for col in 0..nx {
            let a = (row * cols + col) as u32;
            let b = a + cols as u32;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    let count = positions.len();
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_TANGENT,
        vec![[1.0f32, 0.0, 0.0, 1.0]; count],
    )
    .with_inserted_indices(Indices::U32(indices));
    (mesh, nx * nz * 2)
}

/// Builds (or rebuilds when the exaggeration changed) the proxy once the terrain data is there.
#[allow(clippy::too_many_arguments)]
fn build_proxy(
    mut commands: Commands,
    settings: Res<CampaignSolariSettings>,
    mut state: ResMut<ProxyState>,
    mut stats: ResMut<ProxyStats>,
    surface: Option<Res<Surface>>,
    relief: Option<Res<ReliefState>>,
    terrain_handle: Option<Res<TerrainMaterialHandle>>,
    terrain_materials: Res<Assets<TerrainMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    proxies: Query<
        (
            &mut Mesh3d,
            &mut RaytracingMesh3d,
            &MeshMaterial3d<StandardMaterial>,
        ),
        With<TerrainProxy>,
    >,
) {
    let (Some(surface), Some(relief), Some(terrain_handle)) = (surface, relief, terrain_handle)
    else {
        return;
    };
    if !state.albedo_settled {
        return;
    }
    let scale = relief.vertical_scale;
    if state.entity.is_some() {
        let moved = (scale - state.built_scale).abs() > RESCALE_THRESHOLD * state.built_scale.abs();
        let waited = state
            .last_build
            .is_none_or(|at| at.elapsed().as_secs_f32() >= REBUILD_MIN_SECONDS);
        if !(moved && waited) {
            return;
        }
    }
    let Some(terrain) = terrain_materials.get(&terrain_handle.0) else {
        return;
    };
    let (width, depth) = (terrain.params.map.x, terrain.params.map.y);
    let edge_px = terrain.params.scale.w;
    let started = Instant::now();
    let (mesh, triangles) =
        build_proxy_mesh(width, depth, settings.terrain_proxy_triangles, |x, z| {
            let mut y = surface.display_height_at(&relief, x, z);
            if edge_px > 0.0 {
                // Same sink to sea level towards the map edge as the vertex shader.
                let to_edge = x.min(width - x).min(z.min(depth - z));
                let t = (to_edge / edge_px).clamp(0.0, 1.0);
                let sink = 1.0 - t * t * (3.0 - 2.0 * t);
                y += (y.min(0.2) - y) * sink;
            }
            y
        });
    let handle = meshes.add(mesh);
    state.built_scale = scale;
    state.last_build = Some(Instant::now());
    let build_ms = started.elapsed().as_secs_f32() * 1000.0;
    stats.proxy_triangles = triangles;
    stats.build_ms = build_ms;
    stats.rebuilds += 1;
    info!(
        "solari campaign: terrain proxy of {triangles} triangles built in {build_ms:.0} ms (vertical scale {scale:.4}, rebuild {})",
        stats.rebuilds
    );
    if let Some(entity) = state.entity {
        let mut proxies = proxies;
        if let Ok((mut mesh3d, mut raytracing, _)) = proxies.get_mut(entity) {
            mesh3d.0 = handle.clone();
            raytracing.0 = handle;
        }
        return;
    }
    let color = settings.flat_color;
    let material = materials.add(StandardMaterial {
        base_color: if state.albedo.is_some() {
            Color::WHITE
        } else {
            Color::srgb(color[0], color[1], color[2])
        },
        base_color_texture: state.albedo.clone(),
        perceptual_roughness: 0.95,
        reflectance: 0.04,
        opaque_render_method: OpaqueRendererMethod::Deferred,
        ..default()
    });
    let entity = commands
        .spawn((
            Name::new("SolariTerrainProxy"),
            TerrainProxy,
            Mesh3d(handle.clone()),
            RaytracingMesh3d(handle),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            // The whole map is one mesh with exact bounds; the quadtree culling does not apply.
            NoFrustumCulling,
            DespawnOnExit(AppState::Campaign),
        ))
        .id();
    state.entity = Some(entity);
}

/// Hides the CPU-meshed terrain nodes for good (the proxy stands in for them; before it exists the
/// map shows the clear colour rather than a forward terrain popping out).
fn hide_terrain_nodes(mut nodes: Query<&mut Visibility, With<MeshMaterial3d<TerrainMaterial>>>) {
    for mut visibility in &mut nodes {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
    }
}

// --- camera and lights ------------------------------------------------------------------------

fn light_camera(
    mut commands: Commands,
    settings: Res<CampaignSolariSettings>,
    mut stats: ResMut<ProxyStats>,
    mut images: ResMut<Assets<Image>>,
    cameras: Query<Entity, (With<CampaignCamera>, Without<SolariLighting>)>,
) {
    for entity in &cameras {
        // Cold sky (zenith, horizon, ground bounce): Solari reads no `AmbientLight`.
        let sky = EnvironmentMapLight::hemispherical_gradient(
            &mut images,
            Color::srgb(0.52, 0.64, 0.86),
            Color::srgb(0.78, 0.80, 0.82),
            Color::srgb(0.30, 0.30, 0.27),
        )
        .with_intensity(settings.sky_intensity);
        commands.entity(entity).insert((
            SolariLighting {
                restir: settings.restir,
                max_bounces: settings.max_bounces,
                ..default()
            },
            CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
            Msaa::Off,
            sky,
        ));
        stats.camera_deferred = true;
    }
}

/// Solari needs `Msaa::Off`; the quality settings would put the camera (and the UI camera, which
/// must match) back on the player's choice.
fn force_msaa_off(quality: Option<ResMut<RenderQuality>>) {
    if let Some(mut quality) = quality
        && quality.msaa_campaign != Msaa::Off
    {
        quality.msaa_campaign = Msaa::Off;
    }
}

/// Ray queries replace the shadow maps (the cascades would only cost).
fn sun_without_shadow_maps(mut suns: Query<&mut DirectionalLight, With<super::CampaignSun>>) {
    for mut sun in &mut suns {
        if sun.shadow_maps_enabled {
            sun.shadow_maps_enabled = false;
        }
    }
}

fn dump(world: &World) -> (String, Value) {
    let value = world.get_resource::<ProxyStats>().map_or_else(
        || json!({ "enabled": false, "reason": "not built" }),
        |stats| {
            json!({
                "enabled": stats.enabled,
                "reason": stats.reason,
                "proxy_triangles": stats.proxy_triangles,
                "build_ms": stats.build_ms,
                "rebuilds": stats.rebuilds,
                "camera_deferred": stats.camera_deferred,
                "albedo_texels": stats.albedo_texels,
            })
        },
    );
    ("solari_campaign".to_string(), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_follows_the_aspect_and_the_budget() {
        let (nx, nz) = grid_size(7000.0, 5000.0, 500_000);
        let triangles = nx * nz * 2;
        assert!((480_000..=520_000).contains(&triangles), "{triangles}");
        let ratio = (nx as f32 / nz as f32) / (7000.0 / 5000.0);
        assert!((0.97..=1.03).contains(&ratio), "{ratio}");
    }

    #[test]
    fn flat_proxy_faces_up_and_counts_triangles() {
        let (mesh, triangles) = build_proxy_mesh(100.0, 60.0, 2000, |_, _| 1.5);
        assert!((1900..=2100).contains(&triangles), "{triangles}");
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("u32 indices expected");
        };
        assert_eq!(indices.len(), triangles * 3);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        for triangle in indices.as_chunks::<3>().0 {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(positions[triangle[i] as usize]));
            assert!((b - a).cross(c - a).y > 0.0, "front face must look up");
        }
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals");
        };
        assert!(normals.iter().all(|n| (n[1] - 1.0).abs() < 1e-5));
    }

    #[test]
    fn slope_tilts_the_normals_and_uv_cover_the_map() {
        let (mesh, _) = build_proxy_mesh(40.0, 40.0, 800, |x, _| x * 0.5);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals");
        };
        assert!(normals.iter().all(|n| n[0] < 0.0 && n[1] > 0.0));
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("uvs");
        };
        let max_u = uvs.iter().map(|uv| uv[0]).fold(0.0, f32::max);
        let max_v = uvs.iter().map(|uv| uv[1]).fold(0.0, f32::max);
        assert!((max_u - 1.0).abs() < 1e-6 && (max_v - 1.0).abs() < 1e-6);
    }

    #[test]
    fn coarse_albedo_takes_one_mean_texel_per_block_of_the_first_small_mip() {
        // 4100 x 8 level 0 is 1025 blocks wide (above the cap), so the 2050 x 4 level 1 is read.
        let mut data = vec![0u8; level_size(4100, 8)];
        let red: u16 = 31 << 11;
        let blue: u16 = 31;
        for _ in 0..513 {
            data.extend_from_slice(&red.to_le_bytes());
            data.extend_from_slice(&blue.to_le_bytes());
            data.extend_from_slice(&[0; 4]);
        }
        let pixels = coarse_albedo(4100, 8, true, &data).expect("mip 1");
        assert_eq!((pixels.width, pixels.height), (513, 1));
        assert_eq!(&pixels.data[..4], &[128, 0, 128, 255]);
        assert!(coarse_albedo(4100, 8, false, &data).is_err());
    }
}
