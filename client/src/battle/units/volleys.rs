//! Volleys of arrows, bolts, bullets and javelins: every shot resolved by the simulation
//! (`BattleSim::take_shots`) becomes a flight of instanced missiles whose paths are computed on
//! the CPU from the shot (shooter and target regiments, missile count). A share of the missiles
//! stays planted in the ground or in the pavises and stakes for the rest of the battle.
//!
//! Port of game/scripts/battle/battle_volleys.gd (`on_shot`, `arrow_landing`, the constants of
//! `battle_volley.gdshaderinc`). The GPU path of the shader is a CPU parabola here, the stuck
//! layer is a ring buffer of oriented records. Not ported: fire arrows' flames, the stakes and
//! pavises drawn in front of regiments, the sounds (`sound_event`: the audio lot reads the
//! [`ShotFeed`]), missiles that stay planted for `STICK_HOLD` seconds before fading (those are
//! removed at landing, only the sampled share stays).

use std::sync::Arc;

use bevy::prelude::*;
use sim_battle::{BattleSim, MissileKind, ShotCover, ShotEvent};

use super::corpses::hash4;
use super::instancing::MaterialParams;
use super::oriented::OrientedBatch;
use super::smoke::SmokeRequests;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;

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

struct Missile {
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
) {
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
    let mut count = 0u32;
    volleys.flying.retain(|m| {
        let t = (now - m.launch) / m.flight;
        if t < 0.0 {
            return true;
        }
        if t >= 1.0 {
            if m.stick {
                let (p, d) = m.at(1.0);
                landed.push(([p.x, p.y, p.z, SCALE[m.kind], d.x, d.y, d.z, 0.0], true));
            }
            return false;
        }
        let (p, d) = m.at(t);
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
            "by_kind": { "arrow": v.by_kind[0], "bolt": v.by_kind[1], "bullet": v.by_kind[2], "javelin": v.by_kind[3] },
        }),
    )
}

pub struct VolleysPlugin;

impl Plugin for VolleysPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Volleys>()
            .init_resource::<ShotFeed>()
            .register_dump(dump_volleys)
            .add_systems(
                Update,
                update_volleys
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet),
            )
            .add_systems(OnExit(AppState::Battle), clear_volleys);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missile_path_starts_and_ends_at_its_points() {
        let m = Missile {
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
