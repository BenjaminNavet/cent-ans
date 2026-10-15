//! Pure weather model (no Bevy ECS): kinds, tuning read from `data/`, cloud opacity curve,
//! precipitation profile. Ports `CampaignWeatherView` (curves) and `PrecipitationProfile`.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use crate::campaign::terrain::tres::Tres;

/// Weather kind of a province for the current turn (`sim_campaign::MapWeather` keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Kind {
    #[default]
    Clear,
    Fog,
    Rain,
    Snow,
    Storm,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Clear, Kind::Fog, Kind::Rain, Kind::Snow, Kind::Storm];

    pub fn key(self) -> &'static str {
        match self {
            Kind::Clear => "clear",
            Kind::Fog => "fog",
            Kind::Rain => "rain",
            Kind::Snow => "snow",
            Kind::Storm => "storm",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.key() == key.trim())
    }

    /// Rain, snow and storm carry clouds; fog is a ground sheet (`has_clouds`).
    pub fn has_clouds(self) -> bool {
        matches!(self, Kind::Rain | Kind::Snow | Kind::Storm)
    }

    /// Falls from the sky near the camera.
    pub fn precipitates(self) -> bool {
        matches!(self, Kind::Rain | Kind::Snow | Kind::Storm)
    }
}

/// Weather of one province: kind and intensity within its class (0..1).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Entry {
    pub kind: Kind,
    pub intensity: f32,
}

pub type WeatherMap = HashMap<String, Entry>;

/// GLSL-style smoothstep.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if (edge1 - edge0).abs() < f32::EPSILON {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Cloud settings: `clouds` block of `data/ui/campaign_map.json` (Godot defaults as fallback).
#[derive(Debug, Clone, PartialEq)]
pub struct CloudTuning {
    pub height: f32,
    /// Camera distances between which clouds fade in (invisible closer).
    pub near: [f32; 2],
    pub max_alpha: f32,
    pub medium_alpha: f32,
    pub medium: [f32; 2],
    /// Camera-distance band over which the wide-view intensity cut rises.
    pub wide: [f32; 2],
    pub wide_cut_max: f32,
    pub weather_shadow: f32,
    pub clear_feather_px: f32,
    /// Seconds for the morning fog of a new turn to lift.
    pub fog_lift_seconds: f32,
    /// Share of the fog that stays once lifted (`morning_mist.lifted_share`).
    pub lifted_share: f32,
}

impl Default for CloudTuning {
    fn default() -> Self {
        Self {
            height: 36.0,
            near: [260.0, 520.0],
            max_alpha: 0.45,
            medium_alpha: 0.2,
            medium: [600.0, 1000.0],
            wide: [650.0, 1400.0],
            wide_cut_max: 0.5,
            weather_shadow: 0.12,
            clear_feather_px: 8.0,
            fog_lift_seconds: 40.0,
            lifted_share: 0.35,
        }
    }
}

fn pair(block: &Value, key: &str, default: [f32; 2]) -> [f32; 2] {
    match block.get(key).and_then(Value::as_array) {
        Some(a) if a.len() == 2 => [
            a[0].as_f64().unwrap_or(default[0] as f64) as f32,
            a[1].as_f64().unwrap_or(default[1] as f64) as f32,
        ],
        _ => default,
    }
}

fn number(block: &Value, key: &str, default: f32) -> f32 {
    block
        .get(key)
        .and_then(Value::as_f64)
        .map_or(default, |v| v as f32)
}

impl CloudTuning {
    /// Reads the tuning from the parsed `campaign_map.json`.
    pub fn from_json(root: &Value) -> Self {
        let d = Self::default();
        let clouds = root.get("clouds").cloned().unwrap_or(Value::Null);
        let mist = root.get("morning_mist").cloned().unwrap_or(Value::Null);
        Self {
            height: d.height,
            near: d.near,
            max_alpha: number(&clouds, "max_alpha", d.max_alpha),
            medium_alpha: number(&clouds, "medium_alpha", d.medium_alpha),
            medium: pair(&clouds, "medium_distance", d.medium),
            wide: d.wide,
            wide_cut_max: d.wide_cut_max,
            weather_shadow: number(&clouds, "weather_shadow", d.weather_shadow),
            clear_feather_px: number(&clouds, "clear_feather_px", d.clear_feather_px),
            fog_lift_seconds: d.fog_lift_seconds,
            lifted_share: number(&mist, "lifted_share", d.lifted_share),
        }
    }

    pub fn load(root: &Path) -> Self {
        std::fs::read(root.join("data/ui/campaign_map.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .map_or_else(Self::default, |v| Self::from_json(&v))
    }

    /// Cloud opacity at camera distance `distance`: none close, thin at medium range, full far.
    pub fn alpha_at(&self, distance: f32) -> f32 {
        let ceiling = lerp(
            self.medium_alpha,
            self.max_alpha,
            smoothstep(self.medium[0], self.medium[1], distance),
        );
        smoothstep(self.near[0], self.near[1], distance) * ceiling
    }

    /// Wide-view intensity cut (0 close, `wide_cut_max` far): only heavy weather keeps clouds.
    pub fn wide_cut(&self, distance: f32) -> f32 {
        smoothstep(self.wide[0], self.wide[1], distance) * self.wide_cut_max
    }

    /// Morning fog of the turn: 1 at the start, 0 once lifted (`morning_mist`).
    pub fn morning_mist(&self, clock: f32) -> f32 {
        1.0 - smoothstep(self.fog_lift_seconds * 0.15, self.fog_lift_seconds, clock)
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// `PrecipitationProfile` (`game/resources/precipitation.tres`): rain / snow sizes, box, speed
/// and density as continuous functions of the camera distance.
#[derive(Debug, Clone, PartialEq)]
pub struct PrecipitationProfile {
    pub enabled: bool,
    pub near_distance: f32,
    pub far_distance: f32,
    pub meters_per_unit: f32,
    pub rain_width_m: f32,
    pub rain_length_m: f32,
    pub rain_width_far: f32,
    pub rain_length_far: f32,
    pub snow_size_m: f32,
    pub snow_size_far: f32,
    pub box_h_m: f32,
    pub box_h_far: f32,
    pub box_v_m: f32,
    pub box_v_far: f32,
    pub height_m: f32,
    pub height_far: f32,
    pub rain_speed_m: [f32; 2],
    pub rain_speed_far: [f32; 2],
    pub snow_speed_m: [f32; 2],
    pub snow_speed_far: [f32; 2],
    pub snow_sway_m: f32,
    pub snow_sway_far: f32,
    pub amount_near: f32,
    pub amount_far: f32,
    pub rain_amount_scale: f32,
}

impl Default for PrecipitationProfile {
    fn default() -> Self {
        Self {
            enabled: true,
            near_distance: 1.8,
            far_distance: 30.0,
            meters_per_unit: 719.0,
            rain_width_m: 0.35,
            rain_length_m: 2.5,
            rain_width_far: 0.00035,
            rain_length_far: 0.011,
            snow_size_m: 0.2,
            snow_size_far: 0.0018,
            box_h_m: 40.0,
            box_h_far: 0.8,
            box_v_m: 8.0,
            box_v_far: 0.08,
            height_m: 25.0,
            height_far: 0.35,
            rain_speed_m: [6.0, 9.0],
            rain_speed_far: [0.6, 0.75],
            snow_speed_m: [0.8, 1.4],
            snow_speed_far: [0.08, 0.12],
            snow_sway_m: 0.3,
            snow_sway_far: 0.03,
            amount_near: 1.0,
            amount_far: 0.6,
            rain_amount_scale: 1.0,
        }
    }
}

impl PrecipitationProfile {
    /// Reads `precipitation.tres` (missing keys keep the script defaults).
    pub fn from_tres(tres: &Tres) -> Self {
        let d = Self::default();
        let v2 = |key: &str, default: [f32; 2]| match tres.array(key) {
            Some(a) if a.len() == 2 => [a[0], a[1]],
            _ => default,
        };
        Self {
            enabled: tres.bool("enabled", d.enabled),
            near_distance: tres.f32("near_distance", d.near_distance),
            far_distance: tres.f32("far_distance", d.far_distance),
            meters_per_unit: tres.f32("meters_per_unit", d.meters_per_unit),
            rain_width_m: tres.f32("rain_width_m_at_near", d.rain_width_m),
            rain_length_m: tres.f32("rain_length_m_at_near", d.rain_length_m),
            rain_width_far: tres.f32("rain_width_far_ratio", d.rain_width_far),
            rain_length_far: tres.f32("rain_length_far_ratio", d.rain_length_far),
            snow_size_m: tres.f32("snow_size_m_at_near", d.snow_size_m),
            snow_size_far: tres.f32("snow_size_far_ratio", d.snow_size_far),
            box_h_m: tres.f32("box_horizontal_m_at_near", d.box_h_m),
            box_h_far: tres.f32("box_horizontal_far_ratio", d.box_h_far),
            box_v_m: tres.f32("box_vertical_m_at_near", d.box_v_m),
            box_v_far: tres.f32("box_vertical_far_ratio", d.box_v_far),
            height_m: tres.f32("height_m_at_near", d.height_m),
            height_far: tres.f32("height_far_ratio", d.height_far),
            rain_speed_m: v2("rain_speed_ms_at_near", d.rain_speed_m),
            rain_speed_far: v2("rain_speed_far_ratio", d.rain_speed_far),
            snow_speed_m: v2("snow_speed_ms_at_near", d.snow_speed_m),
            snow_speed_far: v2("snow_speed_far_ratio", d.snow_speed_far),
            snow_sway_m: tres.f32("snow_sway_ms_at_near", d.snow_sway_m),
            snow_sway_far: tres.f32("snow_sway_far_ratio", d.snow_sway_far),
            amount_near: tres.f32("amount_ratio_near", d.amount_near),
            amount_far: tres.f32("amount_ratio_far", d.amount_far),
            rain_amount_scale: tres.f32("rain_amount_scale", d.rain_amount_scale),
        }
    }

    pub fn load(root: &Path) -> Self {
        Self::from_tres(&Tres::load(&root.join("game/resources/precipitation.tres")))
    }

    fn mpu(&self) -> f32 {
        self.meters_per_unit.max(1e-3)
    }

    /// Power-law interpolation between the metric size at `near_distance` and the far ratio.
    fn value(&self, distance: f32, near_value: f32, far_ratio: f32) -> f32 {
        let d1 = self.near_distance.max(1e-4);
        let d2 = self.far_distance.max(d1 * 1.01);
        let d = distance.max(1e-4);
        let far_value = far_ratio * d2;
        if d <= d1 {
            return near_value * (d / d1);
        }
        if d >= d2 {
            return far_ratio * d;
        }
        let t = (d.ln() - d1.ln()) / (d2.ln() - d1.ln());
        let lo = near_value.max(1e-9);
        let hi = far_value.max(1e-9);
        lo * (hi / lo).powf(t)
    }

    /// Rain streak (width, length) in world units.
    pub fn rain_size(&self, distance: f32) -> [f32; 2] {
        let m = self.mpu();
        [
            self.value(distance, self.rain_width_m / m, self.rain_width_far),
            self.value(distance, self.rain_length_m / m, self.rain_length_far),
        ]
    }

    pub fn snow_size(&self, distance: f32) -> f32 {
        self.value(distance, self.snow_size_m / self.mpu(), self.snow_size_far)
    }

    /// Emission box half-extents (horizontal, vertical).
    pub fn box_extents(&self, distance: f32) -> [f32; 2] {
        let m = self.mpu();
        [
            self.value(distance, self.box_h_m / m, self.box_h_far),
            self.value(distance, self.box_v_m / m, self.box_v_far),
        ]
    }

    /// Height of the box centre above the focus point.
    pub fn height(&self, distance: f32) -> f32 {
        self.value(distance, self.height_m / self.mpu(), self.height_far)
    }

    pub fn speed(&self, distance: f32, snow: bool) -> [f32; 2] {
        let m = self.mpu();
        let (near, far) = if snow {
            (self.snow_speed_m, self.snow_speed_far)
        } else {
            (self.rain_speed_m, self.rain_speed_far)
        };
        [
            self.value(distance, near[0] / m, far[0]),
            self.value(distance, near[1] / m, far[1]),
        ]
    }

    pub fn sway(&self, distance: f32) -> f32 {
        self.value(distance, self.snow_sway_m / self.mpu(), self.snow_sway_far)
    }

    /// Fraction of the pool that is active.
    pub fn amount_ratio(&self, distance: f32) -> f32 {
        let lo = self.near_distance.max(1e-4).ln();
        let hi = self.far_distance.max(self.near_distance * 1.01).ln();
        let t = smoothstep(lo, hi, distance.max(1e-4).ln());
        lerp(self.amount_near, self.amount_far, t)
    }
}

/// Cheap deterministic hash to unit interval (clouds, particles).
pub fn hash01(seed: u64) -> f32 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// Season banner timing (`SeasonBanner`): total seconds, fade seconds, slide px.
pub const BANNER_TOTAL_SECONDS: f32 = 1.2;
pub const BANNER_FADE_SECONDS: f32 = 0.3;
pub const BANNER_SLIDE_PX: f32 = 12.0;

/// Phase of the banner at `elapsed`: (visible, slide offset px downward). The slide eases in
/// from `SLIDE_PX` below, rests, then leaves upward.
pub fn banner_phase(elapsed: f32, reduce_motion: bool) -> (bool, f32) {
    if !(0.0..BANNER_TOTAL_SECONDS).contains(&elapsed) {
        return (false, 0.0);
    }
    if reduce_motion {
        return (true, 0.0);
    }
    let fade = BANNER_FADE_SECONDS;
    let slide = BANNER_SLIDE_PX;
    let offset = if elapsed < fade {
        let t = elapsed / fade;
        slide * (1.0 - (t * std::f32::consts::FRAC_PI_2).sin())
    } else if elapsed > BANNER_TOTAL_SECONDS - fade {
        let t = (elapsed - (BANNER_TOTAL_SECONDS - fade)) / fade;
        -slide * (1.0 - (t * std::f32::consts::FRAC_PI_2).cos())
    } else {
        0.0
    };
    (true, offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_alpha_follows_distance_bands() {
        let t = CloudTuning::default();
        assert_eq!(t.alpha_at(100.0), 0.0);
        assert!(t.alpha_at(400.0) > 0.0 && t.alpha_at(400.0) < t.medium_alpha);
        assert!((t.alpha_at(2000.0) - t.max_alpha).abs() < 1e-5);
        assert_eq!(t.wide_cut(100.0), 0.0);
        assert!((t.wide_cut(5000.0) - t.wide_cut_max).abs() < 1e-6);
    }

    #[test]
    fn tuning_reads_data_file() {
        let root = crate::paths::find_root().expect("repo root");
        let t = CloudTuning::load(&root);
        assert!((t.max_alpha - 0.3).abs() < 1e-6, "{t:?}");
        assert!((t.medium_alpha - 0.14).abs() < 1e-6);
        assert!((t.lifted_share - 0.35).abs() < 1e-6);
    }

    #[test]
    fn morning_mist_lifts() {
        let t = CloudTuning::default();
        assert_eq!(t.morning_mist(0.0), 1.0);
        assert_eq!(t.morning_mist(100.0), 0.0);
    }

    #[test]
    fn profile_reads_tres_and_is_continuous() {
        let root = crate::paths::find_root().expect("repo root");
        let p = PrecipitationProfile::load(&root);
        assert!((p.rain_width_m - 0.28).abs() < 1e-6);
        assert!((p.rain_amount_scale - 0.45).abs() < 1e-6);
        let below = p.rain_size(p.far_distance - 1e-3);
        let above = p.rain_size(p.far_distance + 1e-3);
        assert!((below[1] - above[1]).abs() / above[1] < 0.01);
        assert!(p.box_extents(100.0)[0] > p.box_extents(2.0)[0]);
        assert!((p.amount_ratio(1000.0) - p.amount_far).abs() < 1e-5);
    }

    #[test]
    fn kinds_round_trip() {
        for k in Kind::ALL {
            assert_eq!(Kind::from_key(k.key()), Some(k));
        }
        assert!(Kind::Storm.has_clouds() && !Kind::Fog.has_clouds());
        assert_eq!(Kind::from_key("hail"), None);
    }

    #[test]
    fn banner_phases() {
        assert_eq!(banner_phase(-0.1, false), (false, 0.0));
        assert_eq!(banner_phase(0.6, false), (true, 0.0));
        assert!(banner_phase(0.0, false).1 > 11.0);
        assert!(banner_phase(1.19, false).1 < -10.0);
        assert!(!banner_phase(1.3, false).0);
        assert_eq!(banner_phase(0.0, true), (true, 0.0));
    }

    #[test]
    fn hash_is_unit_and_stable() {
        let a = hash01(7);
        assert!((0.0..1.0).contains(&a));
        assert_eq!(a, hash01(7));
        assert_ne!(a, hash01(8));
    }
}
