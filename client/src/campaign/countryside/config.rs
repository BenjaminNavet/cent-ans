//! `data/map/map_countryside.json` (schema `map_countryside.schema.json`): props, regions and
//! placement rules of the countryside layer. No placement rule lives in code: a rule is a mode
//! (`scatter`, `paddock`, `village`, `road`), weighted props, regions, a density and site filters.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use serde_json::Value;

use crate::campaign::provinces::projection::MapProjection;

/// Season names in the order of the season vectors (spring, summer, autumn, winter).
pub const SEASONS: [&str; 4] = ["spring", "summer", "autumn", "winter"];

#[derive(Debug, Clone)]
pub struct Render {
    pub cell_px: f32,
    pub load_factor: f32,
    pub spawn_seconds: f32,
    pub load_hysteresis: f32,
    pub lod_hysteresis: f32,
    pub min_radius_px: f32,
    pub max_radius_px: f32,
    pub max_cached_cells: usize,
    pub cells_per_frame: usize,
    /// Roots (props, outcrops) spawned per frame, all spawners together (`budget`).
    pub spawn_roots_per_frame: usize,
    pub size_k: f32,
    pub size_exponent: f32,
    pub spread_exponent: f32,
    pub lod_distances: [f32; 2],
    pub town_clearance_px: f32,
    pub max_instances_per_prop: usize,
    pub max_distance: f32,
    pub fade_from: f32,
}

/// Site filters with the defaults of `render.defaults` already applied.
#[derive(Debug, Clone)]
pub struct Filters {
    pub alt_min_m: f32,
    pub alt_max_m: f32,
    pub slope_max_deg: f32,
    pub marsh_max: f32,
    pub prairie_min: f32,
    pub open_min: f32,
    pub forest_max: f32,
    pub crops_max: f32,
    pub wet_ok: bool,
    /// Bit `b` = biome class `b` allowed; 0 = no biome filter.
    pub biome_mask: u32,
    pub coast_px: i32,
    pub landscapes: Vec<String>,
    pub exclude_landscapes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Prop {
    pub id: String,
    /// Path under `models/dn/` without the `_lod<n>.glb` suffix (`props/well_stone`).
    pub model: String,
    pub max_mult: f32,
    pub size_scale: f32,
    pub fade_from: f32,
    pub max_distance: f32,
    pub stretched: bool,
    pub fixed_size: bool,
    pub max_instances: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Ellipse {
    pub center: Vec2,
    pub radius: Vec2,
}

impl Ellipse {
    pub fn contains(&self, p: Vec2) -> bool {
        ((p - self.center) / self.radius).length_squared() <= 1.0
    }

    pub fn bounds(&self) -> (Vec2, Vec2) {
        (self.center - self.radius, self.center + self.radius)
    }
}

#[derive(Debug, Clone)]
pub enum Mode {
    Scatter,
    Paddock {
        size_m: [f32; 2],
        segment_m: f32,
        gap: f32,
    },
    Village {
        kinds: Vec<String>,
        chance: f32,
        offset_m: [f32; 2],
    },
    Road {
        types: Vec<String>,
        per_100km: f32,
        offset_m: [f32; 2],
        parallel: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub mode: Mode,
    /// Prop index and weight.
    pub props: Vec<(usize, f32)>,
    pub total_weight: f32,
    pub regions: Vec<Ellipse>,
    pub is_global: bool,
    pub density_per_1000km2: f32,
    pub filters: Filters,
    pub seasons: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct Config {
    pub render: Render,
    pub props: Vec<Prop>,
    pub rules: Vec<Rule>,
    pub meters_per_px: f32,
    /// `countryside_size`, `countryside_range`, `folk_scale` of `town_maquettes.json` (maquette
    /// style, the default): size and range multipliers, fixed multiplier of the `fixed_size` props.
    pub maquette_size: f32,
    pub maquette_range: f32,
    pub fixed_mult: f32,
}

fn num(v: &Value, key: &str, default: f32) -> f32 {
    v.get(key)
        .and_then(Value::as_f64)
        .map_or(default, |x| x as f32)
}

fn pair(v: &Value, key: &str, default: [f32; 2]) -> [f32; 2] {
    match v.get(key).and_then(Value::as_array) {
        Some(a) if a.len() >= 2 => [
            a[0].as_f64().unwrap_or(default[0] as f64) as f32,
            a[1].as_f64().unwrap_or(default[1] as f64) as f32,
        ],
        _ => default,
    }
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_render(r: &Value) -> Render {
    let l = pair(r, "lod_distances", [25.0, 60.0]);
    Render {
        cell_px: num(r, "cell_px", 96.0),
        load_factor: num(r, "load_factor", 2.4),
        spawn_seconds: num(r, "spawn_seconds", 0.0),
        load_hysteresis: num(r, "load_hysteresis", 0.0),
        lod_hysteresis: num(r, "lod_hysteresis", 0.0),
        min_radius_px: num(r, "min_radius_px", 24.0),
        max_radius_px: num(r, "max_radius_px", 200.0),
        max_cached_cells: num(r, "max_cached_cells", 64.0) as usize,
        cells_per_frame: num(r, "cells_per_frame", 1.0) as usize,
        spawn_roots_per_frame: num(r, "spawn_roots_per_frame", 150.0) as usize,
        size_k: num(r, "size_k", 0.0045),
        size_exponent: num(r, "size_exponent", 0.85),
        spread_exponent: num(r, "spread_exponent", 0.5),
        lod_distances: l,
        town_clearance_px: num(r, "town_clearance_px", 0.8),
        max_instances_per_prop: num(r, "max_instances_per_prop", 700.0) as usize,
        max_distance: num(r, "max_distance", 110.0),
        fade_from: num(r, "fade_from", 70.0),
    }
}

fn parse_filters(rule: &Value, defaults: &Value, biome_index: &HashMap<String, u32>) -> Filters {
    let f = rule.get("filters").unwrap_or(&Value::Null);
    let get = |k: &str, d: f32| num(f, k, num(defaults, k, d));
    let mut mask = 0u32;
    for name in strings(f, "biomes") {
        if let Some(bit) = biome_index.get(&name) {
            mask |= 1 << bit;
        }
    }
    Filters {
        alt_min_m: get("alt_min_m", -50.0),
        alt_max_m: get("alt_max_m", 4000.0),
        slope_max_deg: get("slope_max_deg", 90.0),
        marsh_max: get("marsh_max", 0.5),
        prairie_min: get("prairie_min", 0.0),
        open_min: get("open_min", 0.0),
        forest_max: get("forest_max", 1.0),
        crops_max: get("crops_max", 1.0),
        wet_ok: f.get("wet_ok").and_then(Value::as_bool).unwrap_or(false),
        biome_mask: mask,
        coast_px: num(f, "coast_px", 0.0) as i32,
        landscapes: strings(f, "landscapes"),
        exclude_landscapes: strings(f, "exclude_landscapes"),
    }
}

fn parse_mode(rule: &Value) -> Mode {
    let mode = rule["mode"].as_str().unwrap_or("scatter");
    match mode {
        "paddock" => {
            let p = &rule["paddock"];
            Mode::Paddock {
                size_m: pair(p, "size_m", [90.0, 220.0]),
                segment_m: num(p, "segment_m", 14.0),
                gap: num(p, "gap", 0.1),
            }
        }
        "village" => {
            let p = &rule["village"];
            let mut kinds = strings(p, "kinds");
            if kinds.is_empty() {
                kinds.push("village".into());
            }
            Mode::Village {
                kinds,
                chance: num(p, "chance", 0.4),
                offset_m: pair(p, "offset_m", [200.0, 380.0]),
            }
        }
        "road" => {
            let p = &rule["road"];
            Mode::Road {
                types: strings(p, "types"),
                per_100km: num(p, "per_100km", 1.0),
                offset_m: pair(p, "offset_m", [6.0, 14.0]),
                parallel: p.get("parallel").and_then(Value::as_bool).unwrap_or(true),
            }
        }
        _ => Mode::Scatter,
    }
}

/// Ellipses of a rule: its named regions, then the sites of `regions_from`.
fn resolve_regions(
    rule: &Value,
    regions: &Value,
    data_dir: &Path,
    projection: &MapProjection,
    mpp: f32,
) -> Vec<Ellipse> {
    let ellipse = |center: &Value, radius_km: [f32; 2]| -> Option<Ellipse> {
        let lon = center.get(0)?.as_f64()?;
        let lat = center.get(1)?.as_f64()?;
        let c = projection.lonlat_to_px(lon, lat)?;
        Some(Ellipse {
            center: c,
            radius: Vec2::new(radius_km[0], radius_km[1]) * 1000.0 / mpp,
        })
    };
    let mut out = Vec::new();
    for id in strings(rule, "regions") {
        if let Some(r) = regions.get(&id)
            && let Some(e) = ellipse(&r["center"], pair(r, "radius_km", [0.0, 0.0]))
        {
            out.push(e);
        }
    }
    if let Some(from) = rule.get("regions_from")
        && let Some(file) = from["file"].as_str()
        && let Ok(text) = std::fs::read_to_string(data_dir.join(file))
        && let Ok(source) = serde_json::from_str::<Value>(&text)
    {
        let ids = strings(from, "ids");
        let scale = num(from, "scale", 1.0);
        for site in source[from["key"].as_str().unwrap_or("")]
            .as_array()
            .into_iter()
            .flatten()
        {
            let id = site["id"].as_str().unwrap_or("");
            if ids.is_empty() || ids.iter().any(|i| i == id) {
                let size = pair(site, "size_km", [0.0, 0.0]);
                if let Some(e) = ellipse(
                    &site["center"],
                    [size[0] * 0.5 * scale, size[1] * 0.5 * scale],
                ) {
                    out.push(e);
                }
            }
        }
    }
    out
}

/// Parses the document. `data_dir` is `<root>/data`.
pub fn parse(doc: &Value, data_dir: &Path, projection: &MapProjection) -> Config {
    let mpp = projection.meters_per_px() as f32;
    let render = parse_render(&doc["render"]);
    let defaults = &doc["render"]["defaults"];
    let biome_index: HashMap<String, u32> = doc["biome_index"]
        .as_object()
        .map(|o| {
            o.iter()
                .filter_map(|(k, v)| v.as_u64().map(|b| (k.clone(), b as u32)))
                .collect()
        })
        .unwrap_or_default();
    let mut props = Vec::new();
    let mut prop_index: HashMap<String, usize> = HashMap::new();
    if let Some(map) = doc["props"].as_object() {
        for (id, p) in map {
            prop_index.insert(id.clone(), props.len());
            props.push(Prop {
                id: id.clone(),
                model: p["model"].as_str().unwrap_or("").to_string(),
                max_mult: num(p, "max_mult", 30.0),
                size_scale: num(p, "size_scale", 1.0),
                fade_from: num(p, "fade_from", render.fade_from),
                max_distance: num(p, "max_distance", render.max_distance),
                stretched: p.get("stretched").and_then(Value::as_bool).unwrap_or(false),
                fixed_size: p
                    .get("fixed_size")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                max_instances: num(p, "max_instances", render.max_instances_per_prop as f32)
                    as usize,
            });
        }
    }
    let mut rules = Vec::new();
    for rule in doc["rules"].as_array().into_iter().flatten() {
        let regions = resolve_regions(rule, &doc["regions"], data_dir, projection, mpp);
        let is_global = rule["regions"].as_array().is_none_or(Vec::is_empty)
            && rule.get("regions_from").is_none_or(Value::is_null);
        if regions.is_empty() && !is_global {
            continue;
        }
        let mut rule_props = Vec::new();
        let mut total = 0.0;
        if let Some(map) = rule["props"].as_object() {
            for (id, w) in map {
                if let Some(&i) = prop_index.get(id) {
                    let w = w.as_f64().unwrap_or(0.0) as f32;
                    rule_props.push((i, w));
                    total += w;
                }
            }
        }
        if rule_props.is_empty() || total <= 0.0 {
            continue;
        }
        let mut seasons = [1.0; 4];
        for (n, name) in SEASONS.iter().enumerate() {
            if let Some(v) = rule["seasons"].get(*name).and_then(Value::as_f64) {
                seasons[n] = v as f32;
            }
        }
        rules.push(Rule {
            id: rule["id"].as_str().unwrap_or("").to_string(),
            mode: parse_mode(rule),
            props: rule_props,
            total_weight: total,
            regions,
            is_global,
            density_per_1000km2: num(rule, "density_per_1000km2", 0.0),
            filters: parse_filters(rule, defaults, &biome_index),
            seasons,
        });
    }
    Config {
        render,
        props,
        rules,
        meters_per_px: mpp,
        maquette_size: 1.0,
        maquette_range: 1.0,
        fixed_mult: 0.0,
    }
}

impl Config {
    /// Applies the maquette multipliers from the `props` block of `art/town_maquettes.json`.
    pub fn with_maquette(mut self, props: &Value) -> Config {
        self.maquette_size = num(props, "countryside_size", 1.0);
        self.maquette_range = num(props, "countryside_range", 1.0);
        self.fixed_mult = num(props, "folk_scale", 0.0);
        self
    }

    /// Camera distance beyond which nothing is drawn.
    pub fn outer_distance(&self) -> f32 {
        self.props
            .iter()
            .map(|p| p.max_distance)
            .fold(0.0, f32::max)
            * self.maquette_range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_document_parses() {
        let root = crate::paths::find_root().expect("root");
        let text = std::fs::read_to_string(root.join("data/map/map_countryside.json")).unwrap();
        let doc: Value = serde_json::from_str(&text).unwrap();
        let projection = MapProjection::load(&root.join("data/map")).unwrap();
        let cfg = parse(&doc, &root.join("data"), &projection);
        assert_eq!(cfg.props.len(), 43);
        assert!(cfg.rules.len() >= 25, "{} rules", cfg.rules.len());
        assert!(
            cfg.rules
                .iter()
                .any(|r| matches!(r.mode, Mode::Paddock { .. }) && r.filters.biome_mask != 0)
        );
        assert!(cfg.rules.iter().all(|r| r.total_weight > 0.0));
        assert_eq!(cfg.render.cell_px, 96.0);
        assert!((cfg.meters_per_px - 719.0).abs() < 50.0);
    }
}
