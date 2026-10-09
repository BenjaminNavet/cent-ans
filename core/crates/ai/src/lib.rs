//! Campaign and battle AI.
//!
//! [`plan_turn`] is the strategic campaign planner of M9 (`campaign.rs`),
//! executed on the navigation grid by `grid.rs`; the
//! Godot bridge resolves turns with it through `CampaignState::end_turn_with`.
//! The simulation's own tests keep using the minimal planner of M2
//! (`sim_campaign::ai_minimal`), which `CampaignState::end_turn` defaults to.
//!
//! Organisation: `campaign` (strategic planner, with `grid` and `spatial` for movement on the
//! navigation grid), `feudal`, `diplomacy_eval`, `alignment` and `support` (diplomacy and
//! historical side changes), `doctrine`, `mercenaries`, `traditions` and `stances` (recruitment,
//! army traditions, encounter stances), `parallel` (read-only work spread over cores) and
//! `salts` (hashed pseudo-random rolls).

pub mod alignment;
pub mod campaign;
pub mod diplomacy_eval;
pub mod doctrine;
pub mod feudal;
pub mod grid;
mod mercenaries;
pub mod parallel;
mod salts;
pub mod spatial;
pub mod stances;
pub mod support;
pub mod traditions;

pub use campaign::{plan_turn, plan_turn_sequential};
pub use sim_campaign::ai_minimal::plan_turn as plan_turn_minimal;
