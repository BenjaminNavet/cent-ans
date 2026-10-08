//! Water and paths of the battlefield (lot EP3, ADR 0033).
//!
//! - **Main river** ([`River`], west to east between the armies): mean width
//!   drawn by terrain (10-40 m), varying along the course; fords many or rare
//!   by width; 0 to 3 **bridges** of wood or stone (4-8 m deck: a true
//!   bottleneck); stretches of **steep** or **marshy** bank; an **oxbow**
//!   (still water in an abandoned bend).
//! - **Streams**: sometimes a tributary from a field edge on a flank, and a
//!   few narrow brooks; both shallow (crossable, slowing), with a wooden
//!   footbridge where a road meets the tributary.
//! - **Roads** ([`Road`]): through every bridge and most fords from one edge
//!   of the field to the other, through the village, or across the field
//!   when there is no river; marching in column on a road is faster.
//! - **Rules** read by the simulation: deep water stops horsemen and engines
//!   and is very slow and dangerous on foot (drowning, fatigue, morale); a
//!   regiment wider than a bridge files across it slowly and fights badly
//!   from it, and whoever holds the bridgehead strikes harder.
//!
//! Everything is drawn from derived streams ([`BattleRng::derive`]): the
//! draws of the older battles (units, weather, relief) are unchanged. All
//! numbers come from `data/rules/battle_water.json` (schema
//! `data/schemas/battle_water_rules.schema.json`).

use std::f64::consts::TAU;
use data_model::Terrain;
use serde::{Deserialize, Serialize};

use crate::field::{Battlefield, River, Zone};
use crate::rng::BattleRng;
use crate::scale::FieldSize;
use crate::setup::CrossingStructure;

/// Salt of the derived stream of the river network (width, fords, bridges,
/// banks, streams, oxbow).
pub(crate) const HYDRO_STREAM: u64 = 0xE3_0A;
/// Salt of the derived stream of the roads.
pub(crate) const ROADS_STREAM: u64 = 0xE3_0B;
/// Salt of the derived stream of a campaign crossing's river (RC2, ADR
/// 0141): drawn only when the setup has a crossing, so the other battles
/// keep every draw.
pub(crate) const CROSSING_STREAM: u64 = 0xC2_0A;

// ----- rules -------------------------------------------------------------------

/// `[min, max]`.
pub type Span = [f64; 2];
/// `[min, max]` (inclusive).
pub type CountSpan = [u32; 2];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiverWidths {
    pub plains: Span,
    pub heath: Span,
    pub bocage: Span,
    pub forest: Span,
    pub hills: Span,
    pub mountains: Span,
    pub marsh: Span,
    /// OM3 (ADR 0116).
    pub steppe: Span,
    pub desert: Span,
}

impl RiverWidths {
    pub fn of(&self, terrain: Terrain) -> Span {
        match terrain {
            Terrain::Plains => self.plains,
            Terrain::Heath => self.heath,
            Terrain::Bocage => self.bocage,
            Terrain::Forest => self.forest,
            Terrain::Hills => self.hills,
            Terrain::Mountains => self.mountains,
            Terrain::Marsh => self.marsh,
            Terrain::Steppe => self.steppe,
            Terrain::Desert => self.desert,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossingRow {
    pub max_width_m: f64,
    pub fords: CountSpan,
    pub bridges: CountSpan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeWidths {
    pub wood: Span,
    pub stone: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiverRules {
    pub width_m: RiverWidths,
    pub width_variation: f64,
    pub ford_half_width_m: Span,
    pub crossings: Vec<CrossingRow>,
    pub stone_bridge_chance: f64,
    pub stone_bridge_from_width_m: f64,
    pub bridge_width_m: BridgeWidths,
    pub abutment_m: f64,
    pub steep_bank_chance: f64,
    pub marsh_bank_chance: f64,
    pub oxbow_chance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamRules {
    pub tributary_chance: f64,
    pub tributary_width_m: Span,
    pub brooks: CountSpan,
    pub brook_width_m: Span,
    pub footbridge_chance: f64,
}

/// Speed multipliers (see the schema).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementRules {
    pub ford: f64,
    pub deep_foot: f64,
    pub deep_mounted: f64,
    pub tributary: f64,
    pub brook: f64,
    pub oxbow: f64,
    pub steep_bank: f64,
    pub marsh_bank: f64,
    pub bank_band_m: f64,
    pub deep_blocks_mounted: bool,
    pub deep_blocks_engines: bool,
    pub road_column: f64,
    pub road_other: f64,
    pub bridge_min_squeeze: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeepWaterRules {
    pub fatigue_per_s: f64,
    pub morale_per_s: f64,
    pub drown_share_per_s: f64,
    pub armour_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterCombatRules {
    pub bridge_head_reach_m: f64,
    pub bridge_holder_bonus: f64,
    pub ford_attacker: f64,
    pub deep_attacker: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadRules {
    pub main_width_m: f64,
    pub track_width_m: f64,
    pub wander_m: f64,
    pub ford_road_chance: f64,
}

/// RC2 (ADR 0141): the river of a battle fought at a campaign crossing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossingSiteRules {
    /// Mean width of the river at a bridge, a boat bridge or a ferry: too
    /// wide to wade anywhere but at the passage.
    pub bridge_river_width_m: Span,
    /// Mean width of the river at a ford.
    pub ford_river_width_m: Span,
    /// Meander amplitude (kept small: the river stays between the lines).
    pub amplitude_m: Span,
    /// The passage lies within this distance of the field's centre (x).
    pub passage_jitter_m: f64,
    /// Half-width of the shallow landing standing for a ferry.
    pub ferry_half_width_m: f64,
}

/// Contents of `data/rules/battle_water.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaterRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub river: RiverRules,
    pub streams: StreamRules,
    pub movement: MovementRules,
    pub deep_water: DeepWaterRules,
    pub combat: WaterCombatRules,
    pub roads: RoadRules,
    pub crossing: CrossingSiteRules,
}

data_model::bundled_rules!(WaterRules, "rules/battle_water.json");

// ----- features ------------------------------------------------------------------

/// Kind of a stretch of river bank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankKind {
    /// Steep, high bank: slow to climb, breaks a charge.
    Steep,
    /// Reeds and mire along the water.
    Marsh,
}

impl BankKind {
    pub fn key(self) -> &'static str {
        match self {
            BankKind::Steep => "steep",
            BankKind::Marsh => "marsh",
        }
    }
}

/// A stretch of the main river's bank, from `x0` to `x1`, on the north
/// (+z) or south side.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bank {
    pub x0: f64,
    pub x1: f64,
    pub north: bool,
    pub kind: BankKind,
}

/// A shallow watercourse (tributary or brook).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    Tributary,
    Brook,
}

impl StreamKind {
    pub fn key(self) -> &'static str {
        match self {
            StreamKind::Tributary => "tributary",
            StreamKind::Brook => "brook",
        }
    }
}

/// A tributary or a brook: a polyline of centre points, shallow water.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stream {
    pub kind: StreamKind,
    pub points: Vec<(f64, f64)>,
    pub width: f64,
}

impl Stream {
    /// Distance from (x, z) to the centre line.
    pub fn distance(&self, x: f64, z: f64) -> f64 {
        polyline_distance(&self.points, x, z)
    }

    pub fn in_water(&self, x: f64, z: f64) -> bool {
        bounds_near(&self.points, x, z, self.width) && self.distance(x, z) <= self.width * 0.5
    }
}

/// A bridge: a deck `width` wide and `length` long centred on (x, z), along
/// the unit vector `dir` (from the south or west end to the other).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bridge {
    pub x: f64,
    pub z: f64,
    pub dir: (f64, f64),
    pub length: f64,
    pub width: f64,
    /// Width of the water under the bridge (the deck rests on the banks
    /// beyond it).
    pub span: f64,
    /// Height of the deck (metres).
    pub deck: f64,
    /// Stone (arches, parapets) or wood (deck on piles).
    pub stone: bool,
    /// Arches of a stone bridge (0 for wood).
    pub arches: u32,
    /// `None` on the main river, else the index of the stream.
    pub stream: Option<usize>,
}

impl Bridge {
    /// (along, across) of (x, z) in the bridge frame (along from the centre).
    pub fn local(&self, x: f64, z: f64) -> (f64, f64) {
        let (dx, dz) = (x - self.x, z - self.z);
        (
            dx * self.dir.0 + dz * self.dir.1,
            -dx * self.dir.1 + dz * self.dir.0,
        )
    }

    /// On the deck (abutments included).
    pub fn on_deck(&self, x: f64, z: f64) -> bool {
        let (along, across) = self.local(x, z);
        along.abs() <= self.length * 0.5 && across.abs() <= self.width * 0.5
    }

    /// Both ends of the deck: `[start, end]` (start towards -dir).
    pub fn ends(&self) -> [(f64, f64); 2] {
        let h = self.length * 0.5;
        [
            (self.x - self.dir.0 * h, self.z - self.dir.1 * h),
            (self.x + self.dir.0 * h, self.z + self.dir.1 * h),
        ]
    }

    /// Yaw of the deck (radians, from +x towards +z).
    pub fn yaw(&self) -> f64 {
        self.dir.1.atan2(self.dir.0)
    }
}

/// Main road (through a bridge, the village) or farm track (through a ford).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoadKind {
    Main,
    Track,
}

impl RoadKind {
    pub fn key(self) -> &'static str {
        match self {
            RoadKind::Main => "main",
            RoadKind::Track => "track",
        }
    }
}

/// A road: centre line and width.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Road {
    pub kind: RoadKind,
    pub points: Vec<(f64, f64)>,
    pub width: f64,
}

impl Road {
    pub fn on_road(&self, x: f64, z: f64) -> bool {
        bounds_near(&self.points, x, z, self.width)
            && polyline_distance(&self.points, x, z) <= self.width * 0.5 + 1.0
    }
}

/// What kind of water stands at a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Water {
    /// Deep water of the main river.
    Deep,
    /// A ford of the main river.
    Ford,
    Stream(StreamKind),
    /// An oxbow (still water of an abandoned bend).
    Oxbow,
    /// A marsh pool (B5).
    Pool,
}

impl Water {
    pub fn deep(self) -> bool {
        self == Water::Deep
    }
}

/// A place where a regiment crosses the main river: a bridge or a ford.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    /// Index in `field.bridges` for a bridge, `None` for a ford.
    pub bridge: Option<usize>,
    /// Index in `river.fords` for a ford.
    pub ford: Option<usize>,
    /// South end (low z) and north end.
    pub south: (f64, f64),
    pub north: (f64, f64),
    /// Usable width (deck or ford), metres.
    pub width: f64,
}

impl Crossing {
    pub fn middle(&self) -> (f64, f64) {
        (
            (self.south.0 + self.north.0) * 0.5,
            (self.south.1 + self.north.1) * 0.5,
        )
    }

    /// End on the side `north` (+z) or south.
    pub fn end(&self, north: bool) -> (f64, f64) {
        if north {
            self.north
        } else {
            self.south
        }
    }
}

/// A spot on a bank for a building by the water (EP6: water mill): the
/// point on the bank, the direction of the water (unit vector) and the kind
/// of water.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WatersideSpot {
    pub x: f64,
    pub z: f64,
    /// Unit vector from the spot towards the water.
    pub towards_water: (f64, f64),
    /// Main river (`None`) or index of the stream.
    pub stream: Option<usize>,
}

// ----- geometry helpers ---------------------------------------------------------------

/// Distance from (x, z) to a polyline.
pub fn polyline_distance(points: &[(f64, f64)], x: f64, z: f64) -> f64 {
    if points.len() == 1 {
        return (x - points[0].0).hypot(z - points[0].1);
    }
    points
        .windows(2)
        .map(|w| segment_distance((x, z), w[0], w[1]))
        .fold(f64::INFINITY, f64::min)
}

fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dz * dz;
    let t = if len2 < 1e-9 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - dx * t).hypot(p.1 - a.1 - dz * t)
}

/// Cheap reject: is (x, z) within `margin` of the bounding box of `points`?
fn bounds_near(points: &[(f64, f64)], x: f64, z: f64, margin: f64) -> bool {
    let (mut x0, mut x1, mut z0, mut z1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for &(px, pz) in points {
        x0 = x0.min(px);
        x1 = x1.max(px);
        z0 = z0.min(pz);
        z1 = z1.max(pz);
    }
    x >= x0 - margin && x <= x1 + margin && z >= z0 - margin && z <= z1 + margin
}

/// Do segments a-b and c-d cross? Returns the parameter along a-b.
fn segments_cross(
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
    d: (f64, f64),
) -> Option<(f64, (f64, f64))> {
    let r = (b.0 - a.0, b.1 - a.1);
    let s = (d.0 - c.0, d.1 - c.1);
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() < 1e-9 {
        return None;
    }
    let t = ((c.0 - a.0) * s.1 - (c.1 - a.1) * s.0) / den;
    let u = ((c.0 - a.0) * r.1 - (c.1 - a.1) * r.0) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u))
        .then_some((t, (a.0 + r.0 * t, a.1 + r.1 * t)))
}

/// Two passes of Chaikin smoothing (ends kept).
pub(crate) fn chaikin(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut result = points.to_vec();
    for _ in 0..2 {
        if result.len() < 3 {
            return result;
        }
        let mut next = vec![result[0]];
        for w in result.windows(2) {
            let (a, b) = (w[0], w[1]);
            next.push((a.0 * 0.75 + b.0 * 0.25, a.1 * 0.75 + b.1 * 0.25));
            next.push((a.0 * 0.25 + b.0 * 0.75, a.1 * 0.25 + b.1 * 0.75));
        }
        next.push(*result.last().expect("non-empty"));
        result = next;
    }
    result
}

fn draw_count(span: CountSpan, stream: &mut BattleRng) -> u32 {
    let (low, high) = (span[0].min(span[1]), span[0].max(span[1]));
    low + stream.below(high - low + 1)
}

fn draw_span(span: Span, stream: &mut BattleRng) -> f64 {
    stream.range(span[0], span[1])
}

// ----- the main river -------------------------------------------------------------------

impl River {
    /// Width of the water at `x` (EP3: it varies along the course).
    pub fn width_at(&self, x: f64) -> f64 {
        if self.width_wave <= 0.0 {
            return self.width;
        }
        self.width * (1.0 + self.width_amp * (x / self.width_wave * TAU + self.width_phase).sin())
    }

    /// Slope dz/dx of the centre line at `x`.
    pub fn slope(&self, x: f64) -> f64 {
        self.amplitude * TAU / self.wavelength * (x / self.wavelength * TAU + self.phase).cos()
    }

    /// Unit normal of the centre line at `x`, pointing north (+z).
    pub fn normal(&self, x: f64) -> (f64, f64) {
        let s = self.slope(x);
        let n = (1.0 + s * s).sqrt();
        (-s / n, 1.0 / n)
    }

    /// `true` north (+z) of the centre line.
    pub fn north_of(&self, x: f64, z: f64) -> bool {
        z > self.center_z(x)
    }

    /// The bank stretch at x on the given side, if any.
    pub fn bank_at(&self, x: f64, north: bool) -> Option<BankKind> {
        self.banks
            .iter()
            .find(|b| b.north == north && (b.x0..=b.x1).contains(&x))
            .map(|b| b.kind)
    }
}

/// EP3: width, fords, bridges (positions), banks of the main river, drawn
/// from the hydro stream. `base_fords` (the two pre-EP3 fords, drawn from
/// the battle stream) are kept as the first candidates.
pub(crate) fn shape_river(
    river: &mut River,
    terrain: Terrain,
    field_width: f64,
    rules: &WaterRules,
    stream: &mut BattleRng,
) {
    let r = &rules.river;
    river.width = draw_span(r.width_m.of(terrain), stream);
    river.width_amp = stream.range(0.0, r.width_variation);
    river.width_wave = stream.range(260.0, 620.0);
    river.width_phase = stream.range(0.0, TAU);
    let row = r
        .crossings
        .iter()
        .find(|row| river.width <= row.max_width_m)
        .or(r.crossings.last())
        .expect("at least one crossing row");
    let ford_count = draw_count(row.fords, stream) as usize;
    let mut bridge_count = draw_count(row.bridges, stream) as usize;
    if ford_count == 0 && bridge_count == 0 {
        bridge_count = 1;
    }
    // Fords: the pre-EP3 positions first, then new ones spread out.
    let mut fords: Vec<crate::field::Ford> = river.fords.iter().take(ford_count).copied().collect();
    let (lo, hi) = (field_width * 0.08, field_width * 0.92);
    let mut tries = 0;
    while fords.len() < ford_count && tries < 60 {
        tries += 1;
        let x = stream.range(lo, hi);
        if fords.iter().all(|f| (f.x - x).abs() > 140.0) {
            fords.push(crate::field::Ford { x, half_width: 0.0 });
        }
    }
    for ford in &mut fords {
        // Wider rivers have narrower fords (a shoal rather than a shallow reach).
        let half = draw_span(r.ford_half_width_m, stream);
        ford.half_width = (half * (24.0 / river.width.max(10.0)).clamp(0.6, 1.2)).max(12.0);
    }
    fords.sort_by(|a, b| a.x.total_cmp(&b.x));
    river.fords = fords;
    // Bridges: away from the fords and from each other.
    let mut xs: Vec<f64> = Vec::new();
    tries = 0;
    while xs.len() < bridge_count && tries < 80 {
        tries += 1;
        let x = stream.range(field_width * 0.1, field_width * 0.9);
        let clear_fords = river
            .fords
            .iter()
            .all(|f| (f.x - x).abs() > f.half_width + 70.0);
        if clear_fords && xs.iter().all(|&o| (o - x).abs() > 180.0) {
            xs.push(x);
        }
    }
    xs.sort_by(f64::total_cmp);
    river.bridge_xs = xs;
    // Banks: stretches of steep or marshy bank on either side, away from
    // the crossings.
    river.banks = draw_banks(field_width, r, stream);
}

/// Stretches of steep or marshy bank on either side of the main river.
fn draw_banks(field_width: f64, r: &RiverRules, stream: &mut BattleRng) -> Vec<Bank> {
    let mut banks = Vec::new();
    for north in [false, true] {
        let mut x = stream.range(0.0, 120.0);
        while x < field_width {
            let len = stream.range(120.0, 320.0);
            let roll = stream.unit();
            let kind = if roll < r.steep_bank_chance {
                Some(BankKind::Steep)
            } else if roll < r.steep_bank_chance + r.marsh_bank_chance {
                Some(BankKind::Marsh)
            } else {
                None
            };
            if let Some(kind) = kind {
                banks.push(Bank {
                    x0: x,
                    x1: (x + len).min(field_width),
                    north,
                    kind,
                });
            }
            x += len + stream.range(40.0, 200.0);
        }
    }
    banks
}

/// RC2 (ADR 0141): the main river of a battle fought at a campaign
/// crossing, drawn from its own stream: across the field between the two
/// battle lines, wide, with exactly one passage near the centre:
///
/// - stone or wooden bridge: that bridge, no ford;
/// - bridge of boats: a wooden bridge at the narrow end of the deck widths
///   (no boat-bridge model yet);
/// - ford: a single ford, no bridge;
/// - ferry: a single narrow ford (approximation: the landing, fought over
///   as a shallow crossing; no boat is simulated).
///
/// Banks as on any river; streams, oxbow and roads follow as usual.
pub(crate) fn crossing_river(
    structure: CrossingStructure,
    size: &FieldSize,
    rules: &WaterRules,
    stream: &mut BattleRng,
) -> River {
    let (r, c) = (&rules.river, &rules.crossing);
    let bridged = !matches!(
        structure,
        CrossingStructure::Ford | CrossingStructure::Ferry
    );
    let z0 = (size.attacker_line_z() + size.defender_line_z()) * 0.5 + stream.range(-15.0, 15.0);
    let amplitude = draw_span(c.amplitude_m, stream);
    let wavelength = stream.range(500.0, 900.0);
    let phase = stream.range(0.0, TAU);
    let width = draw_span(
        if bridged {
            c.bridge_river_width_m
        } else {
            c.ford_river_width_m
        },
        stream,
    );
    let width_amp = stream.range(0.0, r.width_variation * 0.5);
    let width_wave = stream.range(260.0, 620.0);
    let width_phase = stream.range(0.0, TAU);
    let x = size.center_x() + stream.range(-c.passage_jitter_m, c.passage_jitter_m);
    let (fords, bridge_xs) = match structure {
        CrossingStructure::Ford => {
            let half = draw_span(r.ford_half_width_m, stream);
            let half_width = (half * (24.0 / width.max(10.0)).clamp(0.6, 1.2)).max(12.0);
            (vec![crate::field::Ford { x, half_width }], Vec::new())
        }
        CrossingStructure::Ferry => (
            vec![crate::field::Ford {
                x,
                half_width: c.ferry_half_width_m,
            }],
            Vec::new(),
        ),
        _ => (Vec::new(), vec![x]),
    };
    let banks = draw_banks(size.width, r, stream);
    River {
        z0,
        amplitude,
        wavelength,
        phase,
        width,
        fords,
        width_amp,
        width_wave,
        width_phase,
        banks,
        bridge_xs,
    }
}

/// Carving of the main river bed at (x, z) (metres to subtract), with its
/// banks: steep banks cut a sharp edge with a raised lip, marshy banks a
/// wide, low, flat margin.
pub(crate) fn river_carve(river: &River, x: f64, z: f64) -> f64 {
    let c = river.center_z(x);
    let d = (z - c).abs();
    let half = river.width_at(x) * 0.5;
    let ford = river.in_ford(x);
    let depth = if ford { 0.6 } else { 1.6 };
    let bank = if ford || river.near_bridge(x) {
        None
    } else {
        river.bank_at(x, z > c)
    };
    match bank {
        Some(BankKind::Steep) => {
            if d <= half {
                depth + 0.4
            } else if d <= half + 3.0 {
                (depth + 0.4) * (1.0 - (d - half) / 3.0) - 1.4 * ((d - half) / 3.0)
            } else if d <= half + 14.0 {
                // The lip of the bluff.
                -1.4 * (1.0 - (d - half - 3.0) / 11.0)
            } else {
                0.0
            }
        }
        Some(BankKind::Marsh) => {
            let reach = half * 1.5 + 30.0;
            if d <= half {
                depth * (1.0 - d / (half * 1.5))
            } else if d < reach {
                let base = if d < half * 1.5 {
                    depth * (1.0 - d / (half * 1.5))
                } else {
                    0.0
                };
                base.max(0.45 * (1.0 - (d - half) / (reach - half)))
            } else {
                0.0
            }
        }
        None => {
            let reach = half * 3.0;
            if d < reach {
                depth * (1.0 - d / reach)
            } else {
                0.0
            }
        }
    }
}

impl River {
    /// Within 20 m of a bridge planned at `bridge_xs`.
    pub fn near_bridge(&self, x: f64) -> bool {
        self.bridge_xs.iter().any(|&b| (b - x).abs() < 20.0)
    }
}

// ----- streams and oxbow -----------------------------------------------------------------

/// Keeps still water off the deployment lines `lines`.
fn clear_of_lines(lines: (f64, f64), z: f64, radius: f64) -> bool {
    (z - lines.0).abs() > radius + 45.0 && (z - lines.1).abs() > radius + 45.0
}

/// A meandering polyline from `a` to `b` (points every ~20 m).
fn meander(a: (f64, f64), b: (f64, f64), amp: f64, stream: &mut BattleRng) -> Vec<(f64, f64)> {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len = dx.hypot(dz).max(1.0);
    let normal = (-dz / len, dx / len);
    let n = ((len / 20.0).ceil() as usize).max(2);
    let waves: Vec<(f64, f64, f64)> = (0..3)
        .map(|k| {
            (
                amp / (k as f64 + 1.0) * stream.range(0.5, 1.0),
                stream.range(1.0, 3.5) * (k as f64 + 1.0),
                stream.range(0.0, TAU),
            )
        })
        .collect();
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            // Fixed ends: the offset fades out at both ends.
            let fade = (t * std::f64::consts::PI).sin();
            let offset: f64 = waves
                .iter()
                .map(|&(a, f, p)| a * (t * f * TAU + p).sin())
                .sum::<f64>()
                * fade;
            (
                a.0 + dx * t + normal.0 * offset,
                a.1 + dz * t + normal.1 * offset,
            )
        })
        .collect()
}

/// EP3: tributary, brooks and oxbow of a field with a river (drawn after
/// the relief; carved into `heights`).
pub(crate) fn draw_streams(field: &mut Battlefield, rules: &WaterRules, stream: &mut BattleRng) {
    let Some(river) = field.river.clone() else {
        return;
    };
    let (w, d) = (field.width, field.depth);
    let lines = (field.attacker_line_z(), field.defender_line_z());
    let s = &rules.streams;
    let mut streams = Vec::new();
    if stream.unit() < s.tributary_chance {
        let west = stream.unit() < 0.5;
        let x0 = if west {
            stream.range(w * 0.06, w * 0.22)
        } else {
            stream.range(w * 0.78, w * 0.94)
        };
        let north = stream.unit() < 0.5;
        let lean = stream.range(40.0, 180.0) * if west { 1.0 } else { -1.0 };
        let x1 = (x0 + lean).clamp(w * 0.04, w * 0.96);
        let start = (x0, if north { d } else { 0.0 });
        let end = (x1, river.center_z(x1));
        streams.push(Stream {
            kind: StreamKind::Tributary,
            points: meander(start, end, 30.0, stream),
            width: draw_span(s.tributary_width_m, stream),
        });
    }
    let brooks = draw_count(s.brooks, stream);
    for _ in 0..brooks {
        let x0 = stream.range(w * 0.05, w * 0.95);
        let north = stream.unit() < 0.5;
        let x1 = (x0 + stream.range(-150.0, 150.0)).clamp(w * 0.03, w * 0.97);
        let start = (x0, if north { d } else { 0.0 });
        let end = (x1, river.center_z(x1));
        let clash = streams
            .iter()
            .any(|other: &Stream| (other.points[0].0 - x0).abs() < 120.0);
        if clash {
            continue;
        }
        streams.push(Stream {
            kind: StreamKind::Brook,
            points: meander(start, end, 18.0, stream),
            width: draw_span(s.brook_width_m, stream),
        });
    }
    // Carve the beds (shallow), a little deeper for the tributary.
    for st in &streams {
        let depth = match st.kind {
            StreamKind::Tributary => 0.9,
            StreamKind::Brook => 0.45,
        };
        let reach = st.width * 1.6 + 2.0;
        for iz in 0..field.nz {
            for ix in 0..field.nx {
                let x = ix as f64 * field.resolution;
                let z = iz as f64 * field.resolution;
                if !bounds_near(&st.points, x, z, reach) {
                    continue;
                }
                let dist = st.distance(x, z);
                if dist < reach {
                    // Not into the main river bed (already carved).
                    if river.in_water(x, z) {
                        continue;
                    }
                    // A shallow dip only across the deployment lines.
                    let line = (z - lines.0).abs().min((z - lines.1).abs());
                    let soften = 0.25 + 0.75 * ((line - 20.0) / 50.0).clamp(0.0, 1.0);
                    field.heights[iz * field.nx + ix] -= depth * soften * (1.0 - dist / reach);
                }
            }
        }
    }
    field.streams = streams;
    // Oxbow: an arc of still water in the floodplain, on one side.
    if stream.unit() < rules.river.oxbow_chance {
        for _ in 0..12 {
            let x = stream.range(w * 0.15, w * 0.85);
            let north = stream.unit() < 0.5;
            let sign = if north { 1.0 } else { -1.0 };
            let radius = stream.range(28.0, 55.0);
            let gap = river.width_at(x) * 0.5 + stream.range(18.0, 40.0);
            let c = (x, river.center_z(x) + sign * (gap + radius));
            if !clear_of_lines(lines, c.1, radius + 8.0)
                || field.in_forest(c.0, c.1)
                || field.village.is_some()
                    && field
                        .village
                        .as_ref()
                        .is_some_and(|v| v.zone.contains(c.0, c.1))
            {
                continue;
            }
            // Arc opening away from the river (the old bend).
            let facing = if north { -TAU / 4.0 } else { TAU / 4.0 };
            let sweep = stream.range(3.2, 4.2);
            let a0 = facing + std::f64::consts::PI - sweep * 0.5;
            let lobe = stream.range(6.0, 9.0);
            let count = ((sweep * radius) / (lobe * 1.2)).ceil() as usize;
            for k in 0..=count {
                let a = a0 + sweep * k as f64 / count as f64;
                let taper = 1.0 - 0.35 * ((k as f64 / count as f64) * 2.0 - 1.0).abs();
                field.oxbows.push(Zone {
                    x: c.0 + radius * a.cos(),
                    z: c.1 + radius * a.sin(),
                    radius: lobe * taper,
                });
            }
            break;
        }
    }
    // Marshy banks: mire along the water.
    let marsh: Vec<Bank> = river
        .banks
        .iter()
        .copied()
        .filter(|b| b.kind == BankKind::Marsh)
        .collect();
    for bank in marsh {
        let sign = if bank.north { 1.0 } else { -1.0 };
        let mut x = bank.x0 + 15.0;
        while x < bank.x1 - 10.0 {
            if !river.in_ford(x) && !river.near_bridge(x) {
                let r = stream.range(10.0, 16.0);
                let z = river.center_z(x) + sign * (river.width_at(x) * 0.5 + r * 0.7);
                if clear_of_lines(lines, z, r) {
                    field.mud_parts.push(Zone { x, z, radius: r });
                }
            }
            x += stream.range(18.0, 34.0);
        }
    }
}

// ----- bridges ----------------------------------------------------------------------------

/// EP3: the bridges of the main river (at `river.bridge_xs`), their deck
/// heights from the banks; called once the heights are final.
pub(crate) fn build_bridges(field: &mut Battlefield, rules: &WaterRules, stream: &mut BattleRng) {
    let Some(river) = field.river.clone() else {
        return;
    };
    let r = &rules.river;
    for &x in &river.bridge_xs {
        let span = river.width_at(x);
        // RC2: the campaign crossing decides the kind; a bridge of boats is
        // a wooden deck at the narrow end of the widths.
        let (stone, width) = match field.crossing {
            Some(CrossingStructure::StoneBridge) => {
                (true, draw_span(r.bridge_width_m.stone, stream))
            }
            Some(CrossingStructure::WoodBridge) => {
                (false, draw_span(r.bridge_width_m.wood, stream))
            }
            Some(CrossingStructure::BoatBridge) => (false, r.bridge_width_m.wood[0]),
            _ => {
                let stone = river.width >= r.stone_bridge_from_width_m
                    || stream.unit() < r.stone_bridge_chance;
                let width = draw_span(
                    if stone {
                        r.bridge_width_m.stone
                    } else {
                        r.bridge_width_m.wood
                    },
                    stream,
                );
                (stone, width)
            }
        };
        let arches = if stone {
            ((span / 9.0).round() as u32).max(1)
        } else {
            0
        };
        let dir = river.normal(x);
        let length = span + 2.0 * r.abutment_m;
        let mut bridge = Bridge {
            x,
            z: river.center_z(x),
            dir,
            length,
            width,
            span,
            deck: 0.0,
            stone,
            arches,
            stream: None,
        };
        bridge.deck = deck_height(field, &bridge);
        field.bridges.push(bridge);
    }
}

/// Height of a stone bridge's road above the higher bank (arches clear of
/// the water); the abutment ramps down by as much (the kit pieces of
/// `tools/blender_scripts/building_kit.py` share these numbers).
pub const STONE_DECK_RISE: f64 = 2.4;
/// Same for a wooden bridge.
pub const WOOD_DECK_RISE: f64 = 1.0;

/// Deck height: above both banks (at the ends of the deck).
fn deck_height(field: &Battlefield, bridge: &Bridge) -> f64 {
    let [a, b] = bridge.ends();
    field.height(a.0, a.1).max(field.height(b.0, b.1))
        + if bridge.stone {
            STONE_DECK_RISE
        } else {
            WOOD_DECK_RISE
        }
}

// ----- roads ------------------------------------------------------------------------------

/// A road from `from` heading `dir` until it leaves the field, wandering.
fn road_to_edge(
    field: &Battlefield,
    from: (f64, f64),
    dir: (f64, f64),
    wander: f64,
    stream: &mut BattleRng,
) -> Vec<(f64, f64)> {
    let mut points = vec![from];
    // A straight run off the crossing first.
    let mut p = (from.0 + dir.0 * 15.0, from.1 + dir.1 * 15.0);
    points.push(p);
    let mut heading = dir.1.atan2(dir.0);
    let base = heading;
    for _ in 0..60 {
        heading = base + (heading - base + stream.range(-0.35, 0.35)).clamp(-0.5, 0.5);
        let step = 45.0;
        p = (p.0 + heading.cos() * step, p.1 + heading.sin() * step);
        let lateral = stream.range(-wander, wander) * 0.3;
        p = (p.0 - heading.sin() * lateral, p.1 + heading.cos() * lateral);
        if !field.inside(p.0, p.1) {
            // Clip on the edge.
            p = (p.0.clamp(0.0, field.width), p.1.clamp(0.0, field.depth));
            points.push(p);
            break;
        }
        points.push(p);
    }
    points
}

/// Nearest point of the polyline `points` to `p`.
pub(crate) fn nearest_on(points: &[(f64, f64)], p: (f64, f64)) -> (f64, f64) {
    let mut best = (points[0], f64::INFINITY);
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len2 = (dx * dx + dz * dz).max(1e-9);
        let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dz) / len2).clamp(0.0, 1.0);
        let q = (a.0 + dx * t, a.1 + dz * t);
        let d = (q.0 - p.0).hypot(q.1 - p.1);
        if d < best.1 {
            best = (q, d);
        }
    }
    best.0
}

/// EP3: roads through the bridges and most fords, the village, or across
/// the field without a river; a footbridge where a road meets the
/// tributary. Deterministic (stream derived from the battle stream).
pub(crate) fn lay_roads(field: &mut Battlefield, rules: &WaterRules, stream: &mut BattleRng) {
    let rr = &rules.roads;
    let mut roads: Vec<Road> = Vec::new();
    let through = |field: &Battlefield,
                   south: (f64, f64),
                   north: (f64, f64),
                   kind: RoadKind,
                   stream: &mut BattleRng| {
        let dir = ((north.0 - south.0), (north.1 - south.1));
        let len = dir.0.hypot(dir.1).max(1e-6);
        let dir = (dir.0 / len, dir.1 / len);
        let mut down = road_to_edge(field, south, (-dir.0, -dir.1), rr.wander_m, stream);
        let up = road_to_edge(field, north, dir, rr.wander_m, stream);
        down.reverse();
        down.extend(up);
        Road {
            kind,
            points: chaikin(&down),
            width: match kind {
                RoadKind::Main => rr.main_width_m,
                RoadKind::Track => rr.track_width_m,
            },
        }
    };
    let crossings = field.crossings();
    for crossing in &crossings {
        // RC2: the campaign road always runs through the crossing.
        let kind = if crossing.bridge.is_some() || field.crossing.is_some() {
            RoadKind::Main
        } else if stream.unit() < rr.ford_road_chance {
            RoadKind::Track
        } else {
            continue;
        };
        roads.push(through(field, crossing.south, crossing.north, kind, stream));
    }
    if field.river.is_none() {
        let x = stream.range(field.width * 0.3, field.width * 0.7);
        let mid = (x, field.depth * 0.5);
        roads.push(through(
            field,
            (mid.0, mid.1 - 5.0),
            (mid.0, mid.1 + 5.0),
            RoadKind::Main,
            stream,
        ));
    }
    if let Some(village) = field.village.clone() {
        let centre = (village.zone.x, village.zone.z);
        // A lane from the village to the nearest road ...
        let nearest = roads
            .iter()
            .map(|r| nearest_on(&r.points, centre))
            .min_by(|a, b| {
                let da = (a.0 - centre.0).hypot(a.1 - centre.1);
                let db = (b.0 - centre.0).hypot(b.1 - centre.1);
                da.total_cmp(&db)
            });
        if let Some(q) = nearest {
            let d = (q.0 - centre.0).hypot(q.1 - centre.1);
            if d > 20.0 && !field.path_wet(centre, q) {
                let mut pts = meander(centre, q, (d * 0.08).min(20.0), stream);
                pts = chaikin(&pts);
                roads.push(Road {
                    kind: RoadKind::Track,
                    points: pts,
                    width: rr.track_width_m,
                });
            }
        }
        // ... and a road out to the nearer side edge.
        let west = centre.0 < field.width * 0.5;
        let dir = if west { (-1.0, 0.0) } else { (1.0, 0.0) };
        let out = road_to_edge(field, centre, dir, rr.wander_m, stream);
        if !out.windows(2).any(|w| field.path_wet(w[0], w[1])) {
            roads.push(Road {
                kind: RoadKind::Main,
                points: chaikin(&out),
                width: rr.main_width_m,
            });
        }
    }
    // Footbridges where a road meets the tributary.
    let mut footbridges = Vec::new();
    for (si, st) in field.streams.iter().enumerate() {
        if st.kind != StreamKind::Tributary {
            continue;
        }
        for road in &roads {
            for rw in road.points.windows(2) {
                for sw in st.points.windows(2) {
                    let Some((_, p)) = segments_cross(rw[0], rw[1], sw[0], sw[1]) else {
                        continue;
                    };
                    if stream.unit() >= rules.streams.footbridge_chance {
                        continue;
                    }
                    let (dx, dz) = (rw[1].0 - rw[0].0, rw[1].1 - rw[0].1);
                    let len = dx.hypot(dz).max(1e-6);
                    let mut bridge = Bridge {
                        x: p.0,
                        z: p.1,
                        dir: (dx / len, dz / len),
                        length: st.width + 6.0,
                        width: rr.main_width_m.min(road.width + 1.0),
                        span: st.width,
                        deck: 0.0,
                        stone: false,
                        arches: 0,
                        stream: Some(si),
                    };
                    bridge.deck = deck_height(field, &bridge);
                    footbridges.push(bridge);
                }
            }
        }
    }
    field.bridges.extend(footbridges);
    field.roads = roads;
}

// ----- queries ---------------------------------------------------------------------------

impl Battlefield {
    /// Does the straight path a-b run through deep water?
    pub fn path_wet(&self, a: (f64, f64), b: (f64, f64)) -> bool {
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let n = ((len / 5.0).ceil() as usize).max(1);
        (0..=n).any(|k| {
            let t = k as f64 / n as f64;
            let (x, z) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            self.water_kind(x, z).is_some_and(Water::deep)
        })
    }

    /// The bridge whose deck is at (x, z), if any.
    pub fn bridge_at(&self, x: f64, z: f64) -> Option<&Bridge> {
        self.bridges.iter().find(|b| b.on_deck(x, z))
    }

    /// Height at which one walks at (x, z): the deck on a bridge (ramping
    /// down over the abutments), else the ground.
    pub fn walk_height(&self, x: f64, z: f64) -> f64 {
        let ground = self.height(x, z);
        match self.bridge_at(x, z) {
            Some(b) => {
                let (along, _) = b.local(x, z);
                let span = b.span * 0.5;
                let ramp = (b.length * 0.5 - span).max(1e-6);
                let rise = if b.stone {
                    STONE_DECK_RISE
                } else {
                    WOOD_DECK_RISE
                };
                let t = ((along.abs() - span) / ramp).clamp(0.0, 1.0);
                ground.max(b.deck - rise * t)
            }
            None => ground,
        }
    }

    /// EP3: the kind of water at (x, z), `None` on land or on a bridge.
    pub fn water_kind(&self, x: f64, z: f64) -> Option<Water> {
        if let Some(river) = &self.river {
            if river.in_water(x, z) {
                if self.bridge_at(x, z).is_some() {
                    return None;
                }
                return Some(if river.in_ford(x) {
                    Water::Ford
                } else {
                    Water::Deep
                });
            }
        }
        if self.oxbows.iter().any(|o| o.contains(x, z)) {
            return Some(Water::Oxbow);
        }
        if let Some(s) = self.streams.iter().find(|s| s.in_water(x, z)) {
            if self.bridge_at(x, z).is_some() {
                return None;
            }
            return Some(Water::Stream(s.kind));
        }
        self.pools
            .iter()
            .any(|p| p.contains(x, z))
            .then_some(Water::Pool)
    }

    /// The bank at (x, z): within the band of a steep or marshy stretch,
    /// out of the water, away from fords and bridges.
    pub fn bank_kind(&self, x: f64, z: f64) -> Option<BankKind> {
        let river = self.river.as_ref()?;
        if river.banks.is_empty() || river.in_ford(x) || river.in_water(x, z) {
            return None;
        }
        let band = WaterRules::bundled().movement.bank_band_m;
        let d = (z - river.center_z(x)).abs() - river.width_at(x) * 0.5;
        if d > band || self.bridge_at(x, z).is_some() || river.near_bridge(x) {
            return None;
        }
        river.bank_at(x, river.north_of(x, z))
    }

    /// EP3: speed multiplier of water and banks at (x, z).
    pub fn water_speed_factor(&self, x: f64, z: f64, mounted: bool) -> f64 {
        let m = &WaterRules::bundled().movement;
        let water = match self.water_kind(x, z) {
            Some(Water::Deep) => {
                if mounted {
                    m.deep_mounted
                } else {
                    m.deep_foot
                }
            }
            Some(Water::Ford) | Some(Water::Pool) => m.ford,
            Some(Water::Oxbow) => m.oxbow,
            Some(Water::Stream(StreamKind::Tributary)) => m.tributary,
            Some(Water::Stream(StreamKind::Brook)) => m.brook,
            None => 1.0,
        };
        let bank = match self.bank_kind(x, z) {
            Some(BankKind::Steep) => m.steep_bank,
            Some(BankKind::Marsh) => m.marsh_bank,
            None => 1.0,
        };
        water * bank
    }

    /// EP3: the road at (x, z), if any (main roads first).
    pub fn road_at(&self, x: f64, z: f64) -> Option<RoadKind> {
        let mut found = None;
        for road in &self.roads {
            if road.on_road(x, z) {
                if road.kind == RoadKind::Main {
                    return Some(RoadKind::Main);
                }
                found = Some(road.kind);
            }
        }
        found
    }

    /// EP3: the crossings of the main river (bridges, then fords, west to
    /// east within each kind).
    pub fn crossings(&self) -> Vec<Crossing> {
        let Some(river) = &self.river else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for (i, b) in self.bridges.iter().enumerate() {
            if b.stream.is_some() {
                continue;
            }
            let [a, e] = b.ends();
            let (south, north) = if a.1 <= e.1 { (a, e) } else { (e, a) };
            result.push(Crossing {
                bridge: Some(i),
                ford: None,
                south,
                north,
                width: b.width,
            });
        }
        for (i, f) in river.fords.iter().enumerate() {
            let c = river.center_z(f.x);
            let half = river.width_at(f.x) * 0.5 + 4.0;
            result.push(Crossing {
                bridge: None,
                ford: Some(i),
                south: (f.x, c - half),
                north: (f.x, c + half),
                width: f.half_width * 2.0,
            });
        }
        result
    }

    /// EP6: a spot on the bank of the main river (or of a stream) near
    /// `near`, `setback` metres from the water's edge, clear of fords,
    /// bridges and roads: where a water mill (or a wash house, a tannery)
    /// stands. `None` without water.
    pub fn waterside_spot(&self, near: (f64, f64), setback: f64) -> Option<WatersideSpot> {
        let mut best: Option<(WatersideSpot, f64)> = None;
        let mut consider = |spot: WatersideSpot| {
            if !self.inside(spot.x, spot.z)
                || self.water_kind(spot.x, spot.z).is_some()
                || self.road_at(spot.x, spot.z).is_some()
                || self
                    .bridges
                    .iter()
                    .any(|b| (b.x - spot.x).hypot(b.z - spot.z) < b.length * 0.5 + 25.0)
            {
                return;
            }
            let d = (spot.x - near.0).hypot(spot.z - near.1);
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((spot, d));
            }
        };
        if let Some(river) = &self.river {
            let mut x = 20.0;
            while x < self.width - 20.0 {
                if !river.in_ford(x) {
                    let (nx, nz) = river.normal(x);
                    let c = river.center_z(x);
                    let off = river.width_at(x) * 0.5 + setback;
                    for sign in [1.0, -1.0] {
                        consider(WatersideSpot {
                            x: x + nx * off * sign,
                            z: c + nz * off * sign,
                            towards_water: (-nx * sign, -nz * sign),
                            stream: None,
                        });
                    }
                }
                x += 10.0;
            }
        }
        for (si, st) in self.streams.iter().enumerate() {
            for w in st.points.windows(2) {
                let (dx, dz) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                let len = dx.hypot(dz).max(1e-6);
                let n = (-dz / len, dx / len);
                let off = st.width * 0.5 + setback;
                for sign in [1.0, -1.0] {
                    consider(WatersideSpot {
                        x: w[0].0 + n.0 * off * sign,
                        z: w[0].1 + n.1 * off * sign,
                        towards_water: (-n.0 * sign, -n.1 * sign),
                        stream: Some(si),
                    });
                }
            }
        }
        best.map(|(spot, _)| spot)
    }
}
