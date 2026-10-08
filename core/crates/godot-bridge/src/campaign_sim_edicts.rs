//! `CampaignSim` API of lot C4 (regional edicts, province panel selector),
//! kept in its own secondary `#[godot_api]` block (ADR 0002). The
//! `set_edict` order goes through `submit_order({"type": "set_edict",
//! "province": …, "edict": …})`.

use godot::prelude::*;

use crate::campaign_sim::{effects_array, CampaignSim, Ctx};

#[godot_api(secondary)]
impl CampaignSim {
    /// Edict in effect in `province`: `{edict, name, pending, pending_edict,
    /// pending_name, turns_left}`
    /// (`pending`: a delayed change has not taken hold yet); empty for an
    /// unknown province.
    #[func]
    fn get_province_edict(&self, province: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Ok(id) = data_model::ProvinceId::new(province.to_string()) else {
            return VarDictionary::new();
        };
        if state.province_state(&id).is_none() {
            return VarDictionary::new();
        }
        let edict = state.province_edict(data, &id);
        let definition = data.edicts.get(&edict);
        let pending = state.pending_edict(data, &id);
        let pending_name = pending
            .as_ref()
            .and_then(|(p, _)| data.edicts.get(p))
            .map_or("", |e| e.name.display.as_str());
        vdict! {
            "edict" => edict.as_str(),
            "name" => definition.map_or(edict.as_str(), |e| e.name.display.as_str()),
            "pending" => pending.is_some(),
            "pending_edict" => pending.as_ref().map_or("", |(p, _)| p.as_str()),
            "pending_name" => pending_name,
            "turns_left" => pending.as_ref().map_or(0, |(_, t)| *t),
        }
    }

    /// Every edict for `province` as its controller sees it, in id order:
    /// `[{id, name, description, delay_turns, effects[{kind, value, mode,
    /// unit_category, class}], available, reason, current, active,
    /// sources[]}]`. `current`: chosen (even if still pending); `active`:
    /// actually in effect right now.
    #[func]
    fn get_edict_options(&self, province: GString) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        let Ok(id) = data_model::ProvinceId::new(province.to_string()) else {
            return VarArray::new();
        };
        state
            .edict_options(data, &id)
            .into_iter()
            .filter_map(|option| {
                let edict = data.edicts.get(&option.edict)?;
                let sources: PackedStringArray = edict.sources.iter().map(GString::from).collect();
                Some(
                    vdict! {
                        "id" => edict.id.as_str(),
                        "name" => option.name.as_str(),
                        "description" => edict.description.as_str(),
                        "delay_turns" => edict.delay_turns,
                        "effects" => &effects_array(&edict.effects),
                        "available" => option.available,
                        "reason" => option.reason.as_deref().unwrap_or(""),
                        "current" => option.current,
                        "active" => option.active,
                        "sources" => &sources,
                    }
                    .to_variant(),
                )
            })
            .collect()
    }
}
