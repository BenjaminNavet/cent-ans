//! Character trait: personality, martial, governance, physical or acquired (`trait.schema.json`).

use serde::{Deserialize, Serialize};

use crate::common::{Effect, LocalizedName, Sources};
use crate::ids::TraitId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraitCategory {
    Personality,
    Martial,
    Governance,
    Physical,
    Acquired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trait {
    pub id: TraitId,
    pub name: LocalizedName,
    pub category: TraitCategory,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    /// Traits that mutually exclude this one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opposites: Vec<TraitId>,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
