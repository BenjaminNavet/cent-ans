//! Why an order was refused.

use data_model::{BuildingId, CharacterId, FactionId, ProvinceId, SettlementId, UnitTypeId};

use crate::dynasty::{GovernorError, MarriageError};
use crate::research::ResearchError;
use crate::skills::LearnSkillError;
use crate::state::ArmyId;

/// Why an order was refused (messages in French for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrderError {
    #[error("armée inconnue : {0}")]
    UnknownArmy(ArmyId),
    #[error("province inconnue : {0}")]
    UnknownProvince(ProvinceId),
    #[error("colonie inconnue : {0}")]
    UnknownSettlement(SettlementId),
    #[error("type d'unité inconnu : {0}")]
    UnknownUnitType(UnitTypeId),
    #[error("personnage inconnu : {0}")]
    UnknownCharacter(CharacterId),
    #[error("cette armée n'appartient pas à la faction {0}")]
    NotYourArmy(FactionId),
    #[error("ce personnage n'appartient pas à la faction {0}")]
    NotYourCharacter(FactionId),
    #[error("cette province n'est pas contrôlée par la faction {0}")]
    NotYourProvince(FactionId),
    #[error("cette colonie n'est pas contrôlée par la faction {0}")]
    NotYourSettlement(FactionId),
    #[error("chemin vide")]
    EmptyPath,
    #[error("chemin invalide : {from} et {to} ne sont pas reliées (route ou mer entre ports)")]
    NotAdjacent { from: String, to: String },
    #[error("destination inaccessible")]
    NoPath,
    #[error("cette armée n'a plus de points de mouvement ce tour")]
    NoMovementLeft,
    #[error("l'armée ciblée est hors de portée ce tour")]
    OutOfRange,
    #[error("ces deux factions ne sont pas en guerre")]
    NotAtWar,
    #[error("l'armée doit stationner dans un port pour embarquer")]
    NotInPort,
    #[error("aucune route maritime de {from} à {to} en une saison")]
    NoSeaRoute { from: String, to: String },
    #[error("l'embarquement prend toute la saison : l'armée ne doit pas avoir bougé ce tour")]
    EmbarkNeedsFullTurn,
    #[error("l'armée doit stationner dans cette colonie")]
    NotInSettlement,
    #[error("recrutement impossible : {0}")]
    RecruitUnavailable(String),
    #[error("file de recrutement pleine : {slots} recrutement(s) par tour dans cette colonie")]
    RecruitQueueFull { slots: usize },
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("indice d'unité invalide : {0}")]
    InvalidUnitIndex(usize),
    #[error("aucune unité sélectionnée")]
    NoUnitsSelected,
    #[error("une armée doit garder au moins une unité")]
    WouldEmptyArmy,
    #[error("les deux armées doivent être au même endroit")]
    NotSameProvince,
    #[error("les deux armées doivent appartenir à la même faction")]
    NotSameFaction,
    #[error("ce personnage ne peut pas commander (mort, captif, mineur ou d'une autre faction)")]
    CharacterUnavailable,
    #[error("ce personnage n'est pas dans cette province")]
    CharacterElsewhere,
    #[error("ce personnage gouverne déjà une province")]
    AlreadyGoverning,
    #[error("il faut préciser soit une armée soit une province")]
    AmbiguousTarget,
    #[error("faction inconnue : {0}")]
    UnknownFaction(FactionId),
    #[error("bâtiment inconnu : {0}")]
    UnknownBuilding(BuildingId),
    #[error("construction impossible : {0}")]
    BuildUnavailable(String),
    #[error("aucune construction en cours dans cette colonie")]
    NoConstruction,
    #[error("démolition impossible : {0}")]
    DemolitionRefused(String),
    #[error("la colonie est assiégée")]
    SettlementBesieged,
    #[error("posture impossible : {0}")]
    StanceRefused(String),
    #[error("interdit en marche forcée : {0}")]
    ForcedMarchForbids(&'static str),
    #[error("garnison complète : {cap} unités au plus dans ce type de colonie")]
    GarrisonFull { cap: usize },
    /// NT5 (N6): the army would exceed `armies.json` `max_units`.
    #[error("armée complète : {cap} unités au plus par armée")]
    ArmyFull { cap: usize },
    #[error(transparent)]
    LearnSkill(#[from] LearnSkillError),
    #[error(transparent)]
    Governor(#[from] GovernorError),
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
    Coinage(#[from] crate::coinage::CoinageError),
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
    #[error("la place est en ruine : ni recrutement ni chantier")]
    SettlementRuined,
    #[error("engagement impossible : {0}")]
    MercenaryUnavailable(String),
}
