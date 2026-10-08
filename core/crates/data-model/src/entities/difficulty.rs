//! Campaign difficulty levels (lot DF1, ADR 0037), mirroring
//! `data/schemas/difficulty_rules.schema.json` (`data/rules/difficulty.json`).

use serde::{Deserialize, Serialize};

/// Numeric modifiers of one difficulty level. Every field is neutral at
/// 100 % (percentages) or 0 (offsets): the `normal` level changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifficultyModifiers {
    /// Income (taxes and trade) of the AI factions, per cent of normal.
    pub ai_income_percent: u32,
    /// Income of the player, per cent of normal.
    pub player_income_percent: u32,
    /// Upkeep of the AI armies and garrisons, per cent of normal.
    pub ai_upkeep_percent: u32,
    /// Recruitment cost of the AI units, per cent of normal.
    pub ai_recruit_cost_percent: u32,
    /// Points added to the unrest target in the player's provinces.
    pub player_unrest: i32,
    /// Points added to the attitude of every AI faction towards the player.
    pub ai_attitude_to_player: i32,
    /// Power ratio an AI demands before declaring war on the player, per
    /// cent of normal (below 100: more aggressive).
    pub ai_war_ratio_percent: u32,
    /// Morale bonus of the AI armies fighting the player (3D battle and
    /// auto-resolve alike).
    pub ai_morale_vs_player: i32,
}

impl DifficultyModifiers {
    /// The neutral modifiers of the `normal` level.
    pub const NEUTRAL: DifficultyModifiers = DifficultyModifiers {
        ai_income_percent: 100,
        player_income_percent: 100,
        ai_upkeep_percent: 100,
        ai_recruit_cost_percent: 100,
        player_unrest: 0,
        ai_attitude_to_player: 0,
        ai_war_ratio_percent: 100,
        ai_morale_vs_player: 0,
    };

    pub fn is_neutral(&self) -> bool {
        *self == Self::NEUTRAL
    }
}

impl Default for DifficultyModifiers {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

/// One level of `data/rules/difficulty.json` (no `deny_unknown_fields`:
/// serde does not support it beside `flatten`; the schema checks the keys).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DifficultyLevelData {
    /// `easy`, `normal`, `hard` or `very_hard`.
    pub id: String,
    /// French label ("Facile").
    pub label: String,
    /// Short French description for the tooltip.
    pub description: String,
    #[serde(flatten)]
    pub modifiers: DifficultyModifiers,
}

/// Contents of `data/rules/difficulty.json`: the four levels, easiest first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifficultyRules {
    /// Level offered by default at launch.
    pub default: String,
    pub levels: Vec<DifficultyLevelData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl DifficultyRules {
    /// The level `id`, if the file defines it.
    pub fn level(&self, id: &str) -> Option<&DifficultyLevelData> {
        self.levels.iter().find(|l| l.id == id)
    }

    /// Modifiers of level `id`; neutral when the level is unknown.
    pub fn modifiers(&self, id: &str) -> DifficultyModifiers {
        self.level(id).map(|l| l.modifiers).unwrap_or_default()
    }
}

crate::bundled_rules!(DifficultyRules, "rules/difficulty.json", default);
