//! Defensive cover, river holds and crossings.

use super::*;

/// What a defensive side leans on (B6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverKind {
    Obstacle(crate::site::ObstacleKind),
    Village,
    /// EP3: the bank of the river at a crossing.
    River,
}

/// EP3: the bank a defensive `side` holds when the river lies between its
/// deployment line and the enemy (`enemy`: its centroid): the crossing the
/// enemy would take (cheapest from the enemy to the deployment line), its
/// own-side end, shooters `BattleAiRules::bank_setback` back from the water, along the
/// river.
pub fn river_hold(
    field: &crate::field::Battlefield,
    side: SideId,
    enemy: (f64, f64),
) -> Option<Cover> {
    let river = field.river.as_ref()?;
    let home = deployment_center(field, side);
    if !ReliefMap::river_between(field, home, enemy) {
        return None;
    }
    let own_north = river.north_of(home.0, home.1);
    let crossing = field
        .crossings()
        .into_iter()
        .map(|c| {
            let cost = ReliefMap::crossing_cost(field, enemy, &c, home, 40.0);
            (c, cost)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))?
        .0;
    let end = crossing.end(own_north);
    if (end.0 - home.0).hypot(end.1 - home.1) > tuning().river_reach {
        return None;
    }
    let x = end.0;
    let away = if own_north { 1.0 } else { -1.0 };
    let mut center = (
        x,
        river.center_z(x) + away * (river.width_at(x) * 0.5 + tuning().bank_setback),
    );
    // Off the bridge and the road ramp itself: a little aside when needed.
    if field.water_at(center.0, center.1).is_some() {
        center.1 += away * 10.0;
    }
    let slope = river.slope(x);
    let n = (1.0 + slope * slope).sqrt();
    Some(Cover {
        kind: CoverKind::River,
        center,
        along: (1.0 / n, slope / n),
        width: 110.0,
        breaks_charge: true,
    })
}

/// EP3: how an advancing side crosses the river.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossingPlan {
    /// End of the crossing on the own bank, and on the enemy's.
    pub near: (f64, f64),
    pub far: (f64, f64),
    /// Enemy shooters covering the far end.
    pub covered: usize,
    pub bridge: bool,
}

/// EP3: the crossing an advancing side takes from `from` towards the enemy
/// (march with the relief, width filed through, enemy shooters covering the
/// far end); `None` when the river does not lie between.
pub(super) fn crossing_plan(view: &View, from: (f64, f64), line: &[usize]) -> Option<CrossingPlan> {
    let field = view.sim.field();
    let river = field.river.as_ref()?;
    let able: Vec<usize> = view.able_enemies().collect();
    let enemy = view.centroid(&able)?;
    if !ReliefMap::river_between(field, from, enemy) {
        return None;
    }
    let frontage = line
        .iter()
        .map(|&i| view.units[i].extent().0)
        .fold(20.0, f64::max);
    let weather = view.sim.range_factor();
    let foes: Vec<(f64, f64, f64)> = able
        .iter()
        .filter(|&&k| is_shooter(&view.units[k]))
        .map(|&k| {
            let e = &view.units[k];
            (e.x, e.z, f64::from(e.stats.range) * weather)
        })
        .collect();
    let north = river.north_of(from.0, from.1);
    view.sim
        .crossings()
        .iter()
        .map(|c| {
            let far = c.end(!north);
            let covered = ReliefMap::covered(far, &foes);
            let cost = ReliefMap::crossing_cost(field, from, c, enemy, frontage)
                + tuning().crossing_exposure * covered as f64;
            (
                CrossingPlan {
                    near: c.end(north),
                    far,
                    covered,
                    bridge: c.bridge.is_some(),
                },
                cost,
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(plan, _)| plan)
}

/// EP3: in the river or on a bridge (a regiment finishing its crossing).
pub(super) fn crossing_now(view: &View, i: usize) -> bool {
    let (u, field) = (&view.units[i], view.sim.field());
    field.water_kind(u.x, u.z).is_some() || field.bridge_at(u.x, u.z).is_some()
}

/// A defensive position drawn from the site (B6): the front the shooters
/// hold, just behind a hedge, a ditch, a fence, or inside the edge of a
/// village facing the enemy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cover {
    pub kind: CoverKind,
    /// Centre of the shooters' front (x, z), behind the obstacle.
    pub center: (f64, f64),
    /// Unit vector along the front.
    pub along: (f64, f64),
    /// Length of the covered front, in metres.
    pub width: f64,
    /// The obstacle (or the village) breaks cavalry charges.
    pub breaks_charge: bool,
}

impl Cover {
    /// z of the front at `x` (the front follows the obstacle's slope).
    pub(super) fn z_at(&self, x: f64) -> f64 {
        if self.along.0.abs() < 1e-6 {
            return self.center.1;
        }
        self.center.1 + (x - self.center.0) * self.along.1 / self.along.0
    }
}

/// Best cover for `side` within reach of its deployment line, if any
/// (deterministic: obstacles in index order, strict improvements only).
pub fn defensive_cover(field: &crate::field::Battlefield, side: SideId) -> Option<Cover> {
    let mut best: Option<(Cover, f64)> = None;
    for (cover, score) in cover_candidates(field, side) {
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((cover, score));
        }
    }
    best.map(|(cover, _)| cover)
}

/// Every cover `side` may lean on within reach of its deployment line, with
/// its B6 score (weight x length minus distance; the village edge last).
pub(super) fn cover_candidates(
    field: &crate::field::Battlefield,
    side: SideId,
) -> Vec<(Cover, f64)> {
    let (line_z, forward) = match side {
        SideId::Attacker => (field.attacker_line_z(), 1.0),
        SideId::Defender => (field.defender_line_z(), -1.0),
    };
    let reference = (field.size.center_x(), line_z);
    let within = |x: f64, z: f64| {
        let ahead = (z - reference.1) * forward;
        (x - reference.0).abs() <= tuning().cover_lateral
            && (-tuning().cover_behind..=tuning().cover_ahead).contains(&ahead)
    };
    let standable = |x: f64, z: f64| {
        field.inside(x, z)
            && !field.in_forest(x, z)
            && !field.in_mud(x, z)
            && field.water_at(x, z).is_none()
    };
    let penalty =
        |x: f64, z: f64| 0.25 * (x - reference.0).abs() + 0.3 * ((z - reference.1) * forward).abs();
    let mut found: Vec<(Cover, f64)> = Vec::new();
    for obstacle in &field.obstacles {
        let len = obstacle.length();
        if len < 30.0 {
            continue;
        }
        let along = (
            (obstacle.b.0 - obstacle.a.0) / len,
            (obstacle.b.1 - obstacle.a.1) / len,
        );
        // The front must face the enemy (roughly across the field).
        if along.0.abs() < 0.7 {
            continue;
        }
        let along = if along.0 < 0.0 {
            (-along.0, -along.1)
        } else {
            along
        };
        let weight = crate::position::obstacle_weight(obstacle.kind);
        let mid = (
            (obstacle.a.0 + obstacle.b.0) * 0.5,
            (obstacle.a.1 + obstacle.b.1) * 0.5,
        );
        // R4: on a crest, the shooters stand closer to the hedge when the
        // ground just behind it would hide the glacis from them.
        // (Ground seen in front of the hedge, from right below it to bowshot.)
        let seen = |back: f64| {
            let spot = (mid.0, mid.1 - forward * back);
            [10.0, 20.0, 40.0, 70.0, 100.0, 150.0]
                .iter()
                .filter(|&&d| {
                    crate::relief_ai::ReliefMap::sees(field, spot, (mid.0, mid.1 + forward * d))
                })
                .count()
        };
        let mut setback = tuning().cover_setback;
        let mut most = seen(tuning().cover_setback);
        for back in tuning().cover_setbacks {
            let n = seen(back);
            if n > most {
                (setback, most) = (back, n);
            }
        }
        let center = (mid.0, mid.1 - forward * setback);
        if !within(mid.0, mid.1) || !standable(center.0, center.1) {
            continue;
        }
        // A croft hedge with the houses in front of it would mask the
        // shooting.
        let masked = field.village.as_ref().is_some_and(|v| {
            (v.zone.z - mid.1) * forward > 0.0 && (v.zone.x - mid.0).abs() < v.zone.radius
        });
        if masked {
            continue;
        }
        let score = weight * len.min(120.0) - penalty(mid.0, mid.1);
        if score > tuning().cover_threshold {
            found.push((
                Cover {
                    kind: CoverKind::Obstacle(obstacle.kind),
                    center,
                    along,
                    width: len,
                    breaks_charge: obstacle.kind.breaks_charge(),
                },
                score,
            ));
        }
    }
    if let Some(village) = &field.village {
        let zone = village.zone;
        let edge = (
            zone.x,
            zone.z + forward * (zone.radius - tuning().village_setback).max(0.0),
        );
        if within(edge.0, edge.1) && standable(edge.0, edge.1) {
            let width = (zone.radius * 1.4).max(30.0);
            let score = width.min(120.0) - penalty(edge.0, edge.1);
            if score > tuning().cover_threshold {
                found.push((
                    Cover {
                        kind: CoverKind::Village,
                        center: edge,
                        along: (1.0, 0.0),
                        width,
                        breaks_charge: true,
                    },
                    score,
                ));
            }
        }
    }
    // EP6: the edge of a hamlet, churchyard, manor or farm of the decor
    // facing the enemy (walls, hedges and houses: cover like the village).
    for area in &field.decor.areas {
        let effect = area.effect();
        if effect.cover > tuning().village_cover_max || !effect.breaks_charge {
            continue;
        }
        let fp = area.footprint();
        // Half extent of the area along z (towards the enemy).
        let (ax, az) = fp.axis();
        let (fx, fz) = fp.front();
        let half_z = fp.half_length * az.abs() + fp.half_depth * fz.abs();
        let half_x = fp.half_length * ax.abs() + fp.half_depth * fx.abs();
        let edge = (
            area.x,
            area.z + forward * (half_z - tuning().village_setback).max(0.0),
        );
        if !within(edge.0, edge.1) || !standable(edge.0, edge.1) || !area.contains(edge.0, edge.1) {
            continue;
        }
        let width = (half_x * 1.6).max(30.0);
        let score = width.min(120.0) * (1.0 - effect.cover) * 2.0 - penalty(edge.0, edge.1);
        if score > tuning().cover_threshold {
            found.push((
                Cover {
                    kind: CoverKind::Village,
                    center: edge,
                    along: (1.0, 0.0),
                    width,
                    breaks_charge: true,
                },
                score,
            ));
        }
    }
    found
}

/// Slots of the shooters along the cover front, in lateral order (B6).
pub(super) fn cover_slots(
    view: &View,
    shooters: &[usize],
    cover: &Cover,
) -> Vec<(usize, f64, f64)> {
    let mut order: Vec<usize> = shooters.to_vec();
    order.sort_by(|&a, &b| view.units[a].x.total_cmp(&view.units[b].x).then(a.cmp(&b)));
    let widths: Vec<f64> = order
        .iter()
        .map(|&i| view.units[i].extent().0 + 6.0)
        .collect();
    let total: f64 = widths.iter().sum();
    let mut offset = -total * 0.5;
    order
        .iter()
        .zip(widths)
        .map(|(&i, w)| {
            let lateral = offset + w * 0.5;
            offset += w;
            let x = cover.center.0 + cover.along.0 * lateral;
            (i, x, cover.z_at(x))
        })
        .collect()
}
