//! Diplomacy (M5 spec § 2.1-2.3): attitude, casus belli, war and peace,
//! alliances and calls to arms, embargoes, vassals, proposals and offers, and
//! the minimal diplomatic AI.
//!
//! Every proposal is judged by [`evaluate`], a pure function shared by the AI
//! and the UI (which shows the verdict and its reasons before sending).
//! Proposals to the player become [`Offer`]s answered with `answer_offer`.

use data_model::key_enum;
use data_model::EffectKind;
use std::collections::BTreeSet;

use data_model::{ClaimKind, FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::negotiation::{Article, Treaty};
use crate::orders::Order;
use crate::plan_cache::PlanCache;
use crate::religion;
use crate::state::CampaignState;

/// Truce after a negotiated peace (5 years).
pub const TRUCE_TURNS: u32 = 20;
/// Reputation modifier every faction holds against a truce breaker.
pub const PERJURY_REASON: &str = "Parjure : trêve rompue";
/// Reputation modifier every faction holds against an unprovoked attacker.
pub const AGGRESSION_REASON: &str = "Agression sans motif";
/// Attitude reason of two factions at war.
pub const AT_WAR_REASON: &str = "En guerre";
/// Attitude reason of the campaign difficulty, AI towards the player.
pub const DIFFICULTY_REASON: &str = "Niveau de difficulté";
/// Opinion reason of a marriage between two ruling houses.
pub const MARRIAGE_REASON: &str = "Mariage entre nos maisons";
/// Attitude reason of rulers bound by marriage.
pub const MARRIAGE_TIE_REASON: &str = "Liens matrimoniaux";
/// Attitude reason of rulers of the same house.
pub const SAME_HOUSE_REASON: &str = "Même maison régnante";
/// Attitude reasons owed to kinship (EQ6, `war.claim_war_ignores_kinship`).
pub const KINSHIP_REASONS: [&str; 3] = [MARRIAGE_REASON, MARRIAGE_TIE_REASON, SAME_HOUSE_REASON];
/// Reason of the opinion modifier a gift leaves with its recipient.
pub const GIFT_REASON: &str = "Présents diplomatiques";
/// Truce obtained through papal mediation (2 years).
pub const MEDIATION_TRUCE_TURNS: u32 = 8;
/// Turns an offer to the player stays open.
pub const OFFER_LIFETIME: u32 = 2;
/// Minimum turns between two offers of the same AI faction to the player.
pub const OFFER_COOLDOWN: u32 = 4;
/// Income lost by the target of each embargo.
pub const EMBARGO_TARGET_PENALTY: f64 = 0.08;
/// Income lost by the faction imposing each embargo.
pub const EMBARGO_IMPOSER_PENALTY: f64 = 0.03;
/// Cost of a papal mediation, paid to the Papacy.
pub const MEDIATION_COST: i64 = 1000;
/// Papal favour needed to ask for a mediation.
pub const MEDIATION_MIN_FAVOR: u8 = 30;
/// Modifier duration meaning "never expires".
pub const FOREVER: u32 = u32::MAX;

pub const REBELS_FACTION: &str = "fac_rebels";
pub const PAPACY_FACTION: &str = "fac_papacy";

/// RS-C: the capped motive (`data/rules/diplomacy.json`) of an opinion
/// modifier's reason, if any.
pub fn opinion_motive(reason: &str) -> Option<data_model::OpinionMotive> {
    use data_model::OpinionMotive;
    match reason {
        MARRIAGE_REASON => Some(OpinionMotive::Marriage),
        crate::agents::PARLEY_REASON => Some(OpinionMotive::HeraldEmbassy),
        GIFT_REASON => Some(OpinionMotive::Gift),
        crate::negotiation::TREATY_REASON => Some(OpinionMotive::Treaty),
        _ => None,
    }
}

// =========================================================================
// Types
// =========================================================================

/// A claim (casus belli) held by a faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub kind: ClaimKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<FactionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    pub text_fr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_turn: Option<u32>,
}

/// A remembered event changing how the holder sees `with`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpinionModifier {
    pub with: FactionId,
    pub value: i32,
    pub reason_fr: String,
    pub expires_turn: u32,
}

/// A proposal waiting for the player's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: u32,
    pub from: FactionId,
    pub proposal: Treaty,
    pub expires_turn: u32,
    pub text_fr: String,
}

key_enum! {
/// Relation of a faction with another, as shown by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    War => "war",
    Truce => "truce",
    Peace => "peace",
    Alliance => "alliance",
    /// The other faction is our vassal.
    Vassal => "vassal",
    /// The other faction is our suzerain.
    Suzerain => "suzerain",
}
}

/// Why a diplomatic order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DiplomacyError {
    #[error("faction inconnue ou disparue : {0}")]
    UnknownFaction(FactionId),
    #[error("une faction ne peut pas traiter avec elle-même")]
    SelfTarget,
    #[error("déjà en guerre")]
    AlreadyAtWar,
    #[error("pas en guerre avec cette faction")]
    NotAtWar,
    #[error("déjà alliés")]
    AlreadyAllied,
    #[error("rompez d'abord l'alliance ou la vassalité")]
    Allied,
    #[error("pas d'alliance à rompre")]
    NotAllied,
    #[error("cette faction n'est pas votre vassale")]
    NotVassal,
    #[error("{0}")]
    Refused(String),
    #[error("trésor insuffisant")]
    InsufficientFunds,
    #[error("montant invalide")]
    InvalidAmount,
    #[error("offre inconnue ou expirée")]
    UnknownOffer,
    #[error("province non concernée par cette guerre : {0}")]
    InvalidProvince(ProvinceId),
    #[error("faveur pontificale insuffisante")]
    PapalFavorTooLow,
    #[error("seules les factions catholiques peuvent solliciter le pape")]
    NotCatholic,
    #[error("aucun schisme en cours")]
    NoSchism,
    #[error("obédience invalide")]
    InvalidObedience,
    #[error("aucun accès militaire accordé à cette faction")]
    NoMilitaryAccess,
    #[error("pas d'accord commercial à rompre")]
    NoTradeAgreement,
    #[error("la faction virtuelle des rebelles ne négocie pas")]
    Rebels,
}

/// Refusal of an alliance between a suzerain and its direct vassal (ADR 0114).
pub const FEUDAL_TIE_ALLIANCE: &str = "le lien féodal tient déjà lieu d'alliance";

mod offers;
mod queries;
mod ties;
mod upkeep;
mod view;
mod war;
pub use ai::{
    answers_call_to_arms, call_to_arms_forecast, claim_stakes, claimed_provinces, is_cornered,
    main_claim, plan_diplomacy, rivals, war_ready, weariness_to_declare, CallForecast, ClaimStakes,
    DESERTION_WAR_SCORE, MAX_ALLIANCES, OPPORTUNIST_AGGRESSION, OPPORTUNIST_RATIO,
    PRETENDER_AGGRESSION, PRETENDER_PEACE_RELUCTANCE, SURRENDER_WAR_SCORE, WAR_REST_TURNS,
};
pub use upkeep::loyalty_target;
pub(crate) use upkeep::{on_line_extinct, resolve_diplomacy};
pub use view::DiplomacyEntry;
pub use war::{AGGRESSION_PRESTIGE, PERJURY_PRESTIGE};

mod ai;
mod attitude;

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<RelationKind>();
    }
}
