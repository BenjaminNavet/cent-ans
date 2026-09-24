//! H6 « Ordres de chevalerie » : Jarretière, Étoile, Toison d'or...
//!
//! A faction founds at most one order (`found_chivalric_order`, or the
//! « fonder » option of `evt_ordre_de_la_jarretiere` /
//! `evt_ordre_de_l_etoile`): it pays the cost, needs the ruler's prestige,
//! and its best adult characters are named automatically up to the order's
//! (reduced) strength. Members gain loyalty once; the units a member leads
//! fight with more morale; the ruler gains prestige at foundation and every
//! year. Dead or lost members are replaced every season.
//!
//! Mauron (1352): when an order loses half of its members in a single
//! battle (killed or captured), it is broken: it stops giving its bonuses
//! and the ruler loses prestige ([`COLLAPSE_PRESTIGE`]).

use data_model::{CharacterId, ChivalricOrderId, FactionId, GameData};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::state::CampaignState;

/// Ruler prestige lost when the order collapses.
pub const COLLAPSE_PRESTIGE: i32 = 15;

/// The order a faction has founded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderState {
    pub order: ChivalricOrderId,
    pub founded_turn: u32,
    /// Current members, best first.
    #[serde(default)]
    pub members: Vec<CharacterId>,
    /// Broken by a disaster (Mauron): no more bonuses.
    #[serde(default)]
    pub collapsed: bool,
}

/// Why `found_chivalric_order` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChivalryError {
    #[error("ordre de chevalerie inconnu : {0}")]
    UnknownOrder(ChivalricOrderId),
    #[error("votre faction a déjà fondé un ordre")]
    AlreadyFounded,
    #[error("cet ordre appartient à une autre faction")]
    OtherFaction,
    #[error("cet ordre est déjà fondé par une autre faction")]
    TakenByOther,
    #[error("cet ordre ne peut être fondé avant {0}")]
    TooEarly(i32),
    #[error("prestige insuffisant : {needed} requis, {current} actuellement")]
    NotEnoughPrestige { needed: i32, current: i32 },
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("votre faction n'a pas de souverain")]
    NoRuler,
}

/// `found_chivalric_order` (skeleton).
pub fn found_order(
    _state: &mut CampaignState,
    _data: &GameData,
    _faction: &FactionId,
    _order: &ChivalricOrderId,
) -> Result<(), ChivalryError> {
    Ok(())
}

/// Event effect `found_chivalric_order` (skeleton).
pub fn found_order_by_event(
    _state: &mut CampaignState,
    _data: &GameData,
    _faction: &FactionId,
    _order: &ChivalricOrderId,
    _events: &mut Vec<GameEvent>,
) {
}
