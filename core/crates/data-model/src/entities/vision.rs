//! Line of sight on the campaign map (lot C1 fog of war, lot M5a sight
//! radius), mirroring `data/schemas/vision_rules.schema.json`
//! (`data/rules/vision.json`).
//!
//! Since lot M5a the radii themselves (`vision_army_km`,
//! `vision_settlement_km`) live in `data/movement/rules.json`
//! ([`crate::entities::movement::FreeMovementRules`]); this file keeps the
//! alliance rule and how a province counts as seen.

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/vision.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisionRules {
    /// Legacy (lot C1, province steps): no longer read since lot M5a.
    #[serde(default)]
    pub controlled_range: u32,
    /// Legacy (lot C1, province steps): no longer read since lot M5a.
    #[serde(default)]
    pub army_range: u32,
    /// Legacy (lot C1, province steps): no longer read since lot M5a.
    #[serde(default)]
    pub general_bonus: u32,
    /// Whether allies (and vassals/overlords) share their armies' and
    /// settlements' sight.
    pub share_allied_vision: bool,
    /// Share of a province's land (percent of its cells) that must be seen
    /// for the province to count as visible (lot M5a).
    #[serde(default = "default_province_seen_percent")]
    pub province_seen_percent: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_province_seen_percent() -> u32 {
    25
}

impl Default for VisionRules {
    /// Fallback when `data/rules/vision.json` is absent: allies shared, a
    /// quarter of a province seen makes it visible.
    fn default() -> Self {
        VisionRules {
            controlled_range: 0,
            army_range: 0,
            general_bonus: 0,
            share_allied_vision: true,
            province_seen_percent: default_province_seen_percent(),
            description: None,
        }
    }
}
