//! A6-L13 (ADR 0180): pace of the fighting. Rules in
//! `data/rules/battle_pace.json` (schema
//! `data/schemas/battle_pace_rules.schema.json`): lethality of the melee and
//! of the missiles, morale lost per man killed. Field battles and sieges have
//! their own values (the sieges keep their original pace).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/battle_pace.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaceRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub field: Pace,
    pub siege: Pace,
}

/// One set of fighting rates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pace {
    /// Deaths per second per melee fighter, before armour, attack value and cover.
    pub melee_rate: f64,
    /// Lethality factor of a volley (and of tower shots).
    pub ranged_rate: f64,
    /// Morale lost when a regiment loses all its men at once.
    pub loss_morale_factor: f64,
    /// Morale lost per second by a regiment struck on the flank.
    pub flank_morale_per_s: f64,
    /// Morale lost per second by a regiment struck in the rear.
    pub rear_morale_per_s: f64,
    /// A regiment whose morale falls under this routs.
    pub rout_morale: f64,
    /// A routing regiment out of danger whose morale rises above this rallies.
    pub rally_morale: f64,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_pace.json");

impl PaceRules {
    /// `data/rules/battle_pace.json` as compiled into the crate.
    pub fn bundled() -> &'static PaceRules {
        static RULES: OnceLock<PaceRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_pace.json is valid")
        })
    }
}
