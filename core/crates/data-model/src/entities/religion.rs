//! Religion: church, obedience, heresy or other faith (`religion.schema.json`).

use serde::{Deserialize, Serialize};

use crate::common::{Effect, HistoricalDate, LocalizedName, Sources};
use crate::ids::{FactionId, ReligionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReligionKind {
    Church,
    Obedience,
    Heresy,
    OtherFaith,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Religion {
    pub id: ReligionId,
    pub name: LocalizedName,
    pub kind: ReligionKind,
    /// Parent religion (an obedience or heresy derives from Catholicism).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<ReligionId>,
    /// Faction leading the church (the Papacy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_faction: Option<FactionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_from: Option<HistoricalDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_until: Option<HistoricalDate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
