//! Street furniture laid out by the core (lot BR3, ADR 0047): stalls on the
//! facades facing the square, barrels, carts and woodpiles on the street
//! side, the market (a well and groups of stalls leaving the streets open),
//! the props of the suburbs. Deterministic from the
//! houses (index and position, [`crate::rng::hash01`]), never from the
//! battle's random stream, so that laying them out shifts no other draw.
//!
//! A prop stands against the front of its house, back to the wall with a
//! small gap (`wall_gap_m`): it lies in the clearance that regiments keep
//! round the houses, so it never closes a street. Props of the market stand
//! on the square with passages left open in front of every street.

use crate::rng::hash01;
use crate::siege::{House, SiegeWorks};
use crate::town::{Footprint, Prop, PropKind, TownRules};

const SALT_COUNT: u64 = 0xB3_0001;
const SALT_KIND: u64 = 0xB3_0002;
const SALT_SLIDE: u64 = 0xB3_0003;
const SALT_SIDE: u64 = 0xB3_0004;
const SALT_MARKET: u64 = 0xB3_0005;

/// Key of a house for the hashed draws: its index and position.
fn key(index: usize, x: f64, z: f64) -> u64 {
    (index as u64) ^ x.to_bits().rotate_left(17) ^ z.to_bits().rotate_left(41)
}

fn pick(kinds: &[PropKind], draw: f64) -> PropKind {
    let k = ((draw * kinds.len() as f64) as usize).min(kinds.len().saturating_sub(1));
    kinds.get(k).copied().unwrap_or(PropKind::Barrels)
}

/// How many props from the cumulative chances of 0, 1, 2… props.
fn count(chances: &[f64], draw: f64) -> usize {
    let mut acc = 0.0;
    for (n, &c) in chances.iter().enumerate() {
        acc += c;
        if draw < acc {
            return n;
        }
    }
    chances.len().saturating_sub(1)
}

/// `n` props against the face of `house` (rectangle `fp`) turned towards
/// `facing` (unit vector along its front axis, ±), spread in equal slots
/// along the frontage.
#[allow(clippy::too_many_arguments)]
fn along_face(
    out: &mut Vec<Prop>,
    fp: &Footprint,
    facing_sign: f64,
    kinds: &[PropKind],
    n: usize,
    house: usize,
    seed: u64,
    rules: &TownRules,
) {
    let p = &rules.props;
    let (ax, az) = fp.axis();
    let (fx, fz) = fp.front();
    let (fx, fz) = (fx * facing_sign, fz * facing_sign);
    let slot = fp.half_length * 2.0 / n.max(1) as f64;
    for j in 0..n {
        let kind = pick(kinds, hash01(seed, SALT_KIND + j as u64));
        let size = p.size(kind);
        if size.length_m > slot {
            continue;
        }
        let slack = (slot - size.length_m) * 0.5;
        let along = -fp.half_length
            + slot * (j as f64 + 0.5)
            + (hash01(seed, SALT_SLIDE + j as u64) * 2.0 - 1.0) * slack;
        let out_by = fp.half_depth + p.wall_gap_m + size.depth_m * 0.5;
        out.push(Prop {
            kind,
            x: fp.x + ax * along + fx * out_by,
            z: fp.z + az * along + fz * out_by,
            yaw: crate::siege::yaw_facing(fx, fz),
            length: size.length_m,
            depth: size.depth_m,
            house: Some(house),
        });
    }
}

/// Keeps the props that touch no house but their own and no earlier prop.
fn keep_clear(props: Vec<Prop>, houses: &[Footprint]) -> Vec<Prop> {
    let mut kept: Vec<Prop> = Vec::new();
    for prop in props {
        let f = prop.footprint();
        let reach = f.bounding_radius();
        let hits_house = houses.iter().enumerate().any(|(i, h)| {
            Some(i) != prop.house
                && (h.x - f.x).hypot(h.z - f.z) < reach + h.bounding_radius()
                && h.distance_to(&f) < 0.3
        });
        let hits_prop = kept.iter().any(|o| {
            let g = o.footprint();
            (g.x - f.x).hypot(g.z - f.z) < reach + g.bounding_radius() && g.distance_to(&f) < 0.3
        });
        if !hits_house && !hits_prop {
            kept.push(prop);
        }
    }
    kept
}

/// Directions (angles from +z towards +x, as the town's radial streets) in
/// which a street leaves the square: the radial streets of the generic town,
/// the streets of the plan and the lane from the gate of a landmark town.
fn street_angles(works: &SiegeWorks, radius: f64) -> Vec<f64> {
    let (cx, cz) = works.center;
    let mut lines: Vec<Vec<(f64, f64)>> = match &works.landmark {
        Some(l) => l.streets.clone(),
        None => Vec::new(),
    };
    if works.landmark.is_some() {
        lines.push(vec![works.pieces[works.gate].midpoint(), works.center]);
    } else {
        lines.extend(
            works
                .pieces
                .iter()
                .map(|p| vec![works.center, p.midpoint()]),
        );
    }
    let mut angles = Vec::new();
    for line in &lines {
        for w in line.windows(2) {
            let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            let len = dx.hypot(dz);
            let steps = (len / 1.0).ceil().max(1.0) as usize;
            let mut was_inside: Option<bool> = None;
            for k in 0..=steps {
                let t = k as f64 / steps as f64;
                let (x, z) = (w[0].0 + dx * t, w[0].1 + dz * t);
                let inside = (x - cx).hypot(z - cz) < radius;
                if was_inside.is_some_and(|w| w != inside) {
                    angles.push((x - cx).atan2(z - cz));
                }
                was_inside = Some(inside);
            }
        }
    }
    angles.sort_by(f64::total_cmp);
    angles
}

/// The market: a well in the middle, groups of stalls in a ring facing the
/// centre between the streets, now and then a cart or barrels.
fn market(works: &SiegeWorks, rules: &TownRules, seed: u64) -> Vec<Prop> {
    let m = &rules.props.market;
    let (cx, cz) = works.center;
    let well = rules.props.size(PropKind::Well);
    let mut out = vec![Prop {
        kind: PropKind::Well,
        x: cx,
        z: cz,
        yaw: 0.0,
        length: well.length_m,
        depth: well.depth_m,
        house: None,
    }];
    let ring = works.square_radius * m.ring_factor;
    let stall = rules.props.size(PropKind::Stall);
    let streets = street_angles(works, ring);
    let tau = std::f64::consts::TAU;
    let half_gap = (m.passage_m * 0.5 + stall.length_m * 0.5) / ring;
    // Free arcs between the passages (the whole ring without streets).
    let mut arcs: Vec<(f64, f64)> = Vec::new();
    if streets.is_empty() {
        arcs.push((0.0, tau));
    } else {
        for (k, &a) in streets.iter().enumerate() {
            let next = streets.get(k + 1).copied().unwrap_or(streets[0] + tau);
            let (a0, a1) = (a + half_gap, next - half_gap);
            if a1 > a0 {
                arcs.push((a0, a1));
            }
        }
    }
    let step = m.stall_spacing_m / ring;
    for (g, &(a0, a1)) in arcs.iter().enumerate() {
        let draw = hash01(seed, SALT_MARKET + g as u64);
        let (lo, hi) = (
            m.stalls_per_group[0],
            m.stalls_per_group[1].max(m.stalls_per_group[0]),
        );
        let wanted = lo + ((draw * f64::from(hi - lo + 1)) as u32).min(hi - lo);
        let room = ((a1 - a0) / step).floor() as u32 + 1;
        let n = wanted.min(room);
        if n == 0 {
            continue;
        }
        let mid = (a0 + a1) * 0.5;
        for k in 0..n {
            let a = mid + (f64::from(k) - f64::from(n - 1) * 0.5) * step;
            let (x, z) = (cx + a.sin() * ring, cz + a.cos() * ring);
            let kind = PropKind::Stall;
            out.push(Prop {
                kind,
                x,
                z,
                // Front towards the centre of the square.
                yaw: crate::siege::yaw_facing(-a.sin(), -a.cos()),
                length: stall.length_m,
                depth: stall.depth_m,
                house: None,
            });
        }
        if hash01(seed, SALT_MARKET + 100 + g as u64) < m.extra_chance {
            let kind = pick(&m.extra_kinds, hash01(seed, SALT_MARKET + 200 + g as u64));
            let size = rules.props.size(kind);
            let a = mid - f64::from(n) * 0.5 * step - size.length_m * 0.5 / ring;
            if a > a0 - half_gap * 0.25 {
                let r = ring + stall.depth_m;
                out.push(Prop {
                    kind,
                    x: cx + a.sin() * r,
                    z: cz + a.cos() * r,
                    yaw: crate::siege::yaw_facing(-a.sin(), -a.cos()),
                    length: size.length_m,
                    depth: size.depth_m,
                    house: None,
                });
            }
        }
    }
    out
}

/// Props of a besieged town (blocks, suburbs, market square).
pub fn siege_props(works: &SiegeWorks, rules: &TownRules) -> Vec<Prop> {
    let p = &rules.props;
    let (cx, cz) = works.center;
    let mut props = Vec::new();
    for (i, h) in works.houses.iter().enumerate() {
        if h.church {
            continue;
        }
        let fp = h.footprint();
        let seed = key(i, h.x, h.z);
        if h.suburb {
            if hash01(seed, SALT_COUNT) < p.suburb_chance {
                let side = if hash01(seed, SALT_SIDE) < 0.5 {
                    1.0
                } else {
                    -1.0
                };
                along_face(&mut props, &fp, side, &p.suburb_kinds, 1, i, seed, rules);
            }
            continue;
        }
        let rows = if h.rows >= 2 { 2 } else { 1 };
        for r in 0..rows {
            let sign = if r == 0 { 1.0 } else { -1.0 };
            let (fx, fz) = fp.front();
            let towards_square = (fx * (cx - h.x) + fz * (cz - h.z)) * sign > 0.0;
            let kinds = if towards_square {
                &p.square_kinds
            } else {
                &p.street_kinds
            };
            let row_seed = seed ^ (r as u64 + 1).wrapping_mul(0x51_7CC1_B727_220A);
            let n = count(&p.block_row_chances, hash01(row_seed, SALT_COUNT));
            along_face(&mut props, &fp, sign, kinds, n, i, row_seed, rules);
        }
    }
    let seed = key(works.houses.len(), cx, cz);
    props.extend(market(works, rules, seed));
    let footprints: Vec<Footprint> = works.houses.iter().map(House::footprint).collect();
    keep_clear(props, &footprints)
}
