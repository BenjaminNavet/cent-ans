//! Volumetric fog of a battle (port of `_setup_volumetric_fog` and `add_ground_mist` of
//! game/scripts/battle/battle_atmosphere.gd, gated like `RenderQuality.apply`).
//!
//! Godot's environment fog was a froxel volume `volumetric_fog_length` (320 m) deep in front of
//! the camera, denser in bad weather, lit by the sun (light shafts, anisotropy 0.6 in clear
//! weather, 0.25 otherwise); in fog weather a `FogVolume` box 12 m high lay over the whole field
//! (density 0.01, height falloff 0.35, edge fade 0.3). Bevy raymarches `FogVolume` boxes:
//! - "AirVeil": a box `2 x length_m` wide round the camera, from below the lowest ground to
//!   `height_m` above the mean ground, density = Godot's extinction (`absorption + scattering` = 1);
//! - "GroundMist" (fog weather): the field-wide low sheet, its height falloff and edge fade baked
//!   into a small 3D density texture.
//!
//! Gate: the quality preset's `volumetric` (`data/fx/render_quality.json`: `off`, `weather` = fog,
//! rain and snow, `always`); `CENT_ANS_VOLUMETRIC=off|weather|always` overrides it (dev hook).
//! Deviation: Bevy adds `ambient_intensity` without weighting it by the fog density (a thin veil
//! would grey the whole view), so the ambient injection of Godot (0.35) is 0 by default.

use bevy::asset::RenderAssetUsages;
use bevy::light::{FogVolume, VolumetricFog, VolumetricLight};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::atmosphere::{AtmosphereState, BattleSun, weather_from_key};
use super::look::DecorLook;
use super::weather_fx::{GroundMistLook, WeatherFxLook};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::paths;

/// The box of fog that follows the camera.
#[derive(Component)]
pub struct AirVeil;

/// What the volumetric fog is (read by the dump).
#[derive(Resource, Debug, Clone, Default)]
pub struct VolumetricStats {
    /// `off`, `weather` or `always`.
    pub mode: String,
    pub enabled: bool,
    pub density: f32,
    pub anisotropy: f32,
    pub volumes: usize,
    pub ground_mist: bool,
    pub cameras: usize,
    /// Field height range: (min, mean, max).
    ground: (f32, f32, f32),
}

pub struct VolumetricPlugin;

impl Plugin for VolumetricPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                setup_volumetric.run_if(not(resource_exists::<VolumetricStats>)),
                (attach_to_camera, follow_camera).run_if(resource_exists::<VolumetricStats>),
            )
                .chain()
                .run_if(in_state(AppState::Battle))
                .run_if(resource_exists::<AtmosphereState>)
                .run_if(resource_exists::<WeatherFxLook>),
        )
        .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
            commands.remove_resource::<VolumetricStats>();
        });
    }
}

/// Volumetric setting of quality `level` in `render_quality.json` (`off` when unknown).
pub fn preset_mode(presets: &serde_json::Value, level: &str) -> String {
    presets["presets"][level]["volumetric"]
        .as_str()
        .unwrap_or("off")
        .to_owned()
}

/// Whether the fog is drawn for `mode` under `weather` (render_quality.gd).
pub fn enabled_for(mode: &str, weather: &str) -> bool {
    mode == "always" || (mode == "weather" && matches!(weather, "fog" | "rain" | "snow"))
}

/// `size`: texels per axis (x, y, z). Density 1 at the bottom, `exp(-falloff * metres)` upwards,
/// faded over `edge_fade` of the box on the sides and the top.
pub fn mist_density(mist: &GroundMistLook, size: (u32, u32, u32)) -> Vec<u8> {
    let (nx, ny, nz) = size;
    let fade = |t: f32| {
        if mist.edge_fade <= 0.0 {
            1.0
        } else {
            (t.min(1.0 - t) / mist.edge_fade).clamp(0.0, 1.0)
        }
    };
    let mut bytes = Vec::with_capacity((nx * ny * nz) as usize);
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let u = (x as f32 + 0.5) / nx as f32;
                let v = (y as f32 + 0.5) / ny as f32;
                let w = (z as f32 + 0.5) / nz as f32;
                let height = (-mist.height_falloff * v * mist.height_m).exp();
                let top = if mist.edge_fade <= 0.0 {
                    1.0
                } else {
                    ((1.0 - v) / mist.edge_fade).clamp(0.0, 1.0)
                };
                let d = height * top * fade(u) * fade(w);
                bytes.push((d.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
    }
    bytes
}

fn quality_mode(level: Option<&str>) -> String {
    if let Ok(forced) = std::env::var("CENT_ANS_VOLUMETRIC") {
        return forced;
    }
    let presets: serde_json::Value = paths::find_root()
        .ok()
        .and_then(|root| std::fs::read(paths::data_dir(&root).join("fx/render_quality.json")).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    preset_mode(&presets, level.unwrap_or("high"))
}

#[allow(clippy::too_many_arguments)]
fn setup_volumetric(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    atmosphere: Res<AtmosphereState>,
    fx: Res<WeatherFxLook>,
    look: Res<DecorLook>,
    quality: Option<Res<crate::settings::quality::RenderQuality>>,
    suns: Query<Entity, With<BattleSun>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(sun) = suns.iter().next() else {
        return;
    };
    let mode = quality_mode(quality.as_ref().map(|q| q.level.as_str()));
    let weather = atmosphere.weather.clone();
    let v = &fx.volumetric;
    let enabled = enabled_for(&mode, &weather);
    let field = session.sim_mut().field();
    let heights = &field.heights;
    let n = heights.len().max(1) as f32;
    let mean = heights.iter().sum::<f64>() as f32 / n;
    let min = heights.iter().copied().fold(f64::MAX, f64::min) as f32;
    let max = heights.iter().copied().fold(f64::MIN, f64::max) as f32;
    let (width, depth) = (field.width as f32, field.depth as f32);
    let anisotropy = if weather == "clear" {
        v.anisotropy.clear
    } else {
        v.anisotropy.other
    };
    let mut stats = VolumetricStats {
        mode,
        enabled,
        density: v.density.of(&weather),
        anisotropy,
        ground: (min.min(mean), mean, max.max(mean)),
        ..default()
    };
    if !enabled {
        commands.insert_resource(stats);
        return;
    }
    let fog_color = look.fog_of(weather_from_key(&weather)).color;
    let color = Color::srgb(fog_color[0], fog_color[1], fog_color[2]);
    commands.entity(sun).try_insert(VolumetricLight);
    let extinction = (v.absorption + v.scattering).max(1e-3);
    commands.spawn((
        Name::new("AirVeil"),
        AirVeil,
        FogVolume {
            fog_color: color,
            density_factor: stats.density / extinction,
            absorption: v.absorption,
            scattering: v.scattering,
            scattering_asymmetry: anisotropy,
            ..default()
        },
        Transform::IDENTITY,
        DespawnOnExit(AppState::Battle),
    ));
    stats.volumes += 1;
    if weather == "fog" {
        let mist = &v.ground_mist;
        let size = (16, 32, 16);
        let mut image = Image::new(
            Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: size.2,
            },
            TextureDimension::D3,
            mist_density(mist, size),
            TextureFormat::R8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = bevy::image::ImageSampler::linear();
        let c = |x: f32| x + (1.0 - x) * mist.lighten;
        commands.spawn((
            Name::new("GroundMist"),
            FogVolume {
                fog_color: Color::srgb(c(fog_color[0]), c(fog_color[1]), c(fog_color[2])),
                density_factor: mist.density / extinction,
                density_texture: Some(images.add(image)),
                absorption: v.absorption,
                scattering: v.scattering,
                scattering_asymmetry: anisotropy,
                ..default()
            },
            Transform::from_xyz(
                width * 0.5,
                mean + mist.lift_m + mist.height_m * 0.5,
                depth * 0.5,
            )
            .with_scale(Vec3::new(width, mist.height_m, depth)),
            DespawnOnExit(AppState::Battle),
        ));
        stats.volumes += 1;
        stats.ground_mist = true;
    }
    commands.insert_resource(stats);
}

fn attach_to_camera(
    mut commands: Commands,
    fx: Res<WeatherFxLook>,
    mut stats: ResMut<VolumetricStats>,
    cameras: Query<Entity, (With<BattleCamera>, Without<VolumetricFog>)>,
) {
    if !stats.enabled {
        return;
    }
    for camera in &cameras {
        commands.entity(camera).try_insert(VolumetricFog {
            ambient_intensity: fx.volumetric.ambient_intensity,
            step_count: fx.volumetric.step_count,
            ..default()
        });
        stats.cameras += 1;
    }
}

/// The veil box stays centred on the camera (Godot's froxels follow the view).
fn follow_camera(
    fx: Res<WeatherFxLook>,
    stats: Res<VolumetricStats>,
    cameras: Query<&GlobalTransform, With<BattleCamera>>,
    mut veils: Query<&mut Transform, With<AirVeil>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let v = &fx.volumetric;
    let (min, mean, _) = stats.ground;
    let bottom = min - 10.0;
    let top = mean + v.height_m;
    let p = camera.translation();
    for mut transform in &mut veils {
        *transform = Transform::from_xyz(p.x, (bottom + top) * 0.5, p.z).with_scale(Vec3::new(
            v.length_m * 2.0,
            top - bottom,
            v.length_m * 2.0,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_presets_gate_the_fog() {
        let root = paths::find_root().unwrap();
        let bytes = std::fs::read(paths::data_dir(&root).join("fx/render_quality.json")).unwrap();
        let presets: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(preset_mode(&presets, "low"), "off");
        assert_eq!(preset_mode(&presets, "ultra"), "always");
        assert!(enabled_for("weather", "rain"));
        assert!(!enabled_for("weather", "clear"));
        assert!(enabled_for("always", "clear"));
        assert!(!enabled_for("off", "fog"));
    }

    #[test]
    fn mist_is_dense_at_the_bottom_and_fades_up_and_out() {
        let mist = GroundMistLook {
            density: 0.01,
            height_m: 12.0,
            lift_m: 1.0,
            height_falloff: 0.35,
            edge_fade: 0.3,
            lighten: 0.1,
        };
        let size = (8, 8, 8);
        let d = mist_density(&mist, size);
        let at = |x: u32, y: u32, z: u32| d[(z * 64 + y * 8 + x) as usize];
        assert!(at(4, 0, 4) > at(4, 4, 4));
        assert!(at(4, 0, 4) > at(0, 0, 4));
        assert!(at(4, 7, 4) <= 2, "{}", at(4, 7, 4));
    }
}
