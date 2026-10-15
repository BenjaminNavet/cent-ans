//! Assault effects (port of `siege_assault_fx.gd`, `wall_collapse_fx.gd` dust, `siege_fire_fx.gd`
//! and the ladders of `battle_siege.gd`): short-lived particles for the events the core records
//! (`SiegeFx`), fire and smoke on the burning gate and houses, and the ladders of the regiments
//! scaling the walls.
//!
//! Particles are small shared-mesh entities that follow a ballistic path and shrink out (no alpha
//! blending: one shared material per kind); at most [`MAX_PARTICLES`] live at once.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::f32::consts::TAU;

use bevy::prelude::*;
use sim_battle::{FireState, SiegeFxKind};

use super::{SiegeFrame, SiegeStats};
use crate::app::AppState;
use crate::battle::decor::hash01;
use crate::battle::scene::BattleSession;

const MAX_PARTICLES: usize = 700;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Dust,
    Smoke,
    Flame,
    Debris,
    Oil,
}

#[derive(Component)]
pub struct Particle {
    pub kind: Kind,
    pub velocity: Vec3,
    pub gravity: f32,
    pub age: f32,
    pub life: f32,
    pub size: f32,
    /// Growth of a puff (dust, smoke) before it fades; 0: only shrinks.
    pub grow: f32,
}

#[derive(Resource)]
struct FxAssets {
    sphere: Handle<Mesh>,
    cube: Handle<Mesh>,
    dust: Handle<StandardMaterial>,
    smoke: Handle<StandardMaterial>,
    flame: Handle<StandardMaterial>,
    debris: Handle<StandardMaterial>,
    oil: Handle<StandardMaterial>,
    ladder: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct LadderView {
    pub unit: u32,
    pub index: usize,
}

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_assets).add_systems(
            Update,
            (
                spawn_event_particles,
                emit_fire,
                move_particles,
                sync_ladders,
            )
                .chain()
                .after(super::read_frame)
                .run_if(in_state(AppState::Battle))
                .run_if(resource_exists::<BattleSession>),
        );
    }
}

fn unlit(materials: &mut Assets<StandardMaterial>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        unlit: true,
        ..default()
    })
}

fn make_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(FxAssets {
        sphere: meshes.add(Sphere::new(0.5).mesh().ico(1).unwrap()),
        cube: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        dust: unlit(&mut materials, Color::srgb(0.62, 0.57, 0.48)),
        smoke: unlit(&mut materials, Color::srgb(0.22, 0.21, 0.2)),
        flame: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.55, 0.12),
            emissive: LinearRgba::rgb(6.0, 2.4, 0.4),
            unlit: true,
            ..default()
        }),
        debris: materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.47, 0.43),
            perceptual_roughness: 1.0,
            ..default()
        }),
        oil: unlit(&mut materials, Color::srgb(0.85, 0.62, 0.18)),
        ladder: materials.add(StandardMaterial {
            base_color: Color::srgb(0.43, 0.30, 0.17),
            perceptual_roughness: 0.9,
            ..default()
        }),
    });
}

fn spawn_particle(
    commands: &mut Commands,
    assets: &FxAssets,
    stats: &mut SiegeStats,
    live: &mut usize,
    kind: Kind,
    position: Vec3,
    velocity: Vec3,
    size: f32,
    life: f32,
) {
    if *live >= MAX_PARTICLES {
        return;
    }
    *live += 1;
    stats.particles_spawned += 1;
    let (mesh, material, gravity, grow) = match kind {
        Kind::Dust => (&assets.sphere, &assets.dust, -0.4, 1.6),
        Kind::Smoke => (&assets.sphere, &assets.smoke, -1.2, 1.3),
        Kind::Flame => (&assets.sphere, &assets.flame, -2.5, 0.0),
        Kind::Debris => (&assets.cube, &assets.debris, 14.0, 0.0),
        Kind::Oil => (&assets.sphere, &assets.oil, 9.0, 0.0),
    };
    commands.spawn((
        Name::new("SiegeParticle"),
        Particle {
            kind,
            velocity,
            gravity,
            age: 0.0,
            life,
            size,
            grow,
        },
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(position).with_scale(Vec3::splat(size * 0.2)),
        DespawnOnExit(AppState::Battle),
    ));
}

/// Directions around a point from a deterministic hash (no random stream).
fn spread(seed: Vec3, salt: f32) -> Vec3 {
    let a = hash01(seed.x, seed.z, salt) * TAU;
    let b = hash01(seed.z, seed.x, salt + 1.0);
    Vec3::new(a.cos() * b, 0.0, a.sin() * b)
}

fn spawn_event_particles(
    mut commands: Commands,
    assets: Option<Res<FxAssets>>,
    frame: Res<SiegeFrame>,
    mut stats: ResMut<SiegeStats>,
    particles: Query<(), With<Particle>>,
) {
    let Some(assets) = assets else { return };
    let mut live = particles.iter().count();
    for event in &frame.events {
        let p = event.pos;
        let mut burst =
            |kind: Kind, n: usize, speed: f32, up: f32, size: f32, life: f32, salt: f32| {
                for k in 0..n {
                    let s = salt + k as f32 * 1.7;
                    let dir = spread(p + Vec3::splat(k as f32), s);
                    let v = dir * speed + Vec3::Y * up * (0.5 + hash01(p.x + k as f32, p.z, s));
                    spawn_particle(
                        &mut commands,
                        &assets,
                        &mut stats,
                        &mut live,
                        kind,
                        p + dir * 0.5,
                        v,
                        size * (0.7 + hash01(p.z, p.x + k as f32, s) * 0.8),
                        life * (0.8 + hash01(p.x, p.z + k as f32, s + 3.0) * 0.5),
                    );
                }
            };
        match &event.kind {
            SiegeFxKind::EngineShot { breached, .. } => {
                burst(Kind::Dust, 5, 3.0, 2.0, 3.0, 1.8, 1.0);
                burst(Kind::Debris, 6, 6.0, 7.0, 0.5, 1.4, 2.0);
                if *breached {
                    burst(Kind::Dust, 10, 5.0, 2.5, 5.0, 3.0, 3.0);
                }
            }
            SiegeFxKind::RamStrike { breached, .. } => {
                burst(Kind::Dust, 4, 2.0, 1.0, 2.2, 1.4, 4.0);
                burst(Kind::Debris, 3, 4.0, 4.0, 0.4, 1.0, 5.0);
                if *breached {
                    burst(Kind::Dust, 10, 4.0, 2.0, 4.5, 2.8, 6.0);
                }
            }
            SiegeFxKind::WallBreached { .. } | SiegeFxKind::GateBroken { .. } => {
                // The collapse: a rolling cloud along the piece and falling stones.
                burst(Kind::Dust, 18, 6.0, 3.0, 7.0, 4.0, 7.0);
                burst(Kind::Debris, 14, 8.0, 9.0, 0.8, 2.0, 8.0);
                burst(Kind::Smoke, 4, 2.0, 3.0, 5.0, 4.5, 9.0);
            }
            SiegeFxKind::BoilingOil { .. } => {
                burst(Kind::Oil, 14, 3.5, 4.0, 0.45, 1.2, 10.0);
                burst(Kind::Smoke, 3, 1.5, 2.0, 2.5, 2.5, 11.0);
            }
            SiegeFxKind::TowerVolley { .. } => {}
            SiegeFxKind::TowerDocked { .. } => burst(Kind::Dust, 5, 2.5, 1.0, 3.0, 2.0, 12.0),
            _ => {}
        }
    }
}

/// Flames and smoke over the burning gate and houses (`siege_fire_fx.gd`, particles only).
fn emit_fire(
    mut commands: Commands,
    assets: Option<Res<FxAssets>>,
    time: Res<Time>,
    frame: Res<SiegeFrame>,
    mut stats: ResMut<SiegeStats>,
    particles: Query<(), With<Particle>>,
    mut session: ResMut<BattleSession>,
    mut carry: Local<f32>,
) {
    let Some(assets) = assets else { return };
    if !frame.active || session.paused {
        return;
    }
    // Emission at most 20 times a second.
    *carry += time.delta_secs();
    if *carry < 0.05 {
        return;
    }
    *carry = 0.0;
    let mut live = particles.iter().count();
    let sim = session.sim_mut();
    let Some(works) = sim.siege() else { return };
    let wall_h = works.wall_height as f32;
    let mut sources: Vec<(Vec3, f32, f32)> = Vec::new(); // position, intensity, size
    if let Some(fire) = frame.gate_fire
        && fire.state == FireState::Burning
    {
        let (mx, mz) = works.pieces[works.gate].midpoint();
        let base = sim.field().height(mx, mz) as f32;
        sources.push((
            Vec3::new(mx as f32, base + 2.5, mz as f32),
            fire.intensity,
            2.2,
        ));
    }
    for (house, fire) in works.houses.iter().zip(&frame.houses) {
        if fire.state == FireState::Burning {
            let base = sim.field().height(house.x, house.z) as f32;
            sources.push((
                Vec3::new(house.x as f32, base + 6.0, house.z as f32),
                fire.intensity,
                2.6,
            ));
        }
    }
    let _ = wall_h;
    for (index, (pos, intensity, size)) in sources.into_iter().enumerate() {
        let n = 1 + (intensity * 2.0) as usize;
        for k in 0..n {
            let salt = frame.elapsed * 7.0 + k as f32 + index as f32 * 5.0;
            let jitter = spread(pos, salt) * 2.0;
            spawn_particle(
                &mut commands,
                &assets,
                &mut stats,
                &mut live,
                Kind::Flame,
                pos + jitter,
                Vec3::new(jitter.x * 0.2, 3.0 + intensity * 3.0, jitter.z * 0.2),
                size * (0.5 + intensity * 0.7),
                0.9,
            );
        }
        if hash01(frame.elapsed, index as f32, 2.0) > 0.4 {
            spawn_particle(
                &mut commands,
                &assets,
                &mut stats,
                &mut live,
                Kind::Smoke,
                pos + Vec3::Y * 1.5,
                Vec3::new(1.2, 3.0, 0.6),
                size * 1.6,
                4.5,
            );
        }
    }
}

fn move_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform) in &mut particles {
        particle.age += dt;
        if particle.age >= particle.life {
            commands.entity(entity).despawn();
            continue;
        }
        let t = particle.age / particle.life;
        let gravity = particle.gravity;
        particle.velocity.y -= gravity * dt;
        if matches!(particle.kind, Kind::Dust | Kind::Smoke) {
            let drag = (1.0 - 1.5 * dt).max(0.0);
            particle.velocity.x *= drag;
            particle.velocity.z *= drag;
        }
        transform.translation += particle.velocity * dt;
        let scale = if particle.grow > 0.0 {
            // Grows quickly, then dissolves.
            let swell = 0.3 + particle.grow * (1.0 - (1.0 - t).powi(2)) * 0.7;
            swell * (1.0 - t.powi(3)) * particle.size
        } else {
            (1.0 - t * t) * particle.size
        };
        transform.scale = Vec3::splat(scale.max(0.01));
        if matches!(particle.kind, Kind::Debris) {
            transform.rotate_local(Quat::from_rotation_x(dt * 6.0));
        }
    }
}

/// One thin box per ladder of the core (`SiegeAssault`), leaning from foot to top.
fn sync_ladders(
    mut commands: Commands,
    assets: Option<Res<FxAssets>>,
    frame: Res<SiegeFrame>,
    mut views: Query<(Entity, &LadderView, &mut Transform)>,
) {
    let Some(assets) = assets else { return };
    if !frame.active {
        return;
    }
    let mut seen = vec![false; frame.ladders.len()];
    for (entity, view, mut transform) in &mut views {
        let found = frame
            .ladders
            .iter()
            .position(|l| l.unit == view.unit && l.index == view.index);
        match found {
            Some(i) => {
                seen[i] = true;
                *transform = ladder_transform(frame.ladders[i].foot, frame.ladders[i].top);
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (i, ladder) in frame.ladders.iter().enumerate() {
        if seen[i] {
            continue;
        }
        commands.spawn((
            Name::new("SiegeLadder"),
            LadderView {
                unit: ladder.unit,
                index: ladder.index,
            },
            Mesh3d(assets.cube.clone()),
            MeshMaterial3d(assets.ladder.clone()),
            ladder_transform(ladder.foot, ladder.top),
            DespawnOnExit(AppState::Battle),
        ));
    }
}

/// Unit cube stretched into a ladder (1 m wide, 8 cm thick) from `foot` to `top`.
fn ladder_transform(foot: Vec3, top: Vec3) -> Transform {
    let axis = top - foot;
    let length = axis.length().max(0.1);
    let mid = (foot + top) * 0.5;
    Transform::from_translation(mid)
        .with_rotation(Quat::from_rotation_arc(Vec3::Y, axis / length))
        .with_scale(Vec3::new(1.0, length, 0.08))
}
