//! Smoke over the villages of the battle horizon (port of the smoke part of `_build_silhouettes`
//! in game/scripts/battle/battle_horizon.gd and of game/shaders/battle_horizon_smoke.gdshader).
//!
//! Godot raised 1-4 smoke columns (`horizon.json` `silhouettes.smokes`) over the village
//! silhouettes it placed 2.6-8.5 km away, none in rain or snow. The silhouettes themselves are not
//! ported yet (`horizon.rs`), so the sites are drawn here the same way (seeded bearing and
//! distance from the field centre within `min_distance_m..max_distance_m`, base on the horizon
//! ring height). Each column is a 70 x 420 m panel turned to the camera round the vertical
//! (`horizon_smoke.wesl`), tinted by the sun (`light_tint` of battle_horizon.gd) and greyed by the
//! distance fog.

use std::f32::consts::TAU;

use bevy::asset::embedded_asset;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::atmosphere::{AtmosphereState, BattleSun, weather_from_key};
use super::horizon::{HorizonStats, RealRelief, ring_height};
use super::look::DecorLook;
use super::weather_fx::{WeatherFxLook, linear3};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::paths;

#[derive(ShaderType, Clone, Debug, Default)]
pub struct SmokeParams {
    pub color: Vec4,
    pub light: Vec4,
    pub fog: Vec4,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct HorizonSmokeMaterial {
    #[uniform(0)]
    pub params: SmokeParams,
}

impl Material for HorizonSmokeMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/horizon_smoke.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/decor/horizon_smoke.wesl".into()
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

/// Smoke columns of the horizon (read by the dump).
#[derive(Resource, Debug, Clone, Default)]
pub struct HorizonSmokeStats {
    pub columns: usize,
    /// Distances of the columns from the field centre (m).
    pub distances: Vec<f32>,
    pub light_tint: [f32; 3],
    material: Option<Handle<HorizonSmokeMaterial>>,
}

/// `horizon.json` `silhouettes`: distances, count range and colour of the smokes.
#[derive(Debug, Clone, PartialEq)]
pub struct SmokeSites {
    pub min_distance: f32,
    pub max_distance: f32,
    pub count: (u32, u32),
    pub color: [f32; 3],
}

impl SmokeSites {
    pub fn from_horizon(horizon: &serde_json::Value) -> SmokeSites {
        let s = &horizon["silhouettes"];
        let num = |v: &serde_json::Value, d: f64| v.as_f64().unwrap_or(d) as f32;
        SmokeSites {
            min_distance: num(&s["min_distance_m"], 2600.0),
            max_distance: num(&s["max_distance_m"], 8500.0),
            count: (
                s["smokes"][0].as_u64().unwrap_or(1) as u32,
                s["smokes"][1].as_u64().unwrap_or(4) as u32,
            ),
            color: [
                num(&s["smoke"][0], 0.62),
                num(&s["smoke"][1], 0.62),
                num(&s["smoke"][2], 0.64),
            ],
        }
    }

    /// Plan positions (relative to the field centre) of the columns of battle `seed`.
    pub fn draw(&self, seed: u64) -> Vec<Vec2> {
        let salt = (seed % 100_003) as f32;
        let pick = |i: usize, k: f32| super::hash01(salt, i as f32 * 7.31 + 0.5, k);
        let span = self.count.1.saturating_sub(self.count.0) + 1;
        let n = self.count.0 + ((pick(0, 41.0) * span as f32) as u32).min(span - 1);
        (0..n as usize)
            .map(|i| {
                let bearing = pick(i + 1, 11.0) * TAU;
                let d =
                    self.min_distance + (self.max_distance - self.min_distance) * pick(i + 1, 23.0);
                Vec2::new(bearing.sin(), -bearing.cos()) * d
            })
            .collect()
    }
}

/// Light tint of the smoke: white half-way to the sun colour, times the sun energy (battle_horizon.gd).
pub fn light_tint(sun_color: Vec3, sun_energy: f32) -> Vec3 {
    let energy = (sun_energy / 1.75).clamp(0.45, 1.1);
    Vec3::ONE.lerp(sun_color, 0.5) * energy
}

pub struct HorizonSmokePlugin;

impl Plugin for HorizonSmokePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "horizon_smoke.wesl");
        app.add_plugins(MaterialPlugin::<HorizonSmokeMaterial>::default())
            .add_systems(
                Update,
                (
                    spawn_smoke.run_if(not(resource_exists::<HorizonSmokeStats>)),
                    tint_smoke.run_if(resource_exists::<HorizonSmokeStats>),
                )
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<AtmosphereState>)
                    .run_if(resource_exists::<HorizonStats>)
                    .run_if(resource_exists::<WeatherFxLook>),
            )
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<HorizonSmokeStats>();
            });
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_smoke(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    atmosphere: Res<AtmosphereState>,
    fx: Res<WeatherFxLook>,
    look: Res<DecorLook>,
    relief: Option<Res<RealRelief>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HorizonSmokeMaterial>>,
) {
    // Rain and snow beat the smoke down (Godot: none then).
    if matches!(atmosphere.weather.as_str(), "rain" | "snow") {
        commands.insert_resource(HorizonSmokeStats::default());
        return;
    }
    let horizon: serde_json::Value = paths::find_root()
        .ok()
        .and_then(|root| std::fs::read(paths::data_dir(&root).join("fx/horizon.json")).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let sites = SmokeSites::from_horizon(&horizon);
    let seed = session.seed;
    let field = session.sim_mut().field();
    let centre = Vec2::new(field.width as f32 * 0.5, field.depth as f32 * 0.5);
    let sm = &fx.horizon_smoke;
    let material = materials.add(HorizonSmokeMaterial {
        params: SmokeParams {
            color: linear3(sites.color).extend(sm.opacity),
            light: Vec4::ONE,
            fog: Vec4::ZERO,
        },
    });
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let mut stats = HorizonSmokeStats {
        material: Some(material.clone()),
        ..default()
    };
    for (i, offset) in sites.draw(seed).into_iter().enumerate() {
        let jitter = Vec2::new(
            super::hash01(i as f32, seed as f32 % 997.0, 61.0) - 0.5,
            super::hash01(i as f32, seed as f32 % 997.0, 67.0) - 0.5,
        ) * 2.0
            * sm.scatter_m;
        let p = centre + offset + jitter;
        let y = ring_height(field, &look, relief.as_deref(), p.x, p.y) + sm.base_lift_m;
        commands.spawn((
            Name::new(format!("HorizonSmoke{i}")),
            Mesh3d(quad.clone()),
            MeshMaterial3d(material.clone()),
            MeshTag::new((super::hash01(i as f32, seed as f32 % 991.0, 73.0) * 10_000.0) as u32),
            Transform::from_xyz(p.x, y, p.y).with_scale(Vec3::new(sm.size_m[0], sm.size_m[1], 1.0)),
            NotShadowCaster,
            DespawnOnExit(AppState::Battle),
        ));
        stats.columns += 1;
        stats.distances.push(offset.length());
    }
    commands.insert_resource(stats);
}

/// Light tint and fog follow the sun and the fog of the battle.
fn tint_smoke(
    atmosphere: Res<AtmosphereState>,
    look: Res<DecorLook>,
    horizon: Res<HorizonStats>,
    suns: Query<&DirectionalLight, With<BattleSun>>,
    mut stats: ResMut<HorizonSmokeStats>,
    mut materials: ResMut<Assets<HorizonSmokeMaterial>>,
    mut done: Local<Option<(f32, f32)>>,
) {
    let Some(handle) = stats.material.clone() else {
        return;
    };
    let Some(sun) = suns.iter().next() else {
        return;
    };
    let key = (sun.illuminance, atmosphere.fog_density);
    if *done == Some(key) {
        return;
    }
    let Some(mut material) = materials.get_mut(handle.id()) else {
        return;
    };
    *done = Some(key);
    let lin = sun.color.to_linear();
    let tint = light_tint(
        Vec3::new(lin.red, lin.green, lin.blue),
        sun.illuminance / super::atmosphere::LUX_PER_ENERGY,
    );
    let fog = look.fog_of(weather_from_key(&atmosphere.weather));
    let density = if horizon.active {
        fog.horizon_density
    } else {
        fog.density
    };
    material.params.light = tint.extend(1.0);
    material.params.fog = linear3(fog.color).extend(density);
    stats.light_tint = tint.to_array();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sites_follow_the_horizon_bounds() {
        let root = paths::find_root().unwrap();
        let bytes = std::fs::read(paths::data_dir(&root).join("fx/horizon.json")).unwrap();
        let sites = SmokeSites::from_horizon(&serde_json::from_slice(&bytes).unwrap());
        assert_eq!(sites.count, (1, 4));
        for seed in [1u64, 7, 42, 9001, 123_456] {
            let drawn = sites.draw(seed);
            assert!((1..=4).contains(&drawn.len()), "{seed}: {}", drawn.len());
            for p in drawn {
                let d = p.length();
                assert!(
                    d >= sites.min_distance - 1.0 && d <= sites.max_distance + 1.0,
                    "{d}"
                );
            }
        }
    }

    #[test]
    fn tint_is_half_way_to_the_sun() {
        let t = light_tint(Vec3::new(1.0, 0.5, 0.0), 1.75);
        assert!((t - Vec3::new(1.0, 0.75, 0.5)).length() < 1e-5);
        assert!(light_tint(Vec3::ONE, 0.1).x >= 0.45 - 1e-6);
    }
}
