//! Per-culture list of period first names, used to generate new characters (`names.schema.json`).

use serde::{Deserialize, Serialize};

use crate::common::Sources;
use crate::ids::{CultureId, NamesId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameList {
    pub id: NamesId,
    /// Language or language group (e.g. "français médiéval").
    pub language: String,
    /// Cultures (`cul_*`) this name list applies to.
    pub cultures: Vec<CultureId>,
    pub male_first_names: Vec<String>,
    pub female_first_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surnames: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
