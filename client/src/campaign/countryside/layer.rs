//! Streaming and rendering of the countryside props (`CountrysideLayer._process`, `_install_cell`,
//! `_apply_view`, `pack_instances` and `countryside.gdshader`).
//!
//! Cells of `cell_px` px around the focus are sown on demand (`placement`), one root entity per
//! instance holding the mesh primitives of its model (`proto`: shared mesh and material handles,
//! drawn as GPU instances by Bevy's automatic batching; Godot: one `MultiMesh` per model and
//! cell). The live roots are capped at [`MAX_ENTITIES`] (CPU cost of the per-frame transforms).
//! What the vertex shader did is done on the CPU per frame:
//! screen-held size (`size_k` x d^`size_exponent`, never below real size, bounded by `max_mult`),
//! group spread, distance fade, season presence and rank thinning as a scale factor, and the
//! height on the displayed surface.

use std::collections::HashMap;

use bevy::prelude::*;

use super::LoadedCountryside;
use super::placement::{Instance, cell_instances, cell_rect};
use super::proto::{Lookup, Proto, Prototypes, spawn_prims};
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::camera::zoom::{ZoomView, smoothstep};
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::GroundHeight;

/// Live prop roots cap (the Godot layer has no cap: its instances are free). Was 1800 when every
/// instance was a glb scene.
pub const MAX_ENTITIES: usize = 14000;

/// Root of one placed prop.
#[derive(Component)]
pub struct PropInst {
    pub prop: usize,
    pub rule: usize,
    pub cell: IVec2,
    pub base: Vec2,
    pub offset: Vec2,
    pub yaw: f32,
    pub len_m: f32,
    pub rank: f32,
    /// Level of detail of the child model (`usize::MAX`: none yet).
    pub lod: usize,
    pub last_pos: Vec2,
    pub last_relief: u32,
    pub last_ready: bool,
}

/// The glb child of a prop root.
#[derive(Component, Clone)]
pub struct PropModel {
    pub prop: usize,
    pub lod: usize,
}

/// Native size of a prop model (metres) and the origin to put at the foot (centre x, min y,
/// centre z), measured once from the loaded meshes (`mesh_origin` of the Godot shader).
#[derive(Debug, Clone, Copy)]
pub struct Measure {
    pub extent_m: f32,
    pub length_x_m: f32,
    pub origin: Vec3,
}

#[derive(Resource, Default)]
pub struct Measures(pub HashMap<usize, Measure>);

/// Per-prop values of the current frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct PropFrame {
    pub mult: f32,
    pub spread: f32,
    pub fade: f32,
    pub thin: f32,
}

#[derive(Resource, Default)]
pub struct PropView {
    pub active: bool,
    pub focus: Vec2,
    pub distance: f32,
    pub season: usize,
    pub lod: Option<usize>,
    pub frames: Vec<PropFrame>,
    pub relief_version: u32,
}

pub struct Cell {
    pub entities: Vec<Entity>,
    pub count: usize,
    pub counts: HashMap<usize, usize>,
    pub last_seen: u64,
    pub on: bool,
    pub grow: f32,
}

#[derive(Resource, Default)]
pub struct Cells {
    pub cells: HashMap<IVec2, Cell>,
    pub frame: u64,
    pub installed: usize,
    pub evicted: usize,
    pub build_ms_max: f32,
    pub shown_cells: usize,
    pub visible: usize,
    pub radius: f32,
    pub last_instances_per_rule: HashMap<usize, usize>,
    /// Sown cells waiting for their models (`proto`), kept so the sowing is not redone.
    pub staged: HashMap<IVec2, Vec<Instance>>,
    pub waiting_models: usize,
    pub swapped: usize,
}

/// Hysteresis of the level of detail (`CellFade.lod_with_hysteresis`).
pub fn lod_with_hysteresis(
    distance: f32,
    thresholds: &[f32],
    current: usize,
    hysteresis: f32,
) -> usize {
    let mut level = current;
    while level < thresholds.len() && distance >= thresholds[level] * (1.0 + hysteresis) {
        level += 1;
    }
    while level > 0 && distance < thresholds[level - 1] * (1.0 - hysteresis) {
        level -= 1;
    }
    level
}

/// `size_mult`, `spread_mult`, distance fade of one prop at camera distance `d`.
pub fn prop_frame(cfg: &super::config::Config, prop: usize, real_units: f32, d: f32) -> PropFrame {
    let r = &cfg.render;
    let p = &cfg.props[prop];
    let target = r.size_k * d.max(0.01).powf(r.size_exponent);
    let (mult, spread) = if p.fixed_size && cfg.fixed_mult > 0.0 {
        let m = cfg.fixed_mult * p.size_scale;
        (m, m)
    } else {
        let m = (target * cfg.maquette_size * p.size_scale / real_units.max(1e-9))
            .clamp(1.0, p.max_mult * cfg.maquette_size * cfg.maquette_range);
        (m, m.powf(r.spread_exponent))
    };
    let fade = 1.0
        - smoothstep(
            p.fade_from * cfg.maquette_range,
            p.max_distance * cfg.maquette_range,
            d,
        );
    PropFrame {
        mult,
        spread,
        fade,
        thin: 1.0,
    }
}

/// Reads the camera, the season and the relief version.
pub fn update_view(
    cfg: Option<Res<LoadedCountryside>>,
    rig: Option<Res<CampaignRig>>,
    sim: Res<crate::sim::Sim>,
    relief: Option<Res<ReliefState>>,
    measures: Res<Measures>,
    mut view: ResMut<PropView>,
) {
    let Some(cfg) = cfg else {
        view.active = false;
        return;
    };
    if let Some(relief) = relief
        && relief.is_changed()
    {
        view.relief_version = view.relief_version.wrapping_add(1);
    }
    if let Some(state) = sim.state.as_ref() {
        view.season = match state.season {
            sim_campaign::Season::Spring => 0,
            sim_campaign::Season::Summer => 1,
            sim_campaign::Season::Autumn => 2,
            sim_campaign::Season::Winter => 3,
        };
    }
    let Some(rig) = rig else {
        view.active = false;
        return;
    };
    let c = &cfg.config;
    view.focus = Vec2::new(rig.focus.x, rig.focus.z);
    view.distance = rig.distance;
    view.active = rig.distance < c.outer_distance() && !c.rules.is_empty();
    let lod = lod_with_hysteresis(
        rig.distance,
        &c.render.lod_distances,
        view.lod.unwrap_or(0),
        c.render.lod_hysteresis,
    );
    view.lod = Some(lod);
    let mpp = c.meters_per_px;
    let frames = (0..c.props.len())
        .map(|i| {
            let real = measures.0.get(&i).map_or(1.0, |m| m.extent_m / mpp);
            prop_frame(c, i, real, rig.distance)
        })
        .collect();
    view.frames = frames;
}

/// Measure of a model from its prototype bounds (`mesh_origin` of the Godot shader).
fn measure_of(proto: &Proto) -> Measure {
    let size = proto.hi - proto.lo;
    Measure {
        extent_m: size.x.max(size.z).max(1e-3),
        length_x_m: size.x.max(1e-3),
        origin: Vec3::new(
            (proto.lo.x + proto.hi.x) * 0.5,
            proto.lo.y,
            (proto.lo.z + proto.hi.z) * 0.5,
        ),
    }
}

fn spawn_model(
    commands: &mut Commands,
    proto: &Proto,
    prop: usize,
    lod: usize,
    measure: Measure,
) -> Vec<Entity> {
    spawn_prims(commands, proto, -measure.origin, PropModel { prop, lod })
}

/// Sows the cells around the focus, a few per frame, nearest first; evicts the oldest.
pub fn stream_cells(
    mut commands: Commands,
    cfg: Option<Res<LoadedCountryside>>,
    view: Res<PropView>,
    time: Res<Time>,
    server: Res<AssetServer>,
    mut protos: ResMut<Prototypes>,
    mut measures: ResMut<Measures>,
    mut cells: ResMut<Cells>,
) {
    let Some(cfg) = cfg else {
        return;
    };
    cells.frame += 1;
    let c = &cfg.config;
    if !view.active {
        for cell in cells.cells.values_mut() {
            cell.on = false;
        }
        cells.shown_cells = 0;
        return;
    }
    let r = &c.render;
    let side = r.cell_px;
    let radius = (view.distance * r.load_factor).clamp(r.min_radius_px, r.max_radius_px);
    cells.radius = radius;
    let keep_radius = radius * (1.0 + r.load_hysteresis);
    let focus = view.focus;
    let lo = ((focus - Vec2::splat(radius)) / side).floor().as_ivec2();
    let hi = ((focus + Vec2::splat(radius)) / side).floor().as_ivec2();
    let mut wanted: Vec<IVec2> = Vec::new();
    let mut missing: Vec<(f32, IVec2)> = Vec::new();
    for cy in lo.y..=hi.y {
        for cx in lo.x..=hi.x {
            let key = IVec2::new(cx, cy);
            let rect = cell_rect(key, side);
            let nearest = focus.clamp(rect.min, rect.max);
            let reach = if cells.cells.get(&key).is_some_and(|e| e.on) {
                keep_radius
            } else {
                radius
            };
            if nearest.distance_squared(focus) > reach * reach
                || key.x < 0
                || key.y < 0
                || rect.min.x >= cfg.terrain.size.x
                || rect.min.y >= cfg.terrain.size.y
            {
                continue;
            }
            wanted.push(key);
            if !cells.cells.contains_key(&key) {
                missing.push((rect.center().distance_squared(focus), key));
            }
        }
    }
    missing.sort_by(|a, b| a.0.total_cmp(&b.0));
    let live: usize = cells.cells.values().map(|e| e.entities.len()).sum();
    if live < MAX_ENTITIES {
        for (_, key) in missing.into_iter().take(r.cells_per_frame.max(1)) {
            install_cell(
                &mut commands,
                &cfg,
                &server,
                &mut protos,
                &mut measures,
                &view,
                &mut cells,
                key,
            );
        }
    }
    let dt = time.delta_secs().min(0.25);
    let grow_step = if r.spawn_seconds > 0.0 {
        dt / r.spawn_seconds
    } else {
        1.0
    };
    let frame = cells.frame;
    let wanted: std::collections::HashSet<IVec2> = wanted.into_iter().collect();
    cells.staged.retain(|k, _| wanted.contains(k));
    let mut shown = 0;
    for (key, cell) in cells.cells.iter_mut() {
        cell.on = wanted.contains(key);
        if cell.on {
            cell.grow = (cell.grow + grow_step).min(1.0);
            cell.last_seen = frame;
            shown += 1;
        } else {
            cell.grow = 0.0;
        }
    }
    cells.shown_cells = shown;
    // Evict the oldest cells beyond the cache size (or the entity cap).
    let mut total: usize = cells.cells.values().map(|e| e.entities.len()).sum();
    while cells.cells.len() > r.max_cached_cells || total > MAX_ENTITIES + 500 {
        let Some((&oldest, _)) = cells
            .cells
            .iter()
            .filter(|(_, e)| e.last_seen != frame)
            .min_by_key(|(_, e)| e.last_seen)
        else {
            break;
        };
        if let Some(cell) = cells.cells.remove(&oldest) {
            total -= cell.entities.len();
            for e in cell.entities {
                commands.entity(e).despawn();
            }
            cells.evicted += 1;
        }
    }
}

fn install_cell(
    commands: &mut Commands,
    cfg: &LoadedCountryside,
    server: &AssetServer,
    protos: &mut Prototypes,
    measures: &mut Measures,
    view: &PropView,
    cells: &mut Cells,
    key: IVec2,
) {
    let started = std::time::Instant::now();
    let c = &cfg.config;
    let lod = view.lod.unwrap_or(0);
    if let std::collections::hash_map::Entry::Vacant(slot) = cells.staged.entry(key) {
        let mut instances: Vec<Instance> = cell_instances(&*cfg.terrain, c, key);
        // Budget per prop in this cell: lowest ranks first (the rest could never show).
        instances.sort_by(|a, b| a.rank.total_cmp(&b.rank));
        let mut counts: HashMap<usize, usize> = HashMap::new();
        instances.retain(|inst| {
            let n = counts.entry(inst.prop).or_default();
            if *n >= c.props[inst.prop].max_instances.max(40) {
                return false;
            }
            *n += 1;
            true
        });
        slot.insert(instances);
    }
    // Every model of the cell must be ready: the instances are spawned from its primitives.
    let mut resolved: HashMap<usize, Option<std::sync::Arc<Proto>>> = HashMap::new();
    let mut waiting = false;
    for inst in &cells.staged[&key] {
        if resolved.contains_key(&inst.prop) {
            continue;
        }
        let Some(path) = cfg.paths[inst.prop].get(lod.min(2)).cloned().flatten() else {
            resolved.insert(inst.prop, None);
            continue;
        };
        match protos.request(&path, commands, server) {
            Lookup::Ready(p) => {
                measures
                    .0
                    .entry(inst.prop)
                    .or_insert_with(|| measure_of(&p));
                resolved.insert(inst.prop, Some(p));
            }
            Lookup::Failed => {
                resolved.insert(inst.prop, None);
            }
            Lookup::Pending => waiting = true,
        }
    }
    if waiting {
        cells.waiting_models += 1;
        return;
    }
    let instances = cells.staged.remove(&key).unwrap_or_default();
    let mut counts: HashMap<usize, usize> = HashMap::new();
    let mut entities = Vec::new();
    for inst in instances {
        let Some(Some(proto)) = resolved.get(&inst.prop) else {
            continue;
        };
        let measure = measures.0[&inst.prop];
        *counts.entry(inst.prop).or_default() += 1;
        let prims = spawn_model(commands, proto, inst.prop, lod, measure);
        let root = commands
            .spawn((
                Name::new(format!("countryside:{}", c.props[inst.prop].id)),
                Transform::from_xyz(inst.pos.x, 0.0, inst.pos.y).with_scale(Vec3::splat(1e-4)),
                Visibility::Hidden,
                PropInst {
                    prop: inst.prop,
                    rule: inst.rule,
                    cell: key,
                    base: inst.pos,
                    offset: inst.offset,
                    yaw: inst.yaw,
                    len_m: inst.len_m,
                    rank: inst.rank,
                    lod,
                    last_pos: Vec2::splat(f32::NAN),
                    last_relief: u32::MAX,
                    last_ready: false,
                },
                DespawnOnExit(AppState::Campaign),
            ))
            .add_children(&prims)
            .id();
        entities.push(root);
    }
    let count = entities.len();
    cells.installed += 1;
    cells.build_ms_max = cells
        .build_ms_max
        .max(started.elapsed().as_secs_f32() * 1000.0);
    cells.cells.insert(
        key,
        Cell {
            entities,
            count,
            counts,
            last_seen: cells.frame,
            on: false,
            grow: 0.0,
        },
    );
}

/// Replaces the primitives of the props when the level of detail changed.
pub fn swap_lods(
    mut commands: Commands,
    cfg: Option<Res<LoadedCountryside>>,
    view: Res<PropView>,
    server: Res<AssetServer>,
    mut protos: ResMut<Prototypes>,
    mut measures: ResMut<Measures>,
    mut cells: ResMut<Cells>,
    mut roots: Query<(Entity, &mut PropInst, Option<&Children>)>,
    models: Query<(), With<PropModel>>,
) {
    let (Some(cfg), Some(lod)) = (cfg, view.lod) else {
        return;
    };
    let mut budget = 1500usize;
    let mut swapped = 0;
    for (entity, mut inst, children) in &mut roots {
        if inst.lod == lod || budget == 0 {
            continue;
        }
        let Some(path) = cfg.paths[inst.prop].get(lod.min(2)).cloned().flatten() else {
            inst.lod = lod;
            continue;
        };
        let Lookup::Ready(proto) = protos.request(&path, &mut commands, &server) else {
            continue;
        };
        budget -= 1;
        swapped += 1;
        let measure = *measures
            .0
            .entry(inst.prop)
            .or_insert_with(|| measure_of(&proto));
        for child in children.into_iter().flat_map(|c| c.iter()) {
            if models.contains(child) {
                commands.entity(child).despawn();
            }
        }
        let prims = spawn_model(&mut commands, &proto, inst.prop, lod, measure);
        commands.entity(entity).add_children(&prims);
        inst.lod = lod;
    }
    cells.swapped += swapped;
}

/// Writes the transform of every prop (the vertex shader of `countryside.gdshader`).
pub fn apply_view(
    cfg: Option<Res<LoadedCountryside>>,
    view: Res<PropView>,
    zoom: Option<Res<ZoomView>>,
    measures: Res<Measures>,
    ground: GroundHeight,
    mut cells: ResMut<Cells>,
    mut roots: Query<(&mut PropInst, &mut Transform, &mut Visibility)>,
) {
    let Some(cfg) = cfg else {
        return;
    };
    let c = &cfg.config;
    let parchment = zoom.map_or(1.0, |z| 1.0 - z.strategic);
    // Thinning per prop over the shown cells.
    let mut counts = vec![0usize; c.props.len()];
    for cell in cells.cells.values().filter(|e| e.on) {
        for (p, n) in &cell.counts {
            counts[*p] += n;
        }
    }
    let thin: Vec<f32> = counts
        .iter()
        .enumerate()
        .map(|(p, n)| {
            if *n == 0 {
                1.0
            } else {
                (c.props[p].max_instances as f32 / *n as f32).min(1.0)
            }
        })
        .collect();
    let mut visible = 0;
    let grows: HashMap<IVec2, f32> = cells
        .cells
        .iter()
        .filter(|(_, e)| e.on)
        .map(|(k, e)| (*k, e.grow))
        .collect();
    let ready = ground.is_ready();
    for (mut inst, mut transform, mut visibility) in &mut roots {
        let (Some(grow), true) = (grows.get(&inst.cell), view.active) else {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        let (Some(frame), Some(measure)) = (view.frames.get(inst.prop), measures.0.get(&inst.prop))
        else {
            continue;
        };
        let rule = &c.rules[inst.rule];
        let presence = rule.seasons[view.season.min(3)];
        let vis = (((frame.fade.min(thin[inst.prop]) * presence - inst.rank) * 10.0)
            .clamp(0.0, 1.0)
            * grow
            * parchment)
            .max(0.0);
        if vis <= 0.001 {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            continue;
        }
        visible += 1;
        let unit = 1.0 / c.meters_per_px;
        let prop = &c.props[inst.prop];
        let length_factor = if inst.len_m > 0.0 {
            inst.len_m / measure.length_x_m
        } else {
            1.0
        };
        let x_mult = if prop.stretched {
            frame.spread
        } else {
            frame.mult
        };
        transform.scale = Vec3::new(
            unit * length_factor * x_mult * vis,
            unit * frame.mult * vis,
            unit * frame.mult * vis,
        );
        transform.rotation = Quat::from_rotation_y(inst.yaw);
        let pos = inst.base + inst.offset * (frame.spread - 1.0);
        if ready
            && (inst.last_pos != pos || inst.last_relief != view.relief_version || !inst.last_ready)
            && let Some(y) = ground.at(pos.x, pos.y)
        {
            transform.translation = Vec3::new(pos.x, y, pos.y);
            inst.last_pos = pos;
            inst.last_relief = view.relief_version;
            inst.last_ready = true;
        } else if !inst.last_ready {
            // Not grounded yet: stay hidden rather than float at sea level.
            continue;
        }
        if *visibility != Visibility::Visible {
            *visibility = Visibility::Visible;
        }
    }
    cells.visible = visible;
}
