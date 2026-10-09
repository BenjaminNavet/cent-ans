//! Battle site drawn from the campaign (lot B5): ground of the season (mud,
//! snow), coast on a flank, marsh pools, hedges,
//! fences and ditches, bocage hedgerows.
//!
//! These features are drawn from a stream *derived* from the battle RNG
//! without consuming it ([`BattleRng::derive`]): battles drawn before B5 keep
//! their hills, forests, river and every later random draw.

use data_model::key_enum;
use data_model::Terrain;
use serde::{Deserialize, Serialize};

use crate::field::{Weather, Zone};
use crate::rng::BattleRng;
use crate::scale::FieldSize;
use crate::setup::BattleSeason;
use crate::terrain_rules::TerrainRules;

key_enum! {
/// State of the ground at the time of the battle (season and weather).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ground {
    #[default]
    Dry => "dry",
    /// Soaked ground: extra mud zones.
    Muddy => "muddy",
    /// Snow on the ground: slower marches even when no snow falls.
    Snowy => "snowy",
}
}

impl Ground {
    pub fn label_fr(self) -> &'static str {
        match self {
            Ground::Dry => "Sol sec",
            Ground::Muddy => "Sol détrempé",
            Ground::Snowy => "Sol enneigé",
        }
    }

    /// Ground of a season and weather; `roll` in `[0, 1)` decides the
    /// uncertain cases (a muddy spring, a snowy winter without snowfall).
    pub fn draw(season: BattleSeason, weather: Weather, roll: f64) -> Ground {
        match (weather, season) {
            (Weather::Snow, _) => Ground::Snowy,
            (Weather::Rain, _) => Ground::Muddy,
            (_, BattleSeason::Winter) if roll < 0.45 => Ground::Snowy,
            (_, BattleSeason::Winter) if roll < 0.8 => Ground::Muddy,
            (_, BattleSeason::Spring | BattleSeason::Autumn) if roll < 0.3 => Ground::Muddy,
            _ => Ground::Dry,
        }
    }

    /// Multiplier on marching speed from the ground alone. Falling snow is
    /// already counted by the weather: snow on the ground only slows when
    /// the sky is not snowing too.
    pub fn speed_factor(self, weather: Weather) -> f64 {
        if self == Ground::Snowy && weather != Weather::Snow {
            0.9
        } else {
            1.0
        }
    }
}

key_enum! {
/// Side of the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flank {
    West => "west",
    East => "east",
}
}

/// Sea along one flank (coastal provinces): the waterline lies just outside
/// the field, a sand strip inside it slows the march.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coast {
    pub flank: Flank,
    /// x of the waterline (outside the field).
    pub shore_x: f64,
    /// Width of the sand strip inside the field, in metres.
    pub beach: f64,
    /// Width of the field (EP1: the east edge is at x = `field_width`).
    #[serde(default = "standard_width")]
    pub field_width: f64,
}

fn standard_width() -> f64 {
    FieldSize::STANDARD.width
}

impl Coast {
    /// Distance from `x` to the waterline, towards the land (negative at sea).
    pub fn inland(&self, x: f64) -> f64 {
        match self.flank {
            Flank::West => x - self.shore_x,
            Flank::East => self.shore_x - x,
        }
    }

    /// Distance from `x` to the field edge on the coastal flank, inland.
    pub fn from_edge(&self, x: f64) -> f64 {
        match self.flank {
            Flank::West => x,
            Flank::East => self.field_width - x,
        }
    }

    /// On the sand strip (between the waterline and the fields).
    pub fn on_beach(&self, x: f64) -> bool {
        self.inland(x) >= 0.0 && self.from_edge(x) <= self.beach
    }

    /// Speed multiplier on sand.
    pub const SAND_FACTOR: f64 = 0.85;
}

key_enum! {
/// Kind of a linear obstacle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObstacleKind {
    /// Hedgerow on a bank: cover against missiles, breaks charges, slows
    /// horses badly (the English archers' favourite position).
    Hedge => "hedge",
    /// Wattle fence of a croft or a pen: slows horses.
    Fence => "fence",
    /// Drainage ditch: slows everyone, breaks charges.
    Ditch => "ditch",
    /// CV3-2: low palisade of an entrenched camp: slows everyone badly,
    /// breaks charges, covers against missiles and shelters the defenders
    /// behind it in melee (`data/rules/battle_opening.json`).
    Palisade => "palisade",
}
}

impl ObstacleKind {
    /// Speed multiplier while crossing.
    pub fn crossing_factor(self, mounted: bool) -> f64 {
        match (self, mounted) {
            (ObstacleKind::Hedge, false) => 0.6,
            (ObstacleKind::Hedge, true) => 0.3,
            (ObstacleKind::Fence, false) => 0.85,
            (ObstacleKind::Fence, true) => 0.6,
            (ObstacleKind::Ditch, false) => 0.6,
            (ObstacleKind::Ditch, true) => 0.4,
            (ObstacleKind::Palisade, false) => {
                crate::opening::OpeningRules::bundled()
                    .palisade
                    .foot_crossing_factor
            }
            (ObstacleKind::Palisade, true) => {
                crate::opening::OpeningRules::bundled()
                    .palisade
                    .horse_crossing_factor
            }
        }
    }

    /// Missiles crossing it lose part of their effect on a unit behind.
    pub fn gives_cover(self) -> bool {
        matches!(self, ObstacleKind::Hedge | ObstacleKind::Palisade)
    }

    /// A cavalry charge across it loses its impact.
    pub fn breaks_charge(self) -> bool {
        matches!(
            self,
            ObstacleKind::Hedge | ObstacleKind::Ditch | ObstacleKind::Palisade
        )
    }
}

/// Distance within which a unit counts as crossing (or standing at) an
/// obstacle, in metres.
pub const OBSTACLE_REACH: f64 = 4.0;
/// A unit whose centre is within this distance behind a hedge is covered.
pub const HEDGE_COVER_REACH: f64 = 14.0;
/// Multiplier on missile casualties behind a hedge.
pub const HEDGE_COVER: f64 = 0.6;

/// A straight hedge, fence or ditch from `a` to `b` (x, z).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Obstacle {
    pub a: (f64, f64),
    pub b: (f64, f64),
    pub kind: ObstacleKind,
}

impl Obstacle {
    /// Distance from (x, z) to the segment.
    pub fn distance(&self, x: f64, z: f64) -> f64 {
        crate::geom::segment_distance((x, z), self.a, self.b)
    }

    /// `true` when the segment from `p` to `q` crosses the obstacle.
    pub fn crosses(&self, p: (f64, f64), q: (f64, f64)) -> bool {
        crate::geom::segments_intersect(p, q, self.a, self.b)
    }

    pub fn length(&self) -> f64 {
        ((self.b.0 - self.a.0).powi(2) + (self.b.1 - self.a.1).powi(2)).sqrt()
    }

    fn midpoint(&self) -> (f64, f64) {
        ((self.a.0 + self.b.0) * 0.5, (self.a.1 + self.b.1) * 0.5)
    }
}

key_enum! {
/// Kind of a building (rendering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HouseKind {
    /// Cottage of cob or wattle and daub under thatch.
    Cottage => "cottage",
    /// Timber-framed house (colombage).
    Timbered => "timbered",
    /// Barn or byre.
    Barn => "barn",
    /// Small stone church.
    Church => "church",
    /// EP6: stone house (Midi, mountains).
    Stone => "stone",
    /// EP6: post mill.
    Windmill => "windmill",
    /// EP6: water mill (wheel on the front, towards the water).
    Watermill => "watermill",
    /// EP6: manor or tower house.
    Manor => "manor",
}
}

/// A building footprint: centre, size along its own axes, yaw (radians,
/// rotation of the ridge from +x towards +z).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct House {
    pub x: f64,
    pub z: f64,
    /// Length along the ridge.
    pub length: f64,
    pub width: f64,
    pub yaw: f64,
    pub kind: HouseKind,
}

/// What the campaign knows of the battle site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSite {
    pub terrain: Terrain,
    pub river: bool,
    pub coastal: bool,
    pub season: BattleSeason,
}

/// Tree density of the woods of a terrain, 0-1 (rendering).
pub fn woodland(terrain: Terrain) -> f64 {
    TerrainRules::of(terrain).woodland
}

/// The features of the site, drawn by [`SiteFeatures::draw`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SiteFeatures {
    pub ground: Ground,
    pub extra_mud: Vec<Zone>,
    pub pools: Vec<Zone>,
    pub coast: Option<Coast>,
    pub obstacles: Vec<Obstacle>,
}

/// What is already on the field (placement must avoid it).
pub struct Occupied<'a> {
    pub forests: &'a [Zone],
    pub mud: &'a [Zone],
    /// Lobes and copses of the woods and mud (lot R2): avoided, but no pools.
    pub parts: &'a [Zone],
    /// z of the river centre line at x, when there is a river.
    pub river_z: Option<&'a dyn Fn(f64) -> f64>,
    /// Size of the field (EP1).
    pub size: FieldSize,
    /// EP3: distance from (x, z) to the nearest water's edge (the river at
    /// its local width, streams, oxbow), when there is a river.
    pub water_gap: Option<&'a dyn Fn(f64, f64) -> f64>,
}

impl Occupied<'_> {
    fn near_river(&self, x: f64, z: f64, margin: f64) -> bool {
        // Margins were measured from the centre of an 18 m river (EP3: from
        // the water's edge, 9 m less, whatever the water).
        self.river_z.is_some_and(|f| (z - f(x)).abs() < margin)
            || self.water_gap.is_some_and(|g| g(x, z) < margin - 9.0)
    }
}

/// `true` when a disc at (x, z) of `radius` sits on the centre of a
/// deployment line (the armies must be able to form up).
pub(crate) fn on_line(size: &FieldSize, x: f64, z: f64, radius: f64) -> bool {
    [size.attacker_line_z(), size.defender_line_z()]
        .iter()
        .any(|line| {
            (z - line).abs() < radius + 40.0
                && (x - size.center_x()).abs() < size.line_half() + radius
        })
}

impl SiteFeatures {
    /// Draws the features of `site` from `rng` (a derived stream).
    pub fn draw(
        site: &FieldSite,
        weather: Weather,
        occupied: &Occupied,
        rng: &mut BattleRng,
    ) -> SiteFeatures {
        let ground = Ground::draw(site.season, weather, rng.unit());
        let mut features = SiteFeatures {
            ground,
            ..SiteFeatures::default()
        };
        // Soaked ground without rain: the pre-B5 rule already adds two mud
        // zones in rain and snow.
        let wet_weather = matches!(weather, Weather::Rain | Weather::Snow);
        if ground == Ground::Muddy && !wet_weather {
            for _ in 0..2 {
                if let Some(zone) = free_zone(rng, 30.0, 70.0, occupied, &[]) {
                    features.extra_mud.push(zone);
                }
            }
        }
        if site.terrain == Terrain::Marsh {
            features.pools = draw_pools(occupied, rng);
        }
        if site.coastal && rng.unit() < 0.7 {
            let flank = if rng.unit() < 0.5 {
                Flank::West
            } else {
                Flank::East
            };
            let beach = rng.range(35.0, 60.0);
            let offset = rng.range(15.0, 40.0);
            let shore_x = match flank {
                Flank::West => -offset,
                Flank::East => occupied.size.width + offset,
            };
            features.coast = Some(Coast {
                flank,
                shore_x,
                beach,
                field_width: occupied.size.width,
            });
        }
        if site.terrain == Terrain::Bocage {
            let mut hedgerows = bocage_hedgerows(&features, occupied, rng);
            features.obstacles.append(&mut hedgerows);
        }
        if site.terrain == Terrain::Marsh {
            let mut ditches = marsh_ditches(&features, occupied, rng);
            features.obstacles.append(&mut ditches);
        }
        features
    }
}

/// A zone that avoids the deployment lines, the other zones and `extra`.
fn free_zone(
    rng: &mut BattleRng,
    radius_low: f64,
    radius_high: f64,
    occupied: &Occupied,
    extra: &[Zone],
) -> Option<Zone> {
    for _ in 0..40 {
        let zone = Zone {
            x: rng.range(0.0, occupied.size.width),
            z: rng.range(60.0, occupied.size.depth - 60.0),
            radius: rng.range(radius_low, radius_high),
        };
        let clash = extra.iter().any(|other| {
            (zone.x - other.x).powi(2) + (zone.z - other.z).powi(2)
                < (zone.radius + other.radius + 10.0).powi(2)
        });
        if !on_line(&occupied.size, zone.x, zone.z, zone.radius)
            && !clash
            && !occupied.near_river(zone.x, zone.z, zone.radius + 20.0)
        {
            return Some(zone);
        }
    }
    None
}

/// Standing water of a marsh, inside or next to its mud.
fn draw_pools(occupied: &Occupied, rng: &mut BattleRng) -> Vec<Zone> {
    let mut pools = Vec::new();
    for mud in occupied.mud {
        if rng.unit() < 0.3 {
            continue;
        }
        let angle = rng.range(0.0, std::f64::consts::TAU);
        let dist = rng.range(0.0, mud.radius * 0.4);
        pools.push(Zone {
            x: mud.x + angle.cos() * dist,
            z: mud.z + angle.sin() * dist,
            radius: mud.radius * rng.range(0.3, 0.55),
        });
    }
    pools
}

fn inside_field(size: &FieldSize, p: (f64, f64)) -> bool {
    (5.0..=size.width - 5.0).contains(&p.0) && (5.0..=size.depth - 5.0).contains(&p.1)
}

/// Bocage: hedgerows on banks on a skewed grid of fields 90-170 m wide,
/// with gaps (gates), clear of the deployment lines, the river.
fn bocage_hedgerows(
    features: &SiteFeatures,
    occupied: &Occupied,
    rng: &mut BattleRng,
) -> Vec<Obstacle> {
    let mut hedges = Vec::new();
    let skew = rng.range(-0.25, 0.25);
    let (c, s) = (skew.cos(), skew.sin());
    let cell = rng.range(110.0, 150.0);
    let (width, depth) = (occupied.size.width, occupied.size.depth);
    let mut v = -200.0;
    while v < depth + 200.0 {
        let mut u = -200.0;
        while u < width + 200.0 {
            let pu = u + rng.range(-20.0, 20.0);
            let pv = v + rng.range(-20.0, 20.0);
            let to_world = |du: f64, dv: f64| {
                let (uu, vv) = (pu + du - width * 0.5, pv + dv - depth * 0.5);
                (width * 0.5 + uu * c - vv * s, depth * 0.5 + uu * s + vv * c)
            };
            // One field corner: a hedge along u and one along v.
            for horizontal in [true, false] {
                if rng.unit() < 0.2 {
                    continue;
                }
                let len = cell * rng.range(0.55, 0.9);
                let (a, b) = if horizontal {
                    (to_world(0.0, 0.0), to_world(len, rng.range(-8.0, 8.0)))
                } else {
                    (to_world(0.0, 0.0), to_world(rng.range(-8.0, 8.0), len))
                };
                let hedge = Obstacle {
                    a,
                    b,
                    kind: if rng.unit() < 0.15 {
                        ObstacleKind::Ditch
                    } else {
                        ObstacleKind::Hedge
                    },
                };
                if keep_line(&hedge, features, occupied) {
                    hedges.push(hedge);
                }
            }
            u += cell;
        }
        v += cell;
    }
    hedges
}

/// Marsh: drainage ditches crossing the field lengthwise, broken.
fn marsh_ditches(
    features: &SiteFeatures,
    occupied: &Occupied,
    rng: &mut BattleRng,
) -> Vec<Obstacle> {
    let mut ditches = Vec::new();
    let (width, depth) = (occupied.size.width, occupied.size.depth);
    // EP1: as many ditches per hectare on a larger field.
    let count = (8.0 * occupied.size.area_ratio()).round() as usize;
    for _ in 0..count {
        let x = rng.range(40.0, width - 40.0);
        let z = rng.range(40.0, depth - 40.0);
        let angle = rng.range(-0.3, 0.3)
            + if rng.unit() < 0.5 {
                0.0
            } else {
                std::f64::consts::FRAC_PI_2
            };
        let len = rng.range(80.0, 180.0);
        let ditch = Obstacle {
            a: (x, z),
            b: (x + angle.cos() * len, z + angle.sin() * len),
            kind: ObstacleKind::Ditch,
        };
        if keep_line(&ditch, features, occupied) {
            ditches.push(ditch);
        }
    }
    ditches
}

/// A line obstacle stays inside the field, off the deployment lines, the
/// river and the forests.
fn keep_line(line: &Obstacle, features: &SiteFeatures, occupied: &Occupied) -> bool {
    let size = &occupied.size;
    if !inside_field(size, line.a) || !inside_field(size, line.b) || line.length() < 25.0 {
        return false;
    }
    let samples = (line.length() / 10.0).ceil() as usize;
    (0..=samples).all(|i| {
        let t = i as f64 / samples as f64;
        let x = line.a.0 + (line.b.0 - line.a.0) * t;
        let z = line.a.1 + (line.b.1 - line.a.1) * t;
        !on_line(size, x, z, 0.0)
            && !occupied.near_river(x, z, 30.0)
            && !occupied.forests.iter().any(|f| f.contains(x, z))
            && !occupied.parts.iter().any(|f| f.contains(x, z))
            && !features.pools.iter().any(|p| p.contains(x, z))
            && !features
                .coast
                .is_some_and(|c| c.from_edge(x) < c.beach + 10.0)
    }) && {
        let (mx, mz) = line.midpoint();
        (0.0..=size.width).contains(&mx) && (0.0..=size.depth).contains(&mz)
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<Flank>();
        assert_keys_match_serde::<Ground>();
        assert_keys_match_serde::<ObstacleKind>();
        assert_keys_match_serde::<HouseKind>();
    }
}
