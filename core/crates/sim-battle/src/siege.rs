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
/// Seconds the attacker must hold the square to take the town.
pub const HOLD_TO_WIN: f64 = 60.0;
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineRules {
    /// Wall damage per engine shot, per point of `siege_attack`.
    pub wall_damage_per_siege_attack: f64,
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

const BUNDLED_WORKS: &str = include_str!("../../../../data/rules/siege_works.json");

impl SiegeWorkRules {
    /// `data/rules/siege_works.json` as compiled into the crate.
    pub fn bundled() -> &'static SiegeWorkRules {
        static RULES: std::sync::OnceLock<SiegeWorkRules> = std::sync::OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED_WORKS).expect("data/rules/siege_works.json is valid")
        })
    }

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
}

impl WallPiece {
    pub fn intact(&self) -> bool {
        self.hp > 0.0
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
/// pathing, a disc of `radius` metres. Streets run between them from the
/// square to every wall piece.
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
}

impl House {
    /// The house still blocks movement and pathing (not a burnt ruin).
    pub fn standing(&self) -> bool {
        !self.fire.burnt()
    }

    /// Distance from (x, z) to the edge of the house disc (negative inside).
    pub fn edge_distance(&self, x: f64, z: f64) -> f64 {
        ((x - self.x).powi(2) + (z - self.z).powi(2)).sqrt() - self.radius
    }
}

/// Radii (from the town centre) of the two rings of houses.
pub const HOUSE_RINGS: [f64; 2] = [75.0, 110.0];
/// Radius of a house block.
pub const HOUSE_RADIUS: f64 = 9.0;
/// Street width kept free between two blocks.
pub const STREET_WIDTH: f64 = 12.0;

/// Houses between the radial streets (deterministic, no random draw).
fn build_houses(center: (f64, f64), pieces: &[WallPiece]) -> Vec<House> {
    let mut streets: Vec<f64> = pieces
        .iter()
        .map(|p| {
            let (mx, mz) = p.midpoint();
            (mx - center.0).atan2(mz - center.1)
        })
        .collect();
    streets.sort_by(f64::total_cmp);
    let tau = std::f64::consts::TAU;
    let mut houses = Vec::new();
    for (k, &a) in streets.iter().enumerate() {
        let next = streets.get(k + 1).copied().unwrap_or(streets[0] + tau);
        let gap = next - a;
        for r in HOUSE_RINGS {
            // Blocks spread evenly in the gap, streets on both ends.
            let usable = gap * r - STREET_WIDTH;
            let count = (usable / (2.0 * HOUSE_RADIUS + STREET_WIDTH)).floor() as i32;
            for j in 0..count.max(0) {
                let t = (f64::from(j) + 0.5) / f64::from(count);
                let angle = a + gap * t;
                houses.push(House {
                    x: center.0 + angle.sin() * r,
                    z: center.1 + angle.cos() * r,
                    radius: HOUSE_RADIUS,
                    fire: Blaze::default(),
                    suburb: false,
                });
            }
        }
    }
    houses
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
    /// Seconds the attacker has held the central square.
    pub hold_time: f64,
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
        let houses = build_houses(TOWN_CENTER, &pieces);
        let mut works = SiegeWorks {
            houses,
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
            landmark: None,
        };
        works.apply_campaign_breach(breach, rng);
        works
    }

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
        let open = if breach >= 85 {
            2
        } else if breach >= 50 {
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
        let n = self.vertices.len();
        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let (xi, zi) = self.vertices[i];
            let (xj, zj) = self.vertices[j];
            if (zi > z) != (zj > z) && x < (xj - xi) * (z - zi) / (zj - zi) + xi {
                inside = !inside;
            }
            j = i;
        }
        inside
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

    /// The standing house (not a burnt ruin) whose disc (plus `margin`)
    /// contains (x, z).
    pub fn house_at(&self, x: f64, z: f64, margin: f64) -> Option<usize> {
        self.houses.iter().position(|h| {
            h.standing() && (x - h.x).powi(2) + (z - h.z).powi(2) < (h.radius + margin).powi(2)
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
