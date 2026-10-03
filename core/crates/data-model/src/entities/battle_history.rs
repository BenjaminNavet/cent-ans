//! Battle history bounds (chantier TB, lot « historique des batailles »),
//! mirroring `data/schemas/battle_history_rules.schema.json`
//! (`data/rules/battle_history.json`).
//!
//! The campaign state keeps the recent land battles (turn, place, sides,
//! losses) so that the map can mark the battlefields after a reload
//! (`sim-campaign::battle_history`, ADR 0157 « révision »). No game rule
//! reads the history: the file only bounds what is kept and saved.

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/battle_history.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BattleHistoryRules {
    /// A battle is forgotten once it is this many turns old (a battle of
    /// turn `t` is kept while `turn - t < max_age_turns`); 0 keeps nothing.
    pub max_age_turns: u32,
    /// Most battles kept at once; the oldest go first.
    pub max_records: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for BattleHistoryRules {
    /// Fallback when `data/rules/battle_history.json` is absent; kept equal
    /// to that file.
    fn default() -> Self {
        BattleHistoryRules {
            max_age_turns: 8,
            max_records: 256,
            description: None,
        }
    }
}
