//! `CampaignSim` chronicle API (spec M10 § 3), in a secondary `#[godot_api]`
//! block so that each milestone keeps its own file.

use godot::prelude::*;
use sim_campaign::Order;

use crate::campaign_sim::{order_result, CampaignSim};

#[godot_api(secondary)]
impl CampaignSim {
    /// Decisions waiting for the player, oldest first:
    /// `[{id, event, title, text, historical, options[{index, text,
    /// effects_text}], expires_in, province, province_name}]`.
    #[func]
    fn get_pending_decisions(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        state
            .decision_views(data, state.player_faction())
            .iter()
            .map(|view| {
                let options: VarArray = view
                    .options
                    .iter()
                    .map(|option| {
                        vdict! {
                            "index" => option.index as i64,
                            "text" => option.text.as_str(),
                            "effects_text" => option.effects_text.as_str(),
                        }
                        .to_variant()
                    })
                    .collect();
                vdict! {
                    "id" => i64::from(view.id),
                    "event" => view.event.as_str(),
                    "title" => view.title.as_str(),
                    "text" => view.text.as_str(),
                    "historical" => view.historical,
                    "options" => &options,
                    "expires_in" => i64::from(view.expires_in),
                    "province" => view.province.as_ref().map_or("", |p| p.as_str()),
                    "province_name" => view.province_name.as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// Answers a pending decision (same as the `choose_event_option` order).
    #[func]
    fn choose_event_option(&mut self, decision: i64, option: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return order_result(Err("aucune campagne en cours".to_owned()));
        };
        let order = Order::ChooseEventOption {
            decision: decision.max(0) as u32,
            option: option.max(0) as usize,
        };
        order_result(state.submit_order(data, order).map_err(|e| e.to_string()))
    }
}
