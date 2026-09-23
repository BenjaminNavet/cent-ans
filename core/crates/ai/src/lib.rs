//! Campaign and battle AI.
//!
//! M2 ships the minimal planner of spec § 4: `plan_turn(state, data, faction)`
//! is a pure function returning the faction's orders for the turn. Because
//! `sim-campaign::end_turn` must call it and this crate depends on
//! `sim-campaign`, the implementation lives in `sim_campaign::ai_minimal` and
//! is re-exported here; richer strategic AI (M9) will plug into
//! `CampaignState::end_turn_with`.

pub use sim_campaign::ai_minimal::plan_turn;
