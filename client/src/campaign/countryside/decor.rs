//! Decor outside the towns, runtime half (port of `decor_layer.gd`): loads the plan of
//! `decor_plan` on a thread, then stages the instances around the focus as glb scenes (Godot: one
//! `MultiMesh` per type; capped here, see [`MAX_LIVE`]).
//!
//! Size: real below `screen.real_below`, then width held on screen (`fraction` of the screen
//! height) up to `max_factor`; in the maquette style (the default) the size is the one of an
//! object seen from `size_distance`, constant with the camera. Range: `view_range_units`, fading
//! (here shrinking) from `fade_from_units`; per type `max_distance` x `distance_scale`.
//!
//! Not ported: procedural fallback volumes (all 52 types have a glb), smoke plumes of the `smoke`
//! types, shadows by distance, the rebuild-on-move batching (instances are streamed one by one).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use serde_json::{Value, json};

use super::LoadedCountryside;
use super::decor_plan::{
    self, CrossingPoint, DecorConfig, DecorMap, EdgePath, Inputs, Instance, Period, Planner,
    ProvinceInfo, ProvinceShape, Settlement,
};
use super::placement::Terrain;
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::camera::zoom::smoothstep;
use crate::campaign::markers::places::Places;
use crate::campaign::provinces::data::ProvinceMap;
use crate::campaign::terrain::surface::GroundHeight;
use crate::campaign::weather::fog::FogOfWar;

/// Live scene entities cap.
pub const MAX_LIVE: usize = 700;

/// Planned decor, ready for the view.
#[derive(Resource, Clone)]
pub struct DecorPlan {
    pub config: Arc<DecorConfig>,
    pub instances: Arc<Vec<Instance>>,
    /// Instances per `cell_px` cell.
    pub cells: Arc<HashMap<IVec2, Vec<u32>>>,
    /// Model file per type and level of detail.
    pub models: Arc<Vec<[Option<String>; 3]>>,
    pub per_rule: Arc<Vec<(String, usize)>>,
    pub plan_ms: f32,
    pub meters_per_px: f32,
}

#[derive(Resource)]
pub struct PendingPlan(Arc<Mutex<Option<Result<DecorPlan, String>>>>);

#[derive(Component)]
pub struct DecorInst {
    pub index: u32,
    pub lod: usize,
    pub grounded: bool,
    pub last_relief: u32,
}

#[derive(Resource, Default)]
pub struct DecorView {
    pub live: HashMap<u32, Entity>,
    pub shown: usize,
    pub candidates: usize,
    pub lod: usize,
    pub spawned: usize,
    pub despawned: usize,
    pub visible: usize,
    pub errors: Vec<String>,
    pub relief_version: u32,
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

struct World {
    terrain: Arc<super::terrain_data::MapTerrain>,
    provinces: ProvinceMap,
}

impl DecorMap for World {
    fn size(&self) -> Vec2 {
        self.terrain.size
    }
    fn is_land(&self, px: Vec2) -> bool {
        self.terrain.is_land(px)
    }
    fn river_sd(&self, px: Vec2) -> f32 {
        self.terrain.river_sd(px)
    }
    fn height_m(&self, px: Vec2) -> f32 {
        self.terrain.height_m(px)
    }
    fn province_at(&self, px: Vec2) -> Option<String> {
        let index = self.provinces.index_at(px.x, px.y);
        (index != 0)
            .then(|| self.provinces.province(index).map(|p| p.id.clone()))
            .flatten()
    }
}

fn plan_world(
    root: std::path::PathBuf,
    loaded: LoadedCountryside,
    places: Vec<Settlement>,
) -> Result<DecorPlan, String> {
    let started = std::time::Instant::now();
    let data = root.join("data");
    let doc = read_json(&data.join("map/map_landmarks_extra.json"))
        .ok_or("map/map_landmarks_extra.json missing or invalid")?;
    let config = decor_plan::parse(&doc, true);
    let provinces = ProvinceMap::load(&data.join("map"))?;
    let shapes: Vec<ProvinceShape> = provinces
        .provinces()
        .iter()
        .filter_map(|p| {
            let mut lo = Vec2::splat(f32::INFINITY);
            let mut hi = Vec2::splat(f32::NEG_INFINITY);
            for ring in &p.rings {
                for pt in ring {
                    lo = lo.min(Vec2::new(pt[0], pt[1]));
                    hi = hi.max(Vec2::new(pt[0], pt[1]));
                }
            }
            (hi.x >= lo.x).then(|| ProvinceShape {
                id: p.id.clone(),
                area_px: p.area_px as f32,
                min: lo,
                max: hi,
            })
        })
        .collect();
    let mut info: HashMap<String, ProvinceInfo> = HashMap::new();
    if let Ok(dir) = std::fs::read_dir(data.join("provinces")) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Some(v) = read_json(&path) {
                let id = v["id"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
                    .unwrap_or_default();
                info.insert(
                    id.clone(),
                    ProvinceInfo {
                        id,
                        terrain: v["terrain"].as_str().unwrap_or("").to_string(),
                        climate: v["climate"].as_str().unwrap_or("").to_string(),
                        resources: v["resources"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|r| r.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        coastal: v["coastal"].as_bool().unwrap_or(false),
                    },
                );
            }
        }
    }
    let pt = |v: &Value| -> Option<Vec2> {
        Some(Vec2::new(
            v.get(0)?.as_f64()? as f32,
            v.get(1)?.as_f64()? as f32,
        ))
    };
    let edges: Vec<EdgePath> = read_json(&data.join("map/settlement_edge_paths.json"))
        .map(|v| {
            v["edges"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|e| EdgePath {
                    from: e["from"].as_str().unwrap_or("").to_string(),
                    to: e["to"].as_str().unwrap_or("").to_string(),
                    points: e["points"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(pt)
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default();
    let crossings: Vec<CrossingPoint> = read_json(&data.join("map/crossings_px.json"))
        .map(|v| {
            v["crossings"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| {
                    Some(CrossingPoint {
                        kind: c["type"].as_str().unwrap_or("").to_string(),
                        px: pt(&c["px"])?,
                        dir: pt(&c["dir"]).unwrap_or(Vec2::X),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let coastlines: Vec<Vec<Vec2>> = read_json(&data.join("map/coastline.geojson"))
        .map(|v| {
            v["features"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| {
                    let coords = f["geometry"]["coordinates"].as_array()?;
                    (f["geometry"]["type"] == "LineString")
                        .then(|| coords.iter().filter_map(pt).collect())
                })
                .collect()
        })
        .unwrap_or_default();
    let world = World {
        terrain: loaded.terrain.clone(),
        provinces,
    };
    let mut planner = Planner::new(
        &config,
        Inputs {
            map: &world,
            settlements: &places,
            provinces: &shapes,
            province_info: &info,
            edges: &edges,
            crossings: &crossings,
            coastlines: &coastlines,
        },
    );
    planner.plan();
    let instances = std::mem::take(&mut planner.instances);
    let per_rule = std::mem::take(&mut planner.per_rule);
    drop(planner);
    let cell = config.render.cell_px;
    let mut cells: HashMap<IVec2, Vec<u32>> = HashMap::new();
    for (i, inst) in instances.iter().enumerate() {
        cells
            .entry((inst.px / cell).floor().as_ivec2())
            .or_default()
            .push(i as u32);
    }
    // Models: `data/art/dn_manifest.json` -> assets[model].files (lod0..2 under models/).
    let manifest = read_json(&data.join("art/dn_manifest.json"));
    let assets = root.join("game/assets");
    let models: Vec<[Option<String>; 3]> = config
        .types
        .iter()
        .map(|t| {
            let files: Vec<String> = manifest
                .as_ref()
                .and_then(|m| m["assets"][&t.model]["files"].as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|f| f.as_str().map(|s| format!("models/{s}")))
                        .collect()
                })
                .unwrap_or_default();
            let found: Vec<Option<String>> = (0..3)
                .map(|l| {
                    files
                        .get(l)
                        .filter(|p| {
                            assets.join(p).exists()
                                || crate::paths::asset_root(&root).join(p).exists()
                        })
                        .cloned()
                })
                .collect();
            std::array::from_fn(|l| {
                found[l]
                    .clone()
                    .or_else(|| (0..l).rev().find_map(|k| found[k].clone()))
                    .or_else(|| (l + 1..3).find_map(|k| found[k].clone()))
            })
        })
        .collect();
    Ok(DecorPlan {
        config: Arc::new(config),
        instances: Arc::new(instances),
        cells: Arc::new(cells),
        models: Arc::new(models),
        per_rule: Arc::new(per_rule),
        plan_ms: started.elapsed().as_secs_f32() * 1000.0,
        meters_per_px: loaded.config.meters_per_px,
    })
}

pub fn start_plan(mut commands: Commands, loaded: Res<LoadedCountryside>, places: Res<Places>) {
    if places.places.is_empty() {
        return;
    }
    let Ok(root) = crate::paths::find_root() else {
        return;
    };
    let settlements: Vec<Settlement> = places
        .places
        .iter()
        .map(|p| Settlement {
            id: p.id.clone(),
            kind: p.kind.clone(),
            px: p.px,
        })
        .collect();
    let slot = Arc::new(Mutex::new(None));
    let out = slot.clone();
    let loaded = loaded.clone();
    std::thread::spawn(move || {
        let result = plan_world(root, loaded, settlements);
        *out.lock().expect("decor plan slot") = Some(result);
    });
    commands.insert_resource(PendingPlan(slot));
}

pub fn finish_plan(mut commands: Commands, pending: Res<PendingPlan>, mut view: ResMut<DecorView>) {
    let Some(result) = pending.0.lock().expect("decor plan slot").take() else {
        return;
    };
    commands.remove_resource::<PendingPlan>();
    match result {
        Ok(plan) => {
            info!(
                "decor: {} instances ({} sites + rules) in {:.0} ms",
                plan.instances.len(),
                plan.config.sites.len(),
                plan.plan_ms
            );
            commands.insert_resource(plan);
        }
        Err(e) => {
            warn!("decor: {e}");
            view.errors.push(e);
        }
    }
}

/// Is the instance visible in the current year and season?
pub fn in_period(period: &Period, year: i32, season: &str) -> bool {
    if year > 0 {
        if period.from_year.is_some_and(|f| year < f) {
            return false;
        }
        if period.to_year.is_some_and(|t| year > t) {
            return false;
        }
    }
    if !season.is_empty()
        && let Some(seasons) = &period.seasons
        && !seasons.iter().any(|s| s == season)
    {
        return false;
    }
    true
}

/// Enlargement of a type at `rig_distance`: 1 at real scale, then width held on screen.
pub fn factor_of(cfg: &DecorConfig, kind: usize, mpp: f32, rig_distance: f32) -> f32 {
    let r = &cfg.render;
    let t = &cfg.types[kind];
    let blend = smoothstep(
        r.real_below,
        r.full_from.max(r.real_below + 1e-3),
        rig_distance,
    );
    let span = 2.0 * (r.fov_deg.to_radians() * 0.5).tan() * rig_distance;
    let full = (t.fraction * span * mpp / t.size_m[0].max(0.1)).clamp(1.0, r.max_factor);
    1.0 + (full - 1.0) * blend
}

pub fn lod_of(cfg: &DecorConfig, rig_distance: f32) -> usize {
    let s = cfg.render.distance_scale;
    if rig_distance < cfg.render.lod_distances[0] * s {
        0
    } else if rig_distance < cfg.render.lod_distances[1] * s {
        1
    } else {
        2
    }
}

fn load_radius(cfg: &DecorConfig, rig_distance: f32) -> f32 {
    (cfg.render.load_factor * rig_distance)
        .clamp(cfg.render.load_min_units, cfg.render.load_max_units)
}

/// Streams the instances of the neighbourhood (nearest first, capped).
pub fn stream_decor(
    mut commands: Commands,
    plan: Res<DecorPlan>,
    rig: Option<Res<CampaignRig>>,
    sim: Res<crate::sim::Sim>,
    fog: Option<Res<FogOfWar>>,
    server: Res<AssetServer>,
    mut view: ResMut<DecorView>,
    mut timer: Local<f32>,
    time: Res<Time>,
    mut instances: Query<(&mut DecorInst, &Children)>,
) {
    let Some(rig) = rig else {
        return;
    };
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.2;
    let cfg = &plan.config;
    let d = rig.distance;
    let lod = lod_of(cfg, d);
    view.lod = lod;
    if d >= cfg.render.view_range_units {
        let gone: Vec<Entity> = view.live.drain().map(|(_, e)| e).collect();
        view.despawned += gone.len();
        for e in gone {
            commands.entity(e).despawn();
        }
        view.shown = 0;
        return;
    }
    let (year, season) = match sim.state.as_ref() {
        Some(s) => (
            s.year(),
            match s.season() {
                sim_campaign::Season::Spring => "spring",
                sim_campaign::Season::Summer => "summer",
                sim_campaign::Season::Autumn => "autumn",
                sim_campaign::Season::Winter => "winter",
            },
        ),
        None => (0, ""),
    };
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    let radius = load_radius(cfg, d);
    let cell = cfg.render.cell_px;
    let lo = ((focus - Vec2::splat(radius)) / cell).floor().as_ivec2();
    let hi = ((focus + Vec2::splat(radius)) / cell).floor().as_ivec2();
    let mut found: Vec<(f32, u32)> = Vec::new();
    for cy in lo.y..=hi.y {
        for cx in lo.x..=hi.x {
            for &i in plan.cells.get(&IVec2::new(cx, cy)).into_iter().flatten() {
                let inst = &plan.instances[i as usize];
                let dist = inst.px.distance(focus);
                let t = &cfg.types[inst.kind];
                if dist > radius || d > t.max_distance * cfg.render.distance_scale {
                    continue;
                }
                if !in_period(&inst.period, year, season) {
                    continue;
                }
                if let Some(fog) = fog.as_ref()
                    && fog.active
                    && !t.known
                    && !inst.province.is_empty()
                    && !fog.provinces.contains(&inst.province)
                {
                    continue;
                }
                found.push((dist, i));
            }
        }
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    view.candidates = found.len();
    found.truncate(MAX_LIVE.min(cfg.render.max_instances));
    let wanted: std::collections::HashSet<u32> = found.iter().map(|f| f.1).collect();
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
    for (_, i) in &found {
        if view.live.contains_key(i) {
            continue;
        }
        let inst = &plan.instances[*i as usize];
        let Some(path) = plan.models[inst.kind][lod].clone() else {
            continue;
        };
        let handle: Handle<WorldAsset> = server.load(GltfAssetLabel::Scene(0).from_asset(path));
        let root = commands
            .spawn((
                Name::new(format!("decor:{}", cfg.types[inst.kind].id)),
                Transform::from_xyz(inst.px.x, 0.0, inst.px.y)
                    .with_rotation(Quat::from_rotation_y(inst.yaw))
                    .with_scale(Vec3::splat(1e-4)),
                Visibility::Hidden,
                DecorInst {
                    index: *i,
                    lod,
                    grounded: false,
                    last_relief: u32::MAX,
                },
                DespawnOnExit(AppState::Campaign),
            ))
            .with_children(|p| {
                p.spawn(WorldAssetRoot(handle));
            })
            .id();
        view.live.insert(*i, root);
        view.spawned += 1;
    }
    view.shown = view.live.len();
    // Level of detail change: swap the scene of the live instances.
    let mut budget = 120;
    for (mut inst, children) in &mut instances {
        if inst.lod == lod || budget == 0 {
            continue;
        }
        let planned = &plan.instances[inst.index as usize];
        let Some(path) = plan.models[planned.kind][lod].clone() else {
            continue;
        };
        budget -= 1;
        let Some(&scene) = children.first() else {
            continue;
        };
        commands.entity(scene).try_insert(WorldAssetRoot(
            server.load(GltfAssetLabel::Scene(0).from_asset(path)),
        ));
        inst.lod = lod;
    }
}

/// Size, ground and range fade of the live instances.
pub fn apply_decor(
    plan: Res<DecorPlan>,
    rig: Option<Res<CampaignRig>>,
    ground: GroundHeight,
    relief: Option<Res<super::ReliefVersion>>,
    mut view: ResMut<DecorView>,
    mut query: Query<(&mut DecorInst, &mut Transform, &mut Visibility)>,
) {
    let Some(rig) = rig else {
        return;
    };
    if relief.is_some_and(|r| r.is_changed()) {
        view.relief_version = view.relief_version.wrapping_add(1);
    }
    let cfg = &plan.config;
    let d = rig.distance;
    let size_distance = if cfg.render.size_distance > 0.0 {
        cfg.render.size_distance
    } else {
        d
    };
    let r = &cfg.render;
    let band = (r.view_range_units - r.fade_from_units.min(r.view_range_units - 1e-3)).max(1e-3);
    let opacity = ((r.view_range_units - d) / band).clamp(0.0, 1.0);
    let mut visible = 0;
    for (mut inst, mut tr, mut vis) in &mut query {
        let planned = &plan.instances[inst.index as usize];
        let scale = factor_of(cfg, planned.kind, plan.meters_per_px, size_distance)
            / plan.meters_per_px
            * opacity;
        if (!inst.grounded || inst.last_relief != view.relief_version)
            && let Some(y) = ground.at(planned.px.x, planned.px.y)
        {
            tr.translation.y = y;
            inst.grounded = true;
            inst.last_relief = view.relief_version;
        }
        tr.scale = Vec3::splat(scale.max(1e-5));
        let show = inst.grounded && opacity > 0.01;
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

pub fn reset(mut view: ResMut<DecorView>) {
    view.live.clear();
}

pub fn dump(world: &World_) -> (String, Value) {
    let view = world.resource::<DecorView>();
    let plan = world.get_resource::<DecorPlan>();
    let data = plan.map_or(Value::Null, |p| {
        let mut by_type: std::collections::BTreeMap<&str, usize> = Default::default();
        for i in p.instances.iter() {
            *by_type.entry(p.config.types[i.kind].id.as_str()).or_default() += 1;
        }
        let rules: Vec<Value> = p
            .per_rule
            .iter()
            .map(|(id, n)| json!({"rule": id, "instances": n}))
            .collect();
        json!({
            "types": p.config.types.len(),
            "rules": p.config.rules.len(),
            "sites": p.config.sites.len(),
            "instances": p.instances.len(),
            "models_resolved": p.models.iter().filter(|m| m[0].is_some() || m[1].is_some() || m[2].is_some()).count(),
            "plan_ms": p.plan_ms,
            "by_type": by_type,
            "per_rule": rules,
        })
    });
    (
        "countryside_decor".into(),
        json!({
            "plan": data,
            "errors": view.errors,
            "live": view.live.len(),
            "candidates": view.candidates,
            "visible": view.visible,
            "lod": view.lod,
            "spawned": view.spawned,
            "despawned": view.despawned,
        }),
    )
}

type World_ = bevy::ecs::world::World;
