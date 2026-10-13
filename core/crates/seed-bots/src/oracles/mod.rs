//! Oracles: what counts as a bug in a bot's game.

use data_model::{FactionId, GameData};
use serde::{Deserialize, Serialize};
use sim_campaign::{CampaignState, Order};

/// Which oracle raised a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleKind {
    /// A Rust panic during a turn.
    Panic,
    /// A turn slower than the limit.
    SlowTurn,
    /// save → load → save is not identical.
    SaveRoundtrip,
    /// A rule invariant is broken.
    Invariant,
    /// An order the queries declared possible was refused.
    RefusedLegalOrder,
    /// A balance metric outside its band.
    BalanceDrift,
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Balance drift: worth a look.
    Warning,
    /// A plain bug.
    Blocking,
}

/// One finding, with everything needed to replay it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// Oracle that raised it.
    pub oracle: OracleKind,
    /// Severity.
    pub severity: Severity,
    /// Game seed.
    pub seed: u64,
    /// Bot name.
    pub bot: String,
    /// Faction played.
    pub faction: String,
    /// Turn where it was found.
    pub turn: u32,
    /// Message in French, for the report.
    pub message: String,
    /// Player orders submitted since the start, per turn.
    pub order_log: Vec<Vec<Order>>,
}

/// What an oracle may look at after a step of the game.
pub struct OracleContext<'a> {
    /// State after the step.
    pub state: &'a CampaignState,
    /// Game data.
    pub data: &'a GameData,
    /// Faction played by the bot.
    pub faction: &'a FactionId,
    /// Current turn.
    pub turn: u32,
}
