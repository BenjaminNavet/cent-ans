//! Missile troops: firing spots and shooter plans.

use super::*;

/// `cover`: the slot of the shooter behind the site's cover (B6); `crest`:
/// the bare crest the side holds (R4, military crest).
#[allow(clippy::too_many_arguments)]
pub(super) fn plan_shooter(
    view: &mut View,
    i: usize,
    anchor: (f64, f64),
    line_z: f64,
    facing: f64,
    defensive: bool,
    cover: Option<((f64, f64), Cover)>,
    crest: Option<(f64, f64)>,
) {
    let unit = &view.units[i];
    // B6: behind a hedge, a ditch or in a village, horsemen are no threat
    // (their charge breaks on it).
    let threat = match cover {
        Some((_, c)) if c.breaks_charge => {
            view.nearest_enemy(i, |e| is_melee_troop(e) && !is_horse(e))
        }
        // R4: on the military crest of a defensive position, behind
        // planted stakes, horsemen in front are no threat either (their
        // charge breaks on the stakes; Crécy, Agincourt): the archers hold
        // the crest and keep shooting instead of falling back out of sight.
        _ if defensive && crest.is_some() && unit.stakes_planted => view.nearest_enemy(i, |e| {
            is_melee_troop(e) && (!is_horse(e) || attack_angle(unit, e.x, e.z) != 0)
        }),
        _ => view.nearest_enemy(i, is_melee_troop),
    };
    // Engaged or about to be: fall back behind the line.
    if let Some((_, d)) = threat {
        // R4: behind a hedge, a ditch or houses, the foot must cross them
        // too: the shooters keep shooting until it is close.
        // Behind their stakes on the military crest too: falling back would
        // leave the glacis out of sight.
        let safety = match cover {
            Some((_, c)) if c.breaks_charge => tuning().cover_safety,
            _ if defensive && crest.is_some() && unit.stakes_planted => {
                tuning().crest_stakes_safety
            }
            _ => tuning().shooter_safety,
        };
        if d < safety || view.engaged(i) {
            let rear = (line_z * view.forward).min(anchor.1 * view.forward) * view.forward;
            let behind = (unit.x, rear - view.forward * 45.0);
            if (unit.z - behind.1) * view.forward > 8.0 || view.engaged(i) {
                let (x, z) =
                    view.sim
                        .field()
                        .clamp_inside(behind.0, behind.1, tuning().field_margin);
                view.commands.push(Command::Move {
                    units: vec![unit.id],
                    x,
                    z,
                    run: true,
                    facing: Some(facing),
                    queue: false,
                    width: None,
                    match_speed: false,
                    group_tag: None,
                });
            }
            return;
        }
    }
    if !view.free(i) {
        return;
    }
    let front_z = anchor.1 + view.forward * tuning().shooters_ahead;
    if let Some((j, d)) = view.nearest_enemy(i, |_| true) {
        let e = &view.units[j];
        let range = view.sim.effective_range(unit, e.x, e.z);
        if let Some(((x, z), c)) = cover {
            // EP3: on the way to the bank, shoot as soon as in range.
            if c.kind == CoverKind::River && d <= range * 0.95 && dist_to(unit, x, z) < 60.0 {
                view.halt(i);
                return;
            }
            // B6: reach the cover first (the threat above sends them back
            // when the enemy closes in).
            // R4: right up to the slot (6 m short, behind a hedge on a
            // crest, would leave the glacis in dead ground).
            if dist_to(unit, x, z) > 2.0 {
                // R4: at the run when the enemy comes on.
                view.move_to(i, x, z, false, Some(facing));
            } else {
                view.halt(i);
            }
            return;
        }
        // R4: holding a crest, the shooters stand on its military crest
        // (never behind the usual post in front of the line), where they see
        // the glacis, rather than halting wherever a target first comes in
        // range.
        // A crest that sees its glacis needs none of this.
        let post = crest.filter(|_| defensive).and_then(|(_, cz)| {
            let front = Front {
                center: (unit.x, cz),
                forward: view.forward,
                width: unit.extent().0,
            };
            let mc = military_crest(view.sim.field(), front).1;
            (mc != cz).then_some(mc)
        });
        if let Some(mc) = post {
            let z = if (mc - front_z) * view.forward > 0.0 {
                mc
            } else {
                front_z
            };
            if (unit.z - z).abs() > 6.0 {
                // At the run when the enemy comes on: the crest must be
                // held before it arrives.
                let run = d < tuning().post_run;
                view.move_to(i, unit.x, z, run, Some(facing));
            } else {
                view.halt(i);
            }
            return;
        }
        // R4: in range and able to shoot it (in sight, or lobbing at a
        // target a friend sees).
        if d <= range * 0.95 && view.sim.fire_mode(unit, e).is_some() {
            view.halt(i);
            return;
        }
        if defensive {
            view.move_to(i, unit.x, front_z, false, Some(facing));
            return;
        }
        // Advance to shooting range, never far ahead of the line. R2b:
        // to a spot from which the target is seen, preferably higher and
        // out of reach of the enemy shooters.
        let limit = front_z + view.forward * 40.0;
        if let Some((x, z)) = firing_spot(view, i, j, limit) {
            view.move_to(i, x, z, false, None);
            return;
        }
        let wanted = (unit.z * view.forward + d - range * 0.85).min(front_z * view.forward + 40.0)
            * view.forward;
        if (wanted - unit.z) * view.forward > 5.0 {
            view.move_to(i, unit.x, wanted, false, None);
        }
    }
}

/// R2b: where shooter `i` should stand to shoot `j`, marching towards it no
/// farther than `limit` (z): a spot within its range (height counted), from
/// which it sees the target, preferably out of
/// reach of the enemy shooters and higher than the target; `None` when no
/// such spot lies ahead.
pub(super) fn firing_spot(view: &View, i: usize, j: usize, limit: f64) -> Option<(f64, f64)> {
    let field = view.sim.field();
    let (u, t) = (&view.units[i], &view.units[j]);
    let weather = view.sim.range_factor();
    let reach = f64::from(u.stats.range) * weather;
    let ht = field.height(t.x, t.z);
    let foes: Vec<(f64, f64, f64, f64)> = view
        .able_enemies()
        .filter(|&k| is_shooter(&view.units[k]))
        .map(|k| {
            let e = &view.units[k];
            (
                e.x,
                e.z,
                field.height(e.x, e.z),
                f64::from(e.stats.range) * weather,
            )
        })
        .collect();
    let (dx, dz) = (t.x - u.x, t.z - u.z);
    let d = dx.hypot(dz).max(1e-6);
    let (ux, uz) = (dx / d, dz / d);
    let mut best: Option<((f64, f64), f64)> = None;
    for lateral in tuning().firing_laterals {
        for k in 0..=30 {
            let s = f64::from(k) * 10.0;
            let c = (u.x + ux * s + uz * lateral, u.z + uz * s - ux * lateral);
            if (c.1 - limit) * view.forward > 0.0 {
                break;
            }
            if !field.inside(c.0, c.1)
                || field.in_forest(c.0, c.1)
                || field.in_mud(c.0, c.1)
                || field.water_at(c.0, c.1).is_some()
            {
                continue;
            }
            let hc = field.height(c.0, c.1);
            let range = reach * (1.0 + (hc - ht).max(0.0) / 100.0);
            if (t.x - c.0).hypot(t.z - c.1) > range * 0.93 {
                continue;
            }
            // R4: volleys too aim at what they see (a lob over a crest is a
            // last resort, directed by a friend and scattered).
            if !ReliefMap::sees(field, c, (t.x, t.z)) {
                continue;
            }
            let exposed = foes
                .iter()
                .filter(|&&(x, z, h, r)| {
                    (x - c.0).hypot(z - c.1) <= r * (1.0 + (h - hc).max(0.0) / 100.0) + 5.0
                })
                .count();
            let score = ReliefMap::march_cost(field, (u.x, u.z), c)
                + tuning().exposure_cost * exposed as f64
                - 2.0 * (hc - ht).clamp(-10.0, 10.0);
            if best.is_none_or(|(_, b)| score < b) {
                best = Some((c, score));
            }
            // Farther along this ray only costs more march.
            break;
        }
    }
    best.map(|(c, _)| c)
}
