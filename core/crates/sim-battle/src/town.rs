//! Dense besieged town and solid street furniture (lot BR3, ADR 0047).
//!
//! - [`TownRules`]: `data/rules/siege_town.json` (layout of the house blocks,
//!   footprints of the props, market, figure margin), embedded at compile
//!   time like the fire rules.
//! - [`Footprint`]: an oriented rectangle on the (x, z) plane — a house
//!   block, a village house or a prop — with the geometry the pathing and the
//!   figures need (containment, signed distance, distance to a segment, the
//!   nearest way out).
//! - [`Prop`]: a piece of street furniture laid by the core (stall, cart,
//!   barrels, woodpile, well), deterministic without touching the battle's
//!   random stream ([`crate::props`] lays them out).
//!
//! Orientation convention (shared with the village houses of [`crate::site`]
//! and the kit models): `yaw` turns the length axis from +x towards +z, so the
//! length runs along `(cos yaw, sin yaw)` and the front (the model's +Z, the
//! facade) faces `(-sin yaw, cos yaw)`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// An oriented rectangle: centre, half sizes along its own axes and yaw.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Footprint {
    pub x: f64,
    pub z: f64,
    /// Half size along the length axis `(cos yaw, sin yaw)`.
    pub half_length: f64,
    /// Half size along the front axis `(-sin yaw, cos yaw)`.
    pub half_depth: f64,
    pub yaw: f64,
}

impl Footprint {
    /// Rectangle of `length` × `depth` metres centred on (x, z).
    pub fn new(x: f64, z: f64, length: f64, depth: f64, yaw: f64) -> Self {
        Footprint {
            x,
            z,
            half_length: length * 0.5,
            half_depth: depth * 0.5,
            yaw,
        }
    }

    /// Unit vector of the length axis.
    pub fn axis(&self) -> (f64, f64) {
        (self.yaw.cos(), self.yaw.sin())
    }

    /// Unit vector the front faces.
    pub fn front(&self) -> (f64, f64) {
        (-self.yaw.sin(), self.yaw.cos())
    }

    /// Radius of the circle through the corners.
    pub fn bounding_radius(&self) -> f64 {
        self.half_length.hypot(self.half_depth)
    }

    /// (x, z) in the rectangle's frame: (along the length, along the front).
    pub fn local(&self, x: f64, z: f64) -> (f64, f64) {
        let (c, s) = (self.yaw.cos(), self.yaw.sin());
        let (dx, dz) = (x - self.x, z - self.z);
        (dx * c + dz * s, -dx * s + dz * c)
    }

    /// Back to world coordinates from the rectangle's frame.
    pub fn world(&self, u: f64, v: f64) -> (f64, f64) {
        let (c, s) = (self.yaw.cos(), self.yaw.sin());
        (self.x + u * c - v * s, self.z + u * s + v * c)
    }

    /// Corners, counter-clockwise in the local frame.
    pub fn corners(&self) -> [(f64, f64); 4] {
        let (l, d) = (self.half_length, self.half_depth);
        [
            self.world(-l, -d),
            self.world(l, -d),
            self.world(l, d),
            self.world(-l, d),
        ]
    }

    /// (x, z) lies in the rectangle grown by `margin` on every side.
    pub fn contains(&self, x: f64, z: f64, margin: f64) -> bool {
        let (u, v) = self.local(x, z);
        u.abs() < self.half_length + margin && v.abs() < self.half_depth + margin
    }

    /// Signed distance from (x, z) to the rectangle (negative inside).
    pub fn signed_distance(&self, x: f64, z: f64) -> f64 {
        let (u, v) = self.local(x, z);
        let (qu, qv) = (u.abs() - self.half_length, v.abs() - self.half_depth);
        let outside = qu.max(0.0).hypot(qv.max(0.0));
        outside + qu.max(qv).min(0.0)
    }

    /// Distance from the segment `a`-`b` to the rectangle (0 when they meet).
    pub fn distance_to_segment(&self, a: (f64, f64), b: (f64, f64)) -> f64 {
        let (au, av) = self.local(a.0, a.1);
        let (bu, bv) = self.local(b.0, b.1);
        let (l, d) = (self.half_length, self.half_depth);
        // Liang-Barsky clip against the box: the segment enters it.
        let (du, dv) = (bu - au, bv - av);
        let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
        let mut hits = true;
        for (p, q) in [(-du, au + l), (du, l - au), (-dv, av + d), (dv, d - av)] {
            if p.abs() < 1e-12 {
                if q < 0.0 {
                    hits = false;
                    break;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    t0 = t0.max(r);
                } else {
                    t1 = t1.min(r);
                }
                if t0 > t1 {
                    hits = false;
                    break;
                }
            }
        }
        if hits {
            return 0.0;
        }
        let point = |u: f64, v: f64| {
            let (qu, qv) = (u.abs() - l, v.abs() - d);
            qu.max(0.0).hypot(qv.max(0.0))
        };
        let corner = |cu: f64, cv: f64| {
            let len2 = (du * du + dv * dv).max(1e-12);
            let t = (((cu - au) * du + (cv - av) * dv) / len2).clamp(0.0, 1.0);
            (au + du * t - cu).hypot(av + dv * t - cv)
        };
        point(au, av)
            .min(point(bu, bv))
            .min(corner(-l, -d))
            .min(corner(l, -d))
            .min(corner(l, d))
            .min(corner(-l, d))
    }

    /// The two rectangles intersect (separating-axis test).
    pub fn intersects(&self, other: &Footprint) -> bool {
        let axes = [self.axis(), self.front(), other.axis(), other.front()];
        axes.iter().all(|&(ax, az)| {
            let project = |f: &Footprint| {
                let c = f.x * ax + f.z * az;
                let (u, v) = (f.axis(), f.front());
                let r = f.half_length * (u.0 * ax + u.1 * az).abs()
                    + f.half_depth * (v.0 * ax + v.1 * az).abs();
                (c - r, c + r)
            };
            let (a0, a1) = project(self);
            let (b0, b1) = project(other);
            a0 < b1 && b0 < a1
        })
    }

    /// Distance between two rectangles (0 when they meet).
    pub fn distance_to(&self, other: &Footprint) -> f64 {
        if self.intersects(other) {
            return 0.0;
        }
        let edges = |f: &Footprint, g: &Footprint| {
            let c = f.corners();
            (0..4)
                .map(|i| g.distance_to_segment(c[i], c[(i + 1) % 4]))
                .fold(f64::INFINITY, f64::min)
        };
        edges(self, other).min(edges(other, self))
    }

    /// The four ways out of the rectangle grown by `margin` for a point
    /// inside it, nearest first: straight out through each side.
    pub fn exits(&self, x: f64, z: f64, margin: f64) -> [(f64, (f64, f64)); 4] {
        let (u, v) = self.local(x, z);
        let (l, d) = (self.half_length + margin, self.half_depth + margin);
        let mut out = [
            (l - u, self.world(l, v)),
            (u + l, self.world(-l, v)),
            (d - v, self.world(u, d)),
            (v + d, self.world(u, -d)),
        ];
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out
    }
}

/// Kind of a piece of street furniture (kit model kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropKind {
    Stall,
    Cart,
    Barrels,
    Woodpile,
    Well,
}

impl PropKind {
    pub fn key(self) -> &'static str {
        match self {
            PropKind::Stall => "stall",
            PropKind::Cart => "cart",
            PropKind::Barrels => "barrels",
            PropKind::Woodpile => "woodpile",
            PropKind::Well => "well",
        }
    }
}

/// A piece of street furniture: a solid obstacle for the figures (and, on
/// the market square, for the pathing).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Prop {
    pub kind: PropKind,
    pub x: f64,
    pub z: f64,
    /// Length axis `(cos yaw, sin yaw)`, front `(-sin yaw, cos yaw)`.
    pub yaw: f64,
    pub length: f64,
    pub depth: f64,
    /// The house (block) it stands against: it goes when the house burns.
    #[serde(default)]
    pub house: Option<usize>,
}

impl Prop {
    pub fn footprint(&self) -> Footprint {
        Footprint::new(self.x, self.z, self.length, self.depth, self.yaw)
    }

    /// Stands on the market square (not against a house).
    pub fn on_square(&self) -> bool {
        self.house.is_none()
    }
}

/// Footprint of a prop kind (metres).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropSize {
    pub length_m: f64,
    pub depth_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockRules {
    pub depth_m: f64,
    pub min_frontage_m: f64,
    pub max_frontage_m: f64,
    pub clearance_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenericRules {
    pub rings_m: Vec<f64>,
    pub main_street_m: f64,
    pub alley_m: f64,
    pub square_street_m: f64,
    pub wall_walk_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandmarkRules {
    pub frontage_m: f64,
    pub pitch_x_m: f64,
    pub pitch_z_m: f64,
    pub lane_m: f64,
    pub square_street_m: f64,
    pub wall_walk_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChurchRules {
    pub length_m: f64,
    pub depth_m: f64,
    pub toward_back_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuburbRules {
    pub frontage_factor: f64,
    pub depth_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketRules {
    pub ring_factor: f64,
    pub stall_spacing_m: f64,
    pub stalls_per_group: [u32; 2],
    pub passage_m: f64,
    pub extra_chance: f64,
    pub extra_kinds: Vec<PropKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropRules {
    pub footprints: BTreeMap<PropKind, PropSize>,
    pub wall_gap_m: f64,
    pub block_row_chances: Vec<f64>,
    pub square_kinds: Vec<PropKind>,
    pub street_kinds: Vec<PropKind>,
    pub suburb_chance: f64,
    pub suburb_kinds: Vec<PropKind>,
    pub village_chance: f64,
    pub village_kinds: Vec<PropKind>,
    pub market: MarketRules,
    pub path_margin_m: f64,
}

impl PropRules {
    /// Footprint of `kind` (a 1 m square when the file lacks it).
    pub fn size(&self, kind: PropKind) -> PropSize {
        self.footprints.get(&kind).copied().unwrap_or(PropSize {
            length_m: 1.0,
            depth_m: 1.0,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FigureRules {
    pub margin_m: f64,
}

/// Contents of `data/rules/siege_town.json` (schema
/// `data/schemas/siege_town_rules.schema.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TownRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub block: BlockRules,
    pub generic: GenericRules,
    pub landmark: LandmarkRules,
    pub church: ChurchRules,
    pub suburb: SuburbRules,
    pub props: PropRules,
    pub figures: FigureRules,
    /// NT1 (ADR 0126): kinds of besieged places and their plans.
    pub places: crate::siege_layouts::PlaceRules,
}

data_model::bundled_rules!(TownRules, "rules/siege_town.json");

/// Deterministic draw in [0, 1) from a key and a salt (SplitMix64), for the
/// layout of the furniture: it never touches the battle's random stream.
pub fn hash01(key: u64, salt: u64) -> f64 {
    let mut z = key
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(salt.wrapping_mul(0xD1B5_4A32_D192_ED03));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_load() {
        let rules = TownRules::bundled();
        assert!(!rules.generic.rings_m.is_empty());
        assert_eq!(rules.props.footprints.len(), 5);
    }

    #[test]
    fn footprint_geometry() {
        let f = Footprint::new(10.0, 20.0, 8.0, 4.0, 0.5);
        assert!(f.contains(10.0, 20.0, 0.0));
        assert!((f.signed_distance(10.0, 20.0) + 2.0).abs() < 1e-9);
        let (fx, fz) = f.front();
        let p = (10.0 + fx * 5.0, 20.0 + fz * 5.0);
        assert!((f.signed_distance(p.0, p.1) - 3.0).abs() < 1e-9);
        assert!(f.distance_to_segment((0.0, 0.0), (20.0, 40.0)) == 0.0);
        let q = (10.0 + fx * 6.0, 20.0 + fz * 6.0);
        let (ax, az) = f.axis();
        let d = f.distance_to_segment(
            (q.0 - ax * 2.0, q.1 - az * 2.0),
            (q.0 + ax * 2.0, q.1 + az * 2.0),
        );
        assert!((d - 4.0).abs() < 1e-9, "{d}");
        let exits = f.exits(10.0 + ax * 3.0, 20.0 + az * 3.0, 0.5);
        assert!((exits[0].0 - 1.5).abs() < 1e-9);
        assert!(!f.contains(exits[0].1 .0, exits[0].1 .1, 0.49));
    }

    #[test]
    fn hash_is_uniform_enough() {
        let mean = (0..1000).map(|k| hash01(k, 7)).sum::<f64>() / 1000.0;
        assert!((mean - 0.5).abs() < 0.05);
    }
}
