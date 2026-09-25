//! How a field battle ends (lot EP9, ADR 0056): the broken army and the
//! refused battle (`data/rules/battle_decision.json`, schema
//! `data/schemas/battle_decision_rules.schema.json`).
//!
//! - **Broken army**: a side whose fighting strength (soldiers of the
//!   regiments present and in good order, plus the reserves still to come)
//!   falls below [`DecisionRules::break_share`] of its initial soldiers
//!   breaks: its remaining regiments rout and the battle ends. Without its
//!   general (killed or taken) the army lets go sooner
//!   ([`DecisionRules::break_share_without_general`]).
//! - **Refused battle**: when nobody engages for
//!   [`DecisionRules::refusal_seconds`] (no melee, losses under
//!   [`DecisionRules::engagement_loss_share`], and the armies no longer
//!   draw closer by [`DecisionRules::approach_meters`]), the attacker
//!   gives up and withdraws, the defender keeps the field (Buironfosse,
//!   1339). After a melee, a shorter lull ([`DecisionRules::lull_seconds`])
//!   is enough: the side that suffered less keeps the field (the defender unless the attacker lost clearly
//!   less, [`DecisionRules::lull_loss_margin`]).
//!
//! Siege battles keep their own end (the square held, the garrison beaten).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Campaign morale after a refused battle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefusedMorale {
    pub attacker: i32,
    pub defender: i32,
}

/// Contents of `data/rules/battle_decision.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Share of its initial soldiers below which an army breaks.
    pub break_share: f64,
    /// The same once its general is killed or taken.
    pub break_share_without_general: f64,
    /// Seconds without engagement before the battle ends.
    pub refusal_seconds: f64,
    /// The same once a melee has taken place (the fight died down).
    pub lull_seconds: f64,
    /// Window over which an approach is measured.
    pub approach_window_seconds: f64,
    /// Drawing this much closer to the enemy restarts the clock.
    pub approach_meters: f64,
    /// Missiles restart the clock once either side has lost this share of
    /// its initial soldiers since the last engagement (a few stray arrows
    /// across a river do not keep a battle going).
    pub engagement_loss_share: f64,
    /// After a lull, the attacker keeps the field only when its share of
    /// losses is lower than the defender's by this margin.
    pub lull_loss_margin: f64,
    pub refused_morale: RefusedMorale,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_decision.json");

impl DecisionRules {
    /// `data/rules/battle_decision.json` as compiled into the crate.
    pub fn bundled() -> &'static DecisionRules {
        static RULES: OnceLock<DecisionRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_decision.json is valid")
        })
    }
}

/// How a battle ended (result screen, campaign chronicle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleEnd {
    /// Every regiment of the loser fled, fell or left the field.
    #[default]
    Rout,
    /// The loser's army broke (below its break share).
    Broken,
    /// Nobody engaged: the attacker gave up, the defender keeps the field.
    Refused,
    /// The fight died down: the side that suffered less keeps the field.
    Lull,
    /// Nightfall (the hour limit).
    Nightfall,
    /// Siege: the attacker holds the central square.
    SquareHeld,
}

impl BattleEnd {
    pub fn key(self) -> &'static str {
        match self {
            BattleEnd::Rout => "rout",
            BattleEnd::Broken => "broken",
            BattleEnd::Refused => "refused",
            BattleEnd::Lull => "lull",
            BattleEnd::Nightfall => "nightfall",
            BattleEnd::SquareHeld => "square_held",
        }
    }
}

/// Engagement clock of a field battle (derived from the state each step).
#[derive(Debug, Clone, Default)]
pub(crate) struct EngagementClock {
    /// Time of the last loss, melee or approach.
    pub last: f64,
    /// Gap between the armies at the start of the approach window, and
    /// when that window began.
    pub gap_mark: Option<(f64, f64)>,
    /// A loss or a melee happened at some point.
    pub fought: bool,
    /// A melee happened at some point.
    pub melee_seen: bool,
    /// Loss share of each side when the clock last restarted.
    pub losses_mark: [f64; 2],
}
