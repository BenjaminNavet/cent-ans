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

impl Default for DifficultyRules {
    /// Fallback when `data/rules/difficulty.json` is absent; mirrors the file.
    fn default() -> Self {
        #[allow(clippy::too_many_arguments)]
        fn level(
            id: &str,
            label: &str,
            description: &str,
            income: [u32; 2],
            ai_upkeep_percent: u32,
            ai_recruit_cost_percent: u32,
            player_unrest: i32,
            ai_attitude_to_player: i32,
            ai_war_ratio_percent: u32,
            ai_morale_vs_player: i32,
        ) -> DifficultyLevelData {
            DifficultyLevelData {
                id: id.to_owned(),
                label: label.to_owned(),
                description: description.to_owned(),
                modifiers: DifficultyModifiers {
                    ai_income_percent: income[0],
                    player_income_percent: income[1],
                    ai_upkeep_percent,
                    ai_recruit_cost_percent,
                    player_unrest,
                    ai_attitude_to_player,
                    ai_war_ratio_percent,
                    ai_morale_vs_player,
                },
            }
        }
        DifficultyRules {
            default: "normal".to_owned(),
            levels: vec![
                level(
                    "easy",
                    "Facile",
                    "Pour découvrir la guerre de Cent Ans : vos coffres se remplissent mieux, les princes rivaux sont moins riches et moins belliqueux, et leurs troupes plient plus vite.",
                    [80, 115],
                    100,
                    100,
                    -5,
                    10,
                    120,
                    -5,
                ),
                level(
                    "normal",
                    "Normale",
                    "Le jeu tel qu'il est équilibré : aucune faveur, aucun handicap, ni pour vous ni pour l'IA.",
                    [100, 100],
                    100,
                    100,
                    0,
                    0,
                    100,
                    0,
                ),
                level(
                    "hard",
                    "Difficile",
                    "Les couronnes rivales lèvent davantage d'impôts, entretiennent leurs armées à moindre coût et vous regardent d'un œil plus hostile ; leurs soldats tiennent mieux face à vous.",
                    [120, 100],
                    90,
                    100,
                    3,
                    -10,
                    90,
                    5,
                ),
                level(
                    "very_hard",
                    "Très difficile",
                    "Pour les connétables aguerris : l'IA est riche, recrute et entretient ses osts à bon compte et vous déclare la guerre au moindre signe de faiblesse ; vos revenus sont maigres et vos provinces promptes à gronder.",
                    [140, 90],
                    80,
                    85,
                    6,
                    -20,
                    80,
                    10,
                ),
            ],
            description: None,
        }
    }
}
