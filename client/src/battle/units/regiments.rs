//! Per-regiment figure rendering: spawns one skinned batch (and one impostor batch) per
//! regiment, feeds it blended soldier poses, and drives clip choice, regiment clock, gaits,
//! level of detail and distance impostors. Ports the update loop of
//! game/scripts/battle/battle_soldiers.gd (`update`, `_shown_state`, `_cadence`).

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use sim_battle::{BattleSim, DT, SideId, Unit, UnitState};

use super::anim::{self, Mode};
use super::arms::{DroppedArms, RoutDrop};
use super::corpses::{Corpses, Loss};
use super::gaits::{self, GaitMemory};
use super::heraldry::{ArmsAtlasHandle, BattleHeraldry};
use super::library::{FigureLibrary, FigureSpec, render_kind};
use super::looks;
use super::skinned::{RECORD_FLOATS, SkinParams, SkinnedBatch, push_record};
use super::{PoseFrames, SoldierStats, blend_poses, drawn_units};
use crate::app::AppState;
use crate::battle::orders::BattleSelection;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::sim::Sim;

/// Camera distance (m) below which a regiment draws the full mesh.
pub const LOD0_DISTANCE: f32 = 16.0;
/// Below this distance the medium mesh, beyond the far mesh.
pub const LOD1_DISTANCE: f32 = 75.0;
/// Beyond this distance regiments are camera-facing quads (`BattleImpostors.DISTANCE`).
pub const IMPOSTOR_DISTANCE: f32 = 300.0;
/// Hysteresis against flicker at a threshold (fraction of the distance).
const HYSTERESIS: f32 = 0.08;
/// Fade time between two clip sets (`blend_time` of the shader).
const BLEND_TIME_S: f32 = 0.35;
const TRIM_GOLD: [f32; 3] = [0.83, 0.66, 0.24];
const TRIM_SILVER: [f32; 3] = [0.85, 0.85, 0.82];
const WEATHERING: f32 = 0.85;
const MUD_HEIGHT: f32 = 0.45;
const MUD_HEIGHT_HORSE: f32 = 0.65;

/// Level of detail of a regiment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lod {
    Full,
    Medium,
    Far,
    Impostor,
}

impl Lod {
    pub fn index(self) -> usize {
        self as usize
    }

    /// Level for a camera distance, keeping the current one inside the hysteresis band.
    pub fn for_distance(distance: f32, current: Option<Lod>) -> Lod {
        Self::for_distance_scaled(distance, current, 1.0)
    }

    /// Same with the impostor edge multiplied by `impostor_scale` (`battle_lod` of the quality
    /// preset: higher presets keep real figures further out).
    pub fn for_distance_scaled(distance: f32, current: Option<Lod>, impostor_scale: f32) -> Lod {
        const LEVELS: [Lod; 4] = [Lod::Full, Lod::Medium, Lod::Far, Lod::Impostor];
        let edges: [f32; 3] = [
            LOD0_DISTANCE,
            LOD1_DISTANCE,
            IMPOSTOR_DISTANCE * impostor_scale.clamp(0.25, 4.0),
        ];
        let plain = edges.iter().position(|e| distance < *e).unwrap_or(3);
        let Some(current) = current else {
            return LEVELS[plain];
        };
        let at = current.index();
        match plain.cmp(&at) {
            std::cmp::Ordering::Equal => current,
            // Moving away: leave the level only past its outer edge plus the margin.
            std::cmp::Ordering::Greater if distance < edges[at] * (1.0 + HYSTERESIS) => current,
            // Approaching: enter the nearer level only below its edge minus the margin.
            std::cmp::Ordering::Less if distance > edges[at - 1] * (1.0 - HYSTERESIS) => current,
            _ => LEVELS[plain],
        }
    }
}

/// One regiment on screen.
struct Regiment {
    main: Entity,
    impostor: Entity,
    spec: Arc<FigureSpec>,
    params: SkinParams,
    params_version: u64,
    impostor_params_version: u64,
    config_key: String,
    /// Regiment animation clock (s), advanced by the cadence.
    clock: f32,
    speed: f32,
    turn: f32,
    position: Vec2,
    facing: f32,
    gait: GaitMemory,
    ammo: u32,
    volley_time: f32,
    lod: Option<Lod>,
    version: u64,
    /// Records, LOD and params versions sent with the latest `version` (perf3: no bump when nothing changed).
    sent: Option<(Arc<Vec<u8>>, Lod, u64, u64)>,
    drawn: usize,
    state_key: String,
    /// Poses at the last change of head count (slots of the bodies that fall).
    last_poses: Vec<[f32; 4]>,
    /// Regiment clock of the latest reaction to losses.
    hit_clock: f32,
    routing: bool,
}

#[derive(Resource, Default)]
pub struct Regiments {
    map: HashMap<u32, Regiment>,
    quad: Option<Handle<Mesh>>,
    /// Tick of the last measure of speed and turn.
    measured: Option<u64>,
}

impl Regiments {
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Regiments whose figure carries a rider's lance, and the lowest steel occlusion in use.
    pub fn lance_stats(&self) -> (usize, f32) {
        let lances = self
            .map
            .values()
            .filter(|r| r.params.prop_bone() >= 0)
            .count();
        let steel = self
            .map
            .values()
            .map(|r| r.params.steel_occlusion())
            .fold(1.0f32, f32::min);
        (lances, steel)
    }

    /// `(state key, level of detail, drawn figures)` of each regiment, for the dump.
    pub fn summaries(&self) -> impl Iterator<Item = (u32, &str, &str, Lod, usize)> {
        self.map.iter().map(|(id, r)| {
            (
                *id,
                r.spec.key.as_str(),
                r.state_key.as_str(),
                r.lod.unwrap_or(Lod::Far),
                r.drawn,
            )
        })
    }
}

#[derive(SystemParam)]
pub struct RegimentContext<'w, 's> {
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    images: ResMut<'w, Assets<Image>>,
    assets: Res<'w, AssetServer>,
    library: ResMut<'w, FigureLibrary>,
    sim: Res<'w, Sim>,
    corpses: ResMut<'w, Corpses>,
    arms: ResMut<'w, DroppedArms>,
    settings: Option<Res<'w, crate::settings::Settings>>,
    quality: Option<Res<'w, crate::settings::quality::RenderQuality>>,
    heraldry: ResMut<'w, BattleHeraldry>,
    arms_atlas: ResMut<'w, ArmsAtlasHandle>,
}

fn side_index(side: SideId) -> usize {
    usize::from(side == SideId::Defender)
}

fn srgb_to_linear(rgb: [f32; 3]) -> [f32; 3] {
    let c = Color::srgb(rgb[0], rgb[1], rgb[2]).to_linear();
    [c.red, c.green, c.blue]
}

/// Trim metal: silver on light liveries, gold otherwise.
pub fn trim_for(color: [f32; 3]) -> [f32; 3] {
    let luminance = 0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2];
    if luminance > 0.55 || (color[0] > 0.6 && color[1] > 0.5) {
        TRIM_SILVER
    } else {
        TRIM_GOLD
    }
}

fn quad_mesh() -> Mesh {
    use crate::battle::units::figures::{
        ATTRIBUTE_ATLAS, ATTRIBUTE_BONES, ATTRIBUTE_MASK, ATTRIBUTE_WEIGHTS,
    };
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
    let positions = vec![
        [-0.5, 0.0, 0.0],
        [0.5, 0.0, 0.0],
        [0.5, 1.0, 0.0],
        [-0.5, 1.0, 0.0],
    ];
    let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0f32; 4]; 4]);
    mesh.insert_attribute(
        ATTRIBUTE_BONES,
        VertexAttributeValues::Float32x4(vec![[0.0; 4]; 4]),
    );
    mesh.insert_attribute(
        ATTRIBUTE_WEIGHTS,
        VertexAttributeValues::Float32x4(vec![[0.0; 4]; 4]),
    );
    mesh.insert_attribute(ATTRIBUTE_MASK, VertexAttributeValues::Float32(vec![0.0; 4]));
    mesh.insert_attribute(
        ATTRIBUTE_ATLAS,
        VertexAttributeValues::Float32(vec![0.0; 4]),
    );
    // Both windings: the quad is seen from either side.
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2]));
    mesh
}

/// Initial parameters of a regiment's figures (livery, looks, rig tables).
fn build_params(
    spec: &FigureSpec,
    library: &FigureLibrary,
    unit_type: &str,
    side_color: [f32; 3],
    cycle_blend_s: f32,
) -> SkinParams {
    let mut params = SkinParams::default();
    let look = library.looks.get(unit_type);
    let share = look
        .and_then(|l| l.livery_share)
        .unwrap_or(if spec.noble { 0.92 } else { 0.6 });
    params.set_livery(srgb_to_linear(side_color), share);
    params.set_trim(srgb_to_linear(trim_for(side_color)));
    let reload = if spec.kind == "archer" && spec.variant == 2 {
        9.0
    } else {
        6.0
    };
    params.set_rig(
        spec.fps,
        spec.variants,
        if spec.mounted { 0.0 } else { 0.05 },
        reload,
    );
    params.set_blend_time(BLEND_TIME_S);
    params.set_cycle_blend(cycle_blend_s);
    let hit: Vec<u32> = spec
        .clips
        .present(super::settings::hit_clips(&spec.style))
        .iter()
        .map(|n| spec.clips.index(n))
        .collect();
    let hit_seconds = hit
        .iter()
        .filter_map(|i| spec.clips.names.get(*i as usize))
        .filter_map(|n| spec.clips.get(n))
        .map(|c| c.frames as f32 / spec.fps.max(1.0))
        .fold(0.0f32, f32::max)
        .max(0.6);
    params.set_hit_set(&hit, hit_seconds, if spec.mounted { 0.12 } else { 0.35 });
    params.set_clips(&spec.clip_table);
    params.set_ga3(spec.ga3_lum, spec.head_y, spec.albedo.is_some());
    params.set_fine(spec.atlas_layer);
    params.set_prop(spec.lance.as_ref());
    params.set_wear(
        if spec.mounted {
            MUD_HEIGHT_HORSE
        } else {
            MUD_HEIGHT
        },
        WEATHERING,
    );
    if let Some(look) = look {
        let cloth = look.cloth_colours();
        if !cloth.is_empty() {
            params.set_plain(&looks::padded(&cloth), cloth.len());
        }
        let coats = look.coat_colours();
        if !coats.is_empty() {
            params.set_coats(&looks::padded(&coats), coats.len());
        }
    }
    params
}

/// Render state of a regiment: the simulation state, the victory pose and the cavalry
/// rearing against pikes (`_shown_state`).
fn shown_state(sim: &BattleSim, unit: &Unit, styles: &HashMap<u32, String>) -> &'static str {
    let state = match unit.state {
        UnitState::Idle | UnitState::Rallied => "idle",
        UnitState::Marching => "marching",
        UnitState::Charging => "charging",
        UnitState::Melee => "melee",
        UnitState::Shooting => "shooting",
        UnitState::Routing => "routing",
        UnitState::Climbing => "climbing",
    };
    if let Some(winner) = sim.winner().filter(|_| sim.is_finished())
        && unit.side == winner
        && state != "routing"
        && state != "climbing"
    {
        return "victory";
    }
    if unit.mounted
        && state == "melee"
        && unit
            .target
            .and_then(|id| styles.get(&id))
            .is_some_and(|s| s == "pike" || s == "militia")
    {
        return "melee_pikes";
    }
    state
}

#[allow(clippy::too_many_arguments)]
pub fn update_regiments(
    mut ctx: RegimentContext,
    mut session: ResMut<BattleSession>,
    time: Res<Time>,
    selection: Option<Res<BattleSelection>>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    mut regiments: ResMut<Regiments>,
    mut frames: ResMut<PoseFrames>,
    mut stats: ResMut<SoldierStats>,
    mut batches: Query<&mut SkinnedBatch>,
    mut env_lod: Local<Option<Option<f32>>>,
) {
    let paused = session.paused;
    let speed_factor = session.speed as f32;
    let fraction = if paused { 1.0 } else { session.step_fraction() };
    let colors = session.colors;
    let dt = if paused {
        0.0
    } else {
        time.delta_secs() * speed_factor
    };
    // `CENT_ANS_BATTLE_LOD=<factor>` overrides the preset (A/B benches, captures).
    let lod_override = *env_lod.get_or_insert_with(|| {
        std::env::var("CENT_ANS_BATTLE_LOD")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
    });
    let lod_scale =
        lod_override.unwrap_or_else(|| ctx.quality.as_ref().map_or(1.0, |q| q.battle_lod));
    let camera_position = camera
        .iter()
        .next()
        .map_or(Vec3::new(0.0, 100.0, 0.0), |t| t.translation());
    let Some(game) = ctx.sim.data.clone() else {
        return;
    };
    let sim = &*session.sim_mut();
    let ticks = sim.ticks();
    if frames.ticks != Some(ticks) {
        let (unit_size, max_figures) = ctx
            .settings
            .as_ref()
            .map_or((1.0, 0), |s| (s.unit_size, s.max_figures));
        frames.refresh(sim, ticks, unit_size, max_figures);
    }
    let measure = regiments.measured != Some(ticks);
    let tick_dt = regiments
        .measured
        .map_or(0, |old| ticks.saturating_sub(old)) as f32
        * DT as f32;
    regiments.measured = Some(ticks);
    if regiments.quad.is_none() {
        regiments.quad = Some(ctx.meshes.add(quad_mesh()));
    }
    let quad = regiments.quad.clone().expect("quad set above");
    {
        let ctx = &mut ctx;
        ctx.heraldry
            .ensure_built(sim, &mut ctx.images, &mut ctx.arms_atlas);
    }
    let styles: HashMap<u32, String> = regiments
        .map
        .iter()
        .map(|(id, r)| (*id, r.spec.style.clone()))
        .collect();
    let mut seen: Vec<u32> = Vec::new();
    let mut lod_counts = [0usize; 4];
    let mut total = 0usize;
    for unit in drawn_units(sim) {
        let Some(kind) = render_kind(unit) else {
            continue;
        };
        if let Entry::Vacant(slot) = regiments.map.entry(unit.id) {
            let variant = FigureLibrary::variant_for(&game, &unit.unit_type);
            let ctx = &mut ctx;
            let Some(spec) =
                ctx.library
                    .spec(kind, variant, &mut ctx.meshes, &mut ctx.images, &ctx.assets)
            else {
                continue;
            };
            let color = colors[side_index(unit.side)];
            let mut params = build_params(
                &spec,
                &ctx.library,
                &unit.unit_type,
                color,
                ctx.library.fx.cycle_blend_s,
            );
            let arms =
                ctx.heraldry
                    .regiment(side_index(unit.side), unit.id, spec.noble, spec.chest);
            params.set_arms(&arms);
            let make = |name: String, mesh: Handle<Mesh>| {
                (
                    Name::new(name),
                    SkinnedBatch {
                        mesh_handle: mesh,
                        albedo: spec.albedo.clone(),
                        bones: spec.bones.clone(),
                        params: Arc::new(params.data.clone()),
                        params_version: 1,
                        records: Arc::new(Vec::new()),
                        count: 0,
                        version: 0,
                    },
                    DespawnOnExit(AppState::Battle),
                )
            };
            let main = ctx
                .commands
                .spawn(make(format!("Regiment{}", unit.id), spec.lods[0].clone()))
                .id();
            let impostor = ctx
                .commands
                .spawn(make(format!("Impostor{}", unit.id), quad.clone()))
                .id();
            slot.insert(Regiment {
                main,
                impostor,
                spec,
                params,
                params_version: 1,
                impostor_params_version: 0,
                config_key: String::new(),
                clock: 0.0,
                speed: 0.0,
                turn: 0.0,
                position: Vec2::new(unit.x as f32, unit.z as f32),
                facing: unit.facing as f32,
                gait: GaitMemory::default(),
                ammo: unit.ammo,
                volley_time: -1000.0,
                lod: None,
                version: 0,
                sent: None,
                drawn: 0,
                state_key: String::new(),
                last_poses: frames.cur.get(&unit.id).cloned().unwrap_or_default(),
                hit_clock: -1000.0,
                routing: false,
            });
        }
        let Some(regiment) = regiments.map.get_mut(&unit.id) else {
            continue;
        };
        seen.push(unit.id);
        let Some(cur) = frames.cur.get(&unit.id) else {
            continue;
        };
        // Speed and turn rate, measured once per simulation step and smoothed per frame.
        let here = Vec2::new(unit.x as f32, unit.z as f32);
        if measure && tick_dt > 0.0 {
            let v = ((here - regiment.position).length() / tick_dt).min(30.0);
            let rate = gaits::turn_rate(regiment.facing, unit.facing as f32, tick_dt);
            regiment.speed += (v - regiment.speed) * 0.5;
            regiment.turn += (rate - regiment.turn) * 0.5;
        }
        if measure {
            regiment.position = here;
            regiment.facing = unit.facing as f32;
        }

        // Clip set of the regiment's state.
        let mut state = shown_state(sim, unit, &styles);
        let mut running = unit.running;
        let gait_key;
        if regiment.spec.kind == "cavalry"
            && state == "marching"
            && ctx.library.settings.gaits.enabled
        {
            gait_key = gaits::pick(
                &ctx.library.settings.gaits,
                &mut regiment.gait,
                regiment.speed,
                running,
                regiment.turn,
            )
            .key();
            state = gait_key;
            running = false;
        }
        let config = anim::state_config(
            &regiment.spec.style,
            &regiment.spec.clips,
            state,
            running,
            ctx.library.settings.jitter,
        );
        if config.key != regiment.config_key {
            let fade = !regiment.config_key.is_empty();
            regiment.params.set_config(&config, regiment.clock, fade);
            regiment.config_key = config.key.clone();
            regiment.params_version += 1;
            regiment.impostor_params_version = 0;
            regiment.state_key = state.to_owned();
        }
        // Cadence: locomotion follows the real speed so that feet do not slide.
        let mut cadence = 1.0;
        if config.mode == Mode::Loop || (config.mode == Mode::Cycle && state == "charging") {
            let nominal = config
                .names
                .first()
                .and_then(|n| ctx.library.settings.nominal_cadence(n, &unit.unit_type))
                .map(|n| n * config.speed);
            if let Some(nominal) = nominal {
                cadence = (regiment.speed / nominal.max(0.1)).clamp(0.35, 1.8);
            }
        }
        regiment.clock += dt * cadence;
        if unit.ammo < regiment.ammo {
            regiment.volley_time = regiment.clock;
        }
        regiment.ammo = unit.ammo;

        // Losses: bodies fall on the slots of the previous frame, the survivors react.
        if cur.len() < regiment.last_poses.len() && !unit.left_field {
            let count = regiment.last_poses.len() - cur.len();
            let killer = unit
                .loss_by
                .and_then(|id| sim.units().iter().find(|u| u.id == id))
                .map(|k| Vec2::new(k.x as f32, k.z as f32));
            ctx.corpses.spawn(
                &mut ctx.commands,
                Loss {
                    unit: unit.id,
                    spec: &regiment.spec,
                    params: &regiment.params,
                    previous: &regiment.last_poses,
                    count,
                    cause: unit.loss_cause.key(),
                    killer,
                    left: unit.soldiers(),
                    fx: &ctx.library.fx,
                },
                &mut ctx.arms,
                true,
            );
            if regiment.clock - regiment.hit_clock >= ctx.library.fx.melee_hit.window_s {
                regiment.hit_clock = regiment.clock;
                regiment.params.set_hit_time(regiment.clock);
                regiment.params_version += 1;
                regiment.impostor_params_version = 0;
            }
        }
        if cur.len() != regiment.last_poses.len() {
            regiment.last_poses = cur.clone();
        }
        // A regiment that breaks throws its arms once.
        let routing = unit.state == UnitState::Routing;
        if routing && !regiment.routing {
            ctx.arms.push_rout(RoutDrop {
                style: regiment.spec.style.clone(),
                poses: cur.clone(),
                side: side_index(unit.side),
                regiment: unit.id,
            });
        }
        regiment.routing = routing;

        // Level of detail from the camera distance.
        let ground = cur.first().map_or(0.0, |p| p[1]);
        let distance = camera_position.distance(Vec3::new(unit.x as f32, ground, unit.z as f32));
        let lod = Lod::for_distance_scaled(distance, regiment.lod, lod_scale);
        regiment.lod = Some(lod);

        // Records.
        let poses = blend_poses(frames.prev.get(&unit.id).map(Vec::as_slice), cur, fraction);
        let selected = selection
            .as_ref()
            .is_some_and(|s| s.units.contains(&unit.id));
        let far_blend = ((distance - 80.0) / 180.0).clamp(0.0, 1.0);
        let mut bytes = Vec::with_capacity(poses.len() * RECORD_FLOATS * 4);
        let id_offset = (unit.id % 13) as f32 * 500.0;
        for (i, pose) in poses.iter().enumerate() {
            let record: [f32; RECORD_FLOATS] = [
                pose[0],
                pose[1],
                pose[2],
                pose[3].to_degrees(),
                1.0,
                i as f32 + id_offset,
                regiment.clock,
                far_blend,
                regiment.volley_time,
                if selected { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ];
            push_record(&mut bytes, &record);
        }
        let count = poses.len() as u32;
        regiment.drawn = poses.len();
        total += poses.len();
        lod_counts[lod.index()] += poses.len();
        let unchanged = is_unchanged(
            regiment.sent.as_ref(),
            &bytes,
            lod,
            regiment.params_version,
            regiment.impostor_params_version,
        );
        let records = match &regiment.sent {
            Some((old, ..)) if unchanged => old.clone(),
            _ => Arc::new(bytes),
        };
        if !unchanged {
            regiment.version += 1;
        }
        if let Ok(mut batch) = batches.get_mut(regiment.main) {
            let wanted = &regiment.spec.lods[lod.index().min(2)];
            if batch.mesh_handle != *wanted {
                batch.mesh_handle = wanted.clone();
            }
            batch.count = if lod == Lod::Impostor { 0 } else { count };
            batch.records = records.clone();
            batch.version = regiment.version;
            if batch.params_version != regiment.params_version {
                batch.params = Arc::new(regiment.params.data.clone());
                batch.params_version = regiment.params_version;
            }
        }
        if let Ok(mut batch) = batches.get_mut(regiment.impostor) {
            batch.count = if lod == Lod::Impostor { count } else { 0 };
            batch.records = records.clone();
            batch.version = regiment.version;
            if regiment.impostor_params_version != regiment.params_version {
                let mut p = regiment.params.clone();
                let (width, height) = if regiment.spec.mounted {
                    (2.4, 2.5)
                } else {
                    (0.95, 1.9)
                };
                p.set_impostor(width, height);
                batch.params = Arc::new(p.data);
                batch.params_version += 1;
                regiment.impostor_params_version = regiment.params_version;
            }
        }
        regiment.sent = Some((
            records,
            lod,
            regiment.params_version,
            regiment.impostor_params_version,
        ));
    }
    // Regiments that left the field (reserve, destroyed): nothing drawn.
    for (id, regiment) in &mut regiments.map {
        if seen.contains(id) {
            continue;
        }
        regiment.drawn = 0;
        for entity in [regiment.main, regiment.impostor] {
            if let Ok(mut batch) = batches.get_mut(entity) {
                batch.count = 0;
                regiment.version += 1;
                batch.version = regiment.version;
            }
        }
    }
    stats.drawn = total;
    stats.lod = lod_counts;
    stats.regiments = seen.len();
    let mut per_batch = [0usize; 4];
    for unit in drawn_units(sim) {
        if let Some(r) = regiments.map.get(&unit.id) {
            per_batch[side_index(unit.side) * 2 + usize::from(unit.mounted)] += r.drawn;
        }
    }
    stats.per_batch = per_batch;
}

/// Whether what was sent with the previous `version` (records, LOD, params versions) is still exact.
fn is_unchanged(
    sent: Option<&(Arc<Vec<u8>>, Lod, u64, u64)>,
    bytes: &[u8],
    lod: Lod,
    params_version: u64,
    impostor_params_version: u64,
) -> bool {
    sent.is_some_and(|(old, l, p, ip)| {
        old.as_slice() == bytes
            && *l == lod
            && *p == params_version
            && *ip == impostor_params_version
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_only_moves_when_something_changes() {
        let sent = Some((Arc::new(vec![1u8, 2, 3]), Lod::Full, 4, 4));
        assert!(is_unchanged(sent.as_ref(), &[1, 2, 3], Lod::Full, 4, 4));
        assert!(!is_unchanged(sent.as_ref(), &[1, 2, 4], Lod::Full, 4, 4));
        assert!(!is_unchanged(sent.as_ref(), &[1, 2, 3], Lod::Far, 4, 4));
        assert!(!is_unchanged(sent.as_ref(), &[1, 2, 3], Lod::Full, 5, 4));
        assert!(!is_unchanged(sent.as_ref(), &[1, 2, 3], Lod::Full, 4, 0));
        assert!(!is_unchanged(None, &[], Lod::Full, 1, 0));
    }

    #[test]
    fn lod_follows_distance_with_hysteresis() {
        assert_eq!(Lod::for_distance(5.0, None), Lod::Full);
        assert_eq!(Lod::for_distance(40.0, None), Lod::Medium);
        assert_eq!(Lod::for_distance(150.0, None), Lod::Far);
        assert_eq!(Lod::for_distance(500.0, None), Lod::Impostor);
        // Just past the 300 m edge: stays far when coming from far, impostor when coming from it.
        assert_eq!(Lod::for_distance(305.0, Some(Lod::Far)), Lod::Far);
        assert_eq!(Lod::for_distance(295.0, Some(Lod::Impostor)), Lod::Impostor);
        assert_eq!(Lod::for_distance(250.0, Some(Lod::Impostor)), Lod::Far);
        assert_eq!(Lod::for_distance(17.0, Some(Lod::Full)), Lod::Full);
        assert_eq!(Lod::for_distance(15.0, Some(Lod::Medium)), Lod::Medium);
        // battle_lod pushes only the impostor edge.
        assert_eq!(Lod::for_distance_scaled(400.0, None, 1.5), Lod::Far);
        assert_eq!(Lod::for_distance_scaled(460.0, None, 1.5), Lod::Impostor);
        assert_eq!(Lod::for_distance_scaled(40.0, None, 1.5), Lod::Medium);
        assert_eq!(Lod::for_distance_scaled(200.0, None, 0.55), Lod::Impostor);
    }

    #[test]
    fn trim_is_silver_on_light_liveries() {
        assert_eq!(trim_for([0.9, 0.9, 0.9]), TRIM_SILVER);
        assert_eq!(trim_for([0.2, 0.3, 0.75]), TRIM_GOLD);
        assert_eq!(trim_for([0.7, 0.6, 0.1]), TRIM_SILVER);
    }
}
