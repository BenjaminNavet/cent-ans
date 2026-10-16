//! Weather effects of a battle (`data/fx/battle_weather_fx.json`): rain and snow round the camera
//! (`precipitation`), volumetric fog (`volumetric`), smoke of the horizon villages
//! (`horizon_smoke`). Port of the effect half of game/scripts/battle/battle_atmosphere.gd and of
//! game/shaders/battle_horizon_smoke.gdshader.
//!
//! Dump section `battle_weather`: kind and count of the falling particles, volumetric mode and
//! volumes, horizon smoke columns.

use bevy::prelude::*;
use serde::Deserialize;

use crate::paths;

const DATA_FILE: &str = "fx/battle_weather_fx.json";

/// One kind of precipitation (`precipitation.rain` / `.snow`).
#[derive(Debug, Clone, Deserialize)]
pub struct FallLook {
    pub count: u32,
    pub box_m: [f32; 3],
    pub ahead_m: f32,
    pub above_m: f32,
    pub velocity: [f32; 3],
    pub speed_jitter: f32,
    pub size_m: [f32; 2],
    pub color: [f32; 4],
    pub fade_m: [f32; 2],
    pub sway_m: f32,
    pub sway_hz: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrecipitationLook {
    pub rain: FallLook,
    pub snow: FallLook,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DensityByWeather {
    pub clear: f32,
    pub fog: f32,
    pub rain: f32,
    pub snow: f32,
}

impl DensityByWeather {
    pub fn of(&self, weather: &str) -> f32 {
        match weather {
            "fog" => self.fog,
            "rain" => self.rain,
            "snow" => self.snow,
            _ => self.clear,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Anisotropy {
    pub clear: f32,
    pub other: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroundMistLook {
    pub density: f32,
    pub height_m: f32,
    pub lift_m: f32,
    pub height_falloff: f32,
    pub edge_fade: f32,
    pub lighten: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VolumetricLook {
    pub density: DensityByWeather,
    pub anisotropy: Anisotropy,
    pub absorption: f32,
    pub scattering: f32,
    pub length_m: f32,
    pub height_m: f32,
    pub step_count: u32,
    pub ambient_intensity: f32,
    pub ground_mist: GroundMistLook,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HorizonSmokeLook {
    pub size_m: [f32; 2],
    pub scatter_m: f32,
    pub base_lift_m: f32,
    pub opacity: f32,
}

/// Godot's `fog_height` / `fog_height_density` of the fog weather.
#[derive(Debug, Clone, Deserialize)]
pub struct HeightFogLook {
    pub weather: String,
    pub height_m: f32,
    pub density: f32,
    pub reference_m: f32,
}

/// `data/fx/battle_weather_fx.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
pub struct WeatherFxLook {
    pub precipitation: PrecipitationLook,
    pub volumetric: VolumetricLook,
    pub horizon_smoke: HorizonSmokeLook,
    pub height_fog: HeightFogLook,
}

impl WeatherFxLook {
    pub fn load() -> Option<WeatherFxLook> {
        let root = paths::find_root().ok()?;
        let bytes = std::fs::read(paths::data_dir(&root).join(DATA_FILE)).ok()?;
        match serde_json::from_slice(&bytes) {
            Ok(look) => Some(look),
            Err(error) => {
                error!("{DATA_FILE}: {error}");
                None
            }
        }
    }
}

/// sRGB triple to linear.
pub fn linear3(c: [f32; 3]) -> Vec3 {
    let lin = Color::srgb(c[0], c[1], c[2]).to_linear();
    Vec3::new(lin.red, lin.green, lin.blue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_file_parses() {
        let look = WeatherFxLook::load().expect("battle_weather_fx.json");
        assert_eq!(look.precipitation.rain.count, 9000);
        assert!(look.volumetric.density.of("rain") > look.volumetric.density.of("clear"));
        assert!(look.horizon_smoke.size_m[1] > look.horizon_smoke.size_m[0]);
    }
}
