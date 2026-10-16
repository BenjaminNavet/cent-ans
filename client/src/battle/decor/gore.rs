//! Blood sprays and severed limbs of the "full" blood level (port of
//! game/scripts/battle/battle_gore.gd + the spray / `sever` calls of battle_soldiers.gd, shader
//! `battle_gore.gdshader`). Rendering only: who falls comes from [`Losses`] (decor/flatten.rs).
//!
//! Every intensity and cap is read from `levels`, `sprays`, `severed`, `pieces`,
//! `critical_chance`, `weapons`, `deaths` of `data/fx/battle_gore.json`; the setting `blood`
//! (0 off, 1 moderate, 2 full) picks the level (`off`, `moderate`, `full`). Droplets are
//! ballistic (`origin + v t - g t^2 / 2`), drawn as camera-facing hexagons in flight and flat
//! stains once landed, all in one mesh rebuilt each frame while something is alive. Severed
//! pieces (head, forearm, leg) are small meshes thrown the same way, lying `rest_seconds`, with
//! a red cut end. The pool size of the persistent decals (`blood.rs`) reads `levels.stains`.
//!
//! Not ported: the droplets of knocked-down men and of the wounded (no such events in the
//! client yet), the `heavy_charge_mass` test (the sim keeps no charge mass: a fatal charge uses
//! the `heavy_charge` chance), the order of `pieces` weights (alphabetical here).
//!
//! Dump section `battle_gore`.

use std::collections::{BTreeMap, HashMap};
use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use serde::Deserialize;

use super::flatten::{Losses, collect_losses};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::battle::units::corpses::hash4;

const GRAVITY: f32 = 9.8;
/// Seconds a landed droplet stays as a stain.
const GROUND_SECONDS: f32 = 5.0;
const LEVEL_NAMES: [&str; 3] = ["off", "moderate", "full"];
/// Radius of a droplet (fraction of its size).
const DROP_RADIUS: f32 = 0.45;

/// Intensities of one blood level (`levels` of battle_gore.json).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub struct BloodLevel {
    #[serde(default)]
    pub stains: f32,
    #[serde(default)]
    pub sprays: f32,
    #[serde(default)]
    pub corpse_blood: f32,
    #[serde(default)]
    pub dismember: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SpraySettings {
    pub max_droplets: usize,
    pub droplets_per_death: f32,
    pub droplets_per_critical: f32,
    pub speed: [f32; 2],
    pub lifetime: f32,
    pub max_distance_m: f32,
}

impl Default for SpraySettings {
    fn default() -> Self {
        Self {
            max_droplets: 6144,
            droplets_per_death: 7.0,
            droplets_per_critical: 18.0,
            speed: [1.2, 4.2],
            lifetime: 1.4,
            max_distance_m: 220.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct SeveredSettings {
    pub max_pieces: usize,
    pub speed: [f32; 2],
    pub rest_seconds: f32,
}

impl Default for SeveredSettings {
    fn default() -> Self {
        Self {
            max_pieces: 256,
            speed: [2.0, 5.0],
            rest_seconds: 40.0,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct DeathGore {
    #[serde(default)]
    pub impulse: [f32; 2],
    #[serde(default)]
    pub critical: f32,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PieceWeights {
    #[serde(default)]
    pub foot: BTreeMap<String, f32>,
    #[serde(default)]
    pub mounted: BTreeMap<String, f32>,
}

/// The parts of `data/fx/battle_gore.json` this module reads.
#[derive(Resource, Clone, Debug, Default, Deserialize)]
pub struct GoreData {
    #[serde(default)]
    pub levels: HashMap<String, BloodLevel>,
    #[serde(default)]
    pub sprays: SpraySettings,
    #[serde(default)]
    pub severed: SeveredSettings,
    #[serde(default)]
    pub deaths: HashMap<String, DeathGore>,
    #[serde(default)]
    pub weapons: HashMap<String, String>,
    #[serde(default)]
    pub critical_chance: HashMap<String, f32>,
    #[serde(default)]
    pub pieces: PieceWeights,
}

impl GoreData {
    pub fn load(data_dir: &std::path::Path) -> Self {
        std::fs::read_to_string(data_dir.join("fx/battle_gore.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Intensities of the `blood` setting (0 off, 1 moderate, 2 full); an absent level is "off".
    pub fn level(&self, blood: u32) -> BloodLevel {
        self.levels
            .get(LEVEL_NAMES[blood.min(2) as usize])
            .copied()
            .unwrap_or(BloodLevel {
                stains: 0.0,
                sprays: 0.0,
                corpse_blood: 0.0,
                dismember: false,
            })
    }

    /// Chance of a dismembering blow for a death by `cause` struck by a `killer_type` unit.
    pub fn critical_chance(
        &self,
        cause: &str,
        killer_type: Option<&str>,
        mounted_charge: bool,
    ) -> f32 {
        let chance = |key: &str| self.critical_chance.get(key).copied().unwrap_or(0.0);
        if cause == "ball" || cause == "stone" {
            return chance(cause);
        }
        let weapon = killer_type
            .and_then(|t| self.weapons.get(t))
            .map_or("sword", String::as_str);
        let mut value = chance(weapon);
        if cause == "charge" && mounted_charge {
            value = value.max(chance("heavy_charge"));
        }
        let scale = self
            .deaths
            .get(cause)
            .or_else(|| self.deaths.get("other"))
            .map_or(0.0, |d| d.critical);
        value * scale
    }

    /// Part cut by a critical blow, picked by `roll` in `0..1` over the weights.
    pub fn pick_part(&self, mounted: bool, roll: f32) -> Option<&str> {
        let weights = if mounted {
            &self.pieces.mounted
        } else {
            &self.pieces.foot
        };
        let total: f32 = weights.values().sum();
        let mut left = roll * total;
        for (part, weight) in weights {
            left -= weight;
            if left <= 0.0 {
                return Some(part);
            }
        }
        None
    }
}

/// Kind of mesh of a part: 0 head, 1 arm, 2 leg.
pub fn piece_kind(part: &str) -> Option<usize> {
    match part {
        "head" => Some(0),
        "arm_r" | "arm_l" => Some(1),
        "leg_r" | "leg_l" => Some(2),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct Drop {
    origin: Vec3,
    vel: Vec3,
    height: f32,
    size: f32,
    t0: f32,
    /// Seconds to the ground.
    land: f32,
    /// Ground height under the landing point minus the origin's.
    drop_y: f32,
}

impl Drop {
    fn new(origin: Vec3, vel: Vec3, height: f32, size: f32, t0: f32, landing_y: f32) -> Self {
        let land = landing_time(vel.y, height);
        Self {
            origin,
            vel,
            height,
            size,
            t0,
            land,
            drop_y: landing_y - origin.y,
        }
    }

    /// World position at battle time `now` (clamped to the landing).
    fn position(&self, now: f32) -> Vec3 {
        let t = (now - self.t0).clamp(0.0, self.land);
        let share = if self.land > 0.0 { t / self.land } else { 1.0 };
        self.origin
            + Vec3::new(
                self.vel.x * t,
                self.height + self.vel.y * t - 0.5 * GRAVITY * t * t + self.drop_y * share,
                self.vel.z * t,
            )
    }
}

/// Time for `h0 + vy t - g t^2 / 2 = 0` to reach the ground.
fn landing_time(vy: f32, h0: f32) -> f32 {
    (vy + (vy * vy + 2.0 * GRAVITY * h0.max(0.0)).sqrt()) / GRAVITY
}

struct Piece {
    drop: Drop,
    entity: Entity,
    id: f32,
}

#[derive(Component)]
struct GoreDrops;

/// A fall as the emitter needs it.
#[derive(Clone, Copy)]
pub struct FallEvent {
    pub pos: Vec2,
    pub ground_y: f32,
    pub mounted: bool,
    /// Unit vector pointing away from the killer (or behind the victim).
    pub away: Vec2,
}

#[derive(Resource)]
pub struct GoreState {
    pub data: GoreData,
    pub clock: f32,
    counter: u64,
    drops: Vec<Drop>,
    drops_next: usize,
    pieces: [Vec<Piece>; 3],
    pieces_next: [usize; 3],
    pub drops_emitted: usize,
    pub pieces_emitted: usize,
    pub by_part: BTreeMap<String, usize>,
    pub active_drops: usize,
    pub rebuilds: usize,
    last_drawn: bool,
}

impl GoreState {
    pub fn new(data: GoreData) -> Self {
        Self {
            data,
            clock: 0.0,
            counter: 1337,
            drops: Vec::new(),
            drops_next: 0,
            pieces: Default::default(),
            pieces_next: [0; 3],
            drops_emitted: 0,
            pieces_emitted: 0,
            by_part: BTreeMap::new(),
            active_drops: 0,
            rebuilds: 0,
            last_drawn: false,
        }
    }

    fn rnd(&mut self) -> f32 {
        self.counter += 1;
        hash4(self.counter)[0]
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.rnd()
    }

    /// `count` droplets (before the level factor) thrown from `pos` towards `away`.
    pub fn spray(
        &mut self,
        level: BloodLevel,
        event: &FallEvent,
        count: f32,
        height: f32,
        strength: f32,
        ground: &dyn Fn(Vec2) -> f32,
    ) {
        let cap = self.data.sprays.max_droplets;
        if level.sprays <= 0.0 || cap <= 1 || count <= 0.0 {
            return;
        }
        let n = (count * level.sprays).round() as usize;
        let [lo, hi] = self.data.sprays.speed;
        let origin = Vec3::new(event.pos.x, event.ground_y, event.pos.y);
        for _ in 0..n {
            let angle = self.range(-0.9, 0.9);
            let speed = self.range(lo, hi) * strength;
            let dir = Vec2::from_angle(angle).rotate(event.away);
            let vel = Vec3::new(
                dir.x * speed,
                self.range(0.6, 2.8) * strength,
                dir.y * speed,
            );
            let h = height + self.range(-0.2, 0.2);
            let size = self.range(0.025, 0.06);
            let t0 = self.clock + self.range(0.0, 0.08);
            let land = landing_time(vel.y, h);
            let landing = Vec2::new(origin.x + vel.x * land, origin.z + vel.z * land);
            let drop = Drop::new(origin, vel, h, size, t0, ground(landing));
            if self.drops.len() < cap {
                self.drops.push(drop);
            } else {
                self.drops[self.drops_next % cap] = drop;
            }
            self.drops_next = (self.drops_next + 1) % cap;
            self.drops_emitted += 1;
        }
    }

    /// Plans what one fallen body leaves: a spray, and sometimes a severed piece. Returns the
    /// part to spawn (`None` without dismemberment); the caller creates the entity.
    pub fn on_fall(
        &mut self,
        level: BloodLevel,
        event: &FallEvent,
        critical: f32,
        ground: &dyn Fn(Vec2) -> f32,
    ) -> Option<(String, Vec3, f32)> {
        let per_death = self.data.sprays.droplets_per_death;
        self.spray(
            level,
            event,
            per_death,
            if event.mounted { 2.0 } else { 1.2 },
            1.0,
            ground,
        );
        if !level.dismember || critical <= 0.0 || self.rnd() >= critical {
            return None;
        }
        let roll = self.rnd();
        let part = self.data.pick_part(event.mounted, roll)?.to_owned();
        let height = match part.as_str() {
            "head" => 1.6,
            "arm_r" | "arm_l" => 1.25,
            _ => 0.45,
        } + if event.mounted { 1.0 } else { 0.0 };
        let [lo, hi] = self.data.severed.speed;
        let angle = self.range(-0.6, 0.6);
        let speed = self.range(lo, hi);
        let dir = Vec2::from_angle(angle).rotate(event.away);
        let vel = Vec3::new(dir.x * speed, self.range(1.5, 3.5), dir.y * speed);
        let per_critical = self.data.sprays.droplets_per_critical;
        self.spray(level, event, per_critical, height, 1.2, ground);
        self.pieces_emitted += 1;
        *self.by_part.entry(part.clone()).or_default() += 1;
        Some((part, vel, height))
    }
}

/// Live handles of the piece meshes and materials.
#[derive(Resource)]
struct PieceAssets {
    meshes: [Handle<Mesh>; 3],
    /// `[kind][tint step]`.
    materials: [[Handle<StandardMaterial>; 4]; 3],
}

pub struct GorePlugin;

impl Plugin for GorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Battle), setup_gore)
            .add_systems(
                Update,
                (emit_gore, update_gore)
                    .chain()
                    .after(collect_losses)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<GoreState>)
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
                commands.remove_resource::<GoreState>();
                commands.remove_resource::<PieceAssets>();
            })
            .register_dump(dump_gore);
    }
}

/// Piece mesh with the cut end (`-Y`) tinted red through vertex colours.
fn piece_mesh(kind: usize) -> Mesh {
    let mut mesh = if kind == 0 {
        Sphere::new(0.11).mesh().uv(10, 6)
    } else {
        let (radius, total) = if kind == 1 {
            (0.045, 0.42)
        } else {
            (0.06, 0.5)
        };
        Mesh::from(
            Capsule3d::new(radius, total - 2.0 * radius)
                .mesh()
                .rings(2)
                .latitudes(8)
                .longitudes(8),
        )
    };
    let colors: Vec<[f32; 4]> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) => positions
            .iter()
            .map(|p| {
                let cut = 1.0 - smoothstep(-0.12, -0.02, p[1]);
                [1.0 - 0.7 * cut, 1.0 - 0.97 * cut, 1.0 - 0.97 * cut, 1.0]
            })
            .collect(),
        _ => Vec::new(),
    };
    if !colors.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Base colour of a piece for tint step `step` (0..4): head skin / steel, limbs cloth.
fn piece_color(kind: usize, step: usize) -> Color {
    let tint = step as f32 / 3.0;
    let mix = |a: [f32; 3], b: [f32; 3]| {
        Color::srgb(
            a[0] + (b[0] - a[0]) * tint,
            a[1] + (b[1] - a[1]) * tint,
            a[2] + (b[2] - a[2]) * tint,
        )
    };
    if kind == 0 {
        if tint > 0.55 {
            Color::srgb(0.42, 0.42, 0.44)
        } else {
            mix([0.55, 0.38, 0.28], [0.3, 0.19, 0.12])
        }
    } else {
        mix([0.3, 0.22, 0.12], [0.16, 0.16, 0.2])
    }
}

fn setup_gore(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let data = crate::paths::find_root()
        .map(|root| GoreData::load(&crate::paths::data_dir(&root)))
        .unwrap_or_default();
    commands.insert_resource(GoreState::new(data));
    let mesh_handles = [0, 1, 2].map(|k| meshes.add(piece_mesh(k)));
    let material_handles = [0, 1, 2].map(|k| {
        [0, 1, 2, 3].map(|step| {
            materials.add(StandardMaterial {
                base_color: piece_color(k, step),
                perceptual_roughness: 0.6,
                ..default()
            })
        })
    });
    commands.insert_resource(PieceAssets {
        meshes: mesh_handles,
        materials: material_handles,
    });
    // No mesh until the first droplet flies: an empty mesh asset makes the render allocator warn
    // (`Mesh … contains no vertices`, every battle) and draws nothing.
    commands.spawn((
        Name::new("GoreDrops"),
        GoreDrops,
        Visibility::Hidden,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.005, 0.004),
            perceptual_roughness: 0.25,
            reflectance: 0.6,
            cull_mode: None,
            ..default()
        })),
        Transform::default(),
        NoFrustumCulling,
        NotShadowCaster,
        DespawnOnExit(AppState::Battle),
    ));
}

/// Turns the bodies that fell this frame into sprays and severed pieces.
#[allow(clippy::too_many_arguments)]
fn emit_gore(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    losses: Res<Losses>,
    mut gore: ResMut<GoreState>,
    assets: Option<Res<PieceAssets>>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    settings: Option<Res<crate::settings::Settings>>,
) {
    let level = gore.data.level(settings.map_or(1, |s| s.blood));
    if level.sprays <= 0.0 && !level.dismember {
        return;
    }
    if losses.falls.is_empty() {
        return;
    }
    let camera_pos = camera.iter().next().map(|t| t.translation());
    let max_distance = gore.data.sprays.max_distance_m;
    let sim = session.sim_mut();
    let units: HashMap<u32, _> = sim.units().iter().map(|u| (u.id, u)).collect();
    let field = sim.field();
    let ground = |p: Vec2| field.height(f64::from(p.x), f64::from(p.y)) as f32;
    let mut spawned = Vec::new();
    for fall in &losses.falls {
        let ground_y = ground(fall.pos);
        if let Some(cam) = camera_pos
            && cam.distance(Vec3::new(fall.pos.x, ground_y, fall.pos.y)) > max_distance
        {
            continue;
        }
        let Some(unit) = units.get(&fall.unit) else {
            continue;
        };
        let cause = unit.loss_cause.key();
        if cause == "fire" {
            // Burnt men do not bleed.
            continue;
        }
        let killer = unit.loss_by.and_then(|id| units.get(&id));
        let impulse = gore
            .data
            .deaths
            .get(cause)
            .or_else(|| gore.data.deaths.get("other"))
            .map_or([0.0, 0.0], |d| d.impulse);
        let roll = gore.rnd();
        let speed = impulse[0] + (impulse[1] - impulse[0]) * roll;
        let behind = Vec2::new(-(unit.facing as f32).sin(), -(unit.facing as f32).cos());
        let mut away = behind;
        if let Some(k) = killer {
            let d = fall.pos - Vec2::new(k.x as f32, k.z as f32);
            if speed > 0.3 && d.length() > 0.5 {
                away = d.normalize();
            }
        }
        let event = FallEvent {
            pos: fall.pos,
            ground_y,
            mounted: fall.cavalry,
            away,
        };
        let heavy = cause == "charge";
        let critical = gore.data.critical_chance(
            cause,
            killer.map(|k| &*k.unit_type),
            heavy && killer.is_some_and(|k| k.category == data_model::UnitCategory::Cavalry),
        );
        if let Some((part, vel, height)) = gore.on_fall(level, &event, critical, &ground) {
            spawned.push((event, part, vel, height));
        }
    }
    let Some(assets) = assets else {
        return;
    };
    for (event, part, vel, height) in spawned {
        let Some(kind) = piece_kind(&part) else {
            continue;
        };
        let cap = match kind {
            0 => gore.data.severed.max_pieces / 2,
            _ => gore.data.severed.max_pieces / 4,
        };
        if cap <= 1 {
            continue;
        }
        let origin = Vec3::new(event.pos.x, event.ground_y, event.pos.y);
        let land = landing_time(vel.y, height);
        let landing = Vec2::new(origin.x + vel.x * land, origin.z + vel.z * land);
        let drop = Drop::new(origin, vel, height, 1.0, gore.clock, ground(landing));
        let step = ((gore.rnd() * 4.0) as usize).min(3);
        let id = gore.pieces_emitted as f32;
        let entity = commands
            .spawn((
                Name::new(format!("Severed_{part}")),
                Mesh3d(assets.meshes[kind].clone()),
                MeshMaterial3d(assets.materials[kind][step].clone()),
                Transform::from_translation(origin + Vec3::Y * height),
                NotShadowCaster,
                DespawnOnExit(AppState::Battle),
            ))
            .id();
        let slot = gore.pieces_next[kind];
        let list = &mut gore.pieces[kind];
        if list.len() < cap {
            list.push(Piece { drop, entity, id });
        } else {
            let old = std::mem::replace(&mut list[slot % cap], Piece { drop, entity, id });
            commands.entity(old.entity).try_despawn();
        }
        gore.pieces_next[kind] = (slot + 1) % cap;
    }
}

#[allow(clippy::too_many_arguments)]
fn update_gore(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<BattleSession>,
    mut gore: ResMut<GoreState>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    mut transforms: Query<&mut Transform>,
    mut drops: Query<(Entity, Option<&Mesh3d>, &mut Visibility), With<GoreDrops>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !session.paused {
        gore.clock += time.delta_secs() * session.speed as f32;
    }
    let now = gore.clock;
    let rest = gore.data.severed.rest_seconds;
    // Severed pieces: tumbling flight, lying, sinking at the end, then gone.
    for kind in 0..3 {
        let mut gone = Vec::new();
        for (i, piece) in gore.pieces[kind].iter().enumerate() {
            let t = now - piece.drop.t0;
            let Ok(mut transform) = transforms.get_mut(piece.entity) else {
                continue;
            };
            if t > rest {
                gone.push(i);
                continue;
            }
            let p = piece.drop.position(now);
            let sink = smoothstep(rest * 0.8, rest, t) * 0.4;
            let tt = t.clamp(0.0, piece.drop.land);
            let axis = Vec3::new(
                (piece.id * 12.99).sin().abs().fract() + 0.1,
                (piece.id * 78.23).sin().abs().fract(),
                (piece.id * 3.71).sin().abs().fract(),
            )
            .normalize();
            transform.translation = p + Vec3::Y * (0.06 - sink);
            transform.rotation = Quat::from_axis_angle(axis, tt * 9.0);
        }
        for i in gone.into_iter().rev() {
            let piece = gore.pieces[kind].remove(i);
            commands.entity(piece.entity).try_despawn();
            gore.pieces_next[kind] = gore.pieces[kind].len();
        }
    }
    // Droplets: one mesh.
    let camera_transform = camera.iter().next();
    let (right, up, forward) = camera_transform.map_or((Vec3::X, Vec3::Y, Vec3::NEG_Z), |t| {
        (t.right().as_vec3(), t.up().as_vec3(), t.forward().as_vec3())
    });
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut active = 0;
    for drop in &gore.drops {
        let t = now - drop.t0;
        if t < 0.0 || t >= drop.land + GROUND_SECONDS {
            continue;
        }
        active += 1;
        let fade = 1.0
            - smoothstep(
                drop.land + GROUND_SECONDS * 0.4,
                drop.land + GROUND_SECONDS,
                t,
            );
        if fade < 0.35 {
            continue;
        }
        let centre = drop.position(now) + Vec3::Y * 0.015;
        let (axis_a, axis_b, normal, radius) = if t > drop.land {
            let grown = 1.0 + 1.2 * smoothstep(0.0, 0.4, t - drop.land);
            (
                Vec3::X,
                Vec3::Z,
                Vec3::Y,
                DROP_RADIUS * drop.size * grown * fade.sqrt(),
            )
        } else {
            (right, up, -forward, DROP_RADIUS * drop.size)
        };
        let base = positions.len() as u32;
        positions.push(centre.to_array());
        normals.push(normal.to_array());
        for k in 0..6 {
            let a = k as f32 / 6.0 * TAU;
            positions.push((centre + (axis_a * a.cos() + axis_b * a.sin()) * radius).to_array());
            normals.push(normal.to_array());
        }
        for k in 0..6u32 {
            indices.extend_from_slice(&[base, base + 1 + k, base + 1 + (k + 1) % 6]);
        }
    }
    gore.active_drops = active;
    if positions.is_empty() && !gore.last_drawn {
        return;
    }
    gore.last_drawn = !positions.is_empty();
    gore.rebuilds += 1;
    let Ok((entity, handle, mut visibility)) = drops.single_mut() else {
        return;
    };
    let Some(mesh) = drops_mesh(positions, normals, indices) else {
        // Nothing left in the air: hide the last mesh rather than upload an empty one.
        *visibility = Visibility::Hidden;
        return;
    };
    *visibility = Visibility::Inherited;
    match handle {
        Some(handle) => {
            let _ = meshes.insert(&handle.0, mesh);
        }
        None => {
            commands.entity(entity).try_insert(Mesh3d(meshes.add(mesh)));
        }
    }
}

/// The droplets' mesh, `None` when there is none to draw (never an empty mesh).
fn drops_mesh(positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, indices: Vec<u32>) -> Option<Mesh> {
    if positions.is_empty() || indices.is_empty() {
        return None;
    }
    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices)),
    )
}

fn dump_gore(world: &World) -> (String, serde_json::Value) {
    let value = world.get_resource::<GoreState>().map(|g| {
        let blood = world
            .get_resource::<crate::settings::Settings>()
            .map_or(1, |s| s.blood);
        let level = g.data.level(blood);
        serde_json::json!({
            "blood_setting": blood,
            "level": { "stains": level.stains, "sprays": level.sprays,
                "corpse_blood": level.corpse_blood, "dismember": level.dismember },
            "drops_emitted": g.drops_emitted, "drops_active": g.active_drops,
            "drop_capacity": g.data.sprays.max_droplets,
            "pieces_emitted": g.pieces_emitted,
            "pieces_alive": g.pieces.iter().map(Vec::len).sum::<usize>(),
            "by_part": g.by_part, "rebuilds": g.rebuilds, "clock": g.clock,
        })
    });
    ("battle_gore".to_owned(), serde_json::json!(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_droplet_mesh_without_droplets() {
        assert!(drops_mesh(Vec::new(), Vec::new(), Vec::new()).is_none());
        let mesh = drops_mesh(
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            vec![[0.0, 1.0, 0.0]; 3],
            vec![0, 1, 2],
        )
        .expect("one droplet");
        assert_eq!(mesh.count_vertices(), 3);
    }

    fn data() -> GoreData {
        let root = crate::paths::find_root().expect("repo root");
        GoreData::load(&crate::paths::data_dir(&root))
    }

    fn event() -> FallEvent {
        FallEvent {
            pos: Vec2::new(10.0, 20.0),
            ground_y: 1.0,
            mounted: false,
            away: Vec2::X,
        }
    }

    #[test]
    fn levels_come_from_the_data_file() {
        let d = data();
        assert_eq!(d.level(0).sprays, 0.0);
        assert_eq!(d.level(1).stains, 0.55);
        assert_eq!(d.level(1).sprays, 0.45);
        assert!(!d.level(1).dismember);
        assert!(d.level(2).dismember);
        assert_eq!(d.level(7), d.level(2));
        assert_eq!(d.sprays.max_droplets, 6144);
        assert_eq!(d.severed.max_pieces, 256);
    }

    #[test]
    fn critical_chance_follows_weapon_and_cause() {
        let d = data();
        assert_eq!(d.critical_chance("ball", None, false), 0.6);
        assert_eq!(
            d.critical_chance("melee", Some("unit_longbowmen"), false),
            0.14
        );
        assert_eq!(
            d.critical_chance("melee", Some("unit_unknown"), false),
            0.03
        );
        // Arrows never cut (`critical` 0); a heavy charge raises the chance.
        assert_eq!(
            d.critical_chance("arrow", Some("unit_longbowmen"), false),
            0.0
        );
        assert_eq!(d.critical_chance("charge", Some("unit_knights"), true), 0.1);
        assert_eq!(
            d.critical_chance("charge", Some("unit_hobelars"), true),
            0.08
        );
    }

    #[test]
    fn part_pick_covers_the_weights() {
        let d = data();
        assert!(
            ["head", "arm_r", "arm_l", "leg_r", "leg_l"]
                .contains(&d.pick_part(false, 0.99).unwrap())
        );
        assert!(["head", "arm_r", "arm_l"].contains(&d.pick_part(true, 0.5).unwrap()));
        assert_eq!(piece_kind("leg_l"), Some(2));
        assert_eq!(piece_kind("tail"), None);
    }

    #[test]
    fn off_and_moderate_levels_do_not_cut() {
        let mut state = GoreState::new(data());
        let flat = |_: Vec2| 1.0;
        for blood in [0, 1] {
            let level = state.data.level(blood);
            for _ in 0..200 {
                assert!(state.on_fall(level, &event(), 1.0, &flat).is_none());
            }
        }
        // Moderate: 7 * 0.45 = 3 droplets per death; off: none.
        assert_eq!(state.drops_emitted, 200 * 3);
    }

    #[test]
    fn full_level_cuts_and_fills_the_ring() {
        let mut state = GoreState::new(data());
        let flat = |_: Vec2| 1.0;
        let level = state.data.level(2);
        let mut cuts = 0;
        for _ in 0..4000 {
            if state.on_fall(level, &event(), 0.5, &flat).is_some() {
                cuts += 1;
            }
        }
        assert!((1700..2300).contains(&cuts), "{cuts}");
        assert_eq!(state.drops.len(), 6144);
        assert!(state.by_part.contains_key("head"));
    }

    #[test]
    fn droplet_lands_on_the_ground() {
        let drop = Drop::new(
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(2.0, 1.0, 0.0),
            1.2,
            0.04,
            0.0,
            2.5,
        );
        assert!(drop.land > 0.3);
        let end = drop.position(drop.land + 10.0);
        assert!((end.y - 2.5).abs() < 1e-3, "{end:?}");
        assert!((end.x - 2.0 * drop.land).abs() < 1e-3);
    }
}
