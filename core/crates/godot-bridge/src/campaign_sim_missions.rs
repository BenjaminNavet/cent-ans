//! `CampaignSim` short-term missions (lot NT3, ADR 0127): the « Missions »
//! section of the objectives panel and the toasts of the last resolution.

use godot::prelude::*;

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {
    /// The player's active missions: `[{id, kind, title, objective, progress,
    /// progress_ratio, turns_left, deadline, reward, province}]` (French
    /// labels; `province` is `""` when the mission has no target province).
    #[func]
    fn get_missions(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        state
            .missions(data)
            .iter()
            .map(|m| {
                vdict! {
                    "id" => i64::from(m.id),
                    "kind" => m.kind.as_str(),
                    "title" => m.title.as_str(),
                    "objective" => m.objective.as_str(),
                    "progress" => m.progress.as_str(),
                    "progress_ratio" => m.progress_ratio,
                    "turns_left" => i64::from(m.turns_left),
                    "deadline" => m.deadline.as_str(),
                    "reward" => m.reward.as_str(),
                    "province" => m.province.as_ref().map_or("", |p| p.as_str()),
                }
                .to_variant()
            })
            .collect()
    }

    /// Notices of the last end of turn: `[{kind: "offered"|"succeeded"|
    /// "failed", mission, text}]`, for the toasts.
    #[func]
    fn get_mission_notices(&self) -> VarArray {
        let Some(state) = &self.state else {
            return VarArray::new();
        };
        state
            .mission_notices()
            .iter()
            .map(|n| {
                vdict! {
                    "kind" => n.kind.key(),
                    "mission" => i64::from(n.mission),
                    "text" => n.text.as_str(),
                }
                .to_variant()
            })
            .collect()
    }
}
