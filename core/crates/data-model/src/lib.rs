//! Serializable game data types and loading helpers.
//!
//! All game data lives in `data/` as JSON files validated by `data/schemas/`.
//! This crate mirrors those schemas as `serde` structs (every struct uses
//! `deny_unknown_fields`, so drift between schema and code fails loudly), reads
//! them from disk and checks cross-references. No game rules live here.
//!
//! Entry point: [`GameData::load`].

pub mod common;
pub mod entities;
pub mod ids;
pub mod load;
pub mod map;

pub use common::{
    Cost, Effect, EffectKind, EffectMode, HistoricalDate, LocalizedName, Percent, SocialClass,
    Sources, UncertainInteger, UnitCategory,
};
pub use entities::building::{Building, BuildingCategory};
pub use entities::character::{Character, CharacterStatus, Family, Role, Sex, Skills, Title};
pub use entities::faction::{
    AiPersonality, Faction, Government, Heraldry, Relation, RelationStatus, SuccessionLaw,
};
pub use entities::province::{
    CapitalCity, Climate, Population, PopulationClass, PopulationClasses, Province, ProvinceGeo,
    Terrain,
};
pub use entities::religion::{Religion, ReligionKind};
pub use entities::resource::{Resource, ResourceCategory};
pub use entities::technology::{TechBranch, TechUnlocks, Technology};
pub use entities::unit_type::{Ability, UnitStats, UnitType};
pub use ids::{
    BuildingId, CharacterId, CultureId, FactionId, ProvinceId, ReligionId, ResourceId, SeaZoneId,
    TechnologyId, TraitId, UnitTypeId,
};
pub use load::{DataError, GameData, ReferenceError, Warning};
pub use map::{MapMeta, ProvinceGeometry};
