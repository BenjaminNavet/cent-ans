//! Plans and phases read from `data/rules/battle_plans.json`; conditions are data.
//!
//! A plan is a list of phases. A phase is entered when all its `enter`
//! conditions hold and left when all its (non-empty) `exit` conditions hold;
//! it is then replaced by the next phase, in list order, whose `enter`
//! conditions hold. Phases never go back.

use serde::{Deserialize, Serialize};

use super::division::DivisionShares;
use super::roles::PhaseRoles;
use crate::setup::SideId;

/// A number the planner can read from the battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    /// Simulated seconds.
    Time,
    /// Share of its initial soldiers the side has lost.
    OwnLosses,
    /// The same for the enemy.
    EnemyLosses,
    /// Share of the able enemy's power in shooters.
    EnemyShooterShare,
    /// 1 when an enemy regiment has planted stakes, else 0.
    EnemyHasStakes,
    /// Own power over enemy power.
    ForceRatio,
    /// Metres from our line to the nearest able enemy.
    EnemyDistance,
}

/// Comparison of a condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = ">=")]
    Ge,
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = "<=")]
    Le,
}

/// `metric op value`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub metric: Metric,
    pub op: Op,
    pub value: f64,
}

/// The values the conditions read, measured once per decision.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Metrics {
    pub time: f64,
    pub own_losses: f64,
    pub enemy_losses: f64,
    pub enemy_shooter_share: f64,
    pub enemy_has_stakes: f64,
    pub force_ratio: f64,
    pub enemy_distance: f64,
}

impl Metrics {
    fn get(&self, metric: Metric) -> f64 {
        match metric {
            Metric::Time => self.time,
            Metric::OwnLosses => self.own_losses,
            Metric::EnemyLosses => self.enemy_losses,
            Metric::EnemyShooterShare => self.enemy_shooter_share,
            Metric::EnemyHasStakes => self.enemy_has_stakes,
            Metric::ForceRatio => self.force_ratio,
            Metric::EnemyDistance => self.enemy_distance,
        }
    }
}

impl Condition {
    pub fn holds(&self, metrics: &Metrics) -> bool {
        let value = metrics.get(self.metric);
        match self.op {
            Op::Gt => value > self.value,
            Op::Ge => value >= self.value,
            Op::Lt => value < self.value,
            Op::Le => value <= self.value,
        }
    }
}

/// All conditions hold (an empty list always does).
pub fn all_hold(conditions: &[Condition], metrics: &Metrics) -> bool {
    conditions.iter().all(|c| c.holds(metrics))
}

/// One step of a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    pub id: String,
    pub name_fr: String,
    pub roles: PhaseRoles,
    pub enter: Vec<Condition>,
    pub exit: Vec<Condition>,
}

/// A plan: who plays it and its phases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub id: String,
    pub name_fr: String,
    pub sides: Vec<SideId>,
    /// A regiment ordered at another enemy than its division's target goes
    /// for the target when it is this close, otherwise it holds.
    pub retarget_reach_m: f64,
    /// A division keeps its target while it stays this close.
    pub target_reach_m: f64,
    pub phases: Vec<Phase>,
}

impl Plan {
    /// The phase to play given the one played so far (`None` at the start).
    pub fn next_phase(&self, current: Option<usize>, metrics: &Metrics) -> usize {
        match current {
            None => self
                .phases
                .iter()
                .position(|p| all_hold(&p.enter, metrics))
                .unwrap_or(0),
            Some(c) => {
                let phase = &self.phases[c];
                if phase.exit.is_empty() || !all_hold(&phase.exit, metrics) {
                    return c;
                }
                (c + 1..self.phases.len())
                    .find(|&k| all_hold(&self.phases[k].enter, metrics))
                    .unwrap_or(c)
            }
        }
    }
}

/// Contents of `data/rules/battle_plans.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub division_shares: DivisionShares,
    pub plans: Vec<Plan>,
}

data_model::bundled_rules!(PlanRules, "rules/battle_plans.json");

impl PlanRules {
    /// Index of the first plan played by `side`.
    pub fn plan_for(&self, side: SideId) -> Option<usize> {
        self.plans.iter().position(|p| p.sides.contains(&side))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::planner::roles::Role;

    fn cond(metric: Metric, op: Op, value: f64) -> Condition {
        Condition { metric, op, value }
    }

    #[test]
    fn conditions_compare() {
        let m = Metrics {
            time: 600.0,
            own_losses: 0.25,
            enemy_shooter_share: 0.4,
            enemy_has_stakes: 1.0,
            ..Metrics::default()
        };
        assert!(cond(Metric::Time, Op::Ge, 600.0).holds(&m));
        assert!(!cond(Metric::Time, Op::Gt, 600.0).holds(&m));
        assert!(cond(Metric::OwnLosses, Op::Le, 0.25).holds(&m));
        assert!(!cond(Metric::OwnLosses, Op::Lt, 0.25).holds(&m));
        assert!(cond(Metric::EnemyShooterShare, Op::Gt, 0.35).holds(&m));
        assert!(cond(Metric::EnemyHasStakes, Op::Gt, 0.5).holds(&m));
    }

    #[test]
    fn all_hold_needs_every_condition_and_accepts_none() {
        let m = Metrics {
            own_losses: 0.3,
            time: 100.0,
            ..Metrics::default()
        };
        let loss = cond(Metric::OwnLosses, Op::Gt, 0.25);
        let late = cond(Metric::Time, Op::Gt, 600.0);
        assert!(all_hold(&[], &m));
        assert!(all_hold(&[loss], &m));
        assert!(!all_hold(&[loss, late], &m));
    }

    fn two_phase_plan() -> Plan {
        let roles = |rear| PhaseRoles {
            vanguard: Role::Shoot,
            battle: Role::Hold,
            rear,
        };
        Plan {
            id: "t".into(),
            name_fr: "t".into(),
            sides: vec![SideId::Defender],
            retarget_reach_m: 100.0,
            target_reach_m: 500.0,
            phases: vec![
                Phase {
                    id: "a".into(),
                    name_fr: "a".into(),
                    roles: roles(Role::Reserve),
                    enter: vec![],
                    exit: vec![cond(Metric::OwnLosses, Op::Gt, 0.25)],
                },
                Phase {
                    id: "b".into(),
                    name_fr: "b".into(),
                    roles: roles(Role::Hold),
                    enter: vec![cond(Metric::OwnLosses, Op::Gt, 0.25)],
                    exit: vec![],
                },
            ],
        }
    }

    #[test]
    fn phases_advance_on_exit_and_never_go_back() {
        let plan = two_phase_plan();
        let calm = Metrics::default();
        let bled = Metrics {
            own_losses: 0.4,
            ..Metrics::default()
        };
        assert_eq!(plan.next_phase(None, &calm), 0);
        assert_eq!(plan.next_phase(Some(0), &calm), 0);
        assert_eq!(plan.next_phase(Some(0), &bled), 1);
        assert_eq!(plan.next_phase(Some(1), &calm), 1);
        // The first phase whose entry holds starts the plan; its exit then
        // moves on at the next decision.
        assert_eq!(plan.next_phase(None, &bled), 0);
    }

    #[test]
    fn exit_without_a_next_phase_stays() {
        let mut plan = two_phase_plan();
        plan.phases[1].enter = vec![cond(Metric::Time, Op::Gt, 9999.0)];
        let bled = Metrics {
            own_losses: 0.4,
            ..Metrics::default()
        };
        assert_eq!(plan.next_phase(Some(0), &bled), 0);
    }

    #[test]
    fn bundled_plans_are_consistent() {
        let rules = PlanRules::bundled();
        assert!(rules.plan_for(SideId::Defender).is_some());
        assert!(rules.plan_for(SideId::Attacker).is_none());
        for plan in &rules.plans {
            assert!(!plan.phases.is_empty());
            assert!(plan.retarget_reach_m > 0.0 && plan.target_reach_m > 0.0);
            // The first phase can be entered at the start of a calm battle.
            assert!(all_hold(&plan.phases[0].enter, &Metrics::default()));
        }
    }
}
