//! Battle AI, F5a § 2: formation changes and a coordinated cavalry flank.
//!
//! Runs after [`crate::ai::plan`] on the same state and appends commands
//! (later commands win). Deterministic: index order, hysteresis thresholds.
//! - Pikemen (`pike_square`) form a schiltron when enemy horse closes in
//!   (120 m) and go back to line once it is gone (220 m).
//! - Lancers (`charge_lance`) form a wedge when charging a target within
//!   250 m, and a line again once the charge is over.
//! - Foot and horse on a long march (> 250 m, no enemy within 450 m) move in
//!   column, and deploy back into line when the enemy is within 350 m (RJ-a: the line takes time to form).
//! - Two regiments of horse riding round the same enemy regiment take
//!   opposite flanks (the second one's waypoint is mirrored).
//!
//! RJ-a (ADR 0174): the formations are those of `unit_formations.json`
//! with the matching `ai_role` (default, march, anti_cavalry, charge) that
//! the regiment may take; a regiment in a formation the AI does not use
//! (chosen by the player before an AI takeover) is left as it is.

use data_model::{Ability, UnitCategory};

use crate::command::Command;
use crate::formations::AiRole;
use crate::setup::SideId;
use crate::sim::BattleSim;
use crate::unit::{Formation, Unit, UnitState};

const SCHILTRON_ENTER: f64 = 120.0;
const SCHILTRON_LEAVE: f64 = 220.0;
const WEDGE_RANGE: f64 = 250.0;
/// RJ-a: seconds after a change of formation before the AI puts a regiment
/// in march or in wedge again (each change costs a reformation).
const FORMATION_HOLD: f64 = 20.0;
/// RJ-a: closer than this the wedge would not be formed before the impact
/// (the change takes a few seconds): the horse charges as it stands.
const WEDGE_MIN: f64 = 120.0;
/// RJ-a: no wedge within this distance of a hedge or ditch (metres).
const OBSTACLE_CLEAR: f64 = 60.0;
const COLUMN_MARCH: f64 = 250.0;
const COLUMN_CLEAR: f64 = 300.0;
/// RJ-a: deploying back into line takes time (reformation), hence 350 m (was 220).
const COLUMN_DEPLOY: f64 = 220.0;

fn dist(a: &Unit, x: f64, z: f64) -> f64 {
    ((a.x - x).powi(2) + (a.z - z).powi(2)).sqrt()
}

fn horse(u: &Unit) -> bool {
    u.category == UnitCategory::Cavalry && u.mounted
}

/// Wanted formation of `u`, or `None` to keep the current one.
fn wanted(sim: &BattleSim, u: &Unit) -> Option<Formation> {
    let units = sim.units();
    let enemies = || units.iter().filter(|e| e.side != u.side && e.able());
    let nearest = |keep: &dyn Fn(&Unit) -> bool| {
        enemies()
            .filter(|e| keep(e))
            .map(|e| dist(u, e.x, e.z))
            .fold(f64::INFINITY, f64::min)
    };
    let role = |r: AiRole| Formation::for_role(r, u);
    let current = u.formation.role();
    let line = role(AiRole::Default);
    let foot = u.category == UnitCategory::Infantry && !u.mounted;
    if let Some(square) = role(AiRole::AntiCavalry).filter(|_| foot && u.has(Ability::PikeSquare)) {
        let d = nearest(&horse);
        if d < SCHILTRON_ENTER {
            return Some(square);
        }
        if current == Some(AiRole::AntiCavalry) && d > SCHILTRON_LEAVE {
            return line;
        }
        if current == Some(AiRole::AntiCavalry) {
            return None;
        }
    }
    if horse(u) && u.has(Ability::ChargeLance) {
        let charging = u.running
            && u.target
                .and_then(|t| units.get(t as usize))
                .is_some_and(|t| {
                    // A clean charge only: no stakes or pikes around the target.
                    let guarded = enemies().any(|e| {
                        (e.stakes_planted || e.has(Ability::PikeSquare))
                            && dist(e, t.x, t.z) < 100.0
                    });
                    // RJ-a: nor a hedge or ditch close by (riding round
                    // its end, a change of formation would only slow the
                    // horse).
                    let hedge =
                        sim.field().obstacles.iter().any(|o| {
                            o.kind.breaks_charge() && o.distance(u.x, u.z) < OBSTACLE_CLEAR
                        });
                    t.present()
                        && (WEDGE_MIN..WEDGE_RANGE).contains(&dist(u, t.x, t.z))
                        && !guarded
                        && !hedge
                });
        if let Some(wedge) = role(AiRole::Charge).filter(|_| charging) {
            return Some(wedge);
        }
        // The wedge is kept through the melee (reforming in contact would
        // shift the front) and while riding at an enemy close by (RJ-a: a
        // detour round an obstacle does not undo it); back to line once the
        // charge is over.
        let riding_in = u.running && u.target.is_some() && nearest(&|_| true) < WEDGE_RANGE;
        if current == Some(AiRole::Charge) && u.state != UnitState::Melee && !riding_in {
            return line;
        }
    }
    if u.state == UnitState::Melee {
        return None;
    }
    let near = nearest(&|_| true);
    if current == Some(AiRole::March) && near < COLUMN_DEPLOY {
        return line;
    }
    let long_march = u
        .destination
        .is_some_and(|(x, z)| dist(u, x, z) > COLUMN_MARCH);
    let marcher = foot || horse(u);
    if marcher && long_march && near > COLUMN_CLEAR && current == Some(AiRole::Default) {
        return role(AiRole::March);
    }
    if current == Some(AiRole::March) && !long_march {
        return line;
    }
    None
}

/// Formation commands of `side` (units already free of other duties).
pub fn plan_formations(sim: &BattleSim, side: SideId) -> Vec<Command> {
    sim.units()
        .iter()
        .filter(|u| u.side == side && u.able() && !u.synthetic && !u.on_wall)
        .filter(|u| u.climbing.is_none())
        .filter_map(|u| {
            let kind = wanted(sim, u).filter(|&k| k != u.formation)?;
            // RJ-a: a regiment changing formation finishes first, unless
            // horse is upon pikemen.
            // Back to the line (deploying, after a charge) is never held
            // back once formed; a march or a wedge is not taken again
            // within FORMATION_HOLD of the last change.
            let optional = matches!(kind.role(), Some(AiRole::March | AiRole::Charge));
            let settling = u.reforming() || (optional && u.formed_for < FORMATION_HOLD);
            if settling && kind.role() != Some(AiRole::AntiCavalry) {
                return None;
            }
            Some(Command::Formation {
                units: vec![u.id],
                kind,
            })
        })
        .collect()
}

/// Sortie of the garrison (F5a § 4): once `SiegeWorks::sortie` is set, the
/// regiments off the wall walk fall on the nearest besieger.
pub fn plan_sortie(sim: &BattleSim, side: SideId) -> Vec<Command> {
    let sortie = side == SideId::Defender && sim.siege().is_some_and(|w| w.sortie);
    if !sortie {
        return Vec::new();
    }
    let units = sim.units();
    let foes: Vec<&Unit> = units
        .iter()
        .filter(|u| u.side != side && u.able() && !u.synthetic)
        .collect();
    units
        .iter()
        .filter(|u| u.side == side && u.able() && u.state != UnitState::Melee)
        // Shooters keep the wall walk; the foot and horse sally.
        .filter(|u| !(u.on_wall && u.can_shoot()))
        .filter_map(|u| {
            let foe = foes.iter().min_by(|a, b| {
                dist(u, a.x, a.z)
                    .total_cmp(&dist(u, b.x, b.z))
                    .then(a.id.cmp(&b.id))
            })?;
            (u.target != Some(foe.id)).then(|| Command::Attack {
                units: vec![u.id],
                target: foe.id,
                run: true,
                queue: false,
            })
        })
        .collect()
}

/// The enemy regiment in melee whose flank the horse move `(x, z)` rides
/// round to, with the signed lateral offset of the waypoint.
fn flank_goal(sim: &BattleSim, side: SideId, x: f64, z: f64) -> Option<(usize, f64)> {
    let units = sim.units();
    (0..units.len())
        .filter(|&j| units[j].side != side && units[j].able())
        .filter(|&j| units[j].state == UnitState::Melee)
        .filter_map(|j| {
            let e = &units[j];
            let (rx, rz) = e.right();
            let lateral = (x - e.x) * rx + (z - e.z) * rz;
            let half = e.extent().0 * 0.5;
            (lateral.abs() > half && dist(e, x, z) < half + 50.0).then_some((j, lateral))
        })
        .min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()).then(a.0.cmp(&b.0)))
}

/// Two regiments of horse riding round the same enemy on the same side:
/// the second one takes the other flank.
pub fn coordinate_flanks(sim: &BattleSim, side: SideId, commands: &mut [Command]) {
    let units = sim.units();
    let mut taken: Vec<(usize, f64)> = Vec::new();
    for command in commands.iter_mut() {
        let Command::Move {
            units: ids, x, z, ..
        } = command
        else {
            continue;
        };
        let [id] = ids.as_slice() else { continue };
        if !units.get(*id as usize).is_some_and(horse) {
            continue;
        }
        let Some((j, lateral)) = flank_goal(sim, side, *x, *z) else {
            continue;
        };
        if taken.iter().any(|&(k, l)| k == j && l * lateral > 0.0) {
            let (rx, rz) = units[j].right();
            (*x, *z) = sim.field().clamp_inside(
                *x - 2.0 * lateral * rx,
                *z - 2.0 * lateral * rz,
                crate::ai::FIELD_MARGIN,
            );
            taken.push((j, -lateral));
        } else {
            taken.push((j, lateral));
        }
    }
}
