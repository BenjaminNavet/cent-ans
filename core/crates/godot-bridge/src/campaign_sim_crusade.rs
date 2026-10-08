//! `CampaignSim` crusade view (lot JR1, ADR 0165): the « Ferveur » section
//! of the faction panel. The command is the generic order
//! `{"type": "preach_passage"}`.

use godot::prelude::*;

use crate::campaign_sim::{CampaignSim, Ctx};

#[godot_api(secondary)]
impl CampaignSim {
    /// The player's fervour: `{fervor, floor, alms, alms_last_turn,
    /// changes[{cause, delta}], zeal_morale, desertion_percent,
    /// zeal_high_threshold, zeal_low_threshold, zeal_high_morale,
    /// zeal_low_morale, desertion_threshold, desertion_men_percent, target_taken,
    /// target_name, passage_cost, passage_cooldown, passage_available,
    /// passage_blocker, passage_units, passage_delay, pending[{turns_left,
    /// port, port_name, units}]}` (French texts; `passage_blocker` is `""`
    /// when the passage can be preached). Empty unless the player leads the
    /// crusader faction of `data/rules/crusade.json`.
    #[func]
    fn get_crusade(&self) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Some(view) = sim_campaign::crusade::crusade_view(state, data, state.player_faction())
        else {
            return VarDictionary::new();
        };
        let changes: VarArray = view
            .changes
            .iter()
            .map(|c| {
                vdict! {
                    "cause" => c.cause.as_str(),
                    "delta" => i64::from(c.delta),
                }
                .to_variant()
            })
            .collect();
        let pending: VarArray = view
            .pending
            .iter()
            .map(|p| {
                vdict! {
                    "turns_left" => i64::from(p.turns_left),
                    "port" => p.port.as_str(),
                    "port_name" => p.port_name.as_str(),
                    "units" => i64::from(p.units),
                }
                .to_variant()
            })
            .collect();
        vdict! {
            "fervor" => i64::from(view.fervor),
            "floor" => i64::from(view.floor),
            "alms" => view.alms,
            "alms_last_turn" => view.alms_last_turn,
            "changes" => &changes,
            "zeal_morale" => i64::from(view.zeal_morale),
            "desertion_percent" => i64::from(view.desertion_percent),
            "zeal_high_threshold" => i64::from(view.zeal_high_threshold),
            "zeal_low_threshold" => i64::from(view.zeal_low_threshold),
            "zeal_high_morale" => i64::from(view.zeal_high_morale),
            "zeal_low_morale" => i64::from(view.zeal_low_morale),
            "desertion_threshold" => i64::from(view.desertion_threshold),
            "desertion_men_percent" => i64::from(view.desertion_men_percent),
            "target_taken" => view.target_taken,
            "target_name" => view.target_name.as_str(),
            "passage_cost" => view.passage_cost,
            "passage_cooldown" => i64::from(view.passage_cooldown),
            "passage_available" => view.passage_available,
            "passage_blocker" => view.passage_blocker.as_str(),
            "passage_units" => i64::from(view.passage_units),
            "passage_delay" => i64::from(view.passage_delay),
            "pending" => &pending,
        }
    }
}
