//! Historical and random chronicle events (`event.schema.json`, M10).
//!
//! An event has a trigger (a date for historical events, a per-turn chance
//! for random ones) plus typed conditions, a scope (who decides) and one to
//! three options, each with typed effects and an AI weight. The rules that
//! evaluate conditions and apply effects live in `sim-campaign::chronicle`.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::common::{HistoricalDate, Sources};
use crate::entities::faction::ClaimKind;
use crate::ids::{CharacterId, EventId, FactionId, ProvinceId, ReligionId, TraitId, UnitTypeId};

/// Historical (dated, fires at most once) or random (per-turn chance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Historical,
    Random,
}

/// Season of a trigger date or of a `season` condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSeason {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl EventSeason {
    /// 0 for spring ... 3 for winter (turn order within a year).
    pub fn index(self) -> u32 {
        match self {
            EventSeason::Spring => 0,
            EventSeason::Summer => 1,
            EventSeason::Autumn => 2,
            EventSeason::Winter => 3,
        }
    }
}

/// Earliest date of a historical event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDate {
    pub year: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<EventSeason>,
}

/// When an event may fire.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventTrigger {
    /// Historical: earliest date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<EventDate>,
    /// Historical: last year the event may still fire (default: `date.year + 2`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until_year: Option<i32>,
    /// Random: average number of turns before the event happens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_time_to_happen: Option<u32>,
    /// Random: chance per turn, in thousandths (overrides `mean_time_to_happen`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chance_permille: Option<u32>,
    /// All must hold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

/// A typed condition. Optional `faction` / `province` fields default to the
/// event's scoped faction / province.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    /// The faction is alive.
    FactionExists {
        faction: FactionId,
    },
    /// The faction is the player's.
    FactionIsPlayer {
        faction: FactionId,
    },
    /// `a` (default: scope) is at war with `b` (default: anyone).
    AtWar {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        a: Option<FactionId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        b: Option<FactionId>,
    },
    /// `faction` (default: scope) controls `province`.
    Controls {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        province: ProvinceId,
    },
    CharacterAlive {
        id: CharacterId,
    },
    /// The character is alive and held captive.
    CharacterCaptive {
        id: CharacterId,
    },
    RulerIs {
        faction: FactionId,
        character: CharacterId,
    },
    /// The ruler of `faction` (default: scope) has `trait`.
    RulerTrait {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        #[serde(rename = "trait")]
        trait_id: TraitId,
    },
    /// The ruler of `faction` belongs to `house`.
    RulerHouse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        house: String,
    },
    /// The ruler of `faction` is aged `min..=max`.
    RulerAgeBetween {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        min: i32,
        max: i32,
    },
    YearBetween {
        from: i32,
        to: i32,
    },
    /// Weighted-average unrest of the province (default: scope) above `amount`.
    ProvinceUnrestAbove {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceId>,
        amount: u8,
    },
    /// The province (default: scope) is under siege (by `by`, if given).
    ProvinceBesieged {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        by: Option<FactionId>,
    },
    /// The province (default: scope) is on the coast.
    ProvinceCoastal {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceId>,
    },
    TreasuryAbove {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        amount: i64,
    },
    /// `faction` (default: scope) controls fewer than `count` provinces.
    ProvincesBelow {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        count: u32,
    },
    /// State religion of `faction` (default: scope).
    ReligionIs {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        religion: ReligionId,
    },
    /// The Great Schism is (or is not) in progress.
    Schism {
        #[serde(default = "default_true")]
        active: bool,
    },
    NotFired {
        event: EventId,
    },
    Fired {
        event: EventId,
    },
    Season {
        season: EventSeason,
    },
    /// At least one of `conditions` holds.
    AnyOf {
        conditions: Vec<Condition>,
    },
}

fn default_true() -> bool {
    true
}

/// Who receives the event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventScope {
    /// Everyone: the player decides (or the AI weights if the player is gone).
    Global,
    /// One faction (`faction`), or each faction meeting the conditions.
    Faction {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
    },
    /// One province (`province`), or one drawn among those meeting the
    /// conditions (restricted to `faction`'s provinces if given); its
    /// controller decides.
    Province {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceId>,
    },
}

/// A character reference in an effect: `"ruler"`, `"heir"` (of the target
/// faction) or a `chr_` id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CharacterRef {
    #[default]
    Ruler,
    Heir,
    Id(CharacterId),
}

impl fmt::Display for CharacterRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CharacterRef::Ruler => f.write_str("ruler"),
            CharacterRef::Heir => f.write_str("heir"),
            CharacterRef::Id(id) => write!(f, "{id}"),
        }
    }
}

impl Serialize for CharacterRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CharacterRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        match raw.as_str() {
            "ruler" => Ok(CharacterRef::Ruler),
            "heir" => Ok(CharacterRef::Heir),
            _ => CharacterId::new(raw).map(CharacterRef::Id).map_err(|bad| {
                serde::de::Error::custom(format!(
                    "invalid character reference {bad:?}: expected \"ruler\", \"heir\" or chr_…"
                ))
            }),
        }
    }
}

/// A province target in an effect: `"all"` (every province controlled by
/// the target faction) or a `prov_` id. Absent: the scoped province, else
/// `all`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvinceRef {
    All,
    Id(ProvinceId),
}

impl Serialize for ProvinceRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ProvinceRef::All => serializer.serialize_str("all"),
            ProvinceRef::Id(id) => serializer.serialize_str(id.as_str()),
        }
    }
}

impl<'de> Deserialize<'de> for ProvinceRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        if raw == "all" {
            return Ok(ProvinceRef::All);
        }
        ProvinceId::new(raw).map(ProvinceRef::Id).map_err(|bad| {
            serde::de::Error::custom(format!(
                "invalid province reference {bad:?}: expected \"all\" or prov_…"
            ))
        })
    }
}

/// A typed effect. Optional `faction` fields default to the deciding faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventEffect {
    Treasury {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        amount: i64,
    },
    /// Adds `amount` to the unrest of every class.
    Unrest {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceRef>,
        amount: i32,
    },
    /// Changes every class head count by `percent`.
    Population {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceRef>,
        percent: i32,
    },
    /// Adds `amount` to the health of every class.
    Health {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceRef>,
        amount: i32,
    },
    /// Adds `amount` to the wealth of every class.
    Wealth {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceRef>,
        amount: i32,
    },
    Devastation {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceRef>,
        amount: i32,
    },
    Prestige {
        #[serde(default)]
        character: CharacterRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        amount: i32,
    },
    Piety {
        #[serde(default)]
        character: CharacterRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        amount: i32,
    },
    PapalFavor {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        amount: i32,
    },
    /// `faction`'s opinion of `towards` (default: the deciding faction).
    Opinion {
        faction: FactionId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        towards: Option<FactionId>,
        amount: i32,
        reason: String,
        /// Turns (default 20).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<u32>,
    },
    DeclareWar {
        a: FactionId,
        b: FactionId,
    },
    Peace {
        a: FactionId,
        b: FactionId,
    },
    AddTrait {
        #[serde(default)]
        character: CharacterRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        #[serde(rename = "trait")]
        trait_id: TraitId,
    },
    KillCharacter {
        id: CharacterRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
    },
    /// A new army of `units` for `faction` in `province` (default: scoped
    /// province, else the faction's capital).
    SpawnArmy {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        province: Option<ProvinceId>,
        units: Vec<UnitTypeId>,
    },
    /// `faction` gains a claim: `target` is a `fac_` id (throne) or a
    /// `prov_` id (province).
    Claim {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
        kind: ClaimKind,
        target: String,
    },
    /// Loyalty of `vassal` (default: every vassal of the deciding faction).
    Loyalty {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        vassal: Option<FactionId>,
        amount: i32,
    },
    /// Black Death: a wave that strikes every province once, south first,
    /// over `(to_year - from_year) × 4` turns.
    PlagueWave {
        from_year: i32,
        to_year: i32,
    },
}

/// One choice of an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventOption {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EventEffect>,
    /// Relative weight of the option for the AI (highest wins).
    #[serde(default = "default_ai_weight")]
    pub ai_weight: u32,
}

fn default_ai_weight() -> u32 {
    1
}

/// A chronicle event (`data/events/*.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub id: EventId,
    /// French title.
    pub title: String,
    /// French text in a period tone (2-4 sentences).
    pub text: String,
    pub kind: EventCategory,
    pub trigger: EventTrigger,
    pub scope: EventScope,
    pub options: Vec<EventOption>,
    /// Real date of the historical event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_date: Option<HistoricalDate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
