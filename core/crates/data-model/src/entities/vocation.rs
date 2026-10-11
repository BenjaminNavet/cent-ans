//! Settlement vocations (RG10, ADR 0344): `data/settlements/vocations.json`
//! (schema `data/schemas/vocations.schema.json`).
//!
//! A settlement may specialise in one vocation: the buildings it concerns
//! cost less and work better, the others cost more, and enough buildings of
//! the vocation unlock coherence tiers. Every figure is data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::common::{Effect, EffectKind, LocalizedName};
use crate::entities::building::{Building, BuildingCategory};
use crate::entities::settlement::SettlementKind;
use crate::ids::BuildingId;

/// Contents of `data/settlements/vocations.json`; empty by default (no
/// vocation exists, as in saves and fixtures that predate RG10).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VocationRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Livres to adopt or change a vocation, by settlement kind.
    #[serde(default)]
    pub adoption_cost: BTreeMap<SettlementKind, u32>,
    /// Share (per cent) of `adoption_cost` paid for the very first vocation
    /// of a settlement.
    #[serde(default = "default_first_percent")]
    pub first_adoption_percent: u32,
    /// Turns (seasons) during which a vocation cannot be changed again.
    #[serde(default)]
    pub change_lock_turns: u32,
    /// Effects the vocation bonus does not scale (integral or structural
    /// values: wall levels, recruitment slots).
    #[serde(default)]
    pub bonus_excluded_effects: Vec<EffectKind>,
    /// AI: fewest concerned buildings (the one under construction counts)
    /// before it adopts a vocation.
    #[serde(default = "default_one")]
    pub ai_min_buildings: u32,
    /// AI: concerned buildings a rival vocation must fit better by before the
    /// AI changes an existing vocation.
    #[serde(default = "default_two")]
    pub ai_switch_margin: u32,
    pub vocations: Vec<Vocation>,
}

fn default_one() -> u32 {
    1
}

fn default_two() -> u32 {
    2
}

fn default_first_percent() -> u32 {
    100
}

/// One vocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vocation {
    /// Snake-case identifier (`agricultural`, `mercantile`, ...).
    pub id: String,
    pub name: LocalizedName,
    /// Short icon glyph or asset key for the UI; optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Building categories the vocation concerns.
    #[serde(default)]
    pub categories: Vec<BuildingCategory>,
    /// Individual buildings it concerns (besides the categories).
    #[serde(default)]
    pub buildings: Vec<BuildingId>,
    /// Bonus (per cent) to the effects of the concerned buildings.
    #[serde(default)]
    pub effect_bonus_percent: f64,
    /// Construction cost reduction (per cent) of the concerned buildings.
    #[serde(default)]
    pub cost_reduction_percent: f64,
    /// Construction surcharge (per cent) of the buildings it does not concern.
    #[serde(default)]
    pub off_vocation_surcharge_percent: f64,
    /// Coherence tiers, cumulative, by increasing `min_buildings`.
    #[serde(default)]
    pub coherence: Vec<CoherenceTier>,
}

/// A coherence tier: with at least `min_buildings` concerned buildings
/// standing in the settlement, `effects` apply to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoherenceTier {
    pub min_buildings: u32,
    /// Name of the tier, shown in the tooltip.
    pub label: String,
    pub effects: Vec<Effect>,
}

impl Vocation {
    /// `true` when `building` is concerned by the vocation.
    pub fn concerns(&self, building: &Building) -> bool {
        self.categories.contains(&building.category) || self.buildings.contains(&building.id)
    }
}

impl VocationRules {
    /// The vocation `id`, if any.
    pub fn get(&self, id: &str) -> Option<&Vocation> {
        self.vocations.iter().find(|v| v.id == id)
    }

    /// Livres to adopt a vocation in a settlement of `kind`; `first` for the
    /// very first one.
    pub fn cost_for(&self, kind: SettlementKind, first: bool) -> u32 {
        let base = self.adoption_cost.get(&kind).copied().unwrap_or(0);
        if first {
            base * self.first_adoption_percent / 100
        } else {
            base
        }
    }
}
