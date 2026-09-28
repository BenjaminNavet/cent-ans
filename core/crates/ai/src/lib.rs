//! Campaign and battle AI.
//!
//! [`plan_turn`] is the strategic campaign planner of M9 (`campaign.rs`),
//! executed on the navigation grid by `grid.rs` (lot M3); the
//! Godot bridge resolves turns with it through `CampaignState::end_turn_with`.
//! The simulation's own tests keep using the minimal planner of M2
//! (`sim_campaign::ai_minimal`), which `CampaignState::end_turn` defaults to.

pub mod alignment;
pub mod campaign;
pub mod diplomacy_eval;
pub mod doctrine;
pub mod feudal;
pub mod grid;
pub mod parallel;
pub mod stances;
pub mod support;

pub use campaign::{plan_turn, plan_turn_sequential};
pub use sim_campaign::ai_minimal::plan_turn as plan_turn_minimal;
