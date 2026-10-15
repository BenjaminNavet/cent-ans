//! Look of the battle decor read from `data/fx/battle_decor_look.json` (fog, ambient, sky,
//! horizon, wood ring, vines, orchards, grass and wind, ground). Purely visual values: nothing
//! here is a game rule. The file is read at runtime; the copy bundled in the binary is the
//! fallback when the data directory cannot be found.

use bevy::prelude::*;
use serde::Deserialize;
use sim_battle::Weather;

use crate::paths;

const BUNDLED: &str = include_str!("../../../../data/fx/battle_decor_look.json");

#[derive(Debug, Clone, Deserialize)]
pub struct FogLook {
    pub density: f32,
    pub horizon_density: f32,
    pub color: [f32; 3],
    pub ambient: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AmbientLook {
    pub color: [f32; 3],
    pub lux: f32,
    pub low_sun_floor: f32,
    pub low_sun_span_deg: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkyColors {
    pub zenith: [f32; 3],
    pub horizon: [f32; 3],
    pub glow: [f32; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkyLook {
    pub clear: SkyColors,
    pub fog: SkyColors,
    pub rain: SkyColors,
    pub snow: SkyColors,
    pub dome_radius_m: f32,
    pub horizon_band_deg: f32,
    pub glow_width_deg: f32,
    pub glow_strength: f32,
    pub clear_color_mix: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeatherValues {
    pub clear: f32,
    pub fog: f32,
    pub rain: f32,
    pub snow: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FallbackRing {
    pub enabled: bool,
    pub relief_m: f32,
    pub noise_scale_m: f32,
    pub forest_share: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HorizonLook {
    pub enabled: bool,
    pub ring_radius_m: f32,
    pub sectors: usize,
    pub distances_m: Vec<f32>,
    pub colors_mix_with_field_m: f32,
    pub skyline_radius_m: f32,
    pub skyline_exaggeration: f32,
    pub skyline_min_deg: f32,
    pub skyline_max_deg: f32,
    pub skyline_haze: WeatherValues,
    pub skyline_color: [f32; 3],
    pub sea_color: [f32; 3],
    pub fallback_ring: FallbackRing,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForestRing {
    pub clumps_per_km2: f32,
    pub reach_m: f32,
    pub clump_radius_m: [f32; 2],
    pub area_per_tree_m2: f32,
    pub max_trees: usize,
    pub fade_start_m: f32,
    pub fade_end_m: f32,
    pub trunk_range_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrchardLook {
    pub step_m: f32,
    pub dead_share: f32,
    pub margin_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VineyardLook {
    pub row_step_m: f32,
    pub segment_m: f32,
    pub height_m: f32,
    pub bare_height_m: f32,
    pub thickness_m: f32,
    pub leaf_color: [f32; 3],
    pub bare_color: [f32; 3],
    pub range_m: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct WindLook {
    pub amplitude_m: f32,
    pub speed: f32,
    pub gust_amplitude_m: f32,
    pub gust_speed: f32,
    pub direction_deg: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrassLook {
    pub spacing_m: f32,
    pub chunk_m: f32,
    pub fade_near_m: f32,
    pub fade_far_m: f32,
    pub tint_gain: f32,
    pub wind: WindLook,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroundLook {
    pub normal_strength: f32,
    pub normal_fade_m: f32,
    /// Distance (m) where the layer textures fade to their mean colour.
    pub texture_fade_m: f32,
    /// Brightness variation of the macro noise over the ground (0 = none).
    pub macro_contrast: f32,
    pub anisotropy: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FogSet {
    pub clear: FogLook,
    pub fog: FogLook,
    pub rain: FogLook,
    pub snow: FogLook,
}

/// All the values of `battle_decor_look.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
pub struct DecorLook {
    pub fog: FogSet,
    pub ambient: AmbientLook,
    pub sky: SkyLook,
    pub horizon: HorizonLook,
    pub forest_ring: ForestRing,
    pub orchard: OrchardLook,
    pub vineyard: VineyardLook,
    pub grass: GrassLook,
    pub tree_wind: WindLook,
    pub ground: GroundLook,
}

impl DecorLook {
    /// The file under `data/fx`, or the bundled copy.
    pub fn load() -> DecorLook {
        let from_disk = paths::find_root()
            .ok()
            .and_then(|root| {
                std::fs::read(paths::data_dir(&root).join("fx/battle_decor_look.json")).ok()
            })
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        from_disk.unwrap_or_else(|| serde_json::from_str(BUNDLED).expect("bundled decor look"))
    }

    pub fn fog_of(&self, weather: Weather) -> &FogLook {
        match weather {
            Weather::Clear => &self.fog.clear,
            Weather::Fog => &self.fog.fog,
            Weather::Rain => &self.fog.rain,
            Weather::Snow => &self.fog.snow,
        }
    }

    pub fn sky_of(&self, weather: Weather) -> &SkyColors {
        match weather {
            Weather::Clear => &self.sky.clear,
            Weather::Fog => &self.sky.fog,
            Weather::Rain => &self.sky.rain,
            Weather::Snow => &self.sky.snow,
        }
    }

    /// Value of a per-weather block.
    pub fn weather_value(values: &WeatherValues, weather: Weather) -> f32 {
        match weather {
            Weather::Clear => values.clear,
            Weather::Fog => values.fog,
            Weather::Rain => values.rain,
            Weather::Snow => values.snow,
        }
    }
}

/// Wind direction (unit vector on the ground) of a look.
pub fn wind_direction(wind: &WindLook) -> Vec2 {
    let a = wind.direction_deg.to_radians();
    Vec2::new(a.sin(), -a.cos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_look_parses_and_matches_the_data_file() {
        let bundled: DecorLook = serde_json::from_str(BUNDLED).unwrap();
        let loaded = DecorLook::load();
        assert!((bundled.fog.clear.density - loaded.fog.clear.density).abs() < 1e-9);
        assert!(bundled.horizon.distances_m.windows(2).all(|w| w[0] < w[1]));
        assert!(bundled.horizon.ring_radius_m < bundled.sky.dome_radius_m);
    }
}
