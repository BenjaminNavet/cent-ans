//! River crossings on the campaign map (`river_crossings.gd`): bridges, fords and ferries from
//! `data/map/crossings_px.json` (output of `cent-ans geo rivers-render`: historical crossings
//! snapped on the displayed river, plus a generic bridge at every road/river crossing), built from
//! `bridge_meshes` and set on the displayed surface.
//!
//! Not ported: gate bridges under walled settlements (`_add_gate_bridges`, needs the covers of
//! `RiversRenderer`), custom zones, fine mode (`fine_anchors.json`), generated glb bridges
//! (`DnWaterModels`), the ford / ferry uniforms of `river_water.gdshader`. Crossings that fall
//! under a settlement maquette are not filtered (Godot hides them through the covers).

use std::collections::HashMap;

use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use serde_json::Value;

use super::bridge_meshes::{self, STONE, WOOD};
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::GroundHeight;

/// Camera distance beyond which crossings are hidden (`VISIBILITY_RANGE`).
pub const VISIBILITY_RANGE: f32 = 280.0;
/// Vertical and deck exaggeration of the works (`HEIGHT_SCALE`, `DECK_SCALE`).
pub const HEIGHT_SCALE: f32 = 2.0;
pub const DECK_SCALE: f32 = 2.0;
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
    relief: Option<Res<ReliefState>>,
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

#[cfg(test)]
mod tests {
    use super::*;

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
