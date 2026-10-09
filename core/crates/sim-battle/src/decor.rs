//! Countryside of the battlefield (lot EP6): hamlets laid along a road or
//! round their church, lone farmsteads, a windmill on its mound, a water mill
//! on the river or a brook, a parish church in its walled churchyard, a
//! manor behind its moat, vineyards, orchards, ploughland, meadows with
//! haystacks, carts, bocage hedgerows, and behind each army its camp and
//! baggage train.
//!
//! - **Composition by region and season** ([`DecorRules`], data
//!   `data/rules/battle_decor.json`): the province picks a landscape profile
//!   (vineyards of Guyenne, Burgundy and Champagne, hedgerows of Normandy and
//!   Brittany, moated manors of England, street villages of the Midi…), the
//!   season sets the haystacks, the carts and the state of the fields.
//! - **Rules** ([`AreaEffect`]): a regiment whose centre stands in a hamlet,
//!   a churchyard, a manor, an orchard, a vineyard, ploughland or a camp has
//!   cover against missiles, marches slower, defends better in melee and may
//!   break the charges against it. Buildings and the solid camp furniture
//!   push the figures out like the village houses (BR3).
//! - **Camps** ([`Camp`]): each army leaves its tents, wagons, fires and
//!   horse lines behind its deployment zone; an enemy holding an unguarded
//!   camp loots it (`sim/camp.rs`), which costs the owner morale, as at
//!   Agincourt.
//! - **Hand placement** (EP7, historical maps): [`Battlefield::place_building`],
//!   [`Battlefield::place_hamlet`], [`Battlefield::place_plot`],
//!   [`Battlefield::place_camp`]… and a serde [`DecorPlan`] applied by
//!   [`Battlefield::apply_decor_plan`] (schema
//!   `data/schemas/battle_decor_plan.schema.json`).
//!
//! Everything procedural is drawn from a derived stream ([`DECOR_STREAM`]):
//! the draws of the older battles are unchanged. Nothing is laid in deep
//! water, on a bridge, a ford or a road (the roads of EP3 run through the
//! street hamlets), and every size is read from the field ([`FieldSize`]),
//! never assumed to be 1200 × 800.

use data_model::key_enum;
use data_model::Terrain;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::decor_gen::Layout;
use crate::field::Battlefield;
use crate::hydro::{CountSpan, Span};
use crate::rng::BattleRng;
use crate::setup::{BattleSeason, SideId};
use crate::site::{House, HouseKind, Obstacle, ObstacleKind};
use crate::town::Footprint;

/// Salt of the derived stream of the decor.
pub(crate) const DECOR_STREAM: u64 = 0xE6_0D;

// ----- rules -------------------------------------------------------------------

/// A landscape: composition of the decor on the standard field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandscapeProfile {
    pub(crate) label: String,
    pub(crate) provinces: Vec<String>,
    pub(crate) hamlets: CountSpan,
    pub(crate) farmsteads: CountSpan,
    pub(crate) street_share: f64,
    pub(crate) stone_share: f64,
    pub(crate) windmill_chance: f64,
    pub(crate) watermill_chance: f64,
    pub(crate) manor_chance: f64,
    pub(crate) moat_chance: f64,
    pub(crate) vineyards: CountSpan,
    pub(crate) orchards: CountSpan,
    pub(crate) ploughland: CountSpan,
    pub(crate) meadows: CountSpan,
    pub(crate) hedgerows: bool,
}

/// Weights of the states of the ploughland.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldStates {
    pub(crate) ploughed: f64,
    pub(crate) sown: f64,
    pub(crate) crop: f64,
    pub(crate) stubble: f64,
}

/// What a season puts on the fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SeasonDecor {
    pub(crate) haystacks_per_meadow: CountSpan,
    pub(crate) carts: CountSpan,
    pub(crate) field_states: FieldStates,
    pub(crate) vines_leafy: bool,
    pub(crate) orchard_blossom: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SeasonTable {
    pub(crate) spring: SeasonDecor,
    pub(crate) summer: SeasonDecor,
    pub(crate) autumn: SeasonDecor,
    pub(crate) winter: SeasonDecor,
}

impl SeasonTable {
    pub(crate) fn of(&self, season: BattleSeason) -> &SeasonDecor {
        match season {
            BattleSeason::Spring => &self.spring,
            BattleSeason::Summer => &self.summer,
            BattleSeason::Autumn => &self.autumn,
            BattleSeason::Winter => &self.winter,
        }
    }
}

fn one() -> f64 {
    1.0
}

/// Effect of a decor area on a regiment whose centre stands in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AreaEffect {
    /// Multiplier on missile casualties.
    pub(crate) cover: f64,
    pub(crate) foot_speed: f64,
    pub(crate) horse_speed: f64,
    /// Divisor of the melee casualties of the regiment standing in it.
    pub(crate) defense: f64,
    /// A cavalry charge against a regiment standing in it breaks.
    pub(crate) breaks_charge: bool,
    /// Extra speed multiplier in the rain or on soaked ground.
    #[serde(default = "one")]
    pub(crate) wet_speed: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectTable {
    pub(crate) hamlet: AreaEffect,
    pub(crate) church: AreaEffect,
    pub(crate) manor: AreaEffect,
    pub(crate) farmstead: AreaEffect,
    pub(crate) orchard: AreaEffect,
    pub(crate) vineyard: AreaEffect,
    pub(crate) ploughland: AreaEffect,
    pub(crate) meadow: AreaEffect,
    pub(crate) camp: AreaEffect,
}

impl EffectTable {
    pub(crate) fn of(&self, kind: AreaKind) -> &AreaEffect {
        match kind {
            AreaKind::Hamlet => &self.hamlet,
            AreaKind::Church => &self.church,
            AreaKind::Manor => &self.manor,
            AreaKind::Farmstead => &self.farmstead,
            AreaKind::Orchard => &self.orchard,
            AreaKind::Vineyard => &self.vineyard,
            AreaKind::Ploughland => &self.ploughland,
            AreaKind::Meadow => &self.meadow,
            AreaKind::Camp => &self.camp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlotSizes {
    pub(crate) vineyard: Span,
    pub(crate) orchard: Span,
    pub(crate) ploughland: Span,
    pub(crate) meadow: Span,
}

/// Footprints of the props (length × depth, metres; the kit models').
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PropSizes {
    pub(crate) haystack: Span,
    pub(crate) cart: Span,
    pub(crate) tent: Span,
    pub(crate) pavilion: Span,
    pub(crate) wagon: Span,
    pub(crate) campfire: Span,
    pub(crate) graves: Span,
    pub(crate) well: Span,
    pub(crate) woodpile: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlacementRules {
    pub(crate) road_margin_m: f64,
    pub(crate) water_margin_m: f64,
    pub(crate) bridge_margin_m: f64,
    pub(crate) house_gap_m: f64,
    pub(crate) street_spacing_m: Span,
    pub(crate) hamlet_houses: CountSpan,
    pub(crate) farmstead_houses: CountSpan,
    pub(crate) plot_size_m: PlotSizes,
    pub(crate) prop_size_m: PropSizes,
    pub(crate) mound_radius_m: f64,
    pub(crate) mound_height_m: f64,
    pub(crate) churchyard_margin_m: f64,
    pub(crate) moat_width_m: f64,
    pub(crate) moat_margin_m: f64,
}

/// The camp and baggage train of an army, and the looting of the camp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampRules {
    pub(crate) width_m: f64,
    pub(crate) depth_m: f64,
    pub(crate) setback_m: f64,
    pub(crate) tents: CountSpan,
    pub(crate) pavilions: CountSpan,
    pub(crate) wagons: CountSpan,
    pub(crate) fires: CountSpan,
    pub(crate) horse_lines: CountSpan,
    pub(crate) horses_per_line: CountSpan,
    pub(crate) convoy_wagons: CountSpan,
    pub loot_seconds: f64,
    pub(crate) guard_radius_m: f64,
    pub(crate) decay_per_second: f64,
    pub looted_morale: f64,
    pub(crate) alarm_morale: f64,
    pub(crate) looter_fatigue: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainProfiles {
    pub(crate) plains: String,
    pub(crate) heath: String,
    pub(crate) bocage: String,
    pub(crate) forest: String,
    pub(crate) hills: String,
    pub(crate) mountains: String,
    pub(crate) marsh: String,
    /// OM3 (ADR 0116): Pontic steppe and desert fields.
    pub(crate) steppe: String,
    pub(crate) desert: String,
}

impl TerrainProfiles {
    pub fn of(&self, terrain: Terrain) -> &str {
        match terrain {
            Terrain::Plains => &self.plains,
            Terrain::Heath => &self.heath,
            Terrain::Bocage => &self.bocage,
            Terrain::Forest => &self.forest,
            Terrain::Hills => &self.hills,
            Terrain::Mountains => &self.mountains,
            Terrain::Marsh => &self.marsh,
            Terrain::Steppe => &self.steppe,
            Terrain::Desert => &self.desert,
        }
    }
}

/// Contents of `data/rules/battle_decor.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecorRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) default_profile: String,
    pub terrain_profiles: TerrainProfiles,
    pub profiles: BTreeMap<String, LandscapeProfile>,
    pub(crate) seasons: SeasonTable,
    pub(crate) effects: EffectTable,
    pub(crate) placement: PlacementRules,
    pub camp: CampRules,
}

data_model::bundled_rules!(DecorRules, "rules/battle_decor.json");

impl DecorRules {
    /// Key of the landscape of `province` (else of `terrain`).
    pub(crate) fn profile_key(&self, province: &str, terrain: Terrain) -> &str {
        self.profiles
            .iter()
            .find(|(_, p)| p.provinces.iter().any(|id| id == province))
            .map(|(key, _)| key.as_str())
            .unwrap_or_else(|| {
                let key = self.terrain_profiles.of(terrain);
                if self.profiles.contains_key(key) {
                    key
                } else {
                    &self.default_profile
                }
            })
    }

    /// The landscape of `province` (else of `terrain`).
    pub(crate) fn profile(&self, province: &str, terrain: Terrain) -> &LandscapeProfile {
        let key = self.profile_key(province, terrain);
        self.profiles
            .get(key)
            .or_else(|| self.profiles.values().next())
            .expect("at least one landscape profile")
    }
}

// ----- features ----------------------------------------------------------------------

key_enum! {
/// Kind of a decor area (its effect on the regiments).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AreaKind {
    /// Houses, crofts and lanes of a hamlet.
    Hamlet => "hamlet",
    /// Walled churchyard round the parish church.
    Church => "church",
    /// Manor house or tower house with its yard (and moat).
    Manor => "manor",
    /// Lone farm round its yard.
    Farmstead => "farmstead",
    Orchard => "orchard",
    Vineyard => "vineyard",
    Ploughland => "ploughland",
    Meadow => "meadow",
    /// An army's camp.
    Camp => "camp",
}
}

impl AreaKind {
    pub(crate) fn label_fr(self) -> &'static str {
        match self {
            AreaKind::Hamlet => "hameau",
            AreaKind::Church => "cimetière clos",
            AreaKind::Manor => "manoir",
            AreaKind::Farmstead => "ferme",
            AreaKind::Orchard => "verger",
            AreaKind::Vineyard => "vigne",
            AreaKind::Ploughland => "labours",
            AreaKind::Meadow => "pré",
            AreaKind::Camp => "camp",
        }
    }

    /// Where areas overlap, the one with the highest rank counts.
    pub(crate) fn rank(self) -> u8 {
        match self {
            AreaKind::Manor => 9,
            AreaKind::Church => 8,
            AreaKind::Hamlet => 7,
            AreaKind::Farmstead => 6,
            AreaKind::Camp => 5,
            AreaKind::Vineyard => 4,
            AreaKind::Orchard => 3,
            AreaKind::Ploughland => 2,
            AreaKind::Meadow => 1,
        }
    }
}

key_enum! {
/// State of a field of ploughland.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldState {
    /// Bare furrows.
    Ploughed => "ploughed",
    /// Young green corn.
    Sown => "sown",
    /// Ripe standing corn.
    Crop => "crop",
    Stubble => "stubble",
}
}

/// A decor area: an oriented rectangle (same convention as
/// [`Footprint`]: the length runs along `(cos yaw, sin yaw)`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Area {
    pub kind: AreaKind,
    pub x: f64,
    pub z: f64,
    pub length: f64,
    pub width: f64,
    pub yaw: f64,
    /// Ploughland only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<FieldState>,
}

impl Area {
    /// Rules of this kind of area.
    pub(crate) fn effect(&self) -> &'static AreaEffect {
        DecorRules::bundled().effects.of(self.kind)
    }

    pub fn footprint(&self) -> Footprint {
        Footprint::new(self.x, self.z, self.length, self.width, self.yaw)
    }

    pub fn contains(&self, x: f64, z: f64) -> bool {
        self.footprint().contains(x, z, 0.0)
    }
}

key_enum! {
/// Plan of a hamlet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HamletLayout {
    /// Houses lining both sides of a road (street village, bastide).
    Street => "street",
    /// Houses round a green and the parish church.
    Green => "green",
    /// A lone farm: farmhouse, barn and byre round a yard.
    Farmstead => "farmstead",
}
}

/// A hamlet or a farmstead: its plan and its buildings (indices in
/// [`Decor::buildings`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hamlet {
    pub layout: HamletLayout,
    pub x: f64,
    pub z: f64,
    /// Yaw of the street (or of the farmyard).
    pub yaw: f64,
    pub buildings: Vec<usize>,
}

key_enum! {
/// Kind of a decor prop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecorPropKind {
    Haystack => "haystack",
    Cart => "cart",
    Tent => "tent",
    Pavilion => "pavilion",
    Wagon => "wagon",
    Campfire => "campfire",
    /// A line of horses tethered to a rope between two posts (`length`).
    HorseLine => "horse_line",
    Graves => "graves",
    Well => "well",
    Woodpile => "woodpile",
}
}

impl DecorPropKind {
    /// Figures walk round it (not through it).
    pub(crate) fn solid(self) -> bool {
        !matches!(
            self,
            DecorPropKind::Campfire | DecorPropKind::HorseLine | DecorPropKind::Graves
        )
    }
}

/// A prop of the decor (haystack, cart, tent…): footprint and yaw.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DecorProp {
    pub kind: DecorPropKind,
    pub x: f64,
    pub z: f64,
    pub yaw: f64,
    pub length: f64,
    pub depth: f64,
    /// Horse lines: horses along the rope.
    #[serde(default)]
    pub count: u32,
}

impl DecorProp {
    pub fn footprint(&self) -> Footprint {
        Footprint::new(self.x, self.z, self.length, self.depth, self.yaw)
    }
}

/// Mound raised under a windmill.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Mound {
    pub x: f64,
    pub z: f64,
    pub radius: f64,
    pub height: f64,
}

/// Water-filled moat round a manor: a ring `ring` metres wide inside the
/// outer rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Moat {
    pub x: f64,
    pub z: f64,
    pub length: f64,
    pub width: f64,
    pub yaw: f64,
    pub ring: f64,
}

/// An army's camp: its area, tents, pavilions, wagons, fires and horse
/// lines, and the baggage train parked along the road behind it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camp {
    pub side: SideId,
    pub area: Area,
    pub items: Vec<DecorProp>,
    pub convoy: Vec<DecorProp>,
}

/// The decor of the field.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Decor {
    /// Key of the landscape profile ("" when laid by hand only).
    #[serde(default)]
    pub profile: String,
    pub buildings: Vec<House>,
    pub hamlets: Vec<Hamlet>,
    pub areas: Vec<Area>,
    pub props: Vec<DecorProp>,
    pub mounds: Vec<Mound>,
    pub moats: Vec<Moat>,
    pub camps: Vec<Camp>,
    /// Vines in leaf (else bare winter stocks).
    #[serde(default)]
    pub vines_leafy: bool,
    /// Orchards in blossom.
    #[serde(default)]
    pub orchard_blossom: bool,
}

impl Decor {
    pub(crate) fn is_empty(&self) -> bool {
        self.buildings.is_empty()
            && self.areas.is_empty()
            && self.props.is_empty()
            && self.camps.is_empty()
    }

    /// Camp of `side`, if any.
    pub fn camp(&self, side: SideId) -> Option<&Camp> {
        self.camps.iter().find(|c| c.side == side)
    }
}

// ----- hand placement (EP7) ------------------------------------------------------------

/// One element of a hand-made decor (EP7 historical maps).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecorItem {
    /// A single building (size of the kit model when 0).
    Building {
        kind: HouseKind,
        x: f64,
        z: f64,
        #[serde(default)]
        yaw: f64,
        #[serde(default)]
        length: f64,
        #[serde(default)]
        width: f64,
    },
    /// Post mill, on a mound unless `mound` is false.
    Windmill {
        x: f64,
        z: f64,
        #[serde(default)]
        yaw: f64,
        #[serde(default = "yes")]
        mound: bool,
    },
    /// Water mill whose wheel faces `yaw` + 90° (the model's front).
    Watermill {
        x: f64,
        z: f64,
        #[serde(default)]
        yaw: f64,
    },
    /// Parish church in its walled churchyard.
    Church {
        x: f64,
        z: f64,
        #[serde(default)]
        yaw: f64,
    },
    /// Manor (tower house) with its yard and, with `moat`, a moat.
    Manor {
        x: f64,
        z: f64,
        #[serde(default)]
        yaw: f64,
        #[serde(default = "yes")]
        moat: bool,
    },
    Hamlet {
        layout: HamletLayout,
        x: f64,
        z: f64,
        #[serde(default)]
        yaw: f64,
        #[serde(default)]
        houses: u32,
        #[serde(default)]
        seed: u64,
    },
    Plot {
        kind: AreaKind,
        x: f64,
        z: f64,
        length: f64,
        width: f64,
        #[serde(default)]
        yaw: f64,
        #[serde(default)]
        state: Option<FieldState>,
    },
    Prop {
        kind: DecorPropKind,
        x: f64,
        z: f64,
        #[serde(default)]
        yaw: f64,
    },
    Camp {
        side: SideId,
        x: f64,
        z: f64,
        #[serde(default)]
        yaw: f64,
        #[serde(default)]
        seed: u64,
    },
    /// A hedgerow (B5 obstacle) from `a` to `b`.
    Hedge { a: (f64, f64), b: (f64, f64) },
}

fn yes() -> bool {
    true
}

/// A hand-made decor: with `clear`, the procedural decor is removed first
/// (camps included when the plan places its own).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecorPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) clear: bool,
    #[serde(default)]
    pub(crate) items: Vec<DecorItem>,
}

impl Battlefield {
    /// Runs `f` on a layout of this field's decor and writes the result
    /// back (decor, farm tracks, hedges, windmill mounds).
    fn with_layout<R>(&mut self, seed: u64, f: impl FnOnce(&mut Layout) -> R) -> R {
        let mounds_before = self.decor.mounds.len();
        let (result, decor, tracks, hedges) = {
            let mut layout = Layout::new(self, BattleRng::from_seed(seed));
            let result = f(&mut layout);
            let (decor, tracks, hedges) = layout.finish();
            (result, decor, tracks, hedges)
        };
        self.decor = decor;
        self.roads.extend(tracks);
        self.obstacles.extend(hedges);
        let new_mounds = self.decor.mounds[mounds_before..].to_vec();
        crate::decor_gen::raise_mounds(self, &new_mounds);
        result
    }

    /// Lays the procedural decor of `province` (EP6), from a stream derived
    /// from `rng` (not advanced): camps, hamlets, farmsteads, manor,
    /// windmill, water mill, vineyards, orchards, meadows, ploughland,
    /// carts, bocage hedges.
    pub fn lay_decor(&mut self, province: &str, rng: &BattleRng) {
        let rules = DecorRules::bundled();
        let key = rules.profile_key(province, self.terrain).to_owned();
        let seed = rng.derive(DECOR_STREAM).next_u64();
        self.with_layout(seed, |layout| layout.lay(&key));
    }

    /// Lays only the two camps (a bare field: `BattleSetup::village` set to
    /// `Some(false)`), from a stream derived from `rng` (not advanced).
    pub(crate) fn lay_camps(&mut self, rng: &BattleRng) {
        let seed = rng.derive(DECOR_STREAM).next_u64();
        self.with_layout(seed, |layout| {
            for side in SideId::BOTH {
                layout.camp(side, None);
            }
        });
    }

    /// Removes the whole decor (camps included). Farm tracks, hedges and
    /// mounds already laid stay.
    pub fn clear_decor(&mut self) {
        self.decor = Decor::default();
    }

    /// Places one building as given (hand placement: no check). A zero
    /// size takes the usual size of the kind. Its index.
    pub(crate) fn place_building(
        &mut self,
        kind: HouseKind,
        x: f64,
        z: f64,
        yaw: f64,
        length: f64,
        width: f64,
    ) -> usize {
        let (l, w) = match kind {
            HouseKind::Church => (19.0, 8.0),
            HouseKind::Barn => (15.0, 7.5),
            HouseKind::Windmill => (7.0, 7.0),
            HouseKind::Watermill => (11.0, 7.0),
            HouseKind::Manor => (20.5, 8.0),
            _ => (10.0, 6.0),
        };
        self.decor.buildings.push(House {
            x,
            z,
            length: if length > 0.0 { length } else { l },
            width: if width > 0.0 { width } else { w },
            yaw,
            kind,
        });
        self.decor.buildings.len() - 1
    }

    /// Places a post mill at (x, z), on a mound when `mound`.
    pub(crate) fn place_windmill(&mut self, x: f64, z: f64, yaw: f64, mound: bool) {
        self.with_layout(0, |l| l.windmill(Some((x, z, yaw)), Some(mound)));
    }

    /// Places a water mill whose front (wheel) faces `(-sin yaw, cos yaw)`.
    pub(crate) fn place_watermill(&mut self, x: f64, z: f64, yaw: f64) {
        self.with_layout(0, |l| l.watermill(Some((x, z, yaw))));
    }

    /// Places a parish church in its walled churchyard.
    pub(crate) fn place_church(&mut self, x: f64, z: f64, yaw: f64) {
        self.with_layout(0, |l| l.church(x, z, yaw));
    }

    /// Places a manor with its yard (and moat).
    pub fn place_manor(&mut self, x: f64, z: f64, yaw: f64, moat: bool) {
        self.with_layout(0, |l| l.manor(Some((x, z, yaw)), moat));
    }

    /// Places a hamlet of `layout` at (x, z) with about `houses` houses
    /// (0: the usual count), laid round the roads, the water and what is
    /// already there; a street hamlet follows the road nearest (x, z).
    /// `false` when nothing fits.
    pub(crate) fn place_hamlet(
        &mut self,
        layout: HamletLayout,
        x: f64,
        z: f64,
        yaw: f64,
        houses: u32,
        seed: u64,
    ) -> bool {
        let rules = DecorRules::bundled();
        let profile = rules.profile(&self.decor.profile, self.terrain).clone();
        self.with_layout(seed, |l| {
            let houses = if houses > 0 {
                houses
            } else {
                match layout {
                    HamletLayout::Farmstead => rules.placement.farmstead_houses[1],
                    _ => rules.placement.hamlet_houses[1],
                }
            };
            match layout {
                HamletLayout::Street => {
                    let spot = l.road_near((x, z));
                    spot.is_some() && l.street_hamlet(&profile, spot, houses)
                }
                HamletLayout::Green => l.green_hamlet(&profile, Some((x, z, yaw)), houses),
                HamletLayout::Farmstead => l.farmstead(&profile, Some((x, z, yaw)), houses),
            }
        })
    }

    /// Places a plot (orchard, vineyard, ploughland, meadow — or any area
    /// kind) as given; meadows get the haystacks of the season.
    #[allow(clippy::too_many_arguments)]
    pub fn place_plot(
        &mut self,
        kind: AreaKind,
        x: f64,
        z: f64,
        length: f64,
        width: f64,
        yaw: f64,
        state: Option<FieldState>,
    ) {
        let fp = Footprint::new(x, z, length, width, yaw);
        let state = state.or((kind == AreaKind::Ploughland).then_some(FieldState::Ploughed));
        self.with_layout(0, |l| l.lay_plot(kind, fp, state));
    }

    /// Places a prop (haystack, cart, tent…) at its usual size.
    pub(crate) fn place_prop(&mut self, kind: DecorPropKind, x: f64, z: f64, yaw: f64) {
        let s = &DecorRules::bundled().placement.prop_size_m;
        let size = match kind {
            DecorPropKind::Haystack => s.haystack,
            DecorPropKind::Cart => s.cart,
            DecorPropKind::Tent => s.tent,
            DecorPropKind::Pavilion => s.pavilion,
            DecorPropKind::Wagon => s.wagon,
            DecorPropKind::Campfire => s.campfire,
            DecorPropKind::Graves => s.graves,
            DecorPropKind::Well => s.well,
            DecorPropKind::Woodpile => s.woodpile,
            DecorPropKind::HorseLine => [12.0, 3.0],
        };
        self.decor.props.push(DecorProp {
            kind,
            x,
            z,
            yaw,
            length: size[0],
            depth: size[1],
            count: if kind == DecorPropKind::HorseLine {
                6
            } else {
                0
            },
        });
    }

    /// Places the camp of `side` centred on (x, z) (front towards
    /// `(-sin yaw, cos yaw)`, the enemy), replacing its current camp.
    pub(crate) fn place_camp(&mut self, side: SideId, x: f64, z: f64, yaw: f64, seed: u64) -> bool {
        self.decor.camps.retain(|c| c.side != side);
        self.with_layout(seed, |l| l.camp(side, Some((x, z, yaw))))
    }

    /// Applies a hand-made decor plan (EP7). With `clear`, the procedural
    /// decor goes first; its camps stay unless the plan places its own.
    pub(crate) fn apply_decor_plan(&mut self, plan: &DecorPlan) {
        if plan.clear {
            let camps = std::mem::take(&mut self.decor.camps);
            let profile = std::mem::take(&mut self.decor.profile);
            let (leafy, blossom) = (self.decor.vines_leafy, self.decor.orchard_blossom);
            self.decor = Decor {
                profile,
                vines_leafy: leafy,
                orchard_blossom: blossom,
                ..Decor::default()
            };
            if !plan
                .items
                .iter()
                .any(|i| matches!(i, DecorItem::Camp { .. }))
            {
                self.decor.camps = camps;
            }
        }
        for item in &plan.items {
            match *item {
                DecorItem::Building {
                    kind,
                    x,
                    z,
                    yaw,
                    length,
                    width,
                } => {
                    self.place_building(kind, x, z, yaw, length, width);
                }
                DecorItem::Windmill { x, z, yaw, mound } => self.place_windmill(x, z, yaw, mound),
                DecorItem::Watermill { x, z, yaw } => self.place_watermill(x, z, yaw),
                DecorItem::Church { x, z, yaw } => self.place_church(x, z, yaw),
                DecorItem::Manor { x, z, yaw, moat } => self.place_manor(x, z, yaw, moat),
                DecorItem::Hamlet {
                    layout,
                    x,
                    z,
                    yaw,
                    houses,
                    seed,
                } => {
                    self.place_hamlet(layout, x, z, yaw, houses, seed);
                }
                DecorItem::Plot {
                    kind,
                    x,
                    z,
                    length,
                    width,
                    yaw,
                    state,
                } => self.place_plot(kind, x, z, length, width, yaw, state),
                DecorItem::Prop { kind, x, z, yaw } => self.place_prop(kind, x, z, yaw),
                DecorItem::Camp {
                    side,
                    x,
                    z,
                    yaw,
                    seed,
                } => {
                    self.place_camp(side, x, z, yaw, seed);
                }
                DecorItem::Hedge { a, b } => self.obstacles.push(Obstacle {
                    a,
                    b,
                    kind: ObstacleKind::Hedge,
                }),
            }
        }
    }

    // ----- rules of the decor -----------------------------------------------------

    /// Decor area that counts at (x, z) (highest rank), camps included.
    pub fn decor_area_at(&self, x: f64, z: f64) -> Option<Area> {
        self.decor
            .areas
            .iter()
            .chain(self.decor.camps.iter().map(|c| &c.area))
            .filter(|a| {
                let reach = (a.length + a.width) * 0.5;
                (a.x - x).abs() <= reach && (a.z - z).abs() <= reach && a.contains(x, z)
            })
            .max_by_key(|a| a.kind.rank())
            .copied()
    }

    /// Effect of the decor at (x, z), if any.
    pub(crate) fn decor_effect_at(&self, x: f64, z: f64) -> Option<&'static AreaEffect> {
        self.decor_area_at(x, z).map(|a| a.effect())
    }

    /// Speed multiplier of the decor at (x, z); `wet`: rain or soaked
    /// ground (muddy furrows).
    pub fn decor_speed_factor(&self, x: f64, z: f64, mounted: bool, wet: bool) -> f64 {
        self.decor_effect_at(x, z).map_or(1.0, |e| {
            let base = if mounted { e.horse_speed } else { e.foot_speed };
            if wet {
                base * e.wet_speed
            } else {
                base
            }
        })
    }

    /// Multiplier on missile casualties of a regiment at (x, z).
    pub fn decor_cover(&self, x: f64, z: f64) -> f64 {
        self.decor_effect_at(x, z).map_or(1.0, |e| e.cover)
    }

    /// Divisor of the melee casualties of a regiment at (x, z).
    pub fn decor_defense(&self, x: f64, z: f64) -> f64 {
        self.decor_effect_at(x, z).map_or(1.0, |e| e.defense)
    }

    /// A charge against a regiment at (x, z) breaks: where, in French
    /// (« dans le hameau », « dans les vignes »…).
    pub fn decor_breaks_charge(&self, x: f64, z: f64) -> Option<&'static str> {
        let area = self.decor_area_at(x, z)?;
        if !area.effect().breaks_charge {
            return None;
        }
        Some(match area.kind {
            AreaKind::Hamlet => "dans le hameau",
            AreaKind::Church => "contre le mur du cimetière",
            AreaKind::Manor => "contre le fossé du manoir",
            AreaKind::Farmstead => "dans la cour de la ferme",
            AreaKind::Orchard => "dans le verger",
            AreaKind::Vineyard => "dans les vignes",
            AreaKind::Camp => "dans le camp",
            AreaKind::Ploughland | AreaKind::Meadow => "dans les champs",
        })
    }

    /// QW-G1: what the cursor shows over the ground at (x, z): the kind of
    /// area and its rule values as signed percentages (cover = fewer missile
    /// casualties, speed = change of pace, defense = fewer melee casualties).
    pub fn decor_hover_at(&self, x: f64, z: f64) -> Option<DecorHover> {
        let area = self.decor_area_at(x, z)?;
        let e = area.effect();
        let pct = |factor: f64| ((factor - 1.0) * 100.0).round() as i64;
        Some(DecorHover {
            kind: area.kind,
            label: area.kind.label_fr(),
            cover_pct: -pct(e.cover),
            foot_speed_pct: pct(e.foot_speed),
            horse_speed_pct: pct(e.horse_speed),
            defense_pct: pct(e.defense),
            breaks_charge: e.breaks_charge,
        })
    }

    /// Solid footprints of the decor within `radius` of (x, z): buildings,
    /// solid props, camp furniture and the baggage train (figures walk round
    /// them).
    pub(crate) fn decor_footprints_near(&self, x: f64, z: f64, radius: f64) -> Vec<Footprint> {
        let mut out = Vec::new();
        let mut take = |f: Footprint| {
            if (f.x - x).hypot(f.z - z) < radius + f.bounding_radius() {
                out.push(f);
            }
        };
        for b in &self.decor.buildings {
            take(Footprint::new(b.x, b.z, b.length, b.width, b.yaw));
        }
        for p in self.decor.props.iter().filter(|p| p.kind.solid()) {
            take(p.footprint());
        }
        for c in &self.decor.camps {
            let fp = c.area.footprint();
            if (fp.x - x).hypot(fp.z - z) < radius + fp.bounding_radius() {
                for p in c.items.iter().filter(|p| p.kind.solid()) {
                    take(p.footprint());
                }
            }
            for p in &c.convoy {
                take(p.footprint());
            }
        }
        out
    }
}

/// Rule values of the decor under the cursor (see [`Battlefield::decor_hover_at`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecorHover {
    pub kind: AreaKind,
    pub label: &'static str,
    /// Positive = fewer missile casualties.
    pub cover_pct: i64,
    pub foot_speed_pct: i64,
    pub horse_speed_pct: i64,
    /// Positive = fewer melee casualties.
    pub defense_pct: i64,
    pub breaks_charge: bool,
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<AreaKind>();
        assert_keys_match_serde::<FieldState>();
        assert_keys_match_serde::<HamletLayout>();
        assert_keys_match_serde::<DecorPropKind>();
    }
}
