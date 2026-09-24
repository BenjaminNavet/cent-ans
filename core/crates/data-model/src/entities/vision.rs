//! Line of sight on the campaign map (lot C1, fog of war), mirroring
//! `data/schemas/vision_rules.schema.json` (`data/rules/vision.json`).

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/vision.json`: how far a faction sees on the
/// campaign map. Ranges are counted in land steps on the province graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisionRules {
    /// Land steps seen around every controlled province (0 = the province only).
    pub controlled_range: u32,
    /// Land steps seen around the province of every army.
    pub army_range: u32,
    /// Extra steps for an army led by a general (scouts, outriders).
    #[serde(default)]
    pub general_bonus: u32,
    /// Whether allies (and vassals/overlords) share their provinces and armies.
    pub share_allied_vision: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for VisionRules {
    /// Fallback when `data/rules/vision.json` is absent: one step around
    /// provinces and armies, allies shared.
    fn default() -> Self {
        VisionRules {
            controlled_range: 1,
            army_range: 1,
            general_bonus: 0,
            share_allied_vision: true,
            description: None,
        }
    }
}
