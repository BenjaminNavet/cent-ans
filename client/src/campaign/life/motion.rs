//! Procedural motion of the beasts and carts (`visual/animal_motion.gd`, `data/fx/animal_motion.json`).
//!
//! Godot displaces the vertices of the beasts in a shader (legs, neck, tail, wheels). The glb scenes
//! of this client have no bones nor a custom vertex stage, so the measured data drives what a
//! transform can show: the body bob of the draught beasts, the jolt and the roll of the carts
//! (measured lines, `jolt_lines`), the grazing pitch and the breathing of the herds. Legs, necks,
//! tails and wheels stay rigid (listed in `docs/wip/bm-life.md`).

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::Path;

use bevy::prelude::*;
use serde_json::Value;

use super::data::{LifeData, h01};
use super::folk::{Beast, Cart, WALK_SPEED_M_S, world_scale};
use super::stream::LifeView;

/// Merged parameters of one model (`defaults` < `base` < entry, `AnimalMotion.model_params`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MotionSpec {
    pub has_beast: bool,
    pub stride_m: f32,
    pub bob_m: f32,
    pub roll_rad: f32,
    pub jolt_period_m: f32,
    /// (amplitude m, stride ratio) of the measured jolt lines.
    pub jolt_lines: Vec<(f32, f32)>,
    pub graze_rad: f32,
    pub graze_share: f32,
    pub graze_cycle_s: f32,
    pub graze_ramp_s: f32,
    pub nod_rad: f32,
    pub breath_m: f32,
    pub breath_hz: f32,
    // Shader parameters (`AnimalMotion.apply_prop`): body, neck, legs, tail, wheels.
    pub anchor_m: f32,
    pub half_m: f32,
    pub belly_m: f32,
    pub withers_m: f32,
    pub neck_start_m: f32,
    pub neck_len_m: f32,
    pub neck_pivot_m: f32,
    pub swing_m: f32,
    pub lift_m: f32,
    pub tail_len_m: f32,
    pub tail_swing_m: f32,
    pub tail_hz: f32,
    pub tail_walk_gain: f32,
    pub shift_m: f32,
    pub chew_hz: f32,
    pub chew_rad: f32,
    pub wheel_radius_m: f32,
    pub wheel_x_min_m: f32,
    /// Z of the axles along the advance axis (one or two).
    pub axle_z_m: Vec<f32>,
    /// Measured step curves, 16 samples per cycle (`animal_motion_measured.json`, baked in
    /// `animal_motion.json`): forward swing (amplitude 1) and hoof lift (0 to 1).
    pub swing_lut: [f32; 16],
    pub lift_lut: [f32; 16],
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MotionTable {
    pub enabled: bool,
    pub models: HashMap<String, MotionSpec>,
}

/// 16-sample step table (zeros when absent).
fn lut(v: &Value) -> [f32; 16] {
    let mut out = [0.0; 16];
    if let Some(a) = v.as_array() {
        for (slot, x) in out.iter_mut().zip(a) {
            *slot = x.as_f64().unwrap_or(0.0) as f32;
        }
    }
    out
}

fn merged(models: &serde_json::Map<String, Value>, defaults: &Value, name: &str) -> Option<Value> {
    let entry = models.get(name)?;
    let mut out = defaults.as_object().cloned().unwrap_or_default();
    if let Some(base) = entry["base"].as_str()
        && let Some(base_entry) = models.get(base).and_then(Value::as_object)
    {
        out.extend(base_entry.clone());
    }
    if let Some(e) = entry.as_object() {
        out.extend(e.clone());
    }
    out.remove("base");
    Some(Value::Object(out))
}

impl MotionTable {
    pub fn from_json(doc: &Value) -> MotionTable {
        let campaign = &doc["campaign"];
        let defaults = &campaign["defaults"];
        let Some(models) = campaign["models"].as_object() else {
            return MotionTable::default();
        };
        let f = |v: &Value, k: &str, d: f32| v[k].as_f64().map_or(d, |x| x as f32);
        let mut out = HashMap::new();
        for name in models.keys() {
            let Some(p) = merged(models, defaults, name) else {
                continue;
            };
            let jolt_lines = p["jolt_lines"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|l| (f(l, "amp_m", 0.0), f(l, "stride_ratio", 1.0)))
                        .collect()
                })
                .unwrap_or_default();
            out.insert(
                name.clone(),
                MotionSpec {
                    has_beast: f(&p, "half_m", 0.0) > 0.0,
                    stride_m: f(&p, "stride_m", 0.86),
                    bob_m: f(&p, "bob_m", 0.0),
                    roll_rad: f(&p, "roll_rad", 0.0),
                    jolt_period_m: f(&p, "jolt_period_m", 1.0),
                    jolt_lines,
                    graze_rad: f(&p, "graze_rad", 0.0),
                    graze_share: f(&p, "graze_share", 0.0),
                    graze_cycle_s: f(&p, "graze_cycle_s", 12.0),
                    graze_ramp_s: f(&p, "graze_ramp_s", 1.0),
                    nod_rad: f(&p, "nod_rad", 0.0),
                    breath_m: f(&p, "breath_m", 0.0),
                    breath_hz: f(&p, "breath_hz", 0.0),
                    anchor_m: f(&p, "anchor_m", 0.0),
                    half_m: f(&p, "half_m", 0.0),
                    belly_m: f(&p, "belly_m", 0.5),
                    withers_m: f(&p, "withers_m", 1.0),
                    neck_start_m: f(&p, "neck_start_m", 0.5),
                    neck_len_m: f(&p, "neck_len_m", 0.3),
                    neck_pivot_m: f(&p, "neck_pivot_m", 1.0),
                    swing_m: f(&p, "swing_m", 0.0),
                    lift_m: f(&p, "lift_m", 0.0),
                    tail_len_m: f(&p, "tail_len_m", 0.08),
                    tail_swing_m: f(&p, "tail_swing_m", 0.0),
                    tail_hz: f(&p, "tail_hz", 0.5),
                    tail_walk_gain: f(&p, "tail_walk_gain", 1.0),
                    shift_m: f(&p, "shift_m", 0.0),
                    chew_hz: f(&p, "chew_hz", 0.0),
                    chew_rad: f(&p, "chew_rad", 0.0),
                    wheel_radius_m: f(&p, "wheel_radius_m", 0.0),
                    wheel_x_min_m: f(&p, "wheel_x_min_m", 0.65),
                    axle_z_m: p["axle_z_m"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_f64)
                                .map(|x| x as f32)
                                .collect()
                        })
                        .filter(|v: &Vec<f32>| !v.is_empty())
                        .unwrap_or_else(|| vec![0.0]),
                    swing_lut: lut(&p["swing_lut"]),
                    lift_lut: lut(&p["lift_lut"]),
                },
            );
        }
        MotionTable {
            enabled: doc["enabled"].as_bool().unwrap_or(false),
            models: out,
        }
    }

    /// Merges the `campaign_horse` block of `camp_horse_motion.json` (chewing of the horse, CC BY-SA
    /// file of its own) into the `horse` model (`AnimalMotion._merge_camp_horse_motion`).
    pub fn merge_camp_horse(doc: &mut Value, camp: &Value) {
        let Some(extra) = camp["campaign_horse"].as_object() else {
            return;
        };
        if let Some(horse) = doc["campaign"]["models"]["horse"].as_object_mut() {
            horse.extend(extra.clone());
        }
    }

    pub fn load(data_dir: &Path) -> MotionTable {
        let read = |name: &str| -> Result<Value, String> {
            let path = data_dir.join(name);
            std::fs::read_to_string(&path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .and_then(|t| {
                    serde_json::from_str::<Value>(&t)
                        .map_err(|e| format!("{}: {e}", path.display()))
                })
        };
        match read("fx/animal_motion.json") {
            Ok(mut v) => {
                if let Ok(camp) = read("fx/camp_horse_motion.json") {
                    MotionTable::merge_camp_horse(&mut v, &camp);
                }
                MotionTable::from_json(&v)
            }
            Err(e) => {
                warn!("life: {e}");
                MotionTable::default()
            }
        }
    }

    pub fn spec(&self, model: &str) -> Option<&MotionSpec> {
        self.enabled.then(|| self.models.get(model)).flatten()
    }
}

/// Vertical body bob (m) of a beast that has covered `travelled_m` (two bobs per stride: one per
/// diagonal pair of legs).
pub fn bob_m(spec: &MotionSpec, travelled_m: f32) -> f32 {
    if spec.stride_m <= 0.0 {
        return 0.0;
    }
    spec.bob_m * (travelled_m / spec.stride_m * TAU * 2.0).sin().abs()
}

/// Vertical jolt (m) of a cart that has covered `travelled_m`: the sum of the measured lines
/// (period `jolt_period_m` x stride ratio).
pub fn jolt_m(spec: &MotionSpec, travelled_m: f32) -> f32 {
    spec.jolt_lines
        .iter()
        .enumerate()
        .map(|(i, (amp, ratio))| {
            let period = (spec.jolt_period_m * ratio).max(1e-3);
            amp * (travelled_m / period * TAU + i as f32 * 1.7).sin()
        })
        .sum()
}

/// Roll (rad) of a cart: follows the first jolt line, 90 degrees later.
pub fn roll_rad(spec: &MotionSpec, travelled_m: f32) -> f32 {
    let period = (spec.jolt_period_m * spec.jolt_lines.first().map_or(1.0, |l| l.1)).max(1e-3);
    spec.roll_rad * (travelled_m / period * TAU + std::f32::consts::FRAC_PI_2).sin()
}

/// Grazing amount in [0, 1] of an animal at time `t`: in each cycle a share of the animals lowers
/// its head, with a ramp at both ends.
pub fn graze(spec: &MotionSpec, t: f32, seed: u64) -> f32 {
    if spec.graze_cycle_s <= 0.0 || spec.graze_share <= 0.0 {
        return 0.0;
    }
    let x = t / spec.graze_cycle_s + h01(seed, 0, 91);
    let cycle = x.floor() as u64;
    if h01(seed, cycle, 92) >= spec.graze_share {
        return 0.0;
    }
    let local = x.fract() * spec.graze_cycle_s;
    let ramp = spec.graze_ramp_s.max(0.05);
    let rise = (local / ramp).clamp(0.0, 1.0);
    let fall = ((spec.graze_cycle_s - local) / ramp).clamp(0.0, 1.0);
    let s = rise.min(fall);
    s * s * (3.0 - 2.0 * s)
}

/// Metres a cart or a beast walked at the time `t` (shared by every unit: the phase of a unit is
/// added by the caller).
pub fn travelled_m(t: f32, folk_speed: f32) -> f32 {
    t * WALK_SPEED_M_S * folk_speed
}

/// Value of a 16-sample step table at the phase `u` (cycles), linear and wrapped
/// (`am_swing_at` / `am_lift_at` of the shader).
pub fn lut_at(table: &[f32; 16], u: f32) -> f32 {
    let x = u.rem_euclid(1.0) * 16.0;
    let i = x.floor() as usize;
    let f = x - i as f32;
    table[i % 16] * (1.0 - f) + table[(i + 1) % 16] * f
}

/// Angle (rad) a wheel of radius `radius_m` has turned after `t` seconds at `speed` m/s.
pub fn wheel_angle(t: f32, speed: f32, radius_m: f32) -> f32 {
    t * speed / radius_m.max(1e-3)
}

/// Phase (cycles) of the beat of a draught beast: stride cadence plus the seed of the instance.
pub fn stride_turn(spec: &MotionSpec, t: f32, speed: f32, seed: f32) -> f32 {
    t * speed / spec.stride_m.max(0.05) + seed
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Pitch around X through `pivot` in the (first, second) plane (`am_pitch`).
fn pitch(v: Vec2, pivot: Vec2, a: f32) -> Vec2 {
    let d = v - pivot;
    let (s, c) = a.sin_cos();
    pivot + Vec2::new(d.x * c - d.y * s, d.y * c + d.x * s)
}

/// CPU reference of `prop_pose` of `anim.wesl` (`animal_pose` of the Godot shader) for one vertex
/// `v` of a model in the Godot frame (front along +Z, feet at y = 0), at time `t`, ground speed
/// `speed` (m/s) and instance `seed` in [0, 1). The GPU does the real work; this copy is the unit
/// tests' witness that the data drive the legs, head, tail and wheels.
pub fn prop_pose(spec: &MotionSpec, v_in: Vec3, t: f32, speed: f32, seed: f32) -> Vec3 {
    let mut v = v_in;
    let moving = smoothstep(0.05, 0.3, speed);
    let turn = std::f32::consts::TAU * stride_turn(spec, t, speed, seed);
    let cart_ref = if spec.has_beast {
        spec.stride_m
    } else {
        spec.jolt_period_m
    };
    let mut is_wheel = false;
    if spec.wheel_radius_m > 0.0 && v.x.abs() > spec.wheel_x_min_m {
        let r = spec.wheel_radius_m;
        let near = |z: f32| Vec2::new(v.y - r, v.z - z).length() < r + 0.03;
        let first = near(spec.axle_z_m[0]);
        let second = spec.axle_z_m.len() > 1 && near(spec.axle_z_m[1]);
        if first || second {
            let z = if second && !first {
                spec.axle_z_m[1]
            } else {
                spec.axle_z_m[0]
            };
            is_wheel = true;
            let yz = pitch(
                Vec2::new(v.y, v.z),
                Vec2::new(r, z),
                wheel_angle(t, speed, r),
            );
            v = Vec3::new(v.x, yz.x, yz.y);
        }
    }
    if speed > 0.0 && !is_wheel {
        let dist = t * speed;
        let jolt: f32 = spec
            .jolt_lines
            .iter()
            .take(3)
            .enumerate()
            .map(|(k, (amp, ratio))| {
                amp * (std::f32::consts::TAU
                    * (dist / (ratio * cart_ref) + seed * (1.0 + 0.37 * k as f32)))
                    .sin()
            })
            .sum();
        v.y += moving * jolt;
        if spec.wheel_radius_m > 0.0 && (v.z - spec.anchor_m).abs() > spec.half_m + 0.2 {
            let ratio = spec.jolt_lines.first().map_or(1.0, |l| l.1);
            let roll = spec.roll_rad
                * moving
                * (std::f32::consts::TAU * (dist / (ratio * cart_ref) + seed * 1.7)).sin();
            let xy = pitch(
                Vec2::new(v.x, v.y),
                Vec2::new(0.0, spec.wheel_radius_m),
                roll,
            );
            v = Vec3::new(xy.x, xy.y, v.z);
        }
    }
    if !spec.has_beast {
        return v;
    }
    let zr = v.z - spec.anchor_m;
    let half = spec.half_m;
    if zr < -half - spec.tail_len_m - 0.05 || zr > half + 1.4 {
        return v;
    }
    let (belly, withers) = (spec.belly_m, spec.withers_m);
    let side = if v.x >= 0.0 { 1.0 } else { -1.0 };
    let leg_w = ((belly - v.y) / belly).clamp(0.0, 1.0);
    let body_w = 1.0 - leg_w;
    let cyc_t = t + seed * 40.0;
    if zr.abs() < half * 0.95 {
        let front = if zr >= 0.0 { 0.25 } else { 0.0 };
        let leg_u = turn / std::f32::consts::TAU + if side > 0.0 { 0.0 } else { 0.5 } + front;
        v.z += lut_at(&spec.swing_lut, leg_u) * spec.swing_m * leg_w * moving;
        v.y += lut_at(&spec.lift_lut, leg_u) * spec.lift_m * leg_w * leg_w * moving;
    }
    v.y += spec.bob_m * (2.0 * turn + 1.0).sin() * body_w * moving;
    v.x += spec.shift_m * (cyc_t * 0.7).sin() * smoothstep(0.0, withers, v.y) * (1.0 - moving);
    v.y += spec.breath_m
        * (cyc_t * std::f32::consts::TAU * spec.breath_hz).sin()
        * smoothstep(belly, withers, v.y);
    let head_w = smoothstep(spec.neck_start_m, spec.neck_start_m + spec.neck_len_m, zr)
        * smoothstep(belly * 0.75, belly * 1.1, v.y);
    if head_w > 0.0 {
        let u = (cyc_t / spec.graze_cycle_s).rem_euclid(1.0);
        let ramp = (spec.graze_ramp_s / spec.graze_cycle_s.max(1.0)).clamp(0.01, 0.3);
        let mut graze = smoothstep(0.0, ramp, u)
            * (1.0 - smoothstep(spec.graze_share - ramp, spec.graze_share, u));
        graze *= 1.0 - moving;
        let chew = (cyc_t * std::f32::consts::TAU * spec.chew_hz).sin() * spec.chew_rad * graze;
        let nod = turn.sin() * spec.nod_rad * moving
            + (cyc_t * 0.9).sin() * spec.nod_rad * 0.4 * (1.0 - moving);
        let angle = (graze * spec.graze_rad + chew + nod) * head_w;
        let yz = pitch(
            Vec2::new(v.y, v.z),
            Vec2::new(spec.neck_pivot_m, spec.anchor_m + spec.neck_start_m),
            angle,
        );
        v = Vec3::new(v.x, yz.x, yz.y);
    }
    let tail_w = ((-zr - half) / spec.tail_len_m).clamp(0.0, 1.0)
        * if v.x.abs() <= 0.1 { 1.0 } else { 0.0 }
        * if v.y >= belly * 0.6 { 1.0 } else { 0.0 };
    if tail_w > 0.0 {
        let swish = 0.5 + 0.5 * (cyc_t * 0.37).sin().max(0.0).powi(3);
        let rate = spec.tail_hz * (1.0 + (spec.tail_walk_gain - 1.0) * moving);
        v.x += tail_w * spec.tail_swing_m * swish * (cyc_t * std::f32::consts::TAU * rate).sin();
    }
    v
}

/// Warped time of a herd animal (`fauna.gdshader`): the speed falls to zero (stops to graze) then
/// doubles. Returns (`tau`, `moving` in [0, 1]); `herd_seed` is the [0, 1) seed of the herd.
pub fn herd_clock(t: f32, herd_seed: f32, stop_seed: f32) -> (f32, f32) {
    let w = 0.09 + 0.05 * herd_seed;
    let u = w * t + stop_seed * std::f32::consts::TAU;
    (t - u.sin() / w, smoothstep(0.08, 0.5, 1.0 - u.cos()))
}

/// Tag of a prop instance: ground speed (mm/s, low 16 bits) and seed (high 16 bits).
pub fn pack_prop_tag(speed_m_s: f32, seed: f32) -> u32 {
    let speed = (speed_m_s * 1000.0).round().clamp(0.0, 65535.0) as u32;
    let seed = (seed.rem_euclid(1.0) * 65536.0) as u32 & 0xffff;
    speed | (seed << 16)
}

pub fn unpack_prop_tag(tag: u32) -> (f32, f32) {
    ((tag & 0xffff) as f32 * 0.001, (tag >> 16) as f32 / 65536.0)
}

/// Tag of a herd animal: leg phase in cycles (low 16 bits) and moving (8 bits).
pub fn pack_herd_tag(phase_cycles: f32, moving: f32) -> u32 {
    let phase = (phase_cycles.rem_euclid(1.0) * 65536.0) as u32 & 0xffff;
    let moving = (moving.clamp(0.0, 1.0) * 255.0).round() as u32;
    phase | (moving << 16)
}

pub fn unpack_herd_tag(tag: u32) -> (f32, f32) {
    (
        (tag & 0xffff) as f32 / 65536.0,
        ((tag >> 16) & 0xff) as f32 / 255.0,
    )
}

/// Body bob of the draught beasts and jolt/roll of the carts: stand-in for the models that the
/// vertex stage does not animate (non-DN placeholders); a bound model (`anim.rs`) does it in
/// the shader, with the same measured lines.
pub fn animate_carts(
    table: Res<MotionTable>,
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut carts: Query<(&mut Cart, &Children)>,
    mut beasts: Query<(&Beast, &mut Transform, &Children)>,
    bound: Query<&super::anim::AnimBound>,
) {
    if !table.enabled {
        return;
    }
    let is_bound = |entity: Entity| bound.get(entity).is_ok_and(|b| !b.meshes.is_empty());
    let ws = world_scale(&data);
    let travelled = travelled_m(view.time, data.props.folk_speed);
    for (mut cart, children) in &mut carts {
        let phase_m = cart.phase * 7.3;
        // The cart model is a direct child that is not a team holder.
        let shader_box = children.iter().any(is_bound);
        if let Some(spec) = table.spec(&cart.model) {
            if shader_box {
                cart.lift = 0.0;
                cart.roll = 0.0;
            } else {
                cart.lift = jolt_m(spec, travelled + phase_m) * ws;
                cart.roll = roll_rad(spec, travelled + phase_m);
            }
        }
        let Some(team) = cart.team_model.as_deref().and_then(|m| table.spec(m)) else {
            continue;
        };
        for child in children.iter() {
            if let Ok((_, mut t, grand)) = beasts.get_mut(child) {
                t.translation.y = if grand.iter().any(is_bound) {
                    0.0
                } else {
                    bob_m(team, travelled + phase_m) * ws
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> MotionTable {
        let root = crate::paths::find_root().expect("root");
        MotionTable::load(&crate::paths::data_dir(&root))
    }

    #[test]
    fn real_table_merges_base_and_defaults() {
        let t = table();
        assert!(t.enabled);
        let horse = t.spec("animal_horse_draught").expect("horse");
        assert!(horse.has_beast);
        // Entry < base: stride comes from `horse`, graze from the defaults.
        assert!((horse.stride_m - 1.76).abs() < 1e-3);
        assert!((horse.graze_cycle_s - 12.0).abs() < 1e-3);
        let cart = t.spec("cart_merchant").expect("cart");
        assert!(!cart.has_beast);
        assert_eq!(cart.jolt_lines.len(), 3);
    }

    #[test]
    fn bob_and_jolt_are_bounded_and_move() {
        let t = table();
        let horse = t.spec("animal_horse_draught").unwrap();
        let cart = t.spec("cart_merchant").unwrap();
        let mut seen = (f32::MAX, f32::MIN);
        for i in 0..200 {
            let s = i as f32 * 0.1;
            let b = bob_m(horse, s);
            assert!((0.0..=horse.bob_m + 1e-6).contains(&b));
            let j = jolt_m(cart, s);
            seen = (seen.0.min(j), seen.1.max(j));
            assert!(roll_rad(cart, s).abs() <= cart.roll_rad + 1e-6);
        }
        assert!(seen.1 > 0.0 && seen.0 < 0.0, "jolt oscillates around 0");
        let max_amp: f32 = cart.jolt_lines.iter().map(|l| l.0).sum();
        assert!(seen.1 <= max_amp + 1e-6);
    }

    #[test]
    fn grazing_is_a_share_of_the_animals() {
        let t = table();
        let sheep = t.spec("animal_sheep_wool").unwrap();
        let mut grazing = 0;
        for a in 0..400u64 {
            if graze(sheep, 6.0, a) > 0.5 {
                grazing += 1;
            }
        }
        // `graze_share` of the animals graze, minus the ramps: well above 0, well below all.
        assert!(grazing > 40 && grazing < 400, "{grazing}");
        assert!((0.0..=1.0).contains(&graze(sheep, 0.0, 1)));
    }

    fn horse() -> MotionSpec {
        table().spec("animal_horse_draught").unwrap().clone()
    }

    #[test]
    fn step_tables_wrap_and_interpolate() {
        let h = horse();
        for i in 0..16 {
            let u = i as f32 / 16.0;
            assert!((lut_at(&h.swing_lut, u) - h.swing_lut[i]).abs() < 1e-5);
            assert!(
                (lut_at(&h.lift_lut, u + 3.0) - h.lift_lut[i]).abs() < 1e-5,
                "wraps"
            );
        }
        let mid = lut_at(&h.swing_lut, 0.5 / 16.0);
        assert!((mid - (h.swing_lut[0] + h.swing_lut[1]) * 0.5).abs() < 1e-5);
        assert!((lut_at(&h.lift_lut, -0.25) - lut_at(&h.lift_lut, 0.75)).abs() < 1e-6);
        // Measured shape: the hoof lift stays within [0, 1], the swing reaches both sides.
        assert!(h.lift_lut.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(h.swing_lut.iter().any(|v| *v > 0.9) && h.swing_lut.iter().any(|v| *v < -0.5));
    }

    #[test]
    fn wheels_turn_with_the_distance_travelled() {
        let r = 0.6;
        // One revolution after a circumference at any speed.
        for speed in [0.3, 0.6, 1.2] {
            let t = std::f32::consts::TAU * r / speed;
            assert!((wheel_angle(t, speed, r) - std::f32::consts::TAU).abs() < 1e-4);
        }
        assert_eq!(
            wheel_angle(5.0, 0.0, r),
            0.0,
            "a stopped cart does not turn"
        );
        assert!(wheel_angle(2.0, 1.2, r) > wheel_angle(1.0, 1.2, r));
    }

    #[test]
    fn the_walking_horse_swings_its_legs_and_lifts_its_hooves() {
        let h = horse();
        let hoof = Vec3::new(0.2, 0.05, h.anchor_m - h.half_m * 0.5);
        let (mut zs, mut ys) = ((f32::MAX, f32::MIN), (f32::MAX, f32::MIN));
        for k in 0..400 {
            let d = prop_pose(&h, hoof, k as f32 * 0.025, 1.2, 0.0) - hoof;
            zs = (zs.0.min(d.z), zs.1.max(d.z));
            ys = (ys.0.min(d.y), ys.1.max(d.y));
        }
        // Swing: up to swing_m forward and back (the table peaks around 1 and -0.9).
        let leg_w = (h.belly_m - hoof.y) / h.belly_m;
        assert!(
            zs.1 > h.swing_m * leg_w * 0.9 && zs.0 < -h.swing_m * leg_w * 0.7,
            "{zs:?}"
        );
        assert!(
            zs.1 <= h.swing_m * leg_w * 1.05 + 1e-4,
            "bounded by the measured amplitude"
        );
        assert!(ys.1 > h.lift_m * 0.5, "the hoof rises {ys:?}");
        // Standing: the legs do not move at all.
        for k in 0..50 {
            let d = prop_pose(&h, hoof, k as f32 * 0.3, 0.0, 0.0) - hoof;
            assert!(d.z.abs() < 1e-6 && d.y.abs() < 2e-3, "{d:?}");
        }
    }

    #[test]
    fn the_head_lowers_to_graze_only_when_standing() {
        let h = horse();
        let head = Vec3::new(0.0, h.withers_m, h.anchor_m + h.half_m + 0.1);
        let lowest = |speed: f32| {
            (0..1200)
                .map(|k| prop_pose(&h, head, k as f32 * 0.05, speed, 0.3).y - head.y)
                .fold(f32::MAX, f32::min)
        };
        assert!(
            lowest(0.0) < -0.25,
            "grazing lowers the head: {}",
            lowest(0.0)
        );
        assert!(lowest(1.2) > -0.1, "walking only nods: {}", lowest(1.2));
    }

    #[test]
    fn the_tail_swats_and_the_hub_of_a_wheel_stays_put() {
        let h = horse();
        let tail = Vec3::new(0.0, h.belly_m, h.anchor_m - h.half_m - 0.03);
        let xs: Vec<f32> = (0..800)
            .map(|k| prop_pose(&h, tail, k as f32 * 0.05, 0.0, 0.0).x)
            .collect();
        let span = xs.iter().cloned().fold(f32::MIN, f32::max)
            - xs.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            span > h.tail_swing_m * 0.3 && span < h.tail_swing_m * 2.0 + 0.02,
            "{span}"
        );
        let c = table().spec("cart_merchant").unwrap().clone();
        let hub = Vec3::new(c.wheel_x_min_m + 0.1, c.wheel_radius_m, c.axle_z_m[0]);
        for k in 0..20 {
            let p = prop_pose(&c, hub, k as f32 * 0.7, 1.2, 0.0);
            assert!((p.x - hub.x).abs() < 1e-4 && (p.z - hub.z).abs() < 1e-4);
        }
    }

    #[test]
    fn a_wheel_rim_comes_back_after_one_turn_and_the_box_only_jolts() {
        let c = table().spec("cart_merchant").unwrap().clone();
        let r = c.wheel_radius_m;
        let rim = Vec3::new(c.wheel_x_min_m + 0.1, 2.0 * r - 0.02, c.axle_z_m[0]);
        let speed = 1.2;
        let period = std::f32::consts::TAU * r / speed;
        let a = prop_pose(&c, rim, 0.0, speed, 0.0);
        let b = prop_pose(&c, rim, period, speed, 0.0);
        let half = prop_pose(&c, rim, period * 0.5, speed, 0.0);
        assert!(a.distance(b) < 1e-3, "full turn");
        assert!(
            a.distance(half) > r,
            "half a turn moves the rim to the other side"
        );
        // The box (a point far from the wheels): vertical jolt within the sum of the lines, no roll
        // beyond the measured angle.
        let sum: f32 = c.jolt_lines.iter().map(|l| l.0).sum();
        let box_point = Vec3::new(0.0, 1.0, 0.0);
        for k in 0..400 {
            let d = prop_pose(&c, box_point, k as f32 * 0.07, speed, 0.4) - box_point;
            assert!(d.y.abs() <= sum + 1e-4);
            assert!(d.x.abs() <= (1.0 - c.wheel_radius_m).abs() * c.roll_rad + 1e-4);
        }
        // A cart that does not move does not jolt.
        assert!(prop_pose(&c, box_point, 3.0, 0.0, 0.4).distance(box_point) < 1e-6);
    }

    #[test]
    fn herd_clock_stops_and_restarts_the_legs() {
        let mut last_tau = f32::MIN;
        let (mut stopped, mut walking) = (0, 0);
        for k in 0..4000 {
            let (tau, moving) = herd_clock(k as f32 * 0.1, 0.3, 0.7);
            assert!(tau >= last_tau - 1e-3, "warped time never runs back");
            assert!((0.0..=1.0).contains(&moving));
            last_tau = tau;
            stopped += usize::from(moving < 0.05);
            walking += usize::from(moving > 0.95);
        }
        assert!(
            stopped > 100 && walking > 100,
            "{stopped} stops, {walking} walking"
        );
        // Same herd seeds give the same clock; other seeds do not.
        assert_eq!(herd_clock(50.0, 0.3, 0.7), herd_clock(50.0, 0.3, 0.7));
        assert_ne!(herd_clock(50.0, 0.3, 0.7).0, herd_clock(50.0, 0.8, 0.1).0);
    }

    #[test]
    fn tags_round_trip() {
        for (speed, seed) in [(0.0, 0.0), (0.6, 0.25), (1.2, 0.999), (30.0, 0.5)] {
            let (s, d) = unpack_prop_tag(pack_prop_tag(speed, seed));
            assert!(
                (s - speed).abs() < 1e-3 && (d - seed).abs() < 2e-5,
                "{speed} {seed}"
            );
        }
        for (phase, moving) in [(0.0, 0.0), (0.37, 1.0), (7.25, 0.5), (-0.25, 0.2)] {
            let (p, m) = unpack_herd_tag(pack_herd_tag(phase, moving));
            assert!((p - phase.rem_euclid(1.0)).abs() < 2e-5 && (m - moving).abs() < 3e-3);
        }
    }

    #[test]
    fn baked_data_matches_the_measurements() {
        // `animal_motion_measured.json` is the record of the video measurements that
        // `animal_motion.json` bakes: it is not read at run time (the Godot game does not either), so
        // this test is what keeps both files in agreement.
        let root = crate::paths::find_root().expect("root");
        let dir = crate::paths::data_dir(&root);
        let measured: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.join("fx/animal_motion_measured.json")).unwrap(),
        )
        .unwrap();
        let t = table();
        let cart = t.spec("cart_merchant").unwrap();
        for (baked, line) in cart
            .jolt_lines
            .iter()
            .zip(measured["wagon"]["jolt_lines"].as_array().unwrap())
        {
            assert!((baked.0 - line["amp_m"].as_f64().unwrap() as f32).abs() < 1e-4);
        }
        assert!(
            (cart.roll_rad - measured["wagon"]["roll_half_rad"].as_f64().unwrap() as f32).abs()
                < 1e-4
        );
        assert!(
            (horse().stride_m - measured["wagon"]["horse_stride_m"].as_f64().unwrap() as f32).abs()
                < 1e-3
        );
        let cow = t.spec("animal_cattle_red").unwrap();
        assert!(
            (cow.graze_rad - measured["cattle"]["graze_delta_rad"].as_f64().unwrap() as f32).abs()
                < 0.01
        );
        assert!(
            (cow.nod_rad - measured["cattle"]["head_nod_half_rad"].as_f64().unwrap() as f32).abs()
                < 1e-3
        );
        let sheep = t.spec("animal_sheep_wool").unwrap();
        assert!(
            (sheep.graze_rad - measured["sheep"]["graze_delta_rad"].as_f64().unwrap() as f32).abs()
                < 0.01
        );
    }

    #[test]
    fn camp_horse_chewing_reaches_the_campaign_horse() {
        let h = horse();
        assert!((h.chew_hz - 1.11).abs() < 1e-4 && (h.chew_rad - 0.034).abs() < 1e-4);
        let ox = table().spec("animal_ox_draught").unwrap().clone();
        assert!(
            (ox.chew_hz - 0.84).abs() < 1e-4,
            "the ox keeps the cattle values"
        );
    }

    #[test]
    fn disabled_table_gives_no_spec() {
        let mut t = table();
        t.enabled = false;
        assert!(t.spec("animal_horse_draught").is_none());
    }
}
