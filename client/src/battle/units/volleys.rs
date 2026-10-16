//! Volleys of arrows, bolts, bullets and javelins: every shot resolved by the simulation
//! (`BattleSim::take_shots`) becomes a flight of instanced missiles whose paths are computed on
//! the CPU from the shot (shooter and target regiments, missile count). A share of the missiles
//! stays planted in the ground or in the pavises and stakes for the rest of the battle.
//!
//! Port of game/scripts/battle/battle_volleys.gd (`on_shot`, `arrow_landing`, the constants of
//! `battle_volley.gdshaderinc`). The GPU path of the shader is a CPU parabola here, the stuck
//! layer is a ring buffer of oriented records.
//!
//! Fire arrows (`incendiary` shots of siege attackers): a share of the volley (at most
//! `FIRE_ARROWS_PER_VOLLEY`) carries a flame, drawn as flipbook flame sprites shed along the
//! path and a small fire where each one lands (Godot draws a flame panel on every arrow).
//! Stakes (`stakes` of the core: archers who stood still) and planted pavises (`pavise_cover`)
//! are drawn in front of the regiments and stay for the battle ([`Fieldworks`], `update_fieldworks`
//! and the meshes of `_stake_mesh` / `_pavise_mesh`).
//!
//! Not ported: the sounds (`sound_event`: the audio lot reads the [`ShotFeed`]), missiles that
//! stay planted for `STICK_HOLD` seconds before fading (those are removed at landing, only the
//! sampled share stays), the ribbon trail of arrows (the oriented batch has no alpha).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use data_model::Ability;
use sim_battle::{BattleSim, MissileKind, ShotCover, ShotEvent, SideId, UnitState};

use super::corpses::hash4;
use super::instancing::MaterialParams;
use super::oriented::OrientedBatch;
use super::smoke::SmokeRequests;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::fx::flipbook::{FlipbookKind, FlipbookLooks, SmokeSprite, spawn_fire_sprite};

/// Missiles drawn per simulated man and per volley.
const ARROWS_PER_MAN: f32 = 3.0;
const MAX_ARROWS_PER_VOLLEY: usize = 4096;
/// Beyond this camera distance the volleys are thinned (a cloud seen from afar).
const FAR_DISTANCE: f32 = 700.0;
/// Missiles in the air at once (the oldest volleys are thinned past it).
const MAX_FLYING: usize = 24_000;
/// Missiles farther than this from the camera are not drawn (not packed or uploaded).
const DRAW_RANGE_M: f32 = 1200.0;
const MAX_STUCK: usize = 8_000;
const STUCK_PER_VOLLEY: usize = 96;
const STUCK_STRIDE: usize = 5;
/// Metres per second, arc (share of the distance), stagger (s) per kind: arrow, bolt, bullet,
/// javelin (`battle_volley.gdshaderinc`).
const SPEED: [f32; 4] = [48.0, 62.0, 210.0, 24.0];
const ARC: [f32; 4] = [0.16, 0.06, 0.015, 0.22];
const STAGGER: [f32; 4] = [1.5, 0.8, 0.35, 1.1];
/// Arc factor of a lobbed shot over a crest (`VOLLEY_LOB`).
const LOB: f32 = 2.5;
/// Drawn length of a missile (m) per kind.
const SCALE: [f32; 4] = [1.7, 1.3, 0.5, 2.2];

/// Shots taken from the simulation, kept for other consumers (audio) until the next frame.
#[derive(Resource, Default)]
pub struct ShotFeed {
    pub shots: Vec<ShotEvent>,
}

/// Fire arrows of one volley that burn (a flame panel on each in Godot).
const FIRE_ARROWS_PER_VOLLEY: usize = 24;
/// Seconds between two flame sprites shed by a burning arrow, and the life of one.
const FLAME_EVERY_S: f32 = 0.1;
const FLAME_LIFE_S: f32 = 0.35;
/// The fire where a burning arrow stands (s), and the flipbook budget kept for the fires.
const LANDED_FIRE_S: f32 = 2.5;
const FLAME_SPRITE_ROOM: usize = 650;

struct Missile {
    fire: bool,
    /// Next time (s of the volley clock) a flame sprite is shed.
    next_flame: f32,
    start: Vec3,
    end: Vec3,
    launch: f32,
    flight: f32,
    arc: f32,
    kind: usize,
    stick: bool,
}

#[derive(Resource, Default)]
pub struct Volleys {
    clock: f32,
    flying: Vec<Missile>,
    stuck: Vec<u8>,
    stuck_count: usize,
    stuck_next: usize,
    seed: u64,
    flying_batch: Option<Entity>,
    stuck_batch: Option<Entity>,
    stuck_version: u64,
    version: u64,
    /// Missiles launched since the battle began.
    pub launched: usize,
    pub volleys: usize,
    pub by_kind: [usize; 4],
    /// Flame sprites shed by burning arrows, and the fires lit where they landed.
    pub flames_shed: usize,
    pub fires_lit: usize,
    /// Most missiles in flight at once, and CPU cost (microseconds) of `update_volleys`.
    pub peak_flying: usize,
    pub update_us_peak: u64,
    pub update_us_total: u64,
    pub updates: u64,
}

fn kind_index(kind: MissileKind) -> Option<usize> {
    match kind {
        MissileKind::Arrow => Some(0),
        MissileKind::Bolt => Some(1),
        MissileKind::Bullet => Some(2),
        MissileKind::Javelin => Some(3),
        // Engines' balls and stones: drawn by the siege lot.
        MissileKind::Ball | MissileKind::Stone => None,
    }
}

fn missile_mesh() -> Mesh {
    // A shaft along -Z ending at the origin (the tip), a head and fletching.
    let mut mesh =
        Mesh::from(Cuboid::new(0.025, 0.025, 1.0)).translated_by(Vec3::new(0.0, 0.0, -0.5));
    let head = Mesh::from(Cuboid::new(0.05, 0.05, 0.1)).translated_by(Vec3::new(0.0, 0.0, -0.04));
    let fletch =
        Mesh::from(Cuboid::new(0.07, 0.01, 0.16)).translated_by(Vec3::new(0.0, 0.0, -0.93));
    mesh.merge(&head).expect("same layout");
    mesh.merge(&fletch).expect("same layout");
    mesh
}

fn pack(record: [f32; 8], bytes: &mut Vec<u8>) {
    for v in record {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
}

/// Where and when one missile of a shot flies; `None` when the shooter is gone.
#[allow(clippy::too_many_arguments)]
fn plan_missile(
    sim: &BattleSim,
    shot: &ShotEvent,
    kind: usize,
    seed: u64,
    index: usize,
    shooter_half: (f32, f32),
    target_half: (f32, f32),
    now: f32,
) -> Missile {
    let h = |k: u64| hash4(seed.wrapping_mul(131).wrapping_add(index as u64 * 16 + k));
    let field = sim.field();
    let ground = |x: f32, z: f32| field.height(f64::from(x), f64::from(z)) as f32;
    let from = Vec2::new(shot.from.0 as f32, shot.from.1 as f32);
    let aim = Vec2::new(shot.aim.0 as f32, shot.aim.1 as f32);
    let d2 = aim - from;
    let dir = d2 / d2.length().max(1.0);
    let perp = Vec2::new(dir.y, -dir.x);
    let (shw, shd) = shooter_half;
    let (thw, thd) = target_half;
    let (r0, r1, r2, r3) = (h(0), h(1), h(2), h(3));
    let s_off = perp * ((r0[0] * 2.0 - 1.0) * shw) + dir * ((r0[1] * 2.0 - 1.0) * shd);
    let across = (r0[2] + r0[3] - 1.0) * (thw + 2.0);
    let mut along = (r1[0] + r1[1] - 1.0) * (thd * 1.6 + 3.0);
    let cover = !matches!(shot.cover, ShotCover::None | ShotCover::Wall);
    let in_cover = cover && along < -0.35 * thd;
    if in_cover {
        along = -(thd + 0.9);
    }
    let t_off = perp * across + dir * along;
    let sy = ground(from.x, from.y) + 1.5;
    let start = Vec3::new(from.x + s_off.x, sy, from.y + s_off.y);
    let ex = aim.x + t_off.x;
    let ez = aim.y + t_off.y;
    let mut end = Vec3::new(ex, ground(ex, ez), ez);
    if in_cover {
        end.y += 0.35 + r2[0] * 0.8;
    }
    let arc_k = ARC[kind] * if shot.indirect { LOB } else { 1.0 };
    let launch = now + r1[2] * STAGGER[kind];
    let dist = start.distance(end);
    let flight = (dist / SPEED[kind] * (1.0 + arc_k)).max(0.05);
    let arc = dist * arc_k * (0.85 + 0.3 * r1[3]);
    let _ = r3;
    Missile {
        fire: false,
        next_flame: 0.0,
        start,
        end,
        launch,
        flight,
        arc,
        kind,
        stick: false,
    }
}

impl Missile {
    /// Position of the tip and direction of flight at fraction `t`.
    fn at(&self, t: f32) -> (Vec3, Vec3) {
        let t = t.clamp(0.0, 1.0);
        let lift = Vec3::Y * (self.arc * 4.0 * t * (1.0 - t));
        let position = self.start.lerp(self.end, t) + lift;
        let slope = (self.end - self.start) + Vec3::Y * (self.arc * 4.0 * (1.0 - 2.0 * t));
        (position, slope.normalize_or_zero())
    }
}

/// Half width and depth of a regiment.
fn half_extent(sim: &BattleSim, id: Option<u32>, default: (f32, f32)) -> (f32, f32) {
    id.and_then(|id| sim.units().iter().find(|u| u.id == id))
        .map_or(default, |u| {
            let (w, d) = u.extent();
            (w as f32 * 0.5, d as f32 * 0.5)
        })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_volleys(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    time: Res<Time>,
    mut volleys: ResMut<Volleys>,
    mut feed: ResMut<ShotFeed>,
    mut smoke: ResMut<SmokeRequests>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut batches: Query<&mut OrientedBatch>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    looks: Option<ResMut<FlipbookLooks>>,
) {
    let mut looks = looks;
    let started = std::time::Instant::now();
    let dt = if session.paused {
        0.0
    } else {
        time.delta_secs() * session.speed as f32
    };
    volleys.clock += dt;
    let now = volleys.clock;
    let camera_position = camera
        .iter()
        .next()
        .map_or(Vec3::new(0.0, 100.0, 0.0), |t| t.translation());
    if volleys.flying_batch.is_none() {
        let mesh = meshes.add(missile_mesh());
        let spawn = |commands: &mut Commands, name: &str, mesh: Handle<Mesh>| {
            commands
                .spawn((
                    Name::new(name.to_owned()),
                    OrientedBatch {
                        mesh_handle: mesh,
                        records: Arc::new(Vec::new()),
                        count: 0,
                        texture_handle: None,
                        params: MaterialParams {
                            base_color: [0.16, 0.10, 0.05, 1.0],
                            alpha_cutoff: 0.0,
                            roughness: 0.8,
                            metallic: 0.1,
                            double_sided: false,
                        },
                        version: 0,
                    },
                    DespawnOnExit(AppState::Battle),
                ))
                .id()
        };
        volleys.flying_batch = Some(spawn(&mut commands, "VolleyFlying", mesh.clone()));
        volleys.stuck_batch = Some(spawn(&mut commands, "VolleyStuck", mesh));
    }
    let sim = session.sim_mut();
    let shots = sim.take_shots();
    let sim = &*sim;
    for shot in &shots {
        let Some(kind) = kind_index(shot.kind) else {
            // Engines: the bombards' muzzle smoke lingers.
            if matches!(shot.kind, MissileKind::Ball) {
                smoke.bombard.push(Vec3::new(
                    shot.from.0 as f32,
                    sim.field().height(shot.from.0, shot.from.1) as f32 + 1.0,
                    shot.from.1 as f32,
                ));
            }
            continue;
        };
        let from = Vec3::new(shot.from.0 as f32, 0.0, shot.from.1 as f32);
        let aim = Vec3::new(shot.aim.0 as f32, 0.0, shot.aim.1 as f32);
        let lod = if camera_position.distance((from + aim) * 0.5) < FAR_DISTANCE {
            1.0
        } else {
            0.3
        };
        let mut total = ((shot.missiles as f32 * ARROWS_PER_MAN * lod) as usize)
            .clamp(1, MAX_ARROWS_PER_VOLLEY);
        let room = MAX_FLYING.saturating_sub(volleys.flying.len());
        total = total.min(room);
        let stride = STUCK_STRIDE.max(total.div_ceil(STUCK_PER_VOLLEY));
        let shooter_half = half_extent(sim, Some(shot.shooter), (5.0, 1.5));
        let target_half = half_extent(sim, shot.target, (6.0, 1.5));
        volleys.seed += 1;
        let seed = volleys.seed;
        for i in 0..total {
            let mut missile =
                plan_missile(sim, shot, kind, seed, i, shooter_half, target_half, now);
            missile.stick = i % stride == 0;
            // Fire arrows: an even share of the volley burns, up to the cap.
            if shot.incendiary && kind == 0 {
                let burning = total.min(FIRE_ARROWS_PER_VOLLEY);
                missile.fire =
                    i % (total / burning).max(1) == 0 && i / (total / burning).max(1) < burning;
                missile.stick |= missile.fire;
            }
            volleys.flying.push(missile);
        }
        volleys.launched += total;
        volleys.volleys += 1;
        volleys.by_kind[kind] += total;
        if matches!(shot.kind, MissileKind::Bullet) {
            smoke.puffs.push(Vec3::new(
                shot.from.0 as f32,
                sim.field().height(shot.from.0, shot.from.1) as f32 + 1.4,
                shot.from.1 as f32,
            ));
        }
    }
    feed.shots = shots;

    // Positions of the missiles in the air; those that land leave a planted share behind.
    let mut flying_bytes: Vec<u8> = Vec::with_capacity(volleys.flying.len() * 32);
    let mut landed: Vec<([f32; 8], bool)> = Vec::new();
    let mut flames: Vec<Vec3> = Vec::new();
    let mut fires: Vec<Vec3> = Vec::new();
    let mut count = 0u32;
    volleys.flying.retain_mut(|m| {
        let t = (now - m.launch) / m.flight;
        if t < 0.0 {
            return true;
        }
        if t >= 1.0 {
            if m.fire {
                fires.push(m.end);
            }
            if m.stick {
                let (p, d) = m.at(1.0);
                landed.push(([p.x, p.y, p.z, SCALE[m.kind], d.x, d.y, d.z, 0.0], true));
            }
            return false;
        }
        let (p, d) = m.at(t);
        if m.fire && now >= m.next_flame {
            m.next_flame = now + FLAME_EVERY_S;
            // The flame sits on the shaft, behind the head.
            flames.push(p - d * 0.4);
        }
        // Beyond the draw range a bolt is a sub-pixel speck: keep it flying, do not pack it.
        if p.distance_squared(camera_position) <= DRAW_RANGE_M * DRAW_RANGE_M {
            pack(
                [p.x, p.y, p.z, SCALE[m.kind], d.x, d.y, d.z, 0.0],
                &mut flying_bytes,
            );
            count += 1;
        }
        true
    });
    // Flames of the burning arrows, and the small fire where they stand.
    if let Some(looks) = looks.as_deref_mut() {
        let mut shed = |position: Vec3, life: f32, size: f32, rise: f32, serial: usize| {
            if looks.live() >= FLAME_SPRITE_ROOM {
                return;
            }
            let sprite = SmokeSprite {
                position,
                velocity: Vec3::Y * rise,
                accel: Vec3::ZERO,
                damping: 0.0,
                life,
                size,
                spin: 0.0,
                spin_rate: 0.0,
                tint: None,
            };
            if let Some(entity) = spawn_fire_sprite(
                &mut commands,
                looks,
                FlipbookKind::Flame,
                &sprite,
                serial as f32 * 0.37 % 1.0,
            ) {
                commands
                    .entity(entity)
                    .insert(DespawnOnExit(AppState::Battle));
            }
        };
        for (n, position) in flames.iter().enumerate() {
            shed(*position, FLAME_LIFE_S, 0.7, 0.5, n);
        }
        for (n, position) in fires.iter().enumerate() {
            shed(*position + Vec3::Y * 0.1, LANDED_FIRE_S, 0.9, 0.3, n + 7);
        }
        volleys.flames_shed += flames.len();
        volleys.fires_lit += fires.len();
    }
    volleys.version += 1;
    let version = volleys.version;
    if let Some(entity) = volleys.flying_batch
        && let Ok(mut batch) = batches.get_mut(entity)
    {
        batch.records = Arc::new(flying_bytes);
        batch.count = count;
        batch.version = version;
    }
    if !landed.is_empty() {
        for (record, _) in landed {
            let mut bytes = Vec::with_capacity(32);
            pack(record, &mut bytes);
            if volleys.stuck_count < MAX_STUCK {
                volleys.stuck.extend_from_slice(&bytes);
                volleys.stuck_count += 1;
            } else {
                let at = volleys.stuck_next * 32;
                volleys.stuck[at..at + 32].copy_from_slice(&bytes);
            }
            volleys.stuck_next = (volleys.stuck_next + 1) % MAX_STUCK;
        }
        volleys.stuck_version += 1;
        let (records, stuck_count, stuck_version) = (
            Arc::new(volleys.stuck.clone()),
            volleys.stuck_count as u32,
            volleys.stuck_version,
        );
        if let Some(entity) = volleys.stuck_batch
            && let Ok(mut batch) = batches.get_mut(entity)
        {
            batch.records = records;
            batch.count = stuck_count;
            batch.version = stuck_version;
        }
    }
    let spent = started.elapsed().as_micros() as u64;
    volleys.peak_flying = volleys.peak_flying.max(volleys.flying.len());
    volleys.update_us_peak = volleys.update_us_peak.max(spent);
    volleys.update_us_total += spent;
    volleys.updates += 1;
}

// ---------------------------------------------------------------------------------------------
// Stakes and pavises (`update_fieldworks`, `_plant_stakes`, `_plant_pavises` of battle_volleys.gd).
// ---------------------------------------------------------------------------------------------

const MAX_STAKE_ROWS: usize = 60;
const MAX_PAVISE_ROWS: usize = 60;
/// Pavise rows a regiment plants at most, the distance it must move for a new row (m) and the
/// pivot angle that replants the last row at the new front (~30 degrees).
const PAVISE_ROWS_PER_UNIT: u8 = 3;
const PAVISE_STEP_M: f32 = 12.0;
const PAVISE_REPLANT_ANGLE: f32 = 0.52;

/// What a regiment has planted so far.
#[derive(Default)]
struct Planted {
    stakes: bool,
    /// Where the last pavise row stood, its facing and the rows planted.
    pavise_at: Option<Vec2>,
    pavise_facing: f32,
    rows: u8,
    /// Index in [`Fieldworks::pavise_rows`] of its last row.
    last_row: Option<usize>,
}

/// A planted row of pavises: the side that planted it (colour) and one record per pavise.
struct PaviseRow {
    side: SideId,
    records: Vec<[f32; 8]>,
}

/// Stakes and pavises standing on the field.
#[derive(Resource, Default)]
pub struct Fieldworks {
    planted: HashMap<u32, Planted>,
    stake_records: Vec<[f32; 8]>,
    stake_rows: usize,
    pavise_rows: Vec<PaviseRow>,
    /// Shaft, tip, attacker board, defender board, wood parts.
    batches: Option<[Entity; 5]>,
    version: u64,
    dirty: bool,
    pub replanted: usize,
}

impl Fieldworks {
    pub fn stakes(&self) -> usize {
        self.stake_records.len()
    }

    pub fn pavises(&self) -> usize {
        self.pavise_rows.iter().map(|r| r.records.len()).sum()
    }
}

/// Stake of an archer (`_stake_mesh`): a hexagonal debarked pole along +Z from 25 cm in the
/// ground to 1.5 m, the tip apart.
fn stake_shaft() -> Mesh {
    Mesh::from(Cylinder::new(0.055, 1.75).mesh().resolution(6))
        .rotated_by(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
        .translated_by(Vec3::Z * 0.625)
}

/// Hatchet-cut point of the stake, fresh wood (`fresh` of `_stake_mesh`).
fn stake_tip() -> Mesh {
    Mesh::from(Cone::new(0.047, 0.36).mesh().resolution(6))
        .rotated_by(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
        .translated_by(Vec3::Z * 1.68)
}

/// Pavise board (`_pavise_mesh`): 0.6 x 1.15 m, 5 cm thick, faces +Z, standing on the ground.
fn pavise_board() -> Mesh {
    Mesh::from(Cuboid::new(0.6, 1.15, 0.05)).translated_by(Vec3::new(0.0, 0.6, 0.0))
}

/// Central strip and prop of the pavise.
fn pavise_wood() -> Mesh {
    let mut strip =
        Mesh::from(Cuboid::new(0.14, 1.0, 0.02)).translated_by(Vec3::new(0.0, 0.6, 0.03));
    let prop = Mesh::from(Cuboid::new(0.04, 0.9, 0.04)).translated_by(Vec3::new(0.0, 0.45, -0.3));
    strip.merge(&prop).expect("same layout");
    strip
}

fn fieldwork_batch(mesh: Handle<Mesh>, color: [f32; 4]) -> OrientedBatch {
    OrientedBatch {
        mesh_handle: mesh,
        records: Arc::new(Vec::new()),
        count: 0,
        texture_handle: None,
        params: MaterialParams {
            base_color: color,
            alpha_cutoff: 0.0,
            roughness: 0.9,
            metallic: 0.0,
            double_sided: false,
        },
        version: 0,
    }
}

/// Records of a row of `n` stakes in front of a regiment (`_plant_stakes`).
fn stake_row(
    unit_id: u32,
    centre: Vec2,
    facing: f32,
    (width, depth): (f32, f32),
    ground: &dyn Fn(f32, f32) -> f32,
) -> Vec<[f32; 8]> {
    let forward = Vec2::new(facing.sin(), facing.cos());
    let right = Vec2::new(facing.cos(), -facing.sin());
    let front = centre + forward * (depth * 0.5 + 1.5);
    let n = ((width / 1.1) as usize).min(32);
    let mut records = Vec::with_capacity(n * 2);
    for row in 0..2 {
        for k in 0..n {
            let h = hash4(u64::from(unit_id) * 4099 + (row * 64 + k) as u64);
            let lateral =
                (k as f32 - (n as f32 - 1.0) * 0.5) * 1.1 + if row == 1 { 0.55 } else { 0.0 };
            let p = front + right * (lateral + (h[0] - 0.5) * 0.3) + forward * (row as f32 * 0.8);
            // Pointing at the enemy, planted at about 40 degrees above the horizon (50 from
            // the vertical, a few degrees either side), a little crooked and uneven.
            let lean = (50.0 + (h[1] - 0.5) * 12.0).to_radians();
            let yaw = facing + (h[2] - 0.5) * 0.24;
            let scale = 0.88 + 0.22 * h[3];
            records.push([
                p.x,
                ground(p.x, p.y),
                p.y,
                scale,
                yaw.sin() * lean.sin(),
                lean.cos(),
                yaw.cos() * lean.sin(),
                0.0,
            ]);
        }
    }
    records
}

/// Records of a row of pavises in front of a regiment (`_plant_pavises`).
fn pavise_row(
    unit_id: u32,
    row_index: u8,
    centre: Vec2,
    facing: f32,
    (width, depth): (f32, f32),
    ground: &dyn Fn(f32, f32) -> f32,
) -> Vec<[f32; 8]> {
    let forward = Vec2::new(facing.sin(), facing.cos());
    let right = Vec2::new(facing.cos(), -facing.sin());
    let front = centre + forward * (depth * 0.5 + 0.9);
    let n = ((width / 1.05) as usize).min(36);
    (0..n)
        .map(|k| {
            let h = hash4(u64::from(unit_id) * 7919 + u64::from(row_index) * 131 + k as u64);
            let p = front
                + right * ((k as f32 - (n as f32 - 1.0) * 0.5) * 1.05)
                + forward * ((h[0] - 0.5) * 0.24);
            // Leans back against its prop: the top towards the men behind it.
            let lean = (12.0 + (h[1] - 0.5) * 6.0).to_radians();
            let yaw = facing + (h[2] - 0.5) * 0.16;
            [
                p.x,
                ground(p.x, p.y),
                p.y,
                1.0,
                yaw.sin() * lean.cos(),
                lean.sin(),
                yaw.cos() * lean.cos(),
                0.0,
            ]
        })
        .collect()
}

fn angle_difference(a: f32, b: f32) -> f32 {
    (b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn to_bytes(records: impl Iterator<Item = [f32; 8]>) -> (Vec<u8>, u32) {
    let mut bytes = Vec::new();
    let mut count = 0;
    for record in records {
        pack(record, &mut bytes);
        count += 1;
    }
    (bytes, count)
}

/// Plants the stakes of the archers who stood still and the pavises of the crossbowmen behind
/// them, once each, and keeps them for the battle.
pub(super) fn update_fieldworks(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    mut fieldworks: ResMut<Fieldworks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut batches: Query<&mut OrientedBatch>,
) {
    if fieldworks.batches.is_none() {
        let shaft = meshes.add(stake_shaft());
        let tip = meshes.add(stake_tip());
        let board = meshes.add(pavise_board());
        let wood = meshes.add(pavise_wood());
        let mut spawn = |name: &str, mesh: Handle<Mesh>, color: [f32; 4]| {
            commands
                .spawn((
                    Name::new(name.to_owned()),
                    fieldwork_batch(mesh, color),
                    DespawnOnExit(AppState::Battle),
                ))
                .id()
        };
        // Colours: wood of the stake, fresh point, boards tinted by side (the white-ish board
        // 0.82 / 0.78 / 0.70 times the side colour of the Godot instance), dark wood.
        let ids = [
            spawn("StakeShafts", shaft, [0.32, 0.24, 0.15, 1.0]),
            spawn("StakeTips", tip, [0.74, 0.62, 0.43, 1.0]),
            spawn(
                "PaviseAttacker",
                board.clone(),
                [0.82 * 0.30, 0.78 * 0.36, 0.70 * 0.62, 1.0],
            ),
            spawn(
                "PaviseDefender",
                board,
                [0.82 * 0.62, 0.78 * 0.24, 0.70 * 0.20, 1.0],
            ),
            spawn("PaviseWood", wood, [0.35, 0.28, 0.18, 1.0]),
        ];
        fieldworks.batches = Some(ids);
    }
    let sim = session.sim_mut();
    let field = sim.field().clone();
    let ground = |x: f32, z: f32| field.height(f64::from(x), f64::from(z)) as f32;
    let fw = &mut *fieldworks;
    for unit in sim.units() {
        if !unit.present() {
            continue;
        }
        let (width, depth) = unit.extent();
        let size = (width as f32, depth as f32);
        let centre = Vec2::new(unit.x as f32, unit.z as f32);
        let facing = unit.facing as f32;
        let planted = fw.planted.entry(unit.id).or_default();
        if unit.stakes_planted && !planted.stakes {
            planted.stakes = true;
            if fw.stake_rows < MAX_STAKE_ROWS {
                fw.stake_rows += 1;
                fw.stake_records
                    .extend(stake_row(unit.id, centre, facing, size, &ground));
                fw.dirty = true;
            }
        }
        let cover = unit.pavise.is_some()
            || (unit.has(Ability::Pavise) && unit.state != UnitState::Marching);
        let still = !matches!(unit.state, UnitState::Marching | UnitState::Charging);
        if !(cover && still) {
            continue;
        }
        let moved = planted
            .pavise_at
            .is_none_or(|last| centre.distance(last) > PAVISE_STEP_M);
        if moved && planted.rows < PAVISE_ROWS_PER_UNIT {
            if fw.pavise_rows.len() >= MAX_PAVISE_ROWS {
                continue;
            }
            planted.pavise_at = Some(centre);
            planted.pavise_facing = facing;
            planted.rows += 1;
            planted.last_row = Some(fw.pavise_rows.len());
            fw.pavise_rows.push(PaviseRow {
                side: unit.side,
                records: pavise_row(unit.id, planted.rows, centre, facing, size, &ground),
            });
            fw.dirty = true;
        } else if let Some(row) = planted.last_row
            && angle_difference(planted.pavise_facing, facing).abs() > PAVISE_REPLANT_ANGLE
        {
            // Pivot on the spot: the last row follows the new front.
            planted.pavise_at = Some(centre);
            planted.pavise_facing = facing;
            let slots = fw.pavise_rows[row].records.len();
            let mut records = pavise_row(unit.id, planted.rows, centre, facing, size, &ground);
            records.truncate(slots);
            fw.pavise_rows[row].records = records;
            fw.replanted += 1;
            fw.dirty = true;
        }
    }
    if !fw.dirty {
        return;
    }
    fw.dirty = false;
    fw.version += 1;
    let version = fw.version;
    let Some(ids) = fw.batches else { return };
    let (stake_bytes, stake_count) = to_bytes(fw.stake_records.iter().copied());
    let stake_bytes = Arc::new(stake_bytes);
    let boards = |side: SideId| {
        to_bytes(
            fw.pavise_rows
                .iter()
                .filter(move |row| row.side == side)
                .flat_map(|row| row.records.iter().copied()),
        )
    };
    let (attacker_bytes, attacker_count) = boards(SideId::Attacker);
    let (defender_bytes, defender_count) = boards(SideId::Defender);
    let (wood_bytes, wood_count) = to_bytes(
        fw.pavise_rows
            .iter()
            .flat_map(|r| r.records.iter().copied()),
    );
    let sets = [
        (ids[0], stake_bytes.clone(), stake_count),
        (ids[1], stake_bytes, stake_count),
        (ids[2], Arc::new(attacker_bytes), attacker_count),
        (ids[3], Arc::new(defender_bytes), defender_count),
        (ids[4], Arc::new(wood_bytes), wood_count),
    ];
    for (entity, records, count) in sets {
        if let Ok(mut batch) = batches.get_mut(entity) {
            batch.records = records;
            batch.count = count;
            batch.version = version;
        }
    }
}

fn clear_fieldworks(mut fieldworks: ResMut<Fieldworks>) {
    *fieldworks = Fieldworks::default();
}

fn clear_volleys(mut volleys: ResMut<Volleys>, mut feed: ResMut<ShotFeed>) {
    *volleys = Volleys::default();
    feed.shots.clear();
}

fn dump_volleys(world: &World) -> (String, serde_json::Value) {
    let Some(v) = world.get_resource::<Volleys>() else {
        return ("battle_volleys".into(), serde_json::Value::Null);
    };
    (
        "battle_volleys".into(),
        serde_json::json!({
            "volleys": v.volleys,
            "launched": v.launched,
            "flying": v.flying.len(),
            "peak_flying": v.peak_flying,
            "update_us_peak": v.update_us_peak,
            "update_us_mean": v.update_us_total.checked_div(v.updates).unwrap_or(0),
            "stuck": v.stuck_count,
            "flames_shed": v.flames_shed,
            "fires_lit": v.fires_lit,
            "stakes": world.get_resource::<Fieldworks>().map_or(0, Fieldworks::stakes),
            "pavises": world.get_resource::<Fieldworks>().map_or(0, Fieldworks::pavises),
            "pavise_replants": world.get_resource::<Fieldworks>().map_or(0, |f| f.replanted),
            "by_kind": { "arrow": v.by_kind[0], "bolt": v.by_kind[1], "bullet": v.by_kind[2], "javelin": v.by_kind[3] },
        }),
    )
}

pub struct VolleysPlugin;

impl Plugin for VolleysPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Volleys>()
            .init_resource::<Fieldworks>()
            .init_resource::<ShotFeed>()
            .register_dump(dump_volleys)
            .add_systems(
                Update,
                update_volleys
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet),
            )
            .add_systems(
                Update,
                update_fieldworks
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet),
            )
            .add_systems(OnExit(AppState::Battle), (clear_volleys, clear_fieldworks));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missile_path_starts_and_ends_at_its_points() {
        let m = Missile {
            fire: false,
            next_flame: 0.0,
            start: Vec3::new(0.0, 1.5, 0.0),
            end: Vec3::new(100.0, 0.0, 0.0),
            launch: 0.0,
            flight: 2.0,
            arc: 8.0,
            kind: 0,
            stick: false,
        };
        let (p0, d0) = m.at(0.0);
        let (p1, d1) = m.at(1.0);
        let (mid, dm) = m.at(0.5);
        assert!((p0 - m.start).length() < 1e-4);
        assert!((p1 - m.end).length() < 1e-4);
        assert!(mid.y > 8.0, "apex above the chord");
        assert!(d0.y > 0.0 && d1.y < 0.0, "rises then falls");
        assert!(dm.y.abs() < 0.05, "nearly level at the apex");
    }
}
