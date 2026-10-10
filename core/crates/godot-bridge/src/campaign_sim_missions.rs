//! `CampaignSim` short-term missions (ADR 0127): the « Missions »
//! section of the objectives panel and the toasts of the last resolution.

use godot::prelude::*;

use crate::campaign_sim::{CampaignSim, Ctx};
use sim_campaign::Order;

#[godot_api(secondary)]
impl CampaignSim {
    /// The player's active missions: `[{id, kind, title, objective, progress,
    /// progress_ratio, turns_left, deadline, reward, province, faction_mission, source}]` (French
    /// labels; `province` is `""` when the mission has no target province).
    #[func]
    fn get_missions(&self) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
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
                    "faction_mission" => m.faction_mission,
                    "source" => m.source.as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// Notices of the last end of turn: `[{kind: "offered"|"succeeded"|
    /// "failed", mission, text}]`, for the toasts.
    #[func]
    fn get_mission_notices(&self) -> VarArray {
        let Some(Ctx { state, .. }) = self.ctx() else {
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

    /// WR turn (ADR 0304): the pending mission offer, `{}` when none:
    /// `{id, turns_left, candidates: [{index, title, objective, duration,
    /// reward, faction_mission, source, province}]}`.
    #[func]
    fn get_mission_offer(&self) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Some(offer) = state.mission_offer(data) else {
            return VarDictionary::new();
        };
        let candidates: VarArray = offer
            .candidates
            .iter()
            .map(|c| {
                vdict! {
                    "index" => c.index as i64,
                    "title" => c.title.as_str(),
                    "objective" => c.objective.as_str(),
                    "duration" => i64::from(c.duration),
                    "reward" => c.reward.as_str(),
                    "faction_mission" => c.faction_mission,
                    "source" => c.source.as_str(),
                    "province" => c.province.as_ref().map_or("", |p| p.as_str()),
                }
                .to_variant()
            })
            .collect();
        vdict! {
            "id" => i64::from(state.mission_offer_turn()),
            "turns_left" => i64::from(offer.turns_left),
            "candidates" => candidates,
        }
    }

    /// Answers the pending mission offer: candidate `choice`, or a refusal
    /// when `choice` is negative (same as the `choose_mission` order).
    #[func]
    fn choose_mission(&mut self, choice: i64) -> VarDictionary {
        self.run_order("choose_mission", || {
            Ok(Order::ChooseMission {
                choice: usize::try_from(choice).ok(),
            })
        })
    }
}
