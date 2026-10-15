//! Rendering and refresh of the province overlays: the colour fill (one flat textured quad at a
//! quarter of the raster resolution, rebuilt when the colours change) and the border ribbons
//! (`borders.rs`). Also the map mode requests and the Diplomacy-mode focus.
//!
//! Overlays use the Godot world convention (`projection.rs`): flat at `Y = fill_height` /
//! `border_height` (plus the terrain height under the borders when a `SurfaceHeight` exists).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::borders::{self, BorderCounts, BorderStyle, Edge, smoothstep};
use super::data::ProvinceMap;
use super::labels::camera_ground_distance;
use super::modes::{self, MapMode, MapModeState, ProvinceView, SetMapMode, Tuning, num};
use super::picker::{HoveredProvince, ProvinceClicked, SelectedProvince, SurfaceHeight};
use crate::app::AppState;
use crate::sim::Sim;

/// The fill texture covers the map at `1 / FILL_DOWNSCALE` of the raster.
pub const FILL_DOWNSCALE: u32 = 4;
/// Refresh the simulation view at most this often (seconds).
const REFRESH_PERIOD_S: f32 = 0.25;

/// Heights and widths of the overlays (world units; one unit = one map pixel).
#[derive(Resource, Clone, Debug)]
pub struct ProvinceOverlayConfig {
    pub fill_height: f32,
    pub border_height: f32,
    /// World size of one screen pixel of the 1080p border tuning.
    pub border_world_per_px: f32,
}

impl Default for ProvinceOverlayConfig {
    fn default() -> Self {
        Self {
            fill_height: 0.05,
            border_height: 0.08,
            border_world_per_px: 3.0,
        }
    }
}

/// Rendered state, for the dump and for change detection.
#[derive(Resource)]
pub struct OverlayState {
    pub edges: Vec<Edge>,
    pub view: ProvinceView,
    pub heraldic: HashMap<String, Srgba>,
    pub counts: BorderCounts,
    pub colors: Vec<Srgba>,
    pub fill_pixels: (u32, u32),
    pub fill_rebuilds: u32,
    pub border_rebuilds: u32,
    pub border_alpha: f32,
    pub fill_alpha: f32,
    fill_image: Handle<Image>,
    fill_material: Handle<StandardMaterial>,
    border_mesh: Handle<Mesh>,
    border_material: Handle<StandardMaterial>,
    border_key: Option<BorderKey>,
    view_key: Option<(MapMode, String)>,
    since_refresh: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct BorderKey {
    mode: MapMode,
    focus: [String; 2],
    view_generation: u32,
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<MapModeState>()
        .init_resource::<ProvinceOverlayConfig>()
        .add_message::<SetMapMode>()
        .add_systems(OnEnter(AppState::Campaign), load_province_map)
        .add_systems(
            Update,
            (
                apply_mode_requests,
                focus_on_click,
                refresh_overlays,
                fade_with_zoom,
            )
                .chain()
                .run_if(resource_exists::<ProvinceMap>)
                .run_if(resource_exists::<OverlayState>),
        );
}

fn load_province_map(
    mut commands: Commands,
    existing: Option<Res<ProvinceMap>>,
    config: Res<ProvinceOverlayConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut exit: MessageWriter<AppExit>,
) {
    if existing.is_some() {
        return;
    }
    let map_dir = crate::paths::find_root().map(|root| crate::paths::data_dir(&root).join("map"));
    let map = match map_dir.clone().and_then(|dir| ProvinceMap::load(&dir)) {
        Ok(map) => map,
        Err(message) => {
            error!("AGENT_FATAL: province map: {message}");
            exit.write(AppExit::error());
            return;
        }
    };
    let tuning = Tuning::load(&map_dir.unwrap_or_default());
    let edges = borders::extract_edges(&map);
    info!(
        "province map: {} provinces, {} border edges",
        map.provinces().len(),
        edges.len()
    );

    let (fw, fh) = (
        map.width.div_ceil(FILL_DOWNSCALE),
        map.height.div_ceil(FILL_DOWNSCALE),
    );
    let mut fill = Image::new(
        Extent3d {
            width: fw,
            height: fh,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; (fw * fh * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    fill.sampler = ImageSampler::nearest();
    let fill_image = images.add(fill);
    let overlay_material = |texture: Option<Handle<Image>>| StandardMaterial {
        base_color_texture: texture,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    };
    let fill_material = materials.add(overlay_material(Some(fill_image.clone())));
    let border_material = materials.add(overlay_material(None));
    let border_mesh = meshes.add(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    ));
    let fill_mesh = meshes.add(quad_mesh(
        map.width as f32,
        map.height as f32,
        config.fill_height,
    ));

    commands.spawn((
        Name::new("ProvinceFill"),
        DespawnOnExit(AppState::Campaign),
        Mesh3d(fill_mesh),
        MeshMaterial3d(fill_material.clone()),
        Transform::default(),
    ));
    commands.spawn((
        Name::new("ProvinceBorders"),
        DespawnOnExit(AppState::Campaign),
        Mesh3d(border_mesh.clone()),
        MeshMaterial3d(border_material.clone()),
        Transform::default(),
    ));
    commands.insert_resource(OverlayState {
        edges,
        view: ProvinceView::default(),
        heraldic: HashMap::new(),
        counts: BorderCounts::default(),
        colors: Vec::new(),
        fill_pixels: (fw, fh),
        fill_rebuilds: 0,
        border_rebuilds: 0,
        border_alpha: 0.0,
        fill_alpha: 0.0,
        fill_image,
        fill_material,
        border_mesh,
        border_material,
        border_key: None,
        view_key: None,
        since_refresh: f32::INFINITY,
    });
    commands.insert_resource(tuning);
    commands.insert_resource(map);
}

/// Flat quad `0..width x 0..depth` at `height`, UV over the whole map.
fn quad_mesh(width: f32, depth: f32, height: f32) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [0.0, height, 0.0],
            [width, height, 0.0],
            [width, height, depth],
            [0.0, height, depth],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]));
    mesh
}

/// RGBA8 pixels of the fill texture: raster sampled at the centre of each block, `colors` by index.
pub fn paint_fill(map: &ProvinceMap, colors: &[Srgba], width: u32, height: u32, out: &mut [u8]) {
    let table: Vec<[u8; 4]> = colors.iter().map(|c| c.to_u8_array()).collect();
    let half = FILL_DOWNSCALE / 2;
    for y in 0..height {
        let sy = (y * FILL_DOWNSCALE + half).min(map.height - 1);
        for x in 0..width {
            let sx = (x * FILL_DOWNSCALE + half).min(map.width - 1);
            let index = usize::from(map.raster()[(sy * map.width + sx) as usize]);
            let px = table.get(index).copied().unwrap_or([0; 4]);
            let at = ((y * width + x) * 4) as usize;
            out[at..at + 4].copy_from_slice(&px);
        }
    }
}

fn apply_mode_requests(mut requests: MessageReader<SetMapMode>, mut state: ResMut<MapModeState>) {
    for SetMapMode(mode) in requests.read() {
        if state.mode != *mode {
            state.mode = *mode;
            if *mode != MapMode::Diplomacy {
                state.focus_faction = None;
            }
        }
    }
}

/// Diplomacy mode: a click on a province makes its controller the viewed faction; our own lands,
/// rebels or the sea go back to the player (`focus_on_province`).
fn focus_on_click(
    mut clicks: MessageReader<ProvinceClicked>,
    overlay: Res<OverlayState>,
    sim: Res<Sim>,
    mut state: ResMut<MapModeState>,
) {
    for click in clicks.read() {
        if click.button != MouseButton::Left || state.mode != MapMode::Diplomacy {
            continue;
        }
        let controller = overlay.view.controller_of(click.index);
        let focus = (!controller.is_empty()
            && controller != sim.player.as_str()
            && controller != modes::REBELS_FACTION)
            .then(|| controller.to_owned());
        if state.focus_faction != focus {
            state.focus_faction = focus;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn refresh_overlays(
    time: Res<Time>,
    sim: Res<Sim>,
    map: Res<ProvinceMap>,
    tuning: Res<Tuning>,
    config: Res<ProvinceOverlayConfig>,
    mode: Res<MapModeState>,
    hovered: Res<HoveredProvince>,
    selected: Res<SelectedProvince>,
    surface: Option<Res<SurfaceHeight>>,
    mut overlay: ResMut<OverlayState>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut generation: Local<u32>,
) {
    overlay.since_refresh += time.delta_secs();
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    let viewer_name = mode
        .focus_faction
        .clone()
        .unwrap_or_else(|| sim.player.as_str().to_owned());
    let view_key = (mode.mode, viewer_name.clone());
    let mode_changed = overlay.view_key.as_ref() != Some(&view_key);
    if !mode_changed && overlay.since_refresh < REFRESH_PERIOD_S && overlay.border_key.is_some() {
        // Only the hover / selection can change the borders between simulation reads.
        let focus = focus_owners(&overlay.view, &hovered, &selected);
        if overlay
            .border_key
            .as_ref()
            .is_some_and(|k| k.focus == focus)
        {
            return;
        }
    }
    let viewer = data_model::FactionId::new(viewer_name).unwrap_or_else(|_| sim.player.clone());
    let with_lens = matches!(
        mode.mode,
        MapMode::Unrest | MapMode::Wealth | MapMode::Population
    );
    let mut refreshed = false;
    if mode_changed || overlay.since_refresh >= REFRESH_PERIOD_S {
        overlay.since_refresh = 0.0;
        let view = modes::read_view(&map, data, state, &viewer, with_lens);
        if view != overlay.view || mode_changed {
            overlay.view = view;
            if overlay.heraldic.is_empty() {
                overlay.heraldic = modes::heraldic_colors(data);
            }
            *generation += 1;
            refreshed = true;
        }
        overlay.view_key = Some(view_key);
    }
    if refreshed {
        let colors = modes::province_colors(
            mode.mode,
            map.max_index(),
            &overlay.view,
            &overlay.heraldic,
            &tuning,
        );
        if colors != overlay.colors {
            let (fw, fh) = overlay.fill_pixels;
            if let Some(mut image) = images.get_mut(&overlay.fill_image) {
                let mut data = image.data.take().unwrap_or_default();
                data.resize((fw * fh * 4) as usize, 0);
                paint_fill(&map, &colors, fw, fh, &mut data);
                image.data = Some(data);
            }
            overlay.colors = colors;
            overlay.fill_rebuilds += 1;
        }
    }
    let key = BorderKey {
        mode: mode.mode,
        focus: focus_owners(&overlay.view, &hovered, &selected),
        view_generation: *generation,
    };
    if overlay.border_key.as_ref() != Some(&key) {
        let style = BorderStyle {
            mode: mode.mode,
            focus_owners: key.focus.clone(),
            world_per_px: config.border_world_per_px,
        };
        let (border_height, surface_fn) = (
            config.border_height,
            surface.as_ref().map(|s| Arc::clone(&s.0)),
        );
        let lift = move |p: Vec2| {
            surface_fn.as_ref().map_or(0.0, |f| f(p.x, p.y).max(0.0)) + border_height
        };
        let (mesh, counts) = borders::build_mesh(
            &overlay.edges,
            &overlay.view,
            &overlay.heraldic,
            &tuning,
            &style,
            &lift,
        );
        if let Some(mut slot) = meshes.get_mut(&overlay.border_mesh) {
            *slot = mesh;
        }
        overlay.counts = counts;
        overlay.border_key = Some(key);
        overlay.border_rebuilds += 1;
    }
}

/// Owners of the hovered and selected provinces (empty string for none).
fn focus_owners(
    view: &ProvinceView,
    hovered: &HoveredProvince,
    selected: &SelectedProvince,
) -> [String; 2] {
    [
        view.owner_of(hovered.index).to_owned(),
        view.owner_of(selected.index).to_owned(),
    ]
}

/// Border and wash opacity from the mode and the camera distance (`update_view`).
#[allow(clippy::too_many_arguments)]
fn fade_with_zoom(
    mode: Res<MapModeState>,
    tuning: Res<Tuning>,
    map: Res<ProvinceMap>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut overlay: ResMut<OverlayState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let distance = cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .and_then(|(c, t)| camera_ground_distance(c, t));
    let (mode_alpha, _) = modes::mode_border_style(mode.mode, &tuning);
    let mpp = map.projection.meters_per_px() as f32;
    let border =
        mode_alpha * distance.map_or(1.0, |d| borders::zoom_alpha(d, mpp, &tuning.borders));
    let near = num(&tuning.fill, &["zoom", "near_distance"], 60.0);
    let far = num(&tuning.fill, &["zoom", "far_distance"], 260.0).max(near + 1.0);
    let near_scale = num(&tuning.fill, &["zoom", "near_scale"], 1.0);
    let fill = distance.map_or(1.0, |d| {
        near_scale + (1.0 - near_scale) * smoothstep(near, far, d)
    });
    if (border - overlay.border_alpha).abs() > 1e-3 {
        overlay.border_alpha = border;
        if let Some(mut m) = materials.get_mut(&overlay.border_material) {
            m.base_color = Color::srgba(1.0, 1.0, 1.0, border);
        }
    }
    if (fill - overlay.fill_alpha).abs() > 1e-3 {
        overlay.fill_alpha = fill;
        if let Some(mut m) = materials.get_mut(&overlay.fill_material) {
            m.base_color = Color::srgba(1.0, 1.0, 1.0, fill);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::provinces::data::tests::real_map;

    #[test]
    fn fill_paints_each_province_with_its_colour() {
        let map = real_map();
        let paris = map.index_of_id("prov_ile_de_france");
        let mut colors = vec![Srgba::NONE; usize::from(map.max_index()) + 1];
        colors[usize::from(paris)] = Srgba::new(1.0, 0.0, 0.0, 1.0);
        let (fw, fh) = (
            map.width.div_ceil(FILL_DOWNSCALE),
            map.height.div_ceil(FILL_DOWNSCALE),
        );
        let mut pixels = vec![0u8; (fw * fh * 4) as usize];
        paint_fill(map, &colors, fw, fh, &mut pixels);
        let px = map.projection.lonlat_to_px(2.35, 48.85).unwrap();
        let at =
            (((px.y as u32 / FILL_DOWNSCALE) * fw + px.x as u32 / FILL_DOWNSCALE) * 4) as usize;
        assert_eq!(&pixels[at..at + 4], &[255, 0, 0, 255]);
        let red = pixels.chunks(4).filter(|p| p[3] == 255).count();
        assert!(red > 100, "{red} red pixels");
        assert_eq!(&pixels[0..4], &[0, 0, 0, 0]);
    }
}
