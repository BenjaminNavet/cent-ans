//! Plays the games: for each (seed, bot), plan → submit → oracles → end of turn → oracles.

use std::path::PathBuf;

use crate::oracles::Finding;

/// Settings of a run.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Seeds to play.
    pub seeds: Vec<u64>,
    /// Turns per game.
    pub turns: u32,
    /// Bot names (subset of [`crate::bots::ALL`]).
    pub bots: Vec<String>,
    /// Hard cap in minutes; past it the run stops with a partial report.
    pub max_minutes: u32,
    /// Slow-turn limit, milliseconds.
    pub slow_turn_ms: u64,
    /// Whether to check balance drift (long pass only).
    pub balance: bool,
    /// The `data/` directory.
    pub data_root: PathBuf,
}

/// Result of a run.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Every finding, before deduplication.
    pub findings: Vec<Finding>,
    /// Whether the hard cap stopped the run early.
    pub partial: bool,
    /// Games played.
    pub games: u32,
    /// Wall time, seconds.
    pub seconds: f64,
}

/// Plays every (seed, bot) game, one thread per bot.
pub fn run(_config: &RunConfig) -> RunResult {
    todo!("lot D")
}

/// Plays a finding's game again and stops at its turn; returns the findings seen then.
pub fn replay(_finding: &Finding, _data_root: &std::path::Path) -> Vec<Finding> {
    todo!("lot D")
}
