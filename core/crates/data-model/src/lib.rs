//! Serializable game data types and loading helpers.
//!
//! All game data lives in `data/` as JSON files validated by `data/schemas/`.
//! This crate mirrors those schemas as `serde` structs (every struct uses
//! `deny_unknown_fields`, so drift between schema and code fails loudly), reads
//! them from disk and checks cross-references. No game rules live here.
//!
//! Entry point: [`GameData::load`].

pub mod common;
pub mod cover;
pub mod entities;
mod event_check;
pub mod ids;
pub mod load;
pub mod map;
pub mod movement_graph;
pub mod navgrid;
pub mod settlement_load;
pub mod title_check;
pub mod trade_paths;

pub use common::{
    Cost, Effect, EffectKind, EffectMode, HistoricalDate, LocalizedName, Percent, SocialClass,
    Sources, UncertainInteger, UnitCategory,
};
pub use cover::{CoverClass, CoverMap};
pub use entities::agent::{
    AgentActionKind, AgentActionRules, AgentEffects, AgentKind, AgentRules, AgentTypeRules,
};
pub use entities::ai_alignment::{
    AiAlignment, DefectionRules, DynasticRules, GrievanceRules, MoneyFiefRules, WoolRevoltRules,
};
pub use entities::ai_diplomacy::{
    AiDiplomacy, JoinWarRules, MenacingNeighbourRules, NegotiationRules, PassageRules, PeaceRules,
    WarPlanningRules,
};
pub use entities::ai_doctrine::{AiDoctrines, Doctrine};
pub use entities::ai_grid::{
    AiAmbush, AiEncounters, AiEntrenched, AiForcedMarch, AiGrid, AiPostures,
};
pub use entities::army_traditions::{
    ArmyExperienceRules, ArmyTradition, ArmyTraditionRules, BranchWeights, TraditionAiWeights,
    TraditionBranch, TraditionBranchDef, TraditionEffects,
};
pub use entities::auto_resolve::{
    AutoResolveRules, AutoResolveWeather, TerrainEffects, WeatherChances,
};
pub use entities::battle_ability::{
    AbilityCondition, AbilityKind, BattleAbility, BattleAbilityAi, BattleAbilityEffects,
    BattleAbilityFilter,
};
pub use entities::battle_order::{
    BattleOrder, BattleOrderAi, BattleOrderEffects, BattleOrderFilter, BattleOrderKind,
    BattleOrderScope,
};
pub use entities::battle_outcome::{
    BattleOutcomeClass, BattleOutcomeRules, OutcomeConsequence, OutcomeThresholds,
};
pub use entities::battle_standards::BattleStandardRules;
pub use entities::building::{Building, BuildingCategory};
pub use entities::campaign_weather::{
    CampaignWeatherChances, CampaignWeatherRules, ClimateWeather, SeasonalWeather,
};
pub use entities::capture::{
    CaptureAiRules, CaptureRules, OccupyRules, OutcomeRules, OutcomeScores, RazeRules,
};
pub use entities::character::{Character, CharacterStatus, Family, Role, Sex, Skills, Title};
pub use entities::chivalric_order::ChivalricOrder;
pub use entities::diet::{Diet, DietRequirements, LentRule, WinterRule};
pub use entities::difficulty::{DifficultyLevelData, DifficultyModifiers, DifficultyRules};
pub use entities::economy_rules::EconomyRules;
pub use entities::edict::Edict;
pub use entities::encounter::{
    Encounter, EncounterOption, EncounterOutcome, EncounterResult, EncounterRules, EncounterSpawn,
    EncounterUnits, SpawnWar, ARMY_EFFECT_KINDS,
};
pub use entities::event::{
    CharacterRef, Condition, Event, EventCategory, EventDate, EventEffect, EventOption, EventScope,
    EventSeason, EventTrigger, ProvinceRef,
};
pub use entities::faction::{
    AiPersonality, ClaimData, ClaimKind, Faction, Government, Heraldry, Objective,
    ObjectiveCondition, Relation, RelationStatus, SuccessionLaw, VictoryConditions,
};
pub use entities::feudal_rules::{FeudalRules, LoyaltyWeights};
pub use entities::landmark::{
    Landmark, LandmarkBattle, LandmarkGate, LandmarkSiege, LandmarkStreet, LandmarkWall,
};
pub use entities::movement::{EmbarkCost, FreeMovementRules, TerrainCosts};
pub use entities::names::NameList;
pub use entities::naval::{
    FactionFleet, FactionShipNames, Marines, NavalData, NavalFleets, NavalRules, NavalShipNames,
    Propulsion, ShipClass,
};
pub use entities::population_rules::PopulationRules;
pub use entities::posture::{
    AmbushRules, CoverRules, EntrenchedRules, ForcedMarchRules, PostureRules,
};
pub use entities::province::{
    CapitalCity, Climate, Population, PopulationClass, PopulationClasses, Province, ProvinceGeo,
    Terrain,
};
pub use entities::r#trait::{Trait, TraitCategory};
pub use entities::religion::{Religion, ReligionKind};
pub use entities::replenishment::{
    ArmyReplenishmentRules, CategoryPercent, KindValues, RecruitPoolRules, ReplenishmentRules,
    StancePercent, TerritoryPercent,
};
pub use entities::resource::{Resource, ResourceCategory};
pub use entities::retinue::{
    Acquisition, AcquisitionTrigger, Companion, CompanionCategory, CompanionConditions, Retinue,
};
pub use entities::settlement::{
    FullProvinceBonus, MovementRules, RetreatRules, Settlement, SettlementEdge, SettlementGraph,
    SettlementKind, SettlementRules,
};
pub use entities::skill::{Skill, SkillBranch};
pub use entities::technology::{TechBranch, TechUnlocks, Technology};
pub use entities::title::{
    FeudalTitle, TitleHolder, TitleObjective, TitleObjectiveCondition, TitleRank,
};
pub use entities::trade::{TradeCatalog, TradeHub, TradeRouteDef};
pub use entities::unit_type::{Ability, Missile, UnitStats, UnitType};
pub use entities::vision::VisionRules;
pub use ids::{
    BuildingId, CharacterId, ChivalricOrderId, CompanionId, CultureId, DietId, EdictId,
    EncounterId, EventId, FactionId, NamesId, ProvinceId, ReligionId, ResourceId, SeaZoneId,
    SettlementId, ShipClassId, SkillId, TechnologyId, TitleId, TraitId, UnitTypeId,
};
pub use load::{upgrade_regressions, DataError, GameData, ReferenceError, Warning};
pub use map::{MapMeta, ProvinceGeometry};
pub use movement_graph::{distance_km, terrain_cost, GraphEdge, MovementGraph};
pub use navgrid::{MapRasters, NavGrid, ProvinceRaster, IMPASSABLE, PLAIN_COST};
pub use trade_paths::{TradePath, TradePaths};
