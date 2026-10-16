//! Rain and snow round the battle camera (port of `_add_precipitation` of
//! game/scripts/battle/battle_atmosphere.gd).
//!
//! Godot hung a `GPUParticles3D` (9 000 drops / 7 000 flakes) on the camera. Bevy has no GPU
//! particles: one static mesh holds a quad per drop with a random seed, and
//! `precipitation.wesl` places each drop from its seed and the time, wrapping it in a box that
//! follows the camera (`ahead_m` in front, `above_m` above, in camera space like Godot's emitter at
//! `(0, 30, -45)`). The CPU only moves the box centre. Values from `battle_weather_fx.json`.
//! Deviations: drops are world-anchored and wrap (no lifetime), snowflakes are round, streaks are
//! soft-edged and tapered; no scene fog on them (Godot's were mostly within 14 m of the lens).

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::atmosphere::AtmosphereState;
use super::weather_fx::{FallLook, WeatherFxLook, linear3};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;

#[derive(ShaderType, Clone, Debug, Default)]
pub struct FallParams {
    pub area: Vec4,
    pub centre: Vec4,
    pub velocity: Vec4,
    pub shape: Vec4,
    pub color: Vec4,
    pub fade: Vec4,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct PrecipitationMaterial {
    #[uniform(0)]
    pub params: FallParams,
}

impl Material for PrecipitationMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/precipitation.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/precipitation.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// The precipitation entity of the battle.
#[derive(Component)]
pub struct Precipitation {
    material: Handle<PrecipitationMaterial>,
    ahead: f32,
    above: f32,
    snow: bool,
    /// A shower under a sky that is not the rain sky.
    shower: bool,
}

/// What falls (read by the dump).
#[derive(Resource, Debug, Clone, Default)]
pub struct PrecipitationStats {
    /// `rain`, `snow` or empty.
    pub kind: String,
    pub drops: u32,
    /// The rain is a shower of the simulation, not the sky's rain.
    pub shower: bool,
    pub centre: [f32; 3],
}

pub struct PrecipitationPlugin;

impl Plugin for PrecipitationPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "precipitation.wesl");
        app.add_plugins(MaterialPlugin::<PrecipitationMaterial>::default())
            .add_systems(
                Update,
                (
                    update_precipitation,
                    follow_camera.run_if(resource_exists::<PrecipitationStats>),
                )
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<AtmosphereState>)
                    .run_if(resource_exists::<WeatherFxLook>),
            )
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<PrecipitationStats>();
            });
    }
}

/// Deterministic 0..1 hash of an integer (drop seeds).
fn hash(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9) ^ salt.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// A mesh of `count` quads (corner in POSITION, seed in NORMAL, uv in UV_0).
pub fn drops_mesh(count: u32) -> Mesh {
    let n = count as usize;
    let mut positions = Vec::with_capacity(n * 4);
    let mut seeds = Vec::with_capacity(n * 4);
    let mut uvs = Vec::with_capacity(n * 4);
    let mut indices = Vec::with_capacity(n * 6);
    for i in 0..count {
        let seed = [hash(i, 1), hash(i, 2), hash(i, 3)];
        let base = positions.len() as u32;
        for (corner, uv) in [
            ([-0.5, -0.5, 0.0], [0.0, 1.0]),
            ([0.5, -0.5, 0.0], [1.0, 1.0]),
            ([0.5, 0.5, 0.0], [1.0, 0.0]),
            ([-0.5, 0.5, 0.0], [0.0, 0.0]),
        ] {
            positions.push(corner);
            seeds.push(seed);
            uvs.push(uv);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, seeds)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn params(look: &FallLook, snow: bool) -> FallParams {
    let c = linear3([look.color[0], look.color[1], look.color[2]]);
    FallParams {
        area: Vec3::from_array(look.box_m)
            .max(Vec3::splat(1.0))
            .extend(0.0),
        centre: Vec4::new(0.0, 0.0, 0.0, if snow { 1.0 } else { 0.0 }),
        velocity: Vec3::from_array(look.velocity).extend(look.speed_jitter),
        shape: Vec4::new(look.size_m[0], look.size_m[1], look.sway_m, look.sway_hz),
        color: c.extend(look.color[3]),
        fade: Vec4::new(look.fade_m[0], look.fade_m[1], 0.0, 0.0),
    }
}

/// What should fall: `(snow, shower)`. The sky's rain and snow are permanent; a shower is rain
/// from the simulation under a sky that is not the rain sky (historical map: the sky is the one of
/// the end of the battle) and stops when the simulation changes weather (`add_shower`, and the
/// poll of battle_scene.gd).
pub fn desired_fall(sky: &str, sim_weather: &str) -> Option<(bool, bool)> {
    match sky {
        "rain" => Some((false, false)),
        "snow" => Some((true, false)),
        _ if sim_weather == "rain" => Some((false, true)),
        _ => None,
    }
}

fn spawn_fall(
    commands: &mut Commands,
    look: &FallLook,
    snow: bool,
    shower: bool,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<PrecipitationMaterial>,
) {
    let material = materials.add(PrecipitationMaterial {
        params: params(look, snow),
    });
    commands.spawn((
        Name::new(if shower { "Shower" } else { "Precipitation" }),
        Precipitation {
            material: material.clone(),
            ahead: look.ahead_m,
            above: look.above_m,
            snow,
            shower,
        },
        Mesh3d(meshes.add(drops_mesh(look.count))),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        NoFrustumCulling,
        NotShadowCaster,
        DespawnOnExit(AppState::Battle),
    ));
}

/// Keeps the falling particles in step with the sky and the simulation weather (polled once a
/// second like Godot; the first call spawns at once).
#[allow(clippy::too_many_arguments)]
fn update_precipitation(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: Local<f32>,
    mut session: ResMut<BattleSession>,
    atmosphere: Res<AtmosphereState>,
    fx: Res<WeatherFxLook>,
    existing: Query<(Entity, &Precipitation)>,
    stats: Option<ResMut<PrecipitationStats>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PrecipitationMaterial>>,
) {
    let first = stats.is_none();
    *timer += time.delta_secs();
    if !first && *timer < 1.0 {
        return;
    }
    *timer = 0.0;
    // Dev hook (proof of the shower without a historical map): the simulation rains.
    let sim_weather = if std::env::var_os("CENT_ANS_SHOWER").is_some() {
        "rain".to_owned()
    } else {
        super::atmosphere::weather_key(session.sim_mut().weather()).to_owned()
    };
    let want = desired_fall(&atmosphere.weather, &sim_weather);
    let have = existing.iter().next().map(|(_, p)| (p.snow, p.shower));
    if have != want {
        for (entity, _) in &existing {
            commands.entity(entity).despawn();
        }
        if let Some((snow, shower)) = want {
            let look = if snow {
                &fx.precipitation.snow
            } else {
                &fx.precipitation.rain
            };
            spawn_fall(
                &mut commands,
                look,
                snow,
                shower,
                &mut meshes,
                &mut materials,
            );
        }
    }
    let new_stats = PrecipitationStats {
        kind: match want {
            Some((true, _)) => "snow",
            Some((false, _)) => "rain",
            None => "",
        }
        .to_owned(),
        shower: want.is_some_and(|(_, shower)| shower),
        drops: want.map_or(0, |(snow, _)| {
            if snow {
                fx.precipitation.snow.count
            } else {
                fx.precipitation.rain.count
            }
        }),
        centre: stats.as_ref().map_or([0.0; 3], |s| s.centre),
    };
    match stats {
        Some(mut stats) => *stats = new_stats,
        None => commands.insert_resource(new_stats),
    }
}

/// Moves the wrap box with the camera (Godot's emitter at camera-local `(0, above, -ahead)`).
fn follow_camera(
    mut stats: ResMut<PrecipitationStats>,
    cameras: Query<&GlobalTransform, With<BattleCamera>>,
    falls: Query<&Precipitation>,
    mut materials: ResMut<Assets<PrecipitationMaterial>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    for fall in &falls {
        let centre = camera.transform_point(Vec3::new(0.0, fall.above, -fall.ahead));
        if let Some(mut material) = materials.get_mut(fall.material.id()) {
            let mode = material.params.centre.w;
            material.params.centre = centre.extend(mode);
        }
        stats.centre = centre.to_array();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_has_a_quad_per_drop() {
        let mesh = drops_mesh(10);
        assert_eq!(mesh.count_vertices(), 40);
        assert_eq!(mesh.indices().map(|i| i.len()), Some(60));
    }

    #[test]
    fn seeds_spread_over_the_unit_cube() {
        let mean: f32 = (0..2000).map(|i| hash(i, 1)).sum::<f32>() / 2000.0;
        assert!((mean - 0.5).abs() < 0.03, "{mean}");
        assert!((0..2000).all(|i| (0.0..1.0).contains(&hash(i, 2))));
    }
}

#[cfg(test)]
mod fall_tests {
    use super::desired_fall;

    #[test]
    fn sky_rain_and_snow_are_permanent() {
        assert_eq!(desired_fall("rain", "clear"), Some((false, false)));
        assert_eq!(desired_fall("snow", "rain"), Some((true, false)));
    }

    #[test]
    fn a_shower_falls_from_a_dry_sky_while_the_simulation_rains() {
        assert_eq!(desired_fall("clear", "rain"), Some((false, true)));
        assert_eq!(desired_fall("fog", "rain"), Some((false, true)));
        assert_eq!(desired_fall("clear", "clear"), None);
    }
}
