//! Naval battle AI (lot NV1): every [`super::NAVAL_AI_PERIOD`] seconds each
//! free ship of an AI side picks a target and a way to fight it.
//!
//! - Chained ships and fleets told to hold keep their line and shoot.
//! - Fireships run down on the nearest enemy, downwind if they can.
//! - Galleys ram low hulls (galleys, barges) and board what they can take.
//! - Others soften the target with their bows while it is fresh, then board
//!   when the climb and the numbers favour them; they never grapple a
//!   burning ship; a beaten crew breaks off.

use super::combat::climb_factor;
use super::ship::{Ship, ShipOrder};
use super::sim::NavalSim;
use crate::setup::SideId;

/// Plans the orders of `side`'s ships.
pub(crate) fn plan(sim: &mut NavalSim, side: SideId) {
    let hold = sim.setup.side(side).hold;
    let ids: Vec<usize> = sim
        .ships
        .iter()
        .filter(|s| s.side == side && s.is_active())
        .map(|s| s.id as usize)
        .collect();
    for i in ids {
        let order = decide(sim, i, hold);
        sim.ships[i].order = order;
    }
}

fn decide(sim: &NavalSim, i: usize, hold: bool) -> ShipOrder {
    let ship = &sim.ships[i];
    let rules = sim.rules();
    let grappled_enemy =
        ship.grappled.iter().copied().find(|&g| {
            sim.ships[g as usize].side != ship.side && sim.ships[g as usize].is_afloat()
        });
    if ship.chain.is_some() || hold {
        return match grappled_enemy {
            Some(target) => ShipOrder::Board { target },
            None => ShipOrder::Hold,
        };
    }
    if let Some(target) = grappled_enemy {
        // Locked in a melee: stay unless the fight is plainly lost.
        let enemy = &sim.ships[target as usize];
        if ship.morale < 30.0 && ship.melee_power() < enemy.melee_power() * 0.5 {
            return ShipOrder::Disengage;
        }
        return ShipOrder::Board { target };
    }
    let Some(target) = pick_target(sim, ship) else {
        return ShipOrder::Hold;
    };
    let enemy = &sim.ships[target as usize];
    if ship.fireship {
        return ShipOrder::Board { target };
    }
    let crew_share = enemy.fighting_men() / enemy.fighting_initial().max(1.0);
    let shooters = ship.ranged_power() > 0.0;
    if enemy.fire > 0.15 {
        return if shooters {
            ShipOrder::Shoot { target }
        } else {
            ShipOrder::Hold
        };
    }
    if ship.is_galley()
        && enemy.class.freeboard_m < 2.0
        && enemy.grappled.is_empty()
        && ship.stamina > 30.0
    {
        return ShipOrder::Ram { target };
    }
    let climb = enemy.deck_height() - ship.deck_height();
    let mine = ship.melee_power() * climb_factor(climb, rules);
    let theirs = enemy.melee_power() * climb_factor(-climb, rules);
    let boardable = mine > theirs * 0.9 || enemy.morale < 35.0 || crew_share < 0.4;
    let archers = ship.ranged_power() > ship.melee_power() * 0.6;
    let softening = shooters && archers && crew_share > 0.7 && sim.elapsed < 420.0;
    if boardable && !softening {
        ShipOrder::Board { target }
    } else if shooters {
        ShipOrder::Shoot { target }
    } else if mine > theirs * 0.6 {
        ShipOrder::Board { target }
    } else {
        ShipOrder::Shoot { target }
    }
}

/// The enemy ship to go for: near, weak, not already swarmed by friends;
/// runaways only once no enemy stands and fights.
fn pick_target(sim: &NavalSim, ship: &Ship) -> Option<u32> {
    let rules = sim.rules();
    let standing: Vec<&Ship> = sim
        .ships
        .iter()
        .filter(|t| t.side != ship.side && t.is_active())
        .collect();
    let candidates: Vec<&Ship> = if standing.is_empty() {
        sim.ships
            .iter()
            .filter(|t| t.side != ship.side && t.is_afloat())
            .collect()
    } else {
        standing
    };
    let mine = ship.melee_power().max(1.0);
    candidates
        .into_iter()
        .map(|t| {
            let swarm = sim
                .ships
                .iter()
                .filter(|f| {
                    f.side == ship.side
                        && f.id != ship.id
                        && f.is_afloat()
                        && f.order.target() == Some(t.id)
                })
                .count() as f64;
            let downwind = sim.wind.alignment((ship.x, ship.z), (t.x, t.z));
            let fireship_bonus = if ship.fireship {
                -downwind * 200.0
            } else {
                0.0
            };
            let score =
                ship.distance_to(t) + 250.0 * (t.melee_power() / mine).min(4.0) + 180.0 * swarm
                    - 150.0 * t.fire
                    + fireship_bonus
                    + if t.chain.is_some() && !ship.fireship {
                        0.0
                    } else {
                        rules.grapple_gap_m
                    };
            (t.id, score)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .map(|(id, _)| id)
}
