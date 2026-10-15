//! Soot of each town on the campaign map (0-1), port of `town_soot.gd` (ADR 0162, point 5). It is
//! derived from what the engine already exposes, no new rule:
//! - devastation of the province (`soot.devastation_*`);
//! - siege in progress of the place (`soot.siege`, cities only);
//! - capture of the place: its controller changes between two reads. Sack or razing when the
//!   devastation of the province rises by `soot.sack_devastation_rise` at the same time, else a
//!   storm when the place was besieged. This soot decays by `soot.decay_per_turn` per turn.
//!
//! The engine keeps no "sacked town" state: the soot of a capture lives in this session (it does
//! not survive a reload; the devastation one does). Config: `data/map/building_models.json` →
//! `soot`.

use std::collections::HashMap;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct SootConfig {
    pub devastation_start: f32,
    pub devastation_full: f32,
    pub devastation_max: f32,
    pub siege: f32,
    pub storm: f32,
    pub sack: f32,
    pub sack_devastation_rise: f32,
    pub decay_per_turn: f32,
}

impl Default for SootConfig {
    fn default() -> Self {
        SootConfig {
            devastation_start: 20.0,
            devastation_full: 90.0,
            devastation_max: 0.6,
            siege: 0.25,
            storm: 0.45,
            sack: 0.9,
            sack_devastation_rise: 15.0,
            decay_per_turn: 0.1,
        }
    }
}

impl SootConfig {
    pub fn from_json(v: &Value) -> Self {
        let d = SootConfig::default();
        let f = |k: &str, def: f32| v[k].as_f64().map_or(def, |x| x as f32);
        SootConfig {
            devastation_start: f("devastation_start", d.devastation_start),
            devastation_full: f("devastation_full", d.devastation_full),
            devastation_max: f("devastation_max", d.devastation_max),
            siege: f("siege", d.siege),
            storm: f("storm", d.storm),
            sack: f("sack", d.sack),
            sack_devastation_rise: f("sack_devastation_rise", d.sack_devastation_rise),
            decay_per_turn: f("decay_per_turn", d.decay_per_turn),
        }
    }

    /// `building_models.json` → `soot` (defaults when missing).
    pub fn load(data_dir: &std::path::Path) -> Self {
        std::fs::read_to_string(data_dir.join("map/building_models.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|v| Self::from_json(&v["soot"]))
            .unwrap_or_default()
    }

    /// Soot due to the devastation of the province alone.
    pub fn from_devastation(&self, devastation: f32) -> f32 {
        let start = self.devastation_start;
        let full = self.devastation_full.max(start + 1.0);
        let t = ((devastation - start) / (full - start)).clamp(0.0, 1.0);
        self.devastation_max * t * t * (3.0 - 2.0 * t)
    }
}

/// One settlement as read from the simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct SootInput {
    pub id: String,
    pub province: String,
    pub controller: String,
    pub kind: String,
    /// Devastation of the province (0-100).
    pub devastation: f32,
    /// The place is under siege.
    pub besieged: bool,
}

#[derive(Debug, Default)]
pub struct TownSoot {
    pub config: SootConfig,
    controller: HashMap<String, String>,
    devastation: HashMap<String, f32>,
    besieged: HashMap<String, bool>,
    /// id -> (soot of the capture, turn)
    events: HashMap<String, (f32, i64)>,
    amounts: HashMap<String, f32>,
    seen: bool,
    turn: i64,
}

impl TownSoot {
    pub fn new(config: SootConfig) -> Self {
        TownSoot {
            config,
            turn: -1,
            ..Default::default()
        }
    }

    /// Reads the state; returns the soots that changed (id -> soot).
    pub fn update(&mut self, settlements: &[SootInput], turn: i64) -> HashMap<String, f32> {
        let mut changed = HashMap::new();
        let decay = self.config.decay_per_turn;
        // Loaded game or new campaign (the turn jumps or goes back): the controllers read are not
        // captures, start again from this state.
        if self.turn >= 0 && (turn < self.turn || turn > self.turn + 1) {
            self.seen = false;
            self.events.clear();
        }
        self.turn = turn;
        let mut provinces: HashMap<&str, (f32, bool)> = HashMap::new();
        for s in settlements {
            let p = provinces
                .entry(&s.province)
                .or_insert((s.devastation, false));
            p.1 |= s.besieged && s.kind == "city";
            if self.seen
                && self
                    .controller
                    .get(&s.id)
                    .is_some_and(|c| *c != s.controller)
            {
                let rise = s.devastation
                    - self
                        .devastation
                        .get(&s.province)
                        .copied()
                        .unwrap_or(s.devastation);
                let besieged_before = self.besieged.get(&s.province).copied().unwrap_or(false);
                let event = if rise >= self.config.sack_devastation_rise {
                    self.config.sack
                } else if besieged_before {
                    self.config.storm
                } else {
                    0.0
                };
                if event > 0.0 {
                    self.events.insert(s.id.clone(), (event, turn));
                }
            }
            self.controller.insert(s.id.clone(), s.controller.clone());
            let mut amount = self.config.from_devastation(s.devastation);
            if s.besieged && s.kind == "city" {
                amount = amount.max(self.config.siege);
            }
            if let Some(&(event, at)) = self.events.get(&s.id) {
                let left = event - decay * (turn - at).max(0) as f32;
                if left <= 0.0 {
                    self.events.remove(&s.id);
                } else {
                    amount = amount.max(left);
                }
            }
            let amount = ((amount.clamp(0.0, 1.0) / 0.02).round() * 0.02).clamp(0.0, 1.0);
            let old = self.amounts.get(&s.id).copied().unwrap_or(0.0);
            if (old - amount).abs() > 1e-4 {
                if amount > 0.0 {
                    self.amounts.insert(s.id.clone(), amount);
                } else {
                    self.amounts.remove(&s.id);
                }
                changed.insert(s.id.clone(), amount);
            }
        }
        for (province, (dev, besieged)) in provinces {
            self.devastation.insert(province.to_owned(), dev);
            self.besieged.insert(province.to_owned(), besieged);
        }
        self.seen = true;
        changed
    }

    pub fn amount_of(&self, id: &str) -> f32 {
        self.amounts.get(id).copied().unwrap_or(0.0)
    }

    /// Forced soot of a town (scripts, tests): treated as a capture at `turn`.
    pub fn force(&mut self, id: &str, amount: f32, turn: i64) {
        self.events.insert(id.to_owned(), (amount, turn.max(0)));
        self.amounts.insert(id.to_owned(), amount);
    }

    /// Towns with soot.
    pub fn sooty(&self) -> usize {
        self.amounts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(controller: &str, devastation: f32, besieged: bool) -> SootInput {
        SootInput {
            id: "set_a".into(),
            province: "prov_a".into(),
            controller: controller.into(),
            kind: "city".into(),
            devastation,
            besieged,
        }
    }

    #[test]
    fn devastation_and_siege() {
        let mut s = TownSoot::new(SootConfig::default());
        assert!(s.update(&[input("fra", 0.0, false)], 0).is_empty());
        let c = s.update(&[input("fra", 90.0, false)], 1);
        assert!((c["set_a"] - 0.6).abs() < 1e-4);
        let mut s = TownSoot::new(SootConfig::default());
        s.update(&[input("fra", 0.0, true)], 0);
        assert!((s.amount_of("set_a") - 0.26).abs() < 0.011);
    }

    #[test]
    fn sack_and_storm_decay() {
        let mut s = TownSoot::new(SootConfig::default());
        s.update(&[input("fra", 0.0, true)], 0);
        // Captured with a devastation rise of 30: sack.
        s.update(&[input("eng", 30.0, false)], 1);
        assert!((s.amount_of("set_a") - 0.9).abs() < 0.011);
        s.update(&[input("eng", 30.0, false)], 2);
        assert!((s.amount_of("set_a") - 0.8).abs() < 0.011);
        // Storm: besieged, captured without devastation.
        let mut s = TownSoot::new(SootConfig::default());
        s.update(&[input("fra", 0.0, true)], 0);
        s.update(&[input("eng", 0.0, false)], 1);
        assert!((s.amount_of("set_a") - 0.46).abs() < 0.011);
        // A load (turn jump) is not a capture.
        let mut s = TownSoot::new(SootConfig::default());
        s.update(&[input("fra", 0.0, true)], 0);
        s.update(&[input("eng", 0.0, false)], 9);
        assert_eq!(s.amount_of("set_a"), 0.0);
    }

    #[test]
    fn real_config_loads() {
        let root = crate::paths::find_root().expect("root");
        let c = SootConfig::load(&crate::paths::data_dir(&root));
        assert_eq!(c, SootConfig::default());
    }
}
