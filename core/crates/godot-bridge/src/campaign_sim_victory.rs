//! `CampaignSim` objectives and outcome API (M10).

use data_model::FactionId;
use godot::prelude::*;

use crate::campaign_sim::{CampaignSim, Ctx, CtxMut};
use sim_campaign::victory::VictoryLength;

#[godot_api(secondary)]
impl CampaignSim {
    /// `[{id, title, description, done, progress}]` for `faction`.
    #[func]
    fn get_objectives(&self, faction: GString) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarArray::new();
        };
        state
            .objectives(data, &faction)
            .iter()
            .map(|o| {
                vdict! {
                    "id" => o.id.as_str(),
                    "title" => o.title.as_str(),
                    "description" => o.description.as_str(),
                    "done" => o.done,
                    "progress" => o.progress.as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// Chooses the campaign length, `"short"` or `"long"` (ADR 0332); the
    /// objectives and the end year follow. Returns false for another word.
    #[func]
    fn set_victory_length(&mut self, length: GString) -> bool {
        let length = match length.to_string().as_str() {
            "short" => VictoryLength::Short,
            "long" => VictoryLength::Long,
            _ => return false,
        };
        let Some(CtxMut { state, .. }) = self.ctx_mut() else {
            return false;
        };
        state.set_victory_length(length);
        self.revision += 1;
        true
    }

    /// `{state: "ongoing"|"victory"|"defeat"|"ended", text, score, turn}`;
    /// `score` is the current score while the campaign goes on, with
    /// `hold_turns` and `victory_streak` (seasons all objectives have held, F9).
    #[func]
    fn get_outcome(&self) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        match &state.outcome {
            Some(outcome) => {
                let key = serde_json::to_value(outcome.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                vdict! {
                    "state" => key.as_str(),
                    "text" => outcome.text_fr.as_str(),
                    "score" => outcome.score,
                    "turn" => i64::from(outcome.turn),
                }
            }
            None => {
                let hold = state.victory_hold_turns(data, state.player_faction());
                vdict! {
                    "state" => "ongoing",
                    "text" => "",
                    "score" => state.campaign_score(data, state.player_faction()),
                    "turn" => i64::from(state.turn()),
                    "hold_turns" => i64::from(hold),
                    "victory_streak" => i64::from(state.victory_streak),
                }
            }
        }
    }
}
