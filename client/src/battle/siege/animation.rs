//! Motion of the static siege engines and settings of their crew (port of the maths of
//! game/scripts/battle/siege_engines_fx.gd and the layouts of siege_crew_fx.gd).
//!
//! Everything here is pure (settings, poses, choice of gesture) and tested; `engines.rs` applies
//! the poses to the named nodes of the models and `crew.rs` draws the servants.
//!
//! - Trebuchet: the counterweight falls, the arm swings through the measured curve
//!   (`data/fx/trebuchet_swing_curve.json`, CC BY-SA 3.0), the sling whips over and lets the stone
//!   go at `release_phase`; the arm then oscillates and the winch brings it back.
//! - Mangonel: the arm slaps against the crossbar, winch on reload.
//! - Bombard: the barrel recoils along its axis, the mantlet is raised while the regiment fires.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::Path;

use serde::Deserialize;

/// Settings of the trebuchet (`data/fx/siege_engines.json`, `trebuchet`).
#[derive(Debug, Clone, Deserialize)]
pub struct Trebuchet {
    pub cocked_deg: f32,
    pub rest_deg: f32,
    pub overswing_deg: f32,
    pub swing_s: f32,
    pub release_phase: f32,
    pub sling_release_deg: f32,
    pub sling_end_deg: f32,
    pub sling_power: f32,
    pub settle_s: f32,
    pub settle_hz: f32,
    pub settle_damping: f32,
    pub counterweight_lag: f32,
    pub winch_turns: f32,
    pub ready_margin_s: f32,
    pub stagger_s: f32,
    #[serde(default)]
    pub predict: bool,
    /// Measured progress of the arm (0..1 over `0..1` of the swing), when the file is there.
    #[serde(skip)]
    pub lut: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mangonel {
    pub cocked_deg: f32,
    pub fired_deg: f32,
    pub swing_s: f32,
    pub release_phase: f32,
    pub bounce_deg: f32,
    pub settle_s: f32,
    pub winch_turns: f32,
    pub ready_margin_s: f32,
    pub stagger_s: f32,
    #[serde(default)]
    pub predict: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bombard {
    pub recoil_m: f32,
    pub recoil_s: f32,
    pub hold_s: f32,
    pub return_s: f32,
    pub mantlet_raised_deg: f32,
    pub mantlet_s: f32,
    pub stagger_s: f32,
}

/// One gesture, or a list that alternates from one servant to the next.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ClipChoice {
    One(String),
    Many(Vec<String>),
}

impl ClipChoice {
    fn options(&self) -> Vec<&str> {
        match self {
            ClipChoice::One(c) => vec![c.as_str()],
            ClipChoice::Many(l) => l.iter().map(String::as_str).collect(),
        }
    }
}

/// A servant of an engine: post (local to the engine, m), heading, role, figure.
#[derive(Debug, Clone, Deserialize)]
pub struct Servant {
    pub x: f32,
    pub z: f32,
    pub yaw_deg: f32,
    pub role: String,
    #[serde(default)]
    pub figure: u32,
    /// Resting place `[x, z, yaw_deg]` out of the muzzle blast (bombard).
    #[serde(default)]
    pub rest: Option<[f32; 3]>,
}

/// Settings `crew` of `siege_engines.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct CrewSettings {
    pub figure_kind: String,
    pub detail_m: f32,
    pub medium_m: f32,
    pub hide_beyond_m: f32,
    pub load_from: f32,
    pub swab_until: f32,
    pub bombard_load: [f32; 2],
    pub clips: HashMap<String, ClipChoice>,
    #[serde(default)]
    pub clips_by_engine: HashMap<String, HashMap<String, ClipChoice>>,
    pub layouts: HashMap<String, Vec<Servant>>,
}

impl CrewSettings {
    /// Clip of a role (`NT7`: `clips_by_engine` overrides it per engine; a list alternates by
    /// `index`; a clip the rig lacks falls back on the role's default list, then on `idle`).
    pub fn clip_of(
        &self,
        role: &str,
        engine: &str,
        index: usize,
        rig_has: impl Fn(&str) -> bool,
    ) -> String {
        let default = self.clips.get(role);
        let chosen = self
            .clips_by_engine
            .get(engine)
            .and_then(|m| m.get(role))
            .or(default);
        let Some(chosen) = chosen else {
            return role.to_owned();
        };
        let options = chosen.options();
        let clip = options[index % options.len().max(1)];
        if rig_has(clip) {
            return clip.to_owned();
        }
        let fallback = default.map(ClipChoice::options).unwrap_or_default();
        options
            .iter()
            .chain(fallback.iter())
            .find(|c| rig_has(c))
            .map_or_else(
                || fallback.first().copied().unwrap_or(role).to_owned(),
                |c| (*c).to_owned(),
            )
    }

    /// Every clip any role may ask for (the pick set of the batches).
    pub fn all_clips(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let by_engine = self.clips_by_engine.values().flat_map(HashMap::values);
        for choice in self.clips.values().chain(by_engine) {
            for clip in choice.options() {
                if !out.iter().any(|c| c == clip) {
                    out.push(clip.to_owned());
                }
            }
        }
        out.sort();
        out
    }
}

/// Settings of a rolling engine (`ram`, `siege_tower`); a GA3 variant overrides some keys.
#[derive(Debug, Clone, Deserialize)]
pub struct Mover {
    #[serde(default = "Mover::default_wheel")]
    pub wheel_radius: f32,
    #[serde(default = "Mover::default_beam_drop")]
    pub beam_drop: f32,
    #[serde(default = "Mover::default_max_swing")]
    pub max_swing_deg: f32,
    #[serde(default = "Mover::default_sway")]
    pub sway_deg: f32,
    #[serde(default = "Mover::default_sway_per_m")]
    pub sway_per_m: f32,
}

impl Default for Mover {
    fn default() -> Self {
        Self {
            wheel_radius: 0.6,
            beam_drop: 1.95,
            max_swing_deg: 48.0,
            sway_deg: 1.0,
            sway_per_m: 0.5,
        }
    }
}

impl Mover {
    fn default_wheel() -> f32 {
        0.6
    }
    fn default_beam_drop() -> f32 {
        1.95
    }
    fn default_max_swing() -> f32 {
        48.0
    }
    fn default_sway() -> f32 {
        1.0
    }
    fn default_sway_per_m() -> f32 {
        0.5
    }

    /// `kind_settings`: the settings of the variant `over` (a json object) replace the common ones.
    pub fn with_override(&self, over: &serde_json::Value) -> Self {
        let get = |key: &str, base: f32| over[key].as_f64().map_or(base, |v| v as f32);
        Self {
            wheel_radius: get("wheel_radius", self.wheel_radius),
            beam_drop: get("beam_drop", self.beam_drop),
            max_swing_deg: get("max_swing_deg", self.max_swing_deg),
            sway_deg: get("sway_deg", self.sway_deg),
            sway_per_m: get("sway_per_m", self.sway_per_m),
        }
    }

    /// Angle (rad) of the ram's beam pendulum for a head pulled back by `offset` m (<= 0).
    pub fn pivot_angle(&self, offset: f32) -> f32 {
        let limit = self.max_swing_deg.to_radians();
        (-offset / self.beam_drop.max(0.01))
            .clamp(-0.95, 0.95)
            .asin()
            .clamp(-limit, limit)
    }
}

/// Offset of the ram head along the beam (m, <= 0) at `phase` (0..1) of the blow period.
pub fn ram_offset(phase: f32) -> f32 {
    let swing = if phase < 0.75 {
        let t = (phase / 0.75).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    } else {
        let k = (phase - 0.75) / 0.25;
        1.0 - k * k
    };
    -1.5 * swing
}

/// Name of the generated variant of `model` (`ga3` of the settings) when `exists(variant)`;
/// `_lod` is followed. `None`: the base model.
pub fn ga3_variant(
    model: &str,
    ga3: &serde_json::Value,
    exists: impl Fn(&str) -> bool,
) -> Option<String> {
    let base = model.trim_end_matches("_lod");
    let name = ga3[base]["model"].as_str().filter(|s| !s.is_empty())?;
    let variant = if model.ends_with("_lod") {
        format!("{name}_lod")
    } else {
        name.to_owned()
    };
    exists(&variant).then_some(variant)
}

/// Every setting the engines need.
#[derive(Debug, Clone)]
pub struct EngineSettings {
    /// `ga3` entries (generated variants and their overrides), raw.
    pub ga3: serde_json::Value,
    pub ram: Mover,
    pub siege_tower: Mover,
    pub trebuchet: Trebuchet,
    pub mangonel: Mangonel,
    pub bombard: Bombard,
    pub crew: CrewSettings,
}

impl EngineSettings {
    pub fn load(data_dir: &Path) -> Option<Self> {
        let doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(data_dir.join("fx/siege_engines.json")).ok()?)
                .ok()?;
        let mut trebuchet: Trebuchet = serde_json::from_value(doc["trebuchet"].clone()).ok()?;
        trebuchet.lut = std::fs::read(data_dir.join("fx/trebuchet_swing_curve.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|v| {
                v["lut"].as_array().map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_f64().map(|x| x as f32))
                        .collect()
                })
            })
            .unwrap_or_default();
        Some(Self {
            ga3: doc["ga3"].clone(),
            ram: serde_json::from_value(doc["ram"].clone()).unwrap_or_default(),
            siege_tower: serde_json::from_value(doc["siege_tower"].clone()).unwrap_or_default(),
            trebuchet,
            mangonel: serde_json::from_value(doc["mangonel"].clone()).ok()?,
            bombard: serde_json::from_value(doc["bombard"].clone()).ok()?,
            crew: serde_json::from_value(doc["crew"].clone()).ok()?,
        })
    }
}

/// Pose of a trebuchet's parts (rotations about X, rad).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrebuchetPose {
    pub arm: f32,
    pub sling: f32,
    pub counterweight: f32,
    /// Winch rotation (only while winding).
    pub winch: Option<f32>,
    pub stone_visible: bool,
}

/// Share of the winch done (0: arm at rest after the shot, 1: cocked) from the core's reload.
pub fn wound(
    swing_s: f32,
    settle_s: f32,
    ready_margin_s: f32,
    reload_period: f32,
    reload: f32,
    fired: bool,
) -> f32 {
    if !fired {
        return 1.0;
    }
    let period = reload_period.max(0.1);
    let span = (period - swing_s - settle_s - ready_margin_s).max(0.5);
    (1.0 - (reload - ready_margin_s) / span).clamp(0.0, 1.0)
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Trebuchet {
    /// Share of the arc (0 cocked, 1 end of the overswing) at phase `u`: the measured curve when
    /// there is one, else the arm accelerating under the counterweight.
    pub fn swing_progress(&self, u: f32) -> f32 {
        if self.lut.len() >= 2 {
            let x = u.clamp(0.0, 1.0) * (self.lut.len() - 1) as f32;
            let i = (x as usize).min(self.lut.len() - 2);
            let f = x - i as f32;
            return self.lut[i] + (self.lut[i + 1] - self.lut[i]) * f;
        }
        u * u * (2.2 - 1.2 * u)
    }

    /// (arm, sling relative to the arm) in rad at phase `u` of the swing.
    pub fn swing(&self, u: f32) -> (f32, f32) {
        let theta =
            self.cocked_deg + (self.overswing_deg - self.cocked_deg) * self.swing_progress(u);
        let u_rel = self.release_phase;
        let phi = if u <= u_rel {
            self.sling_release_deg * (u / u_rel).powf(self.sling_power)
        } else {
            let k = (u - u_rel) / (1.0 - u_rel);
            self.sling_release_deg + (self.sling_end_deg - self.sling_release_deg) * k
        };
        (theta.to_radians(), (phi - theta).to_radians())
    }

    /// Time between the start of the swing and the release of the stone.
    pub fn lead(&self) -> f32 {
        self.swing_s * self.release_phase
    }

    /// Pose `tau` seconds after the swing started (negative: before), `wound_share` of the winch
    /// done (see [`wound`]).
    pub fn pose(&self, tau: f32, wound_share: f32) -> TrebuchetPose {
        let rest = self.rest_deg.to_radians();
        let (theta, rel, omega, loaded, winch);
        if tau >= 0.0 && tau < self.swing_s {
            let u = tau / self.swing_s;
            let (arm, sling) = self.swing(u);
            let (next, _) = self.swing((u + 0.02).min(1.0));
            theta = arm;
            rel = sling;
            omega = (next - arm) / (0.02 * self.swing_s);
            loaded = u < self.release_phase;
            winch = None;
        } else if tau >= self.swing_s && tau < self.swing_s + self.settle_s {
            // Damped oscillation about the vertical; the empty sling hangs and sways.
            let t = tau - self.swing_s;
            let over = self.overswing_deg.to_radians();
            let w = TAU * self.settle_hz;
            let decay = (-self.settle_damping * t).exp();
            theta = rest + (over - rest) * (w * t).cos() * decay;
            omega = -(over - rest) * w * (w * t).sin() * decay;
            let hang = (90.0 + 25.0 * (w * t * 1.3).sin() * decay).to_radians();
            rel = hang - theta;
            loaded = false;
            winch = None;
        } else {
            let k = smoothstep(wound_share);
            theta = rest + (self.cocked_deg.to_radians() - rest) * k;
            // The sling hangs while the arm is high, lies in the trough once cocked.
            rel = (90.0 * (1.0 - k)).to_radians() - theta;
            loaded = wound_share >= 0.98;
            omega = 0.0;
            winch = Some(-wound_share * self.winch_turns * TAU);
        }
        TrebuchetPose {
            arm: theta,
            sling: rel,
            counterweight: -theta - (omega * self.counterweight_lag).clamp(-0.7, 0.7),
            winch,
            stone_visible: loaded,
        }
    }
}

/// Pose of a mangonel (arm rad, winch rad, stone visible).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MangonelPose {
    pub arm: f32,
    pub winch: Option<f32>,
    pub stone_visible: bool,
}

impl Mangonel {
    pub fn lead(&self) -> f32 {
        self.swing_s * self.release_phase
    }

    pub fn pose(&self, tau: f32, wound_share: f32) -> MangonelPose {
        let (angle, loaded, winch);
        if tau >= 0.0 && tau < self.swing_s {
            let u = tau / self.swing_s;
            angle = self.cocked_deg + (self.fired_deg - self.cocked_deg) * u * u;
            loaded = u < self.release_phase;
            winch = None;
        } else if tau >= self.swing_s && tau < self.swing_s + self.settle_s {
            let t = tau - self.swing_s;
            angle = self.fired_deg - self.bounce_deg * (t * 14.0).sin().abs() * (-4.0 * t).exp();
            loaded = false;
            winch = None;
        } else {
            angle = self.fired_deg + (self.cocked_deg - self.fired_deg) * smoothstep(wound_share);
            loaded = wound_share >= 0.98;
            winch = Some(-wound_share * self.winch_turns * TAU);
        }
        MangonelPose {
            arm: angle.to_radians(),
            winch,
            stone_visible: loaded,
        }
    }
}

impl Bombard {
    /// Share of the recoil at `tau` seconds after the shot (0 at rest, 1 at the end of the kick).
    pub fn recoil_share(&self, tau: f32, fired: bool) -> f32 {
        if tau < 0.0 || !fired {
            return 0.0;
        }
        if tau < self.recoil_s {
            tau / self.recoil_s
        } else if tau < self.recoil_s + self.hold_s {
            1.0
        } else {
            1.0 - smoothstep((tau - self.recoil_s - self.hold_s) / self.return_s)
        }
    }

    /// Mantlet angle (rad) for a raised share 0..1.
    pub fn mantlet_angle(&self, raised: f32) -> f32 {
        self.mantlet_raised_deg.to_radians() * smoothstep(raised)
    }
}

/// Gesture of a servant (role) of an engine, from the reload state (the same rules as
/// `_crew_engine`). `tau`: time since the swing / the shot; `phase`: 0 just fired, 1 ready.
#[allow(clippy::too_many_arguments)]
pub fn servant_act(
    crew: &CrewSettings,
    engine: &str,
    role: &str,
    reloading: bool,
    settled: bool,
    wound_share: f32,
    tau: f32,
    phase: f32,
    bombard: &Bombard,
) -> &'static str {
    match role {
        "winch" if reloading && settled && wound_share < 0.98 => "winch",
        "loader" => {
            if engine == "bombard" {
                let [from, to] = crew.bombard_load;
                if reloading && phase >= from && phase < to {
                    "loader"
                } else {
                    "idle"
                }
            } else if reloading && settled && wound_share >= crew.load_from && wound_share < 0.98 {
                "loader"
            } else {
                "idle"
            }
        }
        "swab"
            if reloading && tau > bombard.recoil_s + bombard.hold_s && phase < crew.swab_until =>
        {
            "swab"
        }
        _ => "idle",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ga3_variant_follows_lod_and_existence() {
        let ga3 = serde_json::json!({"ram": {"model": "ga3_ram"}, "trebuchet": {"model": "ga3_trebuchet"}});
        let all = |_: &str| true;
        assert_eq!(
            super::ga3_variant("ram", &ga3, all).as_deref(),
            Some("ga3_ram")
        );
        assert_eq!(
            super::ga3_variant("ram_lod", &ga3, all).as_deref(),
            Some("ga3_ram_lod")
        );
        assert_eq!(super::ga3_variant("mangonel", &ga3, all), None);
        assert_eq!(super::ga3_variant("ram", &ga3, |_| false), None);
    }

    #[test]
    fn ram_pendulum_is_bounded() {
        let ram = super::Mover::default().with_override(&serde_json::json!({"beam_drop": 2.2}));
        assert_eq!(ram.beam_drop, 2.2);
        assert_eq!(super::ram_offset(0.0), 0.0);
        assert!((super::ram_offset(0.75) + 1.5).abs() < 1e-5);
        let angle = ram.pivot_angle(-1.5);
        assert!(angle > 0.0 && angle <= 48f32.to_radians());
    }

    use super::*;

    fn settings() -> EngineSettings {
        let root = crate::paths::find_root().expect("root");
        EngineSettings::load(&crate::paths::data_dir(&root)).expect("settings")
    }

    #[test]
    fn the_real_settings_load_with_the_measured_curve() {
        let s = settings();
        assert!(s.trebuchet.lut.len() > 10);
        assert!(s.crew.layouts["bombard"].iter().any(|v| v.rest.is_some()));
        assert!(s.crew.all_clips().contains(&"crank".to_owned()));
        assert!(s.trebuchet.predict);
    }

    #[test]
    fn the_trebuchet_swings_from_cocked_to_overswing_and_releases_at_the_phase() {
        let t = settings().trebuchet;
        let (arm0, _) = t.swing(0.0);
        let (arm1, _) = t.swing(1.0);
        assert!((arm0 - t.cocked_deg.to_radians()).abs() < 1e-4);
        assert!((arm1 - t.overswing_deg.to_radians()).abs() < 0.05);
        // Mid-swing the arm has gone up and the stone is still in the sling until the release.
        let before = t.pose(t.swing_s * 0.3, 0.0);
        assert!(before.stone_visible);
        assert!(before.arm > arm0);
        let after = t.pose(t.swing_s * 0.9, 0.0);
        assert!(!after.stone_visible);
        assert!(after.winch.is_none());
        // Past the swing and the settling, the winch brings it back and the stone loads.
        let rewound = t.pose(t.swing_s + t.settle_s + 1.0, 1.0);
        assert!((rewound.arm - t.cocked_deg.to_radians()).abs() < 1e-4);
        assert!(rewound.stone_visible && rewound.winch.is_some());
        let lowered = t.pose(t.swing_s + t.settle_s + 1.0, 0.0);
        assert!((lowered.arm - t.rest_deg.to_radians()).abs() < 1e-4);
        assert!(!lowered.stone_visible);
    }

    #[test]
    fn the_winch_follows_the_reload_of_the_core() {
        // 12 s reload, swing 1.6 + settle 3.2 + margin 1: the winch spans 6.2 s.
        assert_eq!(wound(1.6, 3.2, 1.0, 12.0, 12.0, true), 0.0);
        assert!((wound(1.6, 3.2, 1.0, 12.0, 1.0, true) - 1.0).abs() < 1e-6);
        let mid = wound(1.6, 3.2, 1.0, 12.0, 7.0, true);
        assert!(mid > 0.0 && mid < 1.0);
        assert_eq!(wound(1.6, 3.2, 1.0, 12.0, 12.0, false), 1.0);
    }

    #[test]
    fn the_mangonel_slaps_and_rewinds() {
        let m = settings().mangonel;
        let start = m.pose(0.0, 0.0);
        assert!((start.arm - m.cocked_deg.to_radians()).abs() < 1e-5);
        let end = m.pose(m.swing_s * 0.999, 0.0);
        assert!((end.arm - m.fired_deg.to_radians()).abs() < 0.05);
        let rewound = m.pose(m.swing_s + m.settle_s + 1.0, 1.0);
        assert!(rewound.stone_visible);
    }

    #[test]
    fn the_bombard_recoils_holds_and_returns() {
        let b = settings().bombard;
        assert_eq!(b.recoil_share(-1.0, true), 0.0);
        assert_eq!(b.recoil_share(0.5, false), 0.0);
        assert!((b.recoil_share(b.recoil_s * 0.5, true) - 0.5).abs() < 1e-5);
        assert_eq!(b.recoil_share(b.recoil_s + b.hold_s * 0.5, true), 1.0);
        assert!(b.recoil_share(b.recoil_s + b.hold_s + b.return_s * 0.5, true) < 1.0);
        assert!(b.recoil_share(b.recoil_s + b.hold_s + b.return_s + 1.0, true) < 1e-5);
        assert!(b.mantlet_angle(1.0) < 0.0);
        assert_eq!(b.mantlet_angle(0.0), 0.0);
    }

    #[test]
    fn servants_work_in_the_reload_window_of_their_role() {
        let s = settings();
        let act = |engine: &str, role: &str, wound: f32, tau: f32, phase: f32| {
            servant_act(
                &s.crew, engine, role, true, true, wound, tau, phase, &s.bombard,
            )
        };
        assert_eq!(act("trebuchet", "winch", 0.3, 5.0, 0.4), "winch");
        assert_eq!(act("trebuchet", "winch", 1.0, 5.0, 0.9), "idle");
        assert_eq!(act("trebuchet", "loader", 0.7, 5.0, 0.6), "loader");
        assert_eq!(act("trebuchet", "loader", 0.2, 5.0, 0.3), "idle");
        assert_eq!(act("bombard", "loader", 1.0, 5.0, 0.6), "loader");
        assert_eq!(act("bombard", "loader", 1.0, 5.0, 0.2), "idle");
        assert_eq!(act("bombard", "swab", 1.0, 5.0, 0.3), "swab");
        assert_eq!(act("bombard", "swab", 1.0, 0.5, 0.3), "idle");
    }

    #[test]
    fn a_missing_clip_falls_back_to_the_roles_default() {
        let s = settings();
        let rig = |c: &str| c != "load_heavy" && c != "push_shoulder";
        // The heavy loader of the trebuchet falls back on the plain `load` clip.
        assert_eq!(s.crew.clip_of("loader", "trebuchet", 0, rig), "load");
        assert_eq!(
            s.crew.clip_of("loader", "trebuchet", 0, |_| true),
            "load_heavy"
        );
        // A list alternates by servant index.
        assert_eq!(s.crew.clip_of("pusher", "ram", 0, |_| true), "push");
        assert_eq!(
            s.crew.clip_of("pusher", "ram", 1, |_| true),
            "push_shoulder"
        );
        assert_eq!(s.crew.clip_of("pusher", "ram", 1, rig), "push");
    }
}
