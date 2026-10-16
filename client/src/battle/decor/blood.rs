//! Blood on the ground: persistent flat decals laid from the simulation (port of
//! game/scripts/battle/battle_blood.gd, decals part; the grass stain is in `flatten.rs`).
//!
//! A pool of at most [`MAX_DECALS`] soft blobs (pool, splash, trail) merged into one mesh that is
//! rebuilt a few times per second when new decals arrived. Each corner follows the ground height.
//! Sources: the bodies that fell ([`Losses::falls`]) and the trails behind regiments routing
//! after heavy losses. Setting `battle/blood`: 0 none, 1 moderate (smaller pools), 2 full
//! (splashes and trails). No pool in water. Not ported: the sprays of particles at impact.

use std::f32::consts::{PI, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::flatten::{Losses, collect_losses};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::units::corpses::hash4;

pub const MAX_DECALS: usize = 6000;
/// Seconds of wall time between two mesh rebuilds.
const REBUILD_S: f32 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DecalKind {
    Pool,
    Splash,
    Trail,
}

#[derive(Clone, Copy, Debug)]
pub struct Decal {
    pub pos: Vec2,
    pub size: Vec2,
    pub yaw: f32,
    pub kind: DecalKind,
    pub intensity: f32,
}

#[derive(Resource, Default)]
pub struct BloodDecals {
    pub decals: Vec<Decal>,
    pub added: usize,
    pub rebuilds: usize,
    dirty: bool,
    since_rebuild: f32,
    counter: u64,
    trail_at: std::collections::HashMap<u32, f64>,
}

impl BloodDecals {
    fn add(&mut self, decal: Decal) {
        self.added += 1;
        self.dirty = true;
        if self.decals.len() < MAX_DECALS {
            self.decals.push(decal);
        } else {
            let slot = (hash4(self.counter)[0] * MAX_DECALS as f32) as usize % MAX_DECALS;
            self.decals[slot] = decal;
        }
    }
}

#[derive(Component)]
struct BloodMesh;

pub struct BloodPlugin;

impl Plugin for BloodPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BloodDecals>()
            .add_systems(
                Update,
                (lay_decals, rebuild_mesh)
                    .chain()
                    .after(collect_losses)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.insert_resource(BloodDecals::default());
            });
    }
}

/// Pool size factor of the `blood` level: 0 none, 1 moderate (smaller pools), 2 full.
pub fn pool_scale(level: u32) -> f32 {
    match level {
        0 => 0.0,
        1 => 0.7,
        _ => 1.0,
    }
}

fn lay_decals(
    mut session: ResMut<BattleSession>,
    losses: Res<Losses>,
    mut blood: ResMut<BloodDecals>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    let level = settings.map_or(1, |s| s.blood);
    if level == 0 {
        return;
    }
    let now = session.sim_mut().elapsed();
    let field = session.sim_mut().field();
    let wet = |p: Vec2| field.water_at(f64::from(p.x), f64::from(p.y)).is_some();
    let mut new = Vec::new();
    for fall in &losses.falls {
        if wet(fall.pos) {
            continue;
        }
        blood.counter += 1;
        let h = hash4(blood.counter);
        let size = (0.7 + 0.7 * h[0]) * pool_scale(level) * if fall.cavalry { 1.4 } else { 1.0 };
        new.push(Decal {
            pos: fall.pos,
            size: Vec2::new(size, size * (0.7 + 0.3 * h[1])),
            yaw: h[2] * TAU,
            kind: DecalKind::Pool,
            intensity: 1.0,
        });
        if level >= 2 && h[3] < 0.5 {
            let off = Vec2::new(h[0] - 0.5, h[1] - 0.5) * 1.6;
            new.push(Decal {
                pos: fall.pos + off,
                size: Vec2::splat(1.0 + 0.8 * h[2]),
                yaw: h[3] * TAU,
                kind: DecalKind::Splash,
                intensity: 0.9,
            });
        }
    }
    // Trails behind regiments routing after heavy losses.
    if level >= 2 {
        for &(id, pos, frontage, depth, facing, _) in &losses.fleeing {
            let due = blood.trail_at.get(&id).copied().unwrap_or(0.0);
            if now < due {
                continue;
            }
            blood.trail_at.insert(id, now + 1.2);
            let forward = Vec2::new(facing.sin(), facing.cos());
            let right = Vec2::new(facing.cos(), -facing.sin());
            for _ in 0..2 {
                blood.counter += 1;
                let h = hash4(blood.counter);
                let p = pos + right * frontage * 0.5 * (h[0] * 1.6 - 0.8)
                    - forward * depth * 0.5 * h[1];
                if wet(p) {
                    continue;
                }
                new.push(Decal {
                    pos: p,
                    size: Vec2::new(1.6 + 1.6 * h[2], 0.35 + 0.25 * h[3]),
                    yaw: facing.sin().atan2(facing.cos()) - PI * 0.5 + (h[0] - 0.5) * 0.6,
                    kind: DecalKind::Trail,
                    intensity: 0.8,
                });
            }
        }
    }
    for decal in new {
        blood.add(decal);
    }
}

/// Soft blob (8-gon, opaque centre, transparent rim) of a decal, corners on the ground.
fn push_decal(
    decal: &Decal,
    field: &sim_battle::Battlefield,
    positions: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let ground = |p: Vec2| field.height(f64::from(p.x), f64::from(p.y)) as f32 + 0.05;
    let (sin, cos) = decal.yaw.sin_cos();
    let axis_x = Vec2::new(cos, sin) * decal.size.x * 0.5;
    let axis_z = Vec2::new(-sin, cos) * decal.size.y * 0.5;
    let base = positions.len() as u32;
    let (r, g, b) = match decal.kind {
        DecalKind::Pool => (0.26, 0.02, 0.02),
        DecalKind::Splash => (0.32, 0.03, 0.02),
        DecalKind::Trail => (0.30, 0.03, 0.02),
    };
    positions.push([decal.pos.x, ground(decal.pos), decal.pos.y]);
    colors.push([r, g, b, 0.85 * decal.intensity]);
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        // Lobed rim so pools are not perfect discs.
        let wobble = 0.85 + 0.15 * (a * 3.0 + decal.yaw * 5.0).sin();
        let p = decal.pos + (axis_x * a.cos() + axis_z * a.sin()) * 2.0 * wobble;
        positions.push([p.x, ground(p), p.y]);
        colors.push([r, g, b, 0.0]);
    }
    for k in 0..8u32 {
        indices.extend_from_slice(&[base, base + 1 + k, base + 1 + (k + 1) % 8]);
    }
}

fn rebuild_mesh(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<BattleSession>,
    mut blood: ResMut<BloodDecals>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<&Mesh3d, With<BloodMesh>>,
) {
    blood.since_rebuild += time.delta_secs();
    if !blood.dirty || (blood.since_rebuild < REBUILD_S && !existing.is_empty()) {
        return;
    }
    blood.dirty = false;
    blood.since_rebuild = 0.0;
    blood.rebuilds += 1;
    let field = session.sim_mut().field();
    let (mut positions, mut colors, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    for decal in &blood.decals {
        push_decal(decal, field, &mut positions, &mut colors, &mut indices);
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices));
    if let Ok(handle) = existing.single() {
        let _ = meshes.insert(&handle.0, mesh);
    } else {
        commands.spawn((
            Name::new("BloodDecals"),
            BloodMesh,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.35,
                reflectance: 0.3,
                depth_bias: 2.0,
                cull_mode: None,
                ..default()
            })),
            Transform::default(),
            NotShadowCaster,
            DespawnOnExit(AppState::Battle),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blood_level_scales_the_pools() {
        assert_eq!(pool_scale(0), 0.0);
        assert!(pool_scale(1) < pool_scale(2));
        assert_eq!(pool_scale(2), 1.0);
        assert_eq!(pool_scale(7), 1.0, "levels above full stay full");
    }

    #[test]
    fn pool_replaces_at_random_when_full() {
        let mut blood = BloodDecals::default();
        let decal = Decal {
            pos: Vec2::ZERO,
            size: Vec2::ONE,
            yaw: 0.0,
            kind: DecalKind::Pool,
            intensity: 1.0,
        };
        for _ in 0..MAX_DECALS + 10 {
            blood.add(decal);
        }
        assert_eq!(blood.decals.len(), MAX_DECALS);
        assert_eq!(blood.added, MAX_DECALS + 10);
    }
}
