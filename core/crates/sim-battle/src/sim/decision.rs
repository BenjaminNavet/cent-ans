//! End of a field battle (lot EP9, ADR 0056): the broken army and the
//! refused battle. Rules in [`crate::decision`].

use super::{BattleSim, AI_PERIOD, DT};
use crate::decision::{BattleEnd, DecisionRules};
use crate::setup::SideId;
use crate::unit::UnitState;

impl BattleSim {
    /// Rules of the end of this battle.
    pub fn decision_rules(&self) -> &DecisionRules {
        &self.decision
    }

    /// Replaces the rules of the end of this battle (tests, quick battles).
    pub fn set_decision_rules(&mut self, rules: DecisionRules) {
        self.decision = rules;
    }

    /// How the battle ended, once finished.
    pub fn end_kind(&self) -> Option<BattleEnd> {
        self.end
    }

    /// Seconds since the last engagement (melee, telling losses, approach).
    pub fn quiet_time(&self) -> f64 {
        self.elapsed - self.clock.last
    }

    /// Nobody has engaged for `seconds` and the refusal rule can end the
    /// battle (field battle, end checks on): a defender on its ground has
    /// no reason to come out (AI).
    pub fn quiet_for(&self, seconds: f64) -> bool {
        self.end_conditions && self.siege.is_none() && self.quiet_time() >= seconds
    }

    /// Soldiers of `side` still able to fight (present and in good order),
    /// plus its reserves, as a share of its initial soldiers.
    pub fn fighting_share(&self, side: SideId) -> f64 {
        let (able, initial) = self
            .units
            .iter()
            .filter(|u| u.side == side && !u.synthetic)
            .fold((0.0, 0.0), |(able, initial), u| {
                let fit = if u.able() || u.reserve {
                    f64::from(u.soldiers())
                } else {
                    0.0
                };
                (able + fit, initial + f64::from(u.initial_soldiers))
            });
        if initial > 0.0 {
            able / initial
        } else {
            0.0
        }
    }

    /// Share of its initial soldiers `side` has lost.
    fn loss_share(&self, side: SideId) -> f64 {
        let (left, initial) = self
            .units
            .iter()
            .filter(|u| u.side == side && !u.synthetic)
            .fold((0.0, 0.0), |(left, initial), u| {
                (
                    left + f64::from(u.soldiers()),
                    initial + f64::from(u.initial_soldiers),
                )
            });
        if initial > 0.0 {
            1.0 - left / initial
        } else {
            0.0
        }
    }

    /// Break share of `side` (higher once its general is lost).
    pub fn break_share(&self, side: SideId) -> f64 {
        let had_general = self.units.iter().any(|u| u.side == side && u.is_general);
        if had_general && !self.general_alive[side.index()] {
            self.decision.break_share_without_general
        } else {
            self.decision.break_share
        }
    }

    /// Nearest distance between two able regiments of opposite sides.
    fn army_gap(&self) -> Option<f64> {
        let mut best: Option<f64> = None;
        let able: Vec<_> = self
            .units
            .iter()
            .filter(|u| u.able() && !u.synthetic)
            .collect();
        for (k, a) in able.iter().enumerate() {
            for b in &able[k + 1..] {
                if a.side != b.side {
                    let d = (a.x - b.x).hypot(a.z - b.z);
                    best = Some(best.map_or(d, |m: f64| m.min(d)));
                }
            }
        }
        best
    }

    /// Updates the engagement clock (every step, after the fighting).
    pub(super) fn track_engagement(&mut self) {
        let melee = self
            .units
            .iter()
            .any(|u| u.present() && !u.synthetic && u.state == UnitState::Melee);
        let hit = self
            .units
            .iter()
            .any(|u| !u.synthetic && u.tick_losses > 0.0);
        self.clock.fought |= melee || hit;
        self.clock.melee_seen |= melee;
        let losses = SideId::BOTH.map(|s| self.loss_share(s));
        let bled = SideId::BOTH.iter().any(|s| {
            losses[s.index()] - self.clock.losses_mark[s.index()]
                >= self.decision.engagement_loss_share
        });
        if melee || bled {
            self.clock.last = self.elapsed;
            self.clock.losses_mark = losses;
        }
        let period = (AI_PERIOD / DT).round() as u64;
        if !self.ticks.is_multiple_of(period) {
            return;
        }
        let Some(gap) = self.army_gap() else {
            return;
        };
        let window = self.decision.approach_window_seconds;
        match self.clock.gap_mark {
            Some((mark, _)) if gap <= mark - self.decision.approach_meters => {
                self.clock.last = self.elapsed;
                self.clock.gap_mark = Some((gap, self.elapsed));
                self.clock.losses_mark = SideId::BOTH.map(|s| self.loss_share(s));
            }
            Some((_, since)) if self.elapsed - since < window => {}
            _ => self.clock.gap_mark = Some((gap, self.elapsed)),
        }
    }

    /// EP9 end of a field battle, if any: `(winner, how)`.
    pub(super) fn field_decision(&self) -> Option<(SideId, BattleEnd)> {
        if self.siege.is_some() {
            return None;
        }
        let shares = SideId::BOTH.map(|s| self.fighting_share(s));
        let broken = SideId::BOTH.map(|s| shares[s.index()] < self.break_share(s));
        match broken {
            [true, false] => return Some((SideId::Defender, BattleEnd::Broken)),
            [false, true] => return Some((SideId::Attacker, BattleEnd::Broken)),
            [true, true] => {
                let winner = if shares[0] > shares[1] {
                    SideId::Attacker
                } else {
                    SideId::Defender
                };
                return Some((winner, BattleEnd::Broken));
            }
            [false, false] => {}
        }
        let patience = if self.clock.melee_seen {
            self.decision.lull_seconds
        } else {
            self.decision.refusal_seconds
        };
        if self.quiet_time() < patience {
            return None;
        }
        if !self.clock.fought {
            return Some((SideId::Defender, BattleEnd::Refused));
        }
        let attacker_better = self.loss_share(SideId::Attacker) + self.decision.lull_loss_margin
            < self.loss_share(SideId::Defender);
        let winner = if attacker_better {
            SideId::Attacker
        } else {
            SideId::Defender
        };
        Some((winner, BattleEnd::Lull))
    }

    /// The loser of a broken army routs; the loser of a refused battle or
    /// a lull withdraws in good order.
    pub(super) fn apply_decision(&mut self, loser: SideId, end: BattleEnd) {
        for unit in self
            .units
            .iter_mut()
            .filter(|u| u.side == loser && u.able() && !u.synthetic)
        {
            match end {
                BattleEnd::Broken => {
                    unit.state = UnitState::Routing;
                    unit.morale = unit.morale.min(super::ROUT_MORALE - 1.0);
                    unit.target = None;
                    unit.destination = None;
                    unit.stakes_planted = false;
                    unit.pavise = None;
                    unit.charge_timer = 0.0;
                }
                BattleEnd::Refused | BattleEnd::Lull => {
                    unit.withdrawing = true;
                    unit.target = None;
                    unit.pavise = None;
                }
                _ => {}
            }
        }
    }
}
