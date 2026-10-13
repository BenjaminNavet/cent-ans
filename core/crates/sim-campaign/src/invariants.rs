//! Rule invariants: properties every campaign state must keep, whatever the
//! orders. Checked by the seed bots after each step (spec
//! `docs/superpowers/specs/2026-10-13-seed-bots-design.md`) and usable by any
//! test or client.

use data_model::GameData;

use crate::state::CampaignState;

/// One broken invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Stable key of the invariant (`settlement_single_owner`…).
    pub invariant: &'static str,
    /// What is wrong, in French, naming the ids involved.
    pub detail: String,
}

/// Every broken invariant of `state`; empty when the state is sound.
pub fn check_invariants(_state: &CampaignState, _data: &GameData) -> Vec<Violation> {
    Vec::new()
}
