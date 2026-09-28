//! Queued orders (lot CB-M3, spec
//! `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`,
//! « Ordres en file »).
//!
//! Shift + right click adds a move or an attack after a regiment's current
//! order instead of replacing it (`queue: true` on [`crate::Command::Move`]
//! and [`crate::Command::Attack`]). The regiment keeps at most
//! [`QueueRules::max_queued_orders`] orders waiting
//! (`data/rules/battle_queue.json`); the next one starts when the current
//! order ends (destination reached, target gone, dead or fleeing). Any order
//! given without `queue`, a halt, a withdrawal or a rout empties the queue.
//! The behaviour lives in `sim/queue.rs`.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// An order waiting behind the regiment's current one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueuedOrder {
    /// Walk to the regiment's own place `(x, z)` (group spread already
    /// applied and clamped into the field when the order was given).
    Move {
        x: f64,
        z: f64,
        #[serde(default)]
        facing: Option<f64>,
        #[serde(default)]
        run: bool,
        /// CB1: frontage of this regiment on arrival (its share of the
        /// drag), applied when the order starts.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
        #[serde(default, skip_serializing_if = "is_false")]
        match_speed: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group_tag: Option<u32>,
    },
    /// Close with (or shoot at) regiment `target`.
    Attack { target: u32, run: bool },
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// `data/rules/battle_queue.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueueRules {
    #[serde(default)]
    pub description: String,
    /// Orders waiting at most per regiment (the current one not counted).
    pub max_queued_orders: u32,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_queue.json");

impl QueueRules {
    /// `data/rules/battle_queue.json` as compiled into the crate.
    pub fn bundled() -> &'static QueueRules {
        static RULES: OnceLock<QueueRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_queue.json is valid")
        })
    }
}
