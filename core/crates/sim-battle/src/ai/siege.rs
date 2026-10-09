//! Siege battles: attacker and defender plans.

use super::*;

pub(super) fn outer_point(works: &SiegeWorks, piece: usize, offset: f64) -> (f64, f64) {
    let p = &works.pieces[piece];
    let (mx, mz) = p.midpoint();
    let (nx, nz) = p.outward();
    (mx + nx * offset, mz + nz * offset)
}

/// SG4: the attacker's progress in the assault: the last time a ram struck,
/// ladders went up, a regiment gained the wall walk, a tower docked, or the
/// works gave way (0 before any).
pub(super) fn last_progress(sim: &BattleSim) -> f64 {
    use crate::siege_fx::SiegeFxKind as K;
    sim.siege_fx()
        .iter()
        .rev()
        .find(|fx| {
            matches!(
                fx.kind,
                K::RamStrike { .. }
                    | K::LaddersRaised { .. }
                    | K::OnWall { .. }
                    | K::TowerDocked { .. }
                    | K::GateBroken { .. }
                    | K::WallBreached { .. }
            )
        })
        .map_or(0.0, |fx| fx.time)
}

/// SG4: is the assault clearly lost? No way in (no opening, nobody of ours
/// on the walls, climbing or inside, no tower rolling) and either nothing
/// has moved for `BattleAiRules::assault_stall` seconds since the escalade could start,
/// or our strength fell below `BattleAiRules::assault_hopeless` of the garrison's.
pub(super) fn assault_lost(view: &View, works: &SiegeWorks, towers_rolling: bool) -> bool {
    let units = view.units;
    let elapsed = view.sim.elapsed();
    if elapsed <= tuning().engine_patience || !works.openings().is_empty() || towers_rolling {
        return false;
    }
    let in_the_town = view.own.iter().any(|&i| {
        let u = &units[i];
        u.on_wall || u.climbing.is_some() || works.inside(u.x, u.z)
    });
    if in_the_town {
        return false;
    }
    let stalled = elapsed > tuning().engine_patience + tuning().assault_stall
        && elapsed - last_progress(view.sim) > tuning().assault_stall;
    stalled || view.power(true) < view.power(false) * tuning().assault_hopeless
}

pub(super) fn plan_siege_attack(view: &mut View, works: &SiegeWorks) {
    let units = view.units;
    let elapsed = view.sim.elapsed();
    let front = works.front_walls();
    let band = works.band();
    let own = view.own.clone();
    let engines: Vec<usize> = own
        .iter()
        .copied()
        .filter(|&i| units[i].wall_breaker())
        .collect();
    let towers: Vec<usize> = own
        .iter()
        .copied()
        .filter(|&i| units[i].siege_tower())
        .collect();
    let openings = works.openings();
    let engines_working = engines.iter().any(|&i| units[i].ammo > 0) && openings.is_empty();
    let docked: Vec<usize> = (0..works.pieces.len())
        .filter(|&p| works.pieces[p].intact() && works.pieces[p].docked_tower.is_some())
        .collect();
    let towers_rolling = towers.iter().any(|&t| {
        let u = &units[t];
        u.destination.is_some() || u.state == UnitState::Marching
    });
    let storm = !openings.is_empty();
    let gate_intact = works.pieces[works.gate].intact();
    // SG4: rams of the side, manned or abandoned (their crew all dead).
    let rams: Vec<usize> = (0..units.len())
        .filter(|&i| {
            let u = &units[i];
            u.ram && u.side == view.side && !u.left_field && !u.withdrawing && !u.reserve
        })
        .collect();
    // Nobody left who can get in, or (SG4) the assault is clearly lost:
    // sound the retreat.
    let climbers_left = own
        .iter()
        .any(|&i| units[i].can_climb() || (is_shooter(&units[i]) && !units[i].mounted));
    let ram_left = !rams.is_empty() && gate_intact;
    let engines_left = engines.iter().any(|&i| units[i].ammo > 0);
    let hopeless = !storm && !climbers_left && !ram_left && !engines_left;
    if hopeless || assault_lost(view, works, towers_rolling) {
        let all: Vec<u32> = own
            .iter()
            .filter(|&&i| !units[i].withdrawing)
            .map(|&i| units[i].id)
            .collect();
        if !all.is_empty() {
            view.commands.push(Command::Withdraw { units: all });
        }
        return;
    }
    let escalade = !storm
        && (!docked.is_empty()
            || elapsed > tuning().engine_patience
            || (!engines_working && !towers_rolling && elapsed > 20.0));

    // Engines: concentrate on the weakest front wall (SG4: the nearest of
    // equally battered ones), then shoot the wall walk.
    let engine_center = view.centroid(&engines);
    let target_piece = front
        .iter()
        .copied()
        .filter(|&p| works.pieces[p].intact())
        .min_by(|&a, &b| {
            let score = |p: usize| {
                let d = engine_center.map_or(0.0, |(x, z)| works.pieces[p].distance(x, z));
                works.pieces[p].hp + d * tuning().engine_target_hp_per_m
            };
            score(a).total_cmp(&score(b)).then(a.cmp(&b))
        });
    for &i in &engines {
        let unit = &units[i];
        let range = f64::from(unit.stats.range) * view.sim.range_factor();
        match target_piece {
            Some(p) if openings.len() < 2 => {
                let d = works.pieces[p].distance(unit.x, unit.z);
                if d > range * 0.95 {
                    let (x, z) = outer_point(works, p, range * 0.8);
                    view.move_to(i, x, z, false, None);
                } else {
                    if unit.wall_target != Some(p) {
                        view.commands.push(Command::TargetWall {
                            units: vec![unit.id],
                            piece: p,
                        });
                    }
                    view.halt(i);
                }
            }
            _ => {
                if let Some((j, _)) = view.nearest_enemy(i, |e| e.on_wall) {
                    view.attack(i, j, false);
                }
            }
        }
    }
    // Ram: to the gate while it stands.
    for &i in rams.iter().filter(|&&i| units[i].able()) {
        if gate_intact {
            let (x, z) = outer_point(works, works.gate, band + 1.0);
            view.move_to(i, x, z, false, None);
        } else {
            let (x, z) = outer_point(works, works.gate, 60.0);
            view.move_to(i, x + 25.0, z, false, None);
        }
    }
    // SG4: a ram short of men (or abandoned): the nearest free foot
    // regiment comes to take it over (the core passes its men to the ram,
    // `siege_works.json` `ram.relief_*`), standing behind it, clear of the
    // boiling oil.
    let mut relief: Vec<usize> = Vec::new();
    if gate_intact {
        for &r in &rams {
            let ram = &units[r];
            if ram.hp >= f64::from(ram.initial_soldiers) * tuning().ram_relief_crew {
                continue;
            }
            let (nx, nz) = works.pieces[works.gate].outward();
            let spot = (
                ram.x + nx * tuning().ram_relief_stand,
                ram.z + nz * tuning().ram_relief_stand,
            );
            let donor = own
                .iter()
                .copied()
                .filter(|&i| {
                    let u = &units[i];
                    u.can_climb()
                        && view.free(i)
                        && !u.on_wall
                        && !works.inside(u.x, u.z)
                        && !relief.contains(&i)
                })
                .min_by(|&a, &b| {
                    dist_to(&units[a], spot.0, spot.1)
                        .total_cmp(&dist_to(&units[b], spot.0, spot.1))
                        .then(a.cmp(&b))
                });
            if let Some(i) = donor {
                relief.push(i);
                view.move_to(i, spot.0, spot.1, true, None);
            }
        }
    }
    // Towers: one per front wall.
    let intact_front: Vec<usize> = front
        .iter()
        .copied()
        .filter(|&p| works.pieces[p].intact())
        .collect();
    for (k, &i) in towers.iter().enumerate() {
        if intact_front.is_empty() {
            break;
        }
        let p = intact_front[k % intact_front.len()];
        let (x, z) = outer_point(works, p, band + 2.0);
        view.move_to(i, x, z, false, None);
    }
    // SG4: ladders on several stretches of the front at once (one regiment
    // per stretch, nearest first), the stretches the garrison holds in
    // strength (more than twice the average) last: dilute the defence.
    let held = |p: usize| -> f64 {
        view.able_enemies()
            .filter(|&j| {
                units[j].on_wall && works.pieces[p].distance(units[j].x, units[j].z) < 30.0
            })
            .map(|j| units[j].hp)
            .sum()
    };
    let held_by: Vec<f64> = intact_front.iter().map(|&p| held(p)).collect();
    let average = held_by.iter().sum::<f64>() / held_by.len().max(1) as f64;
    let strong = |k: usize| average > 0.0 && held_by[k] > 2.0 * average;
    let ladder_pieces: Vec<usize> = (0..intact_front.len())
        .filter(|&k| !strong(k))
        .chain((0..intact_front.len()).filter(|&k| strong(k)))
        .map(|k| intact_front[k])
        .collect();

    // Foot and horse.
    let waiting_z = front_z(works) - 250.0;
    let square = works.center;
    let mut ladder_slot = 0usize;
    let mut tower_slot = 0usize;
    // T4: the melee regiments gather inside before they march on the square
    // together (no regiment thrown alone at the garrison's last stand).
    let assault = &crate::capture::CaptureRules::bundled().assault;
    let melee: Vec<usize> = own
        .iter()
        .copied()
        .filter(|&i| {
            let u = &units[i];
            u.able() && !is_shooter(u) && u.category != UnitCategory::Siege
        })
        .collect();
    let melee_in = melee
        .iter()
        .filter(|&&i| units[i].on_wall || works.inside(units[i].x, units[i].z))
        .count();
    let committed = melee
        .iter()
        .any(|&i| dist_to(&units[i], square.0, square.1) < assault.committed_radius_m);
    let gathered = committed || melee_in as f64 >= assault.gather_share * melee.len() as f64 - 1e-9;
    for &i in &own {
        let unit = &units[i];
        if unit.category == UnitCategory::Siege || !view.free(i) || relief.contains(&i) {
            continue;
        }
        if is_shooter(unit) {
            // Duel with the wall walk, out of the melee.
            if let Some((j, d)) = view.nearest_enemy(i, |e| e.on_wall) {
                let e = &units[j];
                let range = view.sim.effective_range(unit, e.x, e.z);
                if d <= range * 0.9 {
                    view.halt(i);
                } else {
                    let (dx, dz) = ((e.x - unit.x) / d, (e.z - unit.z) / d);
                    let step = d - range * 0.8;
                    view.move_to(i, unit.x + dx * step, unit.z + dz * step, false, None);
                }
            } else if storm {
                view.move_to(i, square.0, square.1, false, None);
            }
            continue;
        }
        // Enemies within reach (no wall in between): fight them.
        if let Some((j, d)) = view.nearest_enemy(i, |e| e.state != UnitState::Routing) {
            if d < 50.0 && view.reachable(i, j) {
                view.attack(i, j, true);
                continue;
            }
        }
        // SG4 (BR3b too): over the wall or inside the town: take the square
        // (never wait at the ladders); enemies on the way are fought as they
        // come close.
        if unit.on_wall || works.inside(unit.x, unit.z) {
            let stage = (!gathered && !unit.on_wall)
                .then(|| works.best_opening((unit.x, unit.z), square))
                .flatten()
                .map(|p| {
                    // Clear of the gap, towards the square.
                    let (mx, mz) = works.pieces[p].midpoint();
                    let (dx, dz) = (square.0 - mx, square.1 - mz);
                    let d = dx.hypot(dz).max(1.0);
                    let k = (35.0 / d).min(0.5);
                    (mx + dx * k, mz + dz * k)
                });
            match stage {
                Some((x, z)) => view.move_to(i, x, z, false, None),
                None => view.move_to(i, square.0, square.1, true, None),
            }
            continue;
        }
        if storm {
            // SG4: through the breach or the gate as soon as it opens.
            view.move_to(i, square.0, square.1, true, None);
        } else if (escalade || storm) && unit.can_climb() {
            // Climb at a docked tower if any (one regiment per tower at a
            // time), else ladders on the least-held stretches of the front.
            let piece = if let Some(&p) = docked.get(tower_slot) {
                tower_slot += 1;
                p
            } else if !ladder_pieces.is_empty() {
                let p = ladder_pieces[ladder_slot % ladder_pieces.len()];
                ladder_slot += 1;
                p
            } else {
                works.gate
            };
            let (x, z) = if let Some(t) = works.pieces[piece].docked_tower {
                let t = &units[t as usize];
                let (nx, nz) = works.pieces[piece].outward();
                (t.x - nx * 25.0, t.z - nz * 25.0)
            } else {
                outer_point(works, piece, -25.0)
            };
            view.move_to(i, x, z, true, None);
        } else {
            // Wait out of bowshot for the engines, towers and ram.
            view.move_to(i, unit.x, waiting_z.min(unit.z), false, Some(0.0));
        }
    }
}

pub(super) fn front_z(works: &SiegeWorks) -> f64 {
    works
        .vertices
        .iter()
        .map(|v| v.1)
        .fold(f64::INFINITY, f64::min)
}

/// SG1: the `k`-th rallying point of the garrison on the central square.
pub(super) fn square_point(works: &SiegeWorks, k: usize) -> (f64, f64) {
    let angle = k as f64 * 2.4;
    let r = works.square_radius * 0.45;
    (
        works.center.0 + r * angle.sin(),
        works.center.1 + r * angle.cos(),
    )
}

pub(super) fn plan_siege_defence(view: &mut View, works: &SiegeWorks) {
    let units = view.units;
    let band = works.band();
    let own = view.own.clone();
    let openings: Vec<usize> = works.openings();
    // Attackers on the wall walk or inside the town.
    let inside: Vec<usize> = view
        .able_enemies()
        .filter(|&j| {
            let e = &units[j];
            e.on_wall || works.inside(e.x, e.z)
        })
        .collect();
    let climbers: Vec<usize> = view
        .able_enemies()
        .filter(|&j| units[j].climbing.is_some())
        .collect();
    let mut blockers = 0usize;
    let gate_down = !works.pieces[works.gate].intact();
    // T4 (ADR 0108): the garrison falls back on the square at the first
    // breach (not only when the gate falls), leaving a few regiments in the
    // openings; fallen back, it only charges attackers near the square.
    let fall_back = &crate::capture::CaptureRules::bundled().fall_back;
    let fallen_back = gate_down || (fall_back.on_breach && !openings.is_empty());
    // The broken gate (a gatehouse to hold) keeps `gate_blockers`
    // regiments while it is the only way in; a breach (rubble) keeps
    // `breach_blockers` and, once the walls are breached, the gate too:
    // `max_blockers` in all.
    let breached = openings.iter().any(|&p| p != works.gate);
    let blocked: Vec<usize> = openings
        .iter()
        .flat_map(|&p| {
            let n = if p == works.gate && !breached {
                fall_back.gate_blockers
            } else {
                fall_back.breach_blockers
            };
            std::iter::repeat_n(p, n)
        })
        .take(fall_back.max_blockers)
        .collect();
    let max_blockers = blocked.len();
    let near_square = |j: usize| {
        fall_back.on_breach
            && dist_to(&units[j], works.center.0, works.center.1) < fall_back.engage_radius_m
    };
    let mut square_slot = 0usize;
    for &i in &own {
        let unit = &units[i];
        if !view.free(i) {
            continue;
        }
        if is_shooter(unit) && unit.on_wall {
            // Hold the wall walk; shoot at will. Defend against climbers
            // only with the melee troops.
            view.halt(i);
            continue;
        }
        let near_inside = inside
            .iter()
            .copied()
            .filter(|&j| view.reachable(i, j))
            .map(|j| (j, dist(unit, &units[j])))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        if unit.on_wall {
            // Wall foot: throw back climbers and attackers on the walls nearby.
            let near_climber = climbers
                .iter()
                .chain(inside.iter())
                .copied()
                .map(|j| (j, dist(unit, &units[j])))
                .filter(|&(_, d)| d < 70.0)
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            if let Some((j, _)) = near_climber {
                view.attack(i, j, false);
            } else if fallen_back {
                // SG1: the gate is down (T4: or a breach open), nobody
                // climbs here: come down and regroup on the square.
                let k = square_slot;
                square_slot += 1;
                let (x, z) = square_point(works, k);
                view.move_to(i, x, z, true, None);
            } else {
                view.halt(i);
            }
            continue;
        }
        if let Some((j, d)) = near_inside {
            let engage = if fallen_back && fall_back.on_breach {
                d < 40.0 || near_square(j)
            } else {
                d < 250.0
            };
            if engage || is_horse(unit) {
                view.attack(i, j, true);
                continue;
            }
        }
        // Block the openings from inside, one regiment per opening first;
        // SG1: beyond the blockers of each opening (T4: `gate_blockers`,
        // `breach_blockers`, `max_blockers`), the foot regroups on the central square
        // (and holds it) instead of crowding the gap.
        if !openings.is_empty() && blockers >= max_blockers && !is_horse(unit) {
            if !works.in_square(unit.x, unit.z) {
                let k = square_slot;
                square_slot += 1;
                let (x, z) = square_point(works, k);
                view.move_to(i, x, z, true, None);
            }
            continue;
        }
        if !blocked.is_empty() {
            let p = blocked[blockers % blocked.len()];
            blockers += 1;
            let (x, z) = outer_point(works, p, -(band + 22.0));
            let (nx, nz) = works.pieces[p].outward();
            if let Some((j, d)) = view.nearest_enemy(i, |e| e.state != UnitState::Routing) {
                if d < 45.0 && view.reachable(i, j) {
                    view.attack(i, j, true);
                    continue;
                }
            }
            view.move_to(i, x, z, true, Some(nx.atan2(nz)));
            continue;
        }
        // Attackers on the walls: send the reserve.
        if let Some(&j) = climbers.first() {
            if !is_horse(unit) {
                let p = units[j].climbing.expect("climbing");
                let (x, z) = outer_point(works, p, -(band + 6.0));
                view.move_to(i, x, z, true, None);
                continue;
            }
        }
        // Otherwise: the gate guard stays, the reserve holds the square.
        let gate_guard = dist_to(
            unit,
            works.pieces[works.gate].midpoint().0,
            works.pieces[works.gate].midpoint().1,
        ) < 60.0;
        if !gate_guard && !works.in_square(unit.x, unit.z) && is_horse(unit) {
            view.move_to(i, works.center.0, works.center.1 + 25.0, false, None);
        }
    }
}
