//! River crossings on the campaign map (`river_crossings.gd`): bridges, fords and ferries from
//! `data/map/crossings_px.json` (output of `cent-ans geo rivers-render`: historical crossings
//! snapped on the displayed river, plus a generic bridge at every road/river crossing), built from
//! `bridge_meshes` and set on the displayed surface.
//!
//! Fine mode (`set_fine_anchors` / `set_fine_mode`, ZG4b): under the near tier, when the fine
//! hydrography is loaded (`water/fine`), each crossing with a snapped anchor of
//! `fine_anchors.json` moves onto the fine river at real scale (`FINE_SCALE`), deck at `z_deck`
//! above the water at `z_water` ([`FineBridgeMode`], [`apply_fine_bridges`], a few per frame).
//!
//! Not ported: gate bridges under walled settlements (`_add_gate_bridges`: the settlement covers
//! are empty in Godot since VT, ADR 0138, so none are built there either), zone bridges (every
//! custom zone has `boundary_bridges: false`), generated glb bridges (`DnWaterModels`), the ford /
//! ferry uniforms of `river_water.gdshader`.

use std::collections::HashMap;

use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use serde_json::Value;

use super::bridge_meshes::{self, STONE, WOOD};
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::terrain::surface::GroundHeight;

/// Camera distance beyond which crossings are hidden (`VISIBILITY_RANGE`).
pub const VISIBILITY_RANGE: f32 = 280.0;
/// Vertical and deck exaggeration of the works (`HEIGHT_SCALE`, `DECK_SCALE`).
pub const HEIGHT_SCALE: f32 = 2.0;
pub const DECK_SCALE: f32 = 2.0;
/// Reduction of the works in fine mode (abutments 0.5 unit -> 50 m, deck 10-25 m).
pub const FINE_SCALE: f32 = 0.14;
/// Fine crossings re-shaped per frame (Godot spreads the switch with `FrameBudget`).
const FINE_RESHAPES_PER_FRAME: usize = 12;
const STONE_TEXTURE: &str = "textures/battle/castle_wall_varriation_diff.jpg";
const WOOD_TEXTURE: &str = "textures/battle/wood_planks_diff.jpg";

#[derive(Debug, Clone)]
pub struct Crossing {
    pub id: String,
    pub name: String,
    pub structure: String,
    pub px: Vec2,
    pub dir: Vec2,
    pub axis: Option<Vec2>,
    pub width: f32,
}

#[derive(Component)]
pub struct Bridge {
    pub index: usize,
    pub px: Vec2,
    pub last_relief: u32,
    pub grounded: bool,
    /// Shaped on its fine anchor (see [`FineBridgeMode`]).
    pub fine: bool,
}

/// Fine anchors of the crossings and the wanted mode, filled by `water/fine` (`FineGeoLayer`).
#[derive(Resource, Default)]
pub struct FineBridgeMode {
    /// Snapped anchors by crossing id.
    pub anchors: HashMap<String, crate::campaign::water::fine::store::FineCrossing>,
    /// Near tier reached: crossings with an anchor sit on the fine river.
    pub fine: bool,
    pub meters_per_unit: f32,
    /// Generalised width of `river_display.json` (`RiversRenderer.generalised_width`).
    pub gen_width_scale: f32,
    pub gen_width_ref: f32,
    /// Crossings currently shaped on their fine anchor.
    pub reshaped: usize,
}

impl FineBridgeMode {
    /// Displayed width (units) of a river `w` units wide (`generalised_width`).
    pub fn generalised_width(&self, w: f32) -> f32 {
        if self.gen_width_scale > 0.0 {
            w.max(self.gen_width_scale * (w.max(0.0) * self.gen_width_ref).sqrt())
        } else {
            w
        }
    }
}

/// Real deck width (m) of the fine works (`BridgeMeshes.FINE_DECK_M`).
fn fine_deck_m(structure: &str) -> f32 {
    match structure {
        "gate" => 9.0,
        "wood" => 4.5,
        "boats" => 5.0,
        "ferry" => 8.0,
        "ford" => 6.0,
        _ => 7.0,
    }
}

/// Deck width in the mesh frame, parapets included (`BridgeMeshes.mesh_deck_width`).
fn mesh_deck_width(structure: &str, width: f32) -> f32 {
    let deck = bridge_meshes::deck_width(width);
    match structure {
        "gate" => deck * 1.35,
        "wood" => deck * 0.9,
        _ => deck,
    }
}

/// Scale along the current giving the deck its real width, never above `uniform_scale`
/// (`BridgeMeshes.fine_deck_scale`).
pub fn fine_deck_scale(
    structure: &str,
    mesh_width: f32,
    meters_per_unit: f32,
    uniform_scale: f32,
) -> f32 {
    let target = fine_deck_m(structure) / meters_per_unit.max(1e-3);
    uniform_scale.min(target / mesh_deck_width(structure, mesh_width).max(1e-4))
}

#[derive(Resource, Default)]
pub struct Bridges {
    pub crossings: Vec<Crossing>,
    pub loaded: bool,
    pub spawned: usize,
    pub by_structure: std::collections::BTreeMap<String, usize>,
    pub meshes_built: usize,
    pub relief_version: u32,
    pub visible: usize,
    materials: Option<[Handle<StandardMaterial>; 2]>,
    cache: HashMap<(String, i32, u64), [Option<Handle<Mesh>>; 2]>,
}

fn vec2_of(v: &Value) -> Option<Vec2> {
    Some(Vec2::new(
        v.get(0)?.as_f64()? as f32,
        v.get(1)?.as_f64()? as f32,
    ))
}

/// `crossings_px.json`: only the crossings snapped on the displayed river, outside custom zones.
pub fn parse_crossings(doc: &Value) -> Vec<Crossing> {
    let mut out = Vec::new();
    for raw in doc["crossings"].as_array().into_iter().flatten() {
        if !raw["snapped"].as_bool().unwrap_or(false)
            || raw["in_custom_zone"].as_bool().unwrap_or(false)
        {
            continue;
        }
        let Some(px) = vec2_of(&raw["px"]) else {
            continue;
        };
        let dir = vec2_of(&raw["dir"])
            .map(Vec2::normalize_or_zero)
            .filter(|d| *d != Vec2::ZERO)
            .unwrap_or(Vec2::Y);
        out.push(Crossing {
            id: raw["id"].as_str().unwrap_or("").to_string(),
            name: raw["name"].as_str().unwrap_or("").to_string(),
            structure: raw["structure"].as_str().unwrap_or("stone").to_string(),
            px,
            dir,
            axis: vec2_of(&raw["axis"]).map(Vec2::normalize_or_zero),
            width: raw["width"].as_f64().unwrap_or(0.5).max(0.15) as f32,
        });
    }
    out
}

/// Stable text hash (the Godot code seeds the variant with `String.hash()`).
fn id_seed(id: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in id.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Orientation of a crossing: X across the river (or along the carried road), Z = X x Y.
pub fn basis_of(c: &Crossing) -> (Vec3, Vec3) {
    let across = match c.axis.filter(|a| *a != Vec2::ZERO) {
        Some(a) => Vec3::new(a.x, 0.0, a.y),
        None => Vec3::new(-c.dir.y, 0.0, c.dir.x),
    };
    let along = across.cross(Vec3::Y);
    (across, along)
}

fn repeat_texture(server: &AssetServer, path: &'static str) -> Handle<Image> {
    server
        .load_builder()
        .with_settings(|s: &mut ImageLoaderSettings| {
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..ImageSamplerDescriptor::linear()
            });
        })
        .load(path)
}

/// Reads the crossings once per game (the file does not change).
pub fn load_crossings(mut bridges: ResMut<Bridges>) {
    if bridges.loaded {
        return;
    }
    bridges.loaded = true;
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    let path = root.join("data/map/crossings_px.json");
    match std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
    {
        Some(doc) => bridges.crossings = parse_crossings(&doc),
        None => warn!("bridges: {} missing or invalid", path.display()),
    }
}

/// Spawns the crossings (once per campaign visit) as soon as the terrain can ground them.
pub fn spawn_bridges(
    mut commands: Commands,
    mut bridges: ResMut<Bridges>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
    existing: Query<(), With<Bridge>>,
) {
    if bridges.spawned > 0 && !existing.is_empty() {
        return;
    }
    if bridges.crossings.is_empty() {
        return;
    }
    let handles = bridges
        .materials
        .get_or_insert_with(|| {
            [
                materials.add(StandardMaterial {
                    base_color_texture: Some(repeat_texture(&server, STONE_TEXTURE)),
                    base_color: Color::srgb(0.92, 0.86, 0.76),
                    perceptual_roughness: 0.92,
                    ..default()
                }),
                materials.add(StandardMaterial {
                    base_color_texture: Some(repeat_texture(&server, WOOD_TEXTURE)),
                    base_color: Color::srgb(0.62, 0.50, 0.40),
                    perceptual_roughness: 0.85,
                    ..default()
                }),
            ]
        })
        .clone();
    bridges.by_structure.clear();
    let crossings = bridges.crossings.clone();
    for (index, c) in crossings.iter().enumerate() {
        let seed = id_seed(&c.id);
        let key = bridge_meshes::cache_key(&c.structure, c.width, seed);
        if !bridges.cache.contains_key(&key) {
            let surfaces = bridge_meshes::build_surfaces(&c.structure, c.width, seed);
            let scale = if cfg!(test) { 1.0 } else { 3.0 };
            let built: [Option<Handle<Mesh>>; 2] = std::array::from_fn(|k| {
                (!surfaces[k].is_empty()).then(|| meshes.add(surfaces[k].to_mesh(scale)))
            });
            bridges.meshes_built += 1;
            bridges.cache.insert(key.clone(), built);
        }
        let built = bridges.cache[&key].clone();
        let (across, along) = basis_of(c);
        let rotation = Quat::from_mat3(&Mat3::from_cols(across, Vec3::Y, along));
        commands
            .spawn((
                Name::new(format!("bridge:{}", c.id)),
                Transform::from_xyz(c.px.x, 0.0, c.px.y)
                    .with_rotation(rotation)
                    .with_scale(Vec3::new(1.0, HEIGHT_SCALE, DECK_SCALE)),
                Visibility::Hidden,
                Bridge {
                    index,
                    px: c.px,
                    last_relief: u32::MAX,
                    grounded: false,
                    fine: false,
                },
                DespawnOnExit(AppState::Campaign),
            ))
            .with_children(|p| {
                for kind in [STONE, WOOD] {
                    if let Some(mesh) = &built[kind] {
                        p.spawn((
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(handles[kind].clone()),
                            bevy::light::NotShadowCaster,
                        ));
                    }
                }
            });
        *bridges.by_structure.entry(c.structure.clone()).or_default() += 1;
    }
    bridges.spawned = crossings.len();
}

/// Sets the crossings on the displayed surface and hides them beyond the range.
pub fn ground_bridges(
    ground: GroundHeight,
    rig: Option<Res<CampaignRig>>,
    relief: Option<Res<super::ReliefVersion>>,
    mut bridges: ResMut<Bridges>,
    mut query: Query<(&mut Bridge, &mut Transform, &mut Visibility)>,
) {
    if relief.as_ref().is_some_and(|r| r.is_changed()) {
        bridges.relief_version = bridges.relief_version.wrapping_add(1);
    }
    let version = bridges.relief_version;
    let near = rig.is_some_and(|r| r.distance < VISIBILITY_RANGE);
    let mut visible = 0;
    for (mut bridge, mut transform, mut visibility) in &mut query {
        if bridge.fine {
            // Grounded by `apply_fine_bridges` on the fine water level.
            let wanted = if near {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if near {
                visible += 1;
            }
            if *visibility != wanted {
                *visibility = wanted;
            }
            continue;
        }
        if (!bridge.grounded || bridge.last_relief != version)
            && let Some(y) = ground.at(bridge.px.x, bridge.px.y)
        {
            transform.translation.y = y;
            bridge.last_relief = version;
            bridge.grounded = true;
        }
        let show = near && bridge.grounded;
        if show {
            visible += 1;
        }
        let wanted = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    bridges.visible = visible;
}

/// Mesh children of a crossing for a structure and mesh width (cached by `cache_key`).
fn bridge_children(
    bridges: &mut Bridges,
    meshes: &mut Assets<Mesh>,
    structure: &str,
    width: f32,
    seed: u64,
) -> [Option<Handle<Mesh>>; 2] {
    let key = bridge_meshes::cache_key(structure, width, seed);
    if !bridges.cache.contains_key(&key) {
        let surfaces = bridge_meshes::build_surfaces(structure, width, seed);
        let scale = if cfg!(test) { 1.0 } else { 3.0 };
        let built: [Option<Handle<Mesh>>; 2] = std::array::from_fn(|k| {
            (!surfaces[k].is_empty()).then(|| meshes.add(surfaces[k].to_mesh(scale)))
        });
        bridges.meshes_built += 1;
        bridges.cache.insert(key.clone(), built);
    }
    bridges.cache[&key].clone()
}

/// Fine shape of a crossing: mesh width, transform (`RiverCrossings._shape` / `_ground`, fine).
pub fn fine_transform(
    anchor: &crate::campaign::water::fine::store::FineCrossing,
    structure: &str,
    mode: &FineBridgeMode,
    display: impl Fn(f32, f32, f32) -> f32,
) -> (f32, Transform) {
    let width = mode
        .generalised_width(anchor.width_m / mode.meters_per_unit.max(1e-3))
        .max(0.01);
    let mesh_width = width / FINE_SCALE;
    let across = Vec3::new(-anchor.dir.y, 0.0, anchor.dir.x);
    let along = across.cross(Vec3::Y);
    let q = anchor.px;
    // Deck at `z_deck`: mesh deck top (0.05 + 0.02 x width) scaled to the displayed rise above the
    // water (local exaggeration included).
    let deck_top = (0.05 + 0.02 * mesh_width) * FINE_SCALE;
    let rise = display(
        anchor.z_water + (anchor.z_deck - anchor.z_water).max(3.0),
        q.x,
        q.y,
    ) - display(anchor.z_water, q.x, q.y);
    let k_h = (rise / deck_top.max(1e-4)).clamp(0.3, 12.0);
    let k_along = fine_deck_scale(structure, mesh_width, mode.meters_per_unit, FINE_SCALE);
    let y = display(anchor.z_water, q.x, q.y).max(0.0);
    let transform = Transform::from_xyz(q.x, y, q.y)
        .with_rotation(Quat::from_mat3(&Mat3::from_cols(across, Vec3::Y, along)))
        .with_scale(Vec3::new(FINE_SCALE, FINE_SCALE * k_h, k_along));
    (mesh_width, transform)
}

/// Switches the crossings between their coarse shape and their fine anchor (`set_fine_mode`,
/// spread over frames like `pump_reshape`), and re-grounds the fine ones when the relief changes.
#[allow(clippy::too_many_arguments)]
pub fn apply_fine_bridges(
    mut commands: Commands,
    mut mode: ResMut<FineBridgeMode>,
    mut bridges: ResMut<Bridges>,
    mut meshes: ResMut<Assets<Mesh>>,
    relief: Option<Res<crate::campaign::terrain::relief_state::ReliefState>>,
    relief_version: Option<Res<super::ReliefVersion>>,
    mut query: Query<(Entity, &mut Bridge, &mut Transform)>,
) {
    let Some(relief) = relief else {
        return;
    };
    let Some(handles) = bridges.materials.clone() else {
        return;
    };
    let relief_changed = relief_version.is_some_and(|r| r.is_changed());
    let display = |h: f32, x: f32, z: f32| relief.display_height(h, x, z);
    let mut budget = FINE_RESHAPES_PER_FRAME;
    let mut reshaped = 0;
    for (entity, mut bridge, mut transform) in &mut query {
        let Some(c) = bridges.crossings.get(bridge.index).cloned() else {
            continue;
        };
        let anchor = mode.anchors.get(&c.id).cloned();
        let want = mode.fine && anchor.is_some();
        if want == bridge.fine {
            if bridge.fine
                && relief_changed
                && let Some(a) = &anchor
            {
                *transform = fine_transform(a, &c.structure, &mode, display).1;
            }
            reshaped += usize::from(bridge.fine);
            continue;
        }
        if budget == 0 {
            reshaped += usize::from(bridge.fine);
            continue;
        }
        budget -= 1;
        let seed = id_seed(&c.id);
        let built = if let Some(a) = anchor.as_ref().filter(|_| want) {
            let (mesh_width, t) = fine_transform(a, &c.structure, &mode, display);
            *transform = t;
            bridge_children(&mut bridges, &mut meshes, &c.structure, mesh_width, seed)
        } else {
            let (across, along) = basis_of(&c);
            *transform = Transform::from_xyz(c.px.x, transform.translation.y, c.px.y)
                .with_rotation(Quat::from_mat3(&Mat3::from_cols(across, Vec3::Y, along)))
                .with_scale(Vec3::new(1.0, HEIGHT_SCALE, DECK_SCALE));
            bridge.grounded = false;
            bridge_children(&mut bridges, &mut meshes, &c.structure, c.width, seed)
        };
        bridge.fine = want;
        reshaped += usize::from(want);
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .with_children(|p| {
                for kind in [STONE, WOOD] {
                    if let Some(mesh) = &built[kind] {
                        p.spawn((
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(handles[kind].clone()),
                            bevy::light::NotShadowCaster,
                        ));
                    }
                }
            });
    }
    if mode.reshaped != reshaped {
        mode.reshaped = reshaped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fine_transform_sits_on_the_water_at_real_scale() {
        let anchor = crate::campaign::water::fine::store::FineCrossing {
            id: "b".into(),
            px: Vec2::new(10.0, 20.0),
            snapped: true,
            dir: Vec2::Y,
            width_m: 140.0,
            z_water: 50.0,
            z_deck: 60.0,
        };
        let mode = FineBridgeMode {
            meters_per_unit: 700.0,
            ..Default::default()
        };
        let (mesh_width, t) = fine_transform(&anchor, "stone", &mode, |h, _, _| h * 0.006);
        assert!((mesh_width - 0.2 / FINE_SCALE).abs() < 1e-4);
        assert!((t.translation.y - 0.3).abs() < 1e-5);
        assert_eq!(t.scale.x, FINE_SCALE);
        // Deck 7 m real: well under the uniform scale.
        assert!(t.scale.z < FINE_SCALE);
        assert!(t.scale.y > 0.0);
        // Across the current (+Z flow): X axis maps to -X.
        let across = t.rotation * Vec3::X;
        assert!((across - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn repository_crossings_parse_with_their_structures() {
        let root = crate::paths::find_root().expect("root");
        let doc: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("data/map/crossings_px.json")).unwrap(),
        )
        .unwrap();
        let list = parse_crossings(&doc);
        assert!(list.len() > 100, "{}", list.len());
        assert!(list.iter().any(|c| c.structure == "stone"));
        assert!(list.iter().any(|c| c.structure == "wood"));
        assert!(list.iter().all(|c| c.width >= 0.15));
    }

    #[test]
    fn basis_is_right_handed_across_the_river() {
        let c = Crossing {
            id: "x".into(),
            name: String::new(),
            structure: "stone".into(),
            px: Vec2::ZERO,
            dir: Vec2::new(0.0, 1.0),
            axis: None,
            width: 1.0,
        };
        let (across, along) = basis_of(&c);
        assert!((across.cross(Vec3::Y) - along).length() < 1e-6);
        assert!((across - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-6);
        let carried = Crossing {
            axis: Some(Vec2::new(1.0, 0.0)),
            ..c
        };
        assert!((basis_of(&carried).0 - Vec3::X).length() < 1e-6);
    }
}
