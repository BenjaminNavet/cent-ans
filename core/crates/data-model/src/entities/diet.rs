//! Diet: what a province eats, H3 « La Table » (`diet.schema.json`).
//!
//! A diet has requirements, a seasonal cost per thousand inhabitants,
//! population effects and seasonal rules (Lent, winter). The rules that read
//! these fields live in `sim-campaign::table`.

use serde::{Deserialize, Serialize};

use crate::common::{Effect, LocalizedName, Sources};
use crate::entities::province::Terrain;
use crate::ids::{BuildingId, DietId, ResourceId, TechnologyId};

/// What a diet does during Lent (spring turn).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LentRule {
    /// Unaffected by Lent.
    #[default]
    None,
    /// Meat on lean days: piety and clergy penalty.
    Meat,
    /// Lean fish: piety bonus.
    Fish,
    /// Eggs and dairy, forbidden during Lent: same penalty as meat.
    Dairy,
}

impl LentRule {
    /// The `snake_case` key used in JSON files.
    pub fn key(self) -> &'static str {
        match self {
            LentRule::None => "none",
            LentRule::Meat => "meat",
            LentRule::Fish => "fish",
            LentRule::Dairy => "dairy",
        }
    }
}

/// What a diet does in winter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WinterRule {
    #[default]
    None,
    /// Fresh produce: costs more in winter.
    Fresh,
}

impl WinterRule {
    /// The `snake_case` key used in JSON files.
    pub fn key(self) -> &'static str {
        match self {
            WinterRule::None => "none",
            WinterRule::Fresh => "fresh",
        }
    }
}

/// Conditions a province (and its controller) must meet to adopt a diet.
/// Every listed condition must hold; an empty requirement always holds.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DietRequirements {
    /// Resources the controlling faction must have access to (the goods of
    /// the goods-satisfaction model: produced by a controlled or allied
    /// province). All are required.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ResourceId>,
    /// The province must be coastal.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub coastal: bool,
    /// Technology the controlling faction must know.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technology: Option<TechnologyId>,
    /// At least one of these buildings must stand in the province.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_building: Vec<BuildingId>,
    /// The province terrain must be one of these (empty: any terrain).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrains: Vec<Terrain>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diet {
    pub id: DietId,
    pub name: LocalizedName,
    /// French description; may contain codex links `[[cdx_…]]`.
    pub description: String,
    #[serde(default)]
    pub requirements: DietRequirements,
    /// Livres tournois per season for 1 000 inhabitants.
    pub cost_per_thousand: f64,
    /// Population effects (`class` targets one social class).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    #[serde(default)]
    pub lent_rule: LentRule,
    #[serde(default)]
    pub winter_rule: WinterRule,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}
