//! Campaign and battle AI.
//!
//! [`plan_turn`] is the strategic campaign planner of M9 (`campaign.rs`); the
//! Godot bridge resolves turns with it through `CampaignState::end_turn_with`.
//! The simulation's own tests keep using the minimal planner of M2
//! (`sim_campaign::ai_minimal`), which `CampaignState::end_turn` defaults to.

pub mod alignment;
pub mod campaign;
pub mod doctrine;
pub mod support;

pub use campaign::plan_turn;
pub use sim_campaign::ai_minimal::plan_turn as plan_turn_minimal;
