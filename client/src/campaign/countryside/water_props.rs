//! Shore dressing in generated glb (port of `water_props_layer.gd`): water mills on the banks of
//! the places that own a `bld_water_mill`, piers, cranes and beached boats of the ports, moored
//! ships, shipyard slips, arsenals and wrecks. Rules and models: `data/art/dn_water_models.json`.
//!
//! The plan (positions, headings, kinds) is computed on a thread from the settlement files, the
//! land mask, the rivers and the sea basins (`plan`, pure and tested). At run time the places near
//! the focus are staged as glb scenes (Godot: one `MultiMesh` per model and 320-unit tile), at most
//! [`MAX_LIVE`], nearest first, each placed on the displayed surface (re-placed when the relief
//! changes) and shrunk by the zoom fade (`1 - strategic`, as the Godot `update_view` alpha).
//!
//! Model fit follows `DnWaterModels._measure`: long axis on X (extra `yaw_deg`), scaled so that the
//! long side is `length` world units, centred, foot at the origin.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use bevy::camera::primitives::Aabb;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use serde_json::{Value, json};

use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::markers::places::Places;
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::GroundHeight;

/// Live scene entities cap.
pub const MAX_LIVE: usize = 450;
const SHORE_STEP: f32 = 0.5;
const SHORE_DIRECTIONS: usize = 24;
const GRID_CELL: f32 = 8.0;
/// Cell side (px) of the spatial index of the places.
const INDEX_CELL: f32 = 64.0;

// --- Data ---------------------------------------------------------------------------------

/// A model of the registry (`models` of `dn_water_models.json`).
#[derive(Debug, Clone)]
pub struct WaterModel {
    pub path: String,
    pub length: f32,
    pub yaw_deg: f32,
    pub lift: f32,
    pub from_year: i32,
    pub until_year: i32,
}

#[derive(Debug, Clone, Default)]
pub struct Rules {
    pub year: i32,
    pub lod_main: usize,
    pub prop_range: f32,
    pub models: HashMap<String, WaterModel>,
    pub ports: Value,
    pub mills: Value,
    pub shipyards: Vec<String>,
    pub wrecks: Value,
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str().map(str::to_string))
        .collect()
}

fn f(v: &Value, key: &str, default: f32) -> f32 {
    v[key].as_f64().map_or(default, |x| x as f32)
}

pub fn parse_rules(doc: &Value) -> Rules {
    let d = &doc["defaults"];
    let mut rules = Rules {
        year: d["year"].as_i64().unwrap_or(1337) as i32,
        lod_main: d["lod_main"].as_u64().unwrap_or(1) as usize,
        prop_range: f(d, "prop_range", 420.0),
        ports: doc["ports"].clone(),
        mills: doc["mills"].clone(),
        shipyards: strings(&doc["shipyards"]["places"]),
        wrecks: doc["wrecks"].clone(),
        ..Default::default()
    };
    for (id, e) in doc["models"].as_object().into_iter().flatten() {
        rules.models.insert(
            id.clone(),
            WaterModel {
                path: e["path"].as_str().unwrap_or("").to_string(),
                length: f(e, "length", 1.0),
                yaw_deg: f(e, "yaw_deg", 0.0),
                lift: f(e, "lift", 0.0),
                from_year: e["from_year"].as_i64().unwrap_or(-9999) as i32,
                until_year: e["until_year"].as_i64().unwrap_or(9999) as i32,
            },
        );
    }
    rules
}

impl Rules {
    fn in_service(&self, id: &str) -> bool {
        self.models
            .get(id)
            .is_some_and(|m| self.year >= m.from_year && self.year <= m.until_year)
    }

    /// `DnWaterModels.pick`: the `index`-th model in service of `ids`.
    pub fn pick(&self, ids: &Value, index: usize) -> String {
        let usable: Vec<String> = strings(ids)
            .into_iter()
            .filter(|i| self.in_service(i))
            .collect();
        if usable.is_empty() {
            String::new()
        } else {
            usable[index % usable.len()].clone()
        }
    }
}

/// A settlement as the planner needs it.
#[derive(Debug, Clone)]
pub struct Site {
    pub id: String,
    pub kind: String,
    pub px: Vec2,
    pub weight: u32,
    pub family: String,
    pub port: bool,
    pub buildings: Vec<String>,
}

/// What the planner asks of the map.
pub trait Env {
    fn land(&self, px: Vec2) -> bool;
    fn basin_at(&self, px: Vec2) -> String;
    /// Nearest river point within `max_px`: (point, tangent, importance).
    fn nearest_river(&self, center: Vec2, max_px: f32) -> Option<(Vec2, Vec2, i64)>;
}

/// Where a place stands vertically.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Level {
    Land,
    Water,
    Shore,
}

#[derive(Debug, Clone)]
pub struct WaterPlace {
    pub model: String,
    pub kind: &'static str,
    pub px: Vec2,
    pub heading: Vec2,
    pub scale: f32,
    pub level: Level,
}

fn hash_of(text: &str) -> usize {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    (h >> 20) as usize
}

/// Nearest shore of `center` within `max_px`: (point on the coast, unit vector towards the sea).
pub fn shore_near(env: &dyn Env, center: Vec2, max_px: f32) -> Option<(Vec2, Vec2)> {
    let center_land = env.land(center);
    let mut best = f32::INFINITY;
    let mut result = None;
    for k in 0..SHORE_DIRECTIONS {
        let angle = std::f32::consts::TAU * k as f32 / SHORE_DIRECTIONS as f32;
        let v = Vec2::new(angle.cos(), angle.sin());
        let mut d = SHORE_STEP;
        while d <= max_px.min(best) {
            if env.land(center + v * d) != center_land {
                best = d;
                result = Some((
                    center + v * (d - SHORE_STEP * 0.5),
                    if center_land { v } else { -v },
                ));
                break;
            }
            d += SHORE_STEP;
        }
    }
    result
}

struct Planner<'a> {
    rules: &'a Rules,
    env: &'a dyn Env,
    out: Vec<WaterPlace>,
}

impl Planner<'_> {
    fn add(&mut self, id: &str, kind: &'static str, px: Vec2, heading: Vec2, level: Level, s: f32) {
        if id.is_empty() || !self.rules.models.contains_key(id) {
            return;
        }
        self.out.push(WaterPlace {
            model: id.to_string(),
            kind,
            px,
            heading,
            scale: s,
            level,
        });
    }

    fn ports(&mut self, sites: &[Site]) {
        let rule = &self.rules.ports;
        let search = f(rule, "search_px", 14.0);
        let arsenals = strings(&rule["arsenal_places"]);
        let wreck_every = self.rules.wrecks["every"].as_u64().unwrap_or(0) as usize;
        let mut port_ids: Vec<(String, Vec2, Vec2)> = Vec::new();
        for site in sites {
            let is_port = site.port || site.buildings.iter().any(|b| b == "bld_port");
            let shipyard = self.rules.shipyards.contains(&site.id);
            if !is_port && !shipyard {
                continue;
            }
            let Some((coast, dir)) = shore_near(self.env, site.px, search) else {
                continue;
            };
            if is_port {
                port_ids.push((site.id.clone(), coast, dir));
            }
            self.port(site, coast, dir, &arsenals, shipyard);
        }
        if wreck_every > 0 {
            port_ids.sort_by(|a, b| a.0.cmp(&b.0));
            let model = self.rules.wrecks["model"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let distance = f(&self.rules.wrecks, "distance_px", 6.0);
            for (n, (id, coast, dir)) in port_ids.iter().enumerate() {
                if n % wreck_every == 0 {
                    self.wreck(*coast, *dir, id, &model, distance);
                }
            }
        }
    }

    fn port(&mut self, site: &Site, coast: Vec2, dir: Vec2, arsenals: &[String], shipyard: bool) {
        let side = Vec2::new(-dir.y, dir.x);
        let hash = hash_of(&site.id);
        let by_kind = &self.rules.ports["by_kind"];
        let rule = &by_kind[site.kind.as_str()];
        for prop in strings(&rule["props"]) {
            match prop.as_str() {
                "port_pier" => self.add(&prop, "port", coast + dir, dir, Level::Water, 1.0),
                "port_crane" => self.add(
                    &prop,
                    "port",
                    coast - dir * 0.3 + side * 2.2,
                    dir,
                    Level::Land,
                    1.0,
                ),
                "port_beach" => self.add(&prop, "port", coast + dir * 0.2, side, Level::Shore, 1.0),
                _ => self.add(&prop, "port", coast, dir, Level::Shore, 1.0),
            }
        }
        if arsenals.contains(&site.id) {
            self.add(
                "port_arsenal",
                "arsenal",
                coast - dir * 2.2 - side * 3.2,
                dir,
                Level::Land,
                1.0,
            );
        }
        if shipyard {
            self.add(
                "shipyard_slip",
                "shipyard",
                coast - dir * 0.4 - side * 2.6,
                dir,
                Level::Shore,
                1.0,
            );
        }
        let mut moored = rule["moored"].as_u64().unwrap_or(0) as usize;
        if moored > 0 && site.weight >= 40 {
            moored += 1;
        }
        let ports = &self.rules.ports;
        let basin = self.env.basin_at(coast + dir * 2.0);
        let ids = ports["moored_by_basin"]
            .get(basin.as_str())
            .unwrap_or(&ports["moored_default"])
            .clone();
        let scale = f(ports, "moored_scale", 1.0);
        for i in 0..moored {
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            let lateral = sign * (2.4 + 1.8 * (i / 2) as f32);
            let mut out = 3.8;
            let mut at = coast + dir * out + side * lateral;
            let mut tries = 0;
            while self.env.land(at) && tries < 3 {
                out += 1.2;
                at = coast + dir * out + side * lateral;
                tries += 1;
            }
            if self.env.land(at) {
                continue;
            }
            let jitter = 0.15 * ((hash + i) % 5) as f32 - 0.3;
            let heading = (side * sign + dir * jitter).normalize_or(side);
            let model = self.rules.pick(&ids, hash + i);
            self.add(&model, "moored", at, heading, Level::Water, scale);
        }
    }

    fn wreck(&mut self, coast: Vec2, dir: Vec2, id: &str, model: &str, distance: f32) {
        let side = Vec2::new(-dir.y, dir.x);
        let hash = hash_of(id);
        let sign = if hash % 2 == 0 { 1.0 } else { -1.0 };
        let at = coast + side * distance * sign + dir * 0.8;
        if self.env.land(at) {
            return;
        }
        let a = (hash % 628) as f32 * 0.01;
        self.add(model, "wreck", at, Vec2::new(a.cos(), a.sin()), Level::Water, 1.0);
    }

    fn mills(&mut self, sites: &[Site]) {
        let rule = self.rules.mills.clone();
        if rule.is_null() {
            return;
        }
        let building = rule["building"].as_str().unwrap_or("bld_water_mill");
        let river_max = f(&rule, "river_max_px", 8.0);
        let coast_max = f(&rule, "coast_max_px", 0.0);
        let bank = f(&rule, "bank_offset_px", 0.9);
        let floating_places = strings(&rule["floating_places"]);
        for site in sites {
            if !site.buildings.iter().any(|b| b == building) {
                continue;
            }
            let hash = hash_of(&site.id);
            if coast_max > 0.0
                && let Some((shore, dir)) = shore_near(self.env, site.px, coast_max)
            {
                let model = self.rules.pick(&rule["tidal"], hash);
                self.add(&model, "mill", shore - dir * 0.4, dir, Level::Shore, 1.0);
                continue;
            }
            let Some((at, tangent, importance)) = self.env.nearest_river(site.px, river_max) else {
                continue;
            };
            if floating_places.contains(&site.id)
                && importance >= rule["floating_min_importance"].as_i64().unwrap_or(6)
            {
                let model = self.rules.pick(&rule["floating"], hash);
                self.add(&model, "mill", at, tangent, Level::Water, 1.0);
                continue;
            }
            let mut normal = Vec2::new(-tangent.y, tangent.x);
            if normal.dot(site.px - at) < 0.0 {
                normal = -normal;
            }
            let ids = rule["by_family"]
                .get(site.family.as_str())
                .unwrap_or(&rule["default"]);
            let model = self.rules.pick(ids, hash);
            self.add(&model, "mill", at + normal * bank, tangent, Level::Shore, 1.0);
        }
    }
}

/// All the shore props of the map.
pub fn plan(rules: &Rules, env: &dyn Env, sites: &[Site]) -> Vec<WaterPlace> {
    let mut planner = Planner {
        rules,
        env,
        out: Vec::new(),
    };
    planner.ports(sites);
    planner.mills(sites);
    planner.out
}

// --- Map access ---------------------------------------------------------------------------

/// Land test of the land mask.
pub type LandFn = Arc<dyn Fn(Vec2) -> bool + Send + Sync>;

struct MapEnv {
    land: Option<LandFn>,
    rivers: Vec<(Vec<Vec2>, i64)>,
    grid: HashMap<IVec2, Vec<(u32, u32)>>,
    basins: Vec<(String, Vec<Vec2>)>,
}

fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let mut j = poly.len().wrapping_sub(1);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

impl MapEnv {
    fn new(
        land: Option<LandFn>,
        rivers: Vec<(Vec<Vec2>, i64)>,
        basins: Vec<(String, Vec<Vec2>)>,
    ) -> Self {
        let mut grid: HashMap<IVec2, Vec<(u32, u32)>> = HashMap::new();
        for (r, (points, _)) in rivers.iter().enumerate() {
            for (i, p) in points.iter().enumerate() {
                grid.entry((*p / GRID_CELL).floor().as_ivec2())
                    .or_default()
                    .push((r as u32, i as u32));
            }
        }
        Self {
            land,
            rivers,
            grid,
            basins,
        }
    }
}

impl Env for MapEnv {
    fn land(&self, px: Vec2) -> bool {
        self.land.as_ref().is_some_and(|l| l(px))
    }

    fn basin_at(&self, px: Vec2) -> String {
        self.basins
            .iter()
            .rev()
            .find(|(_, poly)| point_in_polygon(px, poly))
            .map(|(id, _)| id.clone())
            .unwrap_or_default()
    }

    fn nearest_river(&self, center: Vec2, max_px: f32) -> Option<(Vec2, Vec2, i64)> {
        let cell = (center / GRID_CELL).floor().as_ivec2();
        let reach = (max_px / GRID_CELL).ceil() as i32;
        let mut best = max_px * max_px;
        let mut result = None;
        for dx in -reach..=reach {
            for dy in -reach..=reach {
                for &(r, i) in self.grid.get(&(cell + IVec2::new(dx, dy))).into_iter().flatten() {
                    let (points, importance) = &self.rivers[r as usize];
                    let i = i as usize;
                    let d = points[i].distance_squared(center);
                    if d < best {
                        best = d;
                        let before = points[i.saturating_sub(1)];
                        let after = points[(i + 1).min(points.len() - 1)];
                        let tangent = (after - before).normalize_or(Vec2::X);
                        result = Some((points[i], tangent, *importance));
                    }
                }
            }
        }
        result
    }
}

// --- Resources ----------------------------------------------------------------------------

/// Planned places, ready for the view.
#[derive(Resource, Clone)]
pub struct WaterPlan {
    pub rules: Arc<Rules>,
    pub places: Arc<Vec<WaterPlace>>,
    pub cells: Arc<HashMap<IVec2, Vec<u32>>>,
    /// Model file per place model id.
    pub files: Arc<HashMap<String, String>>,
    pub plan_ms: f32,
    pub meters_per_px: f32,
}

#[derive(Resource)]
pub struct PendingWaterPlan(Arc<Mutex<Option<Result<WaterPlan, String>>>>);

#[derive(Component)]
pub struct WaterProp {
    pub index: u32,
    pub grounded: bool,
    pub last_relief: u32,
}

/// Model child waiting for its measure.
#[derive(Component)]
pub struct WaterFit(pub String);

#[derive(Resource, Default)]
pub struct WaterView {
    pub live: HashMap<u32, Entity>,
    pub candidates: usize,
    pub spawned: usize,
    pub despawned: usize,
    pub visible: usize,
    pub fitted: usize,
    pub relief_version: u32,
    pub opacity: f32,
    pub errors: Vec<String>,
    pub measures: HashMap<String, (Vec3, Vec3)>,
}

// --- Loading ------------------------------------------------------------------------------

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn read_sites(data: &Path, places: &[Site]) -> Vec<Site> {
    let mut extra: HashMap<String, (bool, Vec<String>)> = HashMap::new();
    if let Ok(dir) = std::fs::read_dir(data.join("settlements")) {
        for entry in dir.flatten() {
            if let Some(Value::Array(list)) = read_json(&entry.path()) {
                for s in list {
                    if let Some(id) = s["id"].as_str() {
                        extra.insert(
                            id.to_string(),
                            (s["port"].as_bool().unwrap_or(false), strings(&s["buildings"])),
                        );
                    }
                }
            }
        }
    }
    places
        .iter()
        .map(|p| {
            let (port, buildings) = extra.get(&p.id).cloned().unwrap_or_default();
            Site {
                port,
                buildings,
                ..p.clone()
            }
        })
        .collect()
}

fn plan_world(
    root: std::path::PathBuf,
    land: Option<LandFn>,
    sites: Vec<Site>,
    meters_per_px: f32,
) -> Result<WaterPlan, String> {
    let started = std::time::Instant::now();
    let data = root.join("data");
    let doc = read_json(&data.join("art/dn_water_models.json"))
        .ok_or("art/dn_water_models.json missing or invalid")?;
    let rules = parse_rules(&doc);
    let water = crate::campaign::water::data::WaterData::load(&data.join("map"));
    let rivers = water
        .rivers
        .iter()
        .map(|r| (r.points.clone(), r.importance))
        .collect();
    let basins = water
        .basins
        .basins
        .iter()
        .map(|b| {
            (
                b.id.clone(),
                b.polygon_px.iter().map(|p| Vec2::new(p[0], p[1])).collect(),
            )
        })
        .collect();
    let env = MapEnv::new(land, rivers, basins);
    let sites = read_sites(&data, &sites);
    let mut places = plan(&rules, &env, &sites);
    // Models whose glb is missing are dropped (Godot: `DnWaterModels.info` empty).
    let assets = crate::paths::asset_root(&root);
    let mut files: HashMap<String, String> = HashMap::new();
    for (id, m) in &rules.models {
        let found: Vec<Option<String>> = (0..3)
            .map(|l| {
                let p = format!("models/{}_lod{l}.glb", m.path);
                assets.join(&p).exists().then_some(p)
            })
            .collect();
        let want = rules.lod_main.min(2);
        let pick = found[want]
            .clone()
            .or_else(|| (0..want).rev().find_map(|k| found[k].clone()))
            .or_else(|| (want + 1..3).find_map(|k| found[k].clone()));
        if let Some(p) = pick {
            files.insert(id.clone(), p);
        }
    }
    places.retain(|p| files.contains_key(&p.model));
    let mut cells: HashMap<IVec2, Vec<u32>> = HashMap::new();
    for (i, p) in places.iter().enumerate() {
        cells
            .entry((p.px / INDEX_CELL).floor().as_ivec2())
            .or_default()
            .push(i as u32);
    }
    Ok(WaterPlan {
        rules: Arc::new(rules),
        places: Arc::new(places),
        cells: Arc::new(cells),
        files: Arc::new(files),
        plan_ms: started.elapsed().as_secs_f32() * 1000.0,
        meters_per_px,
    })
}

pub fn start_plan(
    mut commands: Commands,
    places: Res<Places>,
    life: Res<crate::campaign::life::LifeData>,
) {
    if places.places.is_empty() {
        return;
    }
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    let sites: Vec<Site> = places
        .places
        .iter()
        .map(|p| Site {
            id: p.id.clone(),
            kind: p.kind.clone(),
            px: p.px,
            weight: p.weight,
            family: p.family.clone(),
            port: false,
            buildings: Vec::new(),
        })
        .collect();
    let mask = life.land.clone();
    let land: Option<LandFn> = mask.map(|m| Arc::new(move |p: Vec2| m.is_land(p)) as LandFn);
    let mpp = life.meters_per_px;
    let slot = Arc::new(Mutex::new(None));
    let out = slot.clone();
    std::thread::spawn(move || {
        *out.lock().expect("water props slot") = Some(plan_world(root, land, sites, mpp));
    });
    commands.insert_resource(PendingWaterPlan(slot));
}

pub fn finish_plan(
    mut commands: Commands,
    pending: Res<PendingWaterPlan>,
    mut view: ResMut<WaterView>,
) {
    let Some(result) = pending.0.lock().expect("water props slot").take() else {
        return;
    };
    commands.remove_resource::<PendingWaterPlan>();
    match result {
        Ok(plan) => {
            info!(
                "water props: {} places in {:.0} ms",
                plan.places.len(),
                plan.plan_ms
            );
            commands.insert_resource(plan);
        }
        Err(e) => {
            warn!("water props: {e}");
            view.errors.push(e);
        }
    }
}

// --- Streaming ----------------------------------------------------------------------------

/// Horizontal radius (px) around the focus inside which a prop can be within `range` of the camera.
pub fn visible_radius(range: f32, distance: f32) -> f32 {
    (range * range - distance * distance).max(0.0).sqrt()
}

/// Opacity of the layer (`max(near, normal)` weight of the Godot `update_view`).
pub fn opacity_of(zoom: &ZoomView) -> f32 {
    (1.0 - zoom.strategic).clamp(0.0, 1.0).max(zoom.near)
}

pub fn stream_props(
    mut commands: Commands,
    plan: Res<WaterPlan>,
    rig: Option<Res<CampaignRig>>,
    zoom: Option<Res<ZoomView>>,
    server: Res<AssetServer>,
    mut view: ResMut<WaterView>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    let Some(rig) = rig else {
        return;
    };
    view.opacity = zoom.as_deref().map_or(1.0, opacity_of);
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.25;
    let radius = visible_radius(plan.rules.prop_range, rig.distance);
    if radius <= 0.0 || view.opacity <= 0.01 {
        let gone: Vec<Entity> = view.live.drain().map(|(_, e)| e).collect();
        view.despawned += gone.len();
        for e in gone {
            commands.entity(e).despawn();
        }
        view.candidates = 0;
        return;
    }
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    let lo = ((focus - Vec2::splat(radius)) / INDEX_CELL).floor().as_ivec2();
    let hi = ((focus + Vec2::splat(radius)) / INDEX_CELL).floor().as_ivec2();
    let mut found: Vec<(f32, u32)> = Vec::new();
    for cy in lo.y..=hi.y {
        for cx in lo.x..=hi.x {
            for &i in plan.cells.get(&IVec2::new(cx, cy)).into_iter().flatten() {
                let d = plan.places[i as usize].px.distance(focus);
                if d <= radius {
                    found.push((d, i));
                }
            }
        }
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    view.candidates = found.len();
    found.truncate(MAX_LIVE);
    let wanted: HashSet<u32> = found.iter().map(|f| f.1).collect();
    let stale: Vec<u32> = view
        .live
        .keys()
        .filter(|k| !wanted.contains(k))
        .copied()
        .collect();
    for k in stale {
        if let Some(e) = view.live.remove(&k) {
            commands.entity(e).despawn();
            view.despawned += 1;
        }
    }
    for (_, i) in found {
        if view.live.contains_key(&i) {
            continue;
        }
        let place = &plan.places[i as usize];
        let Some(path) = plan.files.get(&place.model) else {
            continue;
        };
        let handle: Handle<WorldAsset> =
            server.load(GltfAssetLabel::Scene(0).from_asset(path.clone()));
        let yaw = -place.heading.y.atan2(place.heading.x);
        let root = commands
            .spawn((
                Name::new(format!("water:{}", place.model)),
                Transform::from_xyz(place.px.x, 0.0, place.px.y)
                    .with_rotation(Quat::from_rotation_y(yaw))
                    .with_scale(Vec3::splat(place.scale)),
                Visibility::Hidden,
                WaterProp {
                    index: i,
                    grounded: false,
                    last_relief: u32::MAX,
                },
                DespawnOnExit(AppState::Campaign),
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("water:model"),
                    Transform::default(),
                    Visibility::Hidden,
                    WaterFit(place.model.clone()),
                ))
                .with_children(|m| {
                    m.spawn(WorldAssetRoot(handle));
                });
            })
            .id();
        view.live.insert(i, root);
        view.spawned += 1;
    }
}

/// Native extent of a model (min, max in the model entity frame), measured from its meshes.
fn measure(
    entity: Entity,
    children: &Query<&Children>,
    meshes: &Query<(&Aabb, &GlobalTransform), With<Mesh3d>>,
    globals: &Query<&GlobalTransform>,
) -> Option<(Vec3, Vec3)> {
    let inverse = globals.get(entity).ok()?.affine().inverse();
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for d in children.iter_descendants(entity) {
        let Ok((aabb, global)) = meshes.get(d) else {
            continue;
        };
        let centre = Vec3::from(aabb.center);
        let half = Vec3::from(aabb.half_extents);
        for s in 0..8 {
            let sign = Vec3::new(
                if s & 1 == 0 { -1.0 } else { 1.0 },
                if s & 2 == 0 { -1.0 } else { 1.0 },
                if s & 4 == 0 { -1.0 } else { 1.0 },
            );
            let corner = inverse.transform_point3(global.affine().transform_point3(centre + half * sign));
            lo = lo.min(corner);
            hi = hi.max(corner);
        }
    }
    (hi.x >= lo.x).then_some((lo, hi))
}

/// The model transform of `DnWaterModels._measure`: long axis on X (plus `yaw_deg`), long side
/// `length` units, centred, foot at the origin.
pub fn fit_transform(lo: Vec3, hi: Vec3, yaw_deg: f32, length: f32) -> Transform {
    let size = hi - lo;
    let mut turn = yaw_deg.to_radians();
    if size.z > size.x {
        turn += std::f32::consts::FRAC_PI_2;
    }
    let rotation = Quat::from_rotation_y(turn);
    let mut olo = Vec3::splat(f32::MAX);
    let mut ohi = Vec3::splat(f32::MIN);
    for s in 0..8 {
        let corner = Vec3::new(
            if s & 1 == 0 { lo.x } else { hi.x },
            if s & 2 == 0 { lo.y } else { hi.y },
            if s & 4 == 0 { lo.z } else { hi.z },
        );
        let r = rotation * corner;
        olo = olo.min(r);
        ohi = ohi.max(r);
    }
    let long_side = (ohi.x - olo.x).max(1e-3);
    let fit = length / long_side;
    let centre = (olo + ohi) * 0.5;
    Transform {
        translation: -Vec3::new(centre.x, olo.y, centre.z) * fit,
        rotation,
        scale: Vec3::splat(fit),
    }
}

pub fn fit_models(
    plan: Res<WaterPlan>,
    mut view: ResMut<WaterView>,
    pending: Query<(Entity, &WaterFit), Without<Fitted>>,
    mut commands: Commands,
    children: Query<&Children>,
    meshes: Query<(&Aabb, &GlobalTransform), With<Mesh3d>>,
    globals: Query<&GlobalTransform>,
    mut transforms: Query<(&mut Transform, &mut Visibility)>,
) {
    for (entity, fit) in &pending {
        let Some(info) = plan.rules.models.get(&fit.0) else {
            continue;
        };
        let (lo, hi) = if let Some(m) = view.measures.get(&fit.0) {
            *m
        } else {
            // The measure is taken in the frame of the model entity as it is now (identity).
            let Some(m) = measure(entity, &children, &meshes, &globals) else {
                continue;
            };
            view.measures.insert(fit.0.clone(), m);
            m
        };
        if let Ok((mut t, mut v)) = transforms.get_mut(entity) {
            *t = fit_transform(lo, hi, info.yaw_deg, info.length);
            *v = Visibility::Inherited;
        }
        view.fitted += 1;
        commands.entity(entity).try_insert(Fitted);
    }
}

#[derive(Component)]
pub struct Fitted;

/// Height on the displayed surface, size (fade) and visibility of the live props.
pub fn apply_props(
    plan: Res<WaterPlan>,
    ground: GroundHeight,
    relief: Option<Res<ReliefState>>,
    mut view: ResMut<WaterView>,
    mut query: Query<(&mut WaterProp, &mut Transform, &mut Visibility)>,
) {
    if relief.is_some_and(|r| r.is_changed()) {
        view.relief_version = view.relief_version.wrapping_add(1);
    }
    let opacity = view.opacity;
    let mut visible = 0;
    for (mut prop, mut tr, mut vis) in &mut query {
        let place = &plan.places[prop.index as usize];
        if (!prop.grounded || prop.last_relief != view.relief_version)
            && let Some(y) = ground.at(place.px.x, place.px.y)
        {
            let lift = plan
                .rules
                .models
                .get(&place.model)
                .map_or(0.0, |m| m.lift * place.scale);
            tr.translation.y = y + lift;
            prop.grounded = true;
            prop.last_relief = view.relief_version;
        }
        tr.scale = Vec3::splat((place.scale * opacity).max(1e-5));
        let show = prop.grounded && opacity > 0.01;
        if show {
            visible += 1;
        }
        let wanted = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != wanted {
            *vis = wanted;
        }
    }
    view.visible = visible;
}

pub fn reset(mut view: ResMut<WaterView>) {
    view.live.clear();
}

pub fn dump(world: &World) -> (String, Value) {
    let view = world.resource::<WaterView>();
    let plan = world.get_resource::<WaterPlan>();
    let mut by_kind: std::collections::BTreeMap<&str, usize> = Default::default();
    let mut by_model: std::collections::BTreeMap<String, usize> = Default::default();
    if let Some(p) = plan {
        for place in p.places.iter() {
            *by_kind.entry(place.kind).or_default() += 1;
            *by_model.entry(place.model.clone()).or_default() += 1;
        }
    }
    (
        "water_props".into(),
        json!({
            "planned": plan.map_or(0, |p| p.places.len()),
            "plan_ms": plan.map_or(0.0, |p| p.plan_ms),
            "by_kind": by_kind,
            "by_model": by_model,
            "live": view.live.len(),
            "candidates": view.candidates,
            "spawned": view.spawned,
            "despawned": view.despawned,
            "visible": view.visible,
            "fitted": view.fitted,
            "opacity": view.opacity,
            "errors": view.errors,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Land for x < 50 (a coast running north-south), one river along y at x = 20.
    struct Coast;

    impl Env for Coast {
        fn land(&self, px: Vec2) -> bool {
            px.x < 50.0
        }
        fn basin_at(&self, _: Vec2) -> String {
            "atlantic".into()
        }
        fn nearest_river(&self, center: Vec2, max_px: f32) -> Option<(Vec2, Vec2, i64)> {
            let p = Vec2::new(20.0, center.y);
            (p.distance(center) <= max_px).then_some((p, Vec2::Y, 7))
        }
    }

    fn rules() -> Rules {
        let root = crate::paths::find_root().expect("root");
        parse_rules(&read_json(&root.join("data/art/dn_water_models.json")).expect("json"))
    }

    fn site(id: &str, kind: &str, x: f32, port: bool, buildings: &[&str]) -> Site {
        Site {
            id: id.into(),
            kind: kind.into(),
            px: Vec2::new(x, 100.0),
            weight: 30,
            family: "north".into(),
            port,
            buildings: buildings.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn shore_points_to_the_sea() {
        let (coast, dir) = shore_near(&Coast, Vec2::new(46.0, 100.0), 14.0).expect("shore");
        assert!((coast.x - 50.0).abs() < 0.6, "{coast}");
        assert!(dir.x > 0.9, "{dir}");
        let (_, from_sea) = shore_near(&Coast, Vec2::new(53.0, 100.0), 14.0).expect("shore");
        assert!(from_sea.x > 0.9, "{from_sea}");
        assert!(shore_near(&Coast, Vec2::new(10.0, 100.0), 14.0).is_none());
    }

    #[test]
    fn a_town_port_gets_pier_crane_and_moored_ships() {
        let rules = rules();
        let places = plan(&rules, &Coast, &[site("set_x", "town", 46.0, true, &[])]);
        let kinds = |k: &str| places.iter().filter(|p| p.kind == k).count();
        assert_eq!(kinds("port"), 2, "{places:?}");
        assert_eq!(kinds("moored"), 2);
        let pier = places.iter().find(|p| p.model == "port_pier").unwrap();
        assert_eq!(pier.level, Level::Water);
        assert!(places.iter().filter(|p| p.kind == "moored").all(|p| p.px.x > 50.0));
    }

    #[test]
    fn mill_stands_on_the_river_bank_towards_the_town() {
        let rules = rules();
        let places = plan(
            &rules,
            &Coast,
            &[site("set_m", "village", 16.0, false, &["bld_water_mill"])],
        );
        let mill = places.iter().find(|p| p.kind == "mill").expect("mill");
        assert_eq!(mill.level, Level::Shore);
        assert!(mill.px.x < 20.0, "bank on the town side: {}", mill.px);
        assert!((mill.px.x - 20.0).abs() <= 1.0);
    }

    #[test]
    fn place_without_port_or_mill_gets_nothing() {
        let rules = rules();
        assert!(plan(&rules, &Coast, &[site("set_n", "town", 46.0, false, &[])]).is_empty());
    }

    #[test]
    fn fit_puts_the_long_side_on_x_with_the_foot_at_zero() {
        // A model 1 wide (x), 3 deep (z): turned a quarter so that the long side lies on X.
        let t = fit_transform(Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 2.0, 3.0), 0.0, 6.0);
        assert!((t.scale.x - 2.0).abs() < 1e-4, "{t:?}");
        let foot = t * Vec3::new(0.5, 1.0, 1.5);
        assert!(foot.x.abs() < 1e-4 && foot.y.abs() < 1e-4 && foot.z.abs() < 1e-4);
    }

    #[test]
    fn visible_radius_shrinks_with_height() {
        assert!((visible_radius(420.0, 0.0) - 420.0).abs() < 1e-3);
        assert_eq!(visible_radius(420.0, 500.0), 0.0);
        assert!(visible_radius(420.0, 300.0) < 300.0);
    }

    #[test]
    fn repository_rules_parse() {
        let r = rules();
        assert!(r.models.contains_key("port_pier"));
        assert!(!r.shipyards.is_empty());
        assert_eq!(r.pick(&json!(["mill_small"]), 3), "mill_small");
    }
}
