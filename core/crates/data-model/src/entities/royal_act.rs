//! Royal act: a sovereign's ceremonial or political act with a cooldown
//! (`royal_act.schema.json`), the historical counterpart of the rites of
//! Total War: Warhammer III (WH chars).
//!
//! An act costs prestige and/or livres, can only be repeated after
//! `cooldown_turns`, may be reserved to some factions, and applies the shared
//! [`Effect`] vocabulary to the whole faction for `duration_turns`. The rules
//! live in `sim-campaign::royal_acts`.

use serde::{Deserialize, Serialize};

use crate::common::{Effect, LocalizedName, Sources};
use crate::ids::{FactionId, RoyalActId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoyalAct {
    pub id: RoyalActId,
    pub name: LocalizedName,
    /// French description; may contain codex links `[[cdx_…]]`.
    pub description: String,
    /// Prestige spent from the ruler's purse of renown.
    #[serde(default)]
    pub cost_prestige: i32,
    /// Livres spent from the treasury.
    #[serde(default)]
    pub cost_livres: i64,
    /// Turns (seasons) before the same act can be performed again.
    pub cooldown_turns: u32,
    /// Turns the effects last (0: only the one-off gains below).
    #[serde(default)]
    pub duration_turns: u32,
    /// Factions allowed to perform it (empty: every faction).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub factions: Vec<FactionId>,
    /// Prestige gained at once on performing the act.
    #[serde(default)]
    pub gain_prestige: i32,
    /// Piety gained (or lost, negative) at once by the ruler.
    #[serde(default)]
    pub gain_piety: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

/// Rules of captain hiring (`data/rules/captains.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptainRules {
    /// Livres of a captain at the base price level (100).
    pub base_cost: i64,
    /// Captains a faction may keep: `base_cap + ruler prestige / prestige_per_slot`.
    pub base_cap: u32,
    pub prestige_per_slot: i32,
    /// Command rating of a new captain, `min..=max` (inclusive).
    pub command_min: u8,
    pub command_max: u8,
    /// Governance and court rating ceiling of a new captain.
    pub other_skill_max: u8,
    /// Age range of a new captain.
    pub age_min: i32,
    pub age_max: i32,
    /// Seasons between two hirings by the same faction.
    pub cooldown_turns: u32,
}

crate::bundled_rules!(CaptainRules, "rules/captains.json", default);
