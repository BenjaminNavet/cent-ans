//! Line of sight on the campaign map (fog of war, sight
//! radius), mirroring `data/schemas/vision_rules.schema.json`
//! (`data/rules/vision.json`).
//!
//! The radii themselves (`vision_army_km`,
//! `vision_settlement_km`) live in `data/movement/rules.json`
//! ([`crate::entities::movement::FreeMovementRules`]); this file keeps the
//! alliance rule, how a province counts as seen and the soft edge of the
//! sight discs.

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/vision.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisionRules {
    /// Legacy (province steps): no longer read, see `movement`.
    #[serde(default)]
    pub controlled_range: u32,
    /// Legacy (province steps): no longer read, see `movement`.
    #[serde(default)]
    pub army_range: u32,
    /// Legacy (province steps): no longer read, see `movement`.
    #[serde(default)]
    pub general_bonus: u32,
    /// Whether allies (and vassals/overlords) share their armies' and
    /// settlements' sight.
    pub share_allied_vision: bool,
    /// Share of a province's land (percent of its cells) that must be seen
    /// for the province to count as visible.
    #[serde(default = "default_province_seen_percent")]
    pub province_seen_percent: u32,
    /// Whether every land cell of a province controlled by the faction (or
    /// a lending ally) is seen, as in Total War.
    #[serde(default = "default_own_provinces_visible")]
    pub own_provinces_visible: bool,
    /// Soft edge of a sight disc in the vision mask, in kilometres: the
    /// coverage crosses 50 % exactly at the radius.
    #[serde(default = "default_edge_feather_km")]
    pub edge_feather_km: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_own_provinces_visible() -> bool {
    true
}

fn default_edge_feather_km() -> f64 {
    3.0
}

fn default_province_seen_percent() -> u32 {
    25
}

crate::bundled_rules!(VisionRules, "rules/vision.json", default);
