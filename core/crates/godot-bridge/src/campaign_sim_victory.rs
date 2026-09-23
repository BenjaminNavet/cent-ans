//! `CampaignSim` objectives and outcome API (M10).

use data_model::FactionId;
use godot::prelude::*;

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {
    /// `[{id, title, description, done, progress}]` for `faction`.
    #[func]
    fn get_objectives(&self, faction: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
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

    /// `{state: "ongoing"|"victory"|"defeat"|"ended", text, score, turn}`;
    /// `score` is the current score while the campaign goes on.
    #[func]
    fn get_outcome(&self) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
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
            None => vdict! {
                "state" => "ongoing",
                "text" => "",
                "score" => state.campaign_score(data, state.player_faction()),
                "turn" => i64::from(state.turn()),
            },
        }
    }
}
