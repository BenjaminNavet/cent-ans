//! H6 « Rançons » : prix d'un captif, paiement, échéances, parole, cession.
//!
//! A character taken in battle or by a chronicle event (`captive`,
//! `captor`) carries a ransom computed from his rank, his prestige and the
//! wealth of his faction ([`ransom_amount`]). His captor sets the terms
//! (`set_ransom_terms`: money, a border province, or keep him), may free him
//! on parole (`release_on_parole`); his own faction pays (`pay_ransom`), in
//! full or by yearly installments (the captive comes home with the first
//! one, as Jean II after Brétigny; the rest is a debt, [`RansomDebt`]). A
//! missed installment costs prestige, the creditor's goodwill and a 10 %
//! surcharge.
//!
//! Release always goes through `chronicle::release_character`, shared with
//! the ransom events (`evt_rancon_david_ii`...): a freed character is no
//! longer captive, so a second release (order or event) is a no-op.

use data_model::{CharacterId, FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::state::CampaignState;

/// Base ransom of a ruler (livres tournois).
pub const RANSOM_SOVEREIGN: i64 = 10_000;
/// Base ransom of a ruler's heir.
pub const RANSOM_HEIR: i64 = 5_000;
/// Base ransom of a great noble (titled, or prestige ≥ [`GREAT_NOBLE_PRESTIGE`]).
pub const RANSOM_GREAT_NOBLE: i64 = 1_500;
/// Base ransom of a plain knight.
pub const RANSOM_KNIGHT: i64 = 400;
/// Prestige from which an untitled character counts as a great noble.
pub const GREAT_NOBLE_PRESTIGE: i32 = 30;
/// Maximum number of yearly installments.
pub const MAX_INSTALLMENTS: u32 = 6;
/// Surcharge (per cent) of a ransom paid by installments.
pub const INSTALLMENT_SURCHARGE_PERCENT: i64 = 10;
/// Surcharge (per cent of the remaining debt) of a missed installment.
pub const DEFAULT_SURCHARGE_PERCENT: i64 = 10;
/// Ruler prestige lost per missed installment.
pub const DEFAULT_PRESTIGE: i32 = 5;
/// Creditor's opinion lost per missed installment.
pub const DEFAULT_OPINION: i32 = -15;
/// Prestige the captor's ruler gains by a release on parole.
pub const PAROLE_PRESTIGE: i32 = 8;
/// Opinion the freed character's faction gains towards the captor.
pub const PAROLE_OPINION: i32 = 20;
/// Ruler prestige lost every season the ruler himself is a captive.
pub const CAPTIVE_RULER_PRESTIGE: i32 = 1;

/// Rank of a captive, which sets the base of his ransom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptiveRank {
    Sovereign,
    Heir,
    GreatNoble,
    Knight,
}

impl CaptiveRank {
    pub fn base_ransom(self) -> i64 {
        match self {
            CaptiveRank::Sovereign => RANSOM_SOVEREIGN,
            CaptiveRank::Heir => RANSOM_HEIR,
            CaptiveRank::GreatNoble => RANSOM_GREAT_NOBLE,
            CaptiveRank::Knight => RANSOM_KNIGHT,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            CaptiveRank::Sovereign => "sovereign",
            CaptiveRank::Heir => "heir",
            CaptiveRank::GreatNoble => "great_noble",
            CaptiveRank::Knight => "knight",
        }
    }

    pub fn label_fr(self) -> &'static str {
        match self {
            CaptiveRank::Sovereign => "souverain",
            CaptiveRank::Heir => "héritier",
            CaptiveRank::GreatNoble => "grand seigneur",
            CaptiveRank::Knight => "chevalier",
        }
    }
}

/// Terms the captor sets for a captive (`set_ransom_terms`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RansomTerms {
    /// Freed against the computed ransom (default).
    #[default]
    Money,
    /// Freed against a province of his faction bordering the captor's lands.
    Province { province: ProvinceId },
    /// Freed at once on his word (prestige and goodwill).
    Parole,
    /// Not for sale: the captor keeps him.
    Hold,
}

impl RansomTerms {
    pub fn key(&self) -> &'static str {
        match self {
            RansomTerms::Money => "money",
            RansomTerms::Province { .. } => "province",
            RansomTerms::Parole => "parole",
            RansomTerms::Hold => "hold",
        }
    }
}

/// The unpaid part of a ransom paid by installments (held by the payer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RansomDebt {
    /// The freed character.
    pub character: CharacterId,
    /// Faction owed the money.
    pub creditor: FactionId,
    /// Livres still owed.
    pub remaining: i64,
    /// Livres due at each yearly installment.
    pub installment: i64,
    /// Turn of the next installment.
    pub next_due_turn: u32,
    /// Installments missed so far.
    #[serde(default)]
    pub missed: u32,
}

/// Why a ransom order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RansomError {
    #[error("ce personnage n'est pas captif")]
    NotCaptive,
    #[error("ce captif n'appartient pas à votre faction")]
    NotYourCaptive,
    #[error("ce captif n'est pas détenu par votre faction")]
    NotYourPrisoner,
    #[error("son geôlier refuse toute rançon")]
    Held,
    #[error("son geôlier exige une province, pas de l'argent : {0}")]
    ProvinceDemanded(String),
    #[error("nombre d'échéances invalide (1 à {MAX_INSTALLMENTS})")]
    BadInstallments,
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("province impossible : {0}")]
    BadProvince(String),
}

/// `pay_ransom` (skeleton).
pub fn pay_ransom(
    _state: &mut CampaignState,
    _data: &GameData,
    _faction: &FactionId,
    _character: &CharacterId,
    _installments: u32,
) -> Result<(), RansomError> {
    Ok(())
}

/// `set_ransom_terms` (skeleton).
pub fn set_ransom_terms(
    _state: &mut CampaignState,
    _data: &GameData,
    _faction: &FactionId,
    _character: &CharacterId,
    _terms: RansomTerms,
) -> Result<(), RansomError> {
    Ok(())
}

/// `release_on_parole` (skeleton).
pub fn release_on_parole(
    _state: &mut CampaignState,
    _data: &GameData,
    _faction: &FactionId,
    _character: &CharacterId,
) -> Result<(), RansomError> {
    Ok(())
}
