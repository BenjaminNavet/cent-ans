//! Lot FK1: province scenes shown on the close campaign view
//! (`docs/design/2026-09-29-carte-vivante-folk.md` § 2.1.1).
//!
//! Purely visual, like the weather (ADR 0027): a pure function of the state
//! and the data, no RNG read, no rule effect.

use data_model::{GameData, ProvinceId, SettlementId};
use serde::{Deserialize, Serialize};

use crate::CampaignState;

/// Kind of scene staged by the renderer (closed list, mirrored by the
/// `map_scene` enum of the event schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneKind {
    Plague,
    Famine,
    Revolt,
    Devastation,
    Siege,
    Construction,
    Fair,
    Celebration,
    Flood,
    Muster,
}

/// One scene to stage near a settlement (or the province seat).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapScene {
    pub province: ProvinceId,
    pub settlement: Option<SettlementId>,
    pub kind: SceneKind,
    pub since_turn: u32,
    /// In [0, 1]: from the source value, decreasing with the scene's age.
    pub intensity: f32,
}

/// Scenes of the current state (FK1: skeleton, filled by the lot).
pub fn map_scenes(_state: &CampaignState, _data: &GameData) -> Vec<MapScene> {
    Vec::new()
}
