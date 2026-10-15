//! Settlements, towns and landmarks on the campaign map: stylised maquettes (ADR 0158), landmark
//! cities (Meshy glb, ADR 0356), name labels with screen declutter, ruin and worksite signs.
//!
//! Replaces game/scripts/map/: settlement_markers.gd, settlement_data.gd, settlement_layer.gd
//! (labels), town_maquette_layer.gd / town_maquette_data.gd (the maquettes of ADR 0158 that
//! replaced town_far_*), landmark_city_layer.gd / landmark_library.gd / landmark_model.gd,
//! construction_markers.gd, ruin_markers.gd, marker_declutter.gd, screen_signs.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.
//!
//! World convention (same as the Godot map): 1 unit = 1 map pixel (719 m), `x = px.x`,
//! `z = px.y`, y up; ground height from `campaign::terrain::surface::GroundHeight` (displayed
//! relief, re-applied when `ReliefState` changes).

#![allow(clippy::type_complexity)]

pub mod catalog;
pub mod declutter;
pub mod placement;
pub mod places;
pub mod signs;

use bevy::asset::LoadState;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::provinces::picker::SelectedProvince;
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::GroundHeight;
use crate::campaign::towns::TownsState;
use crate::paths;
use crate::sim::Sim;
use crate::ui::kit::UiTheme;

use self::catalog::{KINDS, Maquettes, MarkerCatalog};
use self::places::{Places, build_places, count_by_kind, load_landmarks};
use self::signs::{RuinSign, WorksiteSign};

pub struct CampaignMarkersPlugin;

impl Plugin for CampaignMarkersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Places>()
            .init_resource::<ShieldCache>()
            .init_resource::<MarkerStats>()
            .init_resource::<MarkersBuilt>()
            .init_resource::<Pins>()
            .register_dump(dump_markers)
            .add_systems(OnExit(AppState::Campaign), reset_markers)
            .add_systems(
                Update,
                (
                    build_markers,
                    (
                        refresh_controllers,
                        update_pins,
                        sink_maquettes,
                        refresh_signs,
                        settle_ground,
                        update_range,
                        update_shields,
                        update_labels,
                        signs::rescale_signs,
                        landmark_fallback,
                        update_stats,
                    )
                        .run_if(|built: Res<MarkersBuilt>| built.0),
                )
                    .chain()
                    .run_if(in_state(AppState::Campaign)),
            );
    }
}

#[derive(Resource, Default)]
struct MarkersBuilt(bool);

/// Counters for the `campaign_markers` dump section.
#[derive(Resource, Default)]
pub struct MarkerStats {
    pub settlements: usize,
    pub maquettes_by_kind: Vec<(String, usize)>,
    pub maquettes_by_family: Vec<(String, usize)>,
    pub family_fallbacks: usize,
    pub absorbed: usize,
    pub landmarks_planned: usize,
    pub landmarks_loaded: usize,
    pub landmarks_failed: usize,
    pub landmarks_fallback: usize,
    pub maquettes_loaded: usize,
    pub maquettes_failed: usize,
    pub maquettes_visible: usize,
    /// Maquettes fully sunk under their shown 1:1 town.
    pub maquettes_sunk: usize,
    pub pinned: usize,
    pub labels: usize,
    pub labels_visible: usize,
    pub grounded: usize,
    /// Probe of loaded scenes: (name, world size, world base y, anchor y).
    pub probes: Vec<(String, Vec3, f32, f32)>,
    pub ground_min: f32,
    pub ground_max: f32,
    pub ruins: usize,
    pub worksites: usize,
    pub controller_changes: u64,
    pub catalog_valid: bool,
    pub build_ms: u128,
}

#[derive(Component)]
struct MarkersRoot;

/// Map position of an object standing on the ground: `settle_ground` keeps its `y` at the
/// displayed relief height plus `lift`.
#[derive(Component)]
pub struct GroundAnchor {
    pub px: Vec2,
    pub lift: f32,
}

/// A maquette: index into `Places`, its range (rig distance beyond which it is hidden).
#[derive(Component)]
struct Maquette {
    place: usize,
    /// Uniform scale of the kit model before any sinking.
    base_scale: f32,
    /// Fully sunk under its 1:1 town (set by `sink_maquettes`).
    sunk: bool,
    range: f32,
    handle: Handle<WorldAsset>,
}

/// A landmark city scene waiting for its width fit; falls back to the flat Blender maquette when
/// the generated glb fails to load.
#[derive(Component)]
struct LandmarkScene {
    target_width: f32,
    handle: Handle<WorldAsset>,
    fallback: Option<String>,
    used_fallback: bool,
}

#[derive(Component)]
struct PlaceLabel {
    index: usize,
}

/// Holder's shield image of a label; `controller` is the faction currently shown.
#[derive(Component)]
struct LabelShield {
    controller: String,
    index: usize,
}

/// Places always shown by the declutter (settings `declutter.pin_*`): the selected place and the
/// player's capital, in that order. Indices into `Places::places`.
#[derive(Resource, Default)]
pub struct Pins {
    pub indices: Vec<usize>,
}

/// Small shield of the occupant, bottom right of the owner's shield (RJ-c).
#[derive(Component)]
struct OccupantShield;

/// Heraldry images by faction (`None` = no armoury file).
#[derive(Resource, Default)]
struct ShieldCache(std::collections::HashMap<String, Option<Handle<Image>>>);

/// Ground height under a map point, 0 until the terrain exists.
fn ground_y(ground: &GroundHeight, px: Vec2) -> f32 {
    ground.at(px.x, px.y).unwrap_or(0.0)
}

type WorldAsset = bevy::world_serialization::WorldAsset;

fn dump_markers(world: &World) -> (String, Value) {
    let s = world.resource::<MarkerStats>();
    let pairs = |v: &[(String, usize)]| -> Value {
        Value::Object(v.iter().map(|(k, n)| (k.clone(), json!(n))).collect())
    };
    (
        "campaign_markers".into(),
        json!({
            "built": world.resource::<MarkersBuilt>().0,
            "settlements_loaded": s.settlements,
            "catalog_valid": s.catalog_valid,
            "maquettes_by_kind": pairs(&s.maquettes_by_kind),
            "maquettes_by_family": pairs(&s.maquettes_by_family),
            "maquettes_loaded": s.maquettes_loaded,
            "maquettes_failed": s.maquettes_failed,
            "maquettes_visible": s.maquettes_visible,
            "maquettes_sunk": s.maquettes_sunk,
            "pinned": s.pinned,
            "family_fallbacks": s.family_fallbacks,
            "absorbed": s.absorbed,
            "landmarks": {
                "planned": s.landmarks_planned,
                "loaded": s.landmarks_loaded,
                "failed": s.landmarks_failed,
                "using_flat_fallback": s.landmarks_fallback,
            },
            "labels": s.labels,
            "labels_visible": s.labels_visible,
            "probes": s.probes.iter().map(|(n, size, base, anchor)| json!({"name": n, "size": [size.x, size.y, size.z], "base_y": base, "anchor_y": anchor})).collect::<Vec<_>>(),
            "ground": { "ready": world.get_resource::<crate::campaign::terrain::surface::Surface>().is_some(), "anchored": s.grounded, "min_y": s.ground_min, "max_y": s.ground_max },
            "ruin_signs": s.ruins,
            "worksite_signs": s.worksites,
            "controller_changes": s.controller_changes,
            "build_ms": s.build_ms,
        }),
    )
}

fn reset_markers(
    mut built: ResMut<MarkersBuilt>,
    mut places: ResMut<Places>,
    mut stats: ResMut<MarkerStats>,
) {
    built.0 = false;
    *places = Places::default();
    *stats = MarkerStats::default();
}

/// Builds the places once the campaign state exists, then spawns maquettes, landmarks, labels.
#[allow(clippy::too_many_arguments)]
fn build_markers(
    mut commands: Commands,
    mut built: ResMut<MarkersBuilt>,
    mut places_res: ResMut<Places>,
    mut stats: ResMut<MarkerStats>,
    sim: Res<Sim>,
    assets: Res<AssetServer>,
    ground: GroundHeight,
    theme: Res<UiTheme>,
) {
    if built.0 || sim.state.is_none() {
        return;
    }
    let Some(data) = sim.data.as_ref() else {
        return;
    };
    let Ok(root) = paths::find_root() else { return };
    let started = std::time::Instant::now();
    let data_dir = paths::data_dir(&root);
    let assets_dir = paths::asset_root(&root);
    let places = build_places(
        data,
        MarkerCatalog::load(&data_dir),
        Maquettes::load(&data_dir),
        load_landmarks(&data_dir, &assets_dir),
        &|n| {
            assets_dir
                .join(format!("models/settlements/{n}.glb"))
                .is_file()
        },
    );

    let root_entity = commands
        .spawn((
            Name::new("CampaignMarkers"),
            MarkersRoot,
            Transform::default(),
            Visibility::default(),
            DespawnOnExit(AppState::Campaign),
        ))
        .id();

    let scene = |path: &str| -> Handle<WorldAsset> {
        assets.load(GltfAssetLabel::Scene(0).from_asset(path.to_string()))
    };

    let mut by_family: std::collections::BTreeMap<String, usize> = Default::default();
    for p in &places.places {
        let at = placement::px_to_world(p.px, ground_y(&ground, p.px));
        // Landmark city: generated glb fitted to the core width, else the flat Blender maquette.
        if let Some(l) = p.landmark.map(|l| &places.landmarks[l]) {
            let (path, yaw, fallback, width) = match &l.generated {
                Some(g) => (
                    g.model.clone(),
                    g.yaw_deg.to_radians(),
                    Some(l.fallback_model.clone()),
                    2.0 * l.core_radius * places.maquettes.landmark_scale * g.width_factor,
                ),
                None => (
                    l.fallback_model.clone(),
                    0.0,
                    None,
                    2.0 * l.core_radius * places.maquettes.landmark_scale,
                ),
            };
            let handle = scene(&path);
            commands
                .spawn((
                    Name::new(format!("Landmark_{}", l.id)),
                    Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw)),
                    Visibility::default(),
                    GroundAnchor {
                        px: p.px,
                        lift: 0.0,
                    },
                    ChildOf(root_entity),
                ))
                .with_children(|parent| {
                    parent
                        .spawn((
                            Name::new("Model"),
                            Transform::default(),
                            WorldAssetRoot(handle.clone()),
                            LandmarkScene {
                                target_width: width,
                                handle,
                                fallback,
                                used_fallback: l.generated.is_none(),
                            },
                        ))
                        .observe(fit_landmark);
                });
            continue;
        }
        if let Some(path) = &p.model {
            *by_family.entry(p.family.clone()).or_default() += 1;
            let handle = scene(path);
            commands.spawn((
                Name::new(format!("Maquette_{}", p.id)),
                Transform::from_translation(at)
                    .with_rotation(Quat::from_rotation_y(p.yaw))
                    .with_scale(Vec3::splat(p.model_scale)),
                Visibility::default(),
                WorldAssetRoot(handle.clone()),
                Maquette {
                    place: places.index_by_id.get(&p.id).copied().unwrap_or_default(),
                    base_scale: p.model_scale,
                    sunk: false,
                    range: places.maquettes.range(p.shown_kind),
                    handle,
                },
                GroundAnchor {
                    px: p.px,
                    lift: 0.0,
                },
                ChildOf(root_entity),
            ));
        }
        // Labels are below.
    }
    // Name labels with the holder's shield above (UI nodes projected each frame), in the kit
    // font so accents render.
    for (i, p) in places.places.iter().enumerate() {
        let size = 10.0 + 2.0 * f32::from(p.rank);
        let side = places.catalog.shield_side(&p.kind, p.rank);
        commands
            .spawn((
                Name::new(format!("Label_{}", p.id)),
                Node {
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(places.catalog.shield_gap()),
                    ..default()
                },
                Visibility::Hidden,
                // World labels sit under the whole UI (the HUD is `GlobalZIndex(0)`).
                GlobalZIndex(-1),
                PlaceLabel { index: i },
                DespawnOnExit(AppState::Campaign),
            ))
            .with_children(|label| {
                label
                    .spawn((
                        Name::new("Shield"),
                        Node {
                            width: Val::Px(side),
                            height: Val::Px(side),
                            display: Display::None,
                            ..default()
                        },
                        ImageNode::default(),
                        LabelShield {
                            controller: String::new(),
                            index: i,
                        },
                    ))
                    .with_children(|shield| {
                        let (scale, overlap) = places.catalog.banner_occupant();
                        let small = side * scale;
                        shield.spawn((
                            Name::new("Occupant"),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(side - small * overlap),
                                top: Val::Px(side - small),
                                width: Val::Px(small),
                                height: Val::Px(small),
                                display: Display::None,
                                ..default()
                            },
                            ImageNode::default(),
                            OccupantShield,
                        ));
                    });
                label.spawn((
                    Name::new("Name"),
                    Text::new(p.name.clone()),
                    theme.text_font(p.rank >= 3, false, size),
                    TextColor(Color::srgb(0.13, 0.085, 0.045)),
                    TextShadow {
                        offset: Vec2::splat(1.0),
                        color: Color::srgba(0.93, 0.87, 0.72, 0.8),
                    },
                ));
            });
    }

    stats.settlements = places.places.len();
    stats.catalog_valid = places.catalog.is_valid() && !places.maquettes.is_empty();
    stats.maquettes_by_kind = {
        let counts = count_by_kind(&places);
        KINDS
            .iter()
            .map(|k| ((*k).to_string(), counts[k]))
            .collect()
    };
    stats.maquettes_by_family = by_family.into_iter().collect();
    stats.family_fallbacks = places.places.iter().filter(|p| p.family_fallback).count();
    stats.absorbed = places.places.iter().filter(|p| p.absorbed).count();
    stats.landmarks_planned = places
        .places
        .iter()
        .filter(|p| p.landmark.is_some())
        .count();
    stats.labels = places.places.len();
    stats.build_ms = started.elapsed().as_millis();
    *places_res = places;
    built.0 = true;
    info!(
        "markers: {} places, {} landmarks, built in {} ms",
        stats.settlements, stats.landmarks_planned, stats.build_ms
    );
}

/// Fits a generated landmark glb to its core width and rests its base on the ground, centred.
fn fit_landmark(
    ready: On<WorldInstanceReady>,
    mut roots: Query<(&LandmarkScene, &mut Transform)>,
    root_children: Query<&Children, With<LandmarkScene>>,
    nodes: Query<(&Transform, Option<&Mesh3d>, Option<&Children>), Without<LandmarkScene>>,
    meshes: Res<Assets<Mesh>>,
) {
    let Ok((scene, mut transform)) = roots.get_mut(ready.entity) else {
        return;
    };
    let mut bounds: Option<(Vec3, Vec3)> = None;
    let mut stack: Vec<(Entity, Mat4)> = Vec::new();
    if let Ok(children) = root_children.get(ready.entity) {
        stack.extend(children.iter().map(|c| (c, Mat4::IDENTITY)));
    }
    while let Some((entity, parent)) = stack.pop() {
        let Ok((t, mesh, children)) = nodes.get(entity) else {
            continue;
        };
        let local = parent * t.to_matrix();
        if let Some((c, h)) = mesh
            .and_then(|m| meshes.get(&m.0))
            .and_then(mesh_bounds)
            .map(|(lo, hi)| ((lo + hi) * 0.5, (hi - lo) * 0.5))
        {
            for corner in 0..8u8 {
                let sign = Vec3::new(
                    if corner & 1 == 0 { -1.0 } else { 1.0 },
                    if corner & 2 == 0 { -1.0 } else { 1.0 },
                    if corner & 4 == 0 { -1.0 } else { 1.0 },
                );
                let p = local.transform_point3(c + h * sign);
                bounds = Some(match bounds {
                    Some((lo, hi)) => (lo.min(p), hi.max(p)),
                    None => (p, p),
                });
            }
        }
        if let Some(children) = children {
            stack.extend(children.iter().map(|c| (c, local)));
        }
    }
    let Some((lo, hi)) = bounds else { return };
    *transform = fit_transform(lo, hi, scene.target_width);
}

/// Local bounds of a mesh from its positions.
fn mesh_bounds(mesh: &Mesh) -> Option<(Vec3, Vec3)> {
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
    let first = Vec3::from(*positions.first()?);
    Some(positions.iter().fold((first, first), |(lo, hi), p| {
        let v = Vec3::from(*p);
        (lo.min(v), hi.max(v))
    }))
}

/// Transform that scales a model of bounds `lo..hi` to `target_width` (largest horizontal side),
/// centres it horizontally and puts its base at y = 0.
pub fn fit_transform(lo: Vec3, hi: Vec3, target_width: f32) -> Transform {
    let size = hi - lo;
    let native = size.x.max(size.z).max(0.001);
    let k = target_width / native;
    let centre = (lo + hi) * 0.5;
    Transform::from_translation(-Vec3::new(centre.x, lo.y, centre.z) * k).with_scale(Vec3::splat(k))
}

/// Controller of each place from the simulation, refreshed twice a second.
fn refresh_controllers(
    time: Res<Time>,
    mut timer: Local<f32>,
    sim: Res<Sim>,
    mut places: ResMut<Places>,
    mut stats: ResMut<MarkerStats>,
) {
    *timer += time.delta_secs();
    if *timer < 0.5 {
        return;
    }
    *timer = 0.0;
    let Some(state) = sim.state.as_ref() else {
        return;
    };
    let places = &mut *places;
    for (p, owner) in places.places.iter_mut().zip(places.owners.iter_mut()) {
        let Ok(id) = data_model::SettlementId::new(&p.id) else {
            continue;
        };
        if let Some(live) = state.settlement_state(&id)
            && (live.controller.as_str() != p.controller || live.owner.as_str() != owner.as_str())
        {
            p.controller = live.controller.as_str().to_string();
            *owner = live.owner.as_str().to_string();
            stats.controller_changes += 1;
        }
    }
}

/// Ruin and worksite signs, rebuilt when the set of ruined or building places changes.
#[allow(clippy::too_many_arguments)]
fn refresh_signs(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: Local<f32>,
    mut signature: Local<String>,
    sim: Res<Sim>,
    places: Res<Places>,
    ground: GroundHeight,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<Entity, Or<(With<RuinSign>, With<WorksiteSign>)>>,
    root: Query<Entity, With<MarkersRoot>>,
    mut stats: ResMut<MarkerStats>,
) {
    *timer += time.delta_secs();
    if *timer < 0.5 {
        return;
    }
    *timer = 0.0;
    let (Some(state), Some(root)) = (sim.state.as_ref(), root.iter().next()) else {
        return;
    };
    let ruins: Vec<&places::Place> = state
        .captures
        .ruins
        .iter()
        .filter(|(_, until)| **until > state.turn)
        .filter_map(|(id, _)| places.get(id.as_str()))
        .collect();
    let worksites: Vec<&places::Place> = places
        .places
        .iter()
        .filter(|p| p.controller == sim.player.as_str())
        .filter(|p| {
            data_model::SettlementId::new(&p.id)
                .ok()
                .and_then(|id| state.settlement_state(&id))
                .is_some_and(|s| s.construction.is_some())
        })
        .collect();
    let sig = format!(
        "{}|{}",
        ruins
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>()
            .join(","),
        worksites
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    if *signature == sig {
        return;
    }
    *signature = sig;
    for e in &existing {
        commands.entity(e).despawn();
    }
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let ink = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.15, 0.11, 0.95),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    for p in &ruins {
        let at = placement::px_to_world(p.px, ground_y(&ground, p.px) + signs::RUIN_LIFT);
        commands.spawn((
            Name::new(format!("Ruin_{}", p.id)),
            RuinSign,
            Mesh3d(quad.clone()),
            MeshMaterial3d(ink.clone()),
            Transform::from_translation(at),
            GroundAnchor {
                px: p.px,
                lift: signs::RUIN_LIFT,
            },
            ChildOf(root),
        ));
    }
    for p in &worksites {
        let at = placement::px_to_world(p.px, ground_y(&ground, p.px));
        commands.spawn((
            Name::new(format!("Worksite_{}", p.id)),
            WorksiteSign,
            WorldAssetRoot(
                assets.load(GltfAssetLabel::Scene(0).from_asset(signs::WORKSITE_MODEL.to_string())),
            ),
            Transform::from_translation(at)
                .with_rotation(Quat::from_rotation_y(signs::WORKSITE_YAW)),
            GroundAnchor {
                px: p.px,
                lift: 0.0,
            },
            ChildOf(root),
        ));
    }
    stats.ruins = ruins.len();
    stats.worksites = worksites.len();
}

/// Hides maquettes beyond their range (distance camera to ground point).
fn update_range(
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut maquettes: Query<(&Maquette, &GlobalTransform, &mut Visibility), Without<Camera3d>>,
    mut stats: ResMut<MarkerStats>,
) {
    let Some(cam) = camera.iter().next() else {
        return;
    };
    let eye = cam.translation();
    let mut visible = 0;
    for (m, g, mut v) in &mut maquettes {
        let show = !m.sunk && eye.distance(g.translation()) < m.range;
        let want = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
        visible += usize::from(show);
    }
    stats.maquettes_visible = visible;
}

/// Re-applies the displayed ground height to every anchored object: once the terrain exists, for
/// new anchors, and whenever the relief exaggeration (camera zoom) changes.
fn settle_ground(
    ground: GroundHeight,
    relief: Option<Res<ReliefState>>,
    fresh: Query<(), Added<GroundAnchor>>,
    mut anchored: Query<(&GroundAnchor, &mut Transform)>,
    mut stats: ResMut<MarkerStats>,
    mut was_ready: Local<bool>,
) {
    if !ground.is_ready() {
        return;
    }
    let changed = relief.is_some_and(|r| r.is_changed()) || !*was_ready || !fresh.is_empty();
    *was_ready = true;
    if !changed {
        return;
    }
    let (mut count, mut lo, mut hi) = (0usize, f32::MAX, f32::MIN);
    for (anchor, mut t) in &mut anchored {
        let y = ground_y(&ground, anchor.px) + anchor.lift;
        if (t.translation.y - y).abs() > 1e-4 {
            t.translation.y = y;
        }
        count += 1;
        lo = lo.min(y);
        hi = hi.max(y);
    }
    stats.grounded = count;
    if count > 0 {
        stats.ground_min = lo;
        stats.ground_max = hi;
    }
}

/// Holder's heraldry image (`None` = no armoury file), cached by faction.
fn heraldry_of(
    cache: &mut ShieldCache,
    assets: &AssetServer,
    dir: &std::path::Path,
    faction: &str,
) -> Option<Handle<Image>> {
    cache
        .0
        .entry(faction.to_string())
        .or_insert_with(|| {
            dir.join(format!("{faction}.png"))
                .is_file()
                .then(|| assets.load(format!("heraldry/{faction}.png")))
        })
        .clone()
}

#[allow(clippy::too_many_arguments)]
/// Shows or hides each label's shield by rank distance and loads the heraldry: the de jure owner's
/// shield above the name and, when the place is occupied, the occupant's small shield at its
/// bottom right (RJ-c; an owner without arms yields the occupant's shield alone). The selected
/// place and the player's capital keep their shield at any distance.
fn update_shields(
    camera: Query<(&GlobalTransform, &Camera), With<Camera3d>>,
    places: Res<Places>,
    pins: Res<Pins>,
    ground: GroundHeight,
    assets: Res<AssetServer>,
    mut cache: ResMut<ShieldCache>,
    mut shields: Query<
        (
            &mut LabelShield,
            &mut Node,
            &mut ImageNode,
            Option<&Children>,
        ),
        (Without<PlaceLabel>, Without<OccupantShield>),
    >,
    mut occupants: Query<(&mut Node, &mut ImageNode), With<OccupantShield>>,
) {
    let Some((cam_t, _)) = camera.iter().next() else {
        return;
    };
    let Ok(root) = paths::find_root() else { return };
    let heraldry_dir = paths::asset_root(&root).join("heraldry");
    let eye = cam_t.translation();
    for (mut shield, mut node, mut image, children) in &mut shields {
        let p = &places.places[shield.index];
        let world = placement::px_to_world(p.px, ground_y(&ground, p.px));
        let near = pins.indices.contains(&shield.index)
            || eye.distance(world) < places.catalog.shield_until(p.rank);
        let owner_id = places
            .owners
            .get(shield.index)
            .map_or(p.controller.as_str(), String::as_str);
        let key = format!("{owner_id}|{}", p.controller);
        if shield.controller != key {
            shield.controller = key;
            let owner = heraldry_of(&mut cache, &assets, &heraldry_dir, owner_id);
            let occupant = (p.controller != owner_id && !p.controller.is_empty())
                .then(|| heraldry_of(&mut cache, &assets, &heraldry_dir, &p.controller))
                .flatten();
            let (main, small) = match (owner, occupant) {
                (Some(o), occ) => (Some(o), occ),
                (None, occ) => (occ, None),
            };
            image.image = main.unwrap_or_default();
            if let Some(children) = children {
                for child in children.iter() {
                    if let Ok((mut small_node, mut small_image)) = occupants.get_mut(child) {
                        small_image.image = small.clone().unwrap_or_default();
                        small_node.display = if small.is_some() {
                            Display::Flex
                        } else {
                            Display::None
                        };
                    }
                }
            }
        }
        let has_image = image.image != Handle::default();
        let want = if near && has_image {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != want {
            node.display = want;
        }
    }
}

/// Pinned places: the selected province's city, then the player's capital city.
fn update_pins(
    sim: Res<Sim>,
    places: Res<Places>,
    selected: Option<Res<SelectedProvince>>,
    mut pins: ResMut<Pins>,
) {
    let mut out = Vec::new();
    if let Some(state) = sim.state.as_ref() {
        let city_of = |province: &str| {
            data_model::ProvinceId::new(province)
                .ok()
                .and_then(|p| state.province_city_id(&p).cloned())
                .and_then(|c| places.index_by_id.get(c.as_str()).copied())
        };
        if places.catalog.declutter_flag("pin_selected")
            && let Some(id) = selected.as_ref().and_then(|s| s.id.as_deref())
            && let Some(i) = city_of(id)
        {
            out.push(i);
        }
        if places.catalog.declutter_flag("pin_player_capital")
            && let Some(faction) = state.factions.get(&sim.player)
            && let Some(i) = city_of(faction.capital.as_str())
            && !out.contains(&i)
        {
            out.push(i);
        }
    }
    if pins.indices != out {
        pins.indices = out;
    }
}

/// Sinks a maquette into the ground as its 1:1 town (towns lot) takes over: the model is squashed
/// by the layer weight and hidden once the town is fully shown.
fn sink_maquettes(
    towns: Res<TownsState>,
    places: Res<Places>,
    mut maquettes: Query<(&mut Maquette, &mut Transform)>,
    mut stats: ResMut<MarkerStats>,
) {
    let mut sunk = 0;
    for (mut m, mut t) in &mut maquettes {
        let shown = places
            .places
            .get(m.place)
            .is_some_and(|p| towns.is_shown(&p.id));
        let keep = if shown {
            (1.0 - towns.weight()).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let want = Vec3::new(m.base_scale, m.base_scale * keep.max(0.02), m.base_scale);
        if (t.scale - want).length() > 1e-5 {
            t.scale = want;
        }
        let is_sunk = keep <= 0.02;
        if m.sunk != is_sunk {
            m.sunk = is_sunk;
        }
        sunk += usize::from(is_sunk);
    }
    stats.maquettes_sunk = sunk;
}

/// Projects each label, applies its rank visibility distance, then the greedy declutter of the
/// name + shield block (measured from the laid-out node).
fn update_labels(
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    places: Res<Places>,
    pins: Res<Pins>,
    ground: GroundHeight,
    mut labels: Query<
        (&PlaceLabel, &mut Node, &mut Visibility, &ComputedNode),
        Without<LabelShield>,
    >,
    plates: Query<(&Name, &Node, &Visibility), Without<PlaceLabel>>,
    mut stats: ResMut<MarkerStats>,
) {
    let Some((camera, cam_t)) = camera.iter().next() else {
        return;
    };
    let eye = cam_t.translation();
    // Priority: rank descending, then the place order (kind, weight, id).
    let mut entries: Vec<_> = labels.iter_mut().collect();
    // Pinned places (selected, player capital) come first and bypass the distance limit and the
    // overlap test.
    entries.sort_by_key(|(l, ..)| {
        (
            pins.indices
                .iter()
                .position(|p| *p == l.index)
                .unwrap_or(usize::MAX),
            std::cmp::Reverse(places.places[l.index].rank),
            l.index,
        )
    });
    let cell = places.catalog.declutter_f32("cell_px", 64.0);
    let margin = places.catalog.declutter_f32("label_margin_px", 3.0);
    let mut grid = declutter::Declutter::new(cell);
    // Army plates (effectives) are obstacles: a name never sits under one.
    for (plate_name, plate, plate_vis) in &plates {
        if plate_name.as_str().starts_with("Plate_")
            && let (Val::Px(left), Val::Px(top), Val::Px(width), Val::Px(height)) =
                (plate.left, plate.top, plate.width, plate.height)
            && *plate_vis != Visibility::Hidden
        {
            let rect =
                Rect::from_corners(Vec2::new(left, top), Vec2::new(left + width, top + height));
            grid.try_place(rect, -2, true);
        }
    }
    let mut shown = 0;
    for (label, node, vis, computed) in &mut entries {
        let p = &places.places[label.index];
        let world = placement::px_to_world(p.px, ground_y(&ground, p.px));
        let pinned = pins.indices.contains(&label.index);
        let limit = if pinned {
            f32::INFINITY
        } else {
            places.catalog.visible_until(&p.kind, p.rank)
        };
        let mut hide = true;
        if eye.distance(world) <= limit
            && let Ok(screen) = camera.world_to_viewport(cam_t, world)
        {
            let measured = computed.size() * computed.inverse_scale_factor();
            let size = if measured.x > 1.0 {
                measured
            } else {
                Vec2::new(p.name.chars().count() as f32 * 6.0, 16.0)
            };
            let rect = Rect::from_corners(
                Vec2::new(screen.x - size.x * 0.5, screen.y - size.y) - Vec2::splat(margin),
                Vec2::new(screen.x + size.x * 0.5, screen.y) + Vec2::splat(margin),
            );
            // Above the settlement; if an army plate or a bigger place is there, just below it.
            let below = Rect::from_corners(
                rect.min + Vec2::new(0.0, size.y + 4.0),
                rect.max + Vec2::new(0.0, size.y + 4.0),
            );
            let slot = if pinned {
                // Always shown; still registered so others avoid it.
                grid.try_place(rect, -1, true);
                Some(0.0)
            } else if grid.try_place(rect, label.index as i32, false) {
                Some(0.0)
            } else if grid.try_place(below, label.index as i32, false) {
                Some(size.y + 4.0)
            } else {
                None
            };
            if let Some(drop) = slot {
                node.left = Val::Px(screen.x - size.x * 0.5);
                node.top = Val::Px(screen.y - size.y + drop);
                hide = false;
                shown += 1;
            }
        }
        let want = if hide {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if **vis != want {
            **vis = want;
        }
    }
    stats.labels_visible = shown;
    stats.pinned = pins.indices.len();
}

/// Falls back to the flat Blender maquette when a generated landmark glb fails to load.
fn landmark_fallback(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut scenes: Query<(Entity, &mut LandmarkScene)>,
) {
    for (entity, mut l) in &mut scenes {
        if l.used_fallback || !matches!(assets.load_state(&l.handle), LoadState::Failed(_)) {
            continue;
        }
        l.used_fallback = true;
        if let Some(path) = l.fallback.clone() {
            let handle: Handle<WorldAsset> = assets.load(GltfAssetLabel::Scene(0).from_asset(path));
            l.handle = handle.clone();
            commands.entity(entity).insert(WorldAssetRoot(handle));
        }
    }
}

/// Load counters of the maquette and landmark scenes.
fn update_stats(
    assets: Res<AssetServer>,
    scenes: Query<&LandmarkScene>,
    maquettes: Query<&Maquette>,
    probed: Query<(Entity, &Name, &GlobalTransform), Or<(With<Maquette>, With<GroundAnchor>)>>,
    bounds: Query<(&GlobalTransform, &bevy::camera::primitives::Aabb)>,
    children: Query<&Children>,
    mut stats: ResMut<MarkerStats>,
) {
    if stats.grounded > 0 && stats.probes.len() < 6 {
        for (entity, name, anchor) in &probed {
            let n = name.as_str();
            if !(n.starts_with("Maquette_") || n.starts_with("Landmark_"))
                || stats.probes.len() >= 6
                || stats.probes.iter().any(|p| p.0 == n)
            {
                continue;
            }
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for d in children.iter_descendants(entity) {
                if let Ok((g, a)) = bounds.get(d) {
                    for c in 0..8u8 {
                        let s = Vec3::new(
                            if c & 1 == 0 { -1.0 } else { 1.0 },
                            if c & 2 == 0 { -1.0 } else { 1.0 },
                            if c & 4 == 0 { -1.0 } else { 1.0 },
                        );
                        let p = g
                            .transform_point(Vec3::from(a.center) + Vec3::from(a.half_extents) * s);
                        lo = lo.min(p);
                        hi = hi.max(p);
                    }
                }
            }
            if lo.x <= hi.x {
                stats
                    .probes
                    .push((n.to_string(), hi - lo, lo.y, anchor.translation().y));
            }
        }
    }
    let (mut loaded, mut failed, mut fallback) = (0, 0, 0);
    for l in &scenes {
        match assets.load_state(&l.handle) {
            LoadState::Loaded => loaded += 1,
            LoadState::Failed(_) => failed += 1,
            _ => {}
        }
        fallback += usize::from(l.used_fallback && l.fallback.is_some());
    }
    stats.landmarks_loaded = loaded;
    stats.landmarks_failed = failed;
    stats.landmarks_fallback = fallback;
    let (mut ok, mut bad) = (0, 0);
    for m in &maquettes {
        match assets.load_state(&m.handle) {
            LoadState::Loaded => ok += 1,
            LoadState::Failed(_) => bad += 1,
            _ => {}
        }
    }
    stats.maquettes_loaded = ok;
    stats.maquettes_failed = bad;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_scales_centres_and_grounds() {
        let t = fit_transform(Vec3::new(-1.0, 2.0, -2.0), Vec3::new(3.0, 6.0, 2.0), 8.0);
        assert!((t.scale.x - 2.0).abs() < 1e-6);
        // centre x = 1 -> -2, base y = 2 -> -4, centre z = 0.
        assert!((t.translation - Vec3::new(-2.0, -4.0, 0.0)).length() < 1e-6);
    }
}
