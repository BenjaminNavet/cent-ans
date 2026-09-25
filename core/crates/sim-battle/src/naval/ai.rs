//! Naval battle AI (lots NV1, NV2): every [`super::NAVAL_AI_PERIOD`] seconds
//! each fleet of an AI side plans the orders of its ships.
//!
//! - Chained ships and fleets told to hold keep their line and shoot; a
//!   chained ship falls on an enemy lashed to its neighbour when it can
//!   reach it.
//! - Fireships run down on the nearest enemy, downwind if they can.
//! - The others close and shoot; once most of the fleet lies within
//!   [`NavalRules::assault_range_m`] of the enemy for
//!   [`NavalRules::assault_softening_s`], the fleet calls the **general
//!   boarding** (l'Écluse, 1340): every ship that can take its target boards
//!   at once.
//! - Targets are shared out: the strongest ships choose first, at most
//!   [`NavalRules::boarders_per_target`] of them on one enemy; the others
//!   pick the next ship of the line, or shoot.
//! - Galleys ram low hulls; nobody grapples a burning ship; a beaten crew
//!   breaks off.
//!
//! [`NavalRules::assault_range_m`]: data_model::NavalRules::assault_range_m
//! [`NavalRules::assault_softening_s`]: data_model::NavalRules::assault_softening_s
//! [`NavalRules::boarders_per_target`]: data_model::NavalRules::boarders_per_target

use super::combat::climb_factor;
use super::outcome::NavalEventKind;
use super::ship::{Ship, ShipOrder};
use super::sim::NavalSim;
use crate::setup::SideId;

/// Plans the orders of `side`'s ships.
pub(crate) fn plan(sim: &mut NavalSim, side: SideId) {
    update_assault(sim, side);
    let hold = sim.setup.side(side).hold;
    let assault = sim.assault[side.index()].is_some();
    let ids: Vec<usize> = sim
        .ships
        .iter()
        .filter(|s| s.side == side && s.is_active())
        .map(|s| s.id as usize)
        .collect();
    // Enemy ships already taken on (boarders lashed alongside).
    let mut boarders = vec![0u32; sim.ships.len()];
    let mut shooters = vec![0u32; sim.ships.len()];
    let mut free = Vec::new();
    for &i in &ids {
        match locked(sim, i, hold) {
            Some(order) => {
                if let ShipOrder::Board { target } = order {
                    boarders[target as usize] += 1;
                }
                sim.ships[i].order = order;
            }
            None => free.push(i),
        }
    }
    // The strongest boarders choose first.
    free.sort_by(|&a, &b| {
        sim.ships[b]
            .melee_power()
            .total_cmp(&sim.ships[a].melee_power())
            .then(a.cmp(&b))
    });
    for i in free {
        let order = match pick_target(sim, &sim.ships[i], &boarders, &shooters) {
            Some(target) => decide(sim, i, target, assault),
            None => ShipOrder::Hold,
        };
        match order {
            ShipOrder::Board { target } | ShipOrder::Ram { target } => {
                boarders[target as usize] += 1;
            }
            ShipOrder::Shoot { target } => shooters[target as usize] += 1,
            _ => {}
        }
        sim.ships[i].order = order;
    }
}

/// Calls the general boarding of `side` once most of its free ships have
/// been within range of the enemy long enough.
fn update_assault(sim: &mut NavalSim, side: SideId) {
    if sim.assault[side.index()].is_some() || sim.setup.side(side).hold {
        return;
    }
    let rules = sim.rules();
    let range = rules.assault_range_m;
    let softening = rules.assault_softening_s;
    let enemies: Vec<&Ship> = sim
        .ships
        .iter()
        .filter(|s| s.side != side && s.is_active())
        .collect();
    let fleet: Vec<&Ship> = sim
        .ships
        .iter()
        .filter(|s| s.side == side && s.is_active() && !s.fireship)
        .collect();
    // A fleet chained at anchor waits for the enemy: no assault to call.
    if enemies.is_empty() || fleet.is_empty() || fleet.iter().all(|s| s.chain.is_some()) {
        return;
    }
    // Close to the ship each one goes for (a skirmish with a stray galley
    // does not count), or to any enemy when it has no target yet.
    let close = fleet
        .iter()
        .filter(|s| match s.order.target() {
            Some(t) => {
                let t = &sim.ships[t as usize];
                t.is_active() && s.distance_to(t) <= range
            }
            None => enemies.iter().any(|e| s.distance_to(e) <= range),
        })
        .count();
    // ... and most of the enemy within reach (not a skirmish on a wing).
    let reached = enemies
        .iter()
        .filter(|e| fleet.iter().any(|s| s.distance_to(e) <= range))
        .count();
    if close * 2 < fleet.len() || reached * 2 < enemies.len() {
        sim.assault_ready[side.index()] = None;
        return;
    }
    let since = *sim.assault_ready[side.index()].get_or_insert(sim.elapsed);
    if sim.elapsed - since < softening {
        return;
    }
    let lead = fleet
        .iter()
        .find(|s| s.flagship)
        .or_else(|| fleet.first())
        .map_or(0, |s| s.id);
    sim.assault[side.index()] = Some(sim.elapsed);
    sim.push_event(NavalEventKind::Assault { ship: lead, side });
}

/// Orders that leave no choice: chained or holding ships, ships locked in
/// a melee, fireships (their target: the nearest enemy, downwind).
fn locked(sim: &NavalSim, i: usize, hold: bool) -> Option<ShipOrder> {
    let ship = &sim.ships[i];
    let grappled_enemy =
        ship.grappled.iter().copied().find(|&g| {
            sim.ships[g as usize].side != ship.side && sim.ships[g as usize].is_afloat()
        });
    if ship.chain.is_some() || hold {
        return Some(match grappled_enemy.or_else(|| chain_counter(sim, ship)) {
            Some(target) => ShipOrder::Board { target },
            None => ShipOrder::Hold,
        });
    }
    if let Some(target) = grappled_enemy {
        // Locked in a melee: stay unless the fight is plainly lost.
        let enemy = &sim.ships[target as usize];
        if ship.morale < 30.0 && ship.melee_power() < enemy.melee_power() * 0.5 {
            return Some(ShipOrder::Disengage);
        }
        return Some(ShipOrder::Board { target });
    }
    if let Some(target) = sim.crossing_of(i) {
        // Crossing a taken chained ship onto the next one.
        return Some(ShipOrder::Board { target });
    }
    if ship.fireship {
        let target = pick_target(sim, ship, &[], &[])?;
        return Some(ShipOrder::Board { target });
    }
    None
}

/// A chained ship falls on an enemy lashed to one of its chain neighbours,
/// if that enemy lies within grapple reach and our men are fresh enough.
fn chain_counter(sim: &NavalSim, ship: &Ship) -> Option<u32> {
    let chain = ship.chain?;
    if ship.morale < 40.0 {
        return None;
    }
    let gap = sim.rules().grapple_gap_m;
    sim.ships
        .iter()
        .filter(|n| {
            n.side == ship.side && n.id != ship.id && n.chain == Some(chain) && n.is_afloat()
        })
        .flat_map(|n| n.grappled.iter().copied())
        .filter(|&e| {
            let enemy = &sim.ships[e as usize];
            enemy.side != ship.side
                && enemy.is_afloat()
                && ship.gap_to(enemy) <= gap
                && ship.melee_power() > enemy.melee_power() * 0.5
        })
        .min()
}

fn decide(sim: &NavalSim, i: usize, target: u32, assault: bool) -> ShipOrder {
    let ship = &sim.ships[i];
    let rules = sim.rules();
    let enemy = &sim.ships[target as usize];
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
    let archers = ship.ranged_power() > ship.melee_power() * 0.6;
    if assault {
        // General boarding: whoever can take its target goes in; pure
        // shooting ships keep the enemy's heads down.
        let pure_shooters = ship.ranged_power() > ship.melee_power() * 3.0;
        return if !pure_shooters && (mine >= theirs * rules.assault_odds || crew_share < 0.4) {
            ShipOrder::Board { target }
        } else if shooters {
            ShipOrder::Shoot { target }
        } else {
            ShipOrder::Board { target }
        };
    }
    let boardable = mine > theirs * 0.9 || enemy.morale < 35.0 || crew_share < 0.4;
    let softening = shooters && archers && crew_share > 0.7 && sim.elapsed < 420.0;
    // Before the general boarding, the fleet closes and shoots together
    // instead of going in ship by ship (a lone boarder is swarmed).
    let waiting = shooters && crew_share > 0.4 && sim.elapsed < 420.0;
    if boardable && !softening && !waiting {
        ShipOrder::Board { target }
    } else if shooters {
        ShipOrder::Shoot { target }
    } else if mine > theirs * 0.6 {
        ShipOrder::Board { target }
    } else {
        ShipOrder::Shoot { target }
    }
}

/// The enemy ship to go for: near, weak, not already taken on by as many
/// friends as [`data_model::NavalRules::boarders_per_target`]; runaways
/// only once no enemy stands and fights. `boarders` and `shooters` count
/// the friends already sent against each ship this round.
fn pick_target(sim: &NavalSim, ship: &Ship, boarders: &[u32], shooters: &[u32]) -> Option<u32> {
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
    let capacity = rules.boarders_per_target.max(1);
    candidates
        .into_iter()
        .map(|t| {
            let on_it = boarders.get(t.id as usize).copied().unwrap_or(0);
            let shot_at = shooters.get(t.id as usize).copied().unwrap_or(0);
            let full = if on_it >= capacity && !ship.fireship {
                2_000.0
            } else {
                0.0
            };
            let downwind = sim.wind.alignment((ship.x, ship.z), (t.x, t.z));
            let fireship_bonus = if ship.fireship {
                -downwind * 200.0
            } else {
                0.0
            };
            // Keep the target of the last plan unless another is much better.
            let steady = if ship.order.target() == Some(t.id) {
                -60.0
            } else {
                0.0
            };
            let screened = if ship.fireship {
                0.0
            } else {
                screen_penalty(sim, ship, t)
            };
            let score = ship.distance_to(t)
                + screened
                + 250.0 * (t.melee_power() / mine).min(4.0)
                + 40.0 * f64::from(on_it)
                + 60.0 * f64::from(shot_at)
                + full
                + steady
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

/// Penalty of a target hidden behind another enemy ship (the second
/// chained line behind the first): boarders take the front ships first.
fn screen_penalty(sim: &NavalSim, ship: &Ship, target: &Ship) -> f64 {
    let (dx, dz) = (target.x - ship.x, target.z - ship.z);
    let length = (dx * dx + dz * dz).sqrt();
    if length < 1e-6 {
        return 0.0;
    }
    let (ux, uz) = (dx / length, dz / length);
    let screened = sim.ships.iter().any(|e| {
        if e.id == target.id || e.side == ship.side || !e.is_afloat() {
            return false;
        }
        let (ex, ez) = (e.x - ship.x, e.z - ship.z);
        let along = ex * ux + ez * uz;
        let across = (ex * uz - ez * ux).abs();
        along > 0.0 && along < length - target.radius() && across < e.radius() + ship.radius() * 0.5
    });
    if screened {
        250.0
    } else {
        0.0
    }
}
