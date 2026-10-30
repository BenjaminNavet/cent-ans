//! IA-PLAN-1: division-based battle planner (audit 2026-10-29, R3).
//!
//! Behind `battle_ai.json` `planner: "plans"`; `"legacy"` (the default) keeps
//! [`super::plan_field`] alone. A side is cut into [`division::Division`]s at
//! deployment; a [`plans::Plan`] (data: `data/rules/battle_plans.json`) is a
//! list of phases, each giving a [`roles::Role`] to every division.
//!
//! This first lot has one plan, *Tenir et recevoir* (defender): the planner
//! picks the phase, gives the reserve to the divisions whose role is
//! `reserve`, holds the posture, and makes every division work on one
//! target. The positioning itself (ground, line, shooters, cavalry) is still
//! `plan_field`'s, called under a [`Posture`]; IA-PLAN-2 replaces it role by
//! role.

pub(super) mod division;
pub(super) mod plans;
pub(super) mod roles;

use super::*;
use crate::ai_rules::PlannerMode;
use division::Division;
use plans::{Metrics, Plan, PlanRules};
use roles::Role;

/// What the planner imposes on [`plan_field_with`].
pub(in crate::ai) struct Posture {
    /// The side holds its ground (no patience, no ratio test).
    pub(in crate::ai) hold: bool,
    /// Unit indices the reserve is picked from; `None`: no reserve.
    reserve_from: Option<Vec<usize>>,
}

impl Posture {
    /// The reserve is the rearmost foot regiment among `reserve_from` (the
    /// historical rule of at least four line regiments applies), or none.
    pub(in crate::ai) fn apply_reserve(&self, view: &View, roles: &mut Roles) {
        if let Some(r) = roles.reserve.take() {
            roles.line.push(r);
        }
        let Some(from) = &self.reserve_from else {
            return;
        };
        if roles.line.len() < 4 {
            return;
        }
        let u = view.units;
        let pick = roles
            .line
            .iter()
            .copied()
            .filter(|i| from.contains(i))
            .max_by(|&a, &b| {
                (-u[a].z * view.forward)
                    .total_cmp(&(-u[b].z * view.forward))
                    .then(a.cmp(&b))
            });
        if let Some(pick) = pick {
            roles.line.retain(|&i| i != pick);
            roles.reserve = Some(pick);
        }
    }
}

/// Planner memory: divisions and phase, per side. Kept in the sim, never
/// part of the replay digest (everything in it follows from the battle).
#[derive(Debug, Clone, Default)]
pub(crate) struct PlannerMemo {
    sides: [SideMemo; 2],
}

#[derive(Debug, Clone, Default)]
struct SideMemo {
    assigned: bool,
    divisions: Vec<Division>,
    phase: Option<usize>,
}

/// Does the planner drive `side` in this battle?
pub(in crate::ai) fn covers(sim: &BattleSim, side: SideId) -> bool {
    sim.planner_mode() == PlannerMode::Plans && PlanRules::bundled().plan_for(side).is_some()
}

/// The measures the conditions read.
fn measure(view: &View) -> Metrics {
    let contact = view
        .own
        .iter()
        .filter_map(|&i| view.nearest_enemy(i, |_| true).map(|(_, d)| d))
        .fold(f64::INFINITY, f64::min);
    Metrics {
        time: view.sim.elapsed(),
        own_losses: side_losses(view.units, view.side),
        enemy_losses: side_losses(view.units, view.side.other()),
        enemy_shooter_share: enemy_shooter_share(view),
        enemy_has_stakes: if view.enemies.iter().any(|&j| view.units[j].stakes_planted) {
            1.0
        } else {
            0.0
        },
        force_ratio: view.power(true) / view.power(false).max(1.0),
        enemy_distance: contact,
    }
}

/// Picks (or keeps) the target of every division: the enemy regiment nearest
/// to any of its able regiments (the threat the division meets first), kept
/// while it stays within `reach` metres of one of them and no other enemy is
/// closer by more than `TARGET_HYSTERESIS`.
fn update_targets(view: &View, divisions: &mut [Division], reach: f64) {
    const TARGET_HYSTERESIS: f64 = 40.0;
    let id_to_ix = |id: u32| view.units.iter().position(|u| u.id == id);
    for division in divisions {
        let members: Vec<usize> = division
            .regiments
            .iter()
            .filter_map(|&id| id_to_ix(id))
            .filter(|i| view.own.contains(i))
            .collect();
        // Distance from the division (its nearest regiment) to an enemy.
        let gap = |j: usize| {
            members
                .iter()
                .map(|&i| dist(&view.units[i], &view.units[j]))
                .fold(f64::INFINITY, f64::min)
        };
        let nearest = view
            .able_enemies()
            .min_by(|&a, &b| gap(a).total_cmp(&gap(b)).then(a.cmp(&b)));
        let Some(nearest) = nearest else {
            division.target = None;
            continue;
        };
        let kept = division.target.and_then(id_to_ix).filter(|&j| {
            view.able_enemies().any(|e| e == j)
                && gap(j) <= reach
                && gap(j) <= gap(nearest) + TARGET_HYSTERESIS
        });
        division.target = Some(view.units[kept.unwrap_or(nearest)].id);
    }
}

/// Makes the orders of `commands` obey the division targets: an attack at
/// another enemy goes to the division target when it is within
/// `retarget_reach` of the regiment, otherwise the regiment holds (a regiment
/// in melee keeps fighting). A regiment still fixed on another enemy and not
/// in melee is turned on the target when it is that close.
fn retarget(
    commands: &mut Vec<Command>,
    units: &[Unit],
    divisions: &[Division],
    retarget_reach: f64,
) {
    let unit = |id: u32| units.iter().find(|u| u.id == id);
    let target_of = |id: u32| {
        divisions
            .iter()
            .find(|d| d.regiments.contains(&id))
            .and_then(|d| d.target)
    };
    let close = |id: u32, target: u32| {
        unit(id)
            .zip(unit(target))
            .is_some_and(|(a, b)| dist(a, b) <= retarget_reach)
    };
    let mut out: Vec<Command> = Vec::with_capacity(commands.len());
    let mut ordered: Vec<u32> = Vec::new();
    for command in commands.drain(..) {
        let Command::Attack {
            units: ids,
            target,
            run,
            queue,
        } = command
        else {
            out.push(command);
            continue;
        };
        for id in ids {
            ordered.push(id);
            let Some(wanted) = target_of(id) else {
                out.push(Command::Attack {
                    units: vec![id],
                    target,
                    run,
                    queue,
                });
                continue;
            };
            if wanted == target {
                out.push(Command::Attack {
                    units: vec![id],
                    target,
                    run,
                    queue,
                });
            } else if close(id, wanted) {
                out.push(Command::Attack {
                    units: vec![id],
                    target: wanted,
                    run,
                    queue,
                });
            } else if unit(id).is_some_and(|u| u.state != UnitState::Melee) {
                out.push(Command::Halt { units: vec![id] });
            }
        }
    }
    // Regiments with no new order that are still fixed on another enemy.
    for division in divisions {
        let Some(wanted) = division.target else {
            continue;
        };
        for &id in &division.regiments {
            let Some(u) = unit(id) else { continue };
            if ordered.contains(&id)
                || !u.able()
                || u.state == UnitState::Melee
                || u.target.is_none_or(|t| t == wanted)
                || !close(id, wanted)
            {
                continue;
            }
            out.push(Command::Attack {
                units: vec![id],
                target: wanted,
                run: false,
                queue: false,
            });
        }
    }
    *commands = out;
}

/// Does every attack order of a division regiment name the division target?
pub(crate) fn obeys_division_targets(commands: &[Command], divisions: &[Division]) -> bool {
    commands.iter().all(|command| match command {
        Command::Attack { units, target, .. } => units.iter().all(|id| {
            divisions
                .iter()
                .find(|d| d.regiments.contains(id))
                .is_none_or(|d| d.target.is_none_or(|t| t == *target))
        }),
        _ => true,
    })
}

/// Field plan of a side the planner covers.
pub(in crate::ai) fn plan_field(view: &mut View) {
    let rules = PlanRules::bundled();
    let plan: &Plan = &rules.plans[rules.plan_for(view.side).expect("covered side")];
    let sim = view.sim;
    let mut memo = sim.planner_memo().borrow_mut();
    let side_memo = &mut memo.sides[view.side.index()];
    if !side_memo.assigned {
        side_memo.assigned = true;
        side_memo.divisions = division::assign(view.units, view.side, rules.division_shares);
    }
    let metrics = measure(view);
    let phase_ix = plan.next_phase(side_memo.phase, &metrics);
    side_memo.phase = Some(phase_ix);
    let phase = &plan.phases[phase_ix];
    update_targets(view, &mut side_memo.divisions, plan.target_reach_m);

    // The reserve comes from the divisions the phase keeps in reserve.
    let reserve_ids: Vec<u32> = side_memo
        .divisions
        .iter()
        .filter(|d| phase.roles.of(d.kind) == Role::Reserve)
        .flat_map(|d| d.regiments.iter().copied())
        .collect();
    let reserve_from = (!reserve_ids.is_empty()).then(|| {
        (0..view.units.len())
            .filter(|&i| reserve_ids.contains(&view.units[i].id))
            .collect()
    });
    let posture = Posture {
        hold: true,
        reserve_from,
    };
    plan_field_with(view, Some(&posture));
    retarget(
        &mut view.commands,
        view.units,
        &side_memo.divisions,
        plan.retarget_reach_m,
    );
    debug_assert!(
        obeys_division_targets(&view.commands, &side_memo.divisions),
        "a division regiment targets another enemy than its division target"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn division(ids: &[u32], target: Option<u32>) -> Division {
        Division {
            kind: division::DivisionKind::Battle,
            regiments: ids.to_vec(),
            target,
        }
    }

    fn attack(id: u32, target: u32) -> Command {
        Command::Attack {
            units: vec![id],
            target,
            run: true,
            queue: false,
        }
    }

    #[test]
    fn assertion_accepts_division_targets_and_foreign_regiments() {
        let divisions = [division(&[1, 2], Some(9))];
        assert!(obeys_division_targets(
            &[attack(1, 9), attack(2, 9), attack(7, 4)],
            &divisions
        ));
        assert!(!obeys_division_targets(&[attack(2, 4)], &divisions));
        // A division without a target constrains nobody.
        assert!(obeys_division_targets(
            &[attack(1, 4)],
            &[division(&[1], None)]
        ));
    }
}
