//! `CampaignSim` API of royal acts and captain hiring (WH chars, ADR 0276 and
//! 0277), in its own secondary `#[godot_api]` block (ADR 0002). The orders go
//! through `submit_order({"type": "royal_act", "act": …})` and
//! `submit_order({"type": "hire_captain", "settlement": …})`.

use godot::prelude::*;

use crate::campaign_sim::{effects_array, CampaignSim, Ctx};

#[godot_api(secondary)]
impl CampaignSim {
    /// Royal acts open to `faction`, in id order: `[{id, name, description,
    /// cost_prestige, cost_livres, cooldown_turns, duration_turns,
    /// gain_prestige, gain_piety, effects[{kind, value, mode, ...}],
    /// available, reason, cooldown_left, active_left, sources[]}]`.
    #[func]
    fn get_royal_acts(&self, faction: GString) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        let Ok(faction) = data_model::FactionId::new(faction.to_string()) else {
            return VarArray::new();
        };
        sim_campaign::royal_acts::royal_act_options(state, data, &faction)
            .into_iter()
            .filter_map(|option| {
                let act = data.royal_acts.get(&option.id)?;
                let sources: PackedStringArray = act.sources.iter().map(GString::from).collect();
                Some(
                    vdict! {
                        "id" => act.id.as_str(),
                        "name" => act.name.display.as_str(),
                        "description" => act.description.as_str(),
                        "cost_prestige" => i64::from(act.cost_prestige),
                        "cost_livres" => act.cost_livres,
                        "cooldown_turns" => i64::from(act.cooldown_turns),
                        "duration_turns" => i64::from(act.duration_turns),
                        "gain_prestige" => i64::from(act.gain_prestige),
                        "gain_piety" => i64::from(act.gain_piety),
                        "effects" => &effects_array(&act.effects),
                        "available" => option.available,
                        "reason" => option.reason.as_deref().unwrap_or(""),
                        "cooldown_left" => i64::from(option.cooldown_left),
                        "active_left" => i64::from(option.active_left),
                        "sources" => &sources,
                    }
                    .to_variant(),
                )
            })
            .collect()
    }

    /// Captain hiring in the capital's city of `faction`: `{cost, count, cap,
    /// cooldown_left, can_hire, reason, settlement}`; empty for an unknown
    /// faction.
    #[func]
    fn get_captain_info(&self, faction: GString) -> VarDictionary {
        let Some(Ctx { state, .. }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Ok(faction) = data_model::FactionId::new(faction.to_string()) else {
            return VarDictionary::new();
        };
        let Some(capital) = state.factions.get(&faction).map(|f| f.capital.clone()) else {
            return VarDictionary::new();
        };
        let Some(city) = state.province_city_id(&capital).cloned() else {
            return VarDictionary::new();
        };
        let refusal = sim_campaign::captains::hire_refusal(state, &faction, &city);
        vdict! {
            "cost" => sim_campaign::captains::captain_cost(state, &faction),
            "count" => i64::from(sim_campaign::captains::captain_count(state, &faction)),
            "cap" => i64::from(sim_campaign::captains::captain_cap(state, &faction)),
            "cooldown_left" => i64::from(sim_campaign::captains::cooldown_left(state, &faction)),
            "can_hire" => refusal.is_none(),
            "reason" => refusal.map(|e| e.to_string()).unwrap_or_default().as_str(),
            "settlement" => city.as_str(),
        }
    }
}
