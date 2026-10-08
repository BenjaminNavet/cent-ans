//! Siege battle geometry (spec `docs/design/m8-sieges.md` § 2): a polygonal
//! wall ring with towers and a gate around a central square.
//!
//! The ring is made of straight [`WallPiece`]s (the side facing the attacker
//! is split in two walls and a gate). Each piece has hit points: engines and
//! the ram wear them down; a piece at 0 HP is an opening (breach or broken
//! gate) that any regiment may cross. Intact pieces block movement, melee
//! and sight, except for defenders on the wall walk and attackers climbing
//! with ladders or from a siege tower.

use serde::{Deserialize, Serialize};

use crate::fire::Blaze;
use crate::rng::BattleRng;
use crate::town::{Footprint, Prop, TownRules};

/// Campaign breach (0-100) from which a siege battle starts with one gap
/// in the front walls (SV4: named so the UI quotes the same threshold)...
pub const BREACH_ONE_GAP: u8 = 50;
/// ... and with two gaps.
pub const BREACH_TWO_GAPS: u8 = 85;

/// Centre of the town (x, z), in metres.
pub const TOWN_CENTER: (f64, f64) = (600.0, 560.0);
/// Mean radius of the wall ring.
pub const RING_RADIUS: f64 = 150.0;
/// Number of sides of the ring.
pub const RING_SIDES: usize = 8;
/// Width of the gate.
pub const GATE_WIDTH: f64 = 14.0;
/// Radius of the central square the attacker must hold.
pub const SQUARE_RADIUS: f64 = 35.0;
/// Seconds for a regiment to scale the wall with ladders.
pub const LADDER_TIME: f64 = 45.0;
/// Seconds to cross onto the wall from a docked siege tower.
pub const TOWER_CLIMB_TIME: f64 = 8.0;
/// HP of a work at fortification level `fort` (0-5): `hp_base + fort × hp_per_fortification`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkHp {
    pub hp_base: f64,
    pub hp_per_fortification: f64,
}

impl WorkHp {
    pub fn at(&self, fort: f64) -> f64 {
        self.hp_base + self.hp_per_fortification * fort
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RamRules {
    /// Gate damage per second from a full-strength ram crew.
    pub damage_per_s: f64,
    /// SG4: a foot regiment of the attacker this close to the ram replaces
    /// its fallen crew (0: no relief).
    pub relief_range_m: f64,
    /// SG4: men passed per second from the regiment to the ram, up to its
    /// full crew.
    pub relief_men_per_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineRules {
    /// Wall damage per engine shot, per point of `siege_attack`.
    pub wall_damage_per_siege_attack: f64,
    /// Men struck down by one engine shot at a regiment, per point of
    /// `siege_attack`, full crew, point-blank, unarmoured.
    pub kills_per_siege_attack: f64,
    /// Share of the target's armour that still counts against a stone
    /// ball (0: ignored, 1: as against an arrow).
    pub armor_weight: f64,
    /// Morale a regiment loses to each engine shot that lands on it.
    pub morale_shock: f64,
}

/// Contents of `data/rules/siege_works.json` (schema
/// `data/schemas/siege_works_rules.schema.json`, SG3): resistance of the
/// walls and the gate, damage of the ram and the engines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiegeWorkRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub wall: WorkHp,
    pub gate: WorkHp,
    pub ram: RamRules,
    pub engine: EngineRules,
}

data_model::bundled_rules!(SiegeWorkRules, "rules/siege_works.json");

impl SiegeWorkRules {

    /// Wall and gate HP at fortification `fortification` (capped at 5).
    pub fn hp(&self, fortification: u32) -> (f64, f64) {
        let fort = f64::from(fortification.min(5));
        (self.wall.at(fort), self.gate.at(fort))
    }
}

/// What a piece of the ring is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PieceKind {
    Wall,
    Gate,
}

/// A straight stretch of wall (or the gate), from `a` to `b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallPiece {
    pub kind: PieceKind,
    pub a: (f64, f64),
    pub b: (f64, f64),
    pub hp: f64,
    pub max_hp: f64,
    /// Siege tower docked against this piece.
    #[serde(default)]
    pub docked_tower: Option<u32>,
    /// SB (ADR 0107): seconds this piece still counts as under attack (a ram
    /// battering it, an engine shooting at it, the gate burning); 0: not.
    #[serde(default)]
    pub attacked_for: f64,
}

/// SB: a ram at the gate, or the gate burning, keeps it "under attack" this
/// many seconds after the last tick of contact.
pub const UNDER_ATTACK_CONTACT_S: f64 = 4.0;
/// SB: an engine's shot keeps its piece "under attack" until a little after
/// its next shot (`shot::ENGINE_RELOAD` = 12 s), so a steady bombardment
/// reads as one continuous attack.
pub const UNDER_ATTACK_SHOT_S: f64 = 15.0;

impl WallPiece {
    pub fn intact(&self) -> bool {
        self.hp > 0.0
    }

    /// SB: battered, shot at or burning in the last few seconds.
    pub fn under_attack(&self) -> bool {
        self.attacked_for > 0.0 && self.intact()
    }

    /// SB: marks the piece as under attack for at least `seconds`.
    pub fn mark_attacked(&mut self, seconds: f64) {
        self.attacked_for = self.attacked_for.max(seconds);
    }

    pub fn length(&self) -> f64 {
        ((self.b.0 - self.a.0).powi(2) + (self.b.1 - self.a.1).powi(2)).sqrt()
    }

    pub fn midpoint(&self) -> (f64, f64) {
        ((self.a.0 + self.b.0) * 0.5, (self.a.1 + self.b.1) * 0.5)
    }

    /// Unit vector from `a` to `b`.
    pub fn tangent(&self) -> (f64, f64) {
        let len = self.length().max(1e-9);
        ((self.b.0 - self.a.0) / len, (self.b.1 - self.a.1) / len)
    }

    /// Unit normal pointing out of the town.
    pub fn outward(&self) -> (f64, f64) {
        let (tx, tz) = self.tangent();
        let (mx, mz) = self.midpoint();
        let n = (tz, -tx);
        let (cx, cz) = (mx - TOWN_CENTER.0, mz - TOWN_CENTER.1);
        if n.0 * cx + n.1 * cz >= 0.0 {
            n
        } else {
            (-n.0, -n.1)
        }
    }

    /// Distance from (x, z) to the piece's centre line segment.
    pub fn distance(&self, x: f64, z: f64) -> f64 {
        let (px, pz) = self.closest_point(x, z);
        ((x - px).powi(2) + (z - pz).powi(2)).sqrt()
    }

    pub fn closest_point(&self, x: f64, z: f64) -> (f64, f64) {
        let (dx, dz) = (self.b.0 - self.a.0, self.b.1 - self.a.1);
        let len2 = (dx * dx + dz * dz).max(1e-9);
        let t = (((x - self.a.0) * dx + (z - self.a.1) * dz) / len2).clamp(0.0, 1.0);
        (self.a.0 + dx * t, self.a.1 + dz * t)
    }

    /// Signed distance along the outward normal (positive outside).
    pub fn outside_offset(&self, x: f64, z: f64) -> f64 {
        let (nx, nz) = self.outward();
        let (mx, mz) = self.midpoint();
        (x - mx) * nx + (z - mz) * nz
    }

    /// `true` when segment `p`-`q` crosses the piece's centre line.
    pub fn crossed_by(&self, p: (f64, f64), q: (f64, f64)) -> bool {
        segments_intersect(p, q, self.a, self.b)
    }
}

/// A tower on the ring (rendering and a landmark for the AI).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tower {
    pub x: f64,
    pub z: f64,
    pub radius: f64,
    pub height: f64,
}

/// A house block inside the walls (F5a): an obstacle for movement and
/// pathing. Streets run between them from the square to every wall piece.
/// BR3: the block is an oriented rectangle (two rows of town houses back to
/// back, [`Footprint`]) — what the renderer draws, what the regiments and
/// the figures go round; the disc of `radius` metres drives the fire.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct House {
    pub x: f64,
    pub z: f64,
    pub radius: f64,
    /// Fire state (S2): a burnt house is a ruin that no longer blocks.
    #[serde(default)]
    pub fire: Blaze,
    /// A suburb house outside the walls (S2).
    #[serde(default)]
    pub suburb: bool,
    /// BR3: size of the block along its frontage (0: a disc, older saves).
    #[serde(default)]
    pub length: f64,
    /// BR3: depth of the block (both rows).
    #[serde(default)]
    pub depth: f64,
    /// BR3: frontage along `(cos yaw, sin yaw)`, row 0 facing
    /// `(-sin yaw, cos yaw)` (towards the square).
    #[serde(default)]
    pub yaw: f64,
    /// BR3: the parish church (a single building, not a block of houses).
    #[serde(default)]
    pub church: bool,
    /// BR3: rows of houses in the block (2 back to back, 1 against the
    /// wall walk).
    #[serde(default = "two_rows")]
    pub rows: u8,
    /// NT1 (ADR 0126): the keep of a castle (a stone tower: it blocks like
    /// a block but never burns).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keep: bool,
    /// NT8: height of a keep in metres from the ground (0: the renderer's
    /// own height, every other building).
    #[serde(default, skip_serializing_if = "zero_height")]
    pub height: f64,
    /// NT11: a keep roofed with a crenellated terrace (else a pavilion
    /// roof), drawn from the place's seed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub terrace: bool,
}

fn two_rows() -> u8 {
    2
}

fn zero_height(value: &f64) -> bool {
    *value == 0.0
}

impl House {
    /// A block of `length` × `depth` metres; its fire disc has the same
    /// area.
    pub fn block(x: f64, z: f64, length: f64, depth: f64, yaw: f64) -> Self {
        House {
            x,
            z,
            radius: (length * depth / std::f64::consts::PI).sqrt(),
            fire: Blaze::default(),
            suburb: false,
            length,
            depth,
            yaw,
            church: false,
            rows: 2,
            keep: false,
            height: 0.0,
            terrace: false,
        }
    }

    /// The house still blocks movement and pathing (not a burnt ruin).
    pub fn standing(&self) -> bool {
        !self.fire.burnt()
    }

    /// Distance from (x, z) to the edge of the house (negative inside):
    /// BR3: its rectangle, the disc for a house without footprint.
    pub fn edge_distance(&self, x: f64, z: f64) -> f64 {
        if self.has_footprint() {
            self.footprint().signed_distance(x, z)
        } else {
            ((x - self.x).powi(2) + (z - self.z).powi(2)).sqrt() - self.radius
        }
    }

    /// BR3: the house has a rectangle (not an older disc-only house).
    pub fn has_footprint(&self) -> bool {
        self.length > 0.0 && self.depth > 0.0
    }

    /// BR3: gap between two houses (fire spread): between their rectangles,
    /// else between their discs.
    pub fn gap_to(&self, other: &House) -> f64 {
        if self.has_footprint() && other.has_footprint() {
            self.footprint().distance_to(&other.footprint())
        } else {
            ((self.x - other.x).powi(2) + (self.z - other.z).powi(2)).sqrt()
                - self.radius
                - other.radius
        }
    }

    /// BR3: the rectangle of the block (a square round the disc when the
    /// house has no footprint).
    pub fn footprint(&self) -> Footprint {
        if self.has_footprint() {
            Footprint::new(self.x, self.z, self.length, self.depth, self.yaw)
        } else {
            let side = self.radius * 2.0;
            Footprint::new(self.x, self.z, side, side, self.yaw)
        }
    }
}

/// Where the blocks of a town may stand (BR3): off the streets, the square,
/// the wall walk and the towers, and apart from each other.
pub(crate) struct TownPlan<'a> {
    pub center: (f64, f64),
    /// Keep-out distance from the centre (square and its street).
    pub square: f64,
    /// Street centre lines (polylines) and the half width kept free.
    pub streets: Vec<Vec<(f64, f64)>>,
    pub street_half: f64,
    pub pieces: &'a [WallPiece],
    pub towers: &'a [Tower],
    /// Free band from the wall centre line (wall band + wall walk).
    pub wall_clear: f64,
    /// Gap kept between two blocks.
    pub alley: f64,
    /// Ring of the walls.
    pub ring: &'a [(f64, f64)],
}

impl TownPlan<'_> {
    /// Single rows of houses backing onto the wall walk, parallel to each
    /// wall piece (not the gate), clear of the towers.
    pub fn wall_rows(&self, houses: &mut Vec<House>, depth: f64, min: f64, max: f64) {
        let end = self.towers.iter().map(|t| t.radius).fold(0.0, f64::max) + self.alley;
        for piece in self.pieces {
            if piece.kind == PieceKind::Gate {
                continue;
            }
            let (nx, nz) = piece.outward();
            let inset = self.wall_clear + depth * 0.5 + 0.05;
            self.row_along(
                houses,
                (piece.a, piece.b),
                (-nx * inset, -nz * inset),
                end,
                depth,
                (min, max),
            );
        }
    }

    /// Rows of houses on both sides of every street of the plan, facing it.
    pub fn street_rows(&self, houses: &mut Vec<House>, depth: f64, min: f64, max: f64) {
        let streets = self.streets.clone();
        for line in &streets {
            for w in line.windows(2) {
                let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                let l = dx.hypot(dz);
                if l < min {
                    continue;
                }
                let off = self.street_half + depth * 0.5 + 0.05;
                for side in [1.0, -1.0] {
                    let (nx, nz) = (-dz / l * side, dx / l * side);
                    self.row_along(
                        houses,
                        (w[0], w[1]),
                        (nx * off, nz * off),
                        0.0,
                        depth,
                        (min, max),
                    );
                }
            }
        }
    }

    /// One row of single-row blocks along `line` shifted by `offset`, its
    /// fronts facing the line, between the streets that cross it (and
    /// `end` metres off both ends), where they fit among the blocks
    /// already laid; a stretch that does not fit is halved.
    fn row_along(
        &self,
        houses: &mut Vec<House>,
        line: ((f64, f64), (f64, f64)),
        offset: (f64, f64),
        end: f64,
        depth: f64,
        (min, max): (f64, f64),
    ) {
        let (a, b) = line;
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        if len < 1e-6 {
            return;
        }
        let (tx, tz) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        let off = offset.0.hypot(offset.1).max(1e-9);
        let yaw = yaw_facing(-offset.0 / off, -offset.1 / off);
        let origin = (a.0 + offset.0, a.1 + offset.1);
        let mut blocked: Vec<(f64, f64)> = vec![(-1e9, end), (len - end, 1e9)];
        for street in &self.streets {
            for w in street.windows(2) {
                let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                let l = dx.hypot(dz);
                if l < 1e-6 {
                    continue;
                }
                let (ux, uz) = (dx / l, dz / l);
                let sin = (ux * tz - uz * tx).abs();
                if sin < 0.2 {
                    continue;
                }
                // Crossing of the street with the row line.
                let (ox, oz) = (w[0].0 - origin.0, w[0].1 - origin.1);
                let denom = tx * uz - tz * ux;
                let t = (ox * uz - oz * ux) / denom;
                let s = (ox * tz - oz * tx) / denom;
                if s < -depth || s > l + depth {
                    continue;
                }
                let cos = (ux * tx + uz * tz).abs();
                let half = (self.street_half + depth * 0.5 * cos) / sin + 0.5;
                blocked.push((t - half, t + half));
            }
        }
        blocked.sort_by(|p, q| p.0.total_cmp(&q.0));
        let mut free: Vec<(f64, f64)> = Vec::new();
        let mut at = f64::NEG_INFINITY;
        for (b0, b1) in blocked {
            if at.is_finite() && b0 > at {
                free.push((at, b0));
            }
            at = at.max(b1);
        }
        let mut stretches: Vec<(f64, f64)> = Vec::new();
        for (f0, f1) in free {
            let count = ((f1 - f0) / (max + self.alley)).ceil().max(1.0) as usize;
            let step = (f1 - f0) / count as f64;
            stretches
                .extend((0..count).map(|k| (f0 + k as f64 * step, f0 + (k + 1) as f64 * step)));
        }
        stretches.reverse();
        while let Some((s0, s1)) = stretches.pop() {
            let frontage = s1 - s0 - self.alley;
            if frontage < min {
                continue;
            }
            let along = (s0 + s1) * 0.5;
            let (x, z) = (origin.0 + tx * along, origin.1 + tz * along);
            let fp = Footprint::new(x, z, frontage, depth, yaw);
            if self.fits(&fp, houses) {
                let mut house = House::block(x, z, frontage, depth, yaw);
                house.rows = 1;
                houses.push(house);
            } else {
                let mid = (s0 + s1) * 0.5;
                stretches.push((mid, s1));
                stretches.push((s0, mid));
            }
        }
    }

    /// `fp` keeps clear of the square, the streets, the walls, the towers
    /// and the blocks already laid (`placed`), and lies inside the ring.
    pub fn fits(&self, fp: &Footprint, placed: &[House]) -> bool {
        if fp
            .corners()
            .iter()
            .any(|&(x, z)| !point_in_ring(self.ring, x, z))
        {
            return false;
        }
        if fp.signed_distance(self.center.0, self.center.1) < self.square {
            return false;
        }
        if self.streets.iter().any(|line| {
            line.windows(2)
                .any(|w| fp.distance_to_segment(w[0], w[1]) < self.street_half - 1e-6)
        }) {
            return false;
        }
        if self
            .pieces
            .iter()
            .any(|p| fp.distance_to_segment(p.a, p.b) < self.wall_clear)
        {
            return false;
        }
        if self
            .towers
            .iter()
            .any(|t| fp.signed_distance(t.x, t.z) < t.radius + self.alley)
        {
            return false;
        }
        let reach = fp.bounding_radius();
        placed.iter().all(|h| {
            let other = h.footprint();
            (h.x - fp.x).hypot(h.z - fp.z) > reach + other.bounding_radius() + self.alley
                || !overlaps(fp, &other, self.alley)
        })
    }
}

/// Two rectangles closer than `gap`.
pub(crate) fn overlaps(a: &Footprint, b: &Footprint, gap: f64) -> bool {
    (a.x - b.x).hypot(a.z - b.z) < a.bounding_radius() + b.bounding_radius() + gap
        && a.distance_to(b) < gap
}

/// Point-in-polygon test on a ring of vertices.
pub(crate) fn point_in_ring(vertices: &[(f64, f64)], x: f64, z: f64) -> bool {
    let n = vertices.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, zi) = vertices[i];
        let (xj, zj) = vertices[j];
        if (zi > z) != (zj > z) && x < (xj - xi) * (z - zi) / (zj - zi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Yaw of a block whose row 0 faces `(fx, fz)` (unit vector).
pub(crate) fn yaw_facing(fx: f64, fz: f64) -> f64 {
    (-fx).atan2(fz)
}

/// BR3: dense rings of oriented blocks between the radial streets that run
/// from the square to the middle of every wall piece, off the wall walk
/// (deterministic, no random draw; layout from `data/rules/siege_town.json`).
/// In each ring, a sector is filled by blocks of equal frontage (between the
/// minimum and the maximum of the rules) separated by alleys; a block that
/// runs into the wall walk keeps only its inner row.
fn build_houses(plan: &TownPlan, rules: &TownRules) -> Vec<House> {
    let g = &rules.generic;
    let block = &rules.block;
    let center = plan.center;
    // The houses along the rampart street first, then the rings.
    let mut houses: Vec<House> = Vec::new();
    plan.wall_rows(
        &mut houses,
        block.depth_m * 0.5,
        block.min_frontage_m,
        block.max_frontage_m,
    );
    let depth = block.depth_m;
    let mut streets: Vec<f64> = plan
        .pieces
        .iter()
        .map(|p| {
            let (mx, mz) = p.midpoint();
            (mx - center.0).atan2(mz - center.1)
        })
        .collect();
    streets.sort_by(f64::total_cmp);
    let tau = std::f64::consts::TAU;
    for &r in &g.rings_m {
        let r_in = r - depth * 0.5;
        if r_in <= 1.0 {
            continue;
        }
        for (k, &a) in streets.iter().enumerate() {
            let next = streets.get(k + 1).copied().unwrap_or(streets[0] + tau);
            let gap = next - a;
            // Angles seen from the centre at the inner edge of the ring.
            let beta = |width: f64| (width * 0.5 / r_in).min(1.0).asin();
            let free = gap - 2.0 * beta(g.main_street_m);
            if free <= 0.0 {
                continue;
            }
            let span = |n: usize| (free - (n - 1) as f64 * 2.0 * beta(g.alley_m)) / n as f64;
            let frontage = |theta: f64| 2.0 * r_in * (theta * 0.5).tan();
            let mut n = 1;
            while frontage(span(n)) > block.max_frontage_m {
                n += 1;
            }
            let theta = span(n);
            let length = frontage(theta);
            if theta <= 0.0 || length < block.min_frontage_m {
                continue;
            }
            for j in 0..n {
                let angle = a
                    + beta(g.main_street_m)
                    + theta * 0.5
                    + j as f64 * (theta + 2.0 * beta(g.alley_m));
                let (sin, cos) = angle.sin_cos();
                // Row 0 faces the square.
                let yaw = yaw_facing(-sin, -cos);
                for rows in [2_u8, 1] {
                    let d = depth * f64::from(rows) * 0.5;
                    let rc = r_in + d * 0.5;
                    let (x, z) = (center.0 + sin * rc, center.1 + cos * rc);
                    let fp = Footprint::new(x, z, length, d, yaw);
                    if plan.fits(&fp, &houses) {
                        let mut house = House::block(x, z, length, d, yaw);
                        house.rows = rows;
                        houses.push(house);
                        break;
                    }
                }
            }
        }
    }
    houses
}

/// BR3: the parish church replaces the block nearest a point towards the
/// back of the town (+z) where the whole church fits; the blocks it would
/// touch are dropped. Its fire disc covers its nave.
pub(crate) fn place_church(houses: &mut Vec<House>, plan: &TownPlan, rules: &TownRules) {
    let c = &rules.church;
    let target = (plan.center.0, plan.center.1 + c.toward_back_m);
    let mut order: Vec<usize> = (0..houses.len()).filter(|&i| !houses[i].suburb).collect();
    order.sort_by(|&a, &b| {
        let d = |i: usize| (houses[i].x - target.0).hypot(houses[i].z - target.1);
        d(a).total_cmp(&d(b)).then(a.cmp(&b))
    });
    for i in order {
        let h = houses[i];
        let fp = Footprint::new(h.x, h.z, c.length_m, c.depth_m, h.yaw);
        if !plan.fits(&fp, &[]) {
            continue;
        }
        let mut church = House::block(h.x, h.z, c.length_m, c.depth_m, h.yaw);
        church.church = true;
        church.rows = 1;
        houses.retain(|o| o.suburb || !overlaps(&fp, &o.footprint(), plan.alley));
        houses.push(church);
        return;
    }
}

/// The town walls during a siege battle (mutable: pieces lose HP).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiegeWorks {
    pub fortification: u32,
    pub center: (f64, f64),
    /// Ring corners in order around the town.
    pub vertices: Vec<(f64, f64)>,
    pub pieces: Vec<WallPiece>,
    pub towers: Vec<Tower>,
    /// Wall thickness in metres.
    pub thickness: f64,
    /// Wall-walk height above the ground.
    pub wall_height: f64,
    pub square_radius: f64,
    /// Index of the gate in `pieces`.
    pub gate: usize,
    /// Seconds the attacker has held the central square (T4: mirror of the
    /// square capture point's progress, see [`crate::capture`]).
    pub hold_time: f64,
    /// T4 (ADR 0108): capture points (market square, gate), built at the
    /// first step of the battle.
    #[serde(default)]
    pub points: Vec<crate::capture::CapturePoint>,
    /// House blocks inside the walls (F5a).
    #[serde(default)]
    pub houses: Vec<House>,
    /// The garrison sallies out: the gate lets its regiments through (F5a).
    #[serde(default)]
    pub sortie: bool,
    /// Fire of the (wooden) gate (S2).
    #[serde(default)]
    pub gate_fire: Blaze,
    /// Wind of the fires (S2): direction × strength (0-1), x and z.
    #[serde(default)]
    pub wind: (f64, f64),
    /// Town drawn from a landmark plan (L3, ADR 0026); `None`: generic town.
    #[serde(default)]
    pub landmark: Option<crate::siege_layout::SiegeLandmark>,
    /// BR3: street furniture (façades, suburbs, market square).
    #[serde(default)]
    pub props: Vec<Prop>,
    /// NT1 (ADR 0126): kind of place (a landmark city is a `City`).
    #[serde(
        default,
        skip_serializing_if = "crate::siege_layouts::PlaceKind::is_city"
    )]
    pub place: crate::siege_layouts::PlaceKind,
    /// NT1: streets of a borough or castle (centre lines inside the walls,
    /// the first one from before the gate); empty for the ring city and a
    /// landmark (whose streets are in `landmark`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub streets: Vec<Vec<(f64, f64)>>,
}

fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Proper intersection of segments `p1-p2` and `q1-q2`.
pub fn segments_intersect(p1: (f64, f64), p2: (f64, f64), q1: (f64, f64), q2: (f64, f64)) -> bool {
    let d1 = cross(q1, q2, p1);
    let d2 = cross(q1, q2, p2);
    let d3 = cross(p1, p2, q1);
    let d4 = cross(p1, p2, q2);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

impl SiegeWorks {
    /// Builds the ring for `fortification` (0-3+), with `breach` percent
    /// (campaign siege damage) already done: from 50 one stretch facing the
    /// attacker is open, from 85 two; the other pieces lose up to 40 % HP.
    pub fn generate(fortification: u32, breach: u8, rng: &mut BattleRng) -> Self {
        let fort = f64::from(fortification.min(5));
        let thickness = 2.5 + 0.5 * fort;
        let wall_height = 6.0 + 1.5 * fort;
        let (wall_hp, gate_hp) = SiegeWorkRules::bundled().hp(fortification);
        let step = std::f64::consts::TAU / RING_SIDES as f64;
        // Vertex k at angle 22.5° + 45° k (x = sin, z = cos): side 3→4 faces -z.
        let vertices: Vec<(f64, f64)> = (0..RING_SIDES)
            .map(|k| {
                let angle = step * 0.5 + step * k as f64;
                let radius = RING_RADIUS * rng.range(0.92, 1.08);
                (
                    TOWN_CENTER.0 + angle.sin() * radius,
                    TOWN_CENTER.1 + angle.cos() * radius,
                )
            })
            .collect();
        let mut pieces = Vec::new();
        let mut gate = 0;
        for k in 0..RING_SIDES {
            let a = vertices[k];
            let b = vertices[(k + 1) % RING_SIDES];
            let wall = |a, b| WallPiece {
                kind: PieceKind::Wall,
                a,
                b,
                hp: wall_hp,
                max_hp: wall_hp,
                docked_tower: None,
                attacked_for: 0.0,
            };
            if k == RING_SIDES / 2 - 1 {
                // The side facing the attacker: wall, gate, wall.
                let mid = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
                let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
                let t = ((b.0 - a.0) / len, (b.1 - a.1) / len);
                let g0 = (
                    mid.0 - t.0 * GATE_WIDTH * 0.5,
                    mid.1 - t.1 * GATE_WIDTH * 0.5,
                );
                let g1 = (
                    mid.0 + t.0 * GATE_WIDTH * 0.5,
                    mid.1 + t.1 * GATE_WIDTH * 0.5,
                );
                pieces.push(wall(a, g0));
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
                pieces.push(wall(g1, b));
            } else {
                pieces.push(wall(a, b));
            }
        }
        let tower_radius = 5.0 + fort;
        let mut towers: Vec<Tower> = vertices
            .iter()
            .map(|&(x, z)| Tower {
                x,
                z,
                radius: tower_radius,
                height: wall_height + 4.0,
            })
            .collect();
        for end in [pieces[gate].a, pieces[gate].b] {
            towers.push(Tower {
                x: end.0,
                z: end.1,
                radius: tower_radius * 0.8,
                height: wall_height + 3.0,
            });
        }
        let mut works = SiegeWorks {
            houses: Vec::new(),
            props: Vec::new(),
            sortie: false,
            gate_fire: Blaze::default(),
            wind: (0.0, 0.0),
            fortification,
            center: TOWN_CENTER,
            vertices,
            pieces,
            towers,
            thickness,
            wall_height,
            square_radius: SQUARE_RADIUS,
            gate,
            hold_time: 0.0,
            points: Vec::new(),
            landmark: None,
            place: crate::siege_layouts::PlaceKind::City,
            streets: Vec::new(),
        };
        works.lay_generic_town(TownRules::bundled());
        works.apply_campaign_breach(breach, rng);
        works
    }

    /// BR3: the dense blocks, the church and the props of the generic town.
    pub(crate) fn lay_generic_town(&mut self, rules: &TownRules) {
        let houses = {
            let g = &rules.generic;
            let plan = TownPlan {
                center: self.center,
                square: self.square_radius + g.square_street_m,
                streets: self
                    .pieces
                    .iter()
                    .map(|p| {
                        // Radial, from the square to just past the wall.
                        let (mx, mz) = p.midpoint();
                        let (dx, dz) = (mx - self.center.0, mz - self.center.1);
                        let k = 1.0 + 8.0 / dx.hypot(dz).max(1.0);
                        vec![
                            self.center,
                            (self.center.0 + dx * k, self.center.1 + dz * k),
                        ]
                    })
                    .collect(),
                street_half: g.main_street_m * 0.5,
                pieces: &self.pieces,
                towers: &self.towers,
                wall_clear: self.band() + g.wall_walk_m,
                alley: g.alley_m,
                ring: &self.vertices,
            };
            let mut houses = build_houses(&plan, rules);
            place_church(&mut houses, &plan, rules);
            houses
        };
        self.houses = houses;
        self.lay_props();
    }

    /// BR3: lays the street furniture out again from the houses (after the
    /// suburbs are added). Deterministic, no random draw.
    pub fn lay_props(&mut self) {
        self.props = crate::props::siege_props(self, TownRules::bundled());
    }

    /// BR3: the props still standing (those of a burnt house are gone).
    pub fn standing_props(&self) -> impl Iterator<Item = &Prop> {
        self.props.iter().filter(|p| {
            p.house
                .is_none_or(|h| self.houses.get(h).is_none_or(House::standing))
        })
    }

    /// BR3: the church, if the town has one.
    pub fn church(&self) -> Option<usize> {
        self.houses.iter().position(|h| h.church)
    }

    /// Wears the walls by the campaign `breach` (0-100) and opens one gap
    /// from [`BREACH_ONE_GAP`], two from [`BREACH_TWO_GAPS`].
    pub(crate) fn apply_campaign_breach(&mut self, breach: u8, rng: &mut BattleRng) {
        if breach == 0 {
            return;
        }
        let wear = f64::from(breach.min(100)) / 100.0 * 0.4;
        for piece in &mut self.pieces {
            if piece.kind == PieceKind::Wall {
                piece.hp = (piece.max_hp * (1.0 - wear)).round();
            }
        }
        let open = if breach >= BREACH_TWO_GAPS {
            2
        } else if breach >= BREACH_ONE_GAP {
            1
        } else {
            0
        };
        let mut front = self.front_walls();
        for _ in 0..open.min(front.len()) {
            let k = rng.below(front.len() as u32) as usize;
            let piece = front.remove(k);
            self.pieces[piece].hp = 0.0;
        }
    }

    /// Wall pieces (not the gate) facing the attacker (outward normal
    /// pointing towards -z), nearest first.
    pub fn front_walls(&self) -> Vec<usize> {
        let mut front: Vec<usize> = (0..self.pieces.len())
            .filter(|&i| {
                self.pieces[i].kind == PieceKind::Wall && self.pieces[i].outward().1 < -0.3
            })
            .collect();
        front.sort_by(|&a, &b| {
            self.pieces[a]
                .midpoint()
                .1
                .total_cmp(&self.pieces[b].midpoint().1)
                .then(a.cmp(&b))
        });
        front
    }

    /// Point-in-polygon test (inside the ring).
    pub fn inside(&self, x: f64, z: f64) -> bool {
        point_in_ring(&self.vertices, x, z)
    }

    pub fn in_square(&self, x: f64, z: f64) -> bool {
        (x - self.center.0).powi(2) + (z - self.center.1).powi(2)
            <= self.square_radius * self.square_radius
    }

    /// Half-width of the band around an intact piece that blocks regiments.
    pub fn band(&self) -> f64 {
        self.thickness * 0.5 + 1.5
    }

    /// First intact piece crossed by the segment `p`-`q`.
    pub fn crosses_intact(&self, p: (f64, f64), q: (f64, f64)) -> Option<usize> {
        (0..self.pieces.len()).find(|&i| self.pieces[i].intact() && self.pieces[i].crossed_by(p, q))
    }

    /// `true` when a regiment walking from `p` to `q` would run into an
    /// intact piece: the path crosses one, or grazes the end of one (the
    /// jamb of a gate or breach) closer than the blocking band.
    pub fn path_blocked(&self, p: (f64, f64), q: (f64, f64)) -> bool {
        if self.crosses_intact(p, q).is_some() {
            return true;
        }
        let margin = self.band() + 1.0;
        let path = WallPiece {
            kind: PieceKind::Wall,
            a: p,
            b: q,
            hp: 0.0,
            max_hp: 0.0,
            docked_tower: None,
            attacked_for: 0.0,
        };
        let start_clear =
            |end: (f64, f64)| ((end.0 - p.0).powi(2) + (end.1 - p.1).powi(2)).sqrt() > margin;
        self.pieces.iter().filter(|w| w.intact()).any(|w| {
            [w.a, w.b]
                .into_iter()
                .any(|end| start_clear(end) && path.distance(end.0, end.1) < margin)
        })
    }

    /// Nearest intact piece to (x, z) and its distance.
    pub fn nearest_intact(&self, x: f64, z: f64) -> Option<(usize, f64)> {
        (0..self.pieces.len())
            .filter(|&i| self.pieces[i].intact())
            .map(|i| (i, self.pieces[i].distance(x, z)))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
    }

    /// Open pieces (breaches and a broken gate).
    pub fn openings(&self) -> Vec<usize> {
        (0..self.pieces.len())
            .filter(|&i| !self.pieces[i].intact())
            .collect()
    }

    /// The opening minimising the detour from `from` to `to`.
    pub fn best_opening(&self, from: (f64, f64), to: (f64, f64)) -> Option<usize> {
        let dist =
            |a: (f64, f64), b: (f64, f64)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        self.openings().into_iter().min_by(|&a, &b| {
            let da = dist(from, self.pieces[a].midpoint()) + dist(self.pieces[a].midpoint(), to);
            let db = dist(from, self.pieces[b].midpoint()) + dist(self.pieces[b].midpoint(), to);
            da.total_cmp(&db).then(a.cmp(&b))
        })
    }

    /// Next waypoint to go from `from` through opening `piece`: first to a
    /// point in front of it, then through to the other side.
    pub fn waypoint_through(&self, piece: usize, from: (f64, f64)) -> (f64, f64) {
        let p = &self.pieces[piece];
        let (mx, mz) = p.midpoint();
        let (nx, nz) = p.outward();
        let (tx, tz) = p.tangent();
        let offset = p.outside_offset(from.0, from.1);
        let side = if offset >= 0.0 { 1.0 } else { -1.0 };
        let lateral = ((from.0 - mx) * tx + (from.1 - mz) * tz).abs();
        let aligned = lateral < (p.length() * 0.5 - 3.0).max(2.0);
        if !aligned {
            (mx + nx * side * 18.0, mz + nz * side * 18.0)
        } else {
            (mx - nx * side * 20.0, mz - nz * side * 20.0)
        }
    }

    /// The standing house (not a burnt ruin) whose block (BR3: its
    /// rectangle, grown by `margin`) contains (x, z).
    pub fn house_at(&self, x: f64, z: f64, margin: f64) -> Option<usize> {
        self.houses.iter().position(|h| {
            if !h.standing() {
                return false;
            }
            let f = h.footprint();
            (x - h.x).hypot(z - h.z) < f.bounding_radius() + margin.max(0.0)
                && f.contains(x, z, margin)
        })
    }

    /// BR3: props that stop the regiments (the stalls, carts and barrels of
    /// the market square; the well and the props against the houses only
    /// stop the figures — the latter stand in the houses' clearance).
    pub fn path_props(&self) -> impl Iterator<Item = &Prop> {
        self.standing_props()
            .filter(|p| p.on_square() && p.kind != crate::town::PropKind::Well)
    }

    /// BR3: a path-blocking prop (grown by `margin`) contains (x, z).
    pub fn prop_at(&self, x: f64, z: f64, margin: f64) -> bool {
        self.path_props().any(|p| {
            let f = p.footprint();
            (x - p.x).hypot(z - p.z) < f.bounding_radius() + margin.max(0.0)
                && f.contains(x, z, margin)
        })
    }

    /// Houses burnt to the ground (S2).
    pub fn burnt_houses(&self) -> usize {
        self.houses.iter().filter(|h| h.fire.burnt()).count()
    }

    /// Houses on fire (S2).
    pub fn burning_houses(&self) -> usize {
        self.houses.iter().filter(|h| h.fire.burning()).count()
    }

    /// Share of the wall ring still standing (HUD).
    pub fn integrity(&self) -> f64 {
        let (hp, max) = self
            .pieces
            .iter()
            .fold((0.0, 0.0), |(h, m), p| (h + p.hp.max(0.0), m + p.max_hp));
        if max > 0.0 {
            hp / max
        } else {
            0.0
        }
    }
}
