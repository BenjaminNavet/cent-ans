//! Short-term campaign missions (lot NT3, ADR 0127).
//!
//! The player's faction (never the AI) receives 1 or 2 missions drawn from
//! `data/missions.json` according to its situation, resolved at the end of
//! each turn: success grants the reward, the deadline passing is a failure
//! with a small prestige loss at most.

use data_model::{BuildingId, FactionId, GameData, MissionKind, MissionReward, ProvinceId, SettlementId};
use serde::{Deserialize, Serialize};

use crate::events::GameEvent;
use crate::state::CampaignState;

/// Missions of the player's faction (absent from older saves: empty).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionsState {
    /// Faction the missions belong to (the player's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<FactionId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active: Vec<Mission>,
    #[serde(default)]
    pub next_id: u32,
    /// Turn of the last success or failure (offer cooldown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_closed_turn: Option<u32>,
    /// Notices of the last resolution, for the interface's toasts; never saved.
    #[serde(skip)]
    pub notices: Vec<MissionNotice>,
}

/// One active mission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mission {
    pub id: u32,
    pub template: String,
    pub kind: MissionKind,
    pub title: String,
    pub objective: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<SettlementId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building: Option<BuildingId>,
    /// Number asked for (battles, units); 1 otherwise.
    pub count: u32,
    /// Battles won or units recruited since the mission was given.
    #[serde(default)]
    pub progress: u32,
    pub issued_turn: u32,
    /// Turn at the start of which the mission is failed (or, for
    /// `hold_place`, fulfilled).
    pub deadline_turn: u32,
    pub reward: MissionReward,
    /// `conclude_treaty`: the faction's treaties when the mission was given.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub baseline: Vec<String>,
}

/// What happened to a mission, for a toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    Offered,
    Succeeded,
    Failed,
}

/// A toast for the interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionNotice {
    pub kind: NoticeKind,
    pub text: String,
}

/// A mission as shown by the objectives panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionView {
    pub id: u32,
    pub kind: String,
    pub title: String,
    pub objective: String,
    pub progress: String,
    pub turns_left: u32,
    pub deadline: String,
    pub reward: String,
}

/// End-of-turn step: progress, success, failure, new offers.
pub fn resolve_missions(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let _ = (state, data, events);
}

/// A battle was won by `faction` (hook of the battle outcome).
pub(crate) fn note_battle_won(state: &mut CampaignState, faction: &FactionId) {
    let _ = (state, faction);
}

/// `units` were recruited or hired by `faction` (hook of the player's orders).
pub(crate) fn note_units_recruited(state: &mut CampaignState, faction: &FactionId, units: u32) {
    let _ = (state, faction, units);
}

impl CampaignState {
    /// The player's active missions, for the objectives panel.
    pub fn missions(&self, data: &GameData) -> Vec<MissionView> {
        let _ = data;
        Vec::new()
    }

    /// Notices (offered, succeeded, failed) of the last resolution.
    pub fn mission_notices(&self) -> &[MissionNotice] {
        &self.missions.notices
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "NT3 skeleton"]
    fn missions_are_offered_to_the_player() {}
}
