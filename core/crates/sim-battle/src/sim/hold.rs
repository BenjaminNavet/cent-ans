//! NT11: "hold ground" (camp tenu), a battle option per side, for the
//! guided battle prologue (lot NT4): while it is on, the regiments of the
//! side keep their place. The battle AI of the side may still change a
//! formation, a mode or its fire at will, never move ([`Self::hold_filter`]);
//! shooters in skirmish mode do not step back (`sim/modes.rs`); turning it on
//! halts the side where it stands. Combat is untouched: the melee, the push,
//! the morale and the rout under pressure (and the flight that follows) run
//! as ever; only the voluntary moves are gone.

use super::BattleSim;
use crate::command::Command;
use crate::modes::UnitMode;
use crate::setup::SideId;
use crate::unit::UnitState;

impl BattleSim {
    /// Does `side` hold its ground?
    pub fn holds(&self, side: SideId) -> bool {
        self.hold[side.index()]
    }

    /// Turns "hold ground" on or off for `side`. On: every able regiment of
    /// the side not routing, withdrawing or climbing stops where it stands
    /// (its march, target and queued orders dropped). Off: nothing moves by
    /// itself; the AI (if enabled) resumes at its next decision.
    pub fn set_hold(&mut self, side: SideId, enabled: bool) {
        self.hold[side.index()] = enabled;
        if !enabled {
            return;
        }
        for unit in self.units.iter_mut().filter(|u| u.side == side) {
            if !unit.present()
                || unit.withdrawing
                || unit.climbing.is_some()
                || unit.state == UnitState::Routing
            {
                continue;
            }
            unit.destination = None;
            unit.destination_facing = None;
            unit.target = None;
            unit.order_queue.clear();
            unit.running = false;
            if matches!(unit.state, UnitState::Marching | UnitState::Charging) {
                unit.state = UnitState::Idle;
            }
        }
    }

    /// A command of the battle AI of `side`, kept while the side holds its
    /// ground only if it moves no one: halt, formation, fire at will, and
    /// the modes but the run and the melee (shooters closing in).
    pub(super) fn hold_filter(&self, side: SideId, command: Command) -> Option<Command> {
        if !self.holds(side) {
            return Some(command);
        }
        match command {
            Command::Halt { .. } | Command::Formation { .. } | Command::FireAtWill { .. } => {
                Some(command)
            }
            Command::SetMode { mode, .. } if !matches!(mode, UnitMode::Run | UnitMode::Melee) => {
                Some(command)
            }
            _ => None,
        }
    }
}
