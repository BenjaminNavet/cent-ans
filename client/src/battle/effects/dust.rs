//! Dust of the regiments (port of `update`, `_assign`, `_place`, `configure_ground`,
//! `configure_staging` and `dust_factor` of battle_effects.gd).
//!
//! Marching, charging and routing regiments near the camera kick up a cloud over their whole
//! footprint, stronger for cavalry and for fast movement, scaled by the number of men, the terrain
//! and the season (`dust` of `data/fx/battle_staging.json`). Big regiments farther away raise a
//! dust column instead. A charge that closes into melee throws one heavy cloud at the contact.
//! No dust under rain or snow, nor on muddy or snowy ground.
//!
//! Godot reassigns a pool of `GPUParticles3D` to the best scored regiments each frame; here the
//! same pool sizes pick the regiments and each emits flipbook smoke sprites tinted earth at a
//! rate that keeps the shared sprite budget (`MAX_PARTICLES`) for the fires. The wind pushes the
//! clouds (Godot pushes along +x).

use std::collections::HashMap;

use bevy::prelude::*;
use sim_battle::UnitState;

use super::hash01;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::battle::units::wind::BattleWind;
use crate::fx::flipbook::{FlipbookLooks, FlipbookMaterial, SmokeSprite, spawn_smoke_sprite};

/// Emitters kept on the nearest regiments (`DUST_EMITTERS` is 10 in Godot; each sprite here is
/// a big opaque cloud, so fewer).
const DUST_EMITTERS: usize = 6;
/// Dust column emitters by quality (`column_emitters`, medium preset).
const COLUMN_EMITTERS: usize = 4;
/// Beyond this camera distance no regiment dusts (`EFFECT_DISTANCE`); below `DUST_DISTANCE` a
/// regiment raises a cloud over its footprint, between that and `column_distance_m` a column.
const DUST_DISTANCE: f32 = 420.0;
/// Puffs per second of a dust emitter at full strength (72 particles over 2.8 s in Godot).
const DUST_RATE: f32 = 9.0;
const DUST_LIFE_S: f32 = 2.8;
const COLUMN_RATE: f32 = 4.5;
const COLUMN_LIFE_S: f32 = 6.0;
/// The flipbook sprites stop being produced beyond this many alive: the fires keep their share.
const SPRITE_ROOM: usize = 700;
/// Dry-ground dust colour (`GROUND_DUST` `dry`, darkened by 0.9 as `configure_ground`).
const DUST_COLOR: [f32; 3] = [0.74 * 0.9, 0.66 * 0.9, 0.52 * 0.9];
/// Heavier cloud of a charge's contact (`DUST_COLOR.darkened(0.1)`).
const IMPACT_COLOR: [f32; 3] = [0.52 * 0.9, 0.45 * 0.9, 0.34 * 0.9];

/// `dust` of `data/fx/battle_staging.json`.
#[derive(Clone, Debug)]
pub struct DustConfig {
    pub reference_soldiers: f32,
    pub max_strength: f32,
    pub column_distance_m: f32,
    pub column_min_soldiers: u32,
    terrain: HashMap<String, f32>,
    season: HashMap<String, f32>,
}

impl Default for DustConfig {
    fn default() -> Self {
        Self {
            reference_soldiers: 240.0,
            max_strength: 1.6,
            column_distance_m: 0.0,
            column_min_soldiers: 400,
            terrain: HashMap::new(),
            season: HashMap::new(),
        }
    }
}

impl DustConfig {
    pub fn from_json(value: &serde_json::Value) -> Self {
        let mut config = Self::default();
        let num = |v: &serde_json::Value, d: f32| v.as_f64().map_or(d, |x| x as f32);
        config.reference_soldiers = num(&value["reference_soldiers"], config.reference_soldiers);
        config.max_strength = num(&value["max_strength"], config.max_strength);
        config.column_distance_m = num(&value["column_distance_m"], config.column_distance_m);
        config.column_min_soldiers = value["column_min_soldiers"]
            .as_u64()
            .map_or(config.column_min_soldiers, |n| n as u32);
        let table = |v: &serde_json::Value| -> HashMap<String, f32> {
            v.as_object()
                .map(|o| {
                    o.iter()
                        .filter_map(|(k, x)| Some((k.clone(), x.as_f64()? as f32)))
                        .collect()
                })
                .unwrap_or_default()
        };
        config.terrain = table(&value["terrain"]);
        config.season = table(&value["season"]);
        config
    }

    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| {
                std::fs::read_to_string(
                    crate::paths::data_dir(&root).join("fx/battle_staging.json"),
                )
                .ok()
            })
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .map_or_else(Self::default, |v| Self::from_json(&v["dust"]))
    }

    /// Terrain and season factor (`_dust_scale`); an empty key is plains / spring.
    pub fn scale(&self, terrain: &str, season: &str) -> f32 {
        let terrain = if terrain.is_empty() {
            "plains"
        } else {
            terrain
        };
        let season = if season.is_empty() { "spring" } else { season };
        self.terrain.get(terrain).copied().unwrap_or(1.0)
            * self.season.get(season).copied().unwrap_or(1.0)
    }

    /// Dust factor of a regiment of `soldiers` men (`dust_factor`).
    pub fn factor(&self, soldiers: u32, scale: f32) -> f32 {
        let men = (soldiers as f32 / self.reference_soldiers.max(1.0)).sqrt();
        men.clamp(0.7, self.max_strength) * scale
    }
}

/// Strength of a moving regiment before the men / terrain factor: fast and mounted raise more.
pub fn base_strength(state: UnitState, running: bool, mounted: bool) -> f32 {
    let fast = matches!(state, UnitState::Charging | UnitState::Routing) || running;
    (if fast { 1.0 } else { 0.45 }) * if mounted { 1.0 } else { 0.55 }
}

#[derive(Resource, Default)]
pub struct Dust {
    config: DustConfig,
    scale: f32,
    enabled: bool,
    /// Why the dust is off, or the ground it rises from.
    ground: String,
    previous: HashMap<u32, UnitState>,
    /// Fractional puffs owed per regiment (bit 32 set: column).
    carry: HashMap<u64, f32>,
    counter: u64,
    // Counters for the dump.
    pub puffs: usize,
    pub impacts: usize,
    pub active_dust: usize,
    pub active_columns: usize,
    pub strongest: f32,
}

fn init_dust(mut dust: ResMut<Dust>, mut session: ResMut<BattleSession>) {
    let config = DustConfig::load();
    let ground = session.sim_mut().field().ground.key().to_owned();
    let weather = session.config.weather.as_str();
    dust.scale = config.scale(&session.config.terrain, &session.config.season);
    dust.enabled = weather != "rain" && weather != "snow" && ground == "dry";
    dust.ground = ground;
    dust.config = config;
    dust.previous.clear();
    dust.carry.clear();
}

struct Candidate {
    id: u32,
    pos: Vec3,
    facing: f32,
    half: Vec2,
    strength: f32,
    score: f32,
}

#[allow(clippy::too_many_arguments)]
fn update_dust(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    time: Res<Time>,
    wind: Option<Res<BattleWind>>,
    mut dust: ResMut<Dust>,
    looks: Option<ResMut<FlipbookLooks>>,
    mut materials: ResMut<Assets<FlipbookMaterial>>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
) {
    let Some(mut looks) = looks else { return };
    let dt = if session.paused {
        0.0
    } else {
        time.delta_secs().min(0.1) * session.speed as f32
    };
    let camera_position = camera
        .iter()
        .next()
        .map_or(Vec3::new(0.0, 100.0, 0.0), |t| t.translation());
    let wind = wind.map_or(BattleWind::default(), |w| *w);
    let wind_push = Vec3::new(wind.dir.x, 0.0, wind.dir.y) * wind.strength.max(0.2) * 0.6;
    let sim = session.sim_mut();
    let field = sim.field().clone();
    let ground = |x: f32, z: f32| field.height(f64::from(x), f64::from(z)) as f32;
    let column_distance = dust.config.column_distance_m;
    let mut dusty: Vec<Candidate> = Vec::new();
    let mut columns: Vec<Candidate> = Vec::new();
    let mut impacts: Vec<(Vec3, f32, f32)> = Vec::new();
    let mut seen_state: Vec<(u32, UnitState)> = Vec::new();
    for unit in sim.units() {
        if !unit.present() {
            continue;
        }
        let pos = Vec3::new(
            unit.x as f32,
            ground(unit.x as f32, unit.z as f32),
            unit.z as f32,
        );
        let distance = camera_position.distance(pos);
        let facing = unit.facing as f32;
        let (width, depth) = unit.extent();
        if dust.previous.get(&unit.id) == Some(&UnitState::Charging)
            && unit.state == UnitState::Melee
            && distance < 700.0
        {
            let forward = Vec3::new(facing.sin(), 0.0, facing.cos());
            let at = pos + forward * depth as f32 * 0.5;
            impacts.push((at, if unit.mounted { 1.6 } else { 1.0 }, depth as f32));
        }
        seen_state.push((unit.id, unit.state));
        if !dust.enabled
            || !matches!(
                unit.state,
                UnitState::Marching | UnitState::Charging | UnitState::Routing
            )
        {
            continue;
        }
        let strength = base_strength(unit.state, unit.running, unit.mounted);
        let men = unit.soldiers();
        let factor = dust.config.factor(men, dust.scale);
        let candidate = Candidate {
            id: unit.id,
            pos,
            facing,
            half: Vec2::new(width as f32 * 0.5, depth as f32 * 0.5),
            strength: strength * factor,
            score: strength * factor / (1.0 + distance / 120.0),
        };
        if distance < DUST_DISTANCE {
            dusty.push(candidate);
        } else if distance < column_distance && men >= dust.config.column_min_soldiers {
            // A column is the whole troop's dust: strength is the men / terrain factor alone.
            columns.push(Candidate {
                strength: factor,
                score: men as f32 / (1.0 + distance / 300.0),
                ..candidate
            });
        }
    }
    dust.previous = seen_state.into_iter().collect();
    dusty.sort_by(|a, b| b.score.total_cmp(&a.score));
    columns.sort_by(|a, b| b.score.total_cmp(&a.score));
    dusty.truncate(DUST_EMITTERS);
    columns.truncate(COLUMN_EMITTERS);
    dust.active_dust = dusty.len();
    dust.active_columns = columns.len();
    dust.strongest = dusty
        .iter()
        .chain(&columns)
        .map(|c| c.strength)
        .fold(0.0, f32::max);
    if dt <= 0.0 {
        return;
    }
    let room = |looks: &FlipbookLooks| looks.live() < SPRITE_ROOM;
    let mut spawn = |dust: &mut Dust,
                     commands: &mut Commands,
                     looks: &mut FlipbookLooks,
                     sprite: SmokeSprite| {
        if let Some(entity) = spawn_smoke_sprite(commands, looks, &mut materials, &sprite) {
            commands
                .entity(entity)
                .insert(DespawnOnExit(AppState::Battle));
            dust.puffs += 1;
        }
    };
    for (list, is_column) in [(&dusty, false), (&columns, true)] {
        for candidate in list {
            let key = u64::from(candidate.id) | if is_column { 1 << 32 } else { 0 };
            let rate = if is_column { COLUMN_RATE } else { DUST_RATE }
                * candidate.strength.clamp(0.05, 1.0);
            let owed = dust.carry.entry(key).or_default();
            *owed += rate * dt;
            let mut due = 0;
            while *owed >= 1.0 {
                *owed -= 1.0;
                due += 1;
            }
            for _ in 0..due {
                if !room(&looks) {
                    break;
                }
                dust.counter += 1;
                let h = |k: u64| hash01(dust.counter.wrapping_mul(8).wrapping_add(k));
                // A point of the footprint (`emission_box_extents`: half size + 1 m), turned to the facing.
                let local = Vec2::new(
                    (h(0) * 2.0 - 1.0) * (candidate.half.x + 1.0),
                    (h(1) * 2.0 - 1.0) * (candidate.half.y + 1.0),
                );
                let right = Vec3::new(candidate.facing.cos(), 0.0, -candidate.facing.sin());
                let forward = Vec3::new(candidate.facing.sin(), 0.0, candidate.facing.cos());
                let mut position = candidate.pos + right * local.x + forward * local.y;
                position.y = ground(position.x, position.z) + 0.2;
                // A cone of 70 degrees around +Y (column: 50), 0.4-1.6 m/s (column: 0.8-2.2).
                let cone = if is_column { 50f32 } else { 70f32 }.to_radians() * h(2);
                let around = h(3) * std::f32::consts::TAU;
                let heading = Vec3::new(
                    cone.sin() * around.cos(),
                    cone.cos(),
                    cone.sin() * around.sin(),
                );
                let speed = if is_column {
                    0.8 + 1.4 * h(4)
                } else {
                    0.4 + 1.2 * h(4)
                };
                // Godot scales 0.35 -> 1.0 of 3-6 m (column 9-16 m, 0.4 -> 1.3); the sprite
                // grows 0.5 -> 1.25 of its size. Big regiments make bigger clouds, not more.
                let grow = candidate.strength.max(1.0).sqrt();
                let size = if is_column {
                    9.0 + 7.0 * h(5)
                } else {
                    3.0 + 3.0 * h(5)
                } / 1.25
                    * grow;
                let sprite = SmokeSprite {
                    position,
                    velocity: heading * speed,
                    accel: wind_push * 0.5 + Vec3::Y * 0.15,
                    damping: 0.3 + 0.5 * h(6),
                    life: if is_column {
                        COLUMN_LIFE_S
                    } else {
                        DUST_LIFE_S
                    },
                    size,
                    spin: h(7) * std::f32::consts::TAU,
                    spin_rate: (h(2) - 0.5) * 0.3,
                    tint: Some([
                        DUST_COLOR[0],
                        DUST_COLOR[1],
                        DUST_COLOR[2],
                        if is_column { 0.35 } else { 0.28 },
                    ]),
                };
                spawn(&mut dust, &mut commands, &mut looks, sprite);
            }
        }
    }
    for (at, scale, depth) in impacts {
        dust.impacts += 1;
        for k in 0..8u64 {
            if !room(&looks) {
                break;
            }
            dust.counter += 1;
            let h = |j: u64| hash01(dust.counter.wrapping_mul(8).wrapping_add(j) + k * 977);
            let around = h(0) * std::f32::consts::TAU;
            let out = Vec3::new(around.cos(), 0.0, around.sin());
            let position = at + out * h(1) * depth.max(3.0) * 0.5 + Vec3::Y * 0.4;
            let sprite = SmokeSprite {
                position,
                velocity: out * (1.0 + 2.0 * h(2)) * scale + Vec3::Y * (0.5 + h(3)),
                accel: wind_push * 0.5,
                damping: 0.6,
                life: 2.2,
                size: (3.0 + 2.5 * h(4)) * scale / 1.25,
                spin: h(5) * std::f32::consts::TAU,
                spin_rate: 0.1,
                tint: Some([IMPACT_COLOR[0], IMPACT_COLOR[1], IMPACT_COLOR[2], 0.4]),
            };
            spawn(&mut dust, &mut commands, &mut looks, sprite);
        }
    }
    let live_keys: std::collections::HashSet<u32> =
        dusty.iter().chain(&columns).map(|c| c.id).collect();
    dust.carry
        .retain(|key, _| live_keys.contains(&((*key & 0xFFFF_FFFF) as u32)));
}

fn clear_dust(mut dust: ResMut<Dust>) {
    *dust = Dust::default();
}

fn dump_dust(world: &World) -> (String, serde_json::Value) {
    let Some(d) = world.get_resource::<Dust>() else {
        return ("battle_dust".into(), serde_json::Value::Null);
    };
    (
        "battle_dust".into(),
        serde_json::json!({
            "enabled": d.enabled, "ground": d.ground, "scale": d.scale,
            "regiments_dusting": d.active_dust, "columns": d.active_columns,
            "puffs": d.puffs, "charge_impacts": d.impacts, "strongest": d.strongest,
            "reference_soldiers": d.config.reference_soldiers,
        }),
    )
}

pub struct DustPlugin;

impl Plugin for DustPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Dust>()
            .register_dump(dump_dust)
            .add_systems(
                Update,
                (
                    init_dust.run_if(resource_added::<BattleSession>),
                    update_dust
                        .run_if(in_state(AppState::Battle))
                        .run_if(resource_exists::<BattleSession>)
                        .after(crate::battle::scene::StepSet),
                )
                    .chain(),
            )
            .add_systems(OnExit(AppState::Battle), clear_dust);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> DustConfig {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/fx/battle_staging.json"
        ))
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        DustConfig::from_json(&value["dust"])
    }

    #[test]
    fn data_gives_the_staging_factors() {
        let c = config();
        assert_eq!(c.reference_soldiers, 120.0);
        assert!((c.scale("plains", "summer") - 1.2).abs() < 1e-5);
        assert!((c.scale("forest", "winter") - 0.45 * 0.3).abs() < 1e-5);
        assert!(
            (c.scale("", "") - 0.7).abs() < 1e-5,
            "plains / spring by default"
        );
        assert_eq!(c.column_min_soldiers, 400);
    }

    #[test]
    fn big_regiments_raise_more_up_to_the_cap() {
        let c = config();
        assert!((c.factor(120, 1.0) - 1.0).abs() < 1e-5);
        assert!((c.factor(10, 1.0) - 0.7).abs() < 1e-5, "floor");
        assert!((c.factor(5000, 1.0) - c.max_strength).abs() < 1e-5, "cap");
        assert!(c.factor(240, 1.0) > c.factor(120, 1.0));
    }

    #[test]
    fn cavalry_at_speed_raises_the_most() {
        let cavalry = base_strength(UnitState::Charging, false, true);
        let foot_running = base_strength(UnitState::Marching, true, false);
        let foot_walking = base_strength(UnitState::Marching, false, false);
        assert_eq!(cavalry, 1.0);
        assert!(cavalry > foot_running && foot_running > foot_walking);
        assert!((foot_walking - 0.45 * 0.55).abs() < 1e-6);
    }
}
