//! Ground-trace decals (lot decals, Bevy 0.20 `pbr_clustered_decals`): blood where men fell and
//! churned mud where a charge ran and closed into melee, projected onto the battle ground by
//! [`ClusteredDecal`] entities. Nothing here is a game rule: the traces follow what the
//! simulation already reports (`Losses::falls`, `UnitState::Charging` then `Melee`).
//!
//! Cost control. The module is OFF by default (`enabled` of `data/fx/battle_decals.json`) and the
//! decals only show when the Cargo feature `decals` (which turns on `bevy/pbr_clustered_decals`
//! and so changes the view bind groups of every PBR material) is compiled in. The pool is
//! bounded (`max_decals`, at most 1024 which is the Bevy view limit): entities are spawned
//! lazily up to that count then recycled oldest-first, and a decal fades out and hides at the end
//! of its `lifetime_s`. No allocation per frame: the request buffer keeps its capacity.
//!
//! The ground shader (`decor/ground.wesl`) calls `apply_decals`; units and props are not decaled.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::light::ClusteredDecal;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::Deserialize;
use sim_battle::UnitState;

use super::hash01;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::decor::flatten::{Losses, collect_losses};
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;

/// Hard cap of the Bevy view (`max_view_decals` of bevy_pbr).
pub const VIEW_DECAL_LIMIT: usize = 1024;
const IMAGE_SIZE: u32 = 64;
/// Decals farther than this from the camera are not laid.
const LAY_DISTANCE: f32 = 450.0;
/// Age updates (fade, expiry) run at this period of simulated time.
const AGE_PERIOD_S: f64 = 0.5;

#[derive(Debug, Clone, Deserialize)]
pub struct BloodSettings {
    pub enabled: bool,
    pub size_m: [f32; 2],
    pub min_interval_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MudSettings {
    pub enabled: bool,
    pub size_m: [f32; 2],
    pub interval_s: f32,
    pub contact_scale: f32,
}

/// `data/fx/battle_decals.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
pub struct DecalSettings {
    pub enabled: bool,
    pub max_decals: usize,
    pub lifetime_s: f32,
    pub fade_s: f32,
    pub projection_depth_m: f32,
    pub max_per_frame: usize,
    pub blood: BloodSettings,
    pub mud: MudSettings,
}

impl Default for DecalSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            max_decals: 256,
            lifetime_s: 120.0,
            fade_s: 20.0,
            projection_depth_m: 3.0,
            max_per_frame: 12,
            blood: BloodSettings {
                enabled: true,
                size_m: [0.9, 1.7],
                min_interval_s: 0.1,
            },
            mud: MudSettings {
                enabled: true,
                size_m: [1.4, 2.6],
                interval_s: 0.7,
                contact_scale: 2.2,
            },
        }
    }
}

impl DecalSettings {
    pub fn load(data_dir: &std::path::Path) -> Self {
        std::fs::read_to_string(data_dir.join("fx/battle_decals.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn capacity(&self) -> usize {
        self.max_decals.clamp(1, VIEW_DECAL_LIMIT)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecalKind {
    Blood,
    Mud,
}

/// A decal to lay (ground position, footprint, yaw).
#[derive(Clone, Copy, Debug)]
struct Request {
    pos: Vec2,
    size: Vec2,
    yaw: f32,
    kind: DecalKind,
}

/// Bounded ring of decal slots: lifetime and recycling logic, independent of Bevy.
#[derive(Debug, Default)]
pub struct DecalPool {
    capacity: usize,
    next: usize,
    /// Birth time (simulated seconds) of the live decal of each slot.
    born: Vec<Option<f64>>,
}

impl DecalPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            next: 0,
            born: Vec::new(),
        }
    }

    /// Slot taking the next decal: a fresh one until `capacity`, then the oldest (the ring order
    /// is the age order because every decal lives the same time).
    pub fn acquire(&mut self, now: f64) -> usize {
        let slot = self.next % self.capacity;
        self.next += 1;
        if slot >= self.born.len() {
            self.born.push(Some(now));
        } else {
            self.born[slot] = Some(now);
        }
        slot
    }

    /// Slots in use (spawned entities), live or not.
    pub fn slots(&self) -> usize {
        self.born.len()
    }

    /// Decals alive at `now`.
    pub fn live(&self, now: f64, lifetime: f32) -> usize {
        self.born
            .iter()
            .flatten()
            .filter(|&&born| now - born < f64::from(lifetime))
            .count()
    }

    /// Age of the decal of `slot`, if one is laid there.
    pub fn age(&self, slot: usize, now: f64) -> Option<f32> {
        self.born
            .get(slot)
            .copied()
            .flatten()
            .map(|born| (now - born).max(0.0) as f32)
    }

    /// Forgets the decal of `slot` (expired).
    pub fn release(&mut self, slot: usize) {
        if let Some(entry) = self.born.get_mut(slot) {
            *entry = None;
        }
    }

    pub fn clear(&mut self) {
        self.next = 0;
        self.born.clear();
    }
}

/// Size factor (1 -> 0) of a decal of `age` seconds: full until `lifetime - fade`, then
/// shrinking to nothing at `lifetime`.
pub fn life_scale(age: f32, lifetime: f32, fade: f32) -> f32 {
    if age >= lifetime {
        return 0.0;
    }
    let fade = fade.clamp(0.0, lifetime);
    if fade <= 0.0 || age <= lifetime - fade {
        1.0
    } else {
        (lifetime - age) / fade
    }
}

#[derive(Component)]
struct GroundDecal {
    slot: usize,
}

/// Runtime state: pool, entities, pending requests and counters of the dump.
#[derive(Resource, Default)]
pub struct DecalState {
    pool: DecalPool,
    entities: Vec<Entity>,
    base_scale: Vec<Vec3>,
    pending: Vec<Request>,
    blood_next: f64,
    mud_next: HashMap<u32, f64>,
    previous: HashMap<u32, UnitState>,
    age_next: f64,
    blood_image: Handle<Image>,
    mud_image: Handle<Image>,
    pub laid: u64,
    pub recycled: u64,
    pub dropped: u64,
    pub live: usize,
}

fn lerp(range: [f32; 2], t: f32) -> f32 {
    range[0] + (range[1] - range[0]) * t
}

/// Soft irregular blob, `rgb` in sRGB, alpha falling off to the rim.
fn blob_image(rgb: [f32; 3], seed: u64, streaks: bool) -> Image {
    let size = IMAGE_SIZE as usize;
    let mut data = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let radius = (u * u + v * v).sqrt();
            let angle = v.atan2(u);
            // Lobed rim.
            let lobes = 0.78
                + 0.14 * (angle * 3.0 + hash01(seed) * TAU).sin()
                + 0.08 * (angle * 7.0 + hash01(seed + 1) * TAU).sin();
            let edge = (1.0 - radius / lobes).clamp(0.0, 1.0);
            let mut alpha = (edge * 2.2).clamp(0.0, 1.0);
            let mut tone = 1.0;
            if streaks {
                // Churned mud: broken by furrows across the blob.
                let furrow = (u * 9.0 + (v * 5.0).sin() * 1.4).sin() * 0.5 + 0.5;
                alpha *= 0.65 + 0.35 * furrow;
                tone = 0.8 + 0.3 * furrow;
            } else {
                let speck = hash01(seed + 100 + (y * size + x) as u64);
                alpha *= 0.82 + 0.18 * speck;
            }
            for channel in rgb {
                data.push((channel * tone * 255.0).clamp(0.0, 255.0) as u8);
            }
            data.push((alpha * 0.9 * 255.0) as u8);
        }
    }
    Image::new(
        Extent3d {
            width: IMAGE_SIZE,
            height: IMAGE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn setup_images(
    settings: Res<DecalSettings>,
    mut state: ResMut<DecalState>,
    mut images: ResMut<Assets<Image>>,
) {
    if !settings.enabled {
        return;
    }
    state.pool = DecalPool::new(settings.capacity());
    state.blood_image = images.add(blob_image([0.30, 0.03, 0.03], 11, false));
    state.mud_image = images.add(blob_image([0.20, 0.14, 0.08], 29, true));
}

/// Turns what the simulation reported into requests (no spawn here).
fn gather(
    mut session: ResMut<BattleSession>,
    settings: Res<DecalSettings>,
    losses: Res<Losses>,
    mut state: ResMut<DecalState>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
) {
    if !settings.enabled || session.paused {
        return;
    }
    let camera_position = camera
        .iter()
        .next()
        .map_or(Vec3::new(0.0, 100.0, 0.0), |t| t.translation());
    let near = |p: Vec2| camera_position.distance(Vec3::new(p.x, camera_position.y, p.y)) < LAY_DISTANCE;
    let sim = session.sim_mut();
    let now = sim.elapsed();
    let state = &mut *state;
    let room = settings.max_per_frame;
    if settings.blood.enabled && now >= state.blood_next {
        let mut laid_one = false;
        for fall in &losses.falls {
            if state.pending.len() >= room {
                state.dropped += 1;
                continue;
            }
            if !near(fall.pos) {
                continue;
            }
            let seed = state.laid + state.pending.len() as u64 + 1;
            let scale = if fall.cavalry { 1.3 } else { 1.0 };
            let side = lerp(settings.blood.size_m, hash01(seed)) * scale;
            state.pending.push(Request {
                pos: fall.pos,
                size: Vec2::new(side, side * (0.7 + 0.3 * hash01(seed + 7))),
                yaw: hash01(seed + 13) * TAU,
                kind: DecalKind::Blood,
            });
            laid_one = true;
        }
        if laid_one {
            state.blood_next = now + f64::from(settings.blood.min_interval_s);
        }
    }
    if settings.mud.enabled {
        let mut seen: Vec<(u32, UnitState)> = Vec::new();
        for unit in sim.units() {
            if !unit.present() {
                continue;
            }
            seen.push((unit.id, unit.state));
            let pos = Vec2::new(unit.x as f32, unit.z as f32);
            if !near(pos) {
                continue;
            }
            let (width, depth) = unit.extent();
            let facing = unit.facing as f32;
            let forward = Vec2::new(facing.sin(), facing.cos());
            let right = Vec2::new(facing.cos(), -facing.sin());
            let was_charging = state.previous.get(&unit.id) == Some(&UnitState::Charging);
            let seed = state.laid + state.pending.len() as u64 + u64::from(unit.id) * 31 + 1;
            if state.pending.len() >= room {
                continue;
            }
            if was_charging && unit.state == UnitState::Melee {
                // The charge closed: one wide churned patch along the contact line.
                let side = lerp(settings.mud.size_m, hash01(seed)) * settings.mud.contact_scale;
                state.pending.push(Request {
                    pos: pos + forward * depth as f32 * 0.5,
                    size: Vec2::new((width as f32 * 0.6).max(side), side * 0.7),
                    yaw: facing.sin().atan2(facing.cos()) - FRAC_PI_2,
                    kind: DecalKind::Mud,
                });
            } else if unit.state == UnitState::Charging
                && now >= state.mud_next.get(&unit.id).copied().unwrap_or(0.0)
            {
                state
                    .mud_next
                    .insert(unit.id, now + f64::from(settings.mud.interval_s));
                let across = (hash01(seed) - 0.5) * width as f32 * 0.8;
                let side = lerp(settings.mud.size_m, hash01(seed + 3));
                state.pending.push(Request {
                    pos: pos + right * across - forward * depth as f32 * 0.3,
                    size: Vec2::new(side, side * 0.6),
                    yaw: facing.sin().atan2(facing.cos()) - FRAC_PI_2 + (hash01(seed + 5) - 0.5) * 0.5,
                    kind: DecalKind::Mud,
                });
            }
        }
        state.previous.clear();
        state.previous.extend(seen);
    }
}

/// Lays the pending requests on pool entities (spawned lazily, then recycled).
fn place(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    settings: Res<DecalSettings>,
    mut state: ResMut<DecalState>,
    mut decals: Query<(&mut Transform, &mut ClusteredDecal, &mut Visibility)>,
) {
    if !settings.enabled || state.pending.is_empty() {
        return;
    }
    let sim = session.sim_mut();
    let now = sim.elapsed();
    let field = sim.field();
    let state = &mut *state;
    let capacity = settings.capacity();
    let mut index = 0;
    while index < state.pending.len() {
        let request = state.pending[index];
        index += 1;
        let wrapped = state.pool.slots() >= capacity;
        let slot = state.pool.acquire(now);
        let ground = field.height(f64::from(request.pos.x), f64::from(request.pos.y)) as f32;
        // The decal looks down its local -Z: turn it to -Y, then yaw around the vertical.
        let rotation = Quat::from_rotation_y(request.yaw) * Quat::from_rotation_x(-FRAC_PI_2);
        let scale = Vec3::new(request.size.x, request.size.y, settings.projection_depth_m);
        let transform = Transform {
            translation: Vec3::new(request.pos.x, ground, request.pos.y),
            rotation,
            scale,
        };
        let image = match request.kind {
            DecalKind::Blood => state.blood_image.clone(),
            DecalKind::Mud => state.mud_image.clone(),
        };
        if wrapped {
            state.recycled += 1;
        }
        state.laid += 1;
        if slot >= state.entities.len() {
            let entity = commands
                .spawn((
                    Name::new("ground_decal"),
                    GroundDecal { slot },
                    ClusteredDecal {
                        base_color_texture: Some(image),
                        ..default()
                    },
                    transform,
                ))
                .id();
            state.entities.push(entity);
            state.base_scale.push(scale);
        } else {
            state.base_scale[slot] = scale;
            if let Ok((mut t, mut decal, mut visibility)) = decals.get_mut(state.entities[slot]) {
                *t = transform;
                decal.base_color_texture = Some(image);
                *visibility = Visibility::Inherited;
            }
        }
    }
    state.pending.clear();
}

/// Shrinks the old decals and hides the expired ones, twice a second of simulated time.
fn age(
    mut session: ResMut<BattleSession>,
    settings: Res<DecalSettings>,
    mut state: ResMut<DecalState>,
    mut decals: Query<(&GroundDecal, &mut Transform, &mut Visibility)>,
) {
    if !settings.enabled {
        return;
    }
    let now = session.sim_mut().elapsed();
    if now < state.age_next {
        return;
    }
    state.age_next = now + AGE_PERIOD_S;
    let state = &mut *state;
    for (decal, mut transform, mut visibility) in &mut decals {
        let Some(age) = state.pool.age(decal.slot, now) else {
            continue;
        };
        let factor = life_scale(age, settings.lifetime_s, settings.fade_s);
        if factor <= 0.0 {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
            state.pool.release(decal.slot);
            continue;
        }
        let base = state.base_scale[decal.slot];
        transform.scale = Vec3::new(base.x * factor, base.y * factor, base.z);
    }
    state.live = state.pool.live(now, settings.lifetime_s);
}

fn clear(
    mut commands: Commands,
    mut state: ResMut<DecalState>,
    decals: Query<Entity, With<GroundDecal>>,
) {
    for entity in &decals {
        commands.entity(entity).despawn();
    }
    *state = DecalState::default();
}

fn dump_decals(world: &World) -> (String, serde_json::Value) {
    let (Some(settings), Some(state)) = (
        world.get_resource::<DecalSettings>(),
        world.get_resource::<DecalState>(),
    ) else {
        return ("decals".into(), serde_json::Value::Null);
    };
    (
        "decals".into(),
        serde_json::json!({
            "enabled": settings.enabled,
            "feature_compiled": cfg!(feature = "decals"),
            "count": state.live,
            "slots": state.pool.slots(),
            "capacity": settings.capacity(),
            "laid": state.laid, "recycled": state.recycled, "dropped": state.dropped,
            "lifetime_s": settings.lifetime_s,
        }),
    )
}

pub struct DecalsPlugin;

impl Plugin for DecalsPlugin {
    fn build(&self, app: &mut App) {
        let settings = crate::paths::find_root()
            .map(|root| DecalSettings::load(&crate::paths::data_dir(&root)))
            .unwrap_or_default();
        app.insert_resource(settings)
            .init_resource::<DecalState>()
            .register_dump(dump_decals)
            .add_systems(OnEnter(AppState::Battle), setup_images)
            .add_systems(
                Update,
                (gather, place, age)
                    .chain()
                    .after(collect_losses)
                    .after(crate::battle::scene::StepSet)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .run_if(|settings: Res<DecalSettings>| settings.enabled),
            )
            .add_systems(OnExit(AppState::Battle), clear);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_grows_then_recycles_oldest() {
        let mut pool = DecalPool::new(3);
        let slots: Vec<usize> = (0..5).map(|n| pool.acquire(f64::from(n))).collect();
        assert_eq!(slots, vec![0, 1, 2, 0, 1]);
        assert_eq!(pool.slots(), 3);
        assert_eq!(pool.age(0, 4.0), Some(1.0));
        assert_eq!(pool.age(2, 4.0), Some(2.0));
    }

    #[test]
    fn decals_expire_after_their_lifetime() {
        let mut pool = DecalPool::new(4);
        pool.acquire(0.0);
        pool.acquire(10.0);
        assert_eq!(pool.live(5.0, 8.0), 2);
        assert_eq!(pool.live(9.0, 8.0), 1);
        pool.release(1);
        assert_eq!(pool.age(1, 12.0), None);
        pool.clear();
        assert_eq!(pool.slots(), 0);
    }

    #[test]
    fn fade_shrinks_to_nothing_at_the_end_of_life() {
        assert_eq!(life_scale(0.0, 100.0, 20.0), 1.0);
        assert_eq!(life_scale(80.0, 100.0, 20.0), 1.0);
        assert!((life_scale(90.0, 100.0, 20.0) - 0.5).abs() < 1e-6);
        assert_eq!(life_scale(100.0, 100.0, 20.0), 0.0);
        assert_eq!(life_scale(50.0, 100.0, 0.0), 1.0);
    }

    #[test]
    fn capacity_is_clamped_to_the_view_limit() {
        let mut settings = DecalSettings {
            max_decals: 5000,
            ..DecalSettings::default()
        };
        assert_eq!(settings.capacity(), VIEW_DECAL_LIMIT);
        settings.max_decals = 0;
        assert_eq!(settings.capacity(), 1);
    }

    #[test]
    fn data_file_parses_and_is_off() {
        let root = crate::paths::find_root().expect("repo root");
        let text = std::fs::read_to_string(crate::paths::data_dir(&root).join("fx/battle_decals.json"))
            .expect("battle_decals.json");
        let settings: DecalSettings = serde_json::from_str(&text).expect("valid settings");
        assert!(!settings.enabled, "decals stay off until measured");
    }

    #[test]
    fn blob_image_has_a_soft_edge() {
        let image = blob_image([0.3, 0.0, 0.0], 1, false);
        let data = image.data.as_ref().expect("data");
        let alpha = |x: usize, y: usize| data[(y * IMAGE_SIZE as usize + x) * 4 + 3];
        assert!(alpha(32, 32) > 150);
        assert_eq!(alpha(0, 0), 0);
    }
}
