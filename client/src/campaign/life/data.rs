//! Data of the living map, read from `data/` (nothing is hard-coded here except the loader).
//!
//! Files: `art/town_maquettes.json` (`props`), `rules/map_scenes.json`, `map/map_birds.json`,
//! `map/map_fauna.json`, `map/wetland_sites_px.json`, `map/sea_lanes_px.json`, `map/hamlets.json`,
//! `map/land_mask.png` (grey, 255 = land), `fx/map_fire_wind.json`, `fx/map_atmosphere.json`
//! (wind), `art/dn_water_models.json` (boats), and through [`WaterData`] the rivers, roads and sea
//! basins. A missing file gives an empty/neutral value and a warning.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;
use serde_json::Value;

use crate::campaign::provinces::projection::MapProjection;
use crate::campaign::water::data::WaterData;

/// Deterministic hash of integers into [0, 1) (`Hash.h01` of the Godot folk code, same idea:
/// splitmix64 finaliser).
pub fn h01(a: u64, b: u64, c: u64) -> f32 {
    let mut x = a
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(b.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(c.wrapping_mul(0x94D0_49BB_1331_11EB))
        .wrapping_add(0x2545_F491_4F6C_DD1D);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    ((x >> 40) as f32) / 16_777_216.0
}

/// Stable seed of a text (FNV-1a), `MapInstancing.text_seed`.
pub fn text_seed(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// `props` of `town_maquettes.json` used by the living map.
#[derive(Debug, Clone)]
pub struct FolkProps {
    pub folk_scale: f32,
    pub folk_density: f32,
    pub folk_range: f32,
    pub folk_speed: f32,
    pub folk_radius_factor: f32,
    pub keep_from: f32,
    pub keep_exponent: f32,
    pub keep_min: f32,
    pub windmill_ratio: f32,
    pub chimney_ratio: f32,
    pub windmill_range: f32,
    pub chimney_range: f32,
    pub zone_margin: f32,
    pub folk_edge_inner: f32,
    pub folk_range_fade: f32,
}

impl Default for FolkProps {
    fn default() -> Self {
        Self {
            folk_scale: 675.0,
            folk_density: 1.0,
            folk_range: 450.0,
            folk_speed: 0.5,
            folk_radius_factor: 1.6,
            keep_from: 50.0,
            keep_exponent: 2.0,
            keep_min: 0.1,
            windmill_ratio: 1.0,
            chimney_ratio: 0.4,
            windmill_range: 300.0,
            chimney_range: 300.0,
            zone_margin: 1.4,
            folk_edge_inner: 0.75,
            folk_range_fade: 0.85,
        }
    }
}

impl FolkProps {
    fn from_json(v: &Value) -> Self {
        let d = Self::default();
        let f = |k: &str, dv: f32| v.get(k).and_then(Value::as_f64).map_or(dv, |x| x as f32);
        Self {
            folk_scale: f("folk_scale", d.folk_scale),
            folk_density: f("folk_density", d.folk_density),
            folk_range: f("folk_range", d.folk_range),
            folk_speed: f("folk_speed", d.folk_speed),
            folk_radius_factor: f("folk_radius_factor", d.folk_radius_factor),
            keep_from: f("folk_keep_from", d.keep_from),
            keep_exponent: f("folk_keep_exponent", d.keep_exponent),
            keep_min: f("folk_keep_min", d.keep_min),
            windmill_ratio: f("windmill_ratio", d.windmill_ratio),
            chimney_ratio: f("chimney_ratio", d.chimney_ratio),
            windmill_range: f("windmill_range", d.windmill_range),
            chimney_range: f("chimney_range", d.chimney_range),
            zone_margin: f("folk_zone_margin", d.zone_margin),
            folk_edge_inner: f("folk_edge_inner", d.folk_edge_inner),
            folk_range_fade: f("folk_range_fade", d.folk_range_fade),
        }
    }

    /// Share of instances kept at camera distance `d` (`keep_for`, ADR 0354).
    pub fn keep_for(&self, d: f32) -> f32 {
        ((self.keep_from / d.max(1e-3)).powf(self.keep_exponent)).clamp(self.keep_min, 1.0)
    }

    /// Fixed world scale of a figure model measured in metres (`FolkPool.world_scale`).
    pub fn world_scale(&self, meters_per_px: f32) -> f32 {
        self.folk_scale / meters_per_px
    }
}

/// `rules/map_scenes.json` (road folk, carts).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ScenesCfg {
    pub pool_cap: usize,
    pub road_folk_per_unit: f32,
    pub field_work_probability: f32,
    pub herd_probability: f32,
    pub carts_per_trade_value: f32,
    pub guard_value: f32,
    pub scene_figures_min: usize,
    pub scene_figures_max: usize,
}

impl Default for ScenesCfg {
    fn default() -> Self {
        Self {
            pool_cap: 2000,
            road_folk_per_unit: 0.05,
            field_work_probability: 0.18,
            herd_probability: 0.07,
            carts_per_trade_value: 0.01,
            guard_value: 40.0,
            scene_figures_min: 6,
            scene_figures_max: 24,
        }
    }
}

/// One bird species of `map_birds.json`.
#[derive(Debug, Clone)]
pub struct BirdSpecies {
    pub id: String,
    pub behavior: String,
    pub flocks: usize,
    pub per_flock: usize,
    pub scale: f32,
    pub color: [f32; 3],
    pub height: [f32; 2],
    pub radius: [f32; 2],
    pub speed: [f32; 2],
    pub jitter: f32,
    pub flap_rate: f32,
    pub flap_amount: f32,
    /// Season weights (spring, summer, autumn, winter).
    pub seasons: [f32; 4],
    pub surface: String,
    pub coast_max_px: Option<f32>,
    pub min_height_m: Option<f32>,
    pub max_height_m: Option<f32>,
    pub wetland_ids: Vec<String>,
    pub towns: Vec<String>,
    pub town_radius: f32,
    pub span: f32,
    pub v_spacing: f32,
    pub v_angle_deg: f32,
    /// Id of a generated glb of `dn_water_models.json` (`dn_model`); empty = wing-triangle V.
    pub dn_model: String,
}

#[derive(Debug, Clone, Default)]
pub struct BirdsCfg {
    pub flock_area: f32,
    pub retarget_seconds: f32,
    pub species: Vec<BirdSpecies>,
}

/// Gait of a fauna species (`render.gaits`).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Gait {
    pub cadence: f32,
    pub swing: f32,
    pub lift: f32,
    pub bounce: f32,
    pub graze: f32,
    pub speed_m_s: f32,
}

#[derive(Debug, Clone)]
pub struct FaunaSpecies {
    pub id: String,
    pub length_m: f32,
    pub placeholder: String,
    pub gait: String,
    pub herd_size: [usize; 2],
    pub wander_m: f32,
}

/// One zone entry: species density per 1000 km2 and optional season weights.
#[derive(Debug, Clone)]
pub struct FaunaDensity {
    pub species: String,
    pub per_1000km2: f32,
    pub seasons: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct FaunaZone {
    pub id: String,
    pub center_px: Vec2,
    pub radius_px: Vec2,
    pub species: Vec<FaunaDensity>,
}

#[derive(Debug, Clone, Default)]
pub struct FaunaCfg {
    pub max_distance: f32,
    pub fade_from: f32,
    pub cell_px: f32,
    pub max_visible: usize,
    pub herd_sigma_m: f32,
    pub town_clearance_px: f32,
    pub lod_distances: [f32; 2],
    pub lod_hysteresis: f32,
    pub gaits: HashMap<String, Gait>,
    pub species: HashMap<String, FaunaSpecies>,
    pub zones: Vec<FaunaZone>,
}

/// `fx/map_fire_wind.json` (`enabled`, `fire`) and the wind of `fx/map_atmosphere.json`.
#[derive(Debug, Clone)]
pub struct FireWind {
    pub enabled: bool,
    pub flame_size: [f32; 2],
    pub flicker_hz: f32,
    pub color_mid: [f32; 3],
    pub color_hot: [f32; 3],
    pub emission: f32,
    pub opacity: f32,
    pub lean: f32,
    pub max_lights: usize,
    pub light_color: [f32; 3],
    pub light_energy: f32,
    pub light_range: f32,
    pub light_height: f32,
    pub light_max_distance: f32,
    pub banner_amplitude: f32,
    pub banner_wave_speed: f32,
    /// Wind direction on the map plane (x, z), unit vector.
    pub wind_dir: Vec2,
}

impl Default for FireWind {
    fn default() -> Self {
        Self {
            enabled: false,
            flame_size: [3.0, 4.6],
            flicker_hz: 1.17,
            color_mid: [1.0, 0.32, 0.04],
            color_hot: [1.0, 0.72, 0.28],
            emission: 2.6,
            opacity: 0.85,
            lean: 0.22,
            max_lights: 4,
            light_color: [1.0, 0.52, 0.18],
            light_energy: 2.2,
            light_range: 9.0,
            light_height: 1.6,
            light_max_distance: 260.0,
            banner_amplitude: 0.16,
            banner_wave_speed: 4.6,
            wind_dir: Vec2::new(0.98, 0.2).normalize(),
        }
    }
}

/// A boat or ship model of `dn_water_models.json`.
#[derive(Debug, Clone)]
pub struct WaterModel {
    pub path: String,
    pub length: f32,
    pub yaw_deg: f32,
    pub from_year: i32,
    pub until_year: i32,
}

#[derive(Debug, Clone, Default)]
pub struct WaterModels {
    pub year: i32,
    pub lod_ambient: u8,
    pub lane_ship_scale: f32,
    pub river_boat_scale: f32,
    pub models: HashMap<String, WaterModel>,
    pub lane_default: Vec<String>,
    pub lane_by_basin: HashMap<String, Vec<String>>,
    pub river_default: Vec<String>,
    pub river_rules: Vec<(Vec<String>, Vec<String>)>,
}

impl WaterModels {
    fn in_service(&self, id: &str) -> bool {
        self.models
            .get(id)
            .is_some_and(|m| self.year >= m.from_year && self.year <= m.until_year)
    }

    /// Id taken in `ids` by `index` among the models in service (`DnWaterModels.pick`).
    pub fn pick<'a>(&self, ids: &'a [String], index: usize) -> Option<&'a str> {
        let usable: Vec<&String> = ids.iter().filter(|i| self.in_service(i)).collect();
        if usable.is_empty() {
            return None;
        }
        Some(usable[index % usable.len()].as_str())
    }

    pub fn lane_ids(&self, basin: &str) -> &[String] {
        self.lane_by_basin
            .get(basin)
            .map_or(self.lane_default.as_slice(), Vec::as_slice)
    }

    pub fn river_ids(&self, river: &str) -> &[String] {
        self.river_rules
            .iter()
            .find(|(rivers, _)| rivers.iter().any(|r| r == river))
            .map_or(self.river_default.as_slice(), |(_, ships)| ships.as_slice())
    }

    /// Asset path of the scene of a model at the ambient level of detail.
    pub fn scene_path(&self, id: &str) -> Option<String> {
        let model = self.models.get(id)?;
        Some(format!("models/{}_lod{}.glb", model.path, self.lod_ambient))
    }
}

/// A back-and-forth route of a boat (map px).
#[derive(Debug, Clone)]
pub struct Route {
    pub points: Vec<Vec2>,
    pub cumulative: Vec<f32>,
    pub length: f32,
    pub offset: f32,
    pub speed: f32,
    pub model: String,
    pub river: bool,
}

impl Route {
    pub fn new(points: Vec<Vec2>, offset: f32, speed: f32, model: String, river: bool) -> Self {
        let mut cumulative = vec![0.0f32];
        for i in 1..points.len() {
            let last = cumulative[i - 1];
            cumulative.push(last + points[i].distance(points[i - 1]));
        }
        let length = *cumulative.last().unwrap_or(&0.0);
        Self {
            points,
            cumulative,
            length,
            offset,
            speed,
            model,
            river,
        }
    }

    /// Position and heading at `time` (`LifeAmbient.route_pose`).
    pub fn pose(&self, time: f32) -> (Vec2, Vec2) {
        if self.length <= 0.0 || self.points.len() < 2 {
            return (self.points.first().copied().unwrap_or(Vec2::ZERO), Vec2::X);
        }
        let travelled = (self.offset + time * self.speed).rem_euclid(2.0 * self.length);
        let forward = travelled <= self.length;
        let s = if forward {
            travelled
        } else {
            2.0 * self.length - travelled
        };
        let i = self
            .cumulative
            .partition_point(|c| *c < s)
            .saturating_sub(1)
            .min(self.points.len() - 2);
        let seg = (self.cumulative[i + 1] - self.cumulative[i]).max(1e-4);
        let t = ((s - self.cumulative[i]) / seg).clamp(0.0, 1.0);
        let dir = (self.points[i + 1] - self.points[i]).normalize_or_zero();
        (
            self.points[i].lerp(self.points[i + 1], t),
            if forward { dir } else { -dir },
        )
    }
}

/// A resampled road piece (a straight run of at most [`ROAD_PIECE`] px).
#[derive(Debug, Clone, Copy)]
pub struct RoadPiece {
    pub a: Vec2,
    pub b: Vec2,
    pub main: bool,
}

pub const ROAD_PIECE: f32 = 6.0;

/// A whole road line with its cumulative lengths (carts follow it).
#[derive(Debug, Clone)]
pub struct RoadRun {
    pub main: bool,
    pub points: Vec<Vec2>,
    pub cumulative: Vec<f32>,
}

impl RoadRun {
    pub fn new(main: bool, points: Vec<Vec2>) -> Self {
        let mut cumulative = vec![0.0f32];
        for i in 1..points.len() {
            let last = cumulative[i - 1];
            cumulative.push(last + points[i].distance(points[i - 1]));
        }
        Self {
            main,
            points,
            cumulative,
        }
    }

    pub fn length(&self) -> f32 {
        *self.cumulative.last().unwrap_or(&0.0)
    }

    /// Position and unit direction at arc length `s` (clamped).
    pub fn at(&self, s: f32) -> (Vec2, Vec2) {
        if self.points.len() < 2 {
            return (self.points.first().copied().unwrap_or(Vec2::ZERO), Vec2::X);
        }
        let s = s.clamp(0.0, self.length());
        let i = self
            .cumulative
            .partition_point(|c| *c < s)
            .saturating_sub(1)
            .min(self.points.len() - 2);
        let seg = (self.cumulative[i + 1] - self.cumulative[i]).max(1e-4);
        let t = ((s - self.cumulative[i]) / seg).clamp(0.0, 1.0);
        (
            self.points[i].lerp(self.points[i + 1], t),
            (self.points[i + 1] - self.points[i]).normalize_or_zero(),
        )
    }
}

/// Everything the living map reads from `data/`.
#[derive(Resource, Clone)]
pub struct LifeData {
    pub map_size: Vec2,
    pub meters_per_px: f32,
    pub props: FolkProps,
    pub scenes: ScenesCfg,
    pub birds: BirdsCfg,
    pub fauna: FaunaCfg,
    pub fire: FireWind,
    pub water_models: WaterModels,
    pub river_routes: Vec<Route>,
    pub sea_lanes: Vec<(Vec<Vec2>, f32)>,
    pub roads: Vec<RoadPiece>,
    pub hamlets: Vec<(Vec2, String, String)>,
    pub towns: HashMap<String, Vec2>,
    pub road_runs: Vec<RoadRun>,
    pub wetlands: HashMap<String, (Vec2, f32)>,
    pub land: Option<LandMask>,
    pub lakes: Vec<Vec<Vec2>>,
    pub table: super::props::PropTable,
    /// Points of every river (flood scenes).
    pub rivers: Vec<Vec<Vec2>>,
    /// `scenes` of `rules/folk_scene_layouts.json`.
    pub scene_layouts: Value,
    /// `fx/sea_life.json` (the wake of the ships; the sea shader parameters belong to the water lot).
    pub sea_life: Value,
}

/// `land_mask.png`: 255 = land.
#[derive(Clone)]
pub struct LandMask {
    pub width: usize,
    pub height: usize,
    pub bytes: std::sync::Arc<Vec<u8>>,
}

impl LandMask {
    pub fn is_land(&self, px: Vec2) -> bool {
        if px.x < 0.0 || px.y < 0.0 {
            return false;
        }
        let (x, y) = (px.x as usize, px.y as usize);
        x < self.width && y < self.height && self.bytes[y * self.width + x] > 127
    }
}

fn read_json(dir: &Path, rel: &str) -> Option<Value> {
    let path = dir.join(rel);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            warn!("life: {}: {e}", path.display());
            return None;
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(v) => Some(v),
        Err(e) => {
            warn!("life: {}: {e}", path.display());
            None
        }
    }
}

fn arr2(v: &Value, dv: [f32; 2]) -> [f32; 2] {
    match v.as_array() {
        Some(a) if a.len() >= 2 => [
            a[0].as_f64().unwrap_or(dv[0] as f64) as f32,
            a[1].as_f64().unwrap_or(dv[1] as f64) as f32,
        ],
        _ => dv,
    }
}

fn arr3(v: &Value, dv: [f32; 3]) -> [f32; 3] {
    match v.as_array() {
        Some(a) if a.len() >= 3 => [
            a[0].as_f64().unwrap_or(0.0) as f32,
            a[1].as_f64().unwrap_or(0.0) as f32,
            a[2].as_f64().unwrap_or(0.0) as f32,
        ],
        _ => dv,
    }
}

fn f32_of(v: &Value, k: &str, dv: f32) -> f32 {
    v.get(k).and_then(Value::as_f64).map_or(dv, |x| x as f32)
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn season_weights(v: Option<&Value>) -> [f32; 4] {
    let mut w = [1.0; 4];
    if let Some(o) = v.and_then(Value::as_object) {
        for (i, name) in ["spring", "summer", "autumn", "winter"].iter().enumerate() {
            w[i] = o
                .get(*name)
                .and_then(Value::as_f64)
                .map_or(1.0, |x| x as f32);
        }
    }
    w
}

pub fn parse_birds(v: &Value) -> BirdsCfg {
    let mut cfg = BirdsCfg {
        flock_area: f32_of(v, "flock_area", 70.0),
        retarget_seconds: f32_of(v, "retarget_seconds", 1.5),
        species: Vec::new(),
    };
    for s in v
        .get("species")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let h = s.get("habitat").cloned().unwrap_or(Value::Null);
        cfg.species.push(BirdSpecies {
            id: s["id"].as_str().unwrap_or("").to_string(),
            behavior: s["behavior"].as_str().unwrap_or("orbit").to_string(),
            flocks: s["flocks"].as_u64().unwrap_or(1) as usize,
            per_flock: s["per_flock"].as_u64().unwrap_or(1) as usize,
            scale: f32_of(s, "scale", 0.5),
            color: arr3(&s["color"], [0.1, 0.1, 0.1]),
            height: arr2(&s["height"], [5.0, 10.0]),
            radius: arr2(&s["radius"], [4.0, 10.0]),
            speed: arr2(&s["speed"], [0.2, 0.3]),
            jitter: f32_of(s, "jitter", 0.0),
            flap_rate: f32_of(s, "flap_rate", 8.0),
            flap_amount: f32_of(s, "flap_amount", 0.5),
            seasons: season_weights(s.get("seasons")),
            surface: h["surface"].as_str().unwrap_or("land").to_string(),
            coast_max_px: h
                .get("coast_max_px")
                .and_then(Value::as_f64)
                .map(|x| x as f32),
            min_height_m: h
                .get("min_height_m")
                .and_then(Value::as_f64)
                .map(|x| x as f32),
            max_height_m: h
                .get("max_height_m")
                .and_then(Value::as_f64)
                .map(|x| x as f32),
            wetland_ids: strings(&h["wetland_ids"]),
            towns: strings(&h["towns"]),
            town_radius: f32_of(&h, "town_radius", 4.0),
            span: f32_of(&h, "span", 60.0),
            v_spacing: f32_of(&h, "v_spacing", 1.6),
            v_angle_deg: f32_of(&h, "v_angle_deg", 32.0),
            dn_model: s["dn_model"].as_str().unwrap_or("").to_string(),
        });
    }
    cfg
}

pub fn parse_fauna(
    v: &Value,
    to_px: &impl Fn(f64, f64) -> Option<Vec2>,
    meters_per_px: f32,
) -> FaunaCfg {
    let r = &v["render"];
    let mut cfg = FaunaCfg {
        max_distance: f32_of(r, "max_distance", 230.0),
        fade_from: f32_of(r, "fade_from", 150.0),
        cell_px: f32_of(r, "cell_px", 96.0),
        max_visible: r["max_visible_instances"].as_u64().unwrap_or(3000) as usize,
        herd_sigma_m: f32_of(r, "herd_sigma_m", 90.0),
        town_clearance_px: f32_of(r, "town_clearance_px", 2.5),
        lod_distances: arr2(&r["lod_distances"], [35.0, 80.0]),
        lod_hysteresis: f32_of(r, "lod_hysteresis", 0.06),
        ..Default::default()
    };
    if let Some(g) = r.get("gaits").and_then(Value::as_object) {
        for (k, gv) in g {
            if let Ok(gait) = serde_json::from_value::<Gait>(gv.clone()) {
                cfg.gaits.insert(k.clone(), gait);
            }
        }
    }
    if let Some(sp) = v.get("species").and_then(Value::as_object) {
        for (id, s) in sp {
            let hs = s["herd_size"].as_array();
            let hs = hs.map_or([3, 6], |a| {
                [
                    a.first().and_then(Value::as_u64).unwrap_or(3) as usize,
                    a.get(1).and_then(Value::as_u64).unwrap_or(6) as usize,
                ]
            });
            cfg.species.insert(
                id.clone(),
                FaunaSpecies {
                    id: id.clone(),
                    length_m: f32_of(s, "length_m", 2.0),
                    placeholder: s["placeholder"].as_str().unwrap_or("").to_string(),
                    gait: s["gait"].as_str().unwrap_or("walker").to_string(),
                    herd_size: hs,
                    wander_m: f32_of(s, "wander_m", 150.0),
                },
            );
        }
    }
    for z in v
        .get("zones")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let c = arr2(&z["center"], [0.0, 0.0]);
        let Some(center) = to_px(f64::from(c[0]), f64::from(c[1])) else {
            continue;
        };
        let rk = arr2(&z["radius_km"], [50.0, 50.0]);
        let km_per_px = meters_per_px / 1000.0;
        cfg.zones.push(FaunaZone {
            id: z["id"].as_str().unwrap_or("").to_string(),
            center_px: center,
            radius_px: Vec2::new(rk[0] / km_per_px, rk[1] / km_per_px),
            species: z["species"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|e| FaunaDensity {
                    species: e["species"].as_str().unwrap_or("").to_string(),
                    per_1000km2: f32_of(e, "per_1000km2", 0.0),
                    seasons: season_weights(e.get("seasons")),
                })
                .collect(),
        });
    }
    cfg
}

pub fn parse_fire(fire: Option<&Value>, atmosphere: Option<&Value>) -> FireWind {
    let mut out = FireWind::default();
    if let Some(v) = fire {
        out.enabled = v["enabled"].as_bool().unwrap_or(false);
        let f = &v["fire"];
        out.flame_size = arr2(&f["flame_size"], out.flame_size);
        out.flicker_hz = f32_of(f, "flicker_hz", out.flicker_hz);
        out.color_mid = arr3(&f["color_mid"], out.color_mid);
        out.color_hot = arr3(&f["color_hot"], out.color_hot);
        out.emission = f32_of(f, "emission", out.emission);
        out.opacity = f32_of(f, "opacity", out.opacity);
        out.lean = f32_of(f, "lean", out.lean);
        let l = &f["light"];
        out.max_lights = if l["enabled"].as_bool().unwrap_or(false) {
            l["max_lights"].as_u64().unwrap_or(4) as usize
        } else {
            0
        };
        out.light_color = arr3(&l["color"], out.light_color);
        out.light_energy = f32_of(l, "energy", out.light_energy);
        out.light_range = f32_of(l, "range", out.light_range);
        out.light_height = f32_of(l, "height", out.light_height);
        out.light_max_distance = f32_of(l, "max_camera_distance", out.light_max_distance);
        let b = &v["maquette_banner"];
        out.banner_amplitude = f32_of(b, "amplitude", out.banner_amplitude);
        out.banner_wave_speed = f32_of(b, "wave_speed", out.banner_wave_speed);
    }
    if let Some(w) = atmosphere.and_then(|a| a.get("wind")) {
        // `wind: [x, z]` (Godot `weather_wind`); an object with `direction` is also accepted.
        let d = if w.is_array() {
            arr2(w, [0.98, 0.2])
        } else {
            arr2(&w["direction"], [0.98, 0.2])
        };
        let dir = Vec2::new(d[0], d[1]);
        if dir.length() > 1e-3 {
            out.wind_dir = dir.normalize();
        }
    }
    out
}

pub fn parse_water_models(v: &Value) -> WaterModels {
    let d = &v["defaults"];
    let mut out = WaterModels {
        year: d["year"].as_i64().unwrap_or(1337) as i32,
        lod_ambient: d["lod_ambient"].as_u64().unwrap_or(2) as u8,
        lane_ship_scale: f32_of(d, "lane_ship_scale", 1.0),
        river_boat_scale: f32_of(d, "river_boat_scale", 1.0),
        ..Default::default()
    };
    if let Some(m) = v.get("models").and_then(Value::as_object) {
        for (id, e) in m {
            out.models.insert(
                id.clone(),
                WaterModel {
                    path: e["path"].as_str().unwrap_or("").to_string(),
                    length: f32_of(e, "length", 4.0),
                    yaw_deg: f32_of(e, "yaw_deg", 0.0),
                    from_year: e["from_year"].as_i64().unwrap_or(-9999) as i32,
                    until_year: e["until_year"].as_i64().unwrap_or(9999) as i32,
                },
            );
        }
    }
    let lanes = &v["sea_lanes"];
    out.lane_default = strings(&lanes["default"]);
    if let Some(b) = lanes.get("by_basin").and_then(Value::as_object) {
        for (k, ids) in b {
            out.lane_by_basin.insert(k.clone(), strings(ids));
        }
    }
    let rivers = &v["river_boats"];
    out.river_default = strings(&rivers["default"]);
    for rule in rivers
        .get("rules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        out.river_rules
            .push((strings(&rule["rivers"]), strings(&rule["ships"])));
    }
    out
}

/// Cuts a polyline into pieces of at most `max` px.
pub fn resample(points: &[Vec2], max: f32) -> Vec<(Vec2, Vec2)> {
    let mut out = Vec::new();
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let n = ((a.distance(b) / max).ceil() as usize).max(1);
        for i in 0..n {
            out.push((
                a.lerp(b, i as f32 / n as f32),
                a.lerp(b, (i + 1) as f32 / n as f32),
            ));
        }
    }
    out
}

fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let mut j = poly.len().saturating_sub(1);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn load_land(map_dir: &Path) -> Option<LandMask> {
    let file = std::fs::File::open(map_dir.join("land_mask.png")).ok()?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Grayscale || info.bit_depth != png::BitDepth::Eight {
        warn!("life: land_mask.png is not 8-bit grey");
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(LandMask {
        width: info.width as usize,
        height: info.height as usize,
        bytes: std::sync::Arc::new(buf),
    })
}

impl LifeData {
    pub fn load(root: &Path) -> LifeData {
        let data_dir = crate::paths::data_dir(root);
        let map_dir = data_dir.join("map");
        let water = WaterData::load(&map_dir);
        let meters_per_px = water.meters_per_px.max(1.0);
        let projection = MapProjection::load(&map_dir).ok();
        let to_px = |lon: f64, lat: f64| projection.as_ref().and_then(|p| p.lonlat_to_px(lon, lat));

        let props = read_json(&data_dir, "art/town_maquettes.json")
            .map(|v| FolkProps::from_json(&v["props"]))
            .unwrap_or_default();
        let scenes = read_json(&data_dir, "rules/map_scenes.json")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        let birds = read_json(&data_dir, "map/map_birds.json")
            .map(|v| parse_birds(&v))
            .unwrap_or_default();
        let fauna = read_json(&data_dir, "map/map_fauna.json")
            .map(|v| parse_fauna(&v, &to_px, meters_per_px))
            .unwrap_or_default();
        let fire = parse_fire(
            read_json(&data_dir, "fx/map_fire_wind.json").as_ref(),
            read_json(&data_dir, "fx/map_atmosphere.json").as_ref(),
        );
        let water_models = read_json(&data_dir, "art/dn_water_models.json")
            .map(|v| parse_water_models(&v))
            .unwrap_or_default();
        let land = load_land(&map_dir);

        // Roads, resampled into pieces.
        let mut roads = Vec::new();
        for road in &water.roads {
            for (a, b) in resample(&road.points, ROAD_PIECE) {
                roads.push(RoadPiece {
                    a,
                    b,
                    main: road.main,
                });
            }
        }
        let road_runs: Vec<RoadRun> = water
            .roads
            .iter()
            .filter(|r| r.points.len() >= 2)
            .map(|r| RoadRun::new(r.main, r.points.clone()))
            .collect();
        let towns: HashMap<String, Vec2> = read_json(&map_dir, "settlements_px.json")
            .and_then(|v| {
                v.as_object().map(|o| {
                    o.iter()
                        .filter_map(|(k, p)| {
                            let a = p.as_array()?;
                            Some((
                                k.clone(),
                                Vec2::new(a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32),
                            ))
                        })
                        .collect()
                })
            })
            .unwrap_or_default();
        let hamlets = read_json(&data_dir, "map/hamlets.json")
            .and_then(|v| {
                v.as_array().map(|a| {
                    a.iter()
                        .filter_map(|h| {
                            let p = arr2(&h["px"], [f32::NAN, 0.0]);
                            (!p[0].is_nan()).then(|| {
                                (
                                    Vec2::new(p[0], p[1]),
                                    h["name"].as_str().unwrap_or("").to_string(),
                                    h["province"].as_str().unwrap_or("").to_string(),
                                )
                            })
                        })
                        .collect()
                })
            })
            .unwrap_or_default();
        let wetlands = read_json(&map_dir, "wetland_sites_px.json")
            .and_then(|v| {
                v["sites"].as_object().map(|o| {
                    o.iter()
                        .map(|(k, s)| {
                            let p = arr2(&s["px"], [0.0, 0.0]);
                            (
                                k.clone(),
                                (Vec2::new(p[0], p[1]), f32_of(s, "radius_px", 10.0)),
                            )
                        })
                        .collect()
                })
            })
            .unwrap_or_default();
        let sea_lanes = read_json(&map_dir, "sea_lanes_px.json")
            .and_then(|v| {
                v["lanes"].as_array().map(|a| {
                    a.iter()
                        .map(|l| {
                            let pts: Vec<Vec2> = l["points"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .map(|p| {
                                    Vec2::new(
                                        p[0].as_f64().unwrap_or(0.0) as f32,
                                        p[1].as_f64().unwrap_or(0.0) as f32,
                                    )
                                })
                                .collect();
                            (pts, f32_of(l, "length_km", 0.0))
                        })
                        .collect()
                })
            })
            .unwrap_or_default();
        let mut data = LifeData {
            map_size: water.map_size,
            meters_per_px,
            props,
            scenes,
            birds,
            fauna,
            fire,
            water_models,
            river_routes: Vec::new(),
            sea_lanes,
            roads,
            hamlets,
            towns,
            road_runs,
            wetlands,
            land,
            lakes: water.lakes.iter().map(|l| l.polygon.clone()).collect(),
            table: super::props::PropTable::load(&data_dir),
            rivers: water.rivers.iter().map(|r| r.points.clone()).collect(),
            scene_layouts: read_json(&data_dir, "rules/folk_scene_layouts.json")
                .map_or(Value::Null, |v| v["scenes"].clone()),
            sea_life: read_json(&data_dir, "fx/sea_life.json").unwrap_or(Value::Null),
        };
        data.build_routes(&water);
        data
    }

    pub fn is_water_basin_point(&self, p: Vec2) -> bool {
        self.land.as_ref().is_some_and(|l| !l.is_land(p))
    }

    pub fn in_lake(&self, p: Vec2) -> bool {
        self.lakes.iter().any(|l| point_in_polygon(p, l))
    }

    /// River boat routes (`LifeAmbient._build_river_routes`) and sea ships on the lanes of
    /// `sea_lanes_px.json` (instead of straight lines between neighbouring ports: the lanes are
    /// already routed over water).
    fn build_routes(&mut self, water: &WaterData) {
        const RIVER_MIN_IMPORTANCE: i64 = 5;
        const RIVER_PX_PER_BOAT: f32 = 260.0;
        const MAX_RIVER_BOATS: usize = 40;
        const RIVER_SPEED: f32 = 0.9;
        const SEA_SPEED: f32 = 2.2;
        let mut routes: Vec<Route> = Vec::new();
        'rivers: for river in &water.rivers {
            if river.importance < RIVER_MIN_IMPORTANCE || river.points.len() < 2 {
                continue;
            }
            let probe = Route::new(river.points.clone(), 0.0, RIVER_SPEED, String::new(), true);
            let boats = (probe.length / RIVER_PX_PER_BOAT) as usize;
            let ids = self.water_models.river_ids(&river.name).to_vec();
            let seed = text_seed(&river.name);
            for k in 0..boats {
                if routes.len() >= MAX_RIVER_BOATS {
                    break 'rivers;
                }
                let Some(model) = self.water_models.pick(&ids, routes.len() + k) else {
                    continue;
                };
                let offset = h01(seed, k as u64, 1) * 2.0 * probe.length;
                let speed = RIVER_SPEED * (0.7 + 0.5 * h01(seed, k as u64, 2));
                routes.push(Route::new(
                    river.points.clone(),
                    offset,
                    speed,
                    model.to_string(),
                    true,
                ));
            }
        }
        self.river_routes = routes;
        let mut lanes = Vec::new();
        for (n, (points, _km)) in self.sea_lanes.iter().enumerate() {
            if points.len() < 2 {
                continue;
            }
            let mid = points[points.len() / 2];
            let basin = water
                .basins
                .basins
                .iter()
                .find(|b| {
                    let poly: Vec<Vec2> =
                        b.polygon_px.iter().map(|p| Vec2::new(p[0], p[1])).collect();
                    point_in_polygon(mid, &poly)
                })
                .map_or("", |b| b.id.as_str());
            let ids = self.water_models.lane_ids(basin).to_vec();
            let Some(model) = self.water_models.pick(&ids, n) else {
                continue;
            };
            let probe = Route::new(points.clone(), 0.0, SEA_SPEED, String::new(), false);
            let offset = h01(7, n as u64, 3) * 2.0 * probe.length;
            let speed = SEA_SPEED * (0.8 + 0.4 * h01(7, n as u64, 4));
            lanes.push(Route::new(
                points.clone(),
                offset,
                speed,
                model.to_string(),
                false,
            ));
        }
        self.river_routes.extend(lanes);
    }
}

#[cfg(test)]
impl LifeData {
    /// Minimal data for unit tests of the placement code.
    pub fn for_tests(fauna: FaunaCfg) -> LifeData {
        LifeData {
            map_size: Vec2::new(1000.0, 1000.0),
            meters_per_px: 719.0,
            props: FolkProps::default(),
            scenes: ScenesCfg::default(),
            birds: BirdsCfg::default(),
            fauna,
            fire: FireWind::default(),
            water_models: WaterModels::default(),
            river_routes: Vec::new(),
            sea_lanes: Vec::new(),
            roads: Vec::new(),
            hamlets: Vec::new(),
            towns: HashMap::new(),
            road_runs: Vec::new(),
            wetlands: HashMap::new(),
            land: None,
            lakes: Vec::new(),
            table: super::props::PropTable::default(),
            rivers: Vec::new(),
            scene_layouts: Value::Null,
            sea_life: Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_in_range() {
        for i in 0..1000 {
            let h = h01(i, 3, 7);
            assert!((0.0..1.0).contains(&h));
        }
        assert_eq!(h01(1, 2, 3), h01(1, 2, 3));
        assert_ne!(h01(1, 2, 3), h01(1, 2, 4));
    }

    #[test]
    fn keep_is_continuous_and_bounded() {
        let p = FolkProps::default();
        assert_eq!(p.keep_for(10.0), 1.0);
        assert!((p.keep_for(100.0) - 0.25).abs() < 1e-5);
        assert_eq!(p.keep_for(10_000.0), p.keep_min);
    }

    #[test]
    fn route_pose_ping_pongs() {
        let r = Route::new(
            vec![Vec2::ZERO, Vec2::new(10.0, 0.0)],
            0.0,
            1.0,
            "x".into(),
            true,
        );
        let (p, d) = r.pose(5.0);
        assert!((p.x - 5.0).abs() < 1e-4 && d.x > 0.0);
        let (p, d) = r.pose(15.0);
        assert!((p.x - 5.0).abs() < 1e-4 && d.x < 0.0);
    }

    #[test]
    fn resample_pieces_are_short() {
        let pieces = resample(&[Vec2::ZERO, Vec2::new(20.0, 0.0)], 6.0);
        assert_eq!(pieces.len(), 4);
        assert!(pieces.iter().all(|(a, b)| a.distance(*b) <= 6.0 + 1e-4));
    }
}
