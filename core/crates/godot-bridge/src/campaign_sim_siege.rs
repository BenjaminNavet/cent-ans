//! `CampaignSim` siege API (M8): assault odds for the army panel.

use godot::prelude::*;
use sim_campaign::ArmyId;

use crate::campaign_sim::{CampaignSim, Ctx};

#[godot_api(secondary)]
impl CampaignSim {
    /// Events produced by orders since the last `end_turn` (they will open
    /// the next turn's journal): `[{kind, text_fr, province, faction}]`.
    #[func]
    fn get_pending_events(&self) -> VarArray {
        let Some(Ctx { state, .. }) = self.ctx() else {
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

    /// `{available, odds, walls, breach, supplies, turns_left, engines,
    /// blocker}` for an army besieging a town; `available` is false
    /// otherwise. NT5: `engines` = `[{id, name, kind, ready, turns_left}]`
    /// (built on the spot, in building order), `blocker` = why the assault
    /// is refused (French, empty when it may be given).
    #[func]
    fn get_assault_odds(&self, army: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return vdict! { "available" => false };
        };
        let Some(id) = ArmyId::parse(&army.to_string()) else {
            return vdict! { "available" => false };
        };
        let Some((_, walls)) = state.assault_odds(data, &id) else {
            return vdict! { "available" => false };
        };
        // Q5: the same estimate as the assault's pre-battle screen.
        let odds = state
            .assault_win_chance(data, &id)
            .map_or(0, |chance| (chance * 100.0).round() as i64);
        let siege = state
            .armies
            .get(&id)
            .and_then(|a| a.settlement())
            .and_then(|s| state.settlement_state(s))
            .and_then(|s| s.siege.clone());
        let place = state.armies.get(&id).and_then(|a| a.settlement().cloned());
        let engines: VarArray = place
            .map(|p| state.siege_engines(data, &p))
            .unwrap_or_default()
            .iter()
            .map(|e| {
                let kind = serde_json::to_value(e.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                vdict! {
                    "id" => e.id.as_str(),
                    "name" => e.name.as_str(),
                    "kind" => kind.as_str(),
                    "ready" => e.ready,
                    "turns_left" => i64::from(e.turns_left),
                }
                .to_variant()
            })
            .collect();
        vdict! {
            "available" => true,
            "odds" => odds,
            "walls" => walls,
            "breach" => siege.as_ref().map_or(0, |s| i64::from(s.breach)),
            "supplies" => siege.as_ref().map_or(0, |s| i64::from(s.supplies)),
            "turns_left" => siege.as_ref().map_or(0, |s| i64::from(s.turns_left)),
            "engines" => &engines,
            "blocker" => state.assault_blocker(data, &id).unwrap_or_default().as_str(),
        }
    }

    /// NT5 (N6): regiments an army may hold at most (`data/rules/armies.json`).
    #[func]
    fn army_unit_cap(&self) -> i64 {
        self.data
            .as_ref()
            .map_or(40, |data| i64::from(data.army_rules.max_units))
    }
}
