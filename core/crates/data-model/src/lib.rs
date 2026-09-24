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
mod event_check;
pub mod ids;
pub mod load;
pub mod map;
pub mod settlement_load;

pub use common::{
    Cost, Effect, EffectKind, EffectMode, HistoricalDate, LocalizedName, Percent, SocialClass,
    Sources, UncertainInteger, UnitCategory,
};
pub use entities::battle_order::{
    BattleOrder, BattleOrderAi, BattleOrderEffects, BattleOrderFilter, BattleOrderKind,
    BattleOrderScope,
};
pub use entities::building::{Building, BuildingCategory};
pub use entities::character::{Character, CharacterStatus, Family, Role, Sex, Skills, Title};
pub use entities::chivalric_order::ChivalricOrder;
pub use entities::diet::{Diet, DietRequirements, LentRule, WinterRule};
pub use entities::event::{
    CharacterRef, Condition, Event, EventCategory, EventDate, EventEffect, EventOption, EventScope,
    EventSeason, EventTrigger, ProvinceRef,
};
pub use entities::faction::{
    AiPersonality, ClaimData, ClaimKind, Faction, Government, Heraldry, Objective,
    ObjectiveCondition, Relation, RelationStatus, SuccessionLaw, VictoryConditions,
};
pub use entities::names::NameList;
pub use entities::province::{
    CapitalCity, Climate, Population, PopulationClass, PopulationClasses, Province, ProvinceGeo,
    Terrain,
};
pub use entities::r#trait::{Trait, TraitCategory};
pub use entities::religion::{Religion, ReligionKind};
pub use entities::resource::{Resource, ResourceCategory};
pub use entities::settlement::{
    FullProvinceBonus, Settlement, SettlementEdge, SettlementGraph, SettlementKind, SettlementRules,
};
pub use entities::skill::{Skill, SkillBranch};
pub use entities::technology::{TechBranch, TechUnlocks, Technology};
pub use entities::unit_type::{Ability, UnitStats, UnitType};
pub use ids::{
    BuildingId, CharacterId, ChivalricOrderId, CultureId, DietId, EventId, FactionId, NamesId,
    ProvinceId, ReligionId, ResourceId, SeaZoneId, SettlementId, SkillId, TechnologyId, TraitId,
    UnitTypeId,
};
pub use load::{DataError, GameData, ReferenceError, Warning};
pub use map::{MapMeta, ProvinceGeometry};
