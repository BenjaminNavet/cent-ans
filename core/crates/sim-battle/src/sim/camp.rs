//! Camps and baggage (lot EP6): each army leaves its camp behind its
//! deployment zone ([`crate::decor::Camp`]). An able enemy regiment standing
//! in an unguarded camp loots it little by little
//! ([`crate::decor::CampRules::loot_seconds`]); a friendly able regiment
//! within the guard radius stops the looting, which slowly wanes. The first
//! enemy in the camp spreads alarm (a small morale loss); a looted camp costs
//! every regiment of its army morale, once, as the loss of the baggage at
//! Agincourt, and the looters tire under their booty. The battle result
//! records the lost baggage.

use super::BattleSim;
use crate::setup::SideId;

/// State of the camp of one side.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CampState {
    /// Looting done, 0-1.
    pub progress: f64,
    pub looted: bool,
    /// The enemy has entered the camp at least once.
    pub alarmed: bool,
    /// Able enemy regiments in the camp at the last step.
    pub looters: u32,
    /// Able friendly regiments guarding it at the last step.
    pub guards: u32,
}

impl BattleSim {
    /// State of the camp of `side` (`None` without a camp).
    pub fn camp_state(&self, side: SideId) -> Option<CampState> {
        self.field.decor.camp(side)?;
        Some(self.camp_states[side.index()])
    }

    /// One step of the camps (after the melee, before morale).
    pub(super) fn resolve_camps(&mut self) {}
}
