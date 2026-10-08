//! `CampaignSim` chronicle API (spec M10 § 3), in a secondary `#[godot_api]`
//! block so that each milestone keeps its own file.

use data_model::{EventId, FactionId, ProvinceId};
use godot::prelude::*;
use sim_campaign::Order;

use crate::campaign_sim::{CampaignSim, Ctx, CtxMut};

#[godot_api(secondary)]
impl CampaignSim {
    /// Decisions waiting for the player, oldest first:
    /// `[{id, event, title, text, historical, options[{index, text,
    /// effects_text}], expires_in, province, province_name, presentation}]`;
    /// `presentation` (FK1) is `map` (incident posed on the map) or `dialog`.
    #[func]
    fn get_pending_decisions(&self) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
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
                    "presentation" => view.presentation.as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// Enables or disables new chronicle events (controlled experiments such
    /// as the smoke test's economy step); pending decisions still expire.
    #[func]
    fn set_chronicle_enabled(&mut self, enabled: bool) {
        if self.refuse_while_turn_pending("set_chronicle_enabled") {
            return;
        }
        if let Some(state) = &mut self.state {
            state.chronicle.disabled = !enabled;
        }
    }

    /// Answers a pending decision (same as the `choose_event_option` order).
    #[func]
    fn choose_event_option(&mut self, decision: i64, option: i64) -> VarDictionary {
        self.run_order("choose_event_option", || {
            Ok(Order::ChooseEventOption {
                decision: decision.max(0) as u32,
                option: option.max(0) as usize,
            })
        })
    }

    /// Staging (UI tests, screenshots, FK5): offers `event` to the player as
    /// a pending decision in `province` (`""`: none); returns its id, -1 if
    /// refused (unknown event or province, turn being resolved).
    #[func]
    fn debug_offer_decision(&mut self, event: GString, province: GString) -> i64 {
        if self.refuse_while_turn_pending("debug_offer_decision") {
            return -1;
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return -1;
        };
        let Ok(event) = EventId::new(event.to_string()) else {
            return -1;
        };
        let province = match province.to_string().as_str() {
            "" => None,
            id => match ProvinceId::new(id) {
                Ok(p) => Some(p),
                Err(_) => return -1,
            },
        };
        state
            .debug_offer_decision(data, &event, province.as_ref())
            .map_or(-1, i64::from)
    }

    /// A6 (U6/U7): how much each journal entry concerns the player, one key
    /// per entry (`player`, `related`, `neighbor`, `far`); `far` entries go to
    /// the folded « Monde » tab and are never pushed as letters. Entries are
    /// the turn journal dictionaries (`faction`, `province`, `public`).
    #[func]
    fn classify_news(&self, events: VarArray) -> PackedStringArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return events
                .iter_shared()
                .map(|_| GString::from("player"))
                .collect();
        };
        let relevance =
            sim_campaign::news_relevance::NewsRelevance::new(state, data, state.player_faction());
        let text = |dict: &VarDictionary, keys: &[&str]| -> String {
            keys.iter()
                .find_map(|key| dict.get(*key).map(|v| v.to_string()))
                .unwrap_or_default()
        };
        events
            .iter_shared()
            .map(|entry| {
                let Ok(dict) = entry.try_to::<VarDictionary>() else {
                    return GString::from("far");
                };
                let faction = FactionId::new(text(&dict, &["faction", "faction_id"])).ok();
                let province = ProvinceId::new(text(&dict, &["province", "province_id"])).ok();
                let public = dict.get("public").is_some_and(|v| v.booleanize());
                GString::from(
                    relevance
                        .classify(faction.as_ref(), province.as_ref(), public)
                        .as_str(),
                )
            })
            .collect()
    }
}
