//! Height fog of the fog weather (port of `env.fog_height = 12` / `fog_height_density = 0.04` of
//! game/scripts/battle/battle_atmosphere.gd: "low sheets in the hollows").
//!
//! Godot raised the fog of every fragment below world height 12 m, whatever its distance:
//! `amount = max(distance_fog, 1 - exp(-0.04 x (12 - y)))`. Bevy's `DistanceFog` has no height term
//! and the per-fragment fog belongs to the global pipeline, so the layer is folded into the
//! exponential density of the battle camera: the opacity the layer gives to a ray (taken at the
//! mean altitude of the ray between the camera and the mean ground) is turned into the density
//! that gives the same opacity over `reference_m`, and the larger of that and the base density is
//! used. The camera rising out of the layer thins the fog out, as Godot's did.

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

use super::atmosphere::AtmosphereState;
use super::weather_fx::{HeightFogLook, WeatherFxLook};
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;

/// What the height fog adds (read by the dump).
#[derive(Resource, Debug, Clone, Default)]
pub struct HeightFogStats {
    pub active: bool,
    pub ground_mean: f32,
    pub camera_y: f32,
    pub density: f32,
    pub base_density: f32,
}

/// Distance-fog density of a camera at `camera_y` over ground of mean altitude `ground_y`.
pub fn height_density(look: &HeightFogLook, base: f32, camera_y: f32, ground_y: f32) -> f32 {
    let ray_y = 0.5 * (camera_y + ground_y);
    let depth = (look.height_m - ray_y).max(0.0);
    let layer = look.density * depth / look.reference_m.max(1.0);
    base.max(layer)
}

pub struct HeightFogPlugin;

impl Plugin for HeightFogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_height_fog
                .run_if(in_state(AppState::Battle))
                .run_if(resource_exists::<AtmosphereState>)
                .run_if(resource_exists::<WeatherFxLook>),
        )
        .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
            commands.remove_resource::<HeightFogStats>();
        });
    }
}

fn apply_height_fog(
    mut commands: Commands,
    atmosphere: Res<AtmosphereState>,
    fx: Res<WeatherFxLook>,
    mut session: ResMut<BattleSession>,
    mut cameras: Query<(&GlobalTransform, &mut DistanceFog), With<BattleCamera>>,
    stats: Option<ResMut<HeightFogStats>>,
) {
    let look = &fx.height_fog;
    // `fog_density` is the base density the camera fog was given (`fog_on_camera`).
    if atmosphere.weather != look.weather || !atmosphere.fog_on_camera {
        if stats.is_none() {
            commands.insert_resource(HeightFogStats::default());
        }
        return;
    }
    let ground = match &stats {
        Some(s) if s.active => s.ground_mean,
        _ => {
            let heights = &session.sim_mut().field().heights;
            heights.iter().sum::<f64>() as f32 / heights.len().max(1) as f32
        }
    };
    let Some((transform, mut fog)) = cameras.iter_mut().next() else {
        return;
    };
    let camera_y = transform.translation().y;
    let density = height_density(look, atmosphere.fog_density, camera_y, ground);
    fog.falloff = FogFalloff::Exponential { density };
    let new = HeightFogStats {
        active: true,
        ground_mean: ground,
        camera_y,
        density,
        base_density: atmosphere.fog_density,
    };
    match stats {
        Some(mut s) => *s = new,
        None => commands.insert_resource(new),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> HeightFogLook {
        HeightFogLook {
            weather: "fog".into(),
            height_m: 12.0,
            density: 0.04,
            reference_m: 150.0,
        }
    }

    #[test]
    fn low_camera_gets_the_layer_and_a_high_one_loses_it() {
        let l = look();
        let low = height_density(&l, 0.0, 3.0, 0.0);
        // ray at 1.5 m: depth 10.5 m, opacity 1 - exp(-0.42)
        assert!((low - 0.04 * 10.5 / 150.0).abs() < 1e-6, "{low}");
        assert_eq!(height_density(&l, 0.0, 60.0, 10.0), 0.0);
    }

    #[test]
    fn never_thinner_than_the_distance_fog() {
        assert_eq!(height_density(&look(), 0.5, 3.0, 0.0), 0.5);
    }
}
