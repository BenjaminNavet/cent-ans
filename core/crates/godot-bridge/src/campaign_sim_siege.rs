//! `CampaignSim` siege API (M8): assault odds for the army panel.

use godot::prelude::*;
use sim_campaign::ArmyId;

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {
    /// Events produced by orders since the last `end_turn` (they will open
    /// the next turn's journal): `[{kind, text_fr, province, faction}]`.
    #[func]
    fn get_pending_events(&self) -> VarArray {
        let Some(state) = &self.state else {
            return VarArray::new();
        };
        state
            .pending_events
            .iter()
            .map(|event| {
                let kind = serde_json::to_value(event.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                vdict! {
                    "kind" => kind.as_str(),
                    "text_fr" => event.text_fr.as_str(),
                    "province" => event.province.as_ref().map_or("", |id| id.as_str()),
                    "faction" => event.faction.as_ref().map_or("", |id| id.as_str()),
                }
                .to_variant()
            })
            .collect()
    }

    /// `{available, odds, walls, breach, supplies, turns_left}` for an army
    /// besieging a town; `available` is false otherwise.
    #[func]
    fn get_assault_odds(&self, army: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return vdict! { "available" => false };
        };
        let Some(id) = ArmyId::parse(&army.to_string()) else {
            return vdict! { "available" => false };
        };
        let Some((odds, walls)) = state.assault_odds(data, &id) else {
            return vdict! { "available" => false };
        };
        let siege = state
            .armies
            .get(&id)
            .and_then(|a| state.province_state(&a.location))
            .and_then(|p| p.siege.clone());
        vdict! {
            "available" => true,
            "odds" => i64::from(odds),
            "walls" => walls,
            "breach" => siege.as_ref().map_or(0, |s| i64::from(s.breach)),
            "supplies" => siege.as_ref().map_or(0, |s| i64::from(s.supplies)),
            "turns_left" => siege.as_ref().map_or(0, |s| i64::from(s.turns_left)),
        }
    }
}
