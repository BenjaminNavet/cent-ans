//! Siege battles in a landmark city (lot L3, ADR 0026): the besieged town is
//! drawn from the historical plan (`data/landmarks/<id>.json`, block
//! `siege.battle`) instead of the generic octagon of [`SiegeWorks::generate`].
//!
//! [`SiegeLayout::from_landmark`] chains the listed walls into a ring (an
//! open wall ending on a river is closed by a straight quay), keeps the
//! gates and the listed streets, in plan metres. [`SiegeWorks::from_layout`]
//! brings the ring to the scale of the battlefield (mean radius of the
//! generic town, bounded so that the deployment zones and the backdrop keep
//! their room), turns the attacked gate towards the attacker (−z), splits the
//! ring into wall pieces with towers, makes the other gates gatehouses and
//! lays the houses on a lattice that leaves the streets, the square and a
//! lane from the gate free. Pure geometry: no random draw except the
//! campaign breach (same rule as the generic town), hence deterministic.
//! When the plan does not give a usable ring (not star-shaped around the
//! square, no wall facing the attacker) the generic town is used.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::geom;
use crate::rng::BattleRng;
use crate::siege::{
    place_church, House, PieceKind, SiegeWorkRules, SiegeWorks, Tower, TownPlan, WallPiece,
    GATE_WIDTH, RING_RADIUS, SQUARE_RADIUS, TOWN_CENTER,
};
use crate::town::{Footprint, TownRules};

/// Longest straight wall piece (m): longer stretches are split (towers).
pub const MAX_PIECE: f64 = 55.0;
/// Douglas-Peucker tolerance when simplifying the scaled ring (m).
const SIMPLIFY_TOLERANCE: f64 = 4.0;
/// Bounds of the ring around [`TOWN_CENTER`] (m): towards the attacker (the
/// deployment zones), behind (the backdrop and the edge of the field) and on
/// the flanks.
const MAX_FRONT: f64 = 240.0;
const MAX_BACK: f64 = 200.0;
const MAX_SIDE: f64 = 380.0;

/// A gate of the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutGate {
    pub name: String,
    /// Plan point (metres, x east, y north).
    pub at: (f64, f64),
}

/// The besieged town of a landmark city, in plan metres (x east, y north).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiegeLayout {
    /// Landmark id (`paris`, `london`…).
    pub id: String,
    pub name: String,
    /// Ring of the walls (polygon, closed implicitly).
    pub ring: Vec<(f64, f64)>,
    /// The closing edge (last → first vertex) is a quay along the river.
    #[serde(default)]
    pub quay: bool,
    pub gates: Vec<LayoutGate>,
    /// Index of the attacked gate in `gates`.
    pub gate: usize,
    #[serde(default)]
    pub streets: Vec<Vec<(f64, f64)>>,
    #[serde(default)]
    pub square: Option<(f64, f64)>,
    #[serde(default)]
    pub radius_m: Option<f64>,
}

/// Why a landmark gives no layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    NoBattleBlock,
    UnknownWall(String),
    UnknownGate(String),
    UnknownStreet(String),
    TooFewPoints,
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::NoBattleBlock => write!(f, "no siege.battle block"),
            LayoutError::UnknownWall(id) => write!(f, "unknown wall {id}"),
            LayoutError::UnknownGate(name) => write!(f, "unknown gate {name}"),
            LayoutError::UnknownStreet(id) => write!(f, "unknown street {id}"),
            LayoutError::TooFewPoints => write!(f, "ring with fewer than 3 points"),
        }
    }
}

impl std::error::Error for LayoutError {}

/// What the renderer needs to know about a town drawn from a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiegeLandmark {
    pub id: String,
    pub name: String,
    /// Name of the attacked gate.
    pub gate_name: String,
    /// The other gates: `(name, x, z)` of their gatehouse on the ring.
    pub gatehouses: Vec<(String, f64, f64)>,
    /// Streets kept as lanes (battle metres).
    pub streets: Vec<Vec<(f64, f64)>>,
    /// Pieces of the quay closing an open wall along the river.
    pub quay: Vec<usize>,
    /// Plan metres per battle metre (the town is drawn smaller than life).
    pub plan_scale: f64,
}

fn tuple(p: [f64; 2]) -> (f64, f64) {
    (p[0], p[1])
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Closest point of the closed polygon `ring` to `p`.
fn snap_to_ring(ring: &[(f64, f64)], p: (f64, f64)) -> (f64, f64) {
    let n = ring.len();
    let mut best = (f64::INFINITY, ring[0]);
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let q = geom::closest_on_segment(a, b, p);
        let d = dist(p, q);
        if d < best.0 {
            best = (d, q);
        }
    }
    best.1
}

/// Signed area (shoelace) of a closed polygon.
fn signed_area(ring: &[(f64, f64)]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        * 0.5
}

/// Area centroid of a closed polygon (vertex mean if degenerate).
fn centroid(ring: &[(f64, f64)]) -> (f64, f64) {
    let n = ring.len();
    let area = signed_area(ring);
    if area.abs() < 1e-9 {
        let k = n as f64;
        return (
            ring.iter().map(|p| p.0).sum::<f64>() / k,
            ring.iter().map(|p| p.1).sum::<f64>() / k,
        );
    }
    let (mut cx, mut cy) = (0.0, 0.0);
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let cross = a.0 * b.1 - b.0 * a.1;
        cx += (a.0 + b.0) * cross;
        cy += (a.1 + b.1) * cross;
    }
    (cx / (6.0 * area), cy / (6.0 * area))
}

/// Removes, one at a time, the vertex that deviates least from the chord of
/// its neighbours while that deviation stays under `tolerance` (closed ring,
/// at least 5 vertices kept). Deterministic: ties go to the lowest index.
fn simplify(ring: &mut Vec<(f64, f64)>, tolerance: f64) {
    while ring.len() > 5 {
        let n = ring.len();
        let (mut best, mut best_d) = (0, f64::INFINITY);
        for i in 0..n {
            let d = geom::segment_distance(ring[i], ring[(i + n - 1) % n], ring[(i + 1) % n]);
            if d < best_d {
                best = i;
                best_d = d;
            }
        }
        if best_d > tolerance {
            break;
        }
        ring.remove(best);
    }
}

impl SiegeLayout {
    /// The besieged town described by the `siege.battle` block of a plan.
    pub fn from_landmark(landmark: &data_model::Landmark) -> Result<Self, LayoutError> {
        let battle = landmark.battle().ok_or(LayoutError::NoBattleBlock)?;
        let mut ring: Vec<(f64, f64)> = Vec::new();
        let mut gates = Vec::new();
        let mut closed = false;
        for id in &battle.walls {
            let wall = landmark
                .walls
                .iter()
                .find(|w| &w.id == id)
                .ok_or_else(|| LayoutError::UnknownWall(id.clone()))?;
            let mut points: Vec<(f64, f64)> = wall.points.iter().copied().map(tuple).collect();
            if let Some(&tail) = ring.last() {
                // Chain by the nearest end.
                if dist(tail, points[points.len() - 1]) < dist(tail, points[0]) {
                    points.reverse();
                }
                if dist(tail, points[0]) < 1.0 {
                    points.remove(0);
                }
            }
            closed = battle.walls.len() == 1 && wall.closed;
            ring.extend(points);
            gates.extend(wall.gates.iter().map(|g| LayoutGate {
                name: g.name.clone(),
                at: tuple(g.at),
            }));
        }
        if ring.len() > 2 && dist(ring[0], ring[ring.len() - 1]) < 1.0 {
            ring.pop();
            closed = true;
        }
        if ring.len() < 3 {
            return Err(LayoutError::TooFewPoints);
        }
        let gate = gates
            .iter()
            .position(|g| g.name == battle.gate)
            .ok_or_else(|| LayoutError::UnknownGate(battle.gate.clone()))?;
        let streets = battle
            .streets
            .iter()
            .map(|id| {
                landmark
                    .streets
                    .iter()
                    .find(|s| &s.id == id)
                    .map(|s| s.points.iter().copied().map(tuple).collect())
                    .ok_or_else(|| LayoutError::UnknownStreet(id.clone()))
            })
            .collect::<Result<Vec<Vec<(f64, f64)>>, _>>()?;
        Ok(SiegeLayout {
            id: landmark.id.clone(),
            name: landmark.name.clone(),
            ring,
            quay: !closed,
            gates,
            gate,
            streets,
            square: battle.square.map(tuple),
            radius_m: battle.radius_m,
        })
    }
}

/// Plan → battlefield similarity: `TOWN_CENTER + scale · R(θ) · (p − origin)`.
#[derive(Debug, Clone, Copy)]
struct PlanTransform {
    origin: (f64, f64),
    cos: f64,
    sin: f64,
    scale: f64,
}

impl PlanTransform {
    fn rotate(&self, p: (f64, f64)) -> (f64, f64) {
        (
            p.0 * self.cos - p.1 * self.sin,
            p.0 * self.sin + p.1 * self.cos,
        )
    }

    fn apply(&self, p: (f64, f64)) -> (f64, f64) {
        let (x, z) = self.rotate((p.0 - self.origin.0, p.1 - self.origin.1));
        (
            TOWN_CENTER.0 + x * self.scale,
            TOWN_CENTER.1 + z * self.scale,
        )
    }
}

/// Points every `step` metres along the closed ring.
fn densify(ring: &[(f64, f64)], step: f64) -> Vec<(f64, f64)> {
    let n = ring.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let count = (dist(a, b) / step).ceil().max(1.0) as usize;
        for k in 0..count {
            let t = k as f64 / count as f64;
            out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
    }
    out
}

impl SiegeWorks {
    /// Siege works of the setup: the town of the landmark plan when the
    /// setup carries a usable layout, otherwise the generic ring town.
    pub fn for_battle(
        fortification: u32,
        breach: u8,
        layout: Option<&SiegeLayout>,
        rng: &mut BattleRng,
    ) -> Self {
        layout
            .and_then(|l| SiegeWorks::from_layout(l, fortification, breach, rng))
            .unwrap_or_else(|| SiegeWorks::generate(fortification, breach, rng))
    }

    /// Farthest point of the ring from [`TOWN_CENTER`] (flattening of the
    /// ground around the town).
    pub fn outer_radius(&self) -> f64 {
        self.vertices
            .iter()
            .map(|&v| dist(v, TOWN_CENTER))
            .fold(RING_RADIUS, f64::max)
    }

    /// The town of a landmark plan; `None` when the ring cannot be used
    /// (the caller then builds the generic town).
    pub fn from_layout(
        layout: &SiegeLayout,
        fortification: u32,
        breach: u8,
        rng: &mut BattleRng,
    ) -> Option<Self> {
        if layout.ring.len() < 3 || layout.gate >= layout.gates.len() {
            return None;
        }
        let fort = f64::from(fortification.min(5));
        let thickness = 2.5 + 0.5 * fort;
        let wall_height = 6.0 + 1.5 * fort;
        let (wall_hp, gate_hp) = SiegeWorkRules::bundled().hp(fortification);
        let gate_plan = snap_to_ring(&layout.ring, layout.gates[layout.gate].at);
        let transform = plan_transform(layout, gate_plan)?;
        let (ring, quay_ends) = scaled_ring(layout, &transform)?;
        let mut pieces = ring_pieces(&ring, wall_hp);
        let gate_point = transform.apply(gate_plan);
        let gate = cut_gate(&mut pieces, gate_point, wall_hp, gate_hp);
        if !pieces_face_out(&pieces) {
            return None;
        }
        let tower_radius = 5.0 + fort;
        let mut towers = ring_towers(&pieces, gate, tower_radius, wall_height);
        let gatehouses = gatehouse_towers(
            layout,
            &ring,
            &transform,
            gate_point,
            (tower_radius * 1.25, wall_height + 7.0),
            &mut towers,
        );
        let streets: Vec<Vec<(f64, f64)>> = layout
            .streets
            .iter()
            .map(|s| s.iter().map(|&p| transform.apply(p)).collect())
            .collect();
        let quay = quay_pieces(&pieces, quay_ends);
        let mut works = SiegeWorks::skeleton(
            fortification,
            (thickness, wall_height),
            ring,
            pieces,
            towers,
            gate,
        );
        if works.front_walls().is_empty() {
            return None;
        }
        works.houses = lattice_houses(&works, &streets, gate_point, TownRules::bundled());
        works.landmark = Some(SiegeLandmark {
            id: layout.id.clone(),
            name: layout.name.clone(),
            gate_name: layout.gates[layout.gate].name.clone(),
            gatehouses,
            streets,
            quay,
            plan_scale: 1.0 / transform.scale,
        });
        works.lay_props();
        works.apply_campaign_breach(breach, rng);
        Some(works)
    }
}

/// The plan → battlefield similarity: the square (or the centroid) at the
/// centre, the attacked gate (`gate_plan`, on the ring) towards −z, scaled
/// to the mean radius of the generic town within the bounds.
fn plan_transform(layout: &SiegeLayout, gate_plan: (f64, f64)) -> Option<PlanTransform> {
    let origin = layout.square.unwrap_or_else(|| centroid(&layout.ring));
    // The wall of the gate faces the attacker: its outward normal (away
    // from the square) is turned towards −z.
    let n = layout.ring.len();
    let edge = (0..n)
        .min_by(|&i, &j| {
            let d = |k: usize| {
                geom::segment_distance(gate_plan, layout.ring[k], layout.ring[(k + 1) % n])
            };
            d(i).total_cmp(&d(j)).then(i.cmp(&j))
        })
        .unwrap_or(0);
    let (a, b) = (layout.ring[edge], layout.ring[(edge + 1) % n]);
    let len = dist(a, b).max(1e-9);
    let mut normal = ((b.1 - a.1) / len, -(b.0 - a.0) / len);
    if normal.0 * (gate_plan.0 - origin.0) + normal.1 * (gate_plan.1 - origin.1) < 0.0 {
        normal = (-normal.0, -normal.1);
    }
    let theta = -PI / 2.0 - normal.1.atan2(normal.0);
    let mut transform = PlanTransform {
        origin,
        cos: theta.cos(),
        sin: theta.sin(),
        scale: 1.0,
    };
    let samples: Vec<(f64, f64)> = densify(&layout.ring, 5.0)
        .into_iter()
        .map(|p| transform.rotate((p.0 - origin.0, p.1 - origin.1)))
        .collect();
    let mean = samples.iter().map(|p| p.0.hypot(p.1)).sum::<f64>() / samples.len() as f64;
    if mean < 1.0 {
        return None;
    }
    let front = samples.iter().map(|p| -p.1).fold(1.0, f64::max);
    let back = samples.iter().map(|p| p.1).fold(1.0, f64::max);
    let side = samples.iter().map(|p| p.0.abs()).fold(1.0, f64::max);
    let target = layout.radius_m.unwrap_or(RING_RADIUS);
    transform.scale = (target / mean)
        .min(MAX_FRONT / front)
        .min(MAX_BACK / back)
        .min(MAX_SIDE / side);
    Some(transform)
}

/// The plan ring in battle metres, simplified and wound like the generic
/// ring, with the ends of the quay edge when the plan has one.
type ScaledRing = (Vec<(f64, f64)>, Option<((f64, f64), (f64, f64))>);

fn scaled_ring(layout: &SiegeLayout, transform: &PlanTransform) -> Option<ScaledRing> {
    let mut ring: Vec<(f64, f64)> = layout.ring.iter().map(|&p| transform.apply(p)).collect();
    // The quay edge (last → first) is remembered before simplification.
    let quay_ends = layout.quay.then(|| (ring[ring.len() - 1], ring[0]));
    simplify(&mut ring, SIMPLIFY_TOLERANCE);
    // Same winding as the generic ring (negative area in x-z).
    if signed_area(&ring) > 0.0 {
        ring.reverse();
    }
    geom::point_in_polygon(&ring, TOWN_CENTER).then_some((ring, quay_ends))
}

/// Wall pieces of at most [`MAX_PIECE`] metres along the closed ring.
fn ring_pieces(ring: &[(f64, f64)], wall_hp: f64) -> Vec<WallPiece> {
    let n = ring.len();
    let mut pieces = Vec::new();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let count = (dist(a, b) / MAX_PIECE).ceil().max(1.0) as usize;
        let at = |k: usize| {
            let t = k as f64 / count as f64;
            (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
        };
        for k in 0..count {
            pieces.push(WallPiece::new(PieceKind::Wall, at(k), at(k + 1), wall_hp));
        }
    }
    pieces
}

/// Cuts the attacked gate into the piece nearest `gate_point` (the whole
/// piece when it is too short for jambs); returns the gate's index.
fn cut_gate(
    pieces: &mut Vec<WallPiece>,
    gate_point: (f64, f64),
    wall_hp: f64,
    gate_hp: f64,
) -> usize {
    let host = (0..pieces.len())
        .min_by(|&i, &j| {
            pieces[i]
                .distance(gate_point.0, gate_point.1)
                .total_cmp(&pieces[j].distance(gate_point.0, gate_point.1))
                .then(i.cmp(&j))
        })
        .unwrap_or(0);
    let piece = pieces[host].clone();
    let len = piece.length();
    let jamb = 4.0;
    if len < GATE_WIDTH + 2.0 * jamb {
        pieces[host].kind = PieceKind::Gate;
        pieces[host].hp = gate_hp;
        pieces[host].max_hp = gate_hp;
        return host;
    }
    let (tx, tz) = piece.tangent();
    let along = ((gate_point.0 - piece.a.0) * tx + (gate_point.1 - piece.a.1) * tz)
        .clamp(GATE_WIDTH * 0.5 + jamb, len - GATE_WIDTH * 0.5 - jamb);
    let at = |s: f64| (piece.a.0 + tx * s, piece.a.1 + tz * s);
    let (g0, g1) = (at(along - GATE_WIDTH * 0.5), at(along + GATE_WIDTH * 0.5));
    pieces.splice(
        host..=host,
        [
            WallPiece::new(PieceKind::Wall, piece.a, g0, wall_hp),
            WallPiece::new(PieceKind::Gate, g0, g1, gate_hp),
            WallPiece::new(PieceKind::Wall, g1, piece.b, wall_hp),
        ],
    );
    host + 1
}

/// Every piece must face out of the town seen from the square.
fn pieces_face_out(pieces: &[WallPiece]) -> bool {
    pieces.iter().all(|p| {
        let (tx, tz) = p.tangent();
        let (ox, oz) = p.outward();
        let (mx, mz) = p.midpoint();
        ox * -tz + oz * tx >= 0.5 && dist((mx, mz), TOWN_CENTER) >= SQUARE_RADIUS + 25.0
    })
}

/// Towers at the end of each wall piece (not beside the gate) and at the
/// two jambs of the attacked gate.
fn ring_towers(pieces: &[WallPiece], gate: usize, radius: f64, wall_height: f64) -> Vec<Tower> {
    let mut towers = Vec::new();
    for (i, p) in pieces.iter().enumerate() {
        let next_is_gate = pieces[(i + 1) % pieces.len()].kind == PieceKind::Gate;
        if p.kind == PieceKind::Gate || next_is_gate {
            continue;
        }
        towers.push(Tower {
            x: p.b.0,
            z: p.b.1,
            radius,
            height: wall_height + 4.0,
        });
    }
    for end in [pieces[gate].a, pieces[gate].b] {
        towers.push(Tower {
            x: end.0,
            z: end.1,
            radius: radius * 0.8,
            height: wall_height + 3.0,
        });
    }
    towers
}

/// The other gates become gatehouse towers on the ring (`size` is their
/// `(radius, height)`); returns their `(name, x, z)`.
fn gatehouse_towers(
    layout: &SiegeLayout,
    ring: &[(f64, f64)],
    transform: &PlanTransform,
    gate_point: (f64, f64),
    size: (f64, f64),
    towers: &mut Vec<Tower>,
) -> Vec<(String, f64, f64)> {
    let mut gatehouses = Vec::new();
    for (k, g) in layout.gates.iter().enumerate() {
        if k == layout.gate {
            continue;
        }
        let on_ring = snap_to_ring(ring, transform.apply(g.at));
        if dist(on_ring, gate_point) < GATE_WIDTH + 10.0 {
            continue;
        }
        towers.push(Tower {
            x: on_ring.0,
            z: on_ring.1,
            radius: size.0,
            height: size.1,
        });
        gatehouses.push((g.name.clone(), on_ring.0, on_ring.1));
    }
    gatehouses
}

/// Pieces lying along the quay edge closing an open wall.
fn quay_pieces(pieces: &[WallPiece], quay_ends: Option<((f64, f64), (f64, f64))>) -> Vec<usize> {
    let Some((a, b)) = quay_ends else {
        return Vec::new();
    };
    (0..pieces.len())
        .filter(|&i| geom::segment_distance(pieces[i].midpoint(), a, b) < 2.0)
        .collect()
}

/// House blocks on a hexagonal lattice inside the ring, off the streets, the
/// square, the lane from the gate to the square and the wall walk (BR3:
/// oriented blocks, a denser lattice from `data/rules/siege_town.json`; a
/// block near a street of the plan lines up with it, the others with the
/// lattice; the church replaces the block nearest the back of the town).
fn lattice_houses(
    works: &SiegeWorks,
    streets: &[Vec<(f64, f64)>],
    gate_point: (f64, f64),
    rules: &TownRules,
) -> Vec<House> {
    let l = &rules.landmark;
    let block = &rules.block;
    let mut lanes: Vec<Vec<(f64, f64)>> = streets.to_vec();
    lanes.push(vec![gate_point, TOWN_CENTER]);
    let plan = TownPlan {
        center: TOWN_CENTER,
        square: works.square_radius + l.square_street_m,
        streets: lanes,
        street_half: l.lane_m * 0.5,
        pieces: &works.pieces,
        towers: &works.towers,
        wall_clear: works.band() + l.wall_walk_m,
        alley: rules.generic.alley_m,
        ring: &works.vertices,
    };
    let half = block.depth_m * 0.5;
    let (min, max) = (block.min_frontage_m, block.max_frontage_m);
    // Houses along the rampart street and the streets of the plan first,
    // then blocks on a lattice in what is left.
    let mut houses = Vec::new();
    plan.wall_rows(&mut houses, half, min, max);
    plan.street_rows(&mut houses, half, min, max);
    let reach = works.outer_radius() + l.pitch_x_m.max(l.pitch_z_m);
    let rows = (reach / l.pitch_z_m).ceil() as i32;
    let cols = (reach / l.pitch_x_m).ceil() as i32;
    for j in -rows..=rows {
        for i in -cols..=cols {
            let p = (
                TOWN_CENTER.0 + f64::from(i) * l.pitch_x_m,
                TOWN_CENTER.1 + f64::from(j) * l.pitch_z_m,
            );
            if !works.inside(p.0, p.1) {
                continue;
            }
            // Row 0 turned towards the square.
            let yaw = if p.1 > TOWN_CENTER.1 { PI } else { 0.0 };
            let front = (-yaw.sin(), yaw.cos());
            // The whole block, else the row on the square's side, else the
            // other one.
            for (shift, rows) in [(0.0, 2_u8), (half * 0.5, 1), (-half * 0.5, 1)] {
                let d = if rows == 2 { block.depth_m } else { half };
                let (x, z) = (p.0 + front.0 * shift, p.1 + front.1 * shift);
                let fp = Footprint::new(x, z, l.frontage_m, d, yaw);
                if plan.fits(&fp, &houses) {
                    let mut house = House::block(x, z, l.frontage_m, d, yaw);
                    house.rows = rows;
                    houses.push(house);
                    break;
                }
            }
        }
    }
    place_church(&mut houses, &plan, rules);
    houses
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_layout() -> SiegeLayout {
        SiegeLayout {
            id: "test".into(),
            name: "Test".into(),
            ring: vec![
                (-500.0, -400.0),
                (500.0, -400.0),
                (500.0, 400.0),
                (-500.0, 400.0),
            ],
            quay: true,
            gates: vec![
                LayoutGate {
                    name: "Nord".into(),
                    at: (0.0, 400.0),
                },
                LayoutGate {
                    name: "Est".into(),
                    at: (500.0, 0.0),
                },
            ],
            gate: 0,
            streets: vec![vec![(0.0, 400.0), (0.0, -400.0)]],
            square: None,
            radius_m: None,
        }
    }

    #[test]
    fn gate_faces_the_attacker() {
        let works = SiegeWorks::from_layout(&square_layout(), 2, 0, &mut BattleRng::from_seed(1))
            .expect("usable ring");
        let gate = &works.pieces[works.gate];
        assert_eq!(gate.kind, PieceKind::Gate);
        assert!(gate.outward().1 < -0.9, "{:?}", gate.outward());
        assert!(gate.midpoint().1 < TOWN_CENTER.1);
        let landmark = works.landmark.as_ref().unwrap();
        assert_eq!(landmark.gate_name, "Nord");
        assert_eq!(landmark.gatehouses.len(), 1);
        // The north street runs from the gate through the centre: no house on it.
        let street = &landmark.streets[0];
        let lane = TownRules::bundled().landmark.lane_m;
        assert!(works.houses.iter().all(|h| {
            street
                .windows(2)
                .all(|w| h.footprint().distance_to_segment(w[0], w[1]) >= lane * 0.5 - 1e-6)
        }));
        assert!(!works.houses.is_empty());
        assert!(works.pieces.iter().all(|p| p.length() <= MAX_PIECE + 1e-6));
    }

    #[test]
    fn ring_within_bounds() {
        let works =
            SiegeWorks::from_layout(&square_layout(), 1, 0, &mut BattleRng::from_seed(1)).unwrap();
        for &(x, z) in &works.vertices {
            assert!(z - TOWN_CENTER.1 <= MAX_BACK + 1e-6);
            assert!(TOWN_CENTER.1 - z <= MAX_FRONT + 1e-6);
            assert!((x - TOWN_CENTER.0).abs() <= MAX_SIDE + 1e-6);
        }
    }

    #[test]
    fn deterministic() {
        let a = SiegeWorks::from_layout(&square_layout(), 2, 90, &mut BattleRng::from_seed(7));
        let b = SiegeWorks::from_layout(&square_layout(), 2, 90, &mut BattleRng::from_seed(7));
        assert_eq!(a, b);
        let opened = a.unwrap().pieces.iter().filter(|p| !p.intact()).count();
        assert_eq!(opened, 2);
    }

    #[test]
    fn unusable_ring_falls_back() {
        // A thin crescent: the square lies outside.
        let mut layout = square_layout();
        layout.ring = vec![(-500.0, 0.0), (500.0, 0.0), (500.0, 10.0), (-500.0, 10.0)];
        layout.square = Some((0.0, -300.0));
        let mut rng = BattleRng::from_seed(3);
        assert!(SiegeWorks::from_layout(&layout, 1, 0, &mut rng).is_none());
        let works = SiegeWorks::for_battle(1, 0, Some(&layout), &mut BattleRng::from_seed(3));
        assert!(works.landmark.is_none());
    }
}
