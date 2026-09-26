//! Lot PB3e (ADR 0090): the fixed step computed ahead, on another thread.
//!
//! The renderer shows state N while a *fork* of it runs step N+1 elsewhere;
//! when the accumulator says the step is due, the fork is adopted in place
//! of the state. A fork is the state itself minus the renderer queues
//! (volleys and impacts not read yet), so the adopted state is exactly the
//! state [`BattleSim::step`] would have produced in place: the queues and
//! read cursors the renderer consumed in between are carried over from the
//! state being replaced.
//!
//! The caller must discard a fork when the battle changed outside a step
//! after the fork was taken (orders, deployment…): only draining the
//! renderer queues is allowed in between. The Godot bridge checks this with
//! its pose epoch (`godot-bridge/src/battle_step_job.rs`).

use super::{BattleSim, DT, MAX_STEPS_PER_CALL};

impl BattleSim {
    /// Advances the battle by `dt` seconds like [`BattleSim::tick`], each
    /// fixed step being run by `step` (which must leave the battle as
    /// [`BattleSim::step`] would, e.g. by adopting a fork). Returns the
    /// number of steps run.
    pub fn tick_with(&mut self, dt: f64, mut step: impl FnMut(&mut BattleSim)) -> u32 {
        if self.finished || self.deploying || !dt.is_finite() || dt <= 0.0 {
            return 0;
        }
        self.accumulator += dt;
        let mut steps = 0;
        while self.accumulator >= DT - 1e-9 && steps < MAX_STEPS_PER_CALL && !self.finished {
            self.accumulator -= DT;
            step(self);
            steps += 1;
        }
        if steps == MAX_STEPS_PER_CALL {
            self.accumulator = 0.0;
        }
        steps
    }

    /// True when a fixed step would change the battle (not finished, not
    /// deploying): a fork is worth computing.
    pub fn can_step(&self) -> bool {
        !self.finished && !self.deploying
    }

    /// A copy of the battle to run the next [`BattleSim::step`] on, without
    /// the renderer queues (they stay with `self` and are merged back by
    /// [`BattleSim::adopt_step`]).
    pub fn fork_for_step(&self) -> BattleSim {
        let mut fork = self.clone();
        fork.shots.clear();
        fork.impacts.clear();
        fork
    }

    /// Replaces the battle by `next`, a [`BattleSim::fork_for_step`] of it
    /// that ran one step, keeping what belongs to the frame loop: the time
    /// accumulator, the volleys and impacts not read yet (the new ones
    /// queued after them, capped as [`BattleSim::step`] caps them) and the
    /// read cursors of the journal and of the siege effects. Returns the
    /// replaced state (to be dropped anywhere).
    pub fn adopt_step(&mut self, next: BattleSim) -> BattleSim {
        let mut old = std::mem::replace(self, next);
        self.accumulator = old.accumulator;
        let new_shots = std::mem::replace(&mut self.shots, std::mem::take(&mut old.shots));
        for shot in new_shots {
            self.record_shot(shot);
        }
        let new_impacts = std::mem::replace(&mut self.impacts, std::mem::take(&mut old.impacts));
        for impact in new_impacts {
            self.record_impact(impact);
        }
        self.events_read = old.events_read;
        self.assault.keep_read_cursor(&old.assault);
        old
    }
}
