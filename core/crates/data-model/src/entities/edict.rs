//! Regional edict: a province-level policy choice (`edict.schema.json`),
//! (`docs/design/2026-09-24-analyse-total-war.md` § 2.1 « Édits
//! régionaux »).
//!
//! An edict has population/economy/military effects (the shared
//! [`Effect`] vocabulary, read into `EffectTotals` like a building's) and a
//! delay in turns before a newly chosen edict actually takes effect. The
//! rules live in `sim-campaign::edicts`.

use serde::{Deserialize, Serialize};

use crate::common::{Effect, LocalizedName, Sources};

/// What an edict costs its controller (WH econ).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdictCost {
    /// Livres paid each season while the edict is in force (court
    /// and administration line of the budget).
    #[serde(default)]
    pub money: i64,
    /// Prestige of the ruler spent once, when the edict is adopted.
    #[serde(default)]
    pub prestige: i32,
}

/// What a province must offer for an edict to be adopted (WH econ).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdictRequirements {
    /// A building (or an upgrade of it) standing in the province's city.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building: Option<crate::ids::BuildingId>,
    /// A technology known by the controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technology: Option<crate::ids::TechnologyId>,
    /// A faith the controller must follow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub religion: Option<crate::ids::ReligionId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edict {
    pub id: crate::ids::EdictId,
    pub name: LocalizedName,
    /// French description; may contain codex links `[[cdx_…]]`.
    pub description: String,
    /// Turns between the choice and the effect taking hold (0: immediate).
    #[serde(default)]
    pub delay_turns: u32,
    /// WH econ: seasonal and one-off price.
    #[serde(default)]
    pub cost: EdictCost,
    /// WH econ: prerequisites.
    #[serde(default)]
    pub requires: EdictRequirements,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
