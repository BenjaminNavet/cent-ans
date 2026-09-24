//! `CampaignSim` API of lot C4 (regional edicts, province panel selector),
//! kept in its own secondary `#[godot_api]` block (ADR 0002). The
//! `set_edict` order goes through `submit_order({"type": "set_edict",
//! "province": …, "edict": …})`.

use godot::prelude::*;

use crate::campaign_sim::{effects_array, CampaignSim};

#[godot_api(secondary)]
impl CampaignSim {
    /// Edict in effect in `province`: `{edict, name, pending, delay_turns}`
    /// (`pending`: a delayed change has not taken hold yet); empty for an
    /// unknown province.
    #[func]
    fn get_province_edict(&self, province: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
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
        vdict! {
            "edict" => edict.as_str(),
            "name" => definition.map_or(edict.as_str(), |e| e.name.display.as_str()),
            "pending" => state.edict_pending(data, &id),
            "delay_turns" => definition.map_or(0, |e| e.delay_turns),
        }
    }

    /// Every edict for `province` as its controller sees it, in id order:
    /// `[{id, name, description, delay_turns, effects[{kind, value, mode,
    /// unit_category, class}], available, reason, current, active,
    /// sources[]}]`. `current`: chosen (even if still pending); `active`:
    /// actually in effect right now.
    #[func]
    fn get_edict_options(&self, province: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
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
