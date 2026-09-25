//! EP8: the hour of the day on the battlefield (ADR 0052). The day moves on
//! with the battle; dawn, dusk and night shorten the shooters' range.

use super::BattleSim;
use crate::time_of_day::{wrap_hour, DayPhase, TimeOfDayRules};

/// Steps between two checks of the phase of the day (1 s of battle).
const PHASE_CHECK_TICKS: u64 = 10;

impl BattleSim {
    /// Sets the hour the battle began (campaign draw or quick battle);
    /// wrapped to `[0, 24)`.
    pub fn set_start_hour(&mut self, hour: f64) {
        self.start_hour = wrap_hour(hour);
        self.day_phase = None;
    }

    /// Hour the battle began.
    pub fn start_hour(&self) -> f64 {
        self.start_hour
    }

    /// Hour of the day now.
    pub fn hour(&self) -> f64 {
        TimeOfDayRules::bundled().hour_after(self.start_hour, self.elapsed)
    }

    /// Phase of the day now.
    pub fn day_phase(&self) -> &'static DayPhase {
        TimeOfDayRules::bundled().phase_at(self.hour())
    }

    /// Factor of the light on the shooters' range (1 in full daylight).
    pub fn visibility(&self) -> f64 {
        TimeOfDayRules::bundled().visibility_at(self.hour())
    }

    /// Factor on every shooting and spotting range: weather × light.
    pub fn range_factor(&self) -> f64 {
        self.weather.range_factor() * self.visibility()
    }

    /// Notes the phase of the day; announces a new one in the journal (not
    /// the phase the battle starts in).
    pub(super) fn advance_day(&mut self) {
        if !self.ticks.is_multiple_of(PHASE_CHECK_TICKS) {
            return;
        }
        let phase = self.day_phase();
        match &self.day_phase {
            Some(key) if *key == phase.key => {}
            Some(_) => {
                self.day_phase = Some(phase.key.clone());
                self.log(phase.announce.clone(), None);
            }
            None => self.day_phase = Some(phase.key.clone()),
        }
    }
}
