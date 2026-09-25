//! Battle site drawn from the campaign (lot B5): ground of the season (mud,
//! snow), coast on a flank, marsh pools, a village or farm with its hedges,
//! fences and ditches, bocage hedgerows.
//!
//! These features are drawn from a stream *derived* from the battle RNG
//! without consuming it ([`BattleRng::derive`]): battles drawn before B5 keep
//! their hills, forests, river and every later random draw.

use data_model::Terrain;
use serde::{Deserialize, Serialize};

use crate::field::{Weather, Zone};
use crate::rng::BattleRng;
use crate::scale::FieldSize;
use crate::setup::BattleSeason;

/// State of the ground at the time of the battle (season and weather).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ground {
    #[default]
    Dry,
    /// Soaked ground: extra mud zones.
    Muddy,
    /// Snow on the ground: slower marches even when no snow falls.
    Snowy,
}

impl Ground {
    pub fn key(self) -> &'static str {
        match self {
            Ground::Dry => "dry",
            Ground::Muddy => "muddy",
            Ground::Snowy => "snowy",
        }
    }

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

/// Side of the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flank {
    West,
    East,
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

/// Kind of a linear obstacle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObstacleKind {
    /// Hedgerow on a bank: cover against missiles, breaks charges, slows
    /// horses badly (the English archers' favourite position).
    Hedge,
    /// Wattle fence of a croft or a pen: slows horses.
    Fence,
    /// Drainage ditch: slows everyone, breaks charges.
    Ditch,
}

impl ObstacleKind {
    pub fn key(self) -> &'static str {
        match self {
            ObstacleKind::Hedge => "hedge",
            ObstacleKind::Fence => "fence",
            ObstacleKind::Ditch => "ditch",
        }
    }

    /// Speed multiplier while crossing.
    pub fn crossing_factor(self, mounted: bool) -> f64 {
        match (self, mounted) {
            (ObstacleKind::Hedge, false) => 0.6,
            (ObstacleKind::Hedge, true) => 0.3,
            (ObstacleKind::Fence, false) => 0.85,
            (ObstacleKind::Fence, true) => 0.6,
            (ObstacleKind::Ditch, false) => 0.6,
            (ObstacleKind::Ditch, true) => 0.4,
        }
    }

    /// Missiles crossing it lose part of their effect on a unit behind.
    pub fn gives_cover(self) -> bool {
        self == ObstacleKind::Hedge
    }

    /// A cavalry charge across it loses its impact.
    pub fn breaks_charge(self) -> bool {
        matches!(self, ObstacleKind::Hedge | ObstacleKind::Ditch)
    }
}

/// Distance within which a unit counts as crossing (or standing at) an
/// obstacle, in metres.
pub const OBSTACLE_REACH: f64 = 4.0;
/// A unit whose centre is within this distance behind a hedge is covered.
pub const HEDGE_COVER_REACH: f64 = 14.0;
/// Multiplier on missile casualties behind a hedge.
pub const HEDGE_COVER: f64 = 0.6;
/// Multiplier on missile casualties inside a village.
pub const VILLAGE_COVER: f64 = 0.6;

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
        let (ax, az) = self.a;
        let (dx, dz) = (self.b.0 - ax, self.b.1 - az);
        let len2 = dx * dx + dz * dz;
        let t = if len2 > 0.0 {
            (((x - ax) * dx + (z - az) * dz) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        ((x - ax - dx * t).powi(2) + (z - az - dz * t).powi(2)).sqrt()
    }

    /// `true` when the segment from `p` to `q` crosses the obstacle.
    pub fn crosses(&self, p: (f64, f64), q: (f64, f64)) -> bool {
        let orient = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
            (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
        };
        let d1 = orient(self.a, self.b, p);
        let d2 = orient(self.a, self.b, q);
        let d3 = orient(p, q, self.a);
        let d4 = orient(p, q, self.b);
        d1 * d2 < 0.0 && d3 * d4 < 0.0
    }

    pub fn length(&self) -> f64 {
        ((self.b.0 - self.a.0).powi(2) + (self.b.1 - self.a.1).powi(2)).sqrt()
    }

    fn midpoint(&self) -> (f64, f64) {
        ((self.a.0 + self.b.0) * 0.5, (self.a.1 + self.b.1) * 0.5)
    }
}

/// Kind of a village building (rendering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HouseKind {
    /// Cottage of cob or wattle and daub under thatch.
    Cottage,
    /// Timber-framed house (colombage).
    Timbered,
    /// Barn or byre.
    Barn,
    /// Small stone church.
    Church,
}

impl HouseKind {
    pub fn key(self) -> &'static str {
        match self {
            HouseKind::Cottage => "cottage",
            HouseKind::Timbered => "timbered",
            HouseKind::Barn => "barn",
            HouseKind::Church => "church",
        }
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

/// A hamlet or a farm: the zone is cover (houses, walls, gardens) and broken
/// ground; its crofts are enclosed by hedges and fences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Village {
    pub zone: Zone,
    /// A lone farm (a few buildings round a yard) rather than a hamlet.
    pub farm: bool,
    pub houses: Vec<House>,
}

impl Village {
    /// Speed multiplier through the lanes, yards and gardens.
    pub fn speed_factor(mounted: bool) -> f64 {
        if mounted {
            0.55
        } else {
            0.8
        }
    }
}

/// What the campaign knows of the battle site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSite {
    pub terrain: Terrain,
    pub river: bool,
    pub coastal: bool,
    pub season: BattleSeason,
    /// `Some(true)` forces a village, `Some(false)` forbids it, `None` draws
    /// it from the terrain.
    pub village: Option<bool>,
}

/// Tree density of the woods of a terrain, 0-1 (rendering).
pub fn woodland(terrain: Terrain) -> f64 {
    match terrain {
        Terrain::Forest => 1.0,
        Terrain::Bocage => 0.75,
        Terrain::Hills => 0.7,
        Terrain::Plains => 0.55,
        Terrain::Mountains => 0.5,
        Terrain::Heath => 0.35,
        Terrain::Marsh => 0.3,
    }
}

/// Chance of a village or a farm on the field.
fn village_chance(terrain: Terrain) -> f64 {
    match terrain {
        Terrain::Plains => 0.7,
        Terrain::Bocage => 0.75,
        Terrain::Hills => 0.5,
        Terrain::Heath => 0.4,
        Terrain::Forest => 0.35,
        Terrain::Marsh => 0.35,
        Terrain::Mountains => 0.3,
    }
}

/// The features of the site, drawn by [`SiteFeatures::draw`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SiteFeatures {
    pub ground: Ground,
    pub extra_mud: Vec<Zone>,
    pub pools: Vec<Zone>,
    pub coast: Option<Coast>,
    pub obstacles: Vec<Obstacle>,
    pub village: Option<Village>,
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
}

impl Occupied<'_> {
    fn near_river(&self, x: f64, z: f64, margin: f64) -> bool {
        self.river_z.is_some_and(|f| (z - f(x)).abs() < margin)
    }

    fn in_zones(&self, x: f64, z: f64, margin: f64) -> bool {
        self.forests
            .iter()
            .chain(self.mud)
            .chain(self.parts)
            .any(|zone| {
                (x - zone.x).powi(2) + (z - zone.z).powi(2) < (zone.radius + margin).powi(2)
            })
    }
}

/// `true` when a disc at (x, z) of `radius` sits on the centre of a
/// deployment line (the armies must be able to form up).
fn on_line(size: &FieldSize, x: f64, z: f64, radius: f64) -> bool {
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
        let wants_village = site
            .village
            .unwrap_or_else(|| rng.unit() < village_chance(site.terrain));
        if wants_village {
            let farm = !matches!(site.village, Some(true)) && rng.unit() < 0.4;
            features.village = draw_village(farm, site.terrain, &features, occupied, rng);
        }
        if let Some(village) = &features.village {
            features.obstacles = crofts(village, &occupied.size, rng);
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

/// A hamlet (6-11 buildings round a green, with a church) or a farm (3-4
/// buildings round a yard), on a flank of the field or behind a line.
fn draw_village(
    farm: bool,
    terrain: Terrain,
    features: &SiteFeatures,
    occupied: &Occupied,
    rng: &mut BattleRng,
) -> Option<Village> {
    let radius = if farm {
        rng.range(35.0, 45.0)
    } else {
        rng.range(60.0, 85.0)
    };
    let (width, depth) = (occupied.size.width, occupied.size.depth);
    for attempt in 0..60 {
        // Flanks first (the classic village on the wing), then anywhere
        // off the lines.
        let (x, z) = if attempt < 30 {
            let west = rng.unit() < 0.5;
            let x = if west {
                rng.range(radius + 30.0, 300.0)
            } else {
                rng.range(width - 300.0, width - radius - 30.0)
            };
            (x, rng.range(radius + 60.0, depth - radius - 60.0))
        } else {
            (
                rng.range(radius + 30.0, width - radius - 30.0),
                rng.range(radius + 40.0, depth - radius - 40.0),
            )
        };
        if on_line(&occupied.size, x, z, radius)
            || occupied.in_zones(x, z, radius + 15.0)
            || occupied.near_river(x, z, radius + 25.0)
            || features
                .pools
                .iter()
                .any(|p| (x - p.x).powi(2) + (z - p.z).powi(2) < (radius + p.radius + 15.0).powi(2))
            || features
                .extra_mud
                .iter()
                .any(|m| (x - m.x).powi(2) + (z - m.z).powi(2) < (radius + m.radius + 10.0).powi(2))
            || features
                .coast
                .is_some_and(|c| c.from_edge(x) < c.beach + radius + 15.0)
        {
            continue;
        }
        let zone = Zone { x, z, radius };
        let houses = if farm {
            farm_buildings(zone, terrain, rng)
        } else {
            hamlet_buildings(zone, terrain, rng)
        };
        return Some(Village { zone, farm, houses });
    }
    None
}

fn house_kind(terrain: Terrain, rng: &mut BattleRng) -> HouseKind {
    // Timber framing in the bocage and the plains of the north, cob and
    // thatch elsewhere.
    let timbered = match terrain {
        Terrain::Bocage | Terrain::Plains | Terrain::Forest => 0.5,
        _ => 0.25,
    };
    if rng.unit() < timbered {
        HouseKind::Timbered
    } else {
        HouseKind::Cottage
    }
}

fn hamlet_buildings(zone: Zone, terrain: Terrain, rng: &mut BattleRng) -> Vec<House> {
    let mut houses: Vec<House> = Vec::new();
    // The church faces east (+x) on the green.
    let church_angle = rng.range(0.0, std::f64::consts::TAU);
    houses.push(House {
        x: zone.x + church_angle.cos() * zone.radius * 0.25,
        z: zone.z + church_angle.sin() * zone.radius * 0.25,
        length: rng.range(16.0, 22.0),
        width: rng.range(7.0, 9.0),
        yaw: rng.range(-0.15, 0.15),
        kind: HouseKind::Church,
    });
    let count = 6 + rng.below(6) as usize;
    let mut tries = 0;
    while houses.len() < count + 1 && tries < 200 {
        tries += 1;
        // Houses line the lanes: a ring round the green, facing its centre.
        let angle = rng.range(0.0, std::f64::consts::TAU);
        let dist = zone.radius * rng.range(0.45, 0.85);
        let (x, z) = (zone.x + angle.cos() * dist, zone.z + angle.sin() * dist);
        let barn = rng.unit() < 0.2;
        let (length, width) = if barn {
            (rng.range(12.0, 18.0), rng.range(6.5, 8.5))
        } else {
            (rng.range(8.0, 13.0), rng.range(5.0, 6.5))
        };
        let candidate = House {
            x,
            z,
            length,
            width,
            yaw: angle + std::f64::consts::FRAC_PI_2 + rng.range(-0.25, 0.25),
            kind: if barn {
                HouseKind::Barn
            } else {
                house_kind(terrain, rng)
            },
        };
        if houses.iter().all(|h| apart(h, &candidate, 3.0)) {
            houses.push(candidate);
        }
    }
    houses
}

fn farm_buildings(zone: Zone, terrain: Terrain, rng: &mut BattleRng) -> Vec<House> {
    // A farmhouse, a barn and a byre round a yard (a U or an L).
    let yaw = rng.range(0.0, std::f64::consts::TAU);
    let (c, s) = (yaw.cos(), yaw.sin());
    let at = |u: f64, v: f64| (zone.x + u * c - v * s, zone.z + u * s + v * c);
    let mut houses = Vec::new();
    let (x, z) = at(0.0, -11.0);
    houses.push(House {
        x,
        z,
        length: rng.range(12.0, 15.0),
        width: 6.5,
        yaw,
        kind: house_kind(terrain, rng),
    });
    let (x, z) = at(-12.0, 3.0);
    houses.push(House {
        x,
        z,
        length: rng.range(14.0, 18.0),
        width: 8.0,
        yaw: yaw + std::f64::consts::FRAC_PI_2,
        kind: HouseKind::Barn,
    });
    if rng.unit() < 0.7 {
        let (x, z) = at(12.0, 3.0);
        houses.push(House {
            x,
            z,
            length: rng.range(9.0, 12.0),
            width: 5.5,
            yaw: yaw + std::f64::consts::FRAC_PI_2,
            kind: HouseKind::Barn,
        });
    }
    if rng.unit() < 0.5 {
        let (x, z) = at(rng.range(-6.0, 6.0), 16.0);
        houses.push(House {
            x,
            z,
            length: 7.0,
            width: 4.5,
            yaw,
            kind: HouseKind::Cottage,
        });
    }
    houses
}

/// Conservative separation test of two footprints (bounding circles).
fn apart(a: &House, b: &House, gap: f64) -> bool {
    let ra = 0.5 * (a.length.powi(2) + a.width.powi(2)).sqrt();
    let rb = 0.5 * (b.length.powi(2) + b.width.powi(2)).sqrt();
    (a.x - b.x).powi(2) + (a.z - b.z).powi(2) > (ra + rb + gap).powi(2)
}

/// Crofts behind the houses and the pound: hedged and fenced plots around
/// the village, with gaps for the lanes.
fn crofts(village: &Village, size: &FieldSize, rng: &mut BattleRng) -> Vec<Obstacle> {
    let zone = village.zone;
    let mut obstacles = Vec::new();
    let sides = if village.farm { 7 } else { 12 };
    let outer = zone.radius + rng.range(10.0, 25.0);
    let start = rng.range(0.0, std::f64::consts::TAU);
    let step = std::f64::consts::TAU / f64::from(sides);
    for k in 0..sides {
        // Gaps for the lanes and the fields beyond.
        if rng.unit() < 0.25 {
            continue;
        }
        let a0 = start + step * f64::from(k);
        let a1 = a0 + step * rng.range(0.7, 0.95);
        let r0 = outer * rng.range(0.92, 1.08);
        let r1 = outer * rng.range(0.92, 1.08);
        let kind = if rng.unit() < 0.7 {
            ObstacleKind::Hedge
        } else {
            ObstacleKind::Fence
        };
        obstacles.push(Obstacle {
            a: (zone.x + a0.cos() * r0, zone.z + a0.sin() * r0),
            b: (zone.x + a1.cos() * r1, zone.z + a1.sin() * r1),
            kind,
        });
    }
    // Radial croft boundaries between the ring and the houses.
    let radial = if village.farm { 2 } else { 5 };
    for _ in 0..radial {
        let angle = rng.range(0.0, std::f64::consts::TAU);
        let r0 = zone.radius * rng.range(0.85, 1.0);
        let kind = if rng.unit() < 0.5 {
            ObstacleKind::Fence
        } else {
            ObstacleKind::Hedge
        };
        obstacles.push(Obstacle {
            a: (zone.x + angle.cos() * r0, zone.z + angle.sin() * r0),
            b: (zone.x + angle.cos() * outer, zone.z + angle.sin() * outer),
            kind,
        });
    }
    obstacles
        .into_iter()
        .filter(|o| inside_field(size, o.a) && inside_field(size, o.b))
        .collect()
}

fn inside_field(size: &FieldSize, p: (f64, f64)) -> bool {
    (5.0..=size.width - 5.0).contains(&p.0) && (5.0..=size.depth - 5.0).contains(&p.1)
}

/// Bocage: hedgerows on banks on a skewed grid of fields 90-170 m wide,
/// with gaps (gates), clear of the deployment lines, the river and the
/// village.
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
/// river, the forests and the village.
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
                .village
                .as_ref()
                .is_some_and(|v| v.zone.contains(x, z) || near(v.zone, x, z, 35.0))
            && !features
                .coast
                .is_some_and(|c| c.from_edge(x) < c.beach + 10.0)
    }) && {
        let (mx, mz) = line.midpoint();
        (0.0..=size.width).contains(&mx) && (0.0..=size.depth).contains(&mz)
    }
}

fn near(zone: Zone, x: f64, z: f64, margin: f64) -> bool {
    (x - zone.x).powi(2) + (z - zone.z).powi(2) < (zone.radius + margin).powi(2)
}
