//! Regional edict: a province-level policy choice (`edict.schema.json`),
//! lot C4 (`docs/design/2026-09-24-analyse-total-war.md` § 2.1 « Édits
//! régionaux »).
//!
//! An edict has population/economy/military effects (the shared
//! [`Effect`] vocabulary, read into `EffectTotals` like a building's) and a
//! delay in turns before a newly chosen edict actually takes effect. The
//! rules live in `sim-campaign::edicts`.

use serde::{Deserialize, Serialize};

use crate::common::{Effect, LocalizedName, Sources};

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
