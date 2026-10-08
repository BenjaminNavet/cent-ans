//! Campaign map weather (lot CM2), mirroring
//! `data/schemas/campaign_weather_rules.schema.json` (`data/rules/campaign_weather.json`).

use serde::{Deserialize, Serialize};

/// Chances (per cent) of each map weather in one season and climate; they
/// add up to 100. Kinds are ordered from the driest to the wettest: the
/// weather field ranks provinces along that order, so a wet front brings
/// rain, snow or storms to a whole region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignWeatherChances {
    pub clear: u32,
    pub fog: u32,
    pub rain: u32,
    pub snow: u32,
    pub storm: u32,
}

impl CampaignWeatherChances {
    pub fn total(&self) -> u32 {
        self.clear + self.fog + self.rain + self.snow + self.storm
    }
}

/// Chances for the four seasons of one climate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonalWeather {
    pub spring: CampaignWeatherChances,
    pub summer: CampaignWeatherChances,
    pub autumn: CampaignWeatherChances,
    pub winter: CampaignWeatherChances,
}

/// Seasonal chances per climate (`Province::climate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClimateWeather {
    pub oceanic: SeasonalWeather,
    pub continental: SeasonalWeather,
    pub mediterranean: SeasonalWeather,
    pub mountain: SeasonalWeather,
    /// OM3 (ADR 0116): hot and dry, almost no rain.
    pub arid: SeasonalWeather,
    /// OM3 (ADR 0116): dry continental grassland.
    pub steppe: SeasonalWeather,
}

/// Contents of `data/rules/campaign_weather.json`: the weather drawn on the
/// campaign map each turn, per province, from the seed and the date. It is
/// purely visual (no rule reads it yet; see ADR 0027).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignWeatherRules {
    /// Size of the weather fronts, in degrees of longitude/latitude.
    pub front_scale_deg: f64,
    /// Share of the per-province noise mixed into the regional field
    /// (0 = whole fronts, 1 = every province on its own).
    pub local_jitter: f64,
    /// Displacement of the weather field between two turns, in degrees
    /// (east, north): fronts come in from the Atlantic.
    pub drift_deg_per_turn: [f64; 2],
    pub climates: ClimateWeather,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

crate::bundled_rules!(CampaignWeatherRules, "rules/campaign_weather.json", default);
