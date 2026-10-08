//! NT1 (ADR 0126): kinds of besieged places other than the landmark cities.
//!
//! - [`PlaceKind::City`]: the ring town of [`SiegeWorks::generate`]
//!   (octagon, radial streets, rings of blocks), unchanged.
//! - [`PlaceKind::Borough`]: a fortified borough — a polygonal enceinte, a
//!   main street running from the attacked gate through the market square to
//!   a gatehouse on the far side, back lanes parallel to it, and houses in
//!   rows along the streets.
//! - [`PlaceKind::Castle`]: a tight polygonal enceinte, a square keep towards
//!   the back, the bailey (courtyard) at the centre and a few buildings
//!   against the curtain wall.
//!
//! The kind comes from a data rule (`places` of `data/rules/siege_town.json`:
//! settlement kind and fortification level, [`PlaceRules::place_kind`]).
//! Variations (number of sides, radius, rotation, gate and keep position,
//! street bend) are drawn from a hash of the province ([`place_seed`]), never
//! from the battle's random stream: a province always gets the same plan.
//! The side of the attacked gate always faces the attacker (−z).

use data_model::util::dist_xz;
use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};

use data_model::SettlementKind;
use serde::{Deserialize, Serialize};

use crate::fire::Blaze;
use crate::rng::BattleRng;
use crate::rng::{hash01, Fnv1a};
use crate::siege::{
    place_church, yaw_facing, House, PieceKind, SiegeWorkRules, SiegeWorks, Tower, TownPlan,
    WallPiece, GATE_WIDTH, TOWN_CENTER,
};
use crate::siege_layout::MAX_PIECE;
use crate::town::{Footprint, PropKind, TownRules};

/// Kind of besieged place (NT1).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    /// Ring town (the generic plan of BR3).
    #[default]
    City,
    /// Fortified borough: main street, market square, rows of houses.
    Borough,
    /// Castle: tight enceinte, keep, bailey, few buildings.
    Castle,
}

impl PlaceKind {
    /// Every kind.
    pub const ALL: [PlaceKind; 3] = [PlaceKind::City, PlaceKind::Borough, PlaceKind::Castle];

    /// The `snake_case` key used in JSON files and by the bridge.
    pub fn key(self) -> &'static str {
        match self {
            PlaceKind::City => "city",
            PlaceKind::Borough => "borough",
            PlaceKind::Castle => "castle",
        }
    }

    /// The default kind (serde `skip_serializing_if`).
    pub fn is_city(&self) -> bool {
        *self == PlaceKind::City
    }
}

/// Plan of the castle (`places.castle`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CastleRules {
    pub sides: [u32; 2],
    pub radius_m: [f64; 2],
    pub radius_jitter: f64,
    pub rotation_deg: f64,
    pub gate_offset: f64,
    pub wall_height_bonus_m: f64,
    pub courtyard_m: f64,
    pub keep_side_m: [f64; 2],
    pub keep_distance: [f64; 2],
    pub keep_arc_deg: f64,
    pub buildings: [u32; 2],
    pub wall_walk_m: f64,
    pub lane_m: f64,
    /// NT8: factor on the radius of the castle's towers.
    pub tower_radius_scale: f64,
    /// NT8: height of the keep above the curtain walls.
    pub keep_height_above_wall_m: f64,
    /// NT11: share (0-1) of keeps roofed with a crenellated terrace
    /// instead of a pavilion roof (drawn from the place's seed).
    pub keep_terrace_share: f64,
    /// NT8: props in the bailey (besides the well).
    pub bailey_props: [u32; 2],
    pub bailey_kinds: Vec<PropKind>,
    /// NT8: props in front of the lean-to outbuildings.
    pub lean_to_kinds: Vec<PropKind>,
}

/// Plan of the fortified borough (`places.borough`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoroughRules {
    pub sides: [u32; 2],
    pub radius_m: [f64; 2],
    pub radius_jitter: f64,
    pub rotation_deg: f64,
    pub gate_offset: f64,
    pub market_m: f64,
    pub main_street_m: f64,
    pub street_bend_m: f64,
    pub back_lane_offset_m: [f64; 2],
    pub cross_lane_chance: f64,
}

/// `places` of `data/rules/siege_town.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceRules {
    pub by_settlement_kind: BTreeMap<SettlementKind, PlaceKind>,
    pub borough_to_city_from_fortification: u32,
    pub city_to_borough_up_to_fortification: u32,
    pub castle: CastleRules,
    pub borough: BoroughRules,
}

impl PlaceRules {
    /// The kind of place for a settlement of `kind` at `fortification`: the
    /// kind of the table, a borough strong enough being laid out as a city
    /// and a weak city as a borough.
    pub fn place_kind(&self, kind: SettlementKind, fortification: u32) -> PlaceKind {
        let base = self
            .by_settlement_kind
            .get(&kind)
            .copied()
            .unwrap_or_default();
        match base {
            PlaceKind::Borough if fortification >= self.borough_to_city_from_fortification => {
                PlaceKind::City
            }
            PlaceKind::City if fortification <= self.city_to_borough_up_to_fortification => {
                PlaceKind::Borough
            }
            other => other,
        }
    }
}

/// Seed of the plan of a province (FNV-1a of its id).
pub fn place_seed(province: &str) -> u64 {
    Fnv1a::default().bytes(province.as_bytes()).finish()
}

/// Draw in `[lo, hi]` from the seed.
fn span(seed: u64, salt: u64, [lo, hi]: [f64; 2]) -> f64 {
    lo + (hi - lo) * hash01(seed, salt)
}

/// Whole draw in `[lo, hi]` (inclusive) from the seed.
fn count(seed: u64, salt: u64, [lo, hi]: [u32; 2]) -> u32 {
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    lo + ((hash01(seed, salt) * f64::from(hi - lo + 1)) as u32).min(hi - lo)
}

/// Signed draw in `[-1, 1]`.
fn signed(seed: u64, salt: u64) -> f64 {
    hash01(seed, salt) * 2.0 - 1.0
}

fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Shape of an enceinte: vertices and the gate cut into side 0.
struct Enceinte {
    vertices: Vec<(f64, f64)>,
    /// Position of the gate along side 0 (m from vertex 0).
    gate_along: f64,
}

/// A polygon of `sides` sides around [`TOWN_CENTER`], winding like the
/// generic ring (angle from +z towards +x), side 0 facing the attacker
/// (−z) up to the rotation; each vertex at `radius` ± `jitter`.
fn enceinte(
    seed: u64,
    sides: u32,
    radius: f64,
    jitter: f64,
    rotation_deg: f64,
    gate_offset: f64,
) -> Enceinte {
    let n = sides.max(4) as usize;
    let step = TAU / n as f64;
    let rot = signed(seed, 11) * rotation_deg.to_radians();
    let vertices: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let angle = PI - step * 0.5 + step * k as f64 + rot;
            let r = radius * (1.0 + jitter * signed(seed, 100 + k as u64));
            (
                TOWN_CENTER.0 + angle.sin() * r,
                TOWN_CENTER.1 + angle.cos() * r,
            )
        })
        .collect();
    let len = dist_xz(vertices[0], vertices[1]);
    let jamb = 4.0;
    let room = (len * 0.5 - GATE_WIDTH * 0.5 - jamb).max(0.0);
    let gate_along = len * 0.5 + signed(seed, 12) * gate_offset * room;
    Enceinte {
        vertices,
        gate_along,
    }
}

/// Wall pieces of at most [`MAX_PIECE`] metres from `a` to `b`.
fn push_split(pieces: &mut Vec<WallPiece>, a: (f64, f64), b: (f64, f64), hp: f64) {
    let pieces_count = (dist_xz(a, b) / MAX_PIECE).ceil().max(1.0) as usize;
    for k in 0..pieces_count {
        let t0 = k as f64 / pieces_count as f64;
        let t1 = (k + 1) as f64 / pieces_count as f64;
        pieces.push(WallPiece {
            kind: PieceKind::Wall,
            a: lerp(a, b, t0),
            b: lerp(a, b, t1),
            hp,
            max_hp: hp,
            docked_tower: None,
            attacked_for: 0.0,
        });
    }
}

/// The works of an enceinte: pieces (gate on side 0, side `split` cut at its
/// middle for a gatehouse), towers at the corners and between pieces, two
/// smaller towers flanking the gate (radii × `tower_scale`). Returns the works (no house yet) and
/// the gatehouse point, if any.
fn ring_works(
    shape: &Enceinte,
    fortification: u32,
    height_bonus: f64,
    tower_scale: f64,
    split: Option<usize>,
) -> (SiegeWorks, Option<(f64, f64)>) {
    let fort = f64::from(fortification.min(5));
    let thickness = 2.5 + 0.5 * fort;
    let wall_height = 6.0 + 1.2 * fort + height_bonus;
    let (wall_hp, gate_hp) = SiegeWorkRules::bundled().hp(fortification);
    let v = &shape.vertices;
    let n = v.len();
    let mut pieces: Vec<WallPiece> = Vec::new();
    let mut gate = 0;
    let mut gatehouse = None;
    for k in 0..n {
        let (a, b) = (v[k], v[(k + 1) % n]);
        if k == 0 {
            let len = dist_xz(a, b).max(1e-9);
            let g0 = lerp(a, b, (shape.gate_along - GATE_WIDTH * 0.5) / len);
            let g1 = lerp(a, b, (shape.gate_along + GATE_WIDTH * 0.5) / len);
            push_split(&mut pieces, a, g0, wall_hp);
            gate = pieces.len();
            pieces.push(WallPiece {
                kind: PieceKind::Gate,
                a: g0,
                b: g1,
                hp: gate_hp,
                max_hp: gate_hp,
                docked_tower: None,
                attacked_for: 0.0,
            });
            push_split(&mut pieces, g1, b, wall_hp);
        } else if split == Some(k) {
            let mid = lerp(a, b, 0.5);
            gatehouse = Some(mid);
            push_split(&mut pieces, a, mid, wall_hp);
            push_split(&mut pieces, mid, b, wall_hp);
        } else {
            push_split(&mut pieces, a, b, wall_hp);
        }
    }
    let tower_radius = (3.5 + 0.4 * fort) * tower_scale;
    let mut towers: Vec<Tower> = Vec::new();
    for (i, p) in pieces.iter().enumerate() {
        let next_is_gate = pieces[(i + 1) % pieces.len()].kind == PieceKind::Gate;
        if p.kind == PieceKind::Gate || next_is_gate {
            continue;
        }
        let at_gatehouse = gatehouse.is_some_and(|g| dist_xz(g, p.b) < 0.5);
        towers.push(Tower {
            x: p.b.0,
            z: p.b.1,
            radius: tower_radius * if at_gatehouse { 1.25 } else { 1.0 },
            height: wall_height + if at_gatehouse { 5.0 } else { 3.5 },
        });
    }
    for end in [pieces[gate].a, pieces[gate].b] {
        towers.push(Tower {
            x: end.0,
            z: end.1,
            radius: tower_radius * 0.8,
            height: wall_height + 3.0,
        });
    }
    let works = SiegeWorks {
        fortification,
        center: TOWN_CENTER,
        vertices: v.clone(),
        pieces,
        towers,
        thickness,
        wall_height,
        square_radius: crate::siege::SQUARE_RADIUS,
        gate,
        hold_time: 0.0,
        points: Vec::new(),
        houses: Vec::new(),
        sortie: false,
        gate_fire: Blaze::default(),
        wind: (0.0, 0.0),
        landmark: None,
        props: Vec::new(),
        place: PlaceKind::City,
        streets: Vec::new(),
    };
    (works, gatehouse)
}

/// A point `metres` outside the gate (streets start there so that no block
/// stands in the gateway).
fn before_gate(works: &SiegeWorks, metres: f64) -> (f64, f64) {
    let g = &works.pieces[works.gate];
    let (mx, mz) = g.midpoint();
    let (nx, nz) = g.outward();
    (mx + nx * metres, mz + nz * metres)
}

impl SiegeWorks {
    /// The works of a place of `kind`: the generic ring town for a city, the
    /// borough or castle plan otherwise (variations from `seed`).
    pub fn generate_place(
        kind: PlaceKind,
        fortification: u32,
        breach: u8,
        seed: u64,
        rng: &mut BattleRng,
    ) -> Self {
        let rules = TownRules::bundled();
        let mut works = match kind {
            PlaceKind::City => return SiegeWorks::generate(fortification, breach, rng),
            PlaceKind::Borough => borough(fortification, seed, rules),
            PlaceKind::Castle => castle(fortification, seed, rules),
        };
        works.place = kind;
        works.lay_props();
        works.apply_campaign_breach(breach, rng);
        works
    }
}

/// The fortified borough of `seed` (no props, no breach yet).
fn borough(fortification: u32, seed: u64, rules: &TownRules) -> SiegeWorks {
    let b = &rules.places.borough;
    let sides = count(seed, 1, b.sides).max(5);
    let shape = enceinte(
        seed,
        sides,
        span(seed, 2, b.radius_m),
        b.radius_jitter,
        b.rotation_deg,
        b.gate_offset,
    );
    // The far gatehouse: the side facing +z (for an odd count, one of the
    // two sides beside the back vertex).
    let n = shape.vertices.len();
    let back = if !n.is_multiple_of(2) && hash01(seed, 3) >= 0.5 {
        n / 2 + 1
    } else {
        n / 2
    };
    let (mut works, gatehouse) = ring_works(&shape, fortification, 0.0, 1.0, Some(back));
    works.square_radius = b.market_m;
    let center = works.center;
    let far = gatehouse.unwrap_or((center.0, center.1 + 100.0));
    let start = before_gate(&works, 8.0);
    // The main street, bent half way on both sides of the market.
    let bend = |salt: u64, from: (f64, f64), to: (f64, f64)| {
        let mid = lerp(from, to, 0.5);
        let len = dist_xz(from, to).max(1e-9);
        let (nx, nz) = (-(to.1 - from.1) / len, (to.0 - from.0) / len);
        let off = signed(seed, salt) * b.street_bend_m;
        (mid.0 + nx * off, mid.1 + nz * off)
    };
    let main = vec![
        start,
        bend(4, start, center),
        center,
        bend(5, center, far),
        far,
    ];
    let mut streets = vec![main];
    // Back lanes parallel to the axis gate → far gate, on both sides.
    let axis_len = dist_xz(start, far).max(1e-9);
    let (ax, az) = ((far.0 - start.0) / axis_len, (far.1 - start.1) / axis_len);
    let (px, pz) = (-az, ax);
    for (salt, side) in [(6, 1.0), (7, -1.0)] {
        let off = span(seed, salt, b.back_lane_offset_m) * side;
        let o = (center.0 + px * off, center.1 + pz * off);
        let reach = 400.0;
        streets.push(vec![
            (o.0 - ax * reach, o.1 - az * reach),
            (o.0 + ax * reach, o.1 + az * reach),
        ]);
    }
    // A cross street through the market.
    if hash01(seed, 8) < b.cross_lane_chance {
        let reach = 400.0;
        streets.push(vec![
            (center.0 - px * reach, center.1 - pz * reach),
            (center.0 + px * reach, center.1 + pz * reach),
        ]);
    }
    let houses = {
        let g = &rules.generic;
        let block = &rules.block;
        let plan = TownPlan {
            center,
            square: works.square_radius + g.square_street_m,
            streets: streets.clone(),
            street_half: b.main_street_m * 0.5,
            pieces: &works.pieces,
            towers: &works.towers,
            wall_clear: works.band() + g.wall_walk_m,
            alley: g.alley_m,
            ring: &works.vertices,
        };
        let mut houses: Vec<House> = Vec::new();
        // Rows of houses along the streets first (the main street first),
        // then against the rampart; the gardens behind stay open.
        let depth = block.depth_m * 0.6;
        plan.street_rows(
            &mut houses,
            depth,
            block.min_frontage_m,
            block.max_frontage_m,
        );
        plan.wall_rows(
            &mut houses,
            block.depth_m * 0.5,
            block.min_frontage_m,
            block.max_frontage_m,
        );
        place_church(&mut houses, &plan, rules);
        if !houses.iter().any(|h| h.church) {
            open_church(&mut houses, &plan, rules);
        }
        houses
    };
    works.houses = houses;
    works.streets = clip_streets(&streets, &works.vertices);
    works
}

/// The parish church of a borough whose rows leave no block wide enough:
/// the free spot nearest the back of the market (facing the centre), clear
/// of the streets and walls; the rows it touches are dropped.
fn open_church(houses: &mut Vec<House>, plan: &TownPlan, rules: &TownRules) {
    let c = &rules.church;
    let target = (plan.center.0, plan.center.1 + c.toward_back_m);
    let mut spots: Vec<(f64, f64)> = Vec::new();
    for i in -40..=40 {
        for j in -10..=40 {
            spots.push((target.0 + i as f64 * 3.0, target.1 + j as f64 * 3.0));
        }
    }
    spots.sort_by(|a, b| dist_xz(*a, target).total_cmp(&dist_xz(*b, target)));
    for (x, z) in spots {
        let (dx, dz) = (plan.center.0 - x, plan.center.1 - z);
        let d = dx.hypot(dz).max(1e-9);
        let yaw = yaw_facing(dx / d, dz / d);
        let fp = Footprint::new(x, z, c.length_m, c.depth_m, yaw);
        if !plan.fits(&fp, &[]) {
            continue;
        }
        let mut church = House::block(x, z, c.length_m, c.depth_m, yaw);
        church.church = true;
        church.rows = 1;
        houses.retain(|o| o.suburb || !crate::siege::overlaps(&fp, &o.footprint(), plan.alley));
        houses.push(church);
        return;
    }
}

/// Streets cut to the part inside the ring (props and rendering), the main
/// street keeping its start before the gate.
fn clip_streets(streets: &[Vec<(f64, f64)>], ring: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let inside = |p: (f64, f64)| crate::geom::point_in_polygon(ring, p);
    streets
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            if i == 0 {
                return Some(line.clone());
            }
            // Straight lanes: keep the inside stretch of the segment.
            let (a, b) = (line[0], line[line.len() - 1]);
            let steps = 400;
            let pts: Vec<(f64, f64)> = (0..=steps)
                .map(|k| lerp(a, b, k as f64 / steps as f64))
                .filter(|&p| inside(p))
                .collect();
            (pts.len() >= 2).then(|| vec![pts[0], pts[pts.len() - 1]])
        })
        .collect()
}

/// The castle of `seed` (no props, no breach yet).
fn castle(fortification: u32, seed: u64, rules: &TownRules) -> SiegeWorks {
    let c = &rules.places.castle;
    let sides = count(seed, 1, c.sides);
    let radius = span(seed, 2, c.radius_m);
    let shape = enceinte(
        seed,
        sides,
        radius,
        c.radius_jitter,
        c.rotation_deg,
        c.gate_offset,
    );
    let (mut works, _) = ring_works(
        &shape,
        fortification,
        c.wall_height_bonus_m,
        c.tower_radius_scale,
        None,
    );
    works.square_radius = c.courtyard_m;
    let center = works.center;
    let start = before_gate(&works, 8.0);
    let block = &rules.block;
    let g = &rules.generic;
    // The keep: towards the back (+z), within the arc of the rules.
    let angle = signed(seed, 20) * c.keep_arc_deg.to_radians() * 0.5;
    let (sin, cos) = angle.sin_cos();
    let mut side = span(seed, 21, c.keep_side_m);
    let mut reach = span(seed, 22, c.keep_distance) * radius;
    let lane_to_keep = |reach: f64| {
        vec![
            center,
            (center.0 + sin * reach * 0.5, center.1 + cos * reach * 0.5),
        ]
    };
    let mut keep: Option<House> = None;
    let mut lanes = vec![vec![start, center]];
    for _ in 0..40 {
        let (x, z) = (center.0 + sin * reach, center.1 + cos * reach);
        let yaw = yaw_facing(-sin, -cos);
        let plan = TownPlan {
            center,
            square: works.square_radius + g.square_street_m * 0.5,
            streets: vec![vec![start, center]],
            street_half: c.lane_m * 0.5,
            pieces: &works.pieces,
            towers: &works.towers,
            wall_clear: works.band() + c.wall_walk_m,
            alley: g.alley_m,
            ring: &works.vertices,
        };
        let fp = Footprint::new(x, z, side, side, yaw);
        if plan.fits(&fp, &[]) {
            let mut house = House::block(x, z, side, side, yaw);
            house.rows = 1;
            house.keep = true;
            house.height = works.wall_height + c.keep_height_above_wall_m;
            house.terrace = hash01(seed, 23) < c.keep_terrace_share;
            keep = Some(house);
            lanes.push(lane_to_keep(reach - side * 0.5));
            break;
        }
        // Closer to the bailey, then smaller.
        if reach - side * 0.5 > works.square_radius + 4.0 {
            reach -= 2.0;
        } else {
            side *= 0.9;
            reach = span(seed, 22, c.keep_distance) * radius;
        }
    }
    let houses = {
        let plan = TownPlan {
            center,
            square: works.square_radius + g.square_street_m * 0.5,
            streets: lanes.clone(),
            street_half: c.lane_m * 0.5,
            pieces: &works.pieces,
            towers: &works.towers,
            wall_clear: works.band() + c.wall_walk_m,
            alley: g.alley_m,
            ring: &works.vertices,
        };
        let mut houses: Vec<House> = keep.into_iter().collect();
        // Lean-to buildings against the curtain: a few of the rows, drawn
        // from the seed.
        plan.wall_rows(
            &mut houses,
            block.depth_m * 0.6,
            block.min_frontage_m,
            block.max_frontage_m,
        );
        let wanted = count(seed, 23, c.buildings) as usize;
        let keep_count = usize::from(houses.first().is_some_and(|h| h.keep));
        let mut rows: Vec<(f64, House)> = houses
            .drain(keep_count..)
            .enumerate()
            .map(|(i, h)| (hash01(seed, 1000 + i as u64), h))
            .collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        houses.extend(rows.into_iter().take(wanted).map(|(_, h)| h));
        houses
    };
    works.houses = houses;
    works.streets = lanes;
    works
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::siege::PieceKind;

    fn works(kind: PlaceKind, province: &str) -> SiegeWorks {
        SiegeWorks::generate_place(
            kind,
            2,
            0,
            place_seed(province),
            &mut BattleRng::from_seed(7),
        )
    }

    /// Walkable cells (2 m) from just inside the gate reach the square
    /// without crossing a house, a prop that blocks or an intact wall.
    fn gate_reaches_square(w: &SiegeWorks) -> bool {
        let cell = 2.0;
        let (x0, z0) = (w.center.0 - 260.0, w.center.1 - 260.0);
        let size = (520.0 / cell) as usize;
        let idx = |x: f64, z: f64| -> Option<(usize, usize)> {
            let (i, j) = (((x - x0) / cell) as isize, ((z - z0) / cell) as isize);
            (i >= 0 && j >= 0 && (i as usize) < size && (j as usize) < size)
                .then_some((i as usize, j as usize))
        };
        let free = |i: usize, j: usize| {
            let (x, z) = (x0 + (i as f64 + 0.5) * cell, z0 + (j as f64 + 0.5) * cell);
            w.inside(x, z)
                && w.house_at(x, z, 1.0).is_none()
                && !w.prop_at(x, z, 0.5)
                && w.pieces
                    .iter()
                    .all(|p| p.kind == PieceKind::Gate || p.distance(x, z) > w.band())
                && w.towers.iter().all(|t| (t.x - x).hypot(t.z - z) > t.radius)
        };
        let g = &w.pieces[w.gate];
        let (mx, mz) = g.midpoint();
        let (nx, nz) = g.outward();
        let start = idx(mx - nx * (w.band() + 2.0), mz - nz * (w.band() + 2.0)).unwrap();
        let goal = idx(w.center.0, w.center.1).unwrap();
        let mut seen = vec![false; size * size];
        let mut queue = std::collections::VecDeque::from([start]);
        seen[start.1 * size + start.0] = true;
        while let Some((i, j)) = queue.pop_front() {
            if (i, j) == goal {
                return true;
            }
            for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (ni, nj) = (i as i32 + di, j as i32 + dj);
                if ni < 0 || nj < 0 || ni as usize >= size || nj as usize >= size {
                    continue;
                }
                let (ni, nj) = (ni as usize, nj as usize);
                if !seen[nj * size + ni] && free(ni, nj) {
                    seen[nj * size + ni] = true;
                    queue.push_back((ni, nj));
                }
            }
        }
        false
    }

    fn assert_valid(w: &SiegeWorks, label: &str) {
        let rules = TownRules::bundled();
        assert!(w.pieces[w.gate].kind == PieceKind::Gate, "{label}: gate");
        assert!(
            w.pieces[w.gate].outward().1 < -0.5,
            "{label}: gate faces −z"
        );
        assert!(!w.front_walls().is_empty(), "{label}: front walls");
        // Every piece faces out of the town.
        for p in &w.pieces {
            let (mx, mz) = p.midpoint();
            assert!(w.inside(mx - p.outward().0, mz - p.outward().1), "{label}");
        }
        let walk = match w.place {
            PlaceKind::Castle => rules.places.castle.wall_walk_m,
            _ => rules.generic.wall_walk_m,
        };
        for (i, h) in w.houses.iter().enumerate().filter(|(_, h)| !h.suburb) {
            let fp = h.footprint();
            for (x, z) in fp.corners() {
                assert!(w.inside(x, z), "{label}: house {i} outside the walls");
            }
            for p in &w.pieces {
                assert!(
                    fp.distance_to_segment(p.a, p.b) >= w.band() + walk - 1e-6,
                    "{label}: house {i} on the wall walk"
                );
            }
            assert!(
                fp.signed_distance(w.center.0, w.center.1) >= w.square_radius,
                "{label}: house {i} on the square"
            );
        }
        assert!(
            gate_reaches_square(w),
            "{label}: gate cut off from the square"
        );
    }

    #[test]
    fn rules_pick_the_kind() {
        let p = &TownRules::bundled().places;
        assert_eq!(p.place_kind(SettlementKind::Castle, 3), PlaceKind::Castle);
        assert_eq!(p.place_kind(SettlementKind::Town, 2), PlaceKind::Borough);
        assert_eq!(p.place_kind(SettlementKind::Town, 4), PlaceKind::City);
        assert_eq!(p.place_kind(SettlementKind::City, 3), PlaceKind::City);
        assert_eq!(p.place_kind(SettlementKind::City, 1), PlaceKind::Borough);
    }

    #[test]
    fn every_kind_is_valid() {
        for province in [
            "prov_paris",
            "prov_gascony",
            "prov_aleppo",
            "prov_kent",
            "x",
        ] {
            for kind in PlaceKind::ALL {
                let w = works(kind, province);
                assert_eq!(w.place, kind);
                assert_valid(&w, &format!("{province}/{}", kind.key()));
            }
        }
    }

    #[test]
    fn castle_has_a_keep_and_few_buildings() {
        let c = &TownRules::bundled().places.castle;
        for province in ["prov_paris", "prov_gascony", "prov_aleppo", "prov_kent"] {
            let w = works(PlaceKind::Castle, province);
            assert_eq!(w.houses.iter().filter(|h| h.keep).count(), 1, "{province}");
            assert!(w.houses.len() as u32 <= c.buildings[1] + 1, "{province}");
            assert!(w.church().is_none());
            let r = w
                .vertices
                .iter()
                .map(|v| dist_xz(*v, w.center))
                .fold(0.0, f64::max);
            assert!(r < 110.0, "{province}: tight enceinte ({r})");
        }
    }

    #[test]
    fn keep_roofs_vary_with_the_place() {
        // NT11: pavilion or crenellated terrace, drawn from the place's seed
        // (stable for a province, both seen over a few).
        let provinces: Vec<String> = (0..24).map(|i| format!("prov_{i}")).collect();
        let terraces: Vec<bool> = provinces
            .iter()
            .map(|p| {
                let w = works(PlaceKind::Castle, p);
                let keep = w.houses.iter().find(|h| h.keep).expect("keep");
                assert_eq!(
                    works(PlaceKind::Castle, p)
                        .houses
                        .iter()
                        .find(|h| h.keep)
                        .map(|h| h.terrace),
                    Some(keep.terrace),
                    "{p}: stable"
                );
                keep.terrace
            })
            .collect();
        assert!(terraces.iter().any(|t| *t) && terraces.iter().any(|t| !*t));
        let city = works(PlaceKind::City, "prov_paris");
        assert!(city.houses.iter().all(|h| !h.terrace));
    }

    #[test]
    fn castle_looks_like_a_castle() {
        let c = &TownRules::bundled().places.castle;
        for province in ["prov_paris", "prov_gascony", "prov_aleppo", "prov_kent"] {
            let w = works(PlaceKind::Castle, province);
            let borough = works(PlaceKind::Borough, province);
            // NT8: slimmer flanking towers than a borough's.
            let widest = w.towers.iter().map(|t| t.radius).fold(0.0, f64::max);
            let borough_corner = borough
                .towers
                .iter()
                .map(|t| t.radius)
                .fold(f64::INFINITY, f64::min)
                / 0.8;
            assert!(widest <= borough_corner * c.tower_radius_scale + 1e-9);
            assert!(widest <= 6.0, "{province}: towers {widest}");
            // A keep standing well above the towers.
            let keep = w.houses.iter().find(|h| h.keep).expect("keep");
            let tallest = w.towers.iter().map(|t| t.height).fold(0.0, f64::max);
            assert!(
                keep.height >= tallest + 8.0,
                "{province}: keep {}",
                keep.height
            );
            // A lived-in bailey: a well, carts, barrels, woodpiles; no stalls.
            let in_bailey: Vec<_> = w
                .props
                .iter()
                .filter(|p| p.house.is_none() && w.in_square(p.x, p.z))
                .collect();
            assert!(in_bailey.iter().any(|p| p.kind == PropKind::Well));
            assert!(
                in_bailey.len() >= 4,
                "{province}: {} props",
                in_bailey.len()
            );
            assert!(w.props.iter().all(|p| p.kind != PropKind::Stall));
            assert!(
                w.houses.iter().filter(|h| !h.keep).count() >= 3,
                "{province}: outbuildings"
            );
        }
    }

    #[test]
    fn borough_lines_its_streets() {
        for province in ["prov_paris", "prov_gascony", "prov_aleppo", "prov_kent"] {
            let w = works(PlaceKind::Borough, province);
            assert!(w.houses.len() >= 30, "{province}: {}", w.houses.len());
            assert!(w.church().is_some(), "{province}: church");
            let main = &w.streets[0];
            let along = w
                .houses
                .iter()
                .filter(|h| {
                    main.windows(2)
                        .any(|s| h.footprint().distance_to_segment(s[0], s[1]) < 8.0)
                })
                .count();
            assert!(along >= 5, "{province}: {along} houses on the main street");
            // A gatehouse on the far side.
            // (a stouter tower than the gate flankers, which are 0.8 of a corner).
            let flanker = w
                .towers
                .iter()
                .map(|t| t.radius)
                .fold(f64::INFINITY, f64::min);
            assert!(w
                .towers
                .iter()
                .any(|t| t.z > w.center.1 + 60.0 && t.radius > flanker * 1.4));
        }
    }

    #[test]
    fn deterministic_per_province() {
        for kind in PlaceKind::ALL {
            assert_eq!(works(kind, "prov_paris"), works(kind, "prov_paris"));
        }
        // The plan does not depend on the battle's stream (only the city
        // jitters its radius with it).
        for kind in [PlaceKind::Borough, PlaceKind::Castle] {
            let a = SiegeWorks::generate_place(
                kind,
                2,
                0,
                place_seed("prov_kent"),
                &mut BattleRng::from_seed(1),
            );
            let b = SiegeWorks::generate_place(
                kind,
                2,
                0,
                place_seed("prov_kent"),
                &mut BattleRng::from_seed(99),
            );
            assert_eq!(a, b);
        }
    }

    #[test]
    fn provinces_differ() {
        for kind in [PlaceKind::Borough, PlaceKind::Castle] {
            let a = works(kind, "prov_paris");
            let b = works(kind, "prov_gascony");
            assert_ne!(a.vertices, b.vertices, "{}", kind.key());
            assert_ne!(a.houses, b.houses, "{}", kind.key());
        }
    }

    #[test]
    fn breach_opens_the_front() {
        for kind in [PlaceKind::Borough, PlaceKind::Castle] {
            let w = SiegeWorks::generate_place(
                kind,
                2,
                90,
                place_seed("prov_kent"),
                &mut BattleRng::from_seed(3),
            );
            assert!(w.openings().len() >= 2, "{}", kind.key());
        }
    }
}
