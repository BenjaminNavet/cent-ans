//! Regiment state to animation clips. Port of the `STYLES` / `state_config` part of
//! game/scripts/battle/battle_skinned.gd: each figure has a style (sword, pike, bow, lance...)
//! giving, per render state, a set of clips and how a soldier picks among them.

use std::collections::{BTreeMap, HashMap};

/// Most clips in one set (two per component of the shader's `clip_set`).
pub const MAX_SET: usize = 8;

/// How a soldier picks and times its clip (`M_*` of the shader).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Looping clip chosen once by the soldier's hash, random phase.
    Loop = 0,
    /// A new clip drawn every `cycle` seconds (melee exchanges).
    Cycle = 1,
    /// One shot per volley, desynchronised by the soldier's hash.
    Volley = 2,
}

/// One state of a style.
#[derive(Debug, Clone, Copy)]
pub struct StyleEntry {
    pub set: &'static [&'static str],
    pub mode: Mode,
    pub speed: f32,
    pub cycle: f32,
    pub release: f32,
    pub fallback: &'static [&'static str],
}

const fn entry(set: &'static [&'static str]) -> StyleEntry {
    StyleEntry {
        set,
        mode: Mode::Loop,
        speed: 1.0,
        cycle: 1.5,
        release: 1.0,
        fallback: &[],
    }
}

impl StyleEntry {
    const fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
    const fn cycle(mut self, cycle: f32) -> Self {
        self.mode = Mode::Cycle;
        self.cycle = cycle;
        self
    }
    const fn volley(mut self, release: f32) -> Self {
        self.mode = Mode::Volley;
        self.release = release;
        self
    }
    const fn fallback(mut self, fallback: &'static [&'static str]) -> Self {
        self.fallback = fallback;
        self
    }
}

const FLEE: StyleEntry = entry(&["flee", "flee_m"]).speed(1.1).fallback(&["run"]);
const CAVALRY_CHARGE: StyleEntry = entry(&[
    "c_charge", "c_charge", "c_charge", "c_charge", "c_charge", "c_charge", "c_charge",
    "c_stumble",
])
.cycle(3.75);

/// Clip set of `state` for `style` (`None` when the style does not know the state).
pub fn style_entry(style: &str, state: &str) -> Option<StyleEntry> {
    let e = match (style, state) {
        ("sword", "idle") => entry(&["guard", "idle", "guard", "idle_look", "idle_lean", "idle_helm"]),
        ("sword", "marching") => entry(&["walk"]),
        ("sword", "running") => entry(&["run"]),
        ("sword", "charging") => entry(&["run"]).speed(1.05),
        ("sword", "melee") => entry(&["slash", "thrust", "overhead", "parry", "slash", "thrust", "guard"]).cycle(1.3),
        ("sword", "victory") => entry(&["victory", "victory_b"]),
        ("sword", "routing") => FLEE,
        ("sword", "climbing") => entry(&["climb", "guard", "idle"]),

        ("militia", "idle") => entry(&["pike_idle", "pike_idle", "pike_look"]),
        ("militia", "victory") => entry(&["victory_pike"]),
        ("militia", "marching") => entry(&["pike_walk"]),
        ("militia", "running") => entry(&["run"]),
        ("militia", "charging") => entry(&["pike_level_walk"]).speed(1.3),
        ("militia", "melee") => entry(&["pike_thrust", "pike_thrust", "pike_level"]).cycle(1.3),
        ("militia", "brace") => entry(&["pike_level"]),
        ("militia", "routing") => entry(&["flee", "flee_m"]).speed(1.15).fallback(&["run"]),
        ("militia", "climbing") => entry(&["climb", "idle", "guard"]),

        ("pike", "idle") => entry(&["pike_idle", "pike_idle", "pike_look"]),
        ("pike", "victory") => entry(&["victory_pike"]),
        ("pike", "marching") => entry(&["pike_walk"]),
        ("pike", "running") => entry(&["run"]),
        ("pike", "charging") => entry(&["pike_level_walk"]).speed(1.3),
        ("pike", "melee") => entry(&["pike_thrust", "pike_thrust", "pike_idle"]).cycle(1.2),
        ("pike", "brace") => entry(&["pike_level"]),
        ("pike", "routing") => FLEE,
        ("pike", "climbing") => entry(&["climb", "pike_idle"]),

        ("bow", "idle") => entry(&["bow_idle", "idle", "bow_look", "idle_look"]),
        ("bow", "marching") => entry(&["bow_walk"]).fallback(&["walk"]),
        ("bow", "running") => entry(&["run"]),
        ("bow", "charging") => entry(&["run"]),
        ("bow", "shooting") => entry(&["bow_shoot"]).volley(1.55),
        ("bow", "melee") => entry(&["push", "push_shoulder", "parry", "push", "guard"]).cycle(1.5).fallback(&["slash", "thrust", "parry", "guard"]),
        ("bow", "routing") => entry(&["flee", "flee_m"]).speed(1.15).fallback(&["run"]),
        ("bow", "victory") => entry(&["victory", "victory_b"]),

        ("crossbow", "idle") => entry(&["xbow_idle", "xbow_idle", "xbow_look"]),
        ("crossbow", "marching") => entry(&["xbow_walk"]).fallback(&["walk"]),
        ("crossbow", "running") => entry(&["run"]),
        ("crossbow", "charging") => entry(&["run"]),
        ("crossbow", "shooting") => entry(&["xbow_shoot"]).volley(0.3),
        ("crossbow", "melee") => entry(&["push", "push_shoulder", "parry", "push", "guard"]).cycle(1.5).fallback(&["slash", "thrust", "parry", "guard"]),
        ("crossbow", "routing") => entry(&["flee", "flee_m"]).speed(1.15).fallback(&["run"]),
        ("crossbow", "victory") => entry(&["victory"]),

        ("lance", "idle") => entry(&["c_idle"]),
        ("lance", "marching") => entry(&["c_walk"]),
        ("lance", "running") => entry(&["c_gallop"]),
        ("lance", "charging") => CAVALRY_CHARGE,
        ("lance", "melee") => entry(&["c_thrust", "c_thrust", "c_rear", "c_turn_l", "c_thrust", "c_turn_r", "c_idle"]).cycle(1.4),
        ("lance", "melee_pikes") => entry(&["c_rear", "c_thrust", "c_rear", "c_idle"]).cycle(1.4),
        ("lance", "trotting") => entry(&["c_trot"]),
        ("lance", "turn_l") => entry(&["c_turn_l"]),
        ("lance", "turn_r") => entry(&["c_turn_r"]),
        ("lance", "trot_turn_l") => entry(&["c_trot_turn_l"]),
        ("lance", "trot_turn_r") => entry(&["c_trot_turn_r"]),
        ("lance", "routing") => entry(&["c_gallop"]),
        ("lance", "victory") => entry(&["c_victory"]),

        ("horse_bow", "idle") => entry(&["c_bow_idle"]),
        ("horse_bow", "marching") => entry(&["c_bow_walk"]),
        ("horse_bow", "running") => entry(&["c_gallop"]),
        ("horse_bow", "charging") => entry(&["c_gallop"]),
        ("horse_bow", "shooting") => entry(&["c_bow_shoot"]).volley(1.55),
        ("horse_bow", "melee") => entry(&["c_thrust", "c_bow_turn_l", "c_thrust", "c_bow_turn_r", "c_rear", "c_bow_idle"]).cycle(1.4),
        ("horse_bow", "melee_pikes") => entry(&["c_rear", "c_bow_idle"]).cycle(1.4),
        ("horse_bow", "trotting") => entry(&["c_bow_trot"]),
        ("horse_bow", "turn_l") => entry(&["c_bow_turn_l"]),
        ("horse_bow", "turn_r") => entry(&["c_bow_turn_r"]),
        ("horse_bow", "trot_turn_l") => entry(&["c_bow_trot_turn_l"]),
        ("horse_bow", "trot_turn_r") => entry(&["c_bow_trot_turn_r"]),
        ("horse_bow", "routing") => entry(&["c_gallop"]),
        ("horse_bow", "victory") => entry(&["c_victory"]),

        ("horse_javelin", "idle") => entry(&["c_javelin_idle"]),
        ("horse_javelin", "marching") => entry(&["c_javelin_walk"]),
        ("horse_javelin", "running") => entry(&["c_gallop"]),
        ("horse_javelin", "charging") => entry(&["c_gallop"]),
        ("horse_javelin", "shooting") => entry(&["c_javelin_throw"]).volley(0.69),
        ("horse_javelin", "melee") => entry(&["c_thrust", "c_javelin_turn_l", "c_thrust", "c_javelin_turn_r", "c_rear", "c_javelin_idle"]).cycle(1.4),
        ("horse_javelin", "melee_pikes") => entry(&["c_rear", "c_javelin_idle"]).cycle(1.4),
        ("horse_javelin", "trotting") => entry(&["c_javelin_trot"]),
        ("horse_javelin", "turn_l") => entry(&["c_javelin_turn_l"]),
        ("horse_javelin", "turn_r") => entry(&["c_javelin_turn_r"]),
        ("horse_javelin", "trot_turn_l") => entry(&["c_javelin_trot_turn_l"]),
        ("horse_javelin", "trot_turn_r") => entry(&["c_javelin_trot_turn_r"]),
        ("horse_javelin", "routing") => entry(&["c_gallop"]),
        ("horse_javelin", "victory") => entry(&["c_victory"]),
        _ => return None,
    };
    Some(e)
}

/// Cavalry render states absent from the style fall back to the next simpler gait.
pub fn gait_fallback(state: &str) -> Option<&'static str> {
    Some(match state {
        "trotting" | "turn_l" | "turn_r" => "marching",
        "trot_turn_l" | "trot_turn_r" => "trotting",
        _ => return None,
    })
}

/// Style of a figure: manifest `style` when known, else the historical rule per family.
pub fn style_of(manifest_style: Option<&str>, kind: &str, variant: u32) -> String {
    if let Some(style) = manifest_style
        && style_entry(style, "idle").is_some()
    {
        return style.to_owned();
    }
    match kind {
        "infantry" => match variant {
            0 => "sword",
            1 => "pike",
            _ => "militia",
        },
        "archer" => {
            if variant == 0 {
                "bow"
            } else {
                "crossbow"
            }
        }
        "cavalry" => {
            if variant == 2 {
                "horse_bow"
            } else {
                "lance"
            }
        }
        _ => "sword",
    }
    .to_owned()
}

/// One clip of a rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipInfo {
    pub start: u32,
    pub frames: u32,
    pub looped: bool,
}

/// Clips of a rig: shader indices are positions in the sorted name list (`clip_index`).
#[derive(Debug, Clone, Default)]
pub struct RigClips {
    pub fps: f32,
    pub names: Vec<String>,
    by_name: HashMap<String, ClipInfo>,
}

impl RigClips {
    pub fn new(fps: f32, clips: &BTreeMap<String, ClipInfo>) -> Self {
        Self {
            fps,
            names: clips.keys().cloned().collect(),
            by_name: clips.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&ClipInfo> {
        self.by_name.get(name)
    }

    pub fn index(&self, name: &str) -> u32 {
        self.names.iter().position(|n| n == name).unwrap_or(0) as u32
    }

    /// Clips of `names` present in the rig, duplicates kept (they weigh in the draw), at most
    /// [`MAX_SET`].
    pub fn present(&self, names: &[&str]) -> Vec<String> {
        names
            .iter()
            .filter(|n| self.by_name.contains_key(**n))
            .take(MAX_SET)
            .map(|n| (*n).to_owned())
            .collect()
    }

    /// Longest non looping clip of `names` in seconds; `fallback_s` when all of them loop.
    pub fn melee_cycle_s(&self, names: &[String], fallback_s: f32) -> f32 {
        let longest = names
            .iter()
            .filter_map(|n| self.by_name.get(n))
            .filter(|c| !c.looped)
            .map(|c| c.frames as f32 / self.fps)
            .fold(0.0, f32::max);
        if longest > 0.0 { longest } else { fallback_s }
    }
}

/// Animation of a regiment in a render state (`state_config`).
#[derive(Debug, Clone, PartialEq)]
pub struct AnimConfig {
    /// Cache key, `style/state`.
    pub key: String,
    pub names: Vec<String>,
    pub set: Vec<u32>,
    pub mode: Mode,
    pub speed: f32,
    pub cycle: f32,
    pub release: f32,
    /// Share of the clip speed a soldier may deviate by (`rate_jitter`).
    pub jitter: f32,
}

/// Locomotion states: cadence follows the speed, little jitter.
pub fn is_locomotion(state: &str) -> bool {
    matches!(
        state,
        "marching"
            | "running"
            | "charging"
            | "routing"
            | "trotting"
            | "turn_l"
            | "turn_r"
            | "trot_turn_l"
            | "trot_turn_r"
    )
}

/// Jitter settings of `battle_animation.json`.
#[derive(Debug, Clone, Copy)]
pub struct JitterSettings {
    pub loco: f32,
    pub idle: f32,
}

impl Default for JitterSettings {
    fn default() -> Self {
        Self {
            loco: 0.07,
            idle: 0.07,
        }
    }
}

/// Configuration of `style` in `state` (`running`: marching at the double).
pub fn state_config(
    style: &str,
    rig: &RigClips,
    state: &str,
    running: bool,
    jitter: JitterSettings,
) -> AnimConfig {
    let mut key = if state == "marching" && running {
        "running"
    } else {
        state
    };
    let usable = |k: &str| {
        style_entry(style, k).is_some_and(|e| !rig.present(e.set).is_empty())
    };
    while let Some(next) = gait_fallback(key) {
        if usable(key) {
            break;
        }
        key = next;
    }
    if style_entry(style, key).is_none() {
        key = "idle";
    }
    let mut chosen = style_entry(style, key).unwrap_or(entry(&["idle"]));
    let mut names = rig.present(chosen.set);
    if !chosen.fallback.is_empty()
        && names.first().map(String::as_str) != chosen.set.first().copied()
    {
        names = chosen.fallback.iter().map(|s| (*s).to_owned()).collect();
    }
    if names.is_empty() {
        chosen = style_entry(style, "idle").unwrap_or(entry(&["idle"]));
        names = rig.present(chosen.set);
    }
    if names.is_empty() {
        names = vec![chosen.set[0].to_owned()];
    }
    let mut cycle = chosen.cycle;
    if chosen.mode == Mode::Cycle && key.starts_with("melee") {
        cycle = rig.melee_cycle_s(&names, cycle);
    }
    AnimConfig {
        key: format!("{style}/{key}"),
        set: names.iter().map(|n| rig.index(n)).collect(),
        names,
        mode: chosen.mode,
        speed: chosen.speed,
        cycle,
        release: chosen.release,
        jitter: if chosen.mode == Mode::Loop && is_locomotion(key) {
            jitter.loco
        } else {
            jitter.idle
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rig(names: &[(&str, u32, bool)]) -> RigClips {
        let clips: BTreeMap<String, ClipInfo> = names
            .iter()
            .enumerate()
            .map(|(i, (n, frames, looped))| {
                (
                    (*n).to_owned(),
                    ClipInfo {
                        start: i as u32 * 100,
                        frames: *frames,
                        looped: *looped,
                    },
                )
            })
            .collect();
        RigClips::new(24.0, &clips)
    }

    #[test]
    fn missing_gait_falls_back_and_missing_state_to_idle() {
        let r = rig(&[("c_idle", 40, true), ("c_walk", 30, true), ("c_gallop", 20, true)]);
        let j = JitterSettings::default();
        let cfg = state_config("lance", &r, "trot_turn_l", false, j);
        assert_eq!(cfg.key, "lance/marching");
        assert_eq!(cfg.names, ["c_walk"]);
        let cfg = state_config("lance", &r, "marching", true, j);
        assert_eq!(cfg.key, "lance/running");
        let cfg = state_config("lance", &r, "rallied", false, j);
        assert_eq!(cfg.key, "lance/idle");
    }

    #[test]
    fn melee_cycle_matches_longest_one_shot_and_fallback_is_used() {
        let r = rig(&[
            ("slash", 36, false),
            ("thrust", 48, false),
            ("guard", 24, true),
            ("walk", 24, true),
            ("idle", 24, true),
        ]);
        let cfg = state_config("sword", &r, "melee", false, JitterSettings::default());
        assert_eq!(cfg.mode, Mode::Cycle);
        assert!((cfg.cycle - 2.0).abs() < 1e-6);
        assert_eq!(cfg.set.len(), cfg.names.len());
        // bow melee: the preferred clips are absent, the fallback applies.
        let cfg = state_config("bow", &r, "melee", false, JitterSettings::default());
        assert_eq!(cfg.names, ["slash", "thrust", "parry", "guard"]);
    }

    #[test]
    fn locomotion_loops_use_the_smaller_jitter() {
        let r = rig(&[("walk", 24, true), ("guard", 24, true)]);
        let j = JitterSettings { loco: 0.03, idle: 0.07 };
        assert_eq!(state_config("sword", &r, "marching", false, j).jitter, 0.03);
        assert_eq!(state_config("sword", &r, "idle", false, j).jitter, 0.07);
    }

    #[test]
    fn style_falls_back_to_the_family_rule() {
        assert_eq!(style_of(None, "infantry", 1), "pike");
        assert_eq!(style_of(Some("bogus"), "archer", 2), "crossbow");
        assert_eq!(style_of(Some("militia"), "infantry", 0), "militia");
        assert_eq!(style_of(None, "cavalry", 2), "horse_bow");
    }
}
