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
                        .map(|a| a.iter().filter_map(Value::as_f64).map(|x| x as f32).collect())
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
                    serde_json::from_str::<Value>(&t).map_err(|e| format!("{}: {e}", path.display()))
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

/// Body bob of the draught beasts and jolt/roll of the carts.
pub fn animate_carts(
    table: Res<MotionTable>,
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut carts: Query<(&mut Cart, &Children)>,
    mut beasts: Query<(&Beast, &mut Transform)>,
) {
    if !table.enabled {
        return;
    }
    let ws = world_scale(&data);
    let travelled = travelled_m(view.time, data.props.folk_speed);
    for (mut cart, children) in &mut carts {
        let phase_m = cart.phase * 7.3;
        if let Some(spec) = table.spec(&cart.model) {
            cart.lift = jolt_m(spec, travelled + phase_m) * ws;
            cart.roll = roll_rad(spec, travelled + phase_m);
        }
        let Some(team) = cart.team_model.as_deref().and_then(|m| table.spec(m)) else {
            continue;
        };
        for child in children.iter() {
            if let Ok((_, mut t)) = beasts.get_mut(child) {
                t.translation.y = bob_m(team, travelled + phase_m) * ws;
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

    #[test]
    fn disabled_table_gives_no_spec() {
        let mut t = table();
        t.enabled = false;
        assert!(t.spec("animal_horse_draught").is_none());
    }
}
