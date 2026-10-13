//! Camera, sun, fog and the two quality presets of CONTRACT.md.

use bevy::{
    anti_alias::{fxaa::Fxaa, taa::TemporalAntiAliasing},
    camera::{Exposure, Hdr},
    core_pipeline::tonemapping::Tonemapping,
    light::{
        Atmosphere, AtmosphereEnvironmentMapLight, CascadeShadowConfigBuilder, FogVolume,
        GlobalAmbientLight, SunDisk, VolumetricFog, VolumetricLight, atmosphere::ScatteringMedium,
        light_consts::lux,
    },
    pbr::{AtmosphereSettings, DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion},
    post_process::bloom::Bloom,
    prelude::*,
};

use crate::scene::{Quality, Scene};

/// Marker of the main camera.
#[derive(Component)]
pub struct MainCamera;

/// Fixed exposure (EV100) shared by both presets.
const EXPOSURE_EV100: f32 = 13.0;

fn color3(rgb: [f32; 3]) -> Color {
    Color::srgb(rgb[0], rgb[1], rgb[2])
}

/// Sun direction as in `godot/proto_shot.gd`: pitch `-elevation`, yaw `180 - azimuth`.
pub fn sun_rotation(elevation_deg: f32, azimuth_deg: f32) -> Quat {
    Quat::from_euler(
        EulerRot::YXZ,
        (180.0 - azimuth_deg).to_radians(),
        (-elevation_deg).to_radians(),
        0.0,
    )
}

pub fn spawn_view(commands: &mut Commands, scene: &Scene, media: &mut Assets<ScatteringMedium>) {
    let spec = &scene.camera;
    let camera_transform = Transform::from_translation(Vec3::from_array(spec.pos))
        .looking_at(Vec3::from_array(spec.look_at), Vec3::Y);
    let camera_id = commands
        .spawn((
            MainCamera,
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: spec.fov_y.to_radians(),
                near: 0.1,
                far: 2000.0,
                ..default()
            }),
            camera_transform,
            Hdr,
            Msaa::Off,
            Tonemapping::AgX,
            Exposure {
                ev100: EXPOSURE_EV100,
            },
            Bloom {
                intensity: 0.08,
                ..Bloom::NATURAL
            },
            ScreenSpaceAmbientOcclusion::default(),
        ))
        .id();
    let mut camera = commands.entity(camera_id);

    let sun_color = color3(scene.sun.color);
    let mut sun_illuminance = scene.sun.lux;
    match scene.quality {
        Quality::Comparable => {
            camera.insert((
                Fxaa::default(),
                DistanceFog {
                    color: color3(scene.fog.color),
                    falloff: FogFalloff::Exponential {
                        density: scene.fog.density,
                    },
                    ..default()
                },
            ));
            commands.insert_resource(ClearColor(color3(scene.fog.color)));
            commands.insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.62, 0.72, 0.9),
                brightness: 1800.0,
                ..default()
            });
        }
        Quality::Best => {
            sun_illuminance = lux::RAW_SUNLIGHT;
            let earth = media.add(ScatteringMedium::earth(256, 256));
            camera.insert((
                AtmosphereSettings::default(),
                AtmosphereEnvironmentMapLight::default(),
                VolumetricFog {
                    ambient_intensity: 0.0,
                    ..default()
                },
                TemporalAntiAliasing::default(),
            ));
            commands.spawn(Atmosphere::earth(earth));
            commands.insert_resource(GlobalAmbientLight::NONE);
            commands.spawn((
                FogVolume {
                    fog_color: color3(scene.fog.color),
                    density_factor: scene.fog.density,
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, 60.0, 0.0))
                    .with_scale(Vec3::new(1000.0, 120.0, 1000.0)),
            ));
        }
    }

    let mut sun = commands.spawn((
        DirectionalLight {
            color: sun_color,
            illuminance: sun_illuminance,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(sun_rotation(scene.sun.elevation, scene.sun.azimuth)),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            maximum_distance: 150.0,
            first_cascade_far_bound: 6.0,
            overlap_proportion: 0.2,
            ..default()
        }
        .build(),
    ));
    if scene.quality == Quality::Best {
        sun.insert((VolumetricLight, SunDisk::EARTH));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_light_travels_away_from_its_azimuth() {
        // Azimuth 0 = sun in the -Z direction, so the light travels toward +Z and downward.
        let direction = sun_rotation(30.0, 0.0) * Vec3::NEG_Z;
        assert!(direction.z > 0.8 && direction.y < -0.4 && direction.x.abs() < 1e-5);
        // Azimuth 90 = sun toward +X, light travels toward -X.
        let direction = sun_rotation(14.0, 90.0) * Vec3::NEG_Z;
        assert!(direction.x < -0.9);
    }
}
