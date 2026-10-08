//! Campaign map weather (lot CM2): rain, snow, morning fog and storms that
//! sweep across the map, per province and per turn.
//!
//! The weather is a pure function of the game seed, the turn (and its
//! season) and the position of each province's capital: nothing is stored in
//! the state and the campaign random stream is not touched, so saves,
//! replays and the AI are unaffected. A regional noise field (fronts about
//! `front_scale_deg` wide, drifting east by `drift_deg_per_turn` each turn)
//! plus a little per-province noise ranks the provinces from the driest to
//! the wettest; each province then reads its rank against the chances of its
//! climate and season (`data/rules/campaign_weather.json`). Ranking makes
//! the chances hold exactly over the map while keeping whole regions under
//! the same sky.
//!
//! Purely visual for now: no rule reads it (ADR 0027).

use data_model::util::splitmix64;
use std::collections::BTreeMap;

use data_model::{
    CampaignWeatherChances, CampaignWeatherRules, Climate, GameData, Province, ProvinceId,
};

use crate::state::{CampaignState, Season};

/// Weather over a province for the current turn, from the driest to the
/// wettest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MapWeather {
    Clear,
    /// Morning fog (lifts during the day; lingers in valleys).
    Fog,
    Rain,
    Snow,
    /// Thunderstorm (summer mostly).
    Storm,
}

impl MapWeather {
    pub const ALL: [MapWeather; 5] = [
        MapWeather::Clear,
        MapWeather::Fog,
        MapWeather::Rain,
        MapWeather::Snow,
        MapWeather::Storm,
    ];

    /// Identifier shared with the bridge and the data files.
    pub fn as_str(self) -> &'static str {
        match self {
            MapWeather::Clear => "clear",
            MapWeather::Fog => "fog",
            MapWeather::Rain => "rain",
            MapWeather::Snow => "snow",
            MapWeather::Storm => "storm",
        }
    }

    /// French label for the interface.
    pub fn label_fr(self) -> &'static str {
        match self {
            MapWeather::Clear => "Temps clair",
            MapWeather::Fog => "Brouillard matinal",
            MapWeather::Rain => "Pluie",
            MapWeather::Snow => "Neige",
            MapWeather::Storm => "Orage",
        }
    }
}

/// Weather of one province: its kind and how strong it is within that kind
/// (0 = barely, 1 = the heaviest of its class on the map this turn).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProvinceWeather {
    pub kind: MapWeather,
    pub intensity: f64,
}

/// Weather of every province for the current turn of `state`.
pub fn weather_map(
    data: &GameData,
    state: &CampaignState,
) -> BTreeMap<ProvinceId, ProvinceWeather> {
    compute_weather(
        &data.campaign_weather,
        &data.provinces,
        state.seed,
        state.turn(),
        state.season(),
    )
}

/// Weather of one province for the current turn (the whole map is ranked:
/// cheap, a few hundred provinces).
pub fn province_weather(
    data: &GameData,
    state: &CampaignState,
    province: &ProvinceId,
) -> Option<ProvinceWeather> {
    weather_map(data, state).get(province).copied()
}

/// Weather of every province of `provinces` for a seed, a turn and a season.
pub fn compute_weather(
    rules: &CampaignWeatherRules,
    provinces: &BTreeMap<ProvinceId, Province>,
    seed: u64,
    turn: u32,
    season: Season,
) -> BTreeMap<ProvinceId, ProvinceWeather> {
    let scale = rules.front_scale_deg.max(0.1);
    let jitter = rules.local_jitter.clamp(0.0, 1.0);
    let shift_x = rules.drift_deg_per_turn[0] * f64::from(turn);
    let shift_y = rules.drift_deg_per_turn[1] * f64::from(turn);
    let turn_seed =
        splitmix64(seed ^ 0x0057_4541_5448_4552_u64 ^ u64::from(turn).wrapping_mul(0x9E37_79B9));
    let mut raw: Vec<(f64, &ProvinceId)> = provinces
        .iter()
        .map(|(id, province)| {
            let (lon, lat) = position(province);
            let x = (lon - shift_x) / scale;
            let y = (lat - shift_y) / scale;
            // Fronts that drift (fixed field) and change shape (per-turn field).
            let front = 0.62 * value_noise(x, y, seed)
                + 0.38 * value_noise(x * 1.7 + 11.3, y * 1.7 - 4.1, turn_seed);
            let local = unit(splitmix64(turn_seed ^ hash_str(id.as_str())));
            ((1.0 - jitter) * front + jitter * local, id)
        })
        .collect();
    raw.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    let n = raw.len().max(1) as f64;
    raw.iter()
        .enumerate()
        .map(|(rank, (_, id))| {
            let u = (rank as f64 + 0.5) / n;
            let province = &provinces[*id];
            let chances = chances_for(rules, province.climate, season);
            (ProvinceId::clone(id), classify(&chances, u))
        })
        .collect()
}

/// Chances of a climate (oceanic when unknown) in a season.
pub fn chances_for(
    rules: &CampaignWeatherRules,
    climate: Option<Climate>,
    season: Season,
) -> CampaignWeatherChances {
    let seasonal = match climate.unwrap_or(Climate::Oceanic) {
        Climate::Oceanic => &rules.climates.oceanic,
        Climate::Continental => &rules.climates.continental,
        Climate::Mediterranean => &rules.climates.mediterranean,
        Climate::Mountain => &rules.climates.mountain,
        Climate::Arid => &rules.climates.arid,
        Climate::Steppe => &rules.climates.steppe,
    };
    match season {
        Season::Spring => seasonal.spring,
        Season::Summer => seasonal.summer,
        Season::Autumn => seasonal.autumn,
        Season::Winter => seasonal.winter,
    }
}

/// Kind and intensity for a wetness rank `u` in [0, 1).
fn classify(chances: &CampaignWeatherChances, u: f64) -> ProvinceWeather {
    let total = f64::from(chances.total().max(1));
    let bins = [
        (MapWeather::Clear, chances.clear),
        (MapWeather::Fog, chances.fog),
        (MapWeather::Rain, chances.rain),
        (MapWeather::Snow, chances.snow),
        (MapWeather::Storm, chances.storm),
    ];
    let mut low = 0.0;
    let mut last = ProvinceWeather {
        kind: MapWeather::Clear,
        intensity: 0.0,
    };
    for (kind, chance) in bins {
        if chance == 0 {
            continue;
        }
        let high = low + f64::from(chance) / total;
        last = ProvinceWeather {
            kind,
            intensity: 1.0,
        };
        if u < high {
            return ProvinceWeather {
                kind,
                intensity: ((u - low) / (high - low)).clamp(0.0, 1.0),
            };
        }
        low = high;
    }
    last
}

/// Longitude and latitude of the capital (geo data, else the capital city).
fn position(province: &Province) -> (f64, f64) {
    if let Some(geo) = &province.geo {
        return (geo.capital_lonlat[0], geo.capital_lonlat[1]);
    }
    match (province.capital_city.lon, province.capital_city.lat) {
        (Some(lon), Some(lat)) => (lon, lat),
        _ => (0.0, 0.0),
    }
}

/// Smooth value noise in [0, 1] on a unit lattice hashed with `seed`.
fn value_noise(x: f64, y: f64, seed: u64) -> f64 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (x - ix, y - iy);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let corner = |dx: f64, dy: f64| {
        let cx = (ix + dx) as i64 as u64;
        let cy = (iy + dy) as i64 as u64;
        unit(splitmix64(
            seed ^ cx.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ cy.wrapping_mul(0xC2B2_AE3D_27D4_EB4F),
        ))
    };
    let top = corner(0.0, 0.0) + (corner(1.0, 0.0) - corner(0.0, 0.0)) * sx;
    let bottom = corner(0.0, 1.0) + (corner(1.0, 1.0) - corner(0.0, 1.0)) * sx;
    top + (bottom - top) * sy
}

fn unit(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// FNV-1a over the id (stable across platforms and builds).
fn hash_str(s: &str) -> u64 {
    s.bytes().fold(0xCBF2_9CE4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chances(clear: u32, fog: u32, rain: u32, snow: u32, storm: u32) -> CampaignWeatherChances {
        CampaignWeatherChances {
            clear,
            fog,
            rain,
            snow,
            storm,
        }
    }

    #[test]
    fn classify_follows_the_cumulative_chances() {
        let c = chances(50, 10, 30, 0, 10);
        assert_eq!(classify(&c, 0.1).kind, MapWeather::Clear);
        assert_eq!(classify(&c, 0.55).kind, MapWeather::Fog);
        assert_eq!(classify(&c, 0.7).kind, MapWeather::Rain);
        assert_eq!(classify(&c, 0.95).kind, MapWeather::Storm);
        // No snow at 0 %; intensity runs across the bin.
        assert!((classify(&c, 0.75).intensity - 0.5).abs() < 1e-9);
    }

    #[test]
    fn value_noise_is_bounded_and_deterministic() {
        for i in 0..200 {
            let x = i as f64 * 0.37 - 20.0;
            let v = value_noise(x, x * 0.5, 7);
            assert!((0.0..=1.0).contains(&v));
            assert_eq!(v, value_noise(x, x * 0.5, 7));
        }
    }
}
