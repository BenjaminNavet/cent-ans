//! Ribbon trail of the arrows in flight (port of `battle_projectile_trail.gdshader` and
//! `_trail_mesh` of battle_effects.gd).
//!
//! Godot draws one thin quad per arrow, turned toward the camera around the flight axis, from the
//! tip back along the path (5 m) and fading out, widened with the distance so that it stays
//! visible (1x up to 30 m, 7x at 210 m). The oriented batch of the arrows has no alpha, so the
//! ribbons are one dynamic alpha-blended mesh rebuilt every frame from `Volleys::trail_tips`
//! (arrows within `TRAIL_RANGE_M` of the camera, at most `MAX_TRAILS`).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::battle::units::volleys::Volleys;

/// `trail_alpha`, `trail_length` of the shader and the half width of the quad (`w`).
const TRAIL_ALPHA: f32 = 0.45;
const TRAIL_LENGTH_M: f32 = 5.0;
const HALF_WIDTH_M: f32 = 0.03;
/// Colour of the ribbon (`ALBEDO`).
const TRAIL_COLOR: [f32; 3] = [0.86, 0.85, 0.8];

#[derive(Component)]
struct TrailMesh;

#[derive(Resource, Default)]
pub struct TrailStats {
    pub ribbons: usize,
    pub peak: usize,
    pub widest_m: f32,
}

/// Vertex data of the ribbons: for each arrow four vertices (tip left/right, tail left/right) and
/// two triangles. The tip is opaque at `TRAIL_ALPHA`, the tail transparent.
pub fn build_ribbons(
    tips: &[(Vec3, Vec3)],
    camera: Vec3,
) -> (Vec<[f32; 3]>, Vec<[f32; 4]>, Vec<u32>) {
    let mut positions = Vec::with_capacity(tips.len() * 4);
    let mut colors = Vec::with_capacity(tips.len() * 4);
    let mut indices = Vec::with_capacity(tips.len() * 6);
    for &(tip, dir) in tips {
        let to_camera = (camera - tip).normalize_or_zero();
        let across = (dir.cross(to_camera) + Vec3::Y * 1e-4).normalize_or_zero();
        let widen = (camera.distance(tip) / 30.0).clamp(1.0, 7.0);
        let half = across * HALF_WIDTH_M * widen;
        let tail = tip - dir * TRAIL_LENGTH_M;
        let base = positions.len() as u32;
        for p in [tip - half, tip + half, tail + half, tail - half] {
            positions.push(p.to_array());
        }
        let near = [TRAIL_COLOR[0], TRAIL_COLOR[1], TRAIL_COLOR[2], TRAIL_ALPHA];
        let far = [TRAIL_COLOR[0], TRAIL_COLOR[1], TRAIL_COLOR[2], 0.0];
        colors.extend_from_slice(&[near, near, far, far]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (positions, colors, indices)
}

fn ribbon_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    mesh.insert_indices(Indices::U32(Vec::new()));
    mesh
}

fn spawn_trail(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("ArrowTrails"),
        TrailMesh,
        Mesh3d(meshes.add(ribbon_mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        })),
        Transform::default(),
        NoFrustumCulling,
        NotShadowCaster,
        DespawnOnExit(AppState::Battle),
    ));
}

fn update_trail(
    volleys: Res<Volleys>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    handle: Query<&Mesh3d, With<TrailMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut stats: ResMut<TrailStats>,
) {
    let Some(camera) = camera.iter().next().map(|t| t.translation()) else {
        return;
    };
    let Ok(Mesh3d(handle)) = handle.single() else {
        return;
    };
    let Some(mut mesh) = meshes.get_mut(handle) else {
        return;
    };
    let tips = &volleys.trail_tips;
    let (positions, colors, indices) = build_ribbons(tips, camera);
    stats.ribbons = tips.len();
    stats.peak = stats.peak.max(tips.len());
    stats.widest_m = tips
        .iter()
        .map(|(tip, _)| 2.0 * HALF_WIDTH_M * (camera.distance(*tip) / 30.0).clamp(1.0, 7.0))
        .fold(stats.widest_m, f32::max);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
}

fn dump_trail(world: &World) -> (String, serde_json::Value) {
    let Some(s) = world.get_resource::<TrailStats>() else {
        return ("battle_arrow_trails".into(), serde_json::Value::Null);
    };
    (
        "battle_arrow_trails".into(),
        serde_json::json!({
            "ribbons": s.ribbons, "peak": s.peak, "widest_m": s.widest_m,
            "length_m": TRAIL_LENGTH_M,
        }),
    )
}

pub struct TrailPlugin;

impl Plugin for TrailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrailStats>()
            .register_dump(dump_trail)
            .add_systems(OnEnter(AppState::Battle), spawn_trail)
            .add_systems(
                Update,
                update_trail
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ribbon_runs_from_the_tip_back_along_the_flight_and_fades() {
        let tip = Vec3::new(10.0, 20.0, 0.0);
        let dir = Vec3::X;
        let (positions, colors, indices) =
            build_ribbons(&[(tip, dir)], Vec3::new(10.0, 20.0, 15.0));
        assert_eq!(positions.len(), 4);
        assert_eq!(indices.len(), 6);
        // The tail is 5 m behind the tip.
        assert!((positions[2][0] - 5.0).abs() < 1e-4);
        // Turned toward the camera: the width is along Z... no, across both flight and view (Y).
        let width = Vec3::from(positions[1]) - Vec3::from(positions[0]);
        assert!(width.dot(dir).abs() < 1e-4 && width.dot(Vec3::Z).abs() < 1e-3);
        assert!((width.length() - 2.0 * HALF_WIDTH_M).abs() < 1e-4);
        assert_eq!(colors[0][3], TRAIL_ALPHA);
        assert_eq!(colors[2][3], 0.0);
    }

    #[test]
    fn the_ribbon_widens_with_the_distance_up_to_seven_times() {
        let width = |distance: f32| {
            let (p, _, _) = build_ribbons(&[(Vec3::ZERO, Vec3::X)], Vec3::new(0.0, 0.0, distance));
            (Vec3::from(p[1]) - Vec3::from(p[0])).length()
        };
        assert!((width(20.0) - 0.06).abs() < 1e-4);
        assert!((width(90.0) - 0.18).abs() < 1e-4);
        assert!((width(1000.0) - 0.42).abs() < 1e-4);
    }
}
