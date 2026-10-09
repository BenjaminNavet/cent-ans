//! Role assignment and defensive ground choice.

use super::*;

pub(super) struct Roles {
    pub(super) line: Vec<usize>,
    pub(super) reserve: Option<usize>,
    pub(super) shooters: Vec<usize>,
    pub(super) horse: Vec<usize>,
    pub(super) engines: Vec<usize>,
}

pub(super) fn roles(view: &View) -> Roles {
    let u = view.units;
    let mut line: Vec<usize> = view
        .own
        .iter()
        .copied()
        .filter(|&i| {
            let unit = &u[i];
            !unit.mounted
                && (unit.category == UnitCategory::Infantry
                    || (unit.category == UnitCategory::Ranged && unit.ammo == 0))
        })
        .collect();
    let reserve = if line.len() >= 4 {
        // The rearmost foot regiment (highest id on ties) is the reserve.
        let pick = *line
            .iter()
            .max_by(|&&a, &&b| {
                (-u[a].z * view.forward)
                    .total_cmp(&(-u[b].z * view.forward))
                    .then(a.cmp(&b))
            })
            .expect("non-empty");
        line.retain(|&i| i != pick);
        Some(pick)
    } else {
        None
    };
    Roles {
        line,
        reserve,
        shooters: view
            .own
            .iter()
            .copied()
            .filter(|&i| is_shooter(&u[i]))
            .collect(),
        horse: view
            .own
            .iter()
            .copied()
            .filter(|&i| is_horse(&u[i]) && u[i].category != UnitCategory::Siege)
            .collect(),
        engines: view
            .own
            .iter()
            .copied()
            .filter(|&i| u[i].category == UnitCategory::Siege)
            .collect(),
    }
}

/// R4: the ground a defensive side takes: the best of its deployment spot,
/// the heights around it (R2b) and the covers of the site (B6), all scored
/// by [`score_position`] (height, glacis, reverse slope, cover, flanks): a
/// hedge on a crest beats a bare crest and a hedge in a hollow. Returns the
/// front the shooters hold (a crest, or the line behind a cover) and the
/// cover when it is one.
///
/// R2b: the relief is read, not only the height: a true crest (ground above
/// its surroundings) and a glacis in front (ground the enemy must climb)
/// are worth more than a gentle tilt, and a scarp too steep to stand on in
/// order is avoided.
pub(super) fn defensive_ground(view: &View, roles: &Roles) -> ((f64, f64), Option<Cover>) {
    let field = view.sim.field();
    let map = view.sim.relief_map();
    let around = deployment_center(field, view.side);
    let base = field.height(around.0, around.1);
    let width = front_width(view, roles);
    let reverse = view.able_enemies().any(|j| is_shooter(&view.units[j]));
    let score = |center: (f64, f64)| {
        let front = Front {
            center,
            forward: view.forward,
            width,
        };
        score_position(field, map, front, base, reverse)
    };
    // R4: a position the enemy would reach first (or hardly later) is no
    // position: the line would be caught marching up to it.
    let first = race(view, roles);
    let mut best = (
        around,
        None,
        score(around).ground() + tuning().ground_margin,
    );
    for ix in -8..=8 {
        for iz in -4..=4 {
            let x = around.0 + f64::from(ix) * 25.0;
            let z = around.1 + f64::from(iz) * 25.0;
            // The centre half of the field's width (300-900 m on the standard field).
            let (center, half) = (field.size.center_x(), 300.0 * field.size.sx());
            if !(center - half..=center + half).contains(&x)
                || !(60.0..=field.depth - 60.0).contains(&z)
            {
                continue;
            }
            // Never step towards the enemy to find a hill.
            if (z - around.1) * view.forward > 30.0 {
                continue;
            }
            // R4: the shooters of a bare crest stand on its military crest:
            // its field of fire is theirs.
            let mut part = score((x, z));
            let front = Front {
                center: (x, z),
                forward: view.forward,
                width,
            };
            part.fire = crate::position::fire_points(
                field,
                Front {
                    center: military_crest(field, front),
                    ..front
                },
            );
            let value = part.ground() - (x - around.0).abs() * 0.01;
            if value > best.2 + 0.5
                && !field.in_forest(x, z)
                && !field.in_mud(x, z)
                && first((x, z))
            {
                best = ((x, z), None, value);
            }
        }
    }
    for (cover, _) in cover_candidates(field, view.side) {
        let (x, z) = cover.center;
        let value = score(cover.center).total()
            - tuning().cover_lateral_cost * (x - around.0).abs()
            - tuning().cover_depth_cost * (z - around.1).abs();
        if value > best.2 && first(cover.center) {
            best = (cover.center, Some(cover), value);
        }
    }
    (best.0, best.1)
}

/// R4: can the regiments that hold the front (the shooters, else the line;
/// at the pace of the slowest, uphill slowed as in the simulation) reach a
/// spot from their deployment line well before the enemy's battle line (its
/// foot, at the pace of the slowest; the AI's horse waits for its line)
/// from its own? Both measured from the deployment lines, so that the
/// choice holds all battle long instead of flipping as the enemy nears.
pub(super) fn race<'v>(view: &'v View, roles: &Roles) -> impl Fn((f64, f64)) -> bool + 'v {
    let field = view.sim.field();
    let walkers: &[usize] = if roles.shooters.is_empty() {
        &roles.line
    } else {
        &roles.shooters
    };
    let from = deployment_center(field, view.side);
    let enemy_from = deployment_center(field, view.side.other());
    let own_speed = walkers
        .iter()
        .map(|&i| f64::from(view.units[i].stats.speed))
        .fold(f64::INFINITY, f64::min);
    let enemy_foot = view
        .able_enemies()
        .filter(|&j| !view.units[j].mounted)
        .map(|j| f64::from(view.units[j].stats.speed))
        .fold(f64::INFINITY, f64::min);
    let enemy_speed = if enemy_foot.is_finite() {
        enemy_foot
    } else {
        view.able_enemies()
            .map(|j| f64::from(view.units[j].stats.speed))
            .fold(1.0, f64::max)
    }
    .max(1.0);
    move |spot: (f64, f64)| {
        if !own_speed.is_finite() || own_speed <= 0.0 {
            return true;
        }
        let ours = ReliefMap::march_cost(field, from, spot) / own_speed;
        let theirs = (enemy_from.0 - spot.0).hypot(enemy_from.1 - spot.1) / enemy_speed;
        ours < tuning().race_margin * theirs
    }
}

/// Width of the line of a side (the front whose flanks and cover count).
pub(super) fn front_width(view: &View, roles: &Roles) -> f64 {
    let list = if roles.line.is_empty() {
        &view.own
    } else {
        &roles.line
    };
    list.iter()
        .map(|&i| view.units[i].extent().0 + 10.0)
        .sum::<f64>()
        .max(40.0)
}

/// R2b: centre of the deployment line of `side`, from which a defensive
/// side looks for its ground (a fixed reference: searching from the moving
/// line would let the reverse slope drag the line back step after step).
/// EP3: read from the field's dimensions (never a fixed 1200 × 800 m).
pub(super) fn deployment_center(field: &crate::field::Battlefield, side: SideId) -> (f64, f64) {
    match side {
        SideId::Attacker => (field.size.center_x(), field.attacker_line_z()),
        SideId::Defender => (field.size.center_x(), field.defender_line_z()),
    }
}

/// R2b: the line of a defensive side on a crest steps back onto the reverse
/// slope, out of sight, when the enemy has shooters and its own shooters
/// hold the crest in front of it. R4: longbows too, since a volley at a
/// target out of sight must be directed by a friend who sees it and
/// scatters (ADR 0046).
pub(super) fn reverse_slope_anchor(view: &View, crest: (f64, f64), shooters: bool) -> (f64, f64) {
    let front = Front {
        center: crest,
        forward: view.forward,
        width: 0.0,
    };
    if !shooters {
        // R4: a line without shooters sees its glacis from the military
        // crest.
        return military_crest(view.sim.field(), front);
    }
    if !view.able_enemies().any(|j| is_shooter(&view.units[j])) {
        return crest;
    }
    crate::position::reverse_slope(view.sim.field(), front).unwrap_or(crest)
}
