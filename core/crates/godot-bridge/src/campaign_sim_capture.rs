//! `CampaignSim` API of the fate of captured places (lot TW2-T1, ADR 0101),
//! in a secondary `#[godot_api]` block.

use godot::prelude::*;
use sim_campaign::{CaptureOutcome, Order};

use crate::campaign_sim::{CampaignSim, Ctx, CtxMut};

#[godot_api(secondary)]
impl CampaignSim {
    /// Captures waiting for the player's choice, shaped like the chronicle
    /// decisions (`ChronicleWindow` shows both): `[{id, kind: "province_captured" (siege vignette),
    /// event, title, text, historical, settlement, settlement_name, province,
    /// province_name, previous_name, expires_in, kind_label, options[{index,
    /// outcome, text, effects_text, allowed, reason, gold, unrest,
    /// population_percent, devastation, buildings_lost, fortification_loss,
    /// becomes_ruin, unit_experience}]}]`. `index` is the position in
    /// `CaptureOutcome::ALL` (0 occupy, 1 ransom, 2 sack, 3 raze).
    #[func]
    fn get_pending_captures(&self) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        state
            .capture_decision_views(data, state.player_faction())
            .iter()
            .map(|view| {
                let options: VarArray = view
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| {
                        let effects = &option.effects;
                        vdict! {
                            "index" => index as i64,
                            "outcome" => option.outcome.id(),
                            "text" => option.label.as_str(),
                            "effects_text" => option.effects_text.as_str(),
                            "allowed" => option.allowed,
                            "reason" => option.reason.as_deref().unwrap_or(""),
                            "gold" => effects.gold,
                            "unrest" => i64::from(effects.unrest),
                            "population_percent" => i64::from(effects.population_percent),
                            "devastation" => i64::from(effects.devastation),
                            "buildings_lost" => effects.buildings_lost.len() as i64,
                            "fortification_loss" => i64::from(effects.fortification_loss),
                            "becomes_ruin" => effects.becomes_ruin,
                            "unit_experience" => i64::from(effects.unit_experience),
                        }
                        .to_variant()
                    })
                    .collect();
                vdict! {
                    "id" => i64::from(view.id),
                    "kind" => "province_captured",
                    "event" => "capture",
                    "kind_label" => "✠ Sort de la place prise",
                    "title" => view.title.as_str(),
                    "text" => view.text.as_str(),
                    "historical" => false,
                    "settlement" => view.settlement.as_str(),
                    "settlement_name" => view.settlement_name.as_str(),
                    "province" => view.province.as_str(),
                    "province_name" => view.province_name.as_str(),
                    "previous_name" => view.previous_name.as_str(),
                    "expires_in" => 1_i64,
                    "options" => &options,
                }
                .to_variant()
            })
            .collect()
    }

    /// Decides the fate of pending capture `decision` (`outcome`: `occupy`,
    /// `ransom`, `sack` or `raze`); same as the `choose_capture_outcome` order.
    #[func]
    fn choose_capture_outcome(&mut self, decision: i64, outcome: GString) -> VarDictionary {
        self.run_order("choose_capture_outcome", || {
            let Some(outcome) = CaptureOutcome::from_id(&outcome.to_string()) else {
                return Err(format!("sort inconnu : {outcome}"));
            };
            Ok(Order::ChooseCaptureOutcome {
                decision: decision.max(0) as u32,
                outcome,
            })
        })
    }

    /// Staging (UI tests): the player takes `place` (a settlement id, or a
    /// province id standing for its city) at once through the common capture
    /// path; `false` when refused (unknown place, already the player's).
    #[func]
    fn debug_capture_place(&mut self, place: GString) -> bool {
        if self.refuse_while_turn_pending("debug_capture_place") {
            return false;
        }
        let Some(CtxMut { state, data }) = self.ctx_mut() else {
            return false;
        };
        let raw = place.to_string();
        let place = match (
            data_model::SettlementId::new(raw.clone()),
            data_model::ProvinceId::new(raw),
        ) {
            (Ok(id), _) => sim_campaign::Place::Settlement(id),
            (_, Ok(id)) => sim_campaign::Place::Province(id),
            _ => return false,
        };
        let Ok(settlement) = state.resolve_place(&place) else {
            return false;
        };
        let player = state.player_faction().clone();
        sim_campaign::capture::debug_capture(state, data, &settlement, &player)
    }
}
