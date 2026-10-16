//! Smoke over the villages of the battle horizon (port of the smoke part of `_build_silhouettes`
//! in game/scripts/battle/battle_horizon.gd and of game/shaders/battle_horizon_smoke.gdshader).
//!
//! Godot raised 1-4 smoke columns (`horizon.json` `silhouettes.smokes`) over the village
//! silhouettes it placed 2.6-8.5 km away (`horizon_silhouettes`, which draws the sites), none in
//! rain or snow. Each column is a 70 x 420 m panel turned to the camera round the vertical
//! (`horizon_smoke.wesl`), tinted by the sun (`light_tint` of battle_horizon.gd) and greyed by the
//! distance fog.

use bevy::asset::embedded_asset;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::atmosphere::{AtmosphereState, BattleSun, weather_from_key};
use super::horizon::HorizonStats;
use super::horizon_silhouettes::HorizonSilhouettes;
use super::look::DecorLook;
use super::weather_fx::{WeatherFxLook, linear3};
use crate::app::AppState;

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

fn spawn_smoke(
    mut commands: Commands,
    atmosphere: Res<AtmosphereState>,
    fx: Res<WeatherFxLook>,
    silhouettes: Option<Res<HorizonSilhouettes>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HorizonSmokeMaterial>>,
) {
    // The sites come from the silhouettes (built the frame the horizon exists).
    let Some(silhouettes) = silhouettes else {
        return;
    };
    // Rain and snow beat the smoke down (Godot: none then).
    if matches!(atmosphere.weather.as_str(), "rain" | "snow") || silhouettes.smokes.is_empty() {
        commands.insert_resource(HorizonSmokeStats::default());
        return;
    }
    let sm = &fx.horizon_smoke;
    let material = materials.add(HorizonSmokeMaterial {
        params: SmokeParams {
            color: linear3(silhouettes.smoke_color).extend(sm.opacity),
            light: Vec4::ONE,
            fog: Vec4::ZERO,
        },
    });
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let mut stats = HorizonSmokeStats {
        material: Some(material.clone()),
        ..default()
    };
    // The site already carries Godot's 5 m lift and +-40 m scatter.
    for (i, site) in silhouettes.smokes.iter().enumerate() {
        let p = site.position;
        commands.spawn((
            Name::new(format!("HorizonSmoke{i}")),
            Mesh3d(quad.clone()),
            MeshMaterial3d(material.clone()),
            MeshTag::new((site.seed / 10.0 * 10_000.0) as u32),
            Transform::from_xyz(p.x, p.y, p.z).with_scale(Vec3::new(
                sm.size_m[0],
                sm.size_m[1],
                1.0,
            )),
            NotShadowCaster,
            DespawnOnExit(AppState::Battle),
        ));
        stats.columns += 1;
        stats.distances.push(p.xz().distance(silhouettes.centre));
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
    fn tint_is_half_way_to_the_sun() {
        let t = light_tint(Vec3::new(1.0, 0.5, 0.0), 1.75);
        assert!((t - Vec3::new(1.0, 0.75, 0.5)).length() < 1e-5);
        assert!(light_tint(Vec3::ONE, 0.1).x >= 0.45 - 1e-6);
    }
}
