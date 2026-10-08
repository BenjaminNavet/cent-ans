//! Unit modes (lot CB2, spec
//! `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`,
//! « Modes d'unité », and `docs/research/cb4-capacites.md` § 6).
//!
//! Persistent toggles of a regiment, set by [`crate::Command::SetMode`]:
//! - **run** (R): every move is made at the run (the double right click
//!   still runs a single move);
//! - **guard** (G): no pursuit: the regiment drops a routing target, and
//!   once a melee breaks it holds instead of following its opponent;
//! - **skirmish** (K): shooters step back from enemy melee troops closing
//!   in; the shot on the move of the `skirmish` ability goes with the mode
//!   (regiments with the ability start with it on);
//! - **melee** (M): shooters ignore their missiles and close in;
//! - **breach** (siege engines, « Battre en brèche »): shots go at walls and
//!   gates only, harder and a little slower (the mangonel, a lighter engine,
//!   less so); no shot at men while the mode is on.
//!
//! Numbers in `data/rules/unit_modes.json`; the behaviour lives in
//! `sim/modes.rs`. The display states of the regiment (charging, under
//! fire, engaged, wavering) are computed here too ([`UnitStatus`]).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A persistent mode of a regiment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitMode {
    Run,
    Guard,
    Skirmish,
    Melee,
    Breach,
}

impl UnitMode {
    pub const ALL: [UnitMode; 5] = [
        UnitMode::Run,
        UnitMode::Guard,
        UnitMode::Skirmish,
        UnitMode::Melee,
        UnitMode::Breach,
    ];

    /// Key of the mode (JSON and bridge).
    pub fn key(self) -> &'static str {
        match self {
            UnitMode::Run => "run",
            UnitMode::Guard => "guard",
            UnitMode::Skirmish => "skirmish",
            UnitMode::Melee => "melee",
            UnitMode::Breach => "breach",
        }
    }

    /// French name of the mode (messages).
    pub fn label_fr(self) -> &'static str {
        match self {
            UnitMode::Run => "course",
            UnitMode::Guard => "garde",
            UnitMode::Skirmish => "escarmouche",
            UnitMode::Melee => "mêlée",
            UnitMode::Breach => "battre en brèche",
        }
    }
}

/// `run` block of `data/rules/unit_modes.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunModeRules {
    pub speed_multiplier: f64,
    pub fatigue_multiplier: f64,
}

/// `skirmish` block.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkirmishRules {
    pub trigger_m: f64,
    pub retreat_m: f64,
    pub min_retreat_m: f64,
    pub approach_cos: f64,
}

/// `status` block.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusRules {
    pub under_fire_seconds: f64,
    pub wavering_morale: f64,
}

/// `breach` block.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BreachRules {
    pub reload_multiplier: f64,
    /// `default`, then per unit type.
    pub wall_damage_multiplier: BTreeMap<String, f64>,
}

impl BreachRules {
    /// Wall damage multiplier of an engine of type `unit_type`.
    pub fn wall_damage(&self, unit_type: &str) -> f64 {
        self.wall_damage_multiplier
            .get(unit_type)
            .or_else(|| self.wall_damage_multiplier.get("default"))
            .copied()
            .unwrap_or(1.0)
    }
}

/// `ai` block.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModeAiRules {
    pub light_shooter_max_armor: u32,
}

/// `data/rules/unit_modes.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitModeRules {
    #[serde(default)]
    pub description: String,
    pub run: RunModeRules,
    pub skirmish: SkirmishRules,
    pub status: StatusRules,
    pub breach: BreachRules,
    pub ai: ModeAiRules,
}

data_model::bundled_rules!(UnitModeRules, "rules/unit_modes.json");

/// Display states of a regiment, computed by the core (badges over the
/// banners and on the cards).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct UnitStatus {
    /// Charging home.
    pub charging: bool,
    /// Took missile casualties within `status.under_fire_seconds`.
    pub under_fire: bool,
    /// In contact with the enemy (melee).
    pub engaged: bool,
    /// Morale under `status.wavering_morale`, not routing yet.
    pub wavering: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_parse() {
        let rules = UnitModeRules::bundled();
        assert!(
            rules.breach.wall_damage("unit_mangonel") < rules.breach.wall_damage("unit_trebuchet")
        );
        assert_eq!(
            rules.breach.wall_damage("unit_bombard"),
            rules.breach.wall_damage_multiplier["default"]
        );
    }

    #[test]
    fn modes_serialise_in_snake_case() {
        for mode in UnitMode::ALL {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, format!("\"{}\"", mode.key()));
        }
    }
}
