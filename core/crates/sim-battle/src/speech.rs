//! Real effect of the general's speech (lot TW bsim, ADR 0320). The text is
//! composed on the Godot side from `data/speeches/battle_speeches.json`; the
//! `morale_bonus` block of that file gives each side a temporary morale
//! bonus when the battle starts, by the odds (own men / enemy men, with the
//! file's `odds_thresholds`): the weaker side is the more stirred.

use data_model::UnitCategory;
use serde::Deserialize;

use crate::setup::SideId;
use crate::sim::BattleSim;

/// The parts of `data/speeches/battle_speeches.json` the rules read.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SpeechRules {
    pub odds_thresholds: OddsThresholds,
    /// Absent: the speech has no effect on the rules.
    #[serde(default)]
    pub morale_bonus: Option<SpeechMoraleBonus>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OddsThresholds {
    pub strong: f64,
    pub weak: f64,
}

/// Morale added by the speech, by the odds, and how long it lasts.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SpeechMoraleBonus {
    pub strong: f64,
    pub even: f64,
    pub weak: f64,
    pub duration_s: f64,
}

data_model::bundled_rules!(SpeechRules, "speeches/battle_speeches.json");

impl SpeechRules {
    /// Morale bonus for a side with `ratio` = own men / enemy men.
    pub fn bonus_for(&self, ratio: f64) -> f64 {
        let Some(bonus) = &self.morale_bonus else {
            return 0.0;
        };
        if ratio >= self.odds_thresholds.strong {
            bonus.strong
        } else if ratio <= self.odds_thresholds.weak {
            bonus.weak
        } else {
            bonus.even
        }
    }
}

impl BattleSim {
    fn side_men(&self, side: SideId) -> f64 {
        self.units()
            .iter()
            .filter(|u| u.side == side && u.present() && !u.synthetic)
            .filter(|u| u.category != UnitCategory::Siege)
            .map(|u| f64::from(u.soldiers()))
            .sum()
    }

    /// The speeches before the battle: each side's regiments get the morale
    /// bonus its odds call for (same mechanism as the war cry: temporary,
    /// added to the cap, wears off after `duration_s`).
    pub(crate) fn apply_speeches(&mut self) {
        let rules = SpeechRules::bundled();
        let Some(bonus) = &rules.morale_bonus else {
            return;
        };
        let men = [
            self.side_men(SideId::Attacker),
            self.side_men(SideId::Defender),
        ];
        for side in [SideId::Attacker, SideId::Defender] {
            let (own, other) = (men[side.index()], men[side.other().index()]);
            let ratio = if other > 0.0 { own / other } else { 1.0 };
            let value = rules.bonus_for(ratio);
            if value <= 0.0 {
                continue;
            }
            for unit in self
                .units_mut()
                .iter_mut()
                .filter(|u| u.side == side && u.present())
            {
                unit.morale_cap -= unit.order_morale;
                unit.order_morale = value;
                unit.order_morale_timer = bonus.duration_s;
                unit.morale_cap += value;
                unit.morale = (unit.morale + value).clamp(0.0, 100.0);
            }
        }
    }
}
