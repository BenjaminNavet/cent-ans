//! The 1:1 towns at close zoom: streaming around the camera rig, plan, instancing, fade with the
//! zoom, ground height, growth from the simulation. Port of `town_layer.gd`, `town_builder.gd`
//! (blocks and kit houses, walls, monuments, ribbons) and the growth part of
//! `outbuilding_layer.gd`.
//!
//! - Active below `max_rig_distance` of the rig (`town_render.tres`), faded over the last
//!   `layer_fade_band`: the built town sinks into the ground (root scaled on Y) while the
//!   maquette of the markers lot (`campaign::markers`) stays visible; it can read
//!   [`TownsState::is_shown`] / [`TownsState::weight`] to sink its own model.
//! - Towns within the stream radius of the rig focus are planned (`plan::generate` on the
//!   resident relief) then spawned, one per frame at most; cached plans survive unloading.
//! - Below `detail_range` the houses are kit glTF models (instanced by Bevy: same mesh and
//!   material), beyond they are simple blocks.
//! - Height: `GroundHeight` at every object, re-applied when the relief exaggeration changes.
//!   The root is at y = 0, scaled `(s, s * weight, s)` with `s = 1 / metres_per_unit`: children
//!   are in local metres, uniform when the weight is 1.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::gltf::GltfAssetLabel;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use serde_json::{Value, json};

use super::data::{Town, TownDb};
use super::growth::{self, Instance, Piece};
use super::kit::Kit;
use super::landmark_plan::LandmarkPlan;
use super::landmark_v2::LandmarkLibrary;
use super::look::TownLook;
use super::plan::{self, HOUSE_KINDS, Plan};
use super::profile::TownRenderProfile;
use super::regions::{BuildingRegions, RegionStyle};
use super::rng::Rng;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::camera::zoom::ZoomView;
use crate::campaign::terrain::relief_state::ReliefState;
use crate::campaign::terrain::surface::{GroundHeight, Surface};
use crate::paths;
use crate::sim::Sim;

type WorldAsset = bevy::world_serialization::WorldAsset;

mod landmark;

/// Level of detail of a built town.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lod {
    /// Simple blocks.
    Blocks,
    /// Kit glTF models.
    Detail,
}

/// Growth shown on a town (from the simulation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Growth {
    pub quarters: u32,
    /// 0 none, 1 palisade, 2 stone (enclosure added beyond the 1340 plan).
    pub enclosure: u8,
}

#[derive(Resource)]
pub struct TownsWorld {
    pub db: Arc<TownDb>,
    pub kit: Arc<Kit>,
    pub profile: TownRenderProfile,
    pub look: TownLook,
    /// Settlements drawn as landmark cities (not here).
    pub skip: HashSet<String>,
    /// `CENT_ANS_TOWN_LOD=detail|blocks`.
    pub lod_override: Option<Lod>,
    /// `CENT_ANS_TOWNS_FORCE=1`: active whatever the camera distance (scripts).
    pub force_active: bool,
    /// `CENT_ANS_TOWN_GROWTH=<quarters>[,palisade|stone]`: forced growth (scripts).
    pub forced_growth: Option<Growth>,
    /// Regional house styles (`building_regions.json`).
    pub regions: Arc<BuildingRegions>,
    /// Emblematic 1:1 cities (`data/landmarks_v2`), by settlement id.
    pub landmarks: Arc<LandmarkLibrary>,
    /// Map-unit anchor of each landmark city (its settlement in `settlements_px.json`).
    pub landmark_px: HashMap<String, Vec2>,
}

#[derive(Resource)]
struct TownMeshes {
    cube: Handle<Mesh>,
    /// Unit house block with a gable roof.
    house: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    blocks: Vec<Handle<StandardMaterial>>,
    masonry: Handle<StandardMaterial>,
    planks: Handle<StandardMaterial>,
    street: Handle<StandardMaterial>,
    ground: Handle<StandardMaterial>,
    /// White, double sided: monument meshes carry their colours in the vertices.
    vertex: Handle<StandardMaterial>,
}

/// Counters of what a town spawned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub houses: usize,
    pub kit_models: usize,
    pub blocks: usize,
    pub monuments: usize,
    pub wall_pieces: usize,
    pub towers: usize,
    pub gates: usize,
    pub suburb_houses: usize,
    pub enclosure_pieces: usize,
    pub props: usize,
    pub entities: usize,
}

struct Shown {
    root: Entity,
    lod: Lod,
    growth: Growth,
    plan: Arc<Plan>,
    ribbons: [Handle<Mesh>; 2],
    counts: Counts,
    /// Map-unit anchor of the town.
    anchor: Vec2,
    /// Extra of an emblematic city (wall ring meshes, plan).
    landmark: Option<landmark::LmShown>,
    /// Merged garden trees (detail only).
    trees: Option<Handle<Mesh>>,
}

#[derive(Resource, Default)]
pub struct TownsState {
    pub active: bool,
    /// Fade weight of the 1:1 layer [0, 1].
    pub weight: f32,
    shown: HashMap<String, Shown>,
    plans: HashMap<String, Arc<Plan>>,
    landmark_plans: HashMap<String, Arc<LandmarkPlan>>,
    order: VecDeque<String>,
    scenes: HashMap<String, Handle<WorldAsset>>,
    relief_sig: [f32; 3],
    last_settle: f32,
    last_growth_check: f32,
    plans_built: usize,
    towns_spawned: usize,
    plan_ms_max: f32,
    spawn_ms_max: f32,
    reposed: usize,
}

impl TownsState {
    /// True when the 1:1 town of settlement `id` is built and shown.
    pub fn is_shown(&self, id: &str) -> bool {
        self.active && self.shown.contains_key(id)
    }

    /// Ids of the built towns.
    pub fn shown_ids(&self) -> Vec<&str> {
        self.shown.keys().map(String::as_str).collect()
    }

    pub fn weight(&self) -> f32 {
        self.weight
    }
}

#[derive(Component)]
pub struct TownRoot {
    pub id: String,
}

/// An object standing on the ground: its height is re-read from `GroundHeight`.
#[derive(Component, Clone, Copy)]
struct TownGround {
    anchor: Vec2,
    local: Vec2,
    /// Direction of the model X axis (plan convention), radians.
    yaw: f32,
    /// Half footprint (m), used for the lowest point.
    half: Vec2,
    lift_m: f32,
    sink_m: f32,
    /// Ground sampled here instead of at `local` (bridge parts stand on the water level).
    sample: Option<Vec2>,
}

pub struct CampaignTownsPlugin;

impl Plugin for CampaignTownsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TownsState>()
            .register_dump(dump_towns)
            .add_systems(OnEnter(AppState::Campaign), init_towns)
            .add_systems(OnExit(AppState::Campaign), reset_towns)
            .add_systems(
                Update,
                (stream_towns, fade_towns, settle_towns)
                    .chain()
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<TownsWorld>),
            );
        super::outbuildings::add(app);
    }
}

/// `towns_1340.json` anchors predate the composed map; the settlements (and their maquettes) are
/// at `settlements_px.json`, the live positions. Town anchors follow their settlement.
fn realign_to_settlements(db: &mut TownDb, data_dir: &std::path::Path) {
    let Some(px) = std::fs::read_to_string(data_dir.join("map/settlements_px.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<HashMap<String, [f32; 2]>>(&t).ok())
    else {
        return;
    };
    for (id, town) in &mut db.towns {
        if let Some(p) = px.get(id) {
            town.px = Vec2::new(p[0], p[1]);
        }
    }
}

fn parse_growth_env(text: &str) -> Option<Growth> {
    let mut parts = text.split(',');
    let quarters = parts.next()?.trim().parse().ok()?;
    let enclosure = match parts.next().map(str::trim) {
        Some("palisade") => 1,
        Some("stone") => 2,
        _ => 0,
    };
    Some(Growth {
        quarters,
        enclosure,
    })
}

fn init_towns(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<TownsState>,
) {
    *state = TownsState::default();
    let Ok(root) = paths::find_root() else { return };
    let data_dir = paths::data_dir(&root);
    let assets_dir = paths::asset_root(&root);
    let mut db = TownDb::load(&data_dir);
    realign_to_settlements(&mut db, &data_dir);
    let kit = Kit::load(&assets_dir);
    let look = TownLook::load(&data_dir);
    let profile = TownRenderProfile::load(&root);
    let mut skip = HashSet::new();
    if let Ok(dir) = std::fs::read_dir(data_dir.join("landmarks_v2")) {
        for entry in dir.filter_map(Result::ok) {
            if let Some(id) = std::fs::read_to_string(entry.path())
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .and_then(|v| v["settlement"].as_str().map(String::from))
            {
                skip.insert(id);
            }
        }
    }
    let mut mat = |c: [f32; 3], rough: f32| {
        materials.add(StandardMaterial {
            base_color: Color::srgb(c[0], c[1], c[2]),
            perceptual_roughness: rough,
            double_sided: true,
            cull_mode: None,
            ..default()
        })
    };
    let shared = TownMeshes {
        cube: meshes
            .add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)).translated_by(Vec3::new(0.0, 0.5, 0.0))),
        house: meshes.add(super::shapes::house_block_mesh()),
        cylinder: meshes.add(
            Cylinder::new(1.0, 1.0)
                .mesh()
                .resolution(14)
                .build()
                .translated_by(Vec3::new(0.0, 0.5, 0.0)),
        ),
        blocks: look
            .block_colors
            .iter()
            .map(|c| mat(*c, 0.95))
            .collect::<Vec<_>>(),
        masonry: mat(look.masonry, 0.9),
        planks: mat(look.planks, 0.95),
        street: mat(look.street, 1.0),
        ground: mat(look.ground, 1.0),
        vertex: mat([1.0, 1.0, 1.0], 0.92),
    };
    let landmarks = LandmarkLibrary::load(&data_dir);
    let landmark_px = landmark::landmark_anchors(&landmarks, &data_dir);
    commands.insert_resource(shared);
    commands.insert_resource(TownsWorld {
        landmarks: Arc::new(landmarks),
        landmark_px,
        db: Arc::new(db),
        kit: Arc::new(kit),
        profile,
        look,
        skip,
        lod_override: match std::env::var("CENT_ANS_TOWN_LOD").as_deref() {
            Ok("detail") => Some(Lod::Detail),
            Ok("blocks") => Some(Lod::Blocks),
            _ => None,
        },
        force_active: std::env::var("CENT_ANS_TOWNS_FORCE").is_ok_and(|v| v == "1"),
        regions: Arc::new(BuildingRegions::load(&data_dir)),
        forced_growth: std::env::var("CENT_ANS_TOWN_GROWTH")
            .ok()
            .and_then(|t| parse_growth_env(&t)),
    });
}

fn reset_towns(mut commands: Commands, mut state: ResMut<TownsState>) {
    *state = TownsState::default();
    commands.remove_resource::<TownsWorld>();
    commands.remove_resource::<TownMeshes>();
}

// --- Growth from the simulation ---------------------------------------------------------------

fn growth_of(sim: &Sim, world: &TownsWorld, town: &Town) -> Growth {
    if let Some(forced) = world.forced_growth {
        return forced;
    }
    let (Some(data), Some(state)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return Growth::default();
    };
    let Ok(sid) = data_model::SettlementId::new(&town.id) else {
        return Growth::default();
    };
    let (Some(settlement), Some(live)) = (data.settlements.get(&sid), state.settlement_state(&sid))
    else {
        return Growth::default();
    };
    let ratio = {
        let now = state
            .province_state(&settlement.province)
            .map_or(0.0, |p| p.population.total() as f32);
        let then = data
            .provinces
            .get(&settlement.province)
            .map_or(0.0, |p| p.population.classes.total() as f32);
        if now > 0.0 && then > 0.0 {
            now / then
        } else {
            1.0
        }
    };
    let cfg = &world.db.growth;
    let built: Vec<String> = live
        .buildings
        .iter()
        .map(|b| b.as_str().to_string())
        .collect();
    let initial: Vec<String> = settlement
        .buildings
        .iter()
        .map(|b| b.as_str().to_string())
        .collect();
    let enclosure = growth::enclosure_to_add(cfg, &built, &initial, &town.walls)
        .and_then(|k| growth::KINDS.iter().position(|x| *x == k))
        .unwrap_or(0) as u8;
    Growth {
        quarters: growth::quarter_count(cfg, ratio),
        enclosure,
    }
}

// --- Ground and fade --------------------------------------------------------------------------

/// Displayed height (local metres) at local point `l` of the town anchored at `anchor`.
/// Regional house style of the province of town `id`.
fn style_of_town(sim: &Sim, world: &TownsWorld, id: &str) -> Option<RegionStyle> {
    let data = sim.data.as_ref()?;
    let settlement = data
        .settlements
        .get(&data_model::SettlementId::new(id).ok()?)?;
    let province = data.provinces.get(&settlement.province)?;
    world
        .regions
        .style_for(settlement.province.as_str(), &province.region)
}

fn local_height(ground: &GroundHeight, anchor: Vec2, scale: f32, l: Vec2) -> f32 {
    ground
        .at(anchor.x + l.x * scale, anchor.y + l.y * scale)
        .unwrap_or(0.0)
        / scale
}

/// Lowest displayed point under a footprint (centre and four corners), then sunk and lifted.
fn footing_y(y_of: &dyn Fn(Vec2) -> f32, g: &TownGround) -> f32 {
    let mut low = y_of(g.sample.unwrap_or(g.local));
    if g.half.x > 0.0 || g.half.y > 0.0 {
        let u = Vec2::new(g.yaw.cos(), g.yaw.sin());
        let v = Vec2::new(-u.y, u.x);
        for (a, b) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            low = low.min(y_of(g.local + u * (g.half.x * a) + v * (g.half.y * b)));
        }
    }
    low - g.sink_m + g.lift_m
}

fn root_scale(scale: f32, weight: f32) -> Vec3 {
    Vec3::new(scale, scale * weight.max(0.001), scale)
}

fn fade_towns(
    state: Res<TownsState>,
    world: Res<TownsWorld>,
    mut roots: Query<(&mut Transform, &mut Visibility), With<TownRoot>>,
) {
    let scale = 1.0 / world.db.meters_per_unit;
    for shown in state.shown.values() {
        if let Ok((mut t, mut vis)) = roots.get_mut(shown.root) {
            let target = root_scale(scale, state.weight);
            if t.scale != target {
                t.scale = target;
            }
            let want = if state.weight < 0.01 {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            if *vis != want {
                *vis = want;
            }
        }
    }
}

/// Re-applies the displayed ground height when the relief exaggeration (camera zoom) changes,
/// and rebuilds the draped ribbons.
fn settle_towns(
    time: Res<Time>,
    world: Res<TownsWorld>,
    shared: Res<TownMeshes>,
    mut state: ResMut<TownsState>,
    relief: Option<Res<ReliefState>>,
    ground: GroundHeight,
    mut meshes: ResMut<Assets<Mesh>>,
    mut objects: Query<(&TownGround, &mut Transform)>,
) {
    let Some(relief) = relief else { return };
    if !ground.is_ready() || state.shown.is_empty() {
        return;
    }
    let sig = [relief.vertical_scale, relief.gain, relief.squash];
    let now = time.elapsed_secs();
    if sig == state.relief_sig || now - state.last_settle < 0.2 {
        return;
    }
    state.relief_sig = sig;
    state.last_settle = now;
    let scale = 1.0 / world.db.meters_per_unit;
    let mut count = 0;
    for (g, mut t) in &mut objects {
        let y_of = |l: Vec2| local_height(&ground, g.anchor, scale, l);
        let y = footing_y(&y_of, g);
        if (t.translation.y - y).abs() > 1e-3 {
            t.translation.y = y;
        }
        count += 1;
    }
    state.reposed = count;
    let _ = &shared;
    for shown in state.shown.values() {
        let y_of = |l: Vec2| local_height(&ground, shown.anchor, scale, l);
        if let Some(mut m) = meshes.get_mut(&shown.ribbons[0]) {
            *m = street_mesh(&shown.plan, &y_of, world.look.ribbon_lift_world / scale);
        }
        if let Some(mut m) = meshes.get_mut(&shown.ribbons[1]) {
            *m = ground_mesh(&shown.plan, &y_of, world.look.ground_lift_world / scale);
        }
        if let Some(handle) = &shown.trees
            && let Some(mut m) = meshes.get_mut(handle)
        {
            *m = super::shapes::trees_mesh(&shown.plan.trees, &y_of);
        }
        if let Some(lm) = &shown.landmark {
            for (ring, handle) in lm.plan.wall_rings.iter().zip(&lm.walls) {
                if let Some(mut m) = meshes.get_mut(handle) {
                    *m = landmark::wall_ring_mesh(ring, &y_of);
                }
            }
        }
    }
}

// --- Streaming --------------------------------------------------------------------------------

fn stream_towns(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<TownsWorld>,
    shared: Res<TownMeshes>,
    mut state: ResMut<TownsState>,
    view: Res<ZoomView>,
    rig: Option<Res<CampaignRig>>,
    surface: Option<Res<Surface>>,
    ground: GroundHeight,
    sim: Res<Sim>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let (Some(rig), Some(surface)) = (rig, surface) else {
        return;
    };
    if !ground.is_ready() {
        return;
    }
    let profile = &world.profile;
    let distance = view.distance;
    let limit = profile.max_rig_distance
        * (1.0
            + if state.active {
                profile.rig_hysteresis
            } else {
                0.0
            });
    state.active = world.force_active || distance < limit;
    state.weight = if world.force_active {
        1.0
    } else {
        profile.layer_weight(distance)
    };
    let despawn = |commands: &mut Commands, shown: Shown| {
        commands.entity(shown.root).despawn();
    };
    if !state.active {
        let ids: Vec<String> = state.shown.keys().cloned().collect();
        for id in ids {
            if let Some(shown) = state.shown.remove(&id) {
                despawn(&mut commands, shown);
            }
        }
        return;
    }
    let focus = Vec2::new(rig.focus.x, rig.focus.z);
    let radius = profile.stream_radius(distance);
    let lod = world
        .lod_override
        .unwrap_or(if distance < profile.detail_range {
            Lod::Detail
        } else {
            Lod::Blocks
        });
    let now = time.elapsed_secs();
    let check_growth = now - state.last_growth_check > 0.5;
    if check_growth {
        state.last_growth_check = now;
    }
    // Unload far towns; rebuild those whose detail level or growth changed.
    let ids: Vec<String> = state.shown.keys().cloned().collect();
    for id in ids {
        let town = world.db.towns.get(&id);
        let far = state.shown[&id].anchor.distance(focus) > radius * profile.unload_factor;
        let stale = {
            let shown = &state.shown[&id];
            shown.lod != lod
                || (check_growth
                    && town.is_some_and(|t| shown.growth != growth_of(&sim, &world, t)))
        };
        if (far || stale)
            && let Some(shown) = state.shown.remove(&id)
        {
            despawn(&mut commands, shown);
        }
    }
    // Build the nearest missing town (one per frame at most).
    let mut wanted: Vec<(&Town, f32)> = world
        .db
        .towns
        .values()
        .filter(|t| !world.skip.contains(&t.id) && !state.shown.contains_key(&t.id))
        .map(|t| (t, t.px.distance(focus)))
        .filter(|(_, d)| *d < radius)
        .collect();
    wanted.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.id.cmp(&b.0.id)));
    let room = profile.plan_cache.saturating_sub(state.shown.len());
    let budget = profile.max_plan_jobs.clamp(1, 2).min(room);
    for (town, _) in wanted.into_iter().take(budget) {
        let started = std::time::Instant::now();
        let scale = 1.0 / world.db.meters_per_unit;
        let plan = match state.plans.get(&town.id) {
            Some(p) => p.clone(),
            None => {
                let mpu = world.db.meters_per_unit;
                let h = |p: Vec2| surface.height_m_at(town.px.x + p.x / mpu, town.px.y + p.y / mpu);
                let p = Arc::new(plan::generate(town, &world.db, &h));
                state.plans.insert(town.id.clone(), p.clone());
                state.order.push_back(town.id.clone());
                while state.order.len() > profile.plan_cache * 2 {
                    if let Some(old) = state.order.pop_front() {
                        state.plans.remove(&old);
                    }
                }
                state.plans_built += 1;
                state.plan_ms_max = state
                    .plan_ms_max
                    .max(started.elapsed().as_secs_f32() * 1000.0);
                p
            }
        };
        let spawn_started = std::time::Instant::now();
        let growth = growth_of(&sim, &world, town);
        let y_of = |l: Vec2| local_height(&ground, town.px, scale, l);
        let mut scenes = std::mem::take(&mut state.scenes);
        let (root, ribbons, counts) = spawn_town(
            &mut commands,
            &assets,
            &mut meshes,
            &shared,
            &world,
            &mut scenes,
            town,
            &plan,
            lod,
            growth,
            state.weight,
            &y_of,
            style_of_town(&sim, &world, &town.id),
        );
        state.scenes = scenes;
        let trees = spawn_trees(
            &mut commands,
            &mut meshes,
            &shared,
            root,
            &plan,
            lod == Lod::Detail,
            &y_of,
        );
        state.towns_spawned += 1;
        state.spawn_ms_max = state
            .spawn_ms_max
            .max(spawn_started.elapsed().as_secs_f32() * 1000.0);
        state.shown.insert(
            town.id.clone(),
            Shown {
                root,
                lod,
                growth,
                plan,
                ribbons,
                counts,
                anchor: town.px,
                landmark: None,
                trees,
            },
        );
    }
    landmark::stream_landmarks(
        &mut commands,
        &assets,
        &mut meshes,
        &shared,
        &world,
        &mut state,
        &surface,
        &ground,
        &sim,
        focus,
        radius,
        lod,
    );
}

// --- Meshes of the ribbons ----------------------------------------------------------------------

fn flat_mesh(positions: Vec<[f32; 3]>, uvs: Vec<[f32; 2]>, indices: Vec<u32>) -> Mesh {
    let n = positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// All the streets as draped ribbons (local metres).
pub fn street_mesh(plan: &Plan, y_of: &dyn Fn(Vec2) -> f32, lift_m: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    for s in &plan.streets {
        let n = s.points.len();
        if n < 2 {
            continue;
        }
        let half = s.width * 0.5;
        let base = positions.len() as u32;
        let mut along = 0.0;
        for i in 0..n {
            let t =
                (s.points[(i + 1).min(n - 1)] - s.points[i.saturating_sub(1)]).normalize_or_zero();
            let normal = Vec2::new(-t.y, t.x) * half;
            let (l, r) = (s.points[i] + normal, s.points[i] - normal);
            positions.push([l.x, y_of(l) + lift_m, l.y]);
            positions.push([r.x, y_of(r) + lift_m, r.y]);
            if i > 0 {
                along += s.points[i].distance(s.points[i - 1]);
            }
            uvs.push([0.0, along / 8.0]);
            uvs.push([1.0, along / 8.0]);
        }
        for i in 0..(n as u32 - 1) {
            let a = base + i * 2;
            indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        }
    }
    flat_mesh(positions, uvs, indices)
}

/// The built core's ground (yards, gardens, beaten earth), draped.
pub fn ground_mesh(plan: &Plan, y_of: &dyn Fn(Vec2) -> f32, lift_m: f32) -> Mesh {
    let g = &plan.ground;
    let mut positions = Vec::with_capacity(g.n * g.n);
    let mut uvs = Vec::with_capacity(g.n * g.n);
    for j in 0..g.n {
        for i in 0..g.n {
            let p = g.origin + Vec2::new(i as f32, j as f32) * g.step;
            let y = if g.inside[j * g.n + i] {
                y_of(p) + lift_m
            } else {
                0.0
            };
            positions.push([p.x, y, p.y]);
            uvs.push([p.x / 10.0, p.y / 10.0]);
        }
    }
    let mut indices = Vec::new();
    for j in 0..g.n.saturating_sub(1) {
        for i in 0..g.n.saturating_sub(1) {
            let k = |a: usize, b: usize| (b * g.n + a) as u32;
            if g.inside[j * g.n + i]
                && g.inside[j * g.n + i + 1]
                && g.inside[(j + 1) * g.n + i]
                && g.inside[(j + 1) * g.n + i + 1]
            {
                indices.extend_from_slice(&[
                    k(i, j),
                    k(i, j + 1),
                    k(i + 1, j),
                    k(i + 1, j),
                    k(i, j + 1),
                    k(i + 1, j + 1),
                ]);
            }
        }
    }
    flat_mesh(positions, uvs, indices)
}

// --- Spawning -----------------------------------------------------------------------------------

/// Garden trees of a plan, merged in one mesh (detail only), standing on the displayed ground.
fn spawn_trees(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    shared: &TownMeshes,
    root: Entity,
    plan: &Plan,
    detail: bool,
    y_of: &dyn Fn(Vec2) -> f32,
) -> Option<Handle<Mesh>> {
    if !detail || plan.trees.is_empty() {
        return None;
    }
    let handle = meshes.add(super::shapes::trees_mesh(&plan.trees, y_of));
    commands.spawn((
        Name::new("Trees"),
        Mesh3d(handle.clone()),
        MeshMaterial3d(shared.vertex.clone()),
        NotShadowCaster,
        Transform::IDENTITY,
        ChildOf(root),
    ));
    Some(handle)
}

struct Spawner<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    assets: &'a AssetServer,
    shared: &'a TownMeshes,
    world: &'a TownsWorld,
    scenes: &'a mut HashMap<String, Handle<WorldAsset>>,
    root: Entity,
    anchor: Vec2,
    y_of: &'a dyn Fn(Vec2) -> f32,
    counts: Counts,
    /// Regional house style of the town's province.
    style: Option<RegionStyle>,
    /// Bridge parts: ground sampled at this point, with this extra lift (m) added.
    fixed: Option<(Vec2, f32)>,
}

impl Spawner<'_, '_, '_> {
    fn ground(&self, local: Vec2, yaw: f32, half: Vec2, lift_m: f32) -> TownGround {
        if let Some((sample, extra)) = self.fixed {
            return TownGround {
                anchor: self.anchor,
                local,
                yaw,
                half: Vec2::ZERO,
                lift_m: lift_m + extra,
                sink_m: 0.0,
                sample: Some(sample),
            };
        }
        TownGround {
            anchor: self.anchor,
            local,
            yaw,
            half,
            lift_m,
            sink_m: self.world.look.foundation_sink_m,
            sample: None,
        }
    }

    fn scene(&mut self, model: &str) -> Handle<WorldAsset> {
        if let Some(h) = self.scenes.get(model) {
            return h.clone();
        }
        let handle = self
            .assets
            .load(GltfAssetLabel::Scene(0).from_asset(Kit::asset_path(model)));
        self.scenes.insert(model.to_string(), handle.clone());
        handle
    }

    /// Kit model at a footprint (plan convention yaw).
    #[allow(clippy::too_many_arguments)]
    fn kit_piece(
        &mut self,
        model: &str,
        local: Vec2,
        yaw: f32,
        scale: Vec3,
        half: Vec2,
        lift_m: f32,
    ) {
        let g = self.ground(local, yaw, half, lift_m);
        let y = footing_y(self.y_of, &g);
        let handle = self.scene(model);
        self.commands.spawn((
            WorldAssetRoot(handle),
            Transform::from_translation(Vec3::new(local.x, y, local.y))
                .with_rotation(Quat::from_rotation_y(-yaw))
                .with_scale(scale),
            g,
            ChildOf(self.root),
        ));
        self.counts.kit_models += 1;
        self.counts.entities += 1;
    }

    /// Box or cylinder (unit meshes, base at y = 0) scaled to a block.
    #[allow(clippy::too_many_arguments)]
    fn block(
        &mut self,
        cylinder: bool,
        material: Handle<StandardMaterial>,
        local: Vec2,
        yaw: f32,
        size: Vec3,
        lift_m: f32,
    ) {
        let mesh = if cylinder {
            self.shared.cylinder.clone()
        } else {
            self.shared.cube.clone()
        };
        self.place_block(mesh, cylinder, material, local, yaw, size, lift_m);
    }

    /// House block with a gable roof (prism), ridge along the facade (X).
    fn house_block(
        &mut self,
        material: Handle<StandardMaterial>,
        local: Vec2,
        yaw: f32,
        size: Vec3,
        lift_m: f32,
    ) {
        let mesh = self.shared.house.clone();
        self.place_block(mesh, false, material, local, yaw, size, lift_m);
    }

    #[allow(clippy::too_many_arguments)]
    fn place_block(
        &mut self,
        mesh: Handle<Mesh>,
        cylinder: bool,
        material: Handle<StandardMaterial>,
        local: Vec2,
        yaw: f32,
        size: Vec3,
        lift_m: f32,
    ) {
        let half = if cylinder {
            Vec2::ZERO
        } else {
            Vec2::new(size.x, size.z) * 0.5
        };
        let g = self.ground(local, yaw, half, lift_m);
        let y = footing_y(self.y_of, &g);
        self.commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(Vec3::new(local.x, y, local.y))
                .with_rotation(Quat::from_rotation_y(-yaw))
                .with_scale(size),
            NotShadowCaster,
            g,
            ChildOf(self.root),
        ));
        self.counts.blocks += 1;
        self.counts.entities += 1;
    }

    fn block_material(&self, tint: f32) -> Handle<StandardMaterial> {
        let n = self.shared.blocks.len();
        self.shared.blocks[((tint * n as f32) as usize).min(n - 1)].clone()
    }

    fn mean_height(&self, kind: &str) -> f32 {
        let models = self.world.kit.models_of(kind);
        if models.is_empty() {
            return self.world.look.default_block_height_m;
        }
        models.iter().map(|m| m.height).sum::<f32>() / models.len() as f32
    }

    fn masonry(&self) -> Handle<StandardMaterial> {
        self.shared.masonry.clone()
    }

    fn planks(&self) -> Handle<StandardMaterial> {
        self.shared.planks.clone()
    }
}

/// One house-like building: kit model in detail, block otherwise.
#[allow(clippy::too_many_arguments)]
fn spawn_building(
    sp: &mut Spawner,
    detail: bool,
    kind: &str,
    local: Vec2,
    yaw: f32,
    front: f32,
    depth: f32,
    tint: f32,
    rng: &mut Rng,
    masonry: bool,
) {
    let half = Vec2::new(front, depth) * 0.5;
    if detail
        && let Some(entry) = sp
            .world
            .kit
            .pick_styled(kind, front, depth, rng, sp.style.as_ref())
            .cloned()
    {
        let scale = sp.world.kit.fit_scale(&entry.name, front, depth);
        sp.kit_piece(&entry.name, local, yaw, scale, half, 0.0);
        return;
    }
    let height = sp.mean_height(kind) * (1.0 + (tint - 0.5) * sp.world.look.block_height_jitter);
    let material = if masonry {
        sp.masonry()
    } else {
        sp.block_material(tint)
    };
    sp.house_block(material, local, yaw, Vec3::new(front, height, depth), 0.0);
}

#[allow(clippy::too_many_arguments)]
fn spawn_town(
    commands: &mut Commands,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    shared: &TownMeshes,
    world: &TownsWorld,
    scenes: &mut HashMap<String, Handle<WorldAsset>>,
    town: &Town,
    plan: &Plan,
    lod: Lod,
    growth: Growth,
    weight: f32,
    y_of: &dyn Fn(Vec2) -> f32,
    style: Option<RegionStyle>,
) -> (Entity, [Handle<Mesh>; 2], Counts) {
    let scale = 1.0 / world.db.meters_per_unit;
    let look = &world.look;
    let detail = lod == Lod::Detail;
    let root = commands
        .spawn((
            Name::new(format!("Town_{}", town.id)),
            TownRoot {
                id: town.id.clone(),
            },
            Transform::from_translation(Vec3::new(town.px.x, 0.0, town.px.y))
                .with_scale(root_scale(scale, weight)),
            Visibility::Inherited,
            DespawnOnExit(AppState::Campaign),
        ))
        .id();
    let street = meshes.add(street_mesh(plan, y_of, look.ribbon_lift_world / scale));
    let ground_handle = meshes.add(ground_mesh(plan, y_of, look.ground_lift_world / scale));
    commands.spawn((
        Name::new("Streets"),
        Mesh3d(street.clone()),
        MeshMaterial3d(shared.street.clone()),
        NotShadowCaster,
        Transform::IDENTITY,
        ChildOf(root),
    ));
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(ground_handle.clone()),
        MeshMaterial3d(shared.ground.clone()),
        NotShadowCaster,
        Transform::IDENTITY,
        ChildOf(root),
    ));
    let mut sp = Spawner {
        commands,
        assets,
        shared,
        world,
        scenes,
        root,
        anchor: town.px,
        y_of,
        style,
        fixed: None,
        counts: Counts {
            entities: 3,
            ..Default::default()
        },
    };
    let seed = town.seed;
    // Houses of the plan.
    for (i, h) in plan.houses.iter().enumerate() {
        let mut rng = Rng::new(seed ^ (i as u32).wrapping_mul(0x9E37_79B1));
        spawn_building(
            &mut sp,
            detail,
            HOUSE_KINDS[h.kind],
            h.pos,
            h.yaw,
            h.front,
            h.depth,
            h.tint,
            &mut rng,
            false,
        );
        sp.counts.houses += 1;
    }
    // Monuments.
    for (i, m) in plan.monuments.iter().enumerate() {
        let mut rng = Rng::new(seed ^ 0x5151 ^ i as u32);
        match m.kind.as_str() {
            "castle" => {
                let k = &look.keep;
                let mat = sp.masonry();
                sp.block(
                    false,
                    mat.clone(),
                    m.pos,
                    m.yaw,
                    Vec3::new(k.side_m, k.height_m, k.side_m),
                    0.0,
                );
                let u = Vec2::new(m.yaw.cos(), m.yaw.sin());
                let v = Vec2::new(-u.y, u.x);
                for (a, b) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                    let p = m.pos + u * (m.length * 0.5 * a) + v * (m.depth * 0.5 * b);
                    let r = k.tower_radius_m;
                    sp.block(
                        true,
                        mat.clone(),
                        p,
                        0.0,
                        Vec3::new(r, k.tower_height_m, r),
                        0.0,
                    );
                }
            }
            kind => {
                let kit_kind = match kind {
                    "cathedral" | "abbey" | "church" => "church",
                    other => other,
                };
                spawn_building(
                    &mut sp, detail, kit_kind, m.pos, m.yaw, m.length, m.depth, 0.5, &mut rng, true,
                );
            }
        }
        sp.counts.monuments += 1;
    }
    // Walls, towers and gates of the plan.
    if !plan.wall_ring.is_empty() {
        let masonry = plan.walls == "stone";
        let mat = if masonry { sp.masonry() } else { sp.planks() };
        let ring = &plan.wall_ring;
        for i in 1..ring.len() {
            if plan.wall_gaps[i] || plan.wall_gaps[i - 1] {
                continue;
            }
            let (a, b) = (ring[i - 1], ring[i]);
            let d = b - a;
            let yaw = d.y.atan2(d.x);
            sp.block(
                false,
                mat.clone(),
                (a + b) * 0.5,
                yaw,
                Vec3::new(
                    d.length() + 0.5,
                    plan.wall_height + 2.0,
                    plan.wall_thickness,
                ),
                -2.0,
            );
            sp.counts.wall_pieces += 1;
        }
        for t in &plan.towers {
            sp.block(
                true,
                mat.clone(),
                t.pos,
                0.0,
                Vec3::new(t.radius, t.height + 2.0, t.radius),
                -2.0,
            );
            sp.counts.towers += 1;
        }
        for g in &plan.gates {
            sp.block(
                false,
                mat.clone(),
                g.pos,
                g.yaw,
                Vec3::new(world.db.gate_depth_m, g.height + 2.0, 11.0),
                -2.0,
            );
            sp.counts.gates += 1;
        }
    }
    // Market furniture (kit models only).
    if detail {
        let market = &plan.market;
        let area = market.length * market.width;
        let n = ((area / look.market.area_m2_per_prop) as usize).min(look.market.max_props);
        let u = Vec2::new(market.yaw.cos(), market.yaw.sin());
        let v = Vec2::new(-u.y, u.x);
        let mut rng = Rng::new(seed ^ 0xA11CE);
        if look.market.well
            && let Some(w) = world.kit.prop_model("well", seed)
        {
            let name = w.name.clone();
            sp.kit_piece(&name, market.pos, 0.0, Vec3::ONE, Vec2::splat(1.0), 0.0);
            sp.counts.props += 1;
        }
        for k in 0..n {
            if look.market.kinds.is_empty() {
                break;
            }
            let kind = &look.market.kinds[k % look.market.kinds.len()];
            let Some(model) = world.kit.prop_model(kind, seed.wrapping_add(k as u32)) else {
                continue;
            };
            let name = model.name.clone();
            let off =
                u * rng.range(-0.4, 0.4) * market.length + v * rng.range(-0.4, 0.4) * market.width;
            if off.length() < 4.0 {
                continue;
            }
            sp.kit_piece(
                &name,
                market.pos + off,
                rng.range(0.0, std::f32::consts::TAU),
                Vec3::ONE,
                Vec2::splat(1.5),
                0.0,
            );
            sp.counts.props += 1;
        }
    }
    // Growth: suburb quarters and added enclosure.
    let mut extra: Vec<Instance> =
        growth::suburb_instances(&world.db.growth, town, &world.kit, growth.quarters);
    sp.counts.suburb_houses = extra.len();
    if growth.enclosure > 0 {
        let kind = growth::KINDS[growth.enclosure as usize];
        let enc = growth::enclosure_instances(
            &world.db.growth,
            &world.db.wall_spec(kind),
            world.db.gate_depth_m,
            town,
            kind,
        );
        sp.counts.enclosure_pieces = enc.len();
        extra.extend(enc);
    }
    for (i, inst) in extra.iter().enumerate() {
        let yaw = -inst.rot_y;
        match &inst.piece {
            Piece::Kit(model) => {
                let entry = world.kit.get(model).cloned();
                let (front, depth) = entry.as_ref().map_or((10.0, 7.0), |e| (e.length, e.depth));
                let mut rng = Rng::new(seed ^ 0x77 ^ i as u32);
                if detail && entry.is_some() {
                    sp.kit_piece(
                        model,
                        inst.pos,
                        yaw,
                        inst.scale,
                        Vec2::new(front, depth) * 0.5,
                        inst.lift,
                    );
                } else {
                    let height = entry
                        .as_ref()
                        .map_or(look.default_block_height_m, |e| e.height);
                    let material = sp.block_material(inst.tint);
                    sp.house_block(
                        material,
                        inst.pos,
                        yaw,
                        Vec3::new(front, height, depth),
                        inst.lift,
                    );
                }
                let _ = &mut rng;
            }
            Piece::Box { masonry } => {
                let mat = if *masonry { sp.masonry() } else { sp.planks() };
                sp.block(false, mat, inst.pos, yaw, inst.scale, inst.lift);
            }
            Piece::Tower => {
                let mat = sp.masonry();
                sp.block(true, mat, inst.pos, yaw, inst.scale, inst.lift);
            }
        }
    }
    let counts = sp.counts;
    (root, [street, ground_handle], counts)
}

// --- Dump -----------------------------------------------------------------------------------

fn dump_towns(world: &World) -> (String, Value) {
    let Some(state) = world.get_resource::<TownsState>() else {
        return ("campaign_towns".into(), json!({"ready": false}));
    };
    let Some(w) = world.get_resource::<TownsWorld>() else {
        return ("campaign_towns".into(), json!({"ready": false}));
    };
    let mut ids: Vec<&String> = state.shown.keys().collect();
    ids.sort();
    let mut total = Counts::default();
    let towns: Vec<Value> = ids
        .iter()
        .map(|id| {
            let s = &state.shown[*id];
            let c = s.counts;
            total.houses += c.houses;
            total.kit_models += c.kit_models;
            total.blocks += c.blocks;
            total.monuments += c.monuments;
            total.wall_pieces += c.wall_pieces;
            total.towers += c.towers;
            total.gates += c.gates;
            total.suburb_houses += c.suburb_houses;
            total.enclosure_pieces += c.enclosure_pieces;
            total.props += c.props;
            total.entities += c.entities;
            let landmark = s.landmark.as_ref().map(|l| {
                let tris: usize = l.plan.monuments.iter().map(|m| m.built.mesh.triangles()).sum();
                json!({
                    "monuments": l.plan.monuments.len(),
                    "monument_triangles": tris,
                    "wall_rings": l.plan.wall_rings.len(),
                    "wall_length_m": l.plan.wall_length_m(),
                    "bridges": l.plan.bridges.len(),
                    "bridge_houses": l.plan.fixed_bases.len(),
                    "wall_gates": l.plan.gates.len(),
                    "garden_trees": l.plan.open_space_trees,
                })
            });
            json!({
                "id": id,
                "landmark": landmark,
                "lod": if s.lod == Lod::Detail { "detail" } else { "blocks" },
                "growth": {"quarters": s.growth.quarters, "enclosure": s.growth.enclosure},
                "plan": {"streets": s.plan.streets.len(), "houses": s.plan.houses.len(), "monuments": s.plan.monuments.len(), "towers": s.plan.towers.len(), "gates": s.plan.gates.len(), "bridge": s.plan.bridge.is_some(), "trees": if s.trees.is_some() { s.plan.trees.len() } else { 0 }},
                "counts": {"houses": c.houses, "kit_models": c.kit_models, "blocks": c.blocks, "monuments": c.monuments, "wall_pieces": c.wall_pieces, "towers": c.towers, "gates": c.gates, "suburb_houses": c.suburb_houses, "enclosure_pieces": c.enclosure_pieces, "props": c.props, "entities": c.entities},
            })
        })
        .collect();
    (
        "campaign_towns".into(),
        json!({
            "ready": true,
            "towns_in_data": w.db.towns.len(),
            "kit_models": w.kit.len(),
            "landmark_skipped": w.skip.len(),
            "landmark_cities": w.landmarks.cities.len(),
            "landmark_anchored": w.landmark_px.len(),
            "forced": w.force_active,
            "active": state.active,
            "weight": state.weight,
            "profile": {"max_rig_distance": w.profile.max_rig_distance, "detail_range": w.profile.detail_range, "stream_max": w.profile.stream_max},
            "shown": towns.len(),
            "plans_cached": state.plans.len(),
            "plans_built": state.plans_built,
            "towns_spawned": state.towns_spawned,
            "plan_ms_max": state.plan_ms_max,
            "spawn_ms_max": state.spawn_ms_max,
            "reposed": state.reposed,
            "totals": {"houses": total.houses, "kit_models": total.kit_models, "blocks": total.blocks, "monuments": total.monuments, "wall_pieces": total.wall_pieces, "towers": total.towers, "gates": total.gates, "suburb_houses": total.suburb_houses, "enclosure_pieces": total.enclosure_pieces, "props": total.props, "entities": total.entities},
            "towns": towns,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_env_parses() {
        assert_eq!(
            parse_growth_env("3"),
            Some(Growth {
                quarters: 3,
                enclosure: 0
            })
        );
        assert_eq!(
            parse_growth_env("2,stone"),
            Some(Growth {
                quarters: 2,
                enclosure: 2
            })
        );
        assert_eq!(parse_growth_env("x"), None);
    }

    #[test]
    fn ribbons_follow_the_given_height() {
        let root = crate::paths::find_root().unwrap();
        let db = TownDb::load(&crate::paths::data_dir(&root));
        let town = &db.towns["set_abbeville"];
        let flat = |_: Vec2| 0.0;
        let plan = plan::generate(town, &db, &flat);
        let high = |_: Vec2| 100.0;
        let mesh = street_mesh(&plan, &high, 2.0);
        let ys: Vec<f32> = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap()
            .iter()
            .map(|p| p[1])
            .collect();
        assert!(!ys.is_empty() && ys.iter().all(|y| (*y - 102.0).abs() < 1e-4));
        let g = ground_mesh(&plan, &high, 1.0);
        assert!(g.indices().is_some_and(|i| i.len() > 100));
    }

    #[test]
    fn footing_takes_the_lowest_corner() {
        let slope = |p: Vec2| p.x;
        let g = TownGround {
            anchor: Vec2::ZERO,
            local: Vec2::ZERO,
            yaw: 0.0,
            half: Vec2::new(5.0, 2.0),
            lift_m: 1.0,
            sink_m: 0.5,
            sample: None,
        };
        assert!((footing_y(&slope, &g) - (-5.0 - 0.5 + 1.0)).abs() < 1e-4);
    }

    #[test]
    fn root_scale_is_uniform_at_full_weight() {
        assert_eq!(root_scale(0.5, 1.0), Vec3::splat(0.5));
        assert!(root_scale(0.5, 0.0).y > 0.0);
    }
}
