//! Rain and snow near the camera as a pool of simple entities (streaks / flakes) around the
//! focus point: size, box, speed and density follow `PrecipitationProfile` of the camera distance
//! (`CampaignWeatherView._update_particles`). Each particle's position is a pure function of the
//! clock (wrapping fall through the emission box), so nothing is stored per frame.
//! Storm uses the rain pool. Shown only under `particles_far` and where the focus weather
//! precipitates.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use super::model::{Kind, PrecipitationProfile, hash01, lerp};
use super::{WeatherTuning, WeatherView};
use crate::app::AppState;

/// Camera distance under which precipitation is drawn (`particles_far`).
pub const PARTICLES_FAR: f32 = 220.0;
/// Pool sizes (Godot: 5000 rain, 2600 snow, then `amount_ratio`).
const RAIN_POOL: usize = 1800;
const SNOW_POOL: usize = 1100;

#[derive(Component, Debug, Clone, Copy)]
pub struct Particle {
    pub snow: bool,
    pub index: u32,
}

#[derive(Resource, Default, Debug)]
pub struct PrecipStats {
    pub rain_active: usize,
    pub snow_active: usize,
    pub box_half: f32,
}

#[derive(Resource)]
struct PrecipAssets {
    mesh: Handle<Mesh>,
    rain: Handle<StandardMaterial>,
    snow: Handle<StandardMaterial>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = |color: Color| StandardMaterial {
        base_color: color,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    };
    commands.insert_resource(PrecipAssets {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        rain: materials.add(material(Color::srgba(0.62, 0.67, 0.76, 0.24))),
        snow: materials.add(material(Color::srgba(0.95, 0.96, 1.0, 0.85))),
    });
}

fn spawn_pool(
    mut commands: Commands,
    assets: Option<Res<PrecipAssets>>,
    existing: Query<(), With<Particle>>,
) {
    let Some(assets) = assets else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    for (snow, count) in [(false, RAIN_POOL), (true, SNOW_POOL)] {
        for index in 0..count as u32 {
            commands.spawn((
                Name::new(if snow { "Snowflake" } else { "Raindrop" }),
                Particle { snow, index },
                Mesh3d(assets.mesh.clone()),
                MeshMaterial3d(if snow {
                    assets.snow.clone()
                } else {
                    assets.rain.clone()
                }),
                Transform::default(),
                Visibility::Hidden,
                NotShadowCaster,
                DespawnOnExit(AppState::Campaign),
            ));
        }
    }
}

/// Position of particle `index` at clock `t`: random column in the box, falling through its
/// height at its own speed and wrapping; snow sways sideways, rain slants with the wind.
pub fn particle_position(
    index: u32,
    t: f32,
    center: Vec3,
    half: [f32; 2],
    speed: [f32; 2],
    sway: f32,
    snow: bool,
) -> Vec3 {
    let seed = u64::from(index) * 4 + u64::from(snow) * 1_000_003;
    let (u, w, p, s) = (
        hash01(seed),
        hash01(seed + 1),
        hash01(seed + 2),
        hash01(seed + 3),
    );
    let span = (half[1] * 2.0).max(1e-4);
    let fall = lerp(speed[0], speed[1], s);
    let travelled = (p * span + t * fall).rem_euclid(span);
    let mut x = (u * 2.0 - 1.0) * half[0];
    let z = (w * 2.0 - 1.0) * half[0];
    let y = half[1] - travelled;
    if snow {
        x += (t * 1.3 + p * std::f32::consts::TAU).sin() * sway;
    } else {
        x += travelled * 0.12; // wind slant (direction 0.12, -1, 0.05)
    }
    center + Vec3::new(x, y, z + travelled * 0.05)
}

/// Active particles of a pool: `ratio` of the pool (rain also by `rain_amount_scale`).
pub fn active_count(profile: &PrecipitationProfile, distance: f32, snow: bool) -> usize {
    let pool = if snow { SNOW_POOL } else { RAIN_POOL };
    let scale = if snow {
        1.0
    } else {
        profile.rain_amount_scale.clamp(0.0, 1.0)
    };
    ((pool as f32) * (profile.amount_ratio(distance) * scale).clamp(0.0, 1.0)).round() as usize
}

fn update_particles(
    time: Res<Time>,
    view: Res<WeatherView>,
    tuning: Res<WeatherTuning>,
    mut stats: ResMut<PrecipStats>,
    mut particles: Query<(&Particle, &mut Transform, &mut Visibility)>,
) {
    let profile = &tuning.precipitation;
    let near = profile.enabled && view.distance < PARTICLES_FAR;
    let kind = view.focus_entry.kind;
    let rain_on = near && matches!(kind, Kind::Rain | Kind::Storm);
    let snow_on = near && kind == Kind::Snow;
    let distance = view.distance;
    let half = profile.box_extents(distance);
    let center = view.focus + Vec3::Y * profile.height(distance);
    let rain_active = if rain_on {
        active_count(profile, distance, false)
    } else {
        0
    };
    let snow_active = if snow_on {
        active_count(profile, distance, true)
    } else {
        0
    };
    stats.rain_active = rain_active;
    stats.snow_active = snow_active;
    stats.box_half = half[0];
    let rain_size = profile.rain_size(distance);
    let snow_size = profile.snow_size(distance);
    let rain_speed = profile.speed(distance, false);
    let snow_speed = profile.speed(distance, true);
    let sway = profile.sway(distance);
    let t = time.elapsed_secs();
    for (particle, mut transform, mut visibility) in &mut particles {
        let active = if particle.snow {
            snow_active
        } else {
            rain_active
        };
        let on = (particle.index as usize) < active;
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        if !on {
            continue;
        }
        let (speed, scale) = if particle.snow {
            (snow_speed, Vec3::splat(snow_size))
        } else {
            (
                rain_speed,
                Vec3::new(rain_size[0], rain_size[1], rain_size[0]),
            )
        };
        transform.translation =
            particle_position(particle.index, t, center, half, speed, sway, particle.snow);
        transform.scale = scale;
    }
}

pub fn dump(world: &World) -> Value {
    let stats = world.resource::<PrecipStats>();
    json!({
        "pool": [RAIN_POOL, SNOW_POOL],
        "rain_active": stats.rain_active, "snow_active": stats.snow_active,
        "box_half": stats.box_half,
    })
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<PrecipStats>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AppState::Campaign), spawn_pool)
        .add_systems(
            Update,
            update_particles
                .after(super::update_view)
                .run_if(in_state(AppState::Campaign)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particles_stay_in_the_box_and_fall() {
        let center = Vec3::new(100.0, 20.0, 50.0);
        let half = [10.0, 4.0];
        for index in 0..200 {
            let a = particle_position(index, 1.0, center, half, [3.0, 4.0], 0.0, false);
            assert!((a.y - center.y).abs() <= half[1] + 1e-4);
            assert!((a.z - center.z).abs() <= half[0] + 4.0);
            let b = particle_position(index, 1.05, center, half, [3.0, 4.0], 0.0, false);
            // falls (or wraps to the top)
            assert!(b.y < a.y || b.y > a.y + half[1]);
        }
    }

    #[test]
    fn snow_sways_and_density_follows_profile() {
        let p = PrecipitationProfile::default();
        let a = particle_position(3, 0.0, Vec3::ZERO, [5.0, 2.0], [1.0, 1.0], 1.0, true);
        let b = particle_position(3, 1.0, Vec3::ZERO, [5.0, 2.0], [1.0, 1.0], 1.0, true);
        assert!((a.x - b.x).abs() > 1e-3);
        assert!(active_count(&p, 1.0, true) >= active_count(&p, 100.0, true));
        let half = PrecipitationProfile {
            rain_amount_scale: 0.5,
            ..default()
        };
        assert!(active_count(&half, 1.0, false) < active_count(&p, 1.0, false));
    }
}
