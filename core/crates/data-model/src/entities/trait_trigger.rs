//! Data-driven trait triggers (`data/rules/trait_triggers.json`,
//! `trait_triggers.schema.json`, WH charsb, ADR 0284): which conditions grant
//! an acquired trait to a character. Evaluated by `sim-campaign::trait_triggers`.

use serde::{Deserialize, Serialize};

use crate::entities::skill::SkillRole;
use crate::ids::TraitId;

/// Conditions of a trigger; every one that is present must hold.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerCondition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battles_fought_min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battles_won_min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sieges_won_min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raids_led_min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_min: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piety_min: Option<u8>,
    /// The character's faction treasury is below this (livres).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treasury_below: Option<i64>,
    /// The character's last battle was a victory.
    #[serde(default)]
    pub last_battle_won: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraitTrigger {
    /// Trait granted (`data/traits`).
    #[serde(rename = "trait")]
    pub trait_id: TraitId,
    /// Roles evaluated (any one suffices); empty: every character.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<SkillRole>,
    pub when: TriggerCondition,
    /// Journal line, `{name}` is the character.
    pub text_fr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraitTriggerRules {
    pub triggers: Vec<TraitTrigger>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

crate::bundled_rules!(TraitTriggerRules, "rules/trait_triggers.json", default);
