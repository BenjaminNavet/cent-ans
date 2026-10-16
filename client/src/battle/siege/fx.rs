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

use super::{EngineKind, SiegeFrame, SiegeStats};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::decor::hash01;
use crate::battle::scene::BattleSession;
use crate::battle::units::wind::BattleWind;
use crate::fx::flipbook::{
    FlipbookData, FlipbookEmitter, FlipbookKind, FlipbookLooks, FlipbookMaterial, SmokeSprite,
    spawn_smoke_sprite,
};

const MAX_PARTICLES: usize = 700;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Dust,
    Smoke,
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
    debris: Handle<StandardMaterial>,
    oil: Handle<StandardMaterial>,
    ladder: Handle<StandardMaterial>,
    stone: Handle<StandardMaterial>,
    ball: Handle<StandardMaterial>,
}

/// An engine's stone or ball in flight (`siege_assault_fx.gd` `_launch_stone`).
#[derive(Component)]
pub struct Flight {
    start: Vec3,
    end: Vec3,
    age: f32,
    flight: f32,
    /// Apex height above the chord (m).
    arc: f32,
    bombard: bool,
    breached: bool,
    /// Trail puffs owed.
    carry: f32,
}

/// Counters of the flights for the dump.
#[derive(Resource, Default, Debug)]
pub struct FlightStats {
    pub launched: usize,
    pub landed: usize,
    pub trail_puffs: usize,
    pub bombards: usize,
    pub in_flight: usize,
    /// Longest flight (s) and highest apex (m) seen.
    pub longest_s: f32,
    pub apex_m: f32,
}

#[derive(Component)]
pub struct LadderView {
    pub unit: u32,
    pub index: usize,
}

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlightStats>()
            .register_dump(dump_flights)
            .add_systems(Startup, make_assets)
            .add_systems(
                OnExit(AppState::Battle),
                |mut stats: ResMut<FlightStats>| *stats = FlightStats::default(),
            )
            .add_systems(
                Update,
                (
                    spawn_event_particles,
                    launch_flights,
                    move_flights,
                    sync_fire_emitters,
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
        stone: materials.add(StandardMaterial {
            base_color: Color::srgb(0.42, 0.40, 0.37),
            perceptual_roughness: 0.95,
            ..default()
        }),
        ball: materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.12, 0.13),
            perceptual_roughness: 0.6,
            metallic: 0.5,
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

/// A burst of `n` particles around `p` (deterministic spread from `salt`).
fn burst_at(
    commands: &mut Commands,
    assets: &FxAssets,
    stats: &mut SiegeStats,
    live: &mut usize,
    p: Vec3,
    (kind, n, speed, up, size, life, salt): (Kind, usize, f32, f32, f32, f32, f32),
) {
    for k in 0..n {
        let s = salt + k as f32 * 1.7;
        let dir = spread(p + Vec3::splat(k as f32), s);
        let v = dir * speed + Vec3::Y * up * (0.5 + hash01(p.x + k as f32, p.z, s));
        spawn_particle(
            commands,
            assets,
            stats,
            live,
            kind,
            p + dir * 0.5,
            v,
            size * (0.7 + hash01(p.z, p.x + k as f32, s) * 0.8),
            life * (0.8 + hash01(p.x, p.z + k as f32, s + 3.0) * 0.5),
        );
    }
}

/// What an engine stone does where it lands: a puff of dust, flying stones, and on a breach a
/// big rolling cloud.
fn engine_impact(
    commands: &mut Commands,
    assets: &FxAssets,
    stats: &mut SiegeStats,
    live: &mut usize,
    p: Vec3,
    breached: bool,
) {
    burst_at(
        commands,
        assets,
        stats,
        live,
        p,
        (Kind::Dust, 5, 3.0, 2.0, 3.0, 1.8, 1.0),
    );
    burst_at(
        commands,
        assets,
        stats,
        live,
        p,
        (Kind::Debris, 6, 6.0, 7.0, 0.5, 1.4, 2.0),
    );
    if breached {
        burst_at(
            commands,
            assets,
            stats,
            live,
            p,
            (Kind::Dust, 10, 5.0, 2.5, 5.0, 3.0, 3.0),
        );
    }
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
        let mut burst = |spec: (Kind, usize, f32, f32, f32, f32, f32)| {
            burst_at(&mut commands, &assets, &mut stats, &mut live, p, spec);
        };
        match &event.kind {
            // Drawn where the stone lands (`flights`); without a flying stone, at once.
            SiegeFxKind::EngineShot { unit, breached, .. } => {
                if !frame.engines.iter().any(|e| e.unit == *unit) {
                    engine_impact(&mut commands, &assets, &mut stats, &mut live, p, *breached);
                }
            }
            SiegeFxKind::RamStrike { breached, .. } => {
                burst((Kind::Dust, 4, 2.0, 1.0, 2.2, 1.4, 4.0));
                burst((Kind::Debris, 3, 4.0, 4.0, 0.4, 1.0, 5.0));
                if *breached {
                    burst((Kind::Dust, 10, 4.0, 2.0, 4.5, 2.8, 6.0));
                }
            }
            SiegeFxKind::WallBreached { .. } | SiegeFxKind::GateBroken { .. } => {
                // The collapse: a rolling cloud along the piece and falling stones.
                burst((Kind::Dust, 18, 6.0, 3.0, 7.0, 4.0, 7.0));
                burst((Kind::Debris, 14, 8.0, 9.0, 0.8, 2.0, 8.0));
                burst((Kind::Smoke, 4, 2.0, 3.0, 5.0, 4.5, 9.0));
            }
            SiegeFxKind::BoilingOil { .. } => {
                burst((Kind::Oil, 14, 3.5, 4.0, 0.45, 1.2, 10.0));
                burst((Kind::Smoke, 3, 1.5, 2.0, 2.5, 2.5, 11.0));
            }
            SiegeFxKind::TowerVolley { .. } => {}
            SiegeFxKind::TowerDocked { .. } => burst((Kind::Dust, 5, 2.5, 1.0, 3.0, 2.0, 12.0)),
            _ => {}
        }
    }
}

/// Flights alive at once (`MAX_FLIGHTS`).
const MAX_FLIGHTS: usize = 12;
/// Arc of a stone's path, share of the distance (`STONE_ARC`); a bombard ball flies straight.
const STONE_ARC: f32 = 0.22;
/// Trail puffs per second behind a stone (160 particles over 2.2 s in Godot).
const TRAIL_PER_S: f32 = 70.0;

/// A stone (trebuchet, mangonel) or ball (bombard) leaves its engine for the struck point of the
/// wall. Start points are the fallback of `_on_engine_shot` (no release point of the animated
/// model yet): above the sling, or at the muzzle.
fn launch_flights(
    mut commands: Commands,
    assets: Option<Res<FxAssets>>,
    frame: Res<SiegeFrame>,
    mut stats: ResMut<FlightStats>,
    mut fx_stats: ResMut<SiegeStats>,
    mut flashes: ResMut<crate::battle::effects::flash::MuzzleFlashes>,
    mut smoke: ResMut<crate::battle::units::smoke::SmokeRequests>,
    flights: Query<(), With<Flight>>,
    particles: Query<(), With<Particle>>,
) {
    let Some(assets) = assets else { return };
    let mut live_flights = flights.iter().count();
    let mut live = particles.iter().count();
    for event in &frame.events {
        let SiegeFxKind::EngineShot { unit, breached, .. } = &event.kind else {
            continue;
        };
        let Some(engine) = frame.engines.iter().find(|e| e.unit == *unit) else {
            continue;
        };
        if live_flights >= MAX_FLIGHTS {
            // Too many in the air: this one lands at once.
            engine_impact(
                &mut commands,
                &assets,
                &mut fx_stats,
                &mut live,
                event.pos,
                *breached,
            );
            continue;
        }
        let bombard =
            matches!(&engine.kind, EngineKind::Static(model) if model.contains("bombard"));
        let mut out = Vec3::new(engine.pos.x - event.pos.x, 0.0, engine.pos.z - event.pos.z);
        out = out.normalize_or_zero();
        // A stone strikes the outer face of the wall (half the thickness in).
        let end = event.pos + out * 1.5;
        let mut dir =
            Vec3::new(end.x - engine.pos.x, 0.0, end.z - engine.pos.z).normalize_or_zero();
        if dir == Vec3::ZERO {
            dir = -out;
        }
        let start = if bombard {
            engine.pos + Vec3::Y * 0.85 + dir * 1.4
        } else {
            engine.pos + Vec3::Y * 9.0 - dir * 2.0
        };
        if bombard {
            // The flash and the lingering smoke of the muzzle (`cannon_fire`).
            flashes.0.push((start, dir));
            smoke.bombard.push(start);
        }
        let distance = start.distance(end);
        let (speed, low, high) = if bombard {
            (140.0, 0.6, 3.5)
        } else {
            (60.0, 1.5, 3.5)
        };
        let flight = (distance / speed).clamp(low, high);
        let arc = if bombard { 0.0 } else { distance * STONE_ARC };
        let (mesh, material, scale) = if bombard {
            (&assets.sphere, &assets.ball, 0.9)
        } else {
            (&assets.sphere, &assets.stone, 1.6)
        };
        commands.spawn((
            Name::new("SiegeFlight"),
            Flight {
                start,
                end,
                age: 0.0,
                flight,
                arc,
                bombard,
                breached: *breached,
                carry: 0.0,
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(start).with_scale(Vec3::splat(scale)),
            DespawnOnExit(AppState::Battle),
        ));
        live_flights += 1;
        stats.launched += 1;
        stats.bombards += usize::from(bombard);
        stats.longest_s = stats.longest_s.max(flight);
        stats.apex_m = stats.apex_m.max(arc);
    }
}

/// Moves the flying stones along their arc, leaves a trail of pale puffs, and on landing throws
/// the dust and the stones of the impact.
fn move_flights(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<BattleSession>,
    assets: Option<Res<FxAssets>>,
    looks: Option<ResMut<FlipbookLooks>>,
    mut materials: ResMut<Assets<FlipbookMaterial>>,
    mut fx_stats: ResMut<SiegeStats>,
    mut stats: ResMut<FlightStats>,
    mut flights: Query<(Entity, &mut Flight, &mut Transform)>,
    particles: Query<(), With<Particle>>,
) {
    let Some(assets) = assets else { return };
    let dt = if session.paused {
        0.0
    } else {
        time.delta_secs() * session.speed as f32
    };
    let mut looks = looks;
    let mut live = particles.iter().count();
    stats.in_flight = 0;
    for (entity, mut flight, mut transform) in &mut flights {
        flight.age += dt;
        let t = flight.age / flight.flight;
        if t >= 1.0 {
            engine_impact(
                &mut commands,
                &assets,
                &mut fx_stats,
                &mut live,
                flight.end + Vec3::new(0.0, 0.0, 0.0),
                flight.breached,
            );
            // Smoke of the strike, low at the foot of the wall.
            burst_at(
                &mut commands,
                &assets,
                &mut fx_stats,
                &mut live,
                flight.end - Vec3::Y * 2.0,
                (Kind::Smoke, 2, 1.5, 1.0, 2.5, 2.0, 21.0),
            );
            stats.landed += 1;
            commands.entity(entity).despawn();
            continue;
        }
        stats.in_flight += 1;
        let mut position = flight.start.lerp(flight.end, t);
        position.y += flight.arc * 4.0 * t * (1.0 - t);
        transform.translation = position;
        transform.rotation = Quat::from_euler(EulerRot::XYZ, t * 9.0, t * 5.0, 0.0);
        flight.carry += dt * TRAIL_PER_S;
        while flight.carry >= 1.0 {
            flight.carry -= 1.0;
            let Some(looks) = looks.as_deref_mut() else {
                flight.carry = 0.0;
                break;
            };
            let (life, size) = if flight.bombard {
                (1.0, 2.6)
            } else {
                (2.2, 1.9)
            };
            let sprite = SmokeSprite {
                position,
                velocity: Vec3::Y * 0.2,
                accel: Vec3::Y * 0.6,
                damping: 0.2,
                life,
                size,
                spin: flight.age * 3.0,
                spin_rate: 0.1,
                tint: Some([0.86, 0.84, 0.8, 0.5]),
            };
            if let Some(puff) = spawn_smoke_sprite(&mut commands, looks, &mut materials, &sprite) {
                commands
                    .entity(puff)
                    .insert(DespawnOnExit(AppState::Battle));
                stats.trail_puffs += 1;
            }
        }
    }
}

fn dump_flights(world: &World) -> (String, serde_json::Value) {
    let Some(s) = world.get_resource::<FlightStats>() else {
        return ("siege_flights".into(), serde_json::Value::Null);
    };
    (
        "siege_flights".into(),
        serde_json::json!({
            "launched": s.launched, "landed": s.landed, "in_flight": s.in_flight,
            "bombards": s.bombards, "trail_puffs": s.trail_puffs,
            "longest_s": s.longest_s, "apex_m": s.apex_m,
        }),
    )
}

/// A flipbook emitter over a burning house or the gate (`siege_fire_fx.gd` `_make_fire`): `key`
/// is the house index, `usize::MAX` the gate.
#[derive(Component)]
pub struct FireEmitter {
    pub key: usize,
    pub kind: FlipbookKind,
}

/// Flames and smoke over the burning gate and houses (`siege_fire_fx.gd`): one flame and one
/// smoke emitter per fire, started and stopped with the fire, `amount_ratio` following the
/// intensity. The sprites themselves are the flipbook particles of `crate::fx::flipbook`.
fn sync_fire_emitters(
    mut commands: Commands,
    data: Option<Res<FlipbookData>>,
    frame: Res<SiegeFrame>,
    mut session: ResMut<BattleSession>,
    mut stats: ResMut<SiegeStats>,
    mut emitters: Query<(Entity, &FireEmitter, &mut FlipbookEmitter, &mut Transform)>,
    time: Res<Time>,
    wind: Option<Res<BattleWind>>,
    mut carry: Local<f32>,
) {
    let Some(data) = data else { return };
    if !frame.active {
        return;
    }
    // `update_period_s` of the data: 0.2 s.
    *carry += time.delta_secs();
    if *carry < 0.2 {
        return;
    }
    *carry = 0.0;
    // Wind carries the smoke (`gravity` of `siege_fire_fx.gd`: wind x drift) and the embers (0.8).
    let wind = wind.map_or(BattleWind::default(), |w| *w);
    let drift = Vec3::new(wind.dir.x, 0.0, wind.dir.y) * wind.strength * data.smoke_wind_drift;
    let sim = session.sim_mut();
    let Some(works) = sim.siege() else { return };
    let mut sources: Vec<(usize, Vec3, f32, f32)> = Vec::new(); // key, ground point, radius, intensity
    if let Some(fire) = frame.gate_fire
        && fire.state == FireState::Burning
    {
        let (mx, mz) = works.pieces[works.gate].midpoint();
        let base = sim.field().height(mx, mz) as f32;
        sources.push((
            usize::MAX,
            Vec3::new(mx as f32, base, mz as f32),
            6.0,
            fire.intensity,
        ));
    }
    for (index, (house, fire)) in works.houses.iter().zip(&frame.houses).enumerate() {
        if fire.state == FireState::Burning {
            let base = sim.field().height(house.x, house.z) as f32;
            sources.push((
                index,
                Vec3::new(house.x as f32, base, house.z as f32),
                house.radius as f32,
                fire.intensity,
            ));
        }
    }
    let mut present = std::collections::HashSet::new();
    for (entity, tag, mut emitter, mut transform) in &mut emitters {
        match sources.iter().find(|s| s.0 == tag.key) {
            Some(&(_, ground, _, intensity)) => {
                present.insert((tag.key, tag.kind));
                emitter.ratio = (0.25 + 0.75 * intensity).clamp(0.0, 1.0);
                let height = match tag.kind {
                    FlipbookKind::Smoke => data.flame_height_m + 1.0,
                    FlipbookKind::Ember => data.flame_height_m * 0.6,
                    _ => data.flame_height_m * 0.45,
                };
                match tag.kind {
                    FlipbookKind::Smoke => emitter.accel = Vec3::Y * 0.6 + drift,
                    FlipbookKind::Ember => emitter.accel = Vec3::Y * 0.4 + drift * 0.8,
                    _ => {}
                }
                transform.translation = ground + Vec3::Y * height;
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for &(key, ground, radius, intensity) in &sources {
        for kind in [
            FlipbookKind::Flame,
            FlipbookKind::Smoke,
            FlipbookKind::Ember,
        ] {
            if present.contains(&(key, kind)) {
                continue;
            }
            stats.fires_started += 1;
            let (mut emitter, height) = match kind {
                FlipbookKind::Smoke => {
                    let mut smoke = FlipbookEmitter::smoke(&data, radius);
                    smoke.accel = Vec3::Y * 0.6 + drift;
                    (smoke, data.flame_height_m + 1.0)
                }
                FlipbookKind::Ember => (
                    FlipbookEmitter::embers(&data, radius, drift),
                    data.flame_height_m * 0.6,
                ),
                _ => (
                    FlipbookEmitter::flames(&data, radius),
                    data.flame_height_m * 0.45,
                ),
            };
            emitter.ratio = (0.25 + 0.75 * intensity).clamp(0.0, 1.0);
            commands.spawn((
                Name::new("SiegeFireEmitter"),
                FireEmitter { key, kind },
                emitter,
                Transform::from_translation(ground + Vec3::Y * height),
                GlobalTransform::default(),
                DespawnOnExit(AppState::Battle),
            ));
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
