//! Campaign ambience layers: pure level computation. Port of `CampaignAmbience.layer_levels`
//! (game/scripts/audio/campaign_ambience.gd). The campaign lot fills `AmbienceEnv` (terrain
//! share under the camera, zoom, season, weather); the director fades the layers.

pub const LAYERS: [&str; 8] = [
    "sea",
    "countryside",
    "crickets",
    "forest",
    "town",
    "wind",
    "wind_strong",
    "rain",
];

#[derive(Debug, Clone, PartialEq)]
pub struct AmbienceEnv {
    /// Shares 0..1 of sea / forest / fields / nearby town under the camera.
    pub sea: f32,
    pub forest: f32,
    pub fields: f32,
    pub town: f32,
    /// Camera distance (zoom).
    pub distance: f32,
    /// `spring`, `summer`, `autumn`, `winter`.
    pub season: String,
    /// `clear`, `fog`, `rain`, `snow`, `storm`.
    pub weather: String,
}

impl Default for AmbienceEnv {
    fn default() -> Self {
        Self {
            sea: 0.0,
            forest: 0.0,
            fields: 0.0,
            town: 0.0,
            distance: 500.0,
            season: "spring".into(),
            weather: "clear".into(),
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn birds(season: &str) -> f32 {
    match season {
        "spring" => 1.0,
        "summer" => 0.8,
        "autumn" => 0.45,
        "winter" => 0.12,
        _ => 0.5,
    }
}

fn crickets(season: &str) -> f32 {
    match season {
        "spring" => 0.2,
        "summer" => 1.0,
        "autumn" => 0.3,
        _ => 0.0,
    }
}

fn winter_wind(season: &str) -> f32 {
    match season {
        "spring" => 0.1,
        "autumn" => 0.35,
        "winter" => 0.8,
        _ => 0.0,
    }
}

/// Level 0..1 of each layer of [`LAYERS`] (same order).
pub fn layer_levels(env: &AmbienceEnv) -> [f32; 8] {
    let near = 1.0 - smoothstep(80.0, 700.0, env.distance);
    let far = smoothstep(250.0, 1100.0, env.distance);
    let wet = env.weather == "rain" || env.weather == "storm";
    let wet_birds = if wet { 0.35 } else { 1.0 };
    let dry = if wet { 0.0 } else { 1.0 };
    let winter_forest = if env.season == "winter" { 0.45 } else { 1.0 };
    let snowy = env.weather == "snow" || env.weather == "storm";
    let c = |x: f32| x.clamp(0.0, 1.0);
    [
        c(env.sea * (0.35 + 0.65 * near)),
        c(env.fields * near * birds(&env.season) * wet_birds * (1.0 - 0.6 * env.town)),
        c(env.fields * near * crickets(&env.season) * dry * (1.0 - env.town)),
        c(env.forest * 1.4 * near * winter_forest),
        c(env.town * near),
        c(0.2 + 0.7 * far + if wet { 0.2 } else { 0.0 }),
        c(winter_wind(&env.season) * (0.35 + 0.65 * far) + if snowy { 0.5 } else { 0.0 }),
        if wet { 0.5 + 0.5 * near } else { 0.0 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(levels: &[f32; 8], name: &str) -> f32 {
        levels[LAYERS.iter().position(|layer| *layer == name).unwrap()]
    }

    #[test]
    fn rain_covers_birds_and_crickets() {
        let mut env = AmbienceEnv {
            fields: 1.0,
            distance: 100.0,
            season: "summer".into(),
            ..Default::default()
        };
        let dry = layer_levels(&env);
        env.weather = "rain".into();
        let wet = layer_levels(&env);
        assert!(level(&dry, "countryside") > level(&wet, "countryside"));
        assert!(level(&dry, "crickets") > 0.9 && level(&wet, "crickets") == 0.0);
        assert!(level(&wet, "rain") > 0.9 && level(&dry, "rain") == 0.0);
    }

    #[test]
    fn zoom_out_brings_the_high_wind() {
        let near = layer_levels(&AmbienceEnv {
            distance: 50.0,
            ..Default::default()
        });
        let far = layer_levels(&AmbienceEnv {
            distance: 1500.0,
            ..Default::default()
        });
        assert!(level(&far, "wind") > level(&near, "wind"));
        assert!(level(&near, "forest") == 0.0);
    }

    #[test]
    fn winter_snow_is_windy() {
        let env = AmbienceEnv {
            season: "winter".into(),
            weather: "snow".into(),
            distance: 1200.0,
            ..Default::default()
        };
        assert!(level(&layer_levels(&env), "wind_strong") > 0.9);
    }
}
