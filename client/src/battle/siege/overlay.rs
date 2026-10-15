//! Health bars and capture points (ports of `siege_health_bars.gd` and `siege_capture_points.gd`).
//!
//! - Bars: over every ram, siege tower and static engine that lost crew, and over every wall
//!   piece that is damaged or under attack; camera-facing, wider with the distance so they stay
//!   readable. Colour by share (green / amber / red).
//! - Capture points (market square, gate): a ring draped on the relief, coloured by who holds
//!   it, an inner ring and a pole whose height follows the progress of the attacker.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::collections::HashMap;
use std::f32::consts::TAU;

use bevy::prelude::*;
use sim_battle::{PointStatus, SideId};

use super::{EngineKind, SiegeFrame, SiegeStats};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum BarKey {
    Engine(u32),
    Piece(usize),
}

#[derive(Component)]
struct BarView {
    fill: Entity,
    tier: usize,
}

#[derive(Resource)]
struct BarAssets {
    quad: Handle<Mesh>,
    back: Handle<StandardMaterial>,
    tiers: [Handle<StandardMaterial>; 3],
}

#[derive(Resource, Default)]
struct BarIndex(HashMap<BarKey, Entity>);

pub struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BarIndex>()
            .add_systems(Startup, make_bar_assets)
            .add_systems(
                Update,
                (sync_bars, draw_points)
                    .after(super::read_frame)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(OnExit(AppState::Battle), |mut index: ResMut<BarIndex>| {
                index.0.clear();
            });
    }
}

fn make_bar_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let flat = |materials: &mut Assets<StandardMaterial>, c: Color| {
        materials.add(StandardMaterial {
            base_color: c,
            unlit: true,
            cull_mode: None,
            ..default()
        })
    };
    commands.insert_resource(BarAssets {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
        back: flat(&mut materials, Color::srgba(0.05, 0.05, 0.05, 1.0)),
        tiers: [
            flat(&mut materials, Color::srgb(0.30, 0.78, 0.30)),
            flat(&mut materials, Color::srgb(0.92, 0.70, 0.18)),
            flat(&mut materials, Color::srgb(0.86, 0.20, 0.16)),
        ],
    });
}

fn tier_of(share: f32) -> usize {
    if share > 0.5 {
        0
    } else if share > 0.25 {
        1
    } else {
        2
    }
}

/// Height of the bar above an engine's origin (m).
fn engine_top(kind: &EngineKind) -> f32 {
    match kind {
        EngineKind::Tower => 15.0,
        EngineKind::Ram => 6.0,
        EngineKind::Static(_) => 9.0,
    }
}

fn sync_bars(
    mut commands: Commands,
    assets: Option<Res<BarAssets>>,
    frame: Res<SiegeFrame>,
    mut index: ResMut<BarIndex>,
    mut stats: ResMut<SiegeStats>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    mut bars: Query<(&mut BarView, &mut Transform), Without<BattleCamera>>,
    mut fills: Query<
        (&mut Transform, &mut MeshMaterial3d<StandardMaterial>),
        (Without<BarView>, Without<BattleCamera>),
    >,
) {
    let Some(assets) = assets else { return };
    if !frame.active {
        return;
    }
    let Ok(camera) = camera.single() else { return };
    let (eye, facing) = (camera.translation(), camera.rotation());
    // What needs a bar: (key, position, share).
    let mut wanted: Vec<(BarKey, Vec3, f32)> = Vec::new();
    for engine in &frame.engines {
        let share = (engine.hp / engine.max_hp).clamp(0.0, 1.0);
        if share < 0.999 && engine.hp > 0.0 {
            wanted.push((
                BarKey::Engine(engine.unit),
                engine.pos + Vec3::Y * engine_top(&engine.kind),
                share,
            ));
        }
    }
    for (i, piece) in frame.pieces.iter().enumerate() {
        let share = if piece.max_hp > 0.0 {
            (piece.hp / piece.max_hp).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if piece.hp > 0.0 && (piece.under_attack || share < 0.999) {
            wanted.push((BarKey::Piece(i), piece.top, share));
        }
    }
    for (key, position, share) in &wanted {
        let distance = eye.distance(*position);
        // Width in metres: constant on screen up to a limit.
        let width = (distance * 0.045).clamp(3.0, 60.0);
        let transform = Transform::from_translation(*position)
            .with_rotation(facing)
            .with_scale(Vec3::new(width, width * 0.14, 1.0));
        let tier = tier_of(*share);
        if let Some(entity) = index.0.get(key).copied()
            && let Ok((mut view, mut t)) = bars.get_mut(entity)
        {
            *t = transform;
            if let Ok((mut fill_t, mut material)) = fills.get_mut(view.fill) {
                fill_t.scale.x = share.max(0.01);
                fill_t.translation.x = -0.5 + share.max(0.01) * 0.5;
                if view.tier != tier {
                    material.0 = assets.tiers[tier].clone();
                    view.tier = tier;
                }
            }
            continue;
        }
        let fill =
            commands
                .spawn((
                    Name::new("SiegeBarFill"),
                    Mesh3d(assets.quad.clone()),
                    MeshMaterial3d(assets.tiers[tier].clone()),
                    Transform::from_xyz(-0.5 + share.max(0.01) * 0.5, 0.0, 0.01)
                        .with_scale(Vec3::new(share.max(0.01), 0.8, 1.0)),
                ))
                .id();
        let back = commands
            .spawn((
                Name::new("SiegeBarBack"),
                Mesh3d(assets.quad.clone()),
                MeshMaterial3d(assets.back.clone()),
                Transform::default(),
            ))
            .id();
        let root = commands
            .spawn((
                Name::new("SiegeBar"),
                BarView { fill, tier },
                transform,
                Visibility::default(),
                DespawnOnExit(AppState::Battle),
            ))
            .add_children(&[back, fill])
            .id();
        index.0.insert(*key, root);
    }
    // Bars of what no longer needs one.
    let keep: Vec<BarKey> = wanted.iter().map(|w| w.0).collect();
    index.0.retain(|key, entity| {
        let keep = keep.contains(key);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
    stats.bars = index.0.len();
}

const RING_SAMPLES: usize = 40;

/// Capture points: ring + progress ring + pole, drawn with gizmos draped on the relief.
fn draw_points(mut gizmos: Gizmos, frame: Res<SiegeFrame>, mut session: ResMut<BattleSession>) {
    if !frame.active {
        return;
    }
    let sim = session.sim_mut();
    let field = sim.field();
    let height = |x: f32, z: f32| field.height(f64::from(x), f64::from(z)) as f32 + 0.5;
    let player_attacks = sim.setup().player_side != Some(SideId::Defender);
    for point in &frame.points {
        let (x, z) = (point.x as f32, point.z as f32);
        let radius = point.radius as f32;
        let held_color = if player_attacks {
            Color::srgb(0.85, 0.22, 0.16)
        } else {
            Color::srgb(0.25, 0.45, 0.9)
        };
        let taker_color = if player_attacks {
            Color::srgb(0.25, 0.45, 0.9)
        } else {
            Color::srgb(0.85, 0.22, 0.16)
        };
        let color = match point.status {
            PointStatus::Held => held_color,
            PointStatus::Contested => Color::srgb(0.95, 0.75, 0.2),
            PointStatus::Capturing => taker_color.mix(&Color::WHITE, 0.25),
            PointStatus::Taken => Color::srgb(0.3, 0.8, 0.35),
        };
        let ring = |r: f32, from: f32, span: f32| -> Vec<Vec3> {
            let n = ((RING_SAMPLES as f32 * span / TAU).ceil() as usize).max(2);
            (0..=n)
                .map(|k| {
                    let a = from + span * k as f32 / n as f32;
                    let (px, pz) = (x + a.cos() * r, z + a.sin() * r);
                    Vec3::new(px, height(px, pz), pz)
                })
                .collect()
        };
        gizmos.linestrip(ring(radius, 0.0, TAU), color);
        let share = point.share() as f32;
        if share > 0.005 {
            gizmos.linestrip(ring(radius * 0.85, -TAU * 0.25, TAU * share), taker_color);
        }
        let base = Vec3::new(x, height(x, z), z);
        gizmos.line(base, base + Vec3::Y * (6.0 + 8.0 * share), color);
    }
}
