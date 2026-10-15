//! Site features beyond the merged volumes of `site.rs` (port of the matching parts of
//! game/scripts/battle/battle_decor.gd and battle_site_features.gd):
//! - camp fires: flickering flame tongues and a pale smoke source for two fires per camp (none in
//!   rain or snow), fed to `units::smoke::SmokeSources`;
//! - moats: water ring round a manor, following the ditch but never below its lowest point;
//! - the sea of coastal fields: one plane from the waterline outwards (near and far sea) at the
//!   field's sea level (`BattleSiteFeatures.SEA_LEVEL` = 0).
//!
//! Not ported: the animated flame / water / sea shaders (plain materials here), night glow of
//! the fires, loot of a camp, churchyard walls, horses of the horse lines (the horse models
//! belong to the units lot), the BuildingKit models for houses (`site.rs` keeps its volumes).

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::VisibilityRange;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use sim_battle::{Battlefield, DecorPropKind, Flank, Weather};

use super::hash01;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::units::smoke::SmokeSources;

/// Smoking fires per camp.
const SMOKING_FIRES_PER_CAMP: usize = 2;
/// Sea level of coastal fields (m).
pub const SEA_LEVEL: f32 = 0.0;
/// How far the sea extends beyond the waterline (m).
const SEA_REACH: f32 = 6000.0;
/// Flames hide beyond this distance (m).
const FIRE_RANGE: f32 = 320.0;

#[derive(Resource, Debug, Clone, Default)]
pub struct FeatureStats {
    pub fires: usize,
    pub smoking_fires: usize,
    pub moats: usize,
    pub sea: bool,
    pub sea_area_m2: f32,
}

#[derive(Component)]
struct Flame {
    phase: f32,
}

pub struct FeaturesPlugin;

impl Plugin for FeaturesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            build_features.run_if(resource_added::<BattleSession>),
        )
        .add_systems(
            Update,
            flicker
                .run_if(in_state(AppState::Battle))
                .run_if(any_with_component::<Flame>),
        )
        .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
            commands.remove_resource::<FeatureStats>();
        });
    }
}

/// Three flame tongues (a fan), foot yellow and opaque, tip orange and transparent.
fn flame_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    for (k, (height, width)) in [(1.3f32, 0.32f32), (0.95, 0.26), (0.8, 0.24)]
        .into_iter()
        .enumerate()
    {
        let angle = k as f32 * TAU / 3.0 + 0.4;
        let along = Vec3::new(angle.cos(), 0.0, angle.sin());
        let base = positions.len() as u32;
        positions.push((-along * width).to_array());
        positions.push((along * width).to_array());
        positions.push((along * width * 0.15 + Vec3::Y * height).to_array());
        colors.push([1.0, 0.85, 0.3, 1.0]);
        colors.push([1.0, 0.85, 0.3, 1.0]);
        colors.push([1.0, 0.35, 0.05, 0.0]);
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn flicker(time: Res<Time>, mut flames: Query<(&Flame, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (flame, mut transform) in &mut flames {
        let k = 1.0
            + 0.18 * (t * 9.0 + flame.phase * TAU).sin()
            + 0.1 * (t * 17.0 + flame.phase * 11.0).sin();
        transform.scale = Vec3::new(1.0 + 0.05 * (t * 7.0 + flame.phase).cos(), k, 1.0);
        transform.rotation = Quat::from_rotation_y(t * 0.6 + flame.phase * TAU);
    }
}

/// Ring of a moat as quads (outer rectangle minus inner), water following the ground.
pub fn moat_mesh(moat: &sim_battle::Moat, field: &Battlefield) -> Mesh {
    let (hl, hw, ring) = (
        moat.length as f32 * 0.5,
        moat.width as f32 * 0.5,
        moat.ring as f32,
    );
    let yaw = moat.yaw as f32;
    let centre = Vec2::new(moat.x as f32, moat.z as f32);
    let rot = |v: Vec2| {
        let (s, c) = yaw.sin_cos();
        centre + Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
    };
    let ground = |p: Vec2| field.height(f64::from(p.x), f64::from(p.y)) as f32;
    let mut level = f32::INFINITY;
    for k in 0..16 {
        let a = TAU * k as f32 / 16.0;
        level = level.min(ground(rot(Vec2::new(a.cos() * hl, a.sin() * hw))));
    }
    let outer = [
        Vec2::new(-hl, -hw),
        Vec2::new(hl, -hw),
        Vec2::new(hl, hw),
        Vec2::new(-hl, hw),
    ];
    let inner = [
        Vec2::new(-hl + ring, -hw + ring),
        Vec2::new(hl - ring, -hw + ring),
        Vec2::new(hl - ring, hw - ring),
        Vec2::new(-hl + ring, hw - ring),
    ];
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices = Vec::new();
    for side in 0..4 {
        let (o0, o1) = (rot(outer[side]), rot(outer[(side + 1) % 4]));
        let (i0, i1) = (rot(inner[side]), rot(inner[(side + 1) % 4]));
        let steps = ((o0.distance(o1) / 4.0) as usize).max(1);
        for k in 0..steps {
            let (t0, t1) = (k as f32 / steps as f32, (k + 1) as f32 / steps as f32);
            let quad = [
                o0.lerp(o1, t0),
                o0.lerp(o1, t1),
                i0.lerp(i1, t1),
                i0.lerp(i1, t0),
            ];
            let base = positions.len() as u32;
            for q in quad {
                positions.push([q.x, level.max(ground(q) - 0.25), q.y]);
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

/// Rectangle (x0, x1, z0, z1) of the sea of a coast, `None` for fields without one.
pub fn sea_rect(field: &Battlefield) -> Option<[f32; 4]> {
    let coast = field.coast.as_ref()?;
    let shore = coast.shore_x as f32;
    let (x0, x1) = match coast.flank {
        Flank::West => (shore - SEA_REACH, shore),
        Flank::East => (shore, shore + SEA_REACH),
    };
    Some([
        x0,
        x1,
        -SEA_REACH * 0.7,
        field.depth as f32 + SEA_REACH * 0.7,
    ])
}

fn quad_mesh(rect: [f32; 4], y: f32) -> Mesh {
    let [x0, x1, z0, z1] = rect;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1]],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4])
    .with_inserted_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]))
}

fn build_features(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut smoke: Option<ResMut<SmokeSources>>,
) {
    let sim = session.sim_mut();
    let weather = sim.weather();
    let field: &Battlefield = sim.field();
    let scope = DespawnOnExit(AppState::Battle);
    let mut stats = FeatureStats::default();
    let ground = |x: f64, z: f64| field.height(x, z) as f32;

    // Camp fires.
    let flame = meshes.add(flame_mesh());
    let flame_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    });
    let dry = !matches!(weather, Weather::Rain | Weather::Snow);
    let mut fire_sets: Vec<Vec<Vec3>> = Vec::new();
    for camp in &field.decor.camps {
        fire_sets.push(
            camp.items
                .iter()
                .filter(|p| p.kind == DecorPropKind::Campfire)
                .map(|p| Vec3::new(p.x as f32, ground(p.x, p.z), p.z as f32))
                .collect(),
        );
    }
    for (index, fires) in fire_sets.iter().enumerate() {
        for (i, fire) in fires.iter().enumerate() {
            stats.fires += 1;
            commands.spawn((
                Name::new("CampFlame"),
                Mesh3d(flame.clone()),
                MeshMaterial3d(flame_material.clone()),
                Transform::from_translation(*fire + Vec3::Y * 0.25),
                Flame {
                    phase: hash01(fire.x, fire.z, index as f32),
                },
                NotShadowCaster,
                VisibilityRange::abrupt(0.0, FIRE_RANGE),
                scope.clone(),
            ));
            let _ = i;
        }
        if dry && let Some(smoke) = smoke.as_deref_mut() {
            let n = SMOKING_FIRES_PER_CAMP.min(fires.len());
            for i in 0..n {
                let fire = fires[((i as f32 + 0.5) * fires.len() as f32 / n as f32) as usize];
                let intensity = 0.6 + 0.4 * hash01(i as f32, index as f32, 73.0);
                if smoke
                    .add(fire + Vec3::Y * 0.6, intensity, "campfire")
                    .is_some()
                {
                    stats.smoking_fires += 1;
                }
            }
        }
    }

    // Moats.
    if !field.decor.moats.is_empty() {
        let water = materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.2, 0.12),
            perceptual_roughness: 0.08,
            reflectance: 0.6,
            cull_mode: None,
            ..default()
        });
        for moat in &field.decor.moats {
            commands.spawn((
                Name::new("Moat"),
                Mesh3d(meshes.add(moat_mesh(moat, field))),
                MeshMaterial3d(water.clone()),
                Transform::default(),
                NotShadowCaster,
                scope.clone(),
            ));
            stats.moats += 1;
        }
    }

    // Sea of a coastal field.
    if let Some(rect) = sea_rect(field) {
        stats.sea = true;
        stats.sea_area_m2 = (rect[1] - rect[0]) * (rect[3] - rect[2]);
        commands.spawn((
            Name::new("Sea"),
            Mesh3d(meshes.add(quad_mesh(rect, SEA_LEVEL))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.14, 0.18),
                perceptual_roughness: 0.12,
                reflectance: 0.7,
                cull_mode: None,
                ..default()
            })),
            Transform::default(),
            NotShadowCaster,
            scope.clone(),
        ));
    }
    commands.insert_resource(stats);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flame_has_three_tongues() {
        let mesh = flame_mesh();
        assert_eq!(mesh.indices().map(|i| i.len()), Some(9));
    }

    #[test]
    fn quad_covers_the_rectangle() {
        let mesh = quad_mesh([0.0, 10.0, 0.0, 5.0], 0.0);
        assert_eq!(mesh.count_vertices(), 4);
    }
}
