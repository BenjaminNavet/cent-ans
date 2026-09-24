//! Chivalric order (H6): Garter, Star, Golden Fleece... (`chivalric_order.schema.json`).
//!
//! The rules that read these fields (foundation, members, effects) live in
//! `sim-campaign::chivalry`.

use serde::{Deserialize, Serialize};

use crate::common::{HistoricalDate, LocalizedName, Sources};
use crate::ids::{ChivalricOrderId, FactionId};

/// An order a faction may found once (`data/chivalric_orders/*.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChivalricOrder {
    pub id: ChivalricOrderId,
    pub name: LocalizedName,
    /// French text; may contain codex links `[[cdx_…]]`.
    pub description: String,
    /// Faction the historical order belongs to; `None`: the generic order,
    /// open to every faction without a historical one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<FactionId>,
    /// Earliest year the order can be founded (historical foundation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_year: Option<i32>,
    /// Foundation cost in livres tournois.
    pub cost: u32,
    /// Ruler prestige required to found it.
    pub prestige_required: i32,
    /// Knights named automatically (the game's reduced strength).
    pub members: u32,
    /// Historical strength, for the codex and the UI.
    pub historical_members: String,
    /// Loyalty gained once by every member at nomination.
    pub member_loyalty: u8,
    /// Morale bonus of the units led by a member general.
    pub member_morale: u8,
    /// Ruler prestige gained at foundation.
    pub founder_prestige: i32,
    /// Ruler prestige gained every year while the order stands.
    pub yearly_prestige: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_date: Option<HistoricalDate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
