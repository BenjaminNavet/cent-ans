//! Province scenes (FK4, `life_folk/folk_scenes.gd`): the scenes of `sim_campaign::map_scenes`
//! (plague, famine, revolt, devastation, siege, construction, fair, celebration, flood, muster)
//! staged at the edge of the settlement they touch, on the close view.
//!
//! Resolution of the place and the effects outside the pool (quiet chimneys of a plague, smoke of
//! the pyres and the riots, fields without workers in a famine) follow Godot; the staging of the
//! layers is `scene_layers.rs`. `CENT_ANS_SCENES=<province>:<kind>[,...]` forces scenes of
//! intensity 1 (tests, probes), like `--scene=` in Godot.
//!
//! Differences with Godot: figures are static meshes (no work poses), the escape path of the
//! refugees follows the road lines of `LifeData`, the landmark disks are replaced by the radius of
//! the maquette of `Places`, the scene seeds use a different hash, and the idle fields of a
//! famine are only reported (`idle_provinces`: the Rust folk has no field workers yet).

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use serde_json::Value;

use super::data::{LifeData, h01, text_seed};
use super::figures::Figures;
use super::folk::{prop_model, world_scale};
use super::models::{Fade, FitAxis, ModelMetrics, spawn_model};
use super::props::PropGeometry;
use super::scene_layers::{Env, Frame, Item, Stager};
use super::stream::{Cat, Grounded, LifeIndex, LifeItem, LifeView};
use crate::app::AppState;
use crate::campaign::markers::places::Places;
use crate::campaign::terrain::surface::GroundHeight;
use crate::sim::Sim;

/// Margin (map units, about 60 m) between the edge of a settlement and a smoke scene.
pub const SIDE_MARGIN: f32 = 0.08;
/// Margin (m) between the edge of the maquette and the heart of a scene.
pub const EDGE_M: f32 = 6.0;

/// A scene resolved to a settlement.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedScene {
    pub kind: String,
    pub province: String,
    pub settlement: String,
    pub index: usize,
    pub center: Vec2,
    pub seed: u64,
    pub intensity: f32,
    pub footprint: f32,
}

#[derive(Resource, Default)]
pub struct StagedScenes {
    pub scenes: Vec<StagedScene>,
    pub quiet_settlements: HashSet<String>,
    pub idle_provinces: HashSet<String>,
    pub fire_points: Vec<Vec2>,
    pub skipped: usize,
    /// Turn of the last resolution (None before the first).
    pub turn: Option<u32>,
    pub forced_applied: bool,
    /// Bumped at each resolution (the effects follow it).
    pub version: u32,
}

/// A scene as the sim gives it, before the resolution of its place.
#[derive(Debug, Clone)]
pub struct RawScene {
    pub province: String,
    pub settlement: String,
    pub kind: String,
    pub intensity: f32,
}

/// `<province>:<kind>[,...]` -> forced scenes (unknown kinds ignored).
pub fn parse_forced(spec: &str, kinds: &[String]) -> Vec<RawScene> {
    spec.split(',')
        .filter_map(|pair| {
            let (province, kind) = pair.split_once(':')?;
            kinds.iter().any(|k| k == kind).then(|| RawScene {
                province: province.trim().to_string(),
                settlement: String::new(),
                kind: kind.trim().to_string(),
                intensity: 1.0,
            })
        })
        .collect()
}

/// Kinds of the layouts file, in sorted order.
pub fn kinds_of(layouts: &Value) -> Vec<String> {
    layouts
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

/// Direction of the side of the settlement where a scene stands (stable per settlement and kind).
pub fn side_of(seed: u64) -> Vec2 {
    let a = h01(seed, 1, 0) * std::f32::consts::TAU;
    Vec2::new(a.cos(), a.sin())
}

/// Resolves the raw scenes onto the places (settlement named, else the seat of the province),
/// merges the duplicates (same place and kind: strongest intensity) and computes the effects.
pub fn resolve(
    raws: &[RawScene],
    places: &Places,
    layouts: &Value,
    seat: &HashMap<String, usize>,
) -> StagedScenes {
    let kinds = kinds_of(layouts);
    let mut by_key: HashMap<String, StagedScene> = HashMap::new();
    let mut skipped = 0;
    for raw in raws {
        let Some(kind_index) = kinds.iter().position(|k| *k == raw.kind) else {
            skipped += 1;
            continue;
        };
        let index = places
            .index_by_id
            .get(&raw.settlement)
            .copied()
            .or_else(|| seat.get(&raw.province).copied());
        let Some(index) = index else {
            skipped += 1;
            continue;
        };
        let place = &places.places[index];
        let intensity = raw.intensity.clamp(0.0, 1.0);
        let key = format!("{}|{}", place.id, raw.kind);
        if by_key.get(&key).is_some_and(|s| s.intensity >= intensity) {
            continue;
        }
        let landmark = place
            .landmark
            .and_then(|l| places.landmarks.get(l))
            .map_or(0.0, |l| l.core_radius);
        by_key.insert(
            key,
            StagedScene {
                kind: raw.kind.clone(),
                province: raw.province.clone(),
                settlement: place.id.clone(),
                index,
                center: place.px,
                seed: text_seed(&place.id) % (1 << 40) + kind_index as u64 * 7919,
                intensity,
                footprint: place.radius.max(landmark),
            },
        );
    }
    let mut scenes: Vec<StagedScene> = by_key.into_values().collect();
    scenes.sort_by(|a, b| (&a.settlement, &a.kind).cmp(&(&b.settlement, &b.kind)));
    let mut out = StagedScenes {
        skipped,
        ..default()
    };
    for scene in &scenes {
        let effects = &layouts[scene.kind.as_str()]["effects"];
        if effects["quiet_settlement"].as_bool().unwrap_or(false) {
            out.quiet_settlements.insert(scene.settlement.clone());
        }
        if let Some(factor) = effects["fire_side_factor"].as_f64() {
            out.fire_points.push(
                scene.center
                    + side_of(scene.seed) * (scene.footprint * factor as f32 + SIDE_MARGIN),
            );
        }
        if effects["idle_province"].as_bool().unwrap_or(false) {
            out.idle_provinces.insert(scene.province.clone());
        }
    }
    out.scenes = scenes;
    out
}

/// Seat of each province: the first place of the (importance sorted) list.
pub fn seats(places: &Places) -> HashMap<String, usize> {
    let mut seat = HashMap::new();
    for (i, p) in places.places.iter().enumerate() {
        seat.entry(p.province.clone()).or_insert(i);
    }
    seat
}

/// Figures of a scene from its intensity.
pub fn figures_for(intensity: f32, lo: usize, hi: usize) -> usize {
    (lo as f32 + (hi as f32 - lo as f32) * intensity.clamp(0.0, 1.0))
        .round()
        .max(1.0) as usize
}

/// Resolves the scenes once per turn (and once when the places become available).
pub fn refresh_scenes(
    sim: Res<Sim>,
    places: Res<Places>,
    data: Res<LifeData>,
    mut staged: ResMut<StagedScenes>,
) {
    let (Some(state), Some(game)) = (sim.state.as_ref(), sim.data.as_ref()) else {
        return;
    };
    if places.places.is_empty() {
        return;
    }
    if staged.turn == Some(state.turn) && staged.forced_applied {
        return;
    }
    let mut raws: Vec<RawScene> = sim_campaign::map_scenes::map_scenes(state, game)
        .iter()
        .map(|s| RawScene {
            province: s.province.as_str().to_string(),
            settlement: s
                .settlement
                .as_ref()
                .map_or(String::new(), |x| x.as_str().to_string()),
            kind: s.kind.key().to_string(),
            intensity: s.intensity,
        })
        .collect();
    if let Ok(spec) = std::env::var("CENT_ANS_SCENES") {
        raws.extend(parse_forced(&spec, &kinds_of(&data.scene_layouts)));
    }
    let mut next = resolve(&raws, &places, &data.scene_layouts, &seats(&places));
    next.turn = Some(state.turn);
    next.forced_applied = true;
    next.version = staged.version.wrapping_add(1);
    *staged = next;
}

// --- Environment of the stager ------------------------------------------------------------

struct SceneEnv<'a> {
    ws: f32,
    data: &'a LifeData,
    geometry: &'a PropGeometry,
    footprints: &'a HashMap<usize, f32>,
}

impl Env for SceneEnv<'_> {
    fn scale(&self) -> f32 {
        self.ws
    }

    fn slot(&self, prop: &str, slot: &str, fallback: Vec2) -> Vec2 {
        self.geometry.slot(prop, slot, fallback)
    }

    fn escape_path(
        &mut self,
        _index: usize,
        group: usize,
        from: Vec2,
        dir: Vec2,
        salt: u64,
    ) -> Vec<Vec2> {
        escape_path(self.data, self.ws, from, dir, salt, group)
    }

    fn river_points(
        &mut self,
        _index: usize,
        center: Vec2,
        range: f32,
        count: usize,
        spacing: f32,
    ) -> Vec<Vec2> {
        river_points(self.data, center, range, count, spacing)
    }

    fn footprint(&self, index: usize) -> f32 {
        self.footprints.get(&index).copied().unwrap_or(0.5)
    }
}

/// Escape path of refugees (4 points, 3 pieces): the nearest road (followed away from `from`
/// over about 70 m), else an arc.
pub fn escape_path(
    data: &LifeData,
    ws: f32,
    from: Vec2,
    dir: Vec2,
    salt: u64,
    _group: usize,
) -> Vec<Vec2> {
    let reach = 60.0 * ws;
    let mut best = reach * reach;
    let mut found: Option<(usize, usize)> = None;
    for (r, run) in data.road_runs.iter().enumerate() {
        for (i, p) in run.points.iter().enumerate() {
            let d2 = p.distance_squared(from);
            if d2 < best {
                best = d2;
                found = Some((r, i));
            }
        }
    }
    if let Some((r, i)) = found {
        let points = &data.road_runs[r].points;
        let step: isize = if i + 1 >= points.len()
            || (i > 0
                && points[i - 1].distance_squared(from) > points[i + 1].distance_squared(from))
        {
            -1
        } else {
            1
        };
        let mut line = vec![from];
        let mut length = 0.0;
        let mut k = i as isize;
        while k >= 0 && (k as usize) < points.len() && length < 70.0 * ws {
            length += line[line.len() - 1].distance(points[k as usize]);
            line.push(points[k as usize]);
            k += step;
        }
        if line.len() >= 2 {
            return resample(&line, 3);
        }
    }
    let to = from + dir * (70.0 * ws);
    let side = Vec2::new(-dir.y, dir.x);
    let mid = (from + to) * 0.5;
    let sign = if h01(salt, 1, 0) < 0.5 { 1.0 } else { -1.0 };
    let control = mid + side * (18.0 * ws) * 2.0 * sign;
    (0..4)
        .map(|i| {
            let t = i as f32 / 3.0;
            from.lerp(control, t).lerp(control.lerp(to, t), t)
        })
        .collect()
}

/// `pieces` pieces of equal length along a polyline (pieces + 1 points).
pub fn resample(line: &[Vec2], pieces: usize) -> Vec<Vec2> {
    let total: f32 = line.windows(2).map(|w| w[0].distance(w[1])).sum();
    let target = total / pieces as f32;
    let mut out = vec![line[0]];
    let mut walked = 0.0;
    let mut next = target;
    for w in line.windows(2) {
        let seg = w[0].distance(w[1]);
        while seg > 0.0 && next <= walked + seg + 1e-6 && out.len() < pieces {
            out.push(w[0].lerp(w[1], (next - walked) / seg));
            next += target;
        }
        walked += seg;
    }
    out.push(line[line.len() - 1]);
    while out.len() < pieces + 1 {
        out.push(line[line.len() - 1]);
    }
    out
}

/// River points (and mid-points of long pieces) within `range` of `center`, at least `spacing`
/// apart, nearest first, at most `count`.
pub fn river_points(
    data: &LifeData,
    center: Vec2,
    range: f32,
    count: usize,
    spacing: f32,
) -> Vec<Vec2> {
    let r2 = range * range;
    let mut found: Vec<(f32, Vec2)> = Vec::new();
    for river in &data.rivers {
        for (i, p) in river.iter().enumerate() {
            let d = p.distance_squared(center);
            if d <= r2 {
                found.push((d, *p));
            }
            if let Some(n) = river.get(i + 1) {
                let mid = (*p + *n) * 0.5;
                let dm = mid.distance_squared(center);
                if dm <= r2 {
                    found.push((dm, mid));
                }
            }
        }
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<Vec2> = Vec::new();
    for (_, p) in found {
        if out.iter().all(|q| q.distance(p) >= spacing) {
            out.push(p);
            if out.len() >= count {
                break;
            }
        }
    }
    out
}

// --- Spawning -------------------------------------------------------------------------------

/// Root of a staged scene: everything of the scene hangs under it (fades as one).
#[derive(Component)]
pub struct SceneRoot {
    pub kind: String,
    pub footprint: f32,
    pub center: Vec2,
    pub items: usize,
}

/// A figure or a prop of a scene (counted by the dump).
#[derive(Component)]
pub struct SceneItem {
    pub figure: bool,
}

/// Walks back and forth between two points (figure or rolling prop).
#[derive(Component)]
pub struct SceneMover {
    pub from: Vec2,
    pub to: Vec2,
    pub phase: f32,
    pub speed_px: f32,
    pub lateral_px: f32,
    pub behind_px: f32,
    pub center: Vec2,
    pub figure: bool,
}

fn pingpong(x: f32) -> f32 {
    let t = x.rem_euclid(2.0);
    if t <= 1.0 { t } else { 2.0 - t }
}

/// Position and heading of a mover at the time `t`.
pub fn mover_pose(m: &SceneMover, t: f32) -> (Vec2, Vec2) {
    let span = m.to - m.from;
    let len = span.length();
    let dir = if len > 1e-5 { span / len } else { Vec2::X };
    let (u, forward) = if m.speed_px > 0.0 && len > 1e-5 {
        let x = m.phase * 2.0 + t * m.speed_px / len;
        (pingpong(x), x.rem_euclid(2.0) <= 1.0)
    } else {
        (0.0, true)
    };
    let heading = if forward { dir } else { -dir };
    let p = m.from.lerp(m.to, u) + Vec2::new(-heading.y, heading.x) * m.lateral_px
        - heading * m.behind_px;
    (p, heading)
}

pub fn spawn_scenes(
    mut commands: Commands,
    data: Res<LifeData>,
    view: Res<LifeView>,
    staged: Res<StagedScenes>,
    geometry: Res<PropGeometry>,
    root: Option<Res<super::fauna::AssetRoot>>,
    asset_server: Res<AssetServer>,
    metrics: Res<ModelMetrics>,
    mut figures: Figures,
    mut flood: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    mut index: ResMut<LifeIndex>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 || !view.ready || staged.scenes.is_empty() {
        return;
    }
    *timer = 0.3;
    if view.range_target(data.props.folk_range, data.props.folk_range_fade) <= 0.0 {
        return;
    }
    let ws = world_scale(&data);
    let radius = view.radius;
    let footprints: HashMap<usize, f32> = staged
        .scenes
        .iter()
        .map(|s| (s.index, s.footprint))
        .collect();
    let assets_root = root.as_deref().map(|r| r.0.as_path());
    let mut order: Vec<(f32, &StagedScene)> = staged
        .scenes
        .iter()
        .map(|s| (s.center.distance(view.focus), s))
        .filter(|(d, s)| *d <= radius + s.footprint)
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let lo = data.scenes.scene_figures_min;
    let hi = data.scenes.scene_figures_max;
    for (_, scene) in order {
        let key = text_seed("life:scene") ^ scene.seed ^ text_seed(&scene.kind).rotate_left(9);
        if index.0.contains(&key) {
            continue;
        }
        if index.0.len() >= data.scenes.pool_cap {
            break;
        }
        index.0.insert(key);
        let out = side_of(scene.seed);
        let frame = Frame {
            anchor: scene.center + out * (scene.footprint + ws * EDGE_M),
            out,
            along: Vec2::new(-out.y, out.x),
            seed: scene.seed,
            center: scene.center,
            index: scene.index,
        };
        let mut env = SceneEnv {
            ws,
            data: &data,
            geometry: &geometry,
            footprints: &footprints,
        };
        let mut stager = Stager {
            env: &mut env,
            items: Vec::new(),
            budget: data.scenes.pool_cap.saturating_sub(index.0.len()).min(400),
        };
        stager.stage(
            &frame,
            &data.scene_layouts[scene.kind.as_str()],
            figures_for(scene.intensity, lo, hi),
        );
        let items = stager.items;
        let root_entity = commands
            .spawn((
                Name::new(format!("life:scene:{}", scene.kind)),
                Transform::from_xyz(scene.center.x, 0.0, scene.center.y)
                    .with_scale(Vec3::splat(1e-4)),
                Visibility::Inherited,
                SceneRoot {
                    kind: scene.kind.clone(),
                    footprint: scene.footprint,
                    center: scene.center,
                    items: items.len(),
                },
                LifeItem {
                    key,
                    cat: Cat::Scene,
                    anchor: scene.center,
                    h: 0.0,
                },
                Fade::new(1.0),
                DespawnOnExit(AppState::Campaign),
            ))
            .id();
        for (n, item) in items.iter().enumerate() {
            let (model, activity, start, yaw) = match item {
                Item::Mover {
                    model,
                    activity,
                    from,
                    ..
                } => (model, activity, *from, 0.0),
                Item::Static {
                    model,
                    activity,
                    at,
                    yaw,
                } => (model, activity, *at, *yaw),
            };
            let local = Vec3::new(start.x - scene.center.x, 0.0, start.y - scene.center.y);
            let is_prop = data.table.is_prop(model);
            let h = h01(scene.seed, n as u64, 3);
            let h2 = h01(scene.seed, n as u64, 4);
            let body = if is_prop {
                spawn_prop(
                    &mut commands,
                    &asset_server,
                    &metrics,
                    &data,
                    assets_root,
                    &mut figures,
                    &mut flood,
                    ws,
                    model,
                )
            } else {
                Some(figures.spawn(&mut commands, model, 1, h, h2, ws, Vec3::ZERO, activity))
            };
            let Some(body) = body else {
                continue;
            };
            let mut holder = commands.spawn((
                Name::new(format!("life:scene_item:{model}")),
                Transform::from_translation(local)
                    .with_rotation(Quat::from_rotation_y(yaw - std::f32::consts::FRAC_PI_2)),
                Visibility::Inherited,
                SceneItem { figure: !is_prop },
                LifeItem {
                    key: 0,
                    cat: Cat::Scene,
                    anchor: start,
                    h: 0.0,
                },
            ));
            match item {
                Item::Mover {
                    model,
                    activity,
                    from,
                    to,
                    phase,
                    lateral_m,
                    behind_m,
                } => {
                    holder.insert(SceneMover {
                        from: *from,
                        to: *to,
                        phase: *phase,
                        speed_px: data.table.speed_of(model, activity) * ws * data.props.folk_speed,
                        lateral_px: lateral_m * ws,
                        behind_px: behind_m * ws,
                        center: scene.center,
                        figure: !is_prop,
                    });
                }
                Item::Static { .. } => {
                    holder.insert(Grounded::new(0.0));
                }
            }
            let holder = holder.add_child(body).id();
            commands.entity(root_entity).add_child(holder);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_prop(
    commands: &mut Commands,
    asset_server: &AssetServer,
    metrics: &ModelMetrics,
    data: &LifeData,
    root: Option<&std::path::Path>,
    figures: &mut Figures,
    flood: &mut Option<(Handle<Mesh>, Handle<StandardMaterial>)>,
    ws: f32,
    id: &str,
) -> Option<Entity> {
    if id == "flood_water" {
        let (mesh, material) = flood
            .get_or_insert_with(|| {
                (
                    figures.meshes.add(flood_mesh()),
                    figures.materials.add(StandardMaterial {
                        base_color: Color::srgba(0.24, 0.36, 0.42, 0.62),
                        perceptual_roughness: 0.15,
                        alpha_mode: AlphaMode::Blend,
                        cull_mode: None,
                        ..default()
                    }),
                )
            })
            .clone();
        return Some(
            commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, 0.3 * ws, 0.0).with_scale(Vec3::splat(ws)),
                    Visibility::Inherited,
                ))
                .id(),
        );
    }
    let (path, yaw) = prop_model(data, root, id, 1)?;
    Some(spawn_model(
        commands,
        asset_server,
        metrics,
        &path,
        FitAxis::Native,
        ws,
        yaw,
    ))
}

/// Disc of flood water (mean radius 15 m, irregular outline), alpha 1 at the centre and 0 at the
/// edge (`FolkModels._flood_mesh`), in metres.
pub fn flood_mesh() -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology};
    const SEGMENTS: usize = 28;
    const RINGS: usize = 4;
    const RADIUS: f32 = 15.0;
    let mut positions = vec![[0.0, 0.0, 0.0]];
    let mut colors = vec![[1.0, 1.0, 1.0, 1.0]];
    for ring in 1..=RINGS {
        let t = ring as f32 / RINGS as f32;
        for i in 0..SEGMENTS {
            let a = std::f32::consts::TAU * i as f32 / SEGMENTS as f32;
            let wobble =
                0.78 + 0.22 * (a * 3.0 + 1.3).sin() * (a * 5.0).cos() + 0.1 * (a * 7.0 + 0.4).sin();
            let r = RADIUS * t * wobble;
            positions.push([a.cos() * r, 0.0, a.sin() * r]);
            colors.push([1.0, 1.0, 1.0, 1.0 - t * t]);
        }
    }
    let mut indices: Vec<u32> = Vec::new();
    for i in 0..SEGMENTS {
        indices.extend([0, (1 + i) as u32, (1 + (i + 1) % SEGMENTS) as u32]);
    }
    for ring in 1..RINGS {
        let inner = 1 + (ring - 1) * SEGMENTS;
        let outer = 1 + ring * SEGMENTS;
        for i in 0..SEGMENTS {
            let j = (i + 1) % SEGMENTS;
            let (a, b, c, d) = (
                (inner + i) as u32,
                (inner + j) as u32,
                (outer + i) as u32,
                (outer + j) as u32,
            );
            indices.extend([a, c, b, b, c, d]);
        }
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Sets the fade target of the scene roots: shown inside the zone, gone outside (never thinned).
pub fn scene_targets(
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut roots: Query<(&SceneRoot, &mut Fade)>,
) {
    if !view.ready {
        return;
    }
    let range = view.range_target(data.props.folk_range, data.props.folk_range_fade);
    for (root, mut fade) in &mut roots {
        let d = root.center.distance(view.focus);
        let reach = view.radius + root.footprint;
        let inside = if d <= reach { 1.0 } else { 0.0 };
        fade.target = inside * range;
    }
}

pub fn move_scene_movers(
    data: Res<LifeData>,
    view: Res<LifeView>,
    ground: GroundHeight,
    mut movers: Query<(&SceneMover, &mut LifeItem, &mut Transform)>,
) {
    let ws = world_scale(&data);
    for (m, mut item, mut t) in &mut movers {
        let (p, heading) = mover_pose(m, view.time);
        item.anchor = p;
        let bounce = if m.figure && m.speed_px > 0.0 {
            (view.time * 6.0 + m.phase * 20.0).sin().abs() * 0.04 * ws
        } else {
            0.0
        };
        let y = ground.at(p.x, p.y).unwrap_or(0.0);
        t.translation = Vec3::new(p.x - m.center.x, y + bounce, p.y - m.center.y);
        t.rotation = Quat::from_rotation_y(-heading.y.atan2(heading.x));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layouts() -> Value {
        let root = crate::paths::find_root().expect("root");
        let text = std::fs::read_to_string(
            crate::paths::data_dir(&root).join("rules/folk_scene_layouts.json"),
        )
        .unwrap();
        serde_json::from_str::<Value>(&text).unwrap()["scenes"].clone()
    }

    fn place(id: &str, province: &str, px: Vec2) -> crate::campaign::markers::places::Place {
        crate::campaign::markers::places::Place {
            id: id.into(),
            province: province.into(),
            kind: "town".into(),
            shown_kind: "town",
            name: id.into(),
            px,
            weight: 3,
            fortification: 0,
            controller: String::new(),
            rank: 1,
            family: String::new(),
            model: None,
            model_scale: 0.0,
            yaw: 0.0,
            radius: 0.4,
            landmark: None,
            absorbed: false,
            family_fallback: false,
            dn: None,
        }
    }

    fn places() -> Places {
        let mut p = Places::default();
        for (i, (id, prov, x)) in [
            ("set_a", "prov_x", 10.0),
            ("set_b", "prov_x", 50.0),
            ("set_c", "prov_y", 90.0),
        ]
        .into_iter()
        .enumerate()
        {
            p.places.push(place(id, prov, Vec2::new(x, 5.0)));
            p.index_by_id.insert(id.into(), i);
        }
        p
    }

    fn raw(province: &str, settlement: &str, kind: &str, intensity: f32) -> RawScene {
        RawScene {
            province: province.into(),
            settlement: settlement.into(),
            kind: kind.into(),
            intensity,
        }
    }

    #[test]
    fn scenes_resolve_to_named_settlement_then_seat() {
        let p = places();
        let l = layouts();
        let staged = resolve(
            &[
                raw("prov_x", "set_b", "fair", 0.5),
                raw("prov_x", "", "revolt", 1.0),
                raw("prov_none", "", "fair", 1.0),
                raw("prov_y", "", "nonsense", 1.0),
            ],
            &p,
            &l,
            &seats(&p),
        );
        assert_eq!(staged.scenes.len(), 2);
        assert_eq!(staged.skipped, 2);
        let fair = staged.scenes.iter().find(|s| s.kind == "fair").unwrap();
        assert_eq!(fair.settlement, "set_b");
        let revolt = staged.scenes.iter().find(|s| s.kind == "revolt").unwrap();
        assert_eq!(revolt.settlement, "set_a", "seat of the province");
    }

    #[test]
    fn duplicates_keep_the_strongest_intensity() {
        let p = places();
        let staged = resolve(
            &[
                raw("prov_x", "set_a", "fair", 0.3),
                raw("prov_x", "set_a", "fair", 0.9),
                raw("prov_x", "set_a", "fair", 0.5),
            ],
            &p,
            &layouts(),
            &seats(&p),
        );
        assert_eq!(staged.scenes.len(), 1);
        assert!((staged.scenes[0].intensity - 0.9).abs() < 1e-6);
    }

    #[test]
    fn effects_of_plague_revolt_and_famine() {
        let p = places();
        let staged = resolve(
            &[
                raw("prov_x", "set_a", "plague", 1.0),
                raw("prov_y", "", "revolt", 1.0),
                raw("prov_x", "set_b", "famine", 1.0),
            ],
            &p,
            &layouts(),
            &seats(&p),
        );
        assert!(staged.quiet_settlements.contains("set_a"));
        assert_eq!(staged.fire_points.len(), 2, "pyres and riots smoke");
        assert!(staged.idle_provinces.contains("prov_x"));
        // The pyre smoke stands beyond the edge of the settlement.
        let plague = staged.scenes.iter().find(|s| s.kind == "plague").unwrap();
        assert!(
            staged
                .fire_points
                .iter()
                .any(|f| f.distance(plague.center) > plague.footprint)
        );
    }

    #[test]
    fn forced_scenes_are_parsed() {
        let kinds = kinds_of(&layouts());
        let forced = parse_forced("prov_x:fair, prov_y:flood,prov_z:nope,broken", &kinds);
        assert_eq!(forced.len(), 2);
        assert_eq!(forced[1].kind, "flood");
    }

    #[test]
    fn figures_scale_with_intensity() {
        assert_eq!(figures_for(0.0, 4, 16), 4);
        assert_eq!(figures_for(1.0, 4, 16), 16);
        assert_eq!(figures_for(0.5, 4, 16), 10);
    }

    #[test]
    fn mover_goes_back_and_forth_with_offsets() {
        let m = SceneMover {
            from: Vec2::ZERO,
            to: Vec2::new(10.0, 0.0),
            phase: 0.0,
            speed_px: 1.0,
            lateral_px: 0.5,
            behind_px: 0.0,
            center: Vec2::ZERO,
            figure: true,
        };
        let (p, h) = mover_pose(&m, 5.0);
        assert!((p.x - 5.0).abs() < 1e-4 && h.x > 0.0);
        assert!((p.y - 0.5).abs() < 1e-4, "right of +X is +Y in map units");
        let (p, h) = mover_pose(&m, 15.0);
        assert!((p.x - 5.0).abs() < 1e-4 && h.x < 0.0);
        let still = SceneMover { speed_px: 0.0, ..m };
        assert_eq!(mover_pose(&still, 99.0).0, Vec2::new(0.0, 0.5));
    }

    #[test]
    fn resample_gives_equal_pieces() {
        let line = [Vec2::ZERO, Vec2::new(6.0, 0.0), Vec2::new(6.0, 3.0)];
        let r = resample(&line, 3);
        assert_eq!(r.len(), 4);
        assert!((r[1].x - 3.0).abs() < 1e-4 && (r[2].x - 6.0).abs() < 1e-4);
        assert_eq!(r[3], Vec2::new(6.0, 3.0));
    }

    #[test]
    fn flood_mesh_fades_to_the_edge() {
        let mesh = flood_mesh();
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(c)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("colours")
        };
        assert_eq!(c[0][3], 1.0);
        assert!(c.last().unwrap()[3] < 0.05);
    }

    #[test]
    fn rivers_points_are_spaced_and_sorted() {
        let mut data = LifeData::for_tests(Default::default());
        data.rivers = vec![(0..20).map(|i| Vec2::new(i as f32, 0.0)).collect()];
        let pts = river_points(&data, Vec2::new(5.0, 1.0), 6.0, 4, 2.0);
        assert!(pts.len() <= 4 && pts.len() >= 2);
        for w in pts.windows(2) {
            assert!(
                w[0].distance(Vec2::new(5.0, 1.0)) <= w[1].distance(Vec2::new(5.0, 1.0)) + 1e-4
            );
        }
        for i in 0..pts.len() {
            for j in 0..i {
                assert!(pts[i].distance(pts[j]) >= 2.0 - 1e-4);
            }
        }
    }
}
