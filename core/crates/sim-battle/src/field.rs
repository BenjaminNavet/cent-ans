//! Battlefield (height, forests, mud, river) and weather, generated from the
//! seed, the province terrain and the season (spec § 1), plus the features of
//! the campaign site (coast, marsh pools, village, hedges, ground of the
//! season: lot B5, [`crate::site`]).

use data_model::Terrain;
use serde::{Deserialize, Serialize};

use crate::relief;
use crate::rng::BattleRng;
use crate::scale::FieldSize;
pub use crate::scale::GRID_RESOLUTION;
use crate::setup::BattleSeason;
use crate::site::{
    self, Coast, FieldSite, Ground, Obstacle, Occupied, SiteFeatures, Village, HEDGE_COVER_REACH,
    OBSTACLE_REACH,
};

/// Width of the standard field (skirmishes, sieges) along x, in metres.
/// EP1: fields are sized by [`crate::scale`]; game code reads
/// [`Battlefield::width`] (this constant is for tests and siege layouts).
pub const FIELD_WIDTH: f64 = FieldSize::STANDARD.width;
/// Depth of the standard field along z, in metres (see [`FIELD_WIDTH`]).
pub const FIELD_DEPTH: f64 = FieldSize::STANDARD.depth;

/// A circular zone (forest or mud).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub x: f64,
    pub z: f64,
    pub radius: f64,
}

impl Zone {
    pub fn contains(&self, x: f64, z: f64) -> bool {
        let (dx, dz) = (x - self.x, z - self.z);
        dx * dx + dz * dz <= self.radius * self.radius
    }
}

/// A ford across the river.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ford {
    pub x: f64,
    pub half_width: f64,
}

/// A river crossing the field from west to east between the two armies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct River {
    pub z0: f64,
    pub amplitude: f64,
    pub wavelength: f64,
    pub phase: f64,
    pub width: f64,
    pub fords: Vec<Ford>,
}

impl River {
    /// z of the river centre line at `x`.
    pub fn center_z(&self, x: f64) -> f64 {
        self.z0 + self.amplitude * (x / self.wavelength * std::f64::consts::TAU + self.phase).sin()
    }

    pub fn in_water(&self, x: f64, z: f64) -> bool {
        (z - self.center_z(x)).abs() <= self.width * 0.5
    }

    pub fn in_ford(&self, x: f64) -> bool {
        self.fords.iter().any(|f| (x - f.x).abs() <= f.half_width)
    }

    /// Centre line sampled every `step` metres from x = 0 to `width` (for
    /// rendering).
    pub fn polyline(&self, step: f64, width: f64) -> Vec<(f64, f64)> {
        let count = (width / step).ceil() as usize;
        (0..=count)
            .map(|i| {
                let x = (i as f64 * step).min(width);
                (x, self.center_z(x))
            })
            .collect()
    }
}

/// Weather of the battle (spec § 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weather {
    Clear,
    /// Longbows and crossbows shoot 40 % worse; mud is heavier.
    Rain,
    /// Ranges are 30 % shorter.
    Fog,
    /// Slower and more tiring marches.
    Snow,
}

impl Weather {
    pub fn key(self) -> &'static str {
        match self {
            Weather::Clear => "clear",
            Weather::Rain => "rain",
            Weather::Fog => "fog",
            Weather::Snow => "snow",
        }
    }

    pub fn label_fr(self) -> &'static str {
        match self {
            Weather::Clear => "Temps clair",
            Weather::Rain => "Pluie",
            Weather::Fog => "Brouillard",
            Weather::Snow => "Neige",
        }
    }

    /// Draw by season (percent chances clear / rain / fog / snow).
    pub fn draw(season: BattleSeason, rng: &mut BattleRng) -> Weather {
        let table: [u32; 4] = match season {
            BattleSeason::Spring => [60, 30, 10, 0],
            BattleSeason::Summer => [80, 15, 5, 0],
            BattleSeason::Autumn => [45, 35, 20, 0],
            BattleSeason::Winter => [35, 20, 15, 30],
        };
        let mut roll = rng.below(100);
        for (weather, chance) in [Weather::Clear, Weather::Rain, Weather::Fog, Weather::Snow]
            .into_iter()
            .zip(table)
        {
            if roll < chance {
                return weather;
            }
            roll -= chance;
        }
        Weather::Clear
    }

    /// Multiplier on the range of every shooter.
    pub fn range_factor(self) -> f64 {
        if self == Weather::Fog {
            0.7
        } else {
            1.0
        }
    }

    /// Multiplier on the accuracy of bows and crossbows (`rain_penalty`).
    pub fn bow_factor(self) -> f64 {
        match self {
            Weather::Rain => 0.6,
            Weather::Snow => 0.8,
            _ => 1.0,
        }
    }
}

/// The generated battlefield.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Battlefield {
    pub width: f64,
    pub depth: f64,
    /// Size and battle lines of the field (EP1); `width` and `depth` repeat
    /// its dimensions.
    #[serde(default)]
    pub size: FieldSize,
    pub resolution: f64,
    /// Grid points along x.
    pub nx: usize,
    /// Grid points along z.
    pub nz: usize,
    /// Row-major heights (z rows of `nx` values), in metres.
    pub heights: Vec<f64>,
    /// Anchor disc of each wood (R2: shrunk and moved along the relief).
    pub forests: Vec<Zone>,
    /// Anchor disc of each patch of mud.
    pub mud: Vec<Zone>,
    /// Lobes and copses of the woods (R2): a wood is its anchor plus its parts.
    #[serde(default)]
    pub forest_parts: Vec<Zone>,
    /// Lobes and puddles of the mud (R2).
    #[serde(default)]
    pub mud_parts: Vec<Zone>,
    pub river: Option<River>,
    /// Province terrain the field was drawn from (B5).
    #[serde(default = "default_terrain")]
    pub terrain: Terrain,
    #[serde(default)]
    pub season: BattleSeason,
    #[serde(default)]
    pub ground: Ground,
    /// Tree density of the woods, 0-1 (rendering; B5).
    #[serde(default = "default_woodland")]
    pub woodland: f64,
    /// Standing water of a marsh: shallow water, like a ford (B5).
    #[serde(default)]
    pub pools: Vec<Zone>,
    #[serde(default)]
    pub coast: Option<Coast>,
    /// Hedges, fences and ditches (B5).
    #[serde(default)]
    pub obstacles: Vec<Obstacle>,
    #[serde(default)]
    pub village: Option<Village>,
}

fn default_terrain() -> Terrain {
    Terrain::Plains
}

fn default_woodland() -> f64 {
    0.5
}

/// Salt of the derived stream of the site features (B5).
const SITE_STREAM: u64 = 0xB5;

struct Hill {
    x: f64,
    z: f64,
    radius: f64,
    height: f64,
}

/// z of the attacker's and defender's battle lines at deployment on the
/// standard field (EP1: game code reads [`Battlefield::attacker_line_z`]).
pub const ATTACKER_LINE_Z: f64 = 250.0;
pub const DEFENDER_LINE_Z: f64 = 550.0;

impl Battlefield {
    /// Builds a standard field for `terrain`, adding a river when `river` is
    /// set and extra mud in the rain or snow (no coast, no village: pre-B5
    /// field).
    pub fn generate(terrain: Terrain, river: bool, weather: Weather, rng: &mut BattleRng) -> Self {
        Self::generate_base(FieldSize::STANDARD, terrain, river, weather, rng)
    }

    /// [`Self::generate`] on a field of `size` (EP1).
    pub fn generate_sized(
        size: FieldSize,
        terrain: Terrain,
        river: bool,
        weather: Weather,
        rng: &mut BattleRng,
    ) -> Self {
        Self::generate_base(size, terrain, river, weather, rng)
    }

    /// Builds the standard field of a campaign site (B5), see
    /// [`Self::generate_site_sized`].
    pub fn generate_site(site: &FieldSite, weather: Weather, rng: &mut BattleRng) -> Self {
        Self::generate_site_sized(site, FieldSize::STANDARD, weather, rng)
    }

    /// Builds the field of a campaign site (B5) on a field of `size` (EP1):
    /// the pre-B5 field (same draws from `rng`), then ground, coast, pools,
    /// village and hedges from a derived stream that leaves `rng` where the
    /// pre-B5 field left it.
    pub fn generate_site_sized(
        site: &FieldSite,
        size: FieldSize,
        weather: Weather,
        rng: &mut BattleRng,
    ) -> Self {
        let mut field = Self::generate_base(size, site.terrain, site.river, weather, rng);
        field.season = site.season;
        let mut stream = rng.derive(SITE_STREAM);
        let parts: Vec<Zone> = field
            .forest_parts
            .iter()
            .chain(&field.mud_parts)
            .copied()
            .collect();
        let river = field.river.clone();
        let river_z = move |x: f64| river.as_ref().map_or(f64::NAN, |r| r.center_z(x));
        let river_fn: &dyn Fn(f64) -> f64 = &river_z;
        let occupied = Occupied {
            forests: &field.forests,
            mud: &field.mud,
            parts: &parts,
            river_z: field.river.is_some().then_some(river_fn),
            size: field.size,
        };
        let features = SiteFeatures::draw(site, weather, &occupied, &mut stream);
        field.ground = features.ground;
        field.mud.extend(features.extra_mud);
        field.pools = features.pools;
        field.obstacles = features.obstacles;
        field.village = features.village;
        field.coast = features.coast;
        if let Some(coast) = field.coast {
            field.shape_coast(coast);
        }
        field
    }

    /// Lowers the coastal flank towards the beach (dunes on the sand).
    fn shape_coast(&mut self, coast: Coast) {
        for iz in 0..self.nz {
            for ix in 0..self.nx {
                let x = ix as f64 * self.resolution;
                let z = iz as f64 * self.resolution;
                let d = coast.from_edge(x);
                let band = coast.beach + 120.0;
                if d >= band {
                    continue;
                }
                let t = (d / band).clamp(0.0, 1.0);
                let blend = t * t * (3.0 - 2.0 * t);
                let dune = if d < coast.beach {
                    let s = d / coast.beach;
                    1.2 * (s * std::f64::consts::PI).sin()
                        * (0.6 + 0.4 * (z * 0.031 + x * 0.017).sin())
                } else {
                    0.0
                };
                let h = &mut self.heights[iz * self.nx + ix];
                *h = *h * blend + (1.0 + dune) * (1.0 - blend);
            }
        }
    }

    /// z of the attacker's battle line at deployment.
    pub fn attacker_line_z(&self) -> f64 {
        self.size.attacker_line_z()
    }

    /// z of the defender's battle line at deployment.
    pub fn defender_line_z(&self) -> f64 {
        self.size.defender_line_z()
    }

    fn generate_base(
        size: FieldSize,
        terrain: Terrain,
        river: bool,
        weather: Weather,
        rng: &mut BattleRng,
    ) -> Self {
        let (width, depth) = (size.width, size.depth);
        let (attacker_line, defender_line) = (size.attacker_line_z(), size.defender_line_z());
        // R2: the relief detail and the shapes of woods and mud come from a
        // derived stream; the draws below are the pre-R2 ones, unchanged.
        let mut relief_stream = rng.derive(relief::RELIEF_STREAM);
        let (hill_count, hill_height, forest_count, mud_count) = match terrain {
            Terrain::Plains => (4, 5.0, 2, 1),
            Terrain::Heath => (5, 7.0, 1, 1),
            Terrain::Bocage => (5, 7.0, 7, 1),
            Terrain::Forest => (5, 9.0, 9, 1),
            Terrain::Hills => (7, 22.0, 3, 0),
            Terrain::Mountains => (8, 45.0, 3, 0),
            Terrain::Marsh => (3, 2.0, 1, 8),
        };
        // EP1: as many hills, woods and mud per hectare on a larger field.
        let per_area = |count: usize| (count as f64 * size.area_ratio()).round() as usize;
        let (hill_count, forest_count, mud_count) = (
            per_area(hill_count),
            per_area(forest_count),
            per_area(mud_count),
        );
        let mut hills: Vec<Hill> = (0..hill_count)
            .map(|_| Hill {
                x: rng.range(0.0, width),
                z: rng.range(0.0, depth),
                radius: rng.range(80.0, 260.0),
                height: hill_height * rng.range(0.4, 1.0),
            })
            .collect();
        // Hills and mountains: the defender holds the high ground.
        if matches!(terrain, Terrain::Hills | Terrain::Mountains) {
            hills.push(Hill {
                x: size.center_x(),
                z: defender_line + 60.0,
                radius: 320.0,
                height: hill_height * 0.8,
            });
        }
        let (nx, nz) = (size.nx(), size.nz());
        let tilt_x = rng.range(-0.004, 0.004);
        let tilt_z = rng.range(-0.004, 0.004);
        let river_def = river.then(|| {
            let fords = (0..2)
                .map(|i| Ford {
                    x: width * (0.3 + 0.4 * i as f64) + rng.range(-80.0, 80.0),
                    half_width: 35.0,
                })
                .collect();
            River {
                z0: (attacker_line + defender_line) * 0.5 + rng.range(-30.0, 30.0),
                amplitude: rng.range(10.0, 35.0),
                wavelength: rng.range(500.0, 900.0),
                phase: rng.range(0.0, std::f64::consts::TAU),
                width: 18.0,
                fords,
            }
        });
        let mut heights = Vec::with_capacity(nx * nz);
        for iz in 0..nz {
            for ix in 0..nx {
                let x = ix as f64 * GRID_RESOLUTION;
                let z = iz as f64 * GRID_RESOLUTION;
                let mut h = 3.0 + tilt_x * (x - width * 0.5) + tilt_z * (z - depth * 0.5);
                for hill in &hills {
                    let d2 = (x - hill.x).powi(2) + (z - hill.z).powi(2);
                    h += hill.height * (-d2 / (hill.radius * hill.radius)).exp();
                }
                if let Some(r) = &river_def {
                    let d = (z - r.center_z(x)).abs();
                    if d < r.width * 1.5 {
                        let depth = if r.in_ford(x) { 0.6 } else { 1.6 };
                        h -= depth * (1.0 - d / (r.width * 1.5));
                    }
                }
                heights.push(h);
            }
        }
        relief::shape_relief(
            &mut heights,
            &size,
            terrain,
            river_def.as_ref(),
            &mut relief_stream,
        );
        // Forests and mud stay off the centre of the deployment lines.
        let zone = |radius_low: f64, radius_high: f64, rng: &mut BattleRng| loop {
            let candidate = Zone {
                x: rng.range(0.0, width),
                z: rng.range(60.0, depth - 60.0),
                radius: rng.range(radius_low, radius_high),
            };
            let near_line = |line: f64| {
                (candidate.z - line).abs() < candidate.radius + 40.0
                    && (candidate.x - size.center_x()).abs() < size.line_half() + candidate.radius
            };
            if !near_line(attacker_line) && !near_line(defender_line) {
                break candidate;
            }
        };
        let mut forests: Vec<Zone> = (0..forest_count).map(|_| zone(35.0, 90.0, rng)).collect();
        let wet = matches!(weather, Weather::Rain | Weather::Snow);
        let mut mud: Vec<Zone> = (0..mud_count + if wet { 2 } else { 0 })
            .map(|_| zone(30.0, 70.0, rng))
            .collect();
        let (forest_parts, mud_parts) = relief::shape_cover(
            &mut forests,
            &mut mud,
            &heights,
            &size,
            river_def.as_ref(),
            &mut relief_stream,
        );
        Battlefield {
            width,
            depth,
            size,
            resolution: GRID_RESOLUTION,
            nx,
            nz,
            heights,
            forests,
            mud,
            forest_parts,
            mud_parts,
            river: river_def,
            terrain,
            season: BattleSeason::Spring,
            ground: Ground::Dry,
            woodland: site::woodland(terrain),
            pools: Vec::new(),
            coast: None,
            obstacles: Vec::new(),
            village: None,
        }
    }

    /// Siege battles: flattens the ground under and around the town and
    /// clears forests and mud from the town and the attacker's approach.
    pub fn prepare_for_siege(&mut self) {
        self.prepare_for_siege_around(crate::siege::RING_RADIUS);
    }

    /// [`Self::prepare_for_siege`] around a ring of `radius` metres (L3: the
    /// town of a landmark plan may be larger than the generic one).
    pub fn prepare_for_siege_around(&mut self, radius: f64) {
        let (cx, cz) = crate::siege::TOWN_CENTER;
        let center_height = self.height(cx, cz);
        for iz in 0..self.nz {
            for ix in 0..self.nx {
                let x = ix as f64 * self.resolution;
                let z = iz as f64 * self.resolution;
                let d = ((x - cx).powi(2) + (z - cz).powi(2)).sqrt();
                let weight = if d < radius + 90.0 {
                    1.0
                } else if d < radius + 200.0 {
                    1.0 - (d - radius - 90.0) / 110.0
                } else {
                    0.0
                };
                let h = &mut self.heights[iz * self.nx + ix];
                *h += (center_height - *h) * weight * 0.9;
            }
        }
        let keep = |zone: &Zone| {
            let d = ((zone.x - cx).powi(2) + (zone.z - cz).powi(2)).sqrt();
            let approach = zone.z - zone.radius < cz - radius + 20.0
                && zone.z + zone.radius > 120.0
                && (zone.x - cx).abs() < 450.0 + zone.radius;
            d > radius + 60.0 + zone.radius && !approach
        };
        self.forests.retain(keep);
        self.mud.retain(keep);
        self.forest_parts.retain(keep);
        self.mud_parts.retain(keep);
        self.pools.retain(keep);
        if self.village.as_ref().is_some_and(|v| !keep(&v.zone)) {
            self.village = None;
        }
        self.obstacles.retain(|o| {
            [o.a, o.b]
                .iter()
                .all(|&(x, z)| keep(&Zone { x, z, radius: 0.0 }))
        });
        self.river = None;
    }

    /// Speed multiplier of the site features at (x, z) (B5): hedges, fences
    /// and ditches being crossed, village lanes, beach sand, snow on the
    /// ground. Marsh pools count as shallow water ([`Self::water_at`]).
    pub fn site_speed_factor(&self, x: f64, z: f64, mounted: bool, weather: Weather) -> f64 {
        let mut factor = self.ground.speed_factor(weather);
        if let Some(obstacle) = self
            .obstacles
            .iter()
            .filter(|o| o.distance(x, z) <= OBSTACLE_REACH)
            .min_by(|a, b| {
                a.kind
                    .crossing_factor(mounted)
                    .total_cmp(&b.kind.crossing_factor(mounted))
            })
        {
            factor *= obstacle.kind.crossing_factor(mounted);
        }
        if self.in_village(x, z) {
            factor *= Village::speed_factor(mounted);
        }
        if self.coast.is_some_and(|c| c.on_beach(x)) {
            factor *= Coast::SAND_FACTOR;
        }
        factor
    }

    /// Parts of the one-line description of the site (B6), in French:
    /// ground (« Terre gelée » for dry winter ground), season, village or
    /// farm, hedges, ditches, fences, pools, river, coast.
    pub fn site_parts_fr(&self) -> Vec<String> {
        use crate::site::{Flank, ObstacleKind};
        let ground = if self.ground == Ground::Dry && self.season == BattleSeason::Winter {
            "Terre gelée"
        } else {
            self.ground.label_fr()
        };
        let mut parts = vec![ground.to_owned(), self.season.label_fr().to_owned()];
        if let Some(village) = &self.village {
            parts.push(if village.farm { "ferme" } else { "village" }.to_owned());
        }
        for (kind, label) in [
            (ObstacleKind::Hedge, "haies"),
            (ObstacleKind::Ditch, "fossés"),
            (ObstacleKind::Fence, "clôtures"),
        ] {
            if self.obstacles.iter().any(|o| o.kind == kind) {
                parts.push(label.to_owned());
            }
        }
        if !self.pools.is_empty() {
            parts.push("mares".to_owned());
        }
        if self.river.is_some() {
            parts.push("rivière et gués".to_owned());
        }
        if let Some(coast) = &self.coast {
            parts.push(
                match coast.flank {
                    Flank::West => "côte ouest",
                    Flank::East => "côte est",
                }
                .to_owned(),
            );
        }
        parts
    }

    /// The site in one compact line (B6): « Terre gelée · hiver · village ·
    /// haies · côte ouest ».
    pub fn site_label_fr(&self) -> String {
        self.site_parts_fr().join(" · ")
    }

    pub fn in_village(&self, x: f64, z: f64) -> bool {
        self.village.as_ref().is_some_and(|v| v.zone.contains(x, z))
    }

    /// `true` when a hedge stands between `from` and a target at `to`, close
    /// in front of the target (B5: archers behind a hedge).
    pub fn hedge_between(&self, from: (f64, f64), to: (f64, f64)) -> bool {
        self.obstacles.iter().any(|o| {
            o.kind.gives_cover()
                && o.distance(to.0, to.1) <= HEDGE_COVER_REACH
                && o.crosses(from, to)
        })
    }

    /// `true` when a charge from `from` against a unit at `to` crosses a
    /// hedge or a ditch close in front of the target, or meets the target while
    /// the horsemen are still in the hedge (B5).
    pub fn breaks_charge(&self, from: (f64, f64), to: (f64, f64)) -> bool {
        self.obstacles.iter().any(|o| {
            o.kind.breaks_charge()
                && o.distance(to.0, to.1) <= HEDGE_COVER_REACH
                && (o.crosses(from, to) || o.distance(from.0, from.1) <= 2.0 * OBSTACLE_REACH)
        })
    }

    /// Bilinear height at (x, z), clamped to the field.
    pub fn height(&self, x: f64, z: f64) -> f64 {
        let fx = (x / self.resolution).clamp(0.0, (self.nx - 1) as f64);
        let fz = (z / self.resolution).clamp(0.0, (self.nz - 1) as f64);
        let (ix, iz) = (fx.floor() as usize, fz.floor() as usize);
        let (ix1, iz1) = ((ix + 1).min(self.nx - 1), (iz + 1).min(self.nz - 1));
        let (tx, tz) = (fx - ix as f64, fz - iz as f64);
        let at = |i: usize, j: usize| self.heights[j * self.nx + i];
        let top = at(ix, iz) * (1.0 - tx) + at(ix1, iz) * tx;
        let bottom = at(ix, iz1) * (1.0 - tx) + at(ix1, iz1) * tx;
        top * (1.0 - tz) + bottom * tz
    }

    pub fn in_forest(&self, x: f64, z: f64) -> bool {
        self.forests
            .iter()
            .chain(&self.forest_parts)
            .any(|f| f.contains(x, z))
    }

    pub fn in_mud(&self, x: f64, z: f64) -> bool {
        self.mud
            .iter()
            .chain(&self.mud_parts)
            .any(|m| m.contains(x, z))
    }

    /// `Some(true)` in a ford, `Some(false)` in deep water, `None` on land.
    /// Marsh pools count as shallow water (B5).
    pub fn water_at(&self, x: f64, z: f64) -> Option<bool> {
        if let Some(river) = &self.river {
            if river.in_water(x, z) {
                return Some(river.in_ford(x));
            }
        }
        self.pools.iter().any(|p| p.contains(x, z)).then_some(true)
    }

    pub fn inside(&self, x: f64, z: f64) -> bool {
        (0.0..=self.width).contains(&x) && (0.0..=self.depth).contains(&z)
    }

    /// `true` when the ground between the two points rises above the line of
    /// sight (eyes 2 m above the ground).
    pub fn blocks_sight(&self, from: (f64, f64), to: (f64, f64)) -> bool {
        let h0 = self.height(from.0, from.1) + 2.0;
        let h1 = self.height(to.0, to.1) + 2.0;
        (1..8).any(|i| {
            let t = f64::from(i) / 8.0;
            let x = from.0 + (to.0 - from.0) * t;
            let z = from.1 + (to.1 - from.1) * t;
            self.height(x, z) > h0 + (h1 - h0) * t + 1.0
        })
    }
}
