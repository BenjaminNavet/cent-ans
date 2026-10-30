//! Why an order was refused.

use data_model::{BuildingId, CharacterId, FactionId, ProvinceId, SettlementId, UnitTypeId};

use crate::dynasty::{GovernorError, MarriageError};
use crate::research::ResearchError;
use crate::skills::LearnSkillError;
use crate::state::ArmyId;

/// Why an order was refused (messages in French for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrderError {
    #[error("{0}")]
    MissionChoice(#[from] crate::missions::MissionChoiceError),
    #[error("{}", data_model::texts::core_text("order.unknown_army", &[("0", &.0)]))]
    UnknownArmy(ArmyId),
    #[error("{}", data_model::texts::core_text("order.unknown_province", &[("0", &.0)]))]
    UnknownProvince(ProvinceId),
    #[error("{}", data_model::texts::core_text("order.unknown_settlement", &[("0", &.0)]))]
    UnknownSettlement(SettlementId),
    #[error("{}", data_model::texts::core_text("order.unknown_unit_type", &[("0", &.0)]))]
    UnknownUnitType(UnitTypeId),
    #[error("{}", data_model::texts::core_text("order.unknown_character", &[("0", &.0)]))]
    UnknownCharacter(CharacterId),
    #[error("{}", data_model::texts::core_text("order.not_your_army", &[("0", &.0)]))]
    NotYourArmy(FactionId),
    #[error("{}", data_model::texts::core_text("order.not_your_character", &[("0", &.0)]))]
    NotYourCharacter(FactionId),
    #[error("{}", data_model::texts::core_text("order.not_your_province", &[("0", &.0)]))]
    NotYourProvince(FactionId),
    #[error("{}", data_model::texts::core_text("order.not_your_settlement", &[("0", &.0)]))]
    NotYourSettlement(FactionId),
    #[error("{}", data_model::texts::core_text("order.empty_path", &[]))]
    EmptyPath,
    #[error("{}", data_model::texts::core_text("order.not_adjacent", &[("from", &.from), ("to", &.to)]))]
    NotAdjacent { from: String, to: String },
    #[error("{}", data_model::texts::core_text("order.no_path", &[]))]
    NoPath,
    #[error("{}", data_model::texts::core_text("order.no_movement_left", &[]))]
    NoMovementLeft,
    #[error("{}", data_model::texts::core_text("order.out_of_range", &[]))]
    OutOfRange,
    #[error("{}", data_model::texts::core_text("order.not_at_war", &[]))]
    NotAtWar,
    #[error("{}", data_model::texts::core_text("order.not_in_port", &[]))]
    NotInPort,
    #[error("{}", data_model::texts::core_text("order.no_sea_route", &[("from", &.from), ("to", &.to)]))]
    NoSeaRoute { from: String, to: String },
    #[error("{}", data_model::texts::core_text("order.no_river_route", &[("from", &.from), ("to", &.to)]))]
    NoRiverRoute { from: String, to: String },
    #[error("{}", data_model::texts::core_text("order.river_too_far", &[("cost", &.cost), ("left", &.left)]))]
    RiverTooFar { cost: u32, left: u32 },
    #[error("{}", data_model::texts::core_text("order.embark_needs_full_turn", &[]))]
    EmbarkNeedsFullTurn,
    #[error("{}", data_model::texts::core_text("order.not_in_settlement", &[]))]
    NotInSettlement,
    #[error("{}", data_model::texts::core_text("order.recruit_unavailable", &[("0", &.0)]))]
    RecruitUnavailable(String),
    #[error("{}", data_model::texts::core_text("order.recruit_queue_full", &[("slots", &.slots)]))]
    RecruitQueueFull { slots: usize },
    #[error("{}", data_model::texts::core_text("order.insufficient_funds", &[("needed", &.needed), ("money", &data_model::currency::MONEY), ("available", &.available)]))]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("{}", data_model::texts::core_text("order.invalid_unit_index", &[("0", &.0)]))]
    InvalidUnitIndex(usize),
    #[error("{}", data_model::texts::core_text("order.no_units_selected", &[]))]
    NoUnitsSelected,
    #[error("{}", data_model::texts::core_text("order.would_empty_army", &[]))]
    WouldEmptyArmy,
    #[error("{}", data_model::texts::core_text("order.host_contingent", &[]))]
    HostContingent,
    #[error("{}", data_model::texts::core_text("order.not_same_province", &[]))]
    NotSameProvince,
    #[error("{}", data_model::texts::core_text("order.not_same_faction", &[]))]
    NotSameFaction,
    #[error("{}", data_model::texts::core_text("order.character_unavailable", &[]))]
    CharacterUnavailable,
    #[error("{}", data_model::texts::core_text("order.character_elsewhere", &[]))]
    CharacterElsewhere,
    #[error("{}", data_model::texts::core_text("order.character_busy", &[]))]
    CharacterBusy,
    #[error("{}", data_model::texts::core_text("order.already_governing", &[]))]
    AlreadyGoverning,
    #[error("{}", data_model::texts::core_text("order.ambiguous_target", &[]))]
    AmbiguousTarget,
    #[error("{}", data_model::texts::core_text("order.unknown_faction", &[("0", &.0)]))]
    UnknownFaction(FactionId),
    #[error("{}", data_model::texts::core_text("order.unknown_building", &[("0", &.0)]))]
    UnknownBuilding(BuildingId),
    #[error("{}", data_model::texts::core_text("order.vocation_refused", &[("0", &.0)]))]
    VocationRefused(String),
    #[error("{}", data_model::texts::core_text("order.build_unavailable", &[("0", &.0)]))]
    BuildUnavailable(String),
    #[error("{}", data_model::texts::core_text("order.no_construction", &[]))]
    NoConstruction,
    #[error("{}", data_model::texts::core_text("order.no_queued_recruit", &[]))]
    NoQueuedRecruit,
    #[error("{}", data_model::texts::core_text("order.demolition_refused", &[("0", &.0)]))]
    DemolitionRefused(String),
    #[error("{}", data_model::texts::core_text("order.settlement_besieged", &[]))]
    SettlementBesieged,
    #[error("{}", data_model::texts::core_text("order.stance_refused", &[("0", &.0)]))]
    StanceRefused(String),
    #[error("{}", data_model::texts::core_text("order.forced_march_forbids", &[("0", &.0)]))]
    ForcedMarchForbids(&'static str),
    #[error("{}", data_model::texts::core_text("order.garrison_full", &[("cap", &.cap)]))]
    GarrisonFull { cap: usize },
    /// NT5 (N6): the army would exceed `armies.json` `max_units`.
    #[error("{}", data_model::texts::core_text("order.army_full", &[("cap", &.cap)]))]
    ArmyFull { cap: usize },
    #[error("{}", data_model::texts::core_text("order.sortie_unavailable", &[("0", &.0)]))]
    SortieUnavailable(String),
    #[error("{}", data_model::texts::core_text("order.surrender_unavailable", &[("0", &.0)]))]
    SurrenderUnavailable(String),
    #[error(transparent)]
    LearnSkill(#[from] LearnSkillError),
    #[error(transparent)]
    Governor(#[from] GovernorError),
    #[error(transparent)]
    Heir(#[from] crate::dynasty::HeirError),
    #[error(transparent)]
    Retinue(#[from] crate::retinue::RetinueError),
    #[error(transparent)]
    Marriage(#[from] MarriageError),
    #[error(transparent)]
    Diplomacy(#[from] crate::diplomacy::DiplomacyError),
    #[error(transparent)]
    Feudal(#[from] crate::feudal::FeudalError),
    #[error(transparent)]
    Research(#[from] ResearchError),
    #[error(transparent)]
    Chronicle(#[from] crate::chronicle::ChronicleError),
    #[error(transparent)]
    Assault(#[from] crate::siege::AssaultError),
    #[error(transparent)]
    Diet(#[from] crate::table::DietError),
    #[error(transparent)]
    Edict(#[from] crate::edicts::EdictError),
    #[error(transparent)]
    RoyalAct(#[from] crate::royal_acts::RoyalActError),
    #[error(transparent)]
    Captain(#[from] crate::captains::CaptainError),
    #[error(transparent)]
    Council(#[from] crate::council::CouncilError),
    #[error(transparent)]
    ProvinceTax(#[from] crate::province_tax::ProvinceTaxError),
    #[error(transparent)]
    Coinage(#[from] crate::coinage::CoinageError),
    #[error(transparent)]
    Finance(#[from] crate::finance::FinanceError),
    #[error(transparent)]
    Assembly(#[from] crate::assembly::AssemblyError),
    #[error(transparent)]
    Ransom(#[from] crate::ransom::RansomError),
    #[error(transparent)]
    Chivalry(#[from] crate::chivalry::ChivalryError),
    #[error(transparent)]
    Crusade(#[from] crate::crusade::CrusadeError),
    #[error(transparent)]
    Agent(#[from] crate::agents::AgentError),
    #[error(transparent)]
    Encounter(#[from] crate::encounter::EncounterError),
    #[error(transparent)]
    Capture(#[from] crate::capture::CaptureError),
    #[error(transparent)]
    Tradition(#[from] crate::traditions::TraditionError),
    #[error("{}", data_model::texts::core_text("order.settlement_ruined", &[]))]
    SettlementRuined,
    #[error("{}", data_model::texts::core_text("order.mercenary_unavailable", &[("0", &.0)]))]
    MercenaryUnavailable(String),
}
