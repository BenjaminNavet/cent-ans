//! Splashes and wakes of the regiments in the water (port of `_wet_span`, the `Splash`, `Wake`
//! and `Ford` pools of battle_effects.gd).
//!
//! A regiment is wet as soon as a part of its footprint stands in water (five samples along its
//! axis of march, ends widened by half a step; a bridge deck is dry land for the core). Moving
//! wet regiments throw white spray at the water surface over the wet part; horses leave a foam
//! wake at the rear edge of it, and a charge entering the water (once) throws a big burst. Water
//! comes from `Battlefield::water_at`; the spray is the player's smoke flipbook, tinted
//! `SPLASH_COLOR`. Godot reassigns 6 splash and 6 wake emitters to the best scored regiments;
//! here the same counts pick the regiments.

use std::collections::HashMap;

use bevy::prelude::*;
use sim_battle::UnitState;

use super::hash01;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::fx::flipbook::{FlipbookLooks, FlipbookMaterial, SmokeSprite, spawn_smoke_sprite};

/// Regiments farther than this from the camera throw nothing (`EFFECT_DISTANCE`).
const EFFECT_DISTANCE: f32 = 700.0;
const SPLASH_EMITTERS: usize = 6;
const WAKE_EMITTERS: usize = 6;
/// `SPLASH_COLOR`, the opacity of a spray puff, of a wake puff and of a ford burst.
const SPLASH_COLOR: [f32; 3] = [0.93, 0.96, 0.98];
const SPLASH_ALPHA: f32 = 0.5;
const WAKE_ALPHA: f32 = 0.38;
const BURST_ALPHA: f32 = 0.55;
/// Puffs per second at full strength (160 particles over 0.8 s in Godot; fewer, bigger sprites
/// here since each is a flipbook quad) and of a wake (90 over 1.6 s).
const SPLASH_RATE: f32 = 16.0;
const WAKE_RATE: f32 = 6.0;
const SPLASH_LIFE_S: f32 = 0.8;
const WAKE_LIFE_S: f32 = 1.6;
const BURST_PUFFS: u64 = 10;
/// Shared sprite budget: the fires keep their share.
const SPRITE_ROOM: usize = 700;
/// Height above the bed at which the spray is born (the bed is ~0.5 m under the water).
const SURFACE_LIFT_M: f32 = 0.45;
const WAKE_LIFT_M: f32 = 0.35;

/// The part of a regiment's axis (metres from its centre, `x` to `y`) that stands in water, or
/// `None` (`_wet_span`). `wet_at(offset)` tells whether the point at that offset along the facing
/// is in water.
pub fn wet_span(depth: f32, wet_at: impl Fn(f32) -> bool) -> Option<(f32, f32)> {
    let half = depth * 0.5 + 1.0;
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for i in 0..5 {
        let t = -half + 2.0 * half * i as f32 / 4.0;
        if wet_at(t) {
            lo = lo.min(t);
            hi = hi.max(t);
        }
    }
    if hi < lo {
        return None;
    }
    let step = half * 0.5;
    Some(((lo - step * 0.5).max(-half), (hi + step * 0.5).min(half)))
}

struct Wet {
    id: u32,
    centre: Vec3,
    facing: f32,
    width: f32,
    span: (f32, f32),
    strength: f32,
    score: f32,
    mounted: bool,
    fast: bool,
    charging: bool,
}

#[derive(Resource, Default)]
pub struct Ford {
    was_wet: HashMap<u32, bool>,
    carry: HashMap<u64, f32>,
    counter: u64,
    pub splashing: usize,
    pub wakes: usize,
    pub puffs: usize,
    pub bursts: usize,
    pub wet_regiments: usize,
    /// Most wet regiments at once, and moving regiments within range (for the dump).
    pub peak_wet: usize,
    pub moving_near: usize,
    pub widest_span_m: f32,
}

/// Dev hook (proofs): `CENT_ANS_FORD_DEMO=1` sends every regiment to the middle of the river, once,
/// a few seconds into the battle, so that the splashes can be dumped without playing.
fn ford_demo(session: &mut BattleSession, issued: &mut bool) {
    if *issued || std::env::var_os("CENT_ANS_FORD_DEMO").is_none() {
        return;
    }
    let sim = session.sim_mut();
    let Some(river) = sim.field().river.clone() else {
        return;
    };
    let units: Vec<(u32, f64)> = sim
        .units()
        .iter()
        .filter(|u| u.present())
        .map(|u| (u.id, u.x))
        .collect();
    *issued = true;
    for (id, x) in units {
        let _ = sim.issue_command(sim_battle::Command::Move {
            units: vec![id],
            x,
            z: river.center_z(x),
            run: true,
            facing: None,
            queue: false,
            width: None,
            match_speed: false,
            group_tag: None,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn update_ford(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    time: Res<Time>,
    mut ford: ResMut<Ford>,
    looks: Option<ResMut<FlipbookLooks>>,
    mut materials: ResMut<Assets<FlipbookMaterial>>,
    camera: Query<&GlobalTransform, With<BattleCamera>>,
    mut demo_issued: Local<bool>,
) {
    ford_demo(&mut session, &mut demo_issued);
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
    let sim = session.sim_mut();
    let field = sim.field().clone();
    let mut wet: Vec<Wet> = Vec::new();
    let mut seen: HashMap<u32, bool> = HashMap::new();
    let mut moving_near = 0;
    for unit in sim.units() {
        seen.insert(unit.id, false);
        if !unit.present()
            || !matches!(
                unit.state,
                UnitState::Marching | UnitState::Charging | UnitState::Routing
            )
        {
            continue;
        }
        let ground = field.height(unit.x, unit.z) as f32;
        let centre = Vec3::new(unit.x as f32, ground, unit.z as f32);
        let distance = camera_position.distance(centre);
        if distance >= EFFECT_DISTANCE {
            continue;
        }
        moving_near += 1;
        let facing = unit.facing as f32;
        let forward = Vec3::new(facing.sin(), 0.0, facing.cos());
        let (width, depth) = unit.extent();
        let Some(span) = wet_span(depth as f32, |t| {
            let p = centre + forward * t;
            field.water_at(f64::from(p.x), f64::from(p.z)).is_some()
        }) else {
            continue;
        };
        let mounted = unit.mounted;
        let fast = unit.state != UnitState::Marching || unit.running;
        let base: f32 = (if fast { 1.0 } else { 0.45 }) * if mounted { 1.0 } else { 0.55 };
        let strength = base.max(if mounted { 0.6 } else { 0.4 });
        seen.insert(unit.id, true);
        wet.push(Wet {
            id: unit.id,
            centre,
            facing,
            width: width as f32,
            span,
            strength,
            score: strength / (1.0 + distance / 120.0),
            mounted,
            fast,
            charging: unit.state == UnitState::Charging,
        });
    }
    let entered: Vec<u32> = wet
        .iter()
        .filter(|w| w.mounted && w.fast && !ford.was_wet.get(&w.id).copied().unwrap_or(false))
        .map(|w| w.id)
        .collect();
    ford.was_wet = seen;
    wet.sort_by(|a, b| b.score.total_cmp(&a.score));
    ford.wet_regiments = wet.len();
    ford.peak_wet = ford.peak_wet.max(wet.len());
    ford.moving_near = moving_near;
    ford.splashing = wet.len().min(SPLASH_EMITTERS);
    ford.widest_span_m = wet
        .iter()
        .map(|w| w.span.1 - w.span.0)
        .fold(ford.widest_span_m, f32::max);
    let wakes: Vec<&Wet> = wet
        .iter()
        .filter(|w| w.mounted)
        .take(WAKE_EMITTERS)
        .collect();
    ford.wakes = wakes.len();
    let tint = |alpha: f32| Some([SPLASH_COLOR[0], SPLASH_COLOR[1], SPLASH_COLOR[2], alpha]);
    let surface = |p: Vec3, lift: f32| -> Vec3 { Vec3::new(p.x, p.y + lift, p.z) };
    let mut spawn = |ford: &mut Ford,
                     commands: &mut Commands,
                     looks: &mut FlipbookLooks,
                     sprite: SmokeSprite| {
        if looks.live() >= SPRITE_ROOM {
            return;
        }
        if let Some(entity) = spawn_smoke_sprite(commands, looks, &mut materials, &sprite) {
            commands
                .entity(entity)
                .insert(DespawnOnExit(AppState::Battle));
            ford.puffs += 1;
        }
    };
    // Bursts of the charges that just entered the water.
    for w in wet.iter().filter(|w| entered.contains(&w.id)) {
        ford.bursts += 1;
        let forward = Vec3::new(w.facing.sin(), 0.0, w.facing.cos());
        let right = Vec3::new(w.facing.cos(), 0.0, -w.facing.sin());
        let at = surface(
            w.centre + forward * (w.span.0 + w.span.1) * 0.5,
            SURFACE_LIFT_M,
        );
        let scale: f32 = if w.charging { 2.2 } else { 1.4 };
        for k in 0..BURST_PUFFS {
            ford.counter += 1;
            let h = |j: u64| hash01(ford.counter.wrapping_mul(8).wrapping_add(j) + k * 613);
            // The ford burst: box 7 x 5 m, 40 degrees, 3.5-7.5 m/s, scales 1.2-2.8, gravity.
            let offset = right * (h(0) - 0.5) * 7.0 + forward * (h(1) - 0.5) * 5.0;
            let cone = 40f32.to_radians() * h(2);
            let around = h(3) * std::f32::consts::TAU;
            let heading = Vec3::new(
                cone.sin() * around.cos(),
                cone.cos(),
                cone.sin() * around.sin(),
            );
            let sprite = SmokeSprite {
                position: at + offset,
                velocity: heading * (3.5 + 4.0 * h(4)) * scale.sqrt(),
                accel: Vec3::NEG_Y * 9.8,
                damping: 0.6 + h(5),
                life: 1.0,
                size: (1.2 + 1.6 * h(6)) * scale / 1.25 * 0.6,
                spin: h(7) * std::f32::consts::TAU,
                spin_rate: 0.2,
                tint: tint(BURST_ALPHA),
            };
            spawn(&mut ford, &mut commands, &mut looks, sprite);
        }
    }
    if dt <= 0.0 {
        return;
    }
    for (list, is_wake) in [
        (wet.iter().take(SPLASH_EMITTERS).collect::<Vec<_>>(), false),
        (wakes, true),
    ] {
        for w in list {
            let key = u64::from(w.id) | if is_wake { 1 << 32 } else { 0 };
            let owed = ford.carry.entry(key).or_default();
            *owed +=
                if is_wake { WAKE_RATE } else { SPLASH_RATE } * w.strength.clamp(0.05, 1.0) * dt;
            let mut due = 0;
            while *owed >= 1.0 {
                *owed -= 1.0;
                due += 1;
            }
            let forward = Vec3::new(w.facing.sin(), 0.0, w.facing.cos());
            let right = Vec3::new(w.facing.cos(), 0.0, -w.facing.sin());
            for _ in 0..due {
                ford.counter += 1;
                let h = |j: u64| hash01(ford.counter.wrapping_mul(8).wrapping_add(j));
                let sprite = if is_wake {
                    // Foam dragged backward from the rear edge of the wet part, spreading aside.
                    let half_width = (w.width * 0.5).max(3.0) + 1.0;
                    let at = w.centre
                        + forward * (w.span.0 + (h(0) - 0.5) * 3.5)
                        + right * (h(1) * 2.0 - 1.0) * half_width;
                    let spread = 45f32.to_radians() * h(2);
                    let around = h(3) * std::f32::consts::TAU;
                    let back = -forward * spread.cos()
                        + (right * around.cos() + Vec3::Y * 0.3) * spread.sin();
                    SmokeSprite {
                        position: surface(at, WAKE_LIFT_M),
                        velocity: back * (0.6 + 1.2 * h(4)),
                        accel: Vec3::NEG_Y * 0.4,
                        damping: 0.6 + 0.8 * h(5),
                        life: WAKE_LIFE_S,
                        size: (0.7 + 0.9 * h(6)) / 1.25 * 1.4,
                        spin: h(7) * std::f32::consts::TAU,
                        spin_rate: 0.1,
                        tint: tint(WAKE_ALPHA),
                    }
                } else {
                    // Spray over the wet part only: box of its length and the regiment's width.
                    let along = w.span.0 + (w.span.1 - w.span.0) * h(0);
                    let at = w.centre
                        + forward * along
                        + right * (h(1) * 2.0 - 1.0) * (w.width * 0.5 + 1.0);
                    let cone = 28f32.to_radians() * h(2);
                    let around = h(3) * std::f32::consts::TAU;
                    let up = Vec3::new(
                        cone.sin() * around.cos(),
                        cone.cos(),
                        cone.sin() * around.sin(),
                    );
                    SmokeSprite {
                        position: surface(at, SURFACE_LIFT_M),
                        velocity: up * (2.0 + 2.8 * h(4)),
                        accel: Vec3::NEG_Y * 9.8,
                        damping: 0.5 + h(5),
                        life: SPLASH_LIFE_S,
                        size: (0.9 + 1.3 * h(6)) / 1.25 * 0.8,
                        spin: h(7) * std::f32::consts::TAU,
                        spin_rate: 0.2,
                        tint: tint(SPLASH_ALPHA),
                    }
                };
                spawn(&mut ford, &mut commands, &mut looks, sprite);
            }
        }
    }
    let live: std::collections::HashSet<u32> = wet.iter().map(|w| w.id).collect();
    ford.carry
        .retain(|key, _| live.contains(&((*key & 0xFFFF_FFFF) as u32)));
}

fn clear_ford(mut ford: ResMut<Ford>) {
    *ford = Ford::default();
}

fn dump_ford(world: &World) -> (String, serde_json::Value) {
    let Some(f) = world.get_resource::<Ford>() else {
        return ("battle_ford_fx".into(), serde_json::Value::Null);
    };
    (
        "battle_ford_fx".into(),
        serde_json::json!({
            "wet_regiments": f.wet_regiments, "peak_wet": f.peak_wet, "moving_near": f.moving_near, "splashing": f.splashing, "wakes": f.wakes,
            "puffs": f.puffs, "entry_bursts": f.bursts, "widest_span_m": f.widest_span_m,
        }),
    )
}

pub struct FordPlugin;

impl Plugin for FordPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Ford>()
            .register_dump(dump_ford)
            .add_systems(
                Update,
                update_ford
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet),
            )
            .add_systems(OnExit(AppState::Battle), clear_ford);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_ground_has_no_wet_span() {
        assert_eq!(wet_span(10.0, |_| false), None);
    }

    #[test]
    fn a_river_across_the_front_wets_only_that_part() {
        // Depth 10 -> half 6; the water begins 3 m ahead of the centre.
        let (lo, hi) = wet_span(10.0, |t| t >= 3.0).unwrap();
        assert!(lo < 3.0 && lo > 0.0, "{lo}");
        assert_eq!(hi, 6.0, "clamped to the footprint");
    }

    #[test]
    fn a_regiment_wholly_in_the_water_is_wet_all_along() {
        let (lo, hi) = wet_span(10.0, |_| true).unwrap();
        assert_eq!((lo, hi), (-6.0, 6.0));
    }
}

#[cfg(test)]
mod field_tests {
    use super::*;
    use data_model::Terrain;
    use sim_battle::{BattleRng, Battlefield, Weather};

    #[test]
    fn the_core_water_query_wets_a_regiment_standing_in_the_river() {
        let mut rng = BattleRng::from_seed(7);
        let field = Battlefield::generate(Terrain::Plains, true, Weather::Clear, &mut rng);
        let river = field.river.clone().expect("a river");
        let x = field.width * 0.37;
        let z = river.center_z(x);
        let in_water = |dz: f32| field.water_at(x, z + f64::from(dz)).is_some();
        // A regiment 12 m deep facing +z, its centre in the middle of the river.
        let span = wet_span(12.0, in_water).expect("wet");
        assert!(span.1 > span.0, "{span:?}");
        // Far from the river bank: dry.
        assert_eq!(
            wet_span(12.0, |dz| field
                .water_at(x, z + 300.0 + f64::from(dz))
                .is_some()),
            None
        );
    }
}
