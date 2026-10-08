//! `CampaignSim` end of turn off the main thread (lot PB3d, ADR 0081).
//!
//! `begin_end_turn()` hands a clone of the state to a worker thread;
//! `poll_end_turn()` returns `null` while it runs, then installs the resolved
//! state and returns the turn's events (the format of `end_turn()`). Until
//! then every getter reads the state of before, and every method that would
//! change the state is refused (see [`CampaignSim::refuse_while_turn_pending`]):
//! the resolved state replaces it wholesale.

use std::sync::Arc;

use godot::prelude::*;

use crate::campaign_sim::{events_array, CampaignSim, Ctx};
use crate::turn_job::TurnJob;

/// French message of an order refused during the end of turn.
pub(crate) const TURN_PENDING_FR: &str = "la fin de tour est en cours";

impl CampaignSim {
    /// Called first by every method that may change the state. True (with
    /// a warning naming `method`) while an end of turn runs on its worker
    /// thread: the caller must then leave the state untouched. Otherwise the
    /// state revision is bumped (see `get_state_revision`).
    pub(crate) fn refuse_while_turn_pending(&mut self, method: &str) -> bool {
        if self.pending_turn.is_some() {
            godot_warn!("CampaignSim.{method}: refused, the end of turn is running");
            return true;
        }
        self.revision += 1;
        false
    }

    /// Drops a running end of turn (new campaign, load): its worker ends on
    /// its own and its result is never installed.
    pub(crate) fn cancel_pending_turn(&mut self) {
        self.pending_turn = None;
        self.revision += 1;
    }

    /// Waits for the running end of turn, installs its state and returns
    /// its events (`None` when no end of turn is running).
    pub(crate) fn finish_pending_turn(&mut self) -> Option<VarArray> {
        let job = self.pending_turn.take()?;
        self.revision += 1;
        match job.join() {
            Ok((state, events)) => {
                self.state = Some(state);
                Some(events_array(&events))
            }
            Err(message) => {
                // The state of before stays: the turn is simply not played.
                godot_error!("CampaignSim: the end of turn failed: {message}");
                Some(VarArray::new())
            }
        }
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Starts resolving the turn on a worker thread. False before
    /// `new_campaign`, while another end of turn runs, or if the thread
    /// cannot start (the caller then falls back to `end_turn`).
    #[func]
    fn begin_end_turn(&mut self) -> bool {
        if self.pending_turn.is_some() {
            godot_warn!("CampaignSim.begin_end_turn: an end of turn is already running");
            return false;
        }
        let Some(Ctx { state, data }) = self.ctx() else {
            godot_warn!("CampaignSim.begin_end_turn called before new_campaign");
            return false;
        };
        match TurnJob::spawn(state.clone(), Arc::clone(data)) {
            Ok(job) => {
                self.pending_turn = Some(job);
                true
            }
            Err(error) => {
                godot_error!("CampaignSim.begin_end_turn: cannot start the thread: {error}");
                false
            }
        }
    }

    /// `null` while the end of turn runs; once done, installs the new state
    /// and returns the turn's events (an `Array`, as `end_turn`). `null` too
    /// when no end of turn was started.
    #[func]
    fn poll_end_turn(&mut self) -> Variant {
        if self
            .pending_turn
            .as_ref()
            .is_none_or(|job| !job.is_finished())
        {
            return Variant::nil();
        }
        self.finish_pending_turn()
            .map_or_else(Variant::nil, |events| events.to_variant())
    }

    /// True between `begin_end_turn` and the `poll_end_turn` that installs
    /// the new state.
    #[func]
    fn is_end_turn_pending(&self) -> bool {
        self.pending_turn.is_some()
    }

    /// Counter bumped by every call that may change the state (orders,
    /// end of turn, load…): the map caches its grouped reads by it.
    #[func]
    fn get_state_revision(&self) -> i64 {
        self.revision as i64
    }
}
