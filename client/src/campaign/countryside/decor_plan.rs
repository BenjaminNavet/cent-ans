//! Placement of the one-off decor outside the towns (port of `decor_planner.gd`): wayside
//! crosses, gibbets, tolls, mines, forges, megaliths, ruins, pilgrim stops, steppe camps...
//! Data in `data/map/map_landmarks_extra.json`: `rules` place instances deterministically (seed
//! of the file x seed of the rule), `sites` set real historical places. Rule modes: `road` (along
//! the roads of the settlement graph), `gate` (at the town gates), `crossing` (at the bridges),
//! `coast`, `province` (per province from resources, terrain, climate, area), `route` (along the
//! shortest road path between settlements). Pure planning: the map is read through [`DecorMap`].
//!
//! The random stream is the splitmix64 of `placement::Rng`, not Godot's PCG.

use std::collections::{HashMap, HashSet};
use std::f32::consts::TAU;

use bevy::math::{IVec2, Vec2};
use serde_json::Value;

use super::placement::Rng;

const SETTLEMENT_CELL: f32 = 8.0;
const DEDUP_CELL: f32 = 1.0;
const SNAP_RADIUS: i32 = 7;
const SAMPLE_TRIES: usize = 40;

#[derive(Debug, Clone)]
pub struct DecorType {
    pub id: String,
    pub label: String,
    /// Id of the DN catalogue model (`data/art/dn_manifest.json`).
    pub model: String,
    pub size_m: [f32; 2],
    pub fraction: f32,
    pub max_distance: f32,
    pub smoke: bool,
    pub known: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Period {
    pub from_year: Option<i32>,
    pub to_year: Option<i32>,
    pub seasons: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub mode: String,
    pub types: Vec<(String, f32)>,
    pub seed: i64,
    pub spacing_px: f32,
    pub chance: f32,
    pub lateral_px: [f32; 2],
    pub terrains: Vec<String>,
    pub provinces: Vec<String>,
    pub climates: Vec<String>,
    pub resources_any: Vec<String>,
    pub resources_all: Vec<String>,
    pub provinces_resource: Vec<String>,
    pub min_height_m: f32,
    pub max_height_m: f32,
    pub min_settlement_px: f32,
    pub settlement_px: Option<[f32; 2]>,
    pub settlement_kinds: Vec<String>,
    pub allow_river: bool,
    pub near_river_px: Option<f32>,
    pub crossing_types: Vec<String>,
    pub offset_px: f32,
    pub inland_px: f32,
    pub kinds: Vec<String>,
    pub distance_px: [f32; 2],
    pub nodes: Vec<String>,
    pub per_province: f32,
    pub per_area_px: f32,
    pub period: Period,
}

#[derive(Debug, Clone)]
pub struct Site {
    pub id: String,
    pub kind: String,
    pub px: Option<Vec2>,
    pub near: Option<String>,
    pub offset_px: Vec2,
    pub yaw_deg: Option<f32>,
    pub period: Period,
}

#[derive(Debug, Clone)]
pub struct RenderCfg {
    pub view_range_units: f32,
    pub fade_from_units: f32,
    pub load_factor: f32,
    pub load_min_units: f32,
    pub load_max_units: f32,
    pub max_instances: usize,
    pub lod_distances: [f32; 2],
    pub shadow_range_units: f32,
    pub cell_px: f32,
    pub real_below: f32,
    pub full_from: f32,
    pub max_factor: f32,
    pub fov_deg: f32,
    /// Maquette overrides (the default style).
    pub size_distance: f32,
    pub distance_scale: f32,
}

#[derive(Debug, Clone)]
pub struct DecorConfig {
    pub seed: i64,
    pub render: RenderCfg,
    pub types: Vec<DecorType>,
    pub type_index: HashMap<String, usize>,
    pub rules: Vec<Rule>,
    pub sites: Vec<Site>,
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

fn period(v: &Value) -> Period {
    Period {
        from_year: v.get("from_year").and_then(Value::as_i64).map(|x| x as i32),
        to_year: v.get("to_year").and_then(Value::as_i64).map(|x| x as i32),
        seasons: v.get("seasons").map(|_| strings(v, "seasons")),
    }
}

/// Parses `map_landmarks_extra.json` (maquette style render values applied: the default).
pub fn parse(doc: &Value, maquette: bool) -> DecorConfig {
    let r = &doc["render"];
    let m = &r["maquette"];
    let pick = |key: &str, default: f32| {
        if maquette && m.get(key).is_some() {
            num(m, key, default)
        } else {
            num(r, key, default)
        }
    };
    let screen = &r["screen"];
    let lod = pair(r, "lod_distances", [14.0, 38.0]);
    let view = pick("view_range_units", 70.0);
    let render = RenderCfg {
        view_range_units: view,
        fade_from_units: pick("fade_from_units", view),
        load_factor: pick("load_factor", 2.6),
        load_min_units: pick("load_min_units", 8.0),
        load_max_units: pick("load_max_units", 140.0),
        max_instances: pick("max_instances", 5000.0) as usize,
        lod_distances: lod,
        shadow_range_units: pick("shadow_range_units", 16.0),
        cell_px: pick("cell_px", 24.0),
        real_below: num(screen, "real_below", 8.0),
        full_from: num(screen, "full_from", 14.0),
        max_factor: num(screen, "max_factor", 200.0),
        fov_deg: num(screen, "fov_deg", 55.0),
        size_distance: if maquette {
            num(m, "size_distance", 0.0)
        } else {
            0.0
        },
        distance_scale: if maquette {
            num(m, "distance_scale", 1.0)
        } else {
            1.0
        },
    };
    let mut types = Vec::new();
    let mut type_index = HashMap::new();
    if let Some(map) = doc["types"].as_object() {
        for (id, t) in map {
            type_index.insert(id.clone(), types.len());
            types.push(DecorType {
                id: id.clone(),
                label: t["label"].as_str().unwrap_or(id).to_string(),
                model: t["model"].as_str().unwrap_or("").to_string(),
                size_m: pair(t, "size_m", [1.0, 1.0]),
                fraction: num(t, "fraction", 0.005),
                max_distance: num(t, "max_distance", 30.0),
                smoke: t["smoke"].as_bool().unwrap_or(false),
                known: t["known"].as_bool().unwrap_or(false),
            });
        }
    }
    let rules = doc["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| Rule {
            id: r["id"].as_str().unwrap_or("").to_string(),
            mode: r["mode"].as_str().unwrap_or("").to_string(),
            types: r["types"]
                .as_object()
                .map(|o| {
                    o.iter()
                        .map(|(k, w)| (k.clone(), w.as_f64().unwrap_or(0.0) as f32))
                        .collect()
                })
                .unwrap_or_default(),
            seed: r["seed"].as_i64().unwrap_or(0),
            spacing_px: num(r, "spacing_px", 12.0),
            chance: num(r, "chance", 1.0),
            lateral_px: pair(r, "lateral_px", [0.0, 0.0]),
            terrains: strings(r, "terrains"),
            provinces: strings(r, "provinces"),
            climates: strings(r, "climates"),
            resources_any: strings(r, "resources_any"),
            resources_all: strings(r, "resources_all"),
            provinces_resource: strings(r, "provinces_resource"),
            min_height_m: num(r, "min_height_m", f32::NEG_INFINITY),
            max_height_m: num(r, "max_height_m", f32::INFINITY),
            min_settlement_px: num(r, "min_settlement_px", 0.6),
            settlement_px: r
                .get("settlement_px")
                .map(|_| pair(r, "settlement_px", [0.0, 0.0])),
            settlement_kinds: strings(r, "settlement_kinds"),
            allow_river: r["allow_river"].as_bool().unwrap_or(false),
            near_river_px: r
                .get("near_river_px")
                .and_then(Value::as_f64)
                .map(|x| x as f32),
            crossing_types: if r.get("crossing_types").is_some() {
                strings(r, "crossing_types")
            } else {
                vec!["bridge".into()]
            },
            offset_px: num(r, "offset_px", 0.2),
            inland_px: num(r, "inland_px", 0.3),
            kinds: strings(r, "kinds"),
            distance_px: pair(r, "distance_px", [1.0, 2.0]),
            nodes: strings(r, "nodes"),
            per_province: num(r, "per_province", 0.0),
            per_area_px: num(r, "per_area_px", 0.0),
            period: period(r),
        })
        .collect();
    let sites = doc["sites"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| {
            let px = pair(s, "px", [f32::NAN, 0.0]);
            let off = pair(s, "offset_px", [0.0, 0.0]);
            Site {
                id: s["id"].as_str().unwrap_or("").to_string(),
                kind: s["type"].as_str().unwrap_or("").to_string(),
                px: (!px[0].is_nan()).then(|| Vec2::new(px[0], px[1])),
                near: s["near"].as_str().map(str::to_string),
                offset_px: Vec2::new(off[0], off[1]),
                yaw_deg: s.get("yaw_deg").and_then(Value::as_f64).map(|x| x as f32),
                period: period(s),
            }
        })
        .collect();
    DecorConfig {
        seed: doc["seed"].as_i64().unwrap_or(1),
        render,
        types,
        type_index,
        rules,
        sites,
    }
}

#[derive(Debug, Clone)]
pub struct ProvinceInfo {
    pub id: String,
    pub terrain: String,
    pub climate: String,
    pub resources: Vec<String>,
    pub coastal: bool,
}

#[derive(Debug, Clone)]
pub struct Settlement {
    pub id: String,
    pub kind: String,
    pub px: Vec2,
}

#[derive(Debug, Clone)]
pub struct EdgePath {
    pub from: String,
    pub to: String,
    pub points: Vec<Vec2>,
}

#[derive(Debug, Clone)]
pub struct CrossingPoint {
    pub kind: String,
    pub px: Vec2,
    pub dir: Vec2,
}

#[derive(Debug, Clone)]
pub struct ProvinceShape {
    pub id: String,
    pub area_px: f32,
    pub min: Vec2,
    pub max: Vec2,
}

/// What the planner reads of the map.
pub trait DecorMap {
    fn size(&self) -> Vec2;
    fn is_land(&self, px: Vec2) -> bool;
    fn river_sd(&self, px: Vec2) -> f32;
    fn height_m(&self, px: Vec2) -> f32;
    /// Id of the province at the point, `None` over the sea.
    fn province_at(&self, px: Vec2) -> Option<String>;
}

/// A decor instance to stage.
#[derive(Debug, Clone)]
pub struct Instance {
    pub kind: usize,
    pub px: Vec2,
    pub yaw: f32,
    pub rule: String,
    pub province: String,
    pub period: Period,
}

pub struct Inputs<'a> {
    pub map: &'a dyn DecorMap,
    pub settlements: &'a [Settlement],
    pub provinces: &'a [ProvinceShape],
    pub province_info: &'a HashMap<String, ProvinceInfo>,
    pub edges: &'a [EdgePath],
    pub crossings: &'a [CrossingPoint],
    pub coastlines: &'a [Vec<Vec2>],
}

pub struct Planner<'a> {
    cfg: &'a DecorConfig,
    inp: Inputs<'a>,
    rng: Rng,
    rule: Option<usize>,
    taken: HashSet<IVec2>,
    cells: HashMap<IVec2, Vec<usize>>,
    adjacency: HashMap<String, Vec<(String, usize, f32, bool)>>,
    pub instances: Vec<Instance>,
    pub per_rule: Vec<(String, usize)>,
}

fn yaw_along(t: Vec2) -> f32 {
    (-t.y).atan2(t.x)
}

fn cell_of(p: Vec2, side: f32) -> IVec2 {
    (p / side).floor().as_ivec2()
}

fn length_of(points: &[Vec2]) -> f32 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn point_at_distance(path: &[Vec2], distance: f32) -> (Vec2, Vec2) {
    let mut walked = 0.0;
    for w in path.windows(2) {
        let len = w[0].distance(w[1]);
        if len > 1e-4 && walked + len >= distance {
            let t = (w[1] - w[0]) / len;
            return (w[0] + t * (distance - walked), t);
        }
        walked += len;
    }
    if path.len() >= 2 {
        let tail = (path[path.len() - 1] - path[path.len() - 2]).normalize_or_zero();
        return (path[path.len() - 1], tail);
    }
    (Vec2::ZERO, Vec2::X)
}

impl<'a> Planner<'a> {
    pub fn new(cfg: &'a DecorConfig, inp: Inputs<'a>) -> Self {
        let mut cells: HashMap<IVec2, Vec<usize>> = HashMap::new();
        for (i, s) in inp.settlements.iter().enumerate() {
            cells
                .entry(cell_of(s.px, SETTLEMENT_CELL))
                .or_default()
                .push(i);
        }
        Planner {
            cfg,
            inp,
            rng: Rng::new(0),
            rule: None,
            taken: HashSet::new(),
            cells,
            adjacency: HashMap::new(),
            instances: Vec::new(),
            per_rule: Vec::new(),
        }
    }

    /// Sites first, then the rules in the order of the file.
    pub fn plan(&mut self) {
        for site in &self.cfg.sites {
            self.plan_site(site);
        }
        for (index, rule) in self.cfg.rules.iter().enumerate() {
            self.rule = Some(index);
            self.rng = Rng::new((self.cfg.seed * 1_000_003 + rule.seed) as u64);
            let before = self.instances.len();
            match rule.mode.as_str() {
                "road" => self.plan_roads(),
                "gate" => self.plan_gates(),
                "crossing" => self.plan_crossings(),
                "coast" => self.plan_coast(),
                "province" => self.plan_provinces(),
                "route" => self.plan_route(),
                _ => {}
            }
            self.per_rule
                .push((rule.id.clone(), self.instances.len() - before));
        }
    }

    fn r(&self) -> &'a Rule {
        &self.cfg.rules[self.rule.expect("rule set")]
    }

    fn is_land(&self, px: Vec2) -> bool {
        let s = self.inp.map.size();
        px.x >= 1.0
            && px.y >= 1.0
            && px.x < s.x - 1.0
            && px.y < s.y - 1.0
            && self.inp.map.is_land(px)
    }

    fn snap_to_land(&self, px: Vec2) -> Option<Vec2> {
        if self.is_land(px) {
            return Some(px);
        }
        let mut best = None;
        let mut best_d = f32::INFINITY;
        for dy in -SNAP_RADIUS..=SNAP_RADIUS {
            for dx in -SNAP_RADIUS..=SNAP_RADIUS {
                let c = px + Vec2::new(dx as f32, dy as f32);
                let d = c.distance_squared(px);
                if d < best_d && self.is_land(c) {
                    best_d = d;
                    best = Some(c);
                }
            }
        }
        best
    }

    fn plan_site(&mut self, site: &Site) {
        let Some(&kind) = self.cfg.type_index.get(&site.kind) else {
            return;
        };
        let px = if let Some(near) = &site.near {
            let Some(s) = self.inp.settlements.iter().find(|s| &s.id == near) else {
                return;
            };
            s.px + site.offset_px
        } else if let Some(p) = site.px {
            p
        } else {
            return;
        };
        let Some(px) = self.snap_to_land(px) else {
            return;
        };
        let yaw = site.yaw_deg.map_or_else(
            || {
                (site
                    .id
                    .bytes()
                    .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32))
                    % 628) as f32
                    / 100.0
            },
            f32::to_radians,
        );
        let province = self.inp.map.province_at(px).unwrap_or_default();
        self.take(px);
        self.instances.push(Instance {
            kind,
            px,
            yaw,
            rule: site.id.clone(),
            province,
            period: site.period.clone(),
        });
    }

    fn plan_roads(&mut self) {
        let mut seen: HashSet<(String, String)> = HashSet::new();
        for e in self.inp.edges {
            if seen.contains(&(e.to.clone(), e.from.clone())) {
                continue;
            }
            seen.insert((e.from.clone(), e.to.clone()));
            self.along(&e.points);
        }
    }

    /// Posts on a polyline every `spacing_px` (random start).
    fn along(&mut self, points: &[Vec2]) {
        let rule = self.r();
        let spacing = rule.spacing_px;
        let mut next_at = self.rng.range(0.2, 1.0) * spacing;
        let mut walked = 0.0;
        for w in points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let length = a.distance(b);
            if length <= 1e-4 {
                continue;
            }
            let tangent = (b - a) / length;
            while next_at <= walked + length {
                let along = next_at - walked;
                next_at += spacing * self.rng.range(0.7, 1.3);
                if self.rng.f32() > rule.chance {
                    continue;
                }
                let normal = Vec2::new(-tangent.y, tangent.x);
                let side = if self.rng.f32() < 0.5 { -1.0 } else { 1.0 };
                let lateral = self.rng.range(rule.lateral_px[0], rule.lateral_px[1]);
                self.try_emit(
                    a + tangent * along + normal * side * lateral,
                    yaw_along(tangent),
                );
            }
            walked += length;
        }
    }

    fn plan_gates(&mut self) {
        let rule = self.r();
        self.build_adjacency();
        for s in self.inp.settlements {
            if !rule.kinds.is_empty() && !rule.kinds.contains(&s.kind) {
                continue;
            }
            if self.rng.f32() > rule.chance {
                continue;
            }
            let wanted = self.rng.range(rule.distance_px[0], rule.distance_px[1]);
            let edges = self.adjacency.get(&s.id).cloned().unwrap_or_default();
            let mut px = s.px;
            let mut yaw = self.rng.f32() * TAU;
            if edges.is_empty() {
                px += Vec2::from_angle(yaw) * wanted;
            } else {
                let pick = (self.rng.next_u64() as usize) % edges.len();
                let (_, edge, _, reversed) = &edges[pick];
                let path = self.oriented_path(*edge, *reversed);
                let (at, tangent) = point_at_distance(&path, wanted);
                let lateral = self.rng.range(0.05, 0.14);
                px = at + Vec2::new(-tangent.y, tangent.x) * lateral;
                yaw = yaw_along(tangent);
            }
            self.try_emit(px, yaw);
        }
    }

    fn plan_crossings(&mut self) {
        let rule = self.r();
        for c in self.inp.crossings {
            if !rule.crossing_types.contains(&c.kind) {
                continue;
            }
            if self.rng.f32() > rule.chance {
                continue;
            }
            let tangent = c.dir.normalize_or_zero();
            let side = if self.rng.f32() < 0.5 { -1.0 } else { 1.0 };
            self.try_emit(c.px + tangent * rule.offset_px * side, yaw_along(tangent));
        }
    }

    fn plan_coast(&mut self) {
        let rule = self.r();
        for line in self.inp.coastlines {
            if line.len() < 2 {
                continue;
            }
            let mut next_at = self.rng.range(0.2, 1.0) * rule.spacing_px;
            let mut walked = 0.0;
            for w in line.windows(2) {
                let (a, b) = (w[0], w[1]);
                let length = a.distance(b);
                if length <= 1e-4 {
                    continue;
                }
                let tangent = (b - a) / length;
                while next_at <= walked + length {
                    let p = a + tangent * (next_at - walked);
                    next_at += rule.spacing_px * self.rng.range(0.7, 1.3);
                    if self.rng.f32() > rule.chance {
                        continue;
                    }
                    let normal = Vec2::new(-tangent.y, tangent.x);
                    let mut px = p + normal * rule.inland_px;
                    if !self.is_land(px) {
                        px = p - normal * rule.inland_px;
                    }
                    if self.settlement_range_ok(px) {
                        self.try_emit(px, yaw_along(tangent));
                    }
                }
                walked += length;
            }
        }
    }

    fn settlement_range_ok(&self, px: Vec2) -> bool {
        let rule = self.r();
        let Some(range) = rule.settlement_px else {
            return true;
        };
        let reach = (range[1] / SETTLEMENT_CELL).ceil() as i32;
        let cell = cell_of(px, SETTLEMENT_CELL);
        let mut nearest = f32::INFINITY;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                for &i in self
                    .cells
                    .get(&(cell + IVec2::new(dx, dy)))
                    .into_iter()
                    .flatten()
                {
                    let s = &self.inp.settlements[i];
                    if rule.settlement_kinds.is_empty() || rule.settlement_kinds.contains(&s.kind) {
                        nearest = nearest.min(px.distance(s.px));
                    }
                }
            }
        }
        nearest >= range[0] && nearest <= range[1]
    }

    fn plan_provinces(&mut self) {
        let rule = self.r();
        for shape in self.inp.provinces {
            let Some(info) = self.inp.province_info.get(&shape.id) else {
                continue;
            };
            if !self.province_matches(info) {
                continue;
            }
            let expected = rule.per_province + shape.area_px * rule.per_area_px;
            let count =
                expected as usize + usize::from(self.rng.f32() < expected - expected.floor());
            let size = shape.max - shape.min;
            if count == 0 || size == Vec2::ZERO {
                continue;
            }
            let mut placed = 0;
            for _ in 0..count * SAMPLE_TRIES {
                if placed >= count {
                    break;
                }
                let px = shape.min + Vec2::new(self.rng.f32() * size.x, self.rng.f32() * size.y);
                if self.inp.map.province_at(px).as_deref() != Some(shape.id.as_str()) {
                    continue;
                }
                let yaw = self.rng.f32() * TAU;
                if self.try_emit(px, yaw) {
                    placed += 1;
                }
            }
        }
    }

    fn province_matches(&self, info: &ProvinceInfo) -> bool {
        let rule = self.r();
        if !rule.provinces.is_empty() && !rule.provinces.contains(&info.id) {
            return false;
        }
        if !rule.terrains.is_empty() && !rule.terrains.contains(&info.terrain) {
            return false;
        }
        if !rule.climates.is_empty() && !rule.climates.contains(&info.climate) {
            return false;
        }
        let has = |r: &String| info.resources.contains(r);
        if !rule.resources_any.is_empty() && !rule.resources_any.iter().any(has) {
            return false;
        }
        if !rule.resources_all.is_empty() && !rule.resources_all.iter().all(has) {
            return false;
        }
        if !rule.provinces_resource.is_empty() && !rule.provinces_resource.iter().any(has) {
            return false;
        }
        true
    }

    fn plan_route(&mut self) {
        let rule = self.r();
        if rule.nodes.len() < 2 {
            return;
        }
        self.build_adjacency();
        let mut line: Vec<Vec2> = Vec::new();
        for pair in rule.nodes.windows(2) {
            let steps = self.shortest_path(&pair[0], &pair[1]);
            if steps.is_empty() {
                let find = |id: &String| {
                    self.inp
                        .settlements
                        .iter()
                        .find(|s| &s.id == id)
                        .map_or(Vec2::ZERO, |s| s.px)
                };
                line.push(find(&pair[0]));
                line.push(find(&pair[1]));
            }
            for (edge, reversed) in steps {
                let mut path = self.oriented_path(edge, reversed);
                if let (Some(last), Some(first)) = (line.last(), path.first())
                    && last.distance(*first) < 0.01
                {
                    path.remove(0);
                }
                line.extend(path);
            }
        }
        if line.len() >= 2 {
            self.along(&line);
        }
    }

    fn build_adjacency(&mut self) {
        if !self.adjacency.is_empty() {
            return;
        }
        for (i, e) in self.inp.edges.iter().enumerate() {
            let len = length_of(&e.points);
            self.adjacency
                .entry(e.from.clone())
                .or_default()
                .push((e.to.clone(), i, len, false));
            self.adjacency
                .entry(e.to.clone())
                .or_default()
                .push((e.from.clone(), i, len, true));
        }
    }

    fn oriented_path(&self, edge: usize, reversed: bool) -> Vec<Vec2> {
        let mut p = self.inp.edges[edge].points.clone();
        if reversed {
            p.reverse();
        }
        p
    }

    /// Shortest path as `(edge, reversed)` steps (Dijkstra).
    fn shortest_path(&self, from: &str, to: &str) -> Vec<(usize, bool)> {
        if from == to || !self.adjacency.contains_key(from) {
            return Vec::new();
        }
        let mut best: HashMap<&str, f32> = HashMap::from([(from, 0.0)]);
        let mut came: HashMap<&str, (&str, usize, bool)> = HashMap::new();
        let mut frontier: HashMap<&str, f32> = HashMap::from([(from, 0.0)]);
        let mut done: HashSet<&str> = HashSet::new();
        while !frontier.is_empty() {
            let (&current, &cost) = frontier
                .iter()
                .min_by(|a, b| a.1.total_cmp(b.1).then(a.0.cmp(b.0)))
                .expect("frontier");
            frontier.remove(current);
            if current == to {
                break;
            }
            done.insert(current);
            for (other, edge, len, reversed) in self.adjacency.get(current).into_iter().flatten() {
                if done.contains(other.as_str()) {
                    continue;
                }
                let c = cost + len;
                if c < best.get(other.as_str()).copied().unwrap_or(f32::INFINITY) {
                    best.insert(other, c);
                    came.insert(other, (current, *edge, *reversed));
                    frontier.insert(other, c);
                }
            }
        }
        let mut steps = Vec::new();
        let mut at = to;
        while let Some(&(prev, edge, reversed)) = came.get(at) {
            steps.push((edge, reversed));
            at = prev;
        }
        steps.reverse();
        if at == from { steps } else { Vec::new() }
    }

    fn settlement_distance(&self, px: Vec2, limit: f32) -> f32 {
        let reach = (limit / SETTLEMENT_CELL).ceil() as i32;
        let cell = cell_of(px, SETTLEMENT_CELL);
        let mut nearest = f32::INFINITY;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                for &i in self
                    .cells
                    .get(&(cell + IVec2::new(dx, dy)))
                    .into_iter()
                    .flatten()
                {
                    nearest = nearest.min(px.distance(self.inp.settlements[i].px));
                }
            }
        }
        nearest
    }

    /// Filters shared by the current rule: land, river, altitude, settlements, province.
    fn accepts(&self, px: Vec2) -> bool {
        let rule = self.r();
        if !self.is_land(px) {
            return false;
        }
        let river = self.inp.map.river_sd(px);
        if river < 0.4 && !rule.allow_river {
            return false;
        }
        let h = self.inp.map.height_m(px);
        if h < rule.min_height_m || h > rule.max_height_m {
            return false;
        }
        if rule.min_settlement_px > 0.0
            && self.settlement_distance(px, rule.min_settlement_px) < rule.min_settlement_px
        {
            return false;
        }
        if let Some(near) = rule.near_river_px
            && river > near
        {
            return false;
        }
        if !rule.provinces.is_empty()
            || !rule.terrains.is_empty()
            || !rule.climates.is_empty()
            || !rule.provinces_resource.is_empty()
        {
            let Some(id) = self.inp.map.province_at(px) else {
                return false;
            };
            let Some(info) = self.inp.province_info.get(&id) else {
                return false;
            };
            if !self.province_matches(info) {
                return false;
            }
        }
        true
    }

    fn take(&mut self, px: Vec2) {
        self.taken.insert(cell_of(px, DEDUP_CELL));
    }

    fn pick_type(&mut self) -> usize {
        let rule = self.r();
        let total: f32 = rule.types.iter().map(|t| t.1).sum();
        let mut roll = self.rng.f32() * total;
        for (name, w) in &rule.types {
            roll -= w;
            if roll <= 0.0 {
                return self.cfg.type_index.get(name).copied().unwrap_or(0);
            }
        }
        rule.types
            .first()
            .and_then(|t| self.cfg.type_index.get(&t.0).copied())
            .unwrap_or(0)
    }

    fn try_emit(&mut self, px: Vec2, yaw: f32) -> bool {
        if !self.accepts(px) || self.taken.contains(&cell_of(px, DEDUP_CELL)) {
            return false;
        }
        let kind = self.pick_type();
        let rule = self.r();
        let province = self.inp.map.province_at(px).unwrap_or_default();
        self.take(px);
        self.instances.push(Instance {
            kind,
            px,
            yaw,
            rule: rule.id.clone(),
            province,
            period: rule.period.clone(),
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct World;
    impl DecorMap for World {
        fn size(&self) -> Vec2 {
            Vec2::splat(1000.0)
        }
        fn is_land(&self, px: Vec2) -> bool {
            px.x > 20.0
        }
        fn river_sd(&self, _: Vec2) -> f32 {
            8.0
        }
        fn height_m(&self, _: Vec2) -> f32 {
            200.0
        }
        fn province_at(&self, _: Vec2) -> Option<String> {
            Some("prov_a".into())
        }
    }

    fn doc() -> Value {
        serde_json::json!({
            "seed": 5,
            "render": {"view_range_units": 70.0},
            "types": {
                "cross": {"model": "m", "size_m": [1.0, 2.0], "fraction": 0.004, "max_distance": 20},
                "mine": {"model": "n", "size_m": [4.0, 5.0], "fraction": 0.006, "max_distance": 30}
            },
            "rules": [
                {"id": "r_road", "mode": "road", "types": {"cross": 1}, "spacing_px": 20.0, "chance": 1.0,
                 "lateral_px": [0.04, 0.12], "seed": 1, "terrains": ["hills"]},
                {"id": "r_mines", "mode": "province", "types": {"mine": 1}, "per_province": 3.0, "per_area_px": 0.0,
                 "resources_any": ["res_iron"], "seed": 2, "min_settlement_px": 1.0},
                {"id": "r_bridge", "mode": "crossing", "types": {"cross": 1}, "seed": 3, "allow_river": true, "chance": 1.0}
            ],
            "sites": [
                {"id": "s1", "type": "mine", "px": [100.0, 100.0]},
                {"id": "s_sea", "type": "mine", "px": [16.0, 100.0]}
            ]
        })
    }

    fn run(terrain: &str, resources: &[&str]) -> Vec<Instance> {
        let cfg = parse(&doc(), true);
        let info = HashMap::from([(
            "prov_a".to_string(),
            ProvinceInfo {
                id: "prov_a".into(),
                terrain: terrain.into(),
                climate: "oceanic".into(),
                resources: resources.iter().map(|s| s.to_string()).collect(),
                coastal: false,
            },
        )]);
        let shapes = [ProvinceShape {
            id: "prov_a".into(),
            area_px: 1.0e4,
            min: Vec2::new(100.0, 100.0),
            max: Vec2::new(300.0, 300.0),
        }];
        let edges = [EdgePath {
            from: "a".into(),
            to: "b".into(),
            points: vec![Vec2::new(100.0, 500.0), Vec2::new(300.0, 500.0)],
        }];
        let crossings = [CrossingPoint {
            kind: "bridge".into(),
            px: Vec2::new(400.0, 400.0),
            dir: Vec2::X,
        }];
        let settlements = [Settlement {
            id: "a".into(),
            kind: "town".into(),
            px: Vec2::new(100.0, 500.0),
        }];
        let map = World;
        let mut planner = Planner::new(
            &cfg,
            Inputs {
                map: &map,
                settlements: &settlements,
                provinces: &shapes,
                province_info: &info,
                edges: &edges,
                crossings: &crossings,
                coastlines: &[],
            },
        );
        planner.plan();
        planner.instances
    }

    #[test]
    fn sites_snap_to_land_and_rules_follow_province_filters() {
        let out = run("hills", &["res_iron"]);
        // Both sites: the sea one snaps to the nearest land pixel.
        assert_eq!(out.iter().filter(|i| i.rule.starts_with("s")).count(), 2);
        let snapped = out.iter().find(|i| i.rule == "s_sea").unwrap();
        assert!(snapped.px.x > 20.0 && snapped.px.x < 24.0);
        let roads = out.iter().filter(|i| i.rule == "r_road").count();
        assert!(roads >= 8, "{roads}");
        assert_eq!(out.iter().filter(|i| i.rule == "r_mines").count(), 3);
        assert_eq!(out.iter().filter(|i| i.rule == "r_bridge").count(), 1);
    }

    #[test]
    fn terrain_and_resource_filters_exclude() {
        let out = run("plains", &[]);
        assert_eq!(out.iter().filter(|i| i.rule == "r_road").count(), 0);
        assert_eq!(out.iter().filter(|i| i.rule == "r_mines").count(), 0);
        // Crossings and sites do not depend on the province.
        assert_eq!(out.iter().filter(|i| i.rule == "r_bridge").count(), 1);
    }

    #[test]
    fn plan_is_deterministic_and_deduplicated() {
        let a = run("hills", &["res_iron"]);
        let b = run("hills", &["res_iron"]);
        assert_eq!(a.len(), b.len());
        assert!(
            a.iter()
                .zip(&b)
                .all(|(x, y)| x.px == y.px && x.kind == y.kind)
        );
        let mut cells = HashSet::new();
        for i in &a {
            assert!(cells.insert(cell_of(i.px, DEDUP_CELL)));
        }
    }

    #[test]
    fn dijkstra_follows_the_graph() {
        let cfg = parse(&doc(), true);
        let edges = [
            EdgePath {
                from: "a".into(),
                to: "b".into(),
                points: vec![Vec2::ZERO, Vec2::new(10.0, 0.0)],
            },
            EdgePath {
                from: "b".into(),
                to: "c".into(),
                points: vec![Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0)],
            },
            EdgePath {
                from: "a".into(),
                to: "c".into(),
                points: vec![Vec2::ZERO, Vec2::new(100.0, 100.0)],
            },
        ];
        let info = HashMap::new();
        let map = World;
        let mut planner = Planner::new(
            &cfg,
            Inputs {
                map: &map,
                settlements: &[],
                provinces: &[],
                province_info: &info,
                edges: &edges,
                crossings: &[],
                coastlines: &[],
            },
        );
        planner.build_adjacency();
        assert_eq!(
            planner.shortest_path("a", "c"),
            vec![(0, false), (1, false)]
        );
        assert_eq!(planner.shortest_path("c", "a"), vec![(1, true), (0, true)]);
        assert!(planner.shortest_path("a", "zz").is_empty());
    }

    #[test]
    fn repository_document_parses() {
        let root = crate::paths::find_root().expect("root");
        let doc: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("data/map/map_landmarks_extra.json")).unwrap(),
        )
        .unwrap();
        let cfg = parse(&doc, true);
        assert_eq!(cfg.types.len(), 52);
        assert_eq!(cfg.rules.len(), 30);
        assert_eq!(cfg.sites.len(), 84);
        assert_eq!(cfg.render.distance_scale, 3.0);
        assert_eq!(cfg.render.size_distance, 50.0);
        assert_eq!(cfg.render.view_range_units, 220.0);
        // The plume types (`decor_smoke.rs`).
        let mut smoky: Vec<&str> = cfg
            .types
            .iter()
            .filter(|t| t.smoke)
            .map(|t| t.id.as_str())
            .collect();
        smoky.sort_unstable();
        assert_eq!(smoky, ["charcoal_kiln", "forge_water", "glassworks"]);
    }
}
