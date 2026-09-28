//! Battle AI, F5a § 2: formation changes and a coordinated cavalry flank.
//!
//! Runs after [`crate::ai::plan`] on the same state and appends commands
//! (later commands win). Deterministic: index order, hysteresis thresholds.
//! - Pikemen (`pike_square`) form a schiltron when enemy horse closes in
//!   (120 m) and go back to line once it is gone (220 m).
//! - Lancers (`charge_lance`) form a wedge when charging a target within
//!   250 m, and a line again once the charge is over.
//! - Foot and horse on a long march (> 250 m, no enemy within 300 m) move in
//!   column, and deploy back into line when the enemy is within 220 m.
//! - Two regiments of horse riding round the same enemy regiment take
//!   opposite flanks (the second one's waypoint is mirrored).

use data_model::{Ability, UnitCategory};

use crate::command::Command;
use crate::setup::SideId;
use crate::sim::BattleSim;
use crate::unit::{Formation, Unit, UnitState};

const SCHILTRON_ENTER: f64 = 120.0;
const SCHILTRON_LEAVE: f64 = 220.0;
const WEDGE_RANGE: f64 = 250.0;
const COLUMN_MARCH: f64 = 250.0;
const COLUMN_CLEAR: f64 = 300.0;
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
    let foot = u.category == UnitCategory::Infantry && !u.mounted;
    if foot && u.has(Ability::PikeSquare) {
        let d = nearest(&horse);
        if d < SCHILTRON_ENTER {
            return Some(Formation::Square);
        }
        if u.formation == Formation::Square && d > SCHILTRON_LEAVE {
            return Some(Formation::Line);
        }
        if u.formation == Formation::Square {
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
                    t.present() && dist(u, t.x, t.z) < WEDGE_RANGE && !guarded
                });
        if charging {
            return Some(Formation::Wedge);
        }
        // The wedge is kept through the melee (reforming in contact would
        // shift the front); back to line once the charge is over.
        if u.formation == Formation::Wedge && u.state != UnitState::Melee {
            return Some(Formation::Line);
        }
    }
    if u.state == UnitState::Melee {
        return None;
    }
    let near = nearest(&|_| true);
    if u.formation == Formation::Column && near < COLUMN_DEPLOY {
        return Some(Formation::Line);
    }
    let long_march = u
        .destination
        .is_some_and(|(x, z)| dist(u, x, z) > COLUMN_MARCH);
    let marcher = foot || horse(u);
    if marcher && long_march && near > COLUMN_CLEAR && u.formation == Formation::Line {
        return Some(Formation::Column);
    }
    if u.formation == Formation::Column && !long_march {
        return Some(Formation::Line);
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
