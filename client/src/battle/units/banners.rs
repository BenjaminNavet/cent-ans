//! Regiment banners: a pole and a waving cloth over every regiment, turned three quarters
//! towards the camera, scaled with the camera distance, white while the regiment routs.
//! Port of game/scripts/battle/battle_banner_layer.gd and `battle_banner.gdshader`; the cloth
//! wave is computed on the CPU (78 vertices per banner) instead of in a vertex shader.
//!
//! Royal banners (oriflamme, dragon with the no-quarter order, Saint George), house banners of
//! the general and his retinue (`heraldry/banners/houses/`), wind direction by weather
//! (`wind.rs`: banners lie downwind when seen from close). The carried standards are in
//! `standards.rs`.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde_json::json;
use sim_battle::{BattleSim, SideId, Unit, UnitState};

use super::library::FigureLibrary;
use super::wind::BattleWind;
use super::{PoseFrames, drawn_units};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::{BattleCamera, BattleCameraState};

/// Height of the pole before scaling (m).
pub const BANNER_HEIGHT: f32 = 7.0;
/// Cloth subdivisions along and across.
const NX: usize = 12;
const NY: usize = 5;
/// Three-quarter view offset (rad) between the camera direction and the cloth's normal.
const THREE_QUARTER: f32 = 0.35;
/// Banners farther than this are not waved (still posed): the wave is invisible there.
const WAVE_DISTANCE: f32 = 700.0;

/// Banner scale for a camera distance (`_banner_scale`).
pub fn banner_scale(camera_distance: f32) -> f32 {
    (camera_distance * 0.014).clamp(0.8, 9.0)
}

/// Counters of the banner layer (dump section `battle_banners`).
#[derive(Resource, Default, Debug, Clone)]
pub struct BannerStats {
    pub banners: usize,
    pub visible: usize,
    pub textured: usize,
    pub routing: usize,
    pub waved: usize,
    pub scale: f32,
}

pub(super) struct Banner {
    pub(super) root: Entity,
    pub(super) mesh: Handle<Mesh>,
    pub(super) material: Handle<StandardMaterial>,
    texture: Option<Handle<Image>>,
    /// Cloth file shown (relative to the asset root).
    cloth_file: Option<String>,
    length: f32,
    height: f32,
    phase: f32,
    routing: bool,
}

#[derive(Resource, Default)]
pub(super) struct Banners {
    pub(super) map: HashMap<u32, Banner>,
    pub(super) pole: Option<(Handle<Mesh>, Handle<StandardMaterial>)>,
    pub(super) finial: Option<(Handle<Mesh>, Handle<StandardMaterial>)>,
    /// House id of each side's general (`None`: no arms), read once.
    houses: Option<[Option<String>; 2]>,
    retinue: Vec<String>,
}

/// Cloth of a regiment: texture file (relative to the asset root) and size in metres, in the
/// order of `cloth_for`: the general's royal cloth (oriflamme or dragon under the no-quarter
/// order, Saint George for the English royal army), the house banner of the general and his
/// retinue, nobility and siege units, then the faction pennon.
pub fn cloth_candidates(
    unit: &Unit,
    faction: &str,
    no_quarter: bool,
    house: Option<&str>,
    retinue: bool,
) -> Vec<(String, Vec2)> {
    let dir = "heraldry/banners/";
    let size = Vec2::new(1.3, 2.6);
    let mut out = Vec::new();
    let mut royal_first = false;
    if unit.is_general && matches!(faction, "fac_france" | "fac_england") {
        if no_quarter {
            let file = if faction == "fac_france" {
                "oriflamme"
            } else {
                "dragon"
            };
            out.push((format!("{dir}{file}.png"), size));
            royal_first = true;
        } else if faction == "fac_england" {
            out.push((format!("{dir}st_george.png"), size));
        }
    }
    if let Some(house) = house.filter(|h| !h.is_empty())
        && (unit.is_general || retinue)
    {
        let banner = (format!("{dir}houses/{house}_banner.png"), size);
        out.insert(usize::from(royal_first), banner);
    }
    let noble = matches!(&*unit.unit_type, "unit_knights" | "unit_men_at_arms_foot");
    if noble || unit.category == data_model::UnitCategory::Siege {
        out.push((format!("{dir}{faction}_banner.png"), size));
    } else {
        out.push((format!("{dir}{faction}_pennon.png"), Vec2::new(3.0, 0.75)));
        out.push((format!("{dir}{faction}_banner.png"), size));
    }
    out
}

/// House id (`houses.json`) of a house given by name or id.
pub fn house_id(data_dir: &std::path::Path, house: &str) -> Option<String> {
    let text = std::fs::read_to_string(data_dir.join("heraldry/houses.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("houses")?.as_array()?.iter().find_map(|entry| {
        let id = entry.get("id")?.as_str()?;
        let name = entry.get("name")?.as_str()?;
        (id == house || name == house).then(|| id.to_owned())
    })
}

/// House of the general of `side`: the `house` of his character file, as an id.
fn general_house(sim: &BattleSim, data_dir: &std::path::Path, side: SideId) -> Option<String> {
    let general = sim.setup().side(side).general.as_ref()?;
    let text =
        std::fs::read_to_string(data_dir.join(format!("characters/{}.json", general.character)))
            .ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    house_id(data_dir, value.get("house")?.as_str()?)
}

/// Unit types of the general's retinue that carry his arms (`house_arms` of battle_standards.json).
fn retinue_types(data_dir: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(data_dir.join("fx/battle_standards.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| {
            v.get("house_arms")?.get("unit_types")?.as_array().map(|a| {
                a.iter()
                    .filter_map(|t| t.as_str().map(str::to_owned))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// First existing cloth of `candidates` (file, size).
fn pick_cloth(root: &std::path::Path, candidates: Vec<(String, Vec2)>) -> Option<(String, Vec2)> {
    candidates
        .into_iter()
        .find(|(file, _)| root.join(file).exists())
}

/// Shortest-arc interpolation of angles.
pub fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let diff =
        (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    from + diff * t
}

/// Cloth vertices and normals at time `time` for wind `strength`/`gust` (the vertex stage of
/// `battle_banner.gdshader`): a wave running from the pole to the free edge, drooping when the
/// wind drops.
pub fn cloth_vertices(
    length: f32,
    height: f32,
    wind: (f32, f32),
    time: f32,
    phase: f32,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let gust = (time * 0.7 + phase * 0.37).sin() * 0.5 + (time * 1.9 + phase).sin() * 0.25;
    let s = (wind.0 + wind.1 * gust).clamp(0.0, 1.0);
    let t = time * (2.5 + (7.5 - 2.5) * s) + phase;
    let amp = 0.08 + (0.3 - 0.08) * s;
    let droop = 0.45 + (0.06 - 0.45) * s;
    let squeeze = 0.8 + 0.2 * s;
    let mut positions = Vec::with_capacity((NX + 1) * (NY + 1));
    let mut normals = Vec::with_capacity(positions.capacity());
    for j in 0..=NY {
        for i in 0..=NX {
            let x = i as f32 / NX as f32;
            let y = j as f32 / NY as f32;
            let wave = (x * 7.0 - t).sin() * amp + (x * 13.0 - t * 1.7 + 1.3).sin() * amp * 0.27;
            let z = wave * x * length * 0.35;
            let sag = x * x * droop * length;
            positions.push([x * length * squeeze, -y * height - sag, z]);
            let slope = ((x * 7.0 - t).cos() * 7.0 * amp
                + (x * 13.0 - t * 1.7 + 1.3).cos() * 13.0 * amp * 0.27)
                * x
                * 0.35;
            let n = Vec3::new(-slope, 0.0, 1.0).normalize();
            normals.push(n.to_array());
        }
    }
    (positions, normals)
}

fn cloth_mesh(length: f32, height: f32) -> Mesh {
    let (positions, normals) = cloth_vertices(length, height, (0.5, 0.0), 0.0, 0.0);
    let uvs: Vec<[f32; 2]> = (0..=NY)
        .flat_map(|j| (0..=NX).map(move |i| [i as f32 / NX as f32, j as f32 / NY as f32]))
        .collect();
    let stride = (NX + 1) as u32;
    let mut indices = Vec::with_capacity(NX * NY * 6);
    for j in 0..NY as u32 {
        for i in 0..NX as u32 {
            let a = j * stride + i;
            let b = a + 1;
            let c = a + stride;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, d, a, d, b]);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub struct BattleBannersPlugin;

impl Plugin for BattleBannersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BannerStats>()
            .init_resource::<Banners>()
            .register_dump(dump_banners)
            .add_systems(
                Update,
                update_banners
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .run_if(resource_exists::<FigureLibrary>)
                    .after(crate::battle::scene::StepSet)
                    .after(super::regiments::update_regiments),
            )
            .add_systems(OnExit(AppState::Battle), clear_banners);
    }
}

fn clear_banners(mut banners: ResMut<Banners>, mut stats: ResMut<BannerStats>) {
    *banners = Banners::default();
    *stats = BannerStats::default();
}

fn dump_banners(world: &World) -> (String, serde_json::Value) {
    let stats = world
        .get_resource::<BannerStats>()
        .cloned()
        .unwrap_or_default();
    (
        "battle_banners".into(),
        json!({
            "banners": stats.banners,
            "visible": stats.visible,
            "textured": stats.textured,
            "routing": stats.routing,
            "waved": stats.waved,
            "scale": stats.scale,
        }),
    )
}

fn side_name(side: SideId) -> &'static str {
    if side == SideId::Attacker {
        "attacker"
    } else {
        "defender"
    }
}

pub(super) fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn faction_of(sim: &BattleSim, side: SideId) -> String {
    sim.setup().side(side).faction.clone()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_banners(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut banners: ResMut<Banners>,
    mut stats: ResMut<BannerStats>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    library: Res<FigureLibrary>,
    assets: Res<AssetServer>,
    time: Res<Time>,
    frames: Res<PoseFrames>,
    camera_state: Option<Res<BattleCameraState>>,
    wind: Option<Res<BattleWind>>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    mut transforms: Query<&mut Transform>,
    mut visibilities: Query<&mut Visibility>,
) {
    let colors = session.colors;
    let sim = &*session.sim_mut();
    let camera_position = camera
        .iter()
        .next()
        .map_or(Vec3::Y * 100.0, |t| t.translation());
    let distance = camera_state.map_or(150.0, |c| c.distance);
    let scale = banner_scale(distance);
    if banners.pole.is_none() {
        let pole_mesh = Mesh::from(Cuboid::new(0.08, BANNER_HEIGHT, 0.08))
            .translated_by(Vec3::Y * BANNER_HEIGHT * 0.5);
        let bar = Mesh::from(Cuboid::new(0.5, 0.06, 0.06)).translated_by(Vec3::new(
            0.2,
            BANNER_HEIGHT - 0.3,
            0.0,
        ));
        let mut pole_mesh = pole_mesh;
        pole_mesh.merge(&bar).expect("same layout");
        let wood = materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.2, 0.1),
            perceptual_roughness: 0.9,
            ..default()
        });
        let gold = materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.66, 0.2),
            metallic: 0.6,
            perceptual_roughness: 0.4,
            ..default()
        });
        banners.pole = Some((meshes.add(pole_mesh), wood));
        banners.finial = Some((
            meshes.add(
                Mesh::from(Cuboid::new(0.18, 0.1, 0.18)).translated_by(Vec3::Y * BANNER_HEIGHT),
            ),
            gold,
        ));
    }
    let wind = wind.map_or(BattleWind::default(), |w| *w);
    if banners.houses.is_none() {
        let data = crate::paths::find_root().map(|r| crate::paths::data_dir(&r));
        banners.houses = Some(match &data {
            Ok(dir) => [
                general_house(sim, dir, SideId::Attacker),
                general_house(sim, dir, SideId::Defender),
            ],
            Err(_) => [None, None],
        });
        banners.retinue = data.map(|d| retinue_types(&d)).unwrap_or_default();
    }
    let houses = banners.houses.clone().unwrap_or([None, None]);
    let retinue_list = banners.retinue.clone();
    let wind_mix = smoothstep(1.0, 2.5, scale);
    let downwind = wind.downwind_yaw();
    let (pole_mesh, wood) = banners.pole.clone().expect("set above");
    let (finial_mesh, gold) = banners.finial.clone().expect("set above");
    let seconds = time.elapsed_secs();
    let mut visible = 0;
    let mut textured = 0;
    let mut routing = 0;
    let mut waved = 0;
    let mut shown_ids = Vec::new();
    for unit in drawn_units(sim) {
        let Some(poses) = frames.cur.get(&unit.id) else {
            continue;
        };
        if let Entry::Vacant(slot) = banners.map.entry(unit.id) {
            let faction = faction_of(sim, unit.side);
            let mut texture = None;
            let mut cloth_file = None;
            let mut size = Vec2::new(2.6, 1.7);
            let side_house = houses[usize::from(unit.side == SideId::Defender)].as_deref();
            let retinue = retinue_list.iter().any(|t| **t == *unit.unit_type);
            let candidates = cloth_candidates(
                unit,
                &faction,
                sim.no_quarter(unit.side),
                side_house,
                retinue,
            );
            if let Some((file, cloth_size)) = pick_cloth(&library.asset_root, candidates) {
                texture = Some(assets.load::<Image>(file.clone()));
                cloth_file = Some(file);
                size = cloth_size;
            }
            let [r, g, b] = colors[usize::from(unit.side == SideId::Defender)];
            let material = materials.add(StandardMaterial {
                base_color: if texture.is_some() {
                    Color::WHITE
                } else {
                    Color::srgb(r, g, b)
                },
                base_color_texture: texture.clone(),
                emissive_texture: texture.clone(),
                emissive: LinearRgba::gray(0.25),
                alpha_mode: if texture.is_some() {
                    AlphaMode::Mask(0.5)
                } else {
                    AlphaMode::Opaque
                },
                double_sided: true,
                cull_mode: None,
                perceptual_roughness: 0.9,
                ..default()
            });
            let mesh = meshes.add(cloth_mesh(size.x, size.y));
            let root = commands
                .spawn((
                    Name::new(format!("Banner{}_{}", unit.id, side_name(unit.side))),
                    Transform::default(),
                    Visibility::Hidden,
                    DespawnOnExit(AppState::Battle),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Mesh3d(pole_mesh.clone()),
                        MeshMaterial3d(wood.clone()),
                        NoFrustumCulling,
                    ));
                    parent.spawn((
                        Mesh3d(finial_mesh.clone()),
                        MeshMaterial3d(gold.clone()),
                        NoFrustumCulling,
                    ));
                    parent.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_xyz(0.03, BANNER_HEIGHT - 0.05, 0.0),
                        NoFrustumCulling,
                    ));
                })
                .id();
            slot.insert(Banner {
                root,
                mesh,
                material,
                texture,
                cloth_file,
                length: size.x,
                height: size.y,
                phase: unit.id as f32 * 1.7,
                routing: false,
            });
        }
        let banner = banners.map.get_mut(&unit.id).expect("inserted above");
        shown_ids.push(unit.id);
        if unit.is_general {
            // The no-quarter order changes the royal cloth (oriflamme, dragon) mid-battle.
            let side_house = houses[usize::from(unit.side == SideId::Defender)].as_deref();
            let faction = faction_of(sim, unit.side);
            let candidates =
                cloth_candidates(unit, &faction, sim.no_quarter(unit.side), side_house, true);
            if let Some((file, size)) = pick_cloth(&library.asset_root, candidates)
                && banner.cloth_file.as_deref() != Some(file.as_str())
            {
                let texture = assets.load::<Image>(file.clone());
                banner.texture = Some(texture.clone());
                banner.cloth_file = Some(file);
                if !banner.routing
                    && let Some(mut material) = materials.get_mut(&banner.material)
                {
                    material.base_color_texture = Some(texture.clone());
                    material.emissive_texture = Some(texture);
                }
                if (banner.length, banner.height) != (size.x, size.y) {
                    banner.length = size.x;
                    banner.height = size.y;
                }
            }
        }
        let ground = poses.first().map_or(0.0, |p| p[1]);
        let foot = Vec3::new(unit.x as f32, ground, unit.z as f32);
        let to_camera = camera_position - foot;
        let camera_yaw = to_camera.x.atan2(to_camera.z) + THREE_QUARTER;
        let yaw = lerp_angle(downwind, camera_yaw, wind_mix);
        if let Ok(mut transform) = transforms.get_mut(banner.root) {
            transform.translation = foot;
            transform.rotation = Quat::from_rotation_y(yaw);
            transform.scale = Vec3::splat(scale);
        }
        if let Ok(mut visibility) = visibilities.get_mut(banner.root) {
            *visibility = Visibility::Inherited;
        }
        visible += 1;
        if banner.texture.is_some() {
            textured += 1;
        }
        let rout = unit.state == UnitState::Routing;
        if rout != banner.routing {
            banner.routing = rout;
            if let Some(mut material) = materials.get_mut(&banner.material) {
                if rout {
                    material.base_color_texture = None;
                    material.emissive_texture = None;
                    material.alpha_mode = AlphaMode::Opaque;
                    material.base_color = Color::srgb(0.92, 0.92, 0.9);
                } else {
                    material.base_color_texture = banner.texture.clone();
                    material.emissive_texture = banner.texture.clone();
                    material.base_color = Color::WHITE;
                    if banner.texture.is_some() {
                        material.alpha_mode = AlphaMode::Mask(0.5);
                    }
                }
            }
        }
        if rout {
            routing += 1;
        }
        if to_camera.length() < WAVE_DISTANCE
            && let Some(mut mesh) = meshes.get_mut(&banner.mesh)
        {
            let (positions, normals) = cloth_vertices(
                banner.length,
                banner.height,
                (wind.strength * if rout { 1.4 } else { 1.0 }, wind.gust),
                seconds,
                banner.phase,
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            waved += 1;
        }
    }
    for (id, banner) in &banners.map {
        if !shown_ids.contains(id)
            && let Ok(mut visibility) = visibilities.get_mut(banner.root)
        {
            *visibility = Visibility::Hidden;
        }
    }
    stats.banners = banners.map.len();
    stats.visible = visible;
    stats.textured = textured;
    stats.routing = routing;
    stats.waved = waved;
    stats.scale = scale;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_follows_camera_distance_within_bounds() {
        assert_eq!(banner_scale(10.0), 0.8);
        assert!((banner_scale(100.0) - 1.4).abs() < 1e-6);
        assert_eq!(banner_scale(5000.0), 9.0);
    }

    #[test]
    fn cloth_is_pinned_at_the_pole_and_waves_towards_the_edge() {
        let (calm, _) = cloth_vertices(2.0, 1.0, (0.5, 0.0), 1.0, 0.0);
        let (later, normals) = cloth_vertices(2.0, 1.0, (0.5, 0.0), 1.4, 0.0);
        assert_eq!(calm.len(), (NX + 1) * (NY + 1));
        assert_eq!(calm[0][2], 0.0, "no wave at the pole");
        assert!(calm[0][0].abs() < 1e-6);
        let edge = NX;
        assert!((calm[edge][2] - later[edge][2]).abs() > 1e-4, "edge moves");
        assert!(
            normals
                .iter()
                .all(|n| (Vec3::from(*n).length() - 1.0).abs() < 1e-4)
        );
        // No wind: the free edge hangs lower than with a strong wind.
        let (still, _) = cloth_vertices(2.0, 1.0, (0.0, 0.0), 1.0, 0.0);
        let (gale, _) = cloth_vertices(2.0, 1.0, (1.0, 0.0), 1.0, 0.0);
        assert!(still[edge][1] < gale[edge][1]);
    }

    #[test]
    fn cloth_mesh_has_two_triangles_per_cell() {
        let mesh = cloth_mesh(1.3, 2.6);
        assert_eq!(mesh.indices().map(|i| i.len()), Some(NX * NY * 6));
    }
}
