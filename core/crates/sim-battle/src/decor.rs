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

use std::collections::BTreeMap;
use std::sync::OnceLock;

use data_model::Terrain;
use serde::{Deserialize, Serialize};

use crate::field::Battlefield;
use crate::hydro::{CountSpan, Span};
use crate::rng::BattleRng;
use crate::setup::{BattleSeason, SideId};
use crate::site::{House, HouseKind};
use crate::town::Footprint;

/// Salt of the derived stream of the decor.
#[allow(dead_code)] // EP6 skeleton
pub(crate) const DECOR_STREAM: u64 = 0xE6_0D;

// ----- rules -------------------------------------------------------------------

/// A landscape: composition of the decor on the standard field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandscapeProfile {
    pub label: String,
    pub provinces: Vec<String>,
    pub hamlets: CountSpan,
    pub farmsteads: CountSpan,
    pub street_share: f64,
    pub stone_share: f64,
    pub windmill_chance: f64,
    pub watermill_chance: f64,
    pub manor_chance: f64,
    pub moat_chance: f64,
    pub vineyards: CountSpan,
    pub orchards: CountSpan,
    pub ploughland: CountSpan,
    pub meadows: CountSpan,
    pub hedgerows: bool,
}

/// Weights of the states of the ploughland.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldStates {
    pub ploughed: f64,
    pub sown: f64,
    pub crop: f64,
    pub stubble: f64,
}

/// What a season puts on the fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonDecor {
    pub haystacks_per_meadow: CountSpan,
    pub carts: CountSpan,
    pub field_states: FieldStates,
    pub vines_leafy: bool,
    pub orchard_blossom: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonTable {
    pub spring: SeasonDecor,
    pub summer: SeasonDecor,
    pub autumn: SeasonDecor,
    pub winter: SeasonDecor,
}

impl SeasonTable {
    pub fn of(&self, season: BattleSeason) -> &SeasonDecor {
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
pub struct AreaEffect {
    /// Multiplier on missile casualties.
    pub cover: f64,
    pub foot_speed: f64,
    pub horse_speed: f64,
    /// Divisor of the melee casualties of the regiment standing in it.
    pub defense: f64,
    /// A cavalry charge against a regiment standing in it breaks.
    pub breaks_charge: bool,
    /// Extra speed multiplier in the rain or on soaked ground.
    #[serde(default = "one")]
    pub wet_speed: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectTable {
    pub hamlet: AreaEffect,
    pub church: AreaEffect,
    pub manor: AreaEffect,
    pub farmstead: AreaEffect,
    pub orchard: AreaEffect,
    pub vineyard: AreaEffect,
    pub ploughland: AreaEffect,
    pub meadow: AreaEffect,
    pub camp: AreaEffect,
}

impl EffectTable {
    pub fn of(&self, kind: AreaKind) -> &AreaEffect {
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
pub struct PlotSizes {
    pub vineyard: Span,
    pub orchard: Span,
    pub ploughland: Span,
    pub meadow: Span,
}

/// Footprints of the props (length × depth, metres; the kit models').
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropSizes {
    pub haystack: Span,
    pub cart: Span,
    pub tent: Span,
    pub pavilion: Span,
    pub wagon: Span,
    pub campfire: Span,
    pub graves: Span,
    pub well: Span,
    pub woodpile: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementRules {
    pub road_margin_m: f64,
    pub water_margin_m: f64,
    pub bridge_margin_m: f64,
    pub house_gap_m: f64,
    pub street_spacing_m: Span,
    pub hamlet_houses: CountSpan,
    pub farmstead_houses: CountSpan,
    pub plot_size_m: PlotSizes,
    pub prop_size_m: PropSizes,
    pub mound_radius_m: f64,
    pub mound_height_m: f64,
    pub churchyard_margin_m: f64,
    pub moat_width_m: f64,
    pub moat_margin_m: f64,
}

/// The camp and baggage train of an army, and the looting of the camp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampRules {
    pub width_m: f64,
    pub depth_m: f64,
    pub setback_m: f64,
    pub tents: CountSpan,
    pub pavilions: CountSpan,
    pub wagons: CountSpan,
    pub fires: CountSpan,
    pub horse_lines: CountSpan,
    pub horses_per_line: CountSpan,
    pub convoy_wagons: CountSpan,
    pub loot_seconds: f64,
    pub guard_radius_m: f64,
    pub decay_per_second: f64,
    pub looted_morale: f64,
    pub alarm_morale: f64,
    pub looter_fatigue: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainProfiles {
    pub plains: String,
    pub heath: String,
    pub bocage: String,
    pub forest: String,
    pub hills: String,
    pub mountains: String,
    pub marsh: String,
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
        }
    }
}

/// Contents of `data/rules/battle_decor.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecorRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub default_profile: String,
    pub terrain_profiles: TerrainProfiles,
    pub profiles: BTreeMap<String, LandscapeProfile>,
    pub seasons: SeasonTable,
    pub effects: EffectTable,
    pub placement: PlacementRules,
    pub camp: CampRules,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_decor.json");

impl DecorRules {
    /// `data/rules/battle_decor.json` as compiled into the crate.
    pub fn bundled() -> &'static DecorRules {
        static RULES: OnceLock<DecorRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_decor.json is valid")
        })
    }

    /// Key of the landscape of `province` (else of `terrain`).
    pub fn profile_key(&self, province: &str, terrain: Terrain) -> &str {
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
    pub fn profile(&self, province: &str, terrain: Terrain) -> &LandscapeProfile {
        let key = self.profile_key(province, terrain);
        self.profiles
            .get(key)
            .or_else(|| self.profiles.values().next())
            .expect("at least one landscape profile")
    }
}

// ----- features ----------------------------------------------------------------------

/// Kind of a decor area (its effect on the regiments).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AreaKind {
    /// Houses, crofts and lanes of a hamlet.
    Hamlet,
    /// Walled churchyard round the parish church.
    Church,
    /// Manor house or tower house with its yard (and moat).
    Manor,
    /// Lone farm round its yard.
    Farmstead,
    Orchard,
    Vineyard,
    Ploughland,
    Meadow,
    /// An army's camp.
    Camp,
}

impl AreaKind {
    pub fn key(self) -> &'static str {
        match self {
            AreaKind::Hamlet => "hamlet",
            AreaKind::Church => "church",
            AreaKind::Manor => "manor",
            AreaKind::Farmstead => "farmstead",
            AreaKind::Orchard => "orchard",
            AreaKind::Vineyard => "vineyard",
            AreaKind::Ploughland => "ploughland",
            AreaKind::Meadow => "meadow",
            AreaKind::Camp => "camp",
        }
    }

    pub fn label_fr(self) -> &'static str {
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
    pub fn rank(self) -> u8 {
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

/// State of a field of ploughland.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldState {
    /// Bare furrows.
    Ploughed,
    /// Young green corn.
    Sown,
    /// Ripe standing corn.
    Crop,
    Stubble,
}

impl FieldState {
    pub fn key(self) -> &'static str {
        match self {
            FieldState::Ploughed => "ploughed",
            FieldState::Sown => "sown",
            FieldState::Crop => "crop",
            FieldState::Stubble => "stubble",
        }
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
    pub fn footprint(&self) -> Footprint {
        Footprint::new(self.x, self.z, self.length, self.width, self.yaw)
    }

    pub fn contains(&self, x: f64, z: f64) -> bool {
        self.footprint().contains(x, z, 0.0)
    }
}

/// Plan of a hamlet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HamletLayout {
    /// Houses lining both sides of a road (street village, bastide).
    Street,
    /// Houses round a green and the parish church.
    Green,
    /// A lone farm: farmhouse, barn and byre round a yard.
    Farmstead,
}

impl HamletLayout {
    pub fn key(self) -> &'static str {
        match self {
            HamletLayout::Street => "street",
            HamletLayout::Green => "green",
            HamletLayout::Farmstead => "farmstead",
        }
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

/// Kind of a decor prop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecorPropKind {
    Haystack,
    Cart,
    Tent,
    Pavilion,
    Wagon,
    Campfire,
    /// A line of horses tethered to a rope between two posts (`length`).
    HorseLine,
    Graves,
    Well,
    Woodpile,
}

impl DecorPropKind {
    pub fn key(self) -> &'static str {
        match self {
            DecorPropKind::Haystack => "haystack",
            DecorPropKind::Cart => "cart",
            DecorPropKind::Tent => "tent",
            DecorPropKind::Pavilion => "pavilion",
            DecorPropKind::Wagon => "wagon",
            DecorPropKind::Campfire => "campfire",
            DecorPropKind::HorseLine => "horse_line",
            DecorPropKind::Graves => "graves",
            DecorPropKind::Well => "well",
            DecorPropKind::Woodpile => "woodpile",
        }
    }

    /// Figures walk round it (not through it).
    pub fn solid(self) -> bool {
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
    pub fn is_empty(&self) -> bool {
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
    pub description: Option<String>,
    #[serde(default)]
    pub clear: bool,
    #[serde(default)]
    pub items: Vec<DecorItem>,
}

impl Battlefield {
    /// Lays the procedural decor of `province` (EP6), from a stream derived
    /// from `rng` (not advanced).
    pub fn lay_decor(&mut self, _province: &str, _rng: &BattleRng) {}

    /// Removes the whole decor (camps included).
    pub fn clear_decor(&mut self) {
        self.decor = Decor::default();
    }

    /// Applies a hand-made decor plan.
    pub fn apply_decor_plan(&mut self, _plan: &DecorPlan) {}

    /// Decor area that counts at (x, z) (highest rank), camps included.
    pub fn decor_area_at(&self, _x: f64, _z: f64) -> Option<Area> {
        None
    }
}
