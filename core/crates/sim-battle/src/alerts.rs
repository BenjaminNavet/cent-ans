//! Typed battle alerts (CB5, ADR 0095). A small, parallel feed to the free
//! text journal ([`crate::outcome::BattleEvent`]): the same moments get a
//! `kind`, a place and (usually) a unit, so the interface can draw a glyph,
//! a short caption, a minimap ping and a battle cry instead of parsing
//! French text.
//!
//! Emitted at the same points as the matching `log()` call (déroute,
//! général tué ou capturé, renforts, munitions épuisées, mur ou porte
//! rompus), plus one point the journal does not cover: the rising edge of
//! [`crate::unit::Unit::flanked`] (flanc ou dos attaqué).
//!
//! **Output only.** [`BattleAlert`] is read by [`crate::sim::BattleSim::take_new_alerts`]
//! exactly like the journal, and never feeds back into the simulated state:
//! it carries no rule, and consuming it is not part of
//! [`crate::replay::state_digest`] (see `sim-battle/tests/cb5_alerts.rs`,
//! `determinism_and_digest_are_unaffected`).
//!
//! Importance, on-screen duration and the merge window used by `game/` are
//! data, not code: `data/rules/battle_alerts.json`
//! (`data/schemas/battle_alerts_rules.schema.json`), loaded the way
//! [`crate::rout::RoutRules`] loads `battle_rout.json`.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::setup::SideId;

/// What happened. Mirrors, without replacing, the journal's free text.
/// "General down" covers both killed and captured (spec: a general's wound
/// has no model yet, ADR 0095 point 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertKind {
    Rout,
    GeneralDown,
    Flanked,
    Reinforcements,
    AmmoOut,
    WallBreached,
    GateDestroyed,
}

impl AlertKind {
    /// Stable key used by the Godot bridge and `data/rules/battle_alerts.json`.
    pub fn key(self) -> &'static str {
        match self {
            AlertKind::Rout => "rout",
            AlertKind::GeneralDown => "general_down",
            AlertKind::Flanked => "flanked",
            AlertKind::Reinforcements => "reinforcements",
            AlertKind::AmmoOut => "ammo_out",
            AlertKind::WallBreached => "wall_breached",
            AlertKind::GateDestroyed => "gate_destroyed",
        }
    }
}

/// One typed battle alert (CB5). Read-only output: see the module doc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BattleAlert {
    pub kind: AlertKind,
    /// Simulated seconds since the start.
    pub time: f64,
    pub x: f64,
    pub z: f64,
    /// Side the alert concerns (`None`: a wall or gate piece, which belongs
    /// to neither side's regiments).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<SideId>,
    /// Campaign/battle unit id concerned (`None` for a wall or gate piece).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<u32>,
}

/// Contents of `data/rules/battle_alerts.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Seconds an alert stays in the column before it clears on its own.
    pub duration_s: f64,
    /// Alerts of the same kind, in the same zone, within this many seconds
    /// of each other are merged into one entry (`game/`).
    pub merge_window_s: f64,
    /// Alerts within this radius of each other count as "the same zone" for
    /// merging (metres).
    pub merge_radius_m: f64,
    /// Display order and glyph weight, highest first, one entry per
    /// [`AlertKind::key`]; a kind missing here sorts last.
    pub importance: std::collections::HashMap<String, u32>,
    /// At most this many alerts shown at once (`game/`); the rest wait.
    pub max_shown: u32,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_alerts.json");

impl AlertRules {
    /// `data/rules/battle_alerts.json` as compiled into the crate.
    pub fn bundled() -> &'static AlertRules {
        static RULES: OnceLock<AlertRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_alerts.json is valid")
        })
    }

    /// Importance of `kind` (0 if absent from the table).
    pub fn importance(&self, kind: AlertKind) -> u32 {
        self.importance.get(kind.key()).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load() {
        let rules = AlertRules::bundled();
        assert!(rules.duration_s > 0.0);
        assert!(rules.merge_window_s > 0.0);
    }

    #[test]
    fn every_kind_has_a_key() {
        for kind in [
            AlertKind::Rout,
            AlertKind::GeneralDown,
            AlertKind::Flanked,
            AlertKind::Reinforcements,
            AlertKind::AmmoOut,
            AlertKind::WallBreached,
            AlertKind::GateDestroyed,
        ] {
            assert!(!kind.key().is_empty());
        }
    }
}
