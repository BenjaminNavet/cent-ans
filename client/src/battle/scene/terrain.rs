//! Battlefield terrain: the sim-battle height grid as one mesh coloured per vertex (ground by
//! season and weather, woods, mud, water), a sun, a sky and an apron around the field.
//!
//! Ports the pure parts of game/scripts/battle/battle_terrain.gd, battle_terrain_mesh.gd and
//! battle_terrain_field.rs. The splat-mapped textured ground lives in `decor::ground`; trees,
//! grass, site elements, bridges and the atmosphere in the other `decor` modules. Not ported yet:
//! rivers as water meshes.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use sim_battle::{BattleSeason, Battlefield, Weather};

use super::BattleSession;
use crate::agent::OffscreenTarget;
use crate::app::AppState;
use crate::battle::decor::ground::{GroundMaterial, build_ground_material, splat_at};
use crate::paths;

/// Mesh vertex spacing (m): half the sim grid resolution, the colours follow the finer sample.
const SUBDIVISION: usize = 2;
/// Extent (m) of the apron plane around the field.
const APRON_HALF_SIZE: f32 = 6000.0;

/// Marks the terrain entity (read by the dump and the camera ground probe).
#[derive(Component)]
pub struct BattleTerrain;

/// Mesh buffers of the terrain, built without any rendering.
#[derive(Debug, Default)]
pub struct TerrainMeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Splat weights (grass, forest, dirt, mud), see [`crate::battle::decor::ground`].
    pub colors: Vec<[f32; 4]>,
    /// World-space texture coordinates (metres).
    pub uvs: Vec<[f32; 2]>,
    /// Splat weights (stone, plough).
    pub uvs_b: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// Ground colour of a season (sRGB), before woods, mud and water tint it.
pub fn season_ground(season: BattleSeason) -> [f32; 3] {
    match season {
        BattleSeason::Spring => [0.36, 0.50, 0.22],
        BattleSeason::Summer => [0.44, 0.48, 0.23],
        BattleSeason::Autumn => [0.50, 0.40, 0.20],
        BattleSeason::Winter => [0.82, 0.84, 0.86],
    }
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Cheap deterministic value noise in `0..1` for the ground variation.
fn noise(x: f32, z: f32) -> f32 {
    let n = (x * 12.9898 + z * 78.233).sin() * 43758.547;
    n - n.floor()
}

/// Colour of the ground at `(x, z)`.
pub fn ground_color(field: &Battlefield, weather: Weather, x: f64, z: f64) -> [f32; 4] {
    let mut color = season_ground(field.season);
    let patch = (noise((x / 18.0).floor() as f32, (z / 18.0).floor() as f32) - 0.5) * 0.08;
    color = [color[0] + patch, color[1] + patch, color[2] + patch * 0.6];
    if matches!(weather, Weather::Snow) || matches!(field.ground, sim_battle::Ground::Snowy) {
        color = mix(color, [0.88, 0.9, 0.93], 0.7);
    }
    if field.in_mud(x, z) {
        color = mix(color, [0.30, 0.22, 0.14], 0.85);
    }
    if field.in_forest(x, z) {
        color = mix(color, [0.13, 0.26, 0.11], 0.7);
    }
    match field.water_at(x, z) {
        Some(true) => color = mix(color, [0.30, 0.45, 0.50], 0.8), // ford
        Some(false) => color = [0.12, 0.28, 0.42],                 // deep water
        None => {}
    }
    [color[0], color[1], color[2], 1.0]
}

/// The terrain mesh of `field`: `SUBDIVISION` vertices per sim grid cell, heights bilinear.
pub fn build_terrain_data(field: &Battlefield) -> TerrainMeshData {
    let step = field.resolution / SUBDIVISION as f64;
    let columns = ((field.width / step).round() as usize).max(1) + 1;
    let rows = ((field.depth / step).round() as usize).max(1) + 1;
    let mut data = TerrainMeshData::default();
    data.positions.reserve(columns * rows);
    let height = |x: f64, z: f64| field.height(x, z) as f32;
    for row in 0..rows {
        let z = (row as f64 * step).min(field.depth);
        for column in 0..columns {
            let x = (column as f64 * step).min(field.width);
            data.positions.push([x as f32, height(x, z), z as f32]);
            // Central differences give the normal.
            let dx = height((x + step).min(field.width), z) - height((x - step).max(0.0), z);
            let dz = height(x, (z + step).min(field.depth)) - height(x, (z - step).max(0.0));
            let normal = Vec3::new(-dx, 2.0 * step as f32, -dz).normalize();
            data.normals.push(normal.to_array());
            let splat = splat_at(field, x, z);
            data.colors.push(splat.color());
            data.uvs_b.push(splat.uv_b());
            data.uvs.push([x as f32, z as f32]);
        }
    }
    for row in 0..rows - 1 {
        for column in 0..columns - 1 {
            let a = (row * columns + column) as u32;
            let b = a + 1;
            let c = a + columns as u32;
            let d = c + 1;
            // Counter-clockwise seen from above (+y).
            data.indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    data
}

impl TerrainMeshData {
    pub fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.uvs_b)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Sky clear colour of a weather.
pub fn sky_color(weather: Weather) -> Color {
    match weather {
        Weather::Clear => Color::srgb(0.55, 0.72, 0.92),
        Weather::Rain => Color::srgb(0.42, 0.46, 0.52),
        Weather::Fog => Color::srgb(0.66, 0.68, 0.70),
        Weather::Snow => Color::srgb(0.74, 0.78, 0.82),
    }
}

pub struct BattleTerrainPlugin;

impl Plugin for BattleTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_scene.run_if(resource_added::<BattleSession>));
    }
}

#[allow(clippy::too_many_arguments)]
/// Spawns terrain, apron and the camera once the session exists (the sun, sky and fog belong to
/// [`crate::battle::decor::atmosphere`]).
fn spawn_scene(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GroundMaterial>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    look: Res<crate::battle::decor::look::DecorLook>,
    assets: Res<AssetServer>,
    target: Option<Res<OffscreenTarget>>,
) {
    let player = session.player;
    let sim = &*session.sim_mut();
    let field = sim.field();
    let weather = sim.weather();
    let scope = DespawnOnExit(AppState::Battle);
    let data = build_terrain_data(field);
    let splat_colors = data.colors.clone();
    let splat_b = data.uvs_b.clone();
    let mesh = data.into_mesh();
    let data_dir = paths::find_root()
        .map(|root| paths::data_dir(&root))
        .unwrap_or_default();
    let ground = materials.add(build_ground_material(
        &mut commands,
        &mut images,
        &assets,
        &look,
        &data_dir,
        field,
        weather,
        &splat_colors,
        &splat_b,
    ));
    commands.spawn((
        Name::new("BattleTerrain"),
        BattleTerrain,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(ground),
        Transform::default(),
        scope.clone(),
    ));
    // Apron: a big flat plane a little under the field so its edge does not float (replaced by the
    // horizon ring of `decor::horizon` when the look enables it).
    if !look.horizon.enabled {
        let lowest = field.heights.iter().copied().fold(f64::MAX, f64::min) as f32;
        let [r, g, b] = season_ground(field.season);
        let apron = standard.add(StandardMaterial {
            base_color: Color::srgb(r * 0.9, g * 0.9, b * 0.9),
            perceptual_roughness: 1.0,
            reflectance: 0.05,
            ..default()
        });
        commands.spawn((
            Name::new("BattleApron"),
            Mesh3d(
                meshes.add(
                    Plane3d::default()
                        .mesh()
                        .size(APRON_HALF_SIZE * 2.0, APRON_HALF_SIZE * 2.0),
                ),
            ),
            MeshMaterial3d(apron),
            Transform::from_xyz(
                field.width as f32 * 0.5,
                lowest - 0.4,
                field.depth as f32 * 0.5,
            ),
            scope.clone(),
        ));
    }
    let (centre, yaw) = opening_frame(player, sim);
    camera::spawn_battle_camera(
        &mut commands,
        field,
        target.as_deref(),
        sky_color(weather),
        centre,
        yaw,
    );
}

use super::camera;

/// Radius (m) around the regiment nearest the army centroid that the opening shot frames.
const OPENING_CLUSTER_M: f32 = 120.0;

/// Opening shot: the knot of the player's regiments nearest the army centroid (attacker without a
/// player), seen from behind it towards the enemy centroid. A deployed army is often split in
/// separate battles, so the plain centroid may fall on an empty meadow.
pub fn opening_frame(
    player: Option<sim_battle::SideId>,
    sim: &sim_battle::BattleSim,
) -> (Vec3, f32) {
    let side = player.unwrap_or(sim_battle::SideId::Attacker);
    let field = sim.field();
    let middle = Vec3::new(field.width as f32 * 0.5, 0.0, field.depth as f32 * 0.5);
    let positions = |own: bool| -> Vec<Vec3> {
        sim.units()
            .iter()
            .filter(|u| (u.side == side) == own && !u.synthetic)
            .map(|u| Vec3::new(u.x as f32, 0.0, u.z as f32))
            .collect()
    };
    let mean = |v: &[Vec3]| v.iter().copied().sum::<Vec3>() / v.len().max(1) as f32;
    let own = positions(true);
    let enemy = positions(false);
    let centre = if own.is_empty() {
        middle
    } else {
        let centroid = mean(&own);
        let nearest = own
            .iter()
            .copied()
            .min_by(|a, b| {
                a.distance_squared(centroid)
                    .total_cmp(&b.distance_squared(centroid))
            })
            .unwrap_or(centroid);
        let knot: Vec<Vec3> = own
            .iter()
            .copied()
            .filter(|p| p.distance(nearest) <= OPENING_CLUSTER_M)
            .collect();
        mean(&knot)
    };
    // Look towards the enemy (the middle of the field without one): the eye sits on the far side.
    let target = if enemy.is_empty() {
        middle
    } else {
        mean(&enemy)
    };
    let toward = target - centre;
    let yaw = if toward.xz().length_squared() < 1.0 {
        0.0
    } else {
        (-toward.x).atan2(-toward.z)
    };
    (centre, yaw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::scene::setup;
    use crate::paths;
    use data_model::GameData;

    #[test]
    fn terrain_mesh_covers_the_field() {
        let root = paths::find_root().unwrap();
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).unwrap();
        setup::install_battle_rules(&dir);
        let custom = setup::default_custom_battle(&game);
        let sim = setup::build_sim(&game, &dir, &custom, 7).unwrap();
        let data = build_terrain_data(sim.field());
        assert_eq!(data.positions.len(), data.normals.len());
        assert_eq!(data.positions.len(), data.colors.len());
        assert!(data.indices.len().is_multiple_of(3) && !data.indices.is_empty());
        let max_x = data.positions.iter().map(|p| p[0]).fold(0.0, f32::max);
        let max_z = data.positions.iter().map(|p| p[2]).fold(0.0, f32::max);
        assert!((f64::from(max_x) - sim.field().width).abs() < 1.0);
        assert!((f64::from(max_z) - sim.field().depth).abs() < 1.0);
        assert!(
            data.indices
                .iter()
                .all(|i| (*i as usize) < data.positions.len())
        );
        assert!(data.normals.iter().all(|n| n[1] > 0.0), "normals point up");
    }
}
