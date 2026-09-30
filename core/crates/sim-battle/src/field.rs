//! Battlefield (height, forests, mud, river) and weather, generated from the
//! seed, the province terrain and the season (spec § 1), plus the features of
//! the campaign site (coast, marsh pools, village, hedges, ground of the
//! season: lot B5, [`crate::site`]).

use data_model::Terrain;
use serde::{Deserialize, Serialize};

use crate::hydro;
use crate::relief;
use crate::rng::BattleRng;
use crate::scale::FieldSize;
pub use crate::scale::GRID_RESOLUTION;
use crate::setup::{BattleSeason, CrossingStructure};
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
    /// Mean width of the water (EP3: drawn by terrain; see [`River::width_at`]).
    pub width: f64,
    pub fords: Vec<Ford>,
    /// EP3: relative variation of the width along the course ...
    #[serde(default)]
    pub width_amp: f64,
    /// ... its wavelength (0: constant width) ...
    #[serde(default)]
    pub width_wave: f64,
    /// ... and phase.
    #[serde(default)]
    pub width_phase: f64,
    /// EP3: stretches of steep or marshy bank.
    #[serde(default)]
    pub banks: Vec<crate::hydro::Bank>,
    /// EP3: x of the bridges (see [`Battlefield::bridges`]).
    #[serde(default)]
    pub bridge_xs: Vec<f64>,
}

impl River {
    /// z of the river centre line at `x`.
    pub fn center_z(&self, x: f64) -> f64 {
        self.z0 + self.amplitude * (x / self.wavelength * std::f64::consts::TAU + self.phase).sin()
    }

    pub fn in_water(&self, x: f64, z: f64) -> bool {
        (z - self.center_z(x)).abs() <= self.width_at(x) * 0.5
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
    /// EP3: tributary and brooks (shallow).
    #[serde(default)]
    pub streams: Vec<crate::hydro::Stream>,
    /// EP3: bridges over the river and the tributary.
    #[serde(default)]
    pub bridges: Vec<crate::hydro::Bridge>,
    /// EP3: still water of an oxbow (arc of discs).
    #[serde(default)]
    pub oxbows: Vec<Zone>,
    /// EP3: roads and tracks.
    #[serde(default)]
    pub roads: Vec<crate::hydro::Road>,
    /// EP6: hamlets, mills, church, manor, plots, props and the camps.
    #[serde(default)]
    pub decor: crate::decor::Decor,
    /// RC2: the field is a campaign river crossing (its single passage is
    /// the only bridge or ford of the main river).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crossing: Option<CrossingStructure>,
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
        Self::generate_sized(FieldSize::STANDARD, terrain, river, weather, rng)
    }

    /// EP3: bridges (deck on the final banks) and roads, from a stream
    /// derived from `rng` (not advanced).
    fn finish_water(&mut self, rng: &BattleRng) {
        let rules = hydro::WaterRules::bundled();
        let mut stream = rng.derive(hydro::ROADS_STREAM);
        hydro::build_bridges(self, rules, &mut stream);
        hydro::lay_roads(self, rules, &mut stream);
    }

    /// EP3: distance from (x, z) to the nearest water's edge (river,
    /// streams, oxbow); infinite without water.
    pub fn water_gap(&self, x: f64, z: f64) -> f64 {
        let mut gap = f64::INFINITY;
        if let Some(r) = &self.river {
            gap = gap.min((z - r.center_z(x)).abs() - r.width_at(x) * 0.5);
        }
        for s in &self.streams {
            gap = gap.min(s.distance(x, z) - s.width * 0.5);
        }
        for o in &self.oxbows {
            gap = gap.min((x - o.x).hypot(z - o.z) - o.radius);
        }
        gap
    }

    /// [`Self::generate`] on a field of `size` (EP1).
    pub fn generate_sized(
        size: FieldSize,
        terrain: Terrain,
        river: bool,
        weather: Weather,
        rng: &mut BattleRng,
    ) -> Self {
        let mut field = Self::generate_base(size, terrain, river, None, weather, rng);
        field.finish_water(rng);
        field
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
        Self::generate_site_crossing(site, None, size, weather, rng)
    }

    /// [`Self::generate_site_sized`] at a campaign river crossing (RC2, ADR
    /// 0117): with `crossing`, the main river runs between the battle lines
    /// with that single passage near the centre (drawn from a derived
    /// stream: `rng` advances exactly as without it); `None` is the plain
    /// site, draw for draw.
    pub fn generate_site_crossing(
        site: &FieldSite,
        crossing: Option<CrossingStructure>,
        size: FieldSize,
        weather: Weather,
        rng: &mut BattleRng,
    ) -> Self {
        let mut field =
            Self::generate_base(size, site.terrain, site.river, crossing, weather, rng);
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
        // EP3: wider rivers, streams and oxbows are kept clear too.
        let gap = |x: f64, z: f64| field.water_gap(x, z);
        let gap_fn: &dyn Fn(f64, f64) -> f64 = &gap;
        let occupied = Occupied {
            forests: &field.forests,
            mud: &field.mud,
            parts: &parts,
            river_z: field.river.is_some().then_some(river_fn),
            size: field.size,
            water_gap: field.river.is_some().then_some(gap_fn),
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
        field.finish_water(rng);
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
        crossing: Option<CrossingStructure>,
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
            // OM3: plains ground, open and treeless; low dunes in the desert.
            Terrain::Steppe => (3, 4.0, 0, 0),
            Terrain::Desert => (4, 6.0, 0, 0),
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
        let mut river_def = river.then(|| {
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
                width_amp: 0.0,
                width_wave: 0.0,
                width_phase: 0.0,
                banks: Vec::new(),
                bridge_xs: Vec::new(),
            }
        });
        // EP3: width, fords, bridges and banks from a derived stream (the
        // draws above are the pre-EP3 ones).
        let mut hydro_stream = rng.derive(hydro::HYDRO_STREAM);
        let water_rules = hydro::WaterRules::bundled();
        if let Some(structure) = crossing {
            // RC2: the crossing's river replaces the drawn one (the draws
            // above still happen: `rng` is unchanged).
            let mut stream = rng.derive(hydro::CROSSING_STREAM);
            river_def = Some(hydro::crossing_river(
                structure,
                &size,
                water_rules,
                &mut stream,
            ));
        } else if let Some(r) = river_def.as_mut() {
            hydro::shape_river(r, terrain, width, water_rules, &mut hydro_stream);
        }
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
                    h -= hydro::river_carve(r, x, z);
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
        let mut field = Battlefield {
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
            streams: Vec::new(),
            bridges: Vec::new(),
            oxbows: Vec::new(),
            roads: Vec::new(),
            decor: Default::default(),
            crossing,
        };
        hydro::draw_streams(&mut field, water_rules, &mut hydro_stream);
        field
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
        // EP3: no streams, bridges nor roads in a siege (the gate road is
        // drawn by the renderer).
        self.streams.clear();
        self.bridges.clear();
        self.oxbows.clear();
        self.roads.clear();
        // EP6: no countryside decor nor camps round a besieged town.
        self.decor = Default::default();
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
        // EP6: lanes of the hamlets, vines, orchards, muddy furrows, camps.
        let wet = weather == Weather::Rain || self.ground == Ground::Muddy;
        factor * self.decor_speed_factor(x, z, mounted, wet)
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
            (ObstacleKind::Palisade, "palissade"),
        ] {
            if self.obstacles.iter().any(|o| o.kind == kind) {
                parts.push(label.to_owned());
            }
        }
        if !self.pools.is_empty() {
            parts.push("mares".to_owned());
        }
        if let Some(river) = &self.river {
            let bridges = self.bridges.iter().filter(|b| b.stream.is_none()).count();
            let fords = river.fords.len();
            parts.push(match (bridges, fords) {
                (0, _) => "rivière et gués".to_owned(),
                (_, 0) => "rivière et ponts".to_owned(),
                _ => "rivière, gués et ponts".to_owned(),
            });
        }
        if !self.streams.is_empty() {
            parts.push("ruisseaux".to_owned());
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
    /// in front of the target (B5: archers behind a hedge). Shooters nearer
    /// the hedge than their target hold it: they shoot over their own hedge
    /// (Poitiers), and the target on the far side gets no cover from it.
    pub fn hedge_between(&self, from: (f64, f64), to: (f64, f64)) -> bool {
        self.obstacles.iter().any(|o| {
            let target_distance = o.distance(to.0, to.1);
            o.kind.gives_cover()
                && target_distance <= HEDGE_COVER_REACH
                && target_distance <= o.distance(from.0, from.1)
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
    /// Marsh pools count as shallow water (B5); so do streams and oxbows,
    /// and a bridge deck is dry land (EP3, [`Self::water_kind`]).
    pub fn water_at(&self, x: f64, z: f64) -> Option<bool> {
        self.water_kind(x, z).map(|w| !w.deep())
    }

    pub fn inside(&self, x: f64, z: f64) -> bool {
        (0.0..=self.width).contains(&x) && (0.0..=self.depth).contains(&z)
    }

    /// `(x, z)` brought at least `margin` metres inside the field (AI
    /// orders: a move outside the field is refused).
    pub fn clamp_inside(&self, x: f64, z: f64, margin: f64) -> (f64, f64) {
        (
            x.clamp(margin, self.width - margin),
            z.clamp(margin, self.depth - margin),
        )
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
