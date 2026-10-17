//! Streaming and display of the relief quadtree: glue between the `relief-lod` crate (CDLOD node
//! selection and page residency, used directly) and Bevy entities. Port of the Godot side of
//! `relief_quadtree.gd` (`update_view`, `_apply_native`, page jobs and LRU), with CPU meshes
//! instead of a shader-displaced shared patch (see `mesh.rs`).

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use relief_lod::{Config, Selector, V3, View};

use super::decode::TileDecoder;
use super::material::TerrainMaterial;
use super::mesh::{NodeSpec, build_node_mesh, patch_counts};
use super::pyramid::ReliefPyramid;
use super::relief_floor::HeightGrid;
use super::relief_state::ReliefState;
use super::surface::Surface;
use crate::app::AppState;
use crate::campaign::camera::CampaignCamera;
use crate::campaign::camera::rig::CampaignRig;

/// Resident pages at most (`max_pages` of `relief_quadtree.gd`).
pub const MAX_PAGES: usize = 256;
/// Tiles decoded at once on the worker threads (`max_jobs`).
pub const MAX_JOBS: usize = 4;
/// Time budget for building node meshes each frame.
pub const BUILD_BUDGET_MS: f32 = 6.0;
/// A node left by the selector stays drawn this long at most while its replacements build.
pub const RETIRE_MAX_S: f64 = 2.0;

/// A displayed node.
struct Node {
    entity: Entity,
    spec: NodeSpec,
    /// Page of the last built mesh (`i64::MIN`: not built yet).
    built_fine: i64,
    triangles: usize,
}

impl Node {
    /// Footprint (min x, min z, max x, max z) in map units.
    fn rect(&self) -> (f32, f32, f32, f32) {
        let extent = self.spec.quads as f32 * self.spec.spacing;
        let (x, z) = self.spec.origin;
        (x, z, x + extent, z + extent)
    }
}

/// Two footprints share some area (touching edges do not count).
fn overlaps(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

#[derive(Resource)]
pub struct TerrainLod {
    selector: Selector,
    decoder: TileDecoder,
    pub material: Handle<TerrainMaterial>,
    nodes: HashMap<i64, Node>,
    /// Nodes the selector dropped, kept on screen (with the time they left) until the nodes
    /// replacing them are built: despawning them at once opened black holes while zooming.
    retiring: Vec<(Node, f64)>,
    requested: HashSet<i64>,
    failed: HashSet<i64>,
    pub config: Config,
    pub frame: i64,
    pub selected_items: usize,
    pub px_scale: f64,
    pub missing_pages: i64,
    pub meshes_built: u64,
    pub pages_loaded: u64,
    pub build_ms_last: f32,
    pub pending_builds: usize,
    /// Peak of nodes drawn at once.
    pub peak_nodes: usize,
}

impl TerrainLod {
    /// Displayed patches: the spec of each built node mesh, with the page it was built on.
    pub fn built_specs(&self) -> impl Iterator<Item = NodeSpec> + '_ {
        self.nodes
            .values()
            .filter(|n| n.built_fine != i64::MIN)
            .map(|n| NodeSpec {
                fine: n.built_fine,
                ..n.spec
            })
    }

    pub fn new(
        pyramid: &ReliefPyramid,
        heights: &HeightGrid,
        material: Handle<TerrainMaterial>,
    ) -> TerrainLod {
        let bounds =
            heights.chunk_bounds(256, pyramid.root_cols as usize, pyramid.root_rows as usize);
        TerrainLod {
            selector: Selector::new(pyramid.to_lod(), bounds),
            decoder: TileDecoder::start(MAX_JOBS),
            material,
            nodes: HashMap::new(),
            retiring: Vec::new(),
            requested: HashSet::new(),
            failed: HashSet::new(),
            config: Config::default(),
            frame: 0,
            selected_items: 0,
            px_scale: 1.0,
            missing_pages: 0,
            meshes_built: 0,
            pages_loaded: 0,
            build_ms_last: 0.0,
            pending_builds: 0,
            peak_nodes: 0,
        }
    }

    /// Tells the selector about pages that stayed resident from a previous campaign visit.
    pub fn adopt_pages(&mut self, surface: &Surface, now: f64) {
        for key in surface.page_keys() {
            self.selector.add_page(key, 0, now - 10.0, 0);
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Nodes whose mesh exists.
    pub fn built_nodes(&self) -> usize {
        self.nodes
            .values()
            .filter(|n| n.built_fine != i64::MIN)
            .count()
    }

    pub fn triangles(&self) -> usize {
        self.nodes.values().map(|n| n.triangles).sum()
    }

    pub fn in_flight(&self) -> usize {
        self.requested.len()
    }

    /// True when every wanted page is resident and nothing is being decoded or built.
    pub fn is_settled(&self) -> bool {
        self.requested.is_empty() && self.missing_pages == 0 && self.pending_builds == 0
    }
}

#[allow(clippy::too_many_arguments)]
/// Outward-normal frustum planes (`Camera3D.get_frustum` convention: a point is outside when
/// `n . p - d > 0`) of a perspective camera.
pub fn frustum_planes(
    eye: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    fov_y: f32,
    aspect: f32,
    near: f32,
    far: f32,
) -> Vec<(V3, f32)> {
    let ty = (fov_y * 0.5).tan();
    let tx = ty * aspect;
    let plane = |outward: Vec3| {
        let n = outward.normalize();
        (V3::new(n.x, n.y, n.z), n.dot(eye))
    };
    vec![
        plane(-(right + forward * tx)),
        plane(right - forward * tx),
        plane(-(-up + forward * ty)),
        plane(-(up + forward * ty)),
        (
            V3::new(-forward.x, -forward.y, -forward.z),
            -forward.dot(eye) - near,
        ),
        (
            V3::new(forward.x, forward.y, forward.z),
            forward.dot(eye) + far,
        ),
    ]
}

/// Builds the selector view of a camera.
#[allow(clippy::too_many_arguments)]
pub fn make_view(
    transform: &Transform,
    fov_y: f32,
    aspect: f32,
    near: f32,
    far: f32,
    viewport_height: f32,
    relief: &ReliefState,
    now: f64,
    frame: i64,
) -> View {
    let scale = relief.vertical_scale;
    let eye = transform.translation;
    View {
        cam: V3::new(eye.x, eye.y, eye.z),
        planes: frustum_planes(
            eye,
            *transform.right(),
            *transform.up(),
            *transform.forward(),
            fov_y,
            aspect,
            near,
            far,
        ),
        k: viewport_height as f64 * 0.5 / (fov_y as f64 * 0.5).tan(),
        vertical_scale: scale as f64,
        up: relief.up_factor(scale) as f64,
        down: relief.down_factor(scale) as f64,
        lift: relief.massif_max(scale) as f64,
        now,
        shadow_cast_distance: f64::INFINITY,
        frame,
    }
}

/// Per-frame streaming: selection, page requests and arrivals, node entities and meshes.
#[allow(clippy::too_many_arguments)]
pub fn update_lod(
    mut commands: Commands,
    time: Res<Time>,
    rig: Res<CampaignRig>,
    mut lod: ResMut<TerrainLod>,
    mut surface: ResMut<Surface>,
    mut relief: ResMut<ReliefState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    cameras: Query<(&Transform, &Projection, &Camera), With<CampaignCamera>>,
) {
    // Dynamic vertical exaggeration, quantised with hysteresis (`TerrainBuilder.set_vertical_scale`).
    let scale = relief
        .camera
        .quantized_scale(rig.distance, relief.vertical_scale, &relief.relief);
    // Mutating through `ResMut` would flag `ReliefState` as changed every frame and make every
    // reader re-place its objects: write through the bypass and flag only a real change.
    let scale_changed = relief.bypass_change_detection().set_vertical_scale(scale);
    if scale_changed {
        relief.set_changed();
    }
    if scale_changed && let Some(mut material) = materials.get_mut(&lod.material) {
        material.params.scale.x = relief.vertical_scale;
        material.params.scale.y = relief.gain;
        material.params.scale.z = relief.squash;
    }

    let Ok((transform, projection, camera)) = cameras.single() else {
        return;
    };
    let Projection::Perspective(perspective) = projection else {
        return;
    };
    let viewport = camera
        .physical_target_size()
        .map_or(Vec2::new(1280.0, 800.0), |s| s.as_vec2());
    let aspect = viewport.x / viewport.y.max(1.0);
    let now = time.elapsed_secs_f64();
    lod.frame += 1;

    // Pages decoded by the workers: residency (LRU) and notification of the selector.
    let frame = lod.frame;
    for done in lod.decoder.poll(8) {
        lod.requested.remove(&done.key);
        match done.result {
            Ok(image) if image.data.len() == super::pyramid::TILE_PX * super::pyramid::TILE_PX => {
                if surface.page_count() >= MAX_PAGES {
                    let oldest = lod.selector.oldest_page(frame - 1);
                    if oldest >= 0 {
                        lod.selector.remove_page(oldest);
                        surface.remove_page(oldest);
                    }
                }
                if surface.page_count() < MAX_PAGES {
                    surface.insert_page(done.key, std::sync::Arc::new(image.data));
                    lod.selector.add_page(done.key, 0, now, frame);
                    lod.pages_loaded += 1;
                }
            }
            Ok(_) | Err(_) => {
                warn!(
                    "relief: unreadable tile {}",
                    surface.pyramid.tile_path_of_key(done.key).display()
                );
                lod.selector.mark_broken(done.key);
                lod.failed.insert(done.key);
            }
        }
    }

    // CDLOD selection.
    let view = make_view(
        transform,
        perspective.fov,
        aspect,
        perspective.near,
        perspective.far,
        viewport.y,
        &relief,
        now,
        frame,
    );
    let config = lod.config;
    let update = lod.selector.update(&view, config);
    lod.selected_items = update.items;
    lod.px_scale = update.px_scale;
    lod.missing_pages = update.missing;

    // Pages wanted by priority, a few jobs at a time.
    let wanted: Vec<i64> = update.wanted.clone();
    for key in wanted {
        if lod.requested.len() >= MAX_JOBS {
            break;
        }
        if lod.requested.contains(&key) || lod.failed.contains(&key) || surface.page(key).is_some()
        {
            continue;
        }
        let path = surface.pyramid.tile_path_of_key(key);
        if lod.decoder.request(key, path) {
            lod.requested.insert(key);
        }
    }

    // Nodes removed (kept until replaced, see `retiring`) and added.
    for key in &update.removed {
        if let Some(node) = lod.nodes.remove(key) {
            if node.built_fine == i64::MIN {
                commands.entity(node.entity).despawn();
            } else {
                lod.retiring.push((node, now));
            }
        }
    }
    let material = lod.material.clone();
    for added in &update.added {
        let skirt =
            (added.spacing as f64 * config.skirt_factor).min(config.skirt_max) as f32 + 0.02;
        let spec = NodeSpec {
            origin: added.origin,
            spacing: added.spacing,
            quads: added.quads as u32,
            fine: -1,
            skirt,
        };
        let extent = added.quads * added.spacing;
        // Raw metres are in the y of the positions: the automatic bounding box would be wrong
        // (and too loose), so the node declares its own: any displayed height fits in -10..500.
        let entity = commands
            .spawn((
                Name::new("TerrainNode"),
                Transform::IDENTITY,
                Visibility::Hidden,
                Aabb::from_min_max(
                    Vec3::new(added.origin.0, -10.0, added.origin.1),
                    Vec3::new(added.origin.0 + extent, 500.0, added.origin.1 + extent),
                ),
                MeshMaterial3d(material.clone()),
                // Bevy's CPU frustum test culled whole interior nodes (holes onto the abyss plane at
                // close range, offscreen and windowed): the selector already does the culling.
                bevy::camera::visibility::NoFrustumCulling,
                DespawnOnExit(AppState::Campaign),
            ))
            .id();
        lod.nodes.insert(
            added.key,
            Node {
                entity,
                spec,
                built_fine: i64::MIN,
                triangles: 0,
            },
        );
    }
    lod.peak_nodes = lod.peak_nodes.max(lod.nodes.len());

    // Which nodes need a (re)build: new ones, or whose page changed. Coarse and near first.
    let mut todo: Vec<(i32, f64, i64, i64)> = Vec::new();
    for item in lod.selector.items() {
        if let Some(node) = lod.nodes.get(&item.key)
            && node.built_fine != item.fine
        {
            todo.push((item.n, item.dist, item.key, item.fine));
        }
    }
    todo.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    lod.pending_builds = todo.len();
    let started = Instant::now();
    let mut built = 0usize;
    for (_, _, key, fine) in todo {
        if built > 0 && started.elapsed().as_secs_f32() * 1000.0 > BUILD_BUDGET_MS {
            break;
        }
        let Some(node) = lod.nodes.get_mut(&key) else {
            continue;
        };
        node.spec.fine = fine;
        let mesh = build_node_mesh(&node.spec, &surface, &relief);
        node.built_fine = fine;
        node.triangles = patch_counts(node.spec.quads).1;
        commands
            .entity(node.entity)
            .insert((Mesh3d(meshes.add(mesh)), Visibility::Visible));
        built += 1;
    }
    lod.pending_builds = lod.pending_builds.saturating_sub(built);

    // Retired nodes go once no unbuilt node overlaps them any more (or after a delay).
    let unbuilt: Vec<_> = lod
        .nodes
        .values()
        .filter(|n| n.built_fine == i64::MIN)
        .map(Node::rect)
        .collect();
    let retiring = std::mem::take(&mut lod.retiring);
    for (node, since) in retiring {
        let waiting = unbuilt.iter().any(|r| overlaps(*r, node.rect()));
        if waiting && now - since < RETIRE_MAX_S {
            lod.retiring.push((node, since));
        } else {
            commands.entity(node.entity).despawn();
        }
    }
    lod.meshes_built += built as u64;
    lod.build_ms_last = started.elapsed().as_secs_f32() * 1000.0;
}

#[cfg(test)]
mod tests {
    #[test]
    fn footprints_overlap_only_when_sharing_area() {
        let parent = (0.0, 0.0, 64.0, 64.0);
        assert!(super::overlaps(parent, (32.0, 32.0, 64.0, 64.0)));
        assert!(!super::overlaps(parent, (64.0, 0.0, 128.0, 64.0)));
        assert!(!super::overlaps(parent, (100.0, 100.0, 110.0, 110.0)));
    }
    use super::*;

    #[test]
    fn frustum_contains_the_view_axis_and_rejects_behind() {
        let eye = Vec3::new(100.0, 50.0, 100.0);
        let transform =
            Transform::from_translation(eye).looking_at(Vec3::new(100.0, 0.0, 0.0), Vec3::Y);
        let planes = frustum_planes(
            eye,
            *transform.right(),
            *transform.up(),
            *transform.forward(),
            55f32.to_radians(),
            1.6,
            1.0,
            6000.0,
        );
        let inside = |p: Vec3| {
            planes
                .iter()
                .all(|(n, d)| n.x * p.x + n.y * p.y + n.z * p.z - d <= 0.0)
        };
        assert!(inside(Vec3::new(100.0, 0.0, 0.0)), "target in view");
        assert!(inside(eye + *transform.forward() * 20.0));
        assert!(!inside(eye - *transform.forward() * 20.0), "behind");
        assert!(
            !inside(eye + *transform.forward() * 20.0 + *transform.right() * 200.0),
            "far right"
        );
        assert!(!inside(eye + *transform.forward() * 7000.0), "past far");
    }

    #[test]
    fn selector_selects_nodes_and_wants_pages_for_a_camera_over_the_map() {
        // Synthetic pyramid: 4 x 4 E0 tiles, no finer data.
        let e0: Vec<i64> = (0..16).collect();
        let pyramid = relief_lod::Pyramid::new(0, vec![e0], 4, 4);
        let mut selector = Selector::new(pyramid, vec![(-10.0, 100.0); 16]);
        let eye = Vec3::new(512.0, 200.0, 512.0);
        let transform =
            Transform::from_translation(eye).looking_at(Vec3::new(512.0, 0.0, 400.0), Vec3::Y);
        let relief = ReliefState::new(
            super::super::profile::ReliefProfile::from_tres(&super::super::tres::Tres::default()),
            super::super::profile::CloseCameraProfile::from_tres(
                &super::super::tres::Tres::default(),
            ),
        );
        let view = make_view(
            &transform,
            55f32.to_radians(),
            1.6,
            1.0,
            6000.0,
            800.0,
            &relief,
            1.0,
            1,
        );
        let update = selector.update(&view, Config::default());
        assert!(update.items > 0, "some nodes selected");
        assert_eq!(update.added.len(), update.items);
        assert!(!update.wanted.is_empty(), "pages wanted");
        // A second identical frame changes nothing.
        let again = selector.update(
            &view,
            Config {
                ..Config::default()
            },
        );
        assert!(again.added.is_empty() && again.removed.is_empty());
        // Every added node declares a patch of 64 or 32 quads.
        assert!(
            update
                .added
                .iter()
                .all(|a| a.quads == 64.0 || a.quads == 32.0)
        );
    }
}
