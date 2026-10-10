//! Aftermath of a 3D field battle (lot TW pursuit, ADR 0321): the victors
//! chase the routed and the forced retreats (killed and taken alive), and
//! each regiment earns experience for its kills and survival. Pure
//! functions of the final state (no random draw: same battle, same result).
//! Rates in `data/rules/battle_outcome.json`.

use data_model::{BattleOutcomeRules, UnitCategory};

use super::BattleSim;
use crate::decision::BattleEnd;
use crate::setup::SideId;
use crate::unit::UnitFate;

/// What the pursuit and the fight earned one side (indices follow
/// [`crate::SideSetup::units`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Aftermath {
    /// Soldiers cut down by the pursuit, per campaign unit.
    pub pursuit_killed: Vec<u32>,
    /// Soldiers taken alive, per campaign unit.
    pub captured_by_unit: Vec<u32>,
    /// Experience earned, in thousandths of a level, per campaign unit.
    pub unit_xp_milli: Vec<u32>,
}

impl BattleSim {
    /// Pursuit losses and experience of `side` once the battle is over.
    pub(crate) fn aftermath(&self, side: SideId, winner: SideId, end: BattleEnd) -> Aftermath {
        let rules = BattleOutcomeRules::bundled();
        let count = self.setup.side(side).units.len();
        let mut result = Aftermath {
            pursuit_killed: vec![0; count],
            captured_by_unit: vec![0; count],
            unit_xp_milli: vec![0; count],
        };
        let won = side == winner;
        // A refused battle has no fight to learn from.
        if end == BattleEnd::Refused {
            return result;
        }
        // Field battles only; a refused battle or a lull has no rout.
        let pursued = !won
            && self.siege.is_none()
            && !matches!(end, BattleEnd::Refused | BattleEnd::Lull);
        let pursuit = &rules.pursuit;
        let fleeing: f64 = self
            .units
            .iter()
            .filter(|u| u.side == side && !u.synthetic && u.fate() == UnitFate::Routed)
            .map(|u| f64::from(u.soldiers()))
            .sum();
        let cavalry: f64 = self
            .units
            .iter()
            .filter(|u| u.side == winner && !u.synthetic && u.category == UnitCategory::Cavalry)
            .filter(|u| u.able())
            .map(|u| f64::from(u.soldiers()))
            .sum();
        let chasers = if fleeing > 0.0 { cavalry / fleeing } else { 0.0 };
        let routed_share = (pursuit.routed_base + pursuit.per_cavalry_ratio * chasers)
            .clamp(0.0, pursuit.max_share.clamp(0.0, 1.0));
        // "No quarter": every man caught is killed, none taken.
        let captive_share = if self.no_quarter[winner.index()] {
            0.0
        } else {
            pursuit.captive_share.clamp(0.0, 1.0)
        };
        let xp = &rules.unit_xp;
        let mut kills = vec![0.0_f64; count];
        let mut survived = vec![false; count];
        for unit in self.units.iter().filter(|u| u.side == side && !u.synthetic) {
            let index = unit.setup_index;
            if index >= count {
                continue;
            }
            kills[index] += unit.kills;
            let fate = unit.fate();
            survived[index] |= matches!(fate, UnitFate::Held | UnitFate::Withdrawn);
            if !pursued {
                continue;
            }
            let mut share = match fate {
                UnitFate::Routed => routed_share,
                UnitFate::Withdrawn => pursuit.withdrawn_share,
                _ => 0.0,
            };
            if unit.category == UnitCategory::Cavalry {
                share *= pursuit.fleeing_cavalry_factor.clamp(0.0, 1.0);
            }
            let men = unit.soldiers();
            let caught = ((f64::from(men) * share.clamp(0.0, 1.0)).floor() as u32).min(men);
            let taken = ((f64::from(caught) * captive_share).floor() as u32).min(caught);
            result.captured_by_unit[index] += taken;
            result.pursuit_killed[index] += caught - taken;
        }
        for index in 0..count {
            let earned = if survived[index] { xp.survival_milli } else { 0 }
                + (kills[index] / 10.0 * f64::from(xp.per_ten_kills_milli)).floor() as u32
                + if won { xp.victory_milli } else { 0 };
            result.unit_xp_milli[index] = earned.min(xp.max_milli);
        }
        result
    }
}
