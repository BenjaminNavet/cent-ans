//! `CampaignSim` API of H3 « La Table » (province diets), kept in its own
//! secondary `#[godot_api]` block (ADR 0002). The `set_diet` order goes
//! through `submit_order({"type": "set_diet", "province": …, "diet": …})`.
//! Documented in `docs/design/h3-h4-api.md`.

use data_model::{Diet, FactionId, ProvinceId};
use godot::prelude::*;

use crate::campaign_sim::{effects_array, CampaignSim};

fn requirements_dict(diet: &Diet) -> VarDictionary {
    let requirements = &diet.requirements;
    let resources: PackedStringArray = requirements
        .resources
        .iter()
        .map(|r| GString::from(r.as_str()))
        .collect();
    let buildings: PackedStringArray = requirements
        .any_building
        .iter()
        .map(|b| GString::from(b.as_str()))
        .collect();
    let terrains: PackedStringArray = requirements
        .terrains
        .iter()
        .map(|t| GString::from(t.key()))
        .collect();
    vdict! {
        "resources" => &resources,
        "coastal" => requirements.coastal,
        "technology" => requirements.technology.as_ref().map_or("", |t| t.as_str()),
        "any_building" => &buildings,
        "terrains" => &terrains,
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// `true` during the spring turn (Lent: meat and dairy diets are
    /// penalised, lean fish rewarded).
    #[func]
    fn is_lent(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.is_lent())
    }

    /// Diet eaten in `province`: `{diet, name, cost, changed_this_turn,
    /// lent_rule, winter_rule}` (`cost`: livres this season); empty for an
    /// unknown province.
    #[func]
    fn get_province_diet(&self, province: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some((id, p)) = ProvinceId::new(province.to_string())
            .ok()
            .and_then(|id| state.province_state(&id).map(|p| (id, p)))
        else {
            return VarDictionary::new();
        };
        let diet = state.province_diet(&id);
        let definition = data.diets.get(&diet);
        let changed = p.diet.as_ref().is_some_and(|c| {
            c.turn == state.turn() && state.province_controller(&id) == Some(&c.faction)
        });
        vdict! {
            "diet" => diet.as_str(),
            "name" => definition.map_or(diet.as_str(), |d| d.name.display.as_str()),
            "cost" => state.diet_cost(data, &id, &diet),
            "changed_this_turn" => changed,
            "lent_rule" => definition.map_or("none", |d| d.lent_rule.key()),
            "winter_rule" => definition.map_or("none", |d| d.winter_rule.key()),
        }
    }

    /// Diet of every province: `{province_id: diet_id}`.
    #[func]
    fn get_province_diets(&self) -> VarDictionary {
        let mut dict = VarDictionary::new();
        if let Some(state) = &self.state {
            for id in state.provinces.keys() {
                dict.set(id.as_str(), state.province_diet(id).as_str());
            }
        }
        dict
    }

    /// Every diet for `province` as its controller sees it, in id order:
    /// `[{id, name, description, cost_per_thousand, cost, effects[{kind,
    /// value, mode, unit_category, class}], requirements{resources[], coastal,
    /// technology, any_building[], terrains[]}, lent_rule, winter_rule,
    /// available, reasons[], current, sources[]}]`. `cost` is the effective
    /// cost this season (winter surcharge included); `reasons` are French.
    #[func]
    fn get_diet_options(&self, province: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(id) = ProvinceId::new(province.to_string()) else {
            return VarArray::new();
        };
        state
            .diet_options(data, &id)
            .into_iter()
            .filter_map(|option| {
                let diet = data.diets.get(&option.diet)?;
                let reasons: PackedStringArray = option.reasons.iter().map(GString::from).collect();
                let sources: PackedStringArray = diet.sources.iter().map(GString::from).collect();
                Some(
                    vdict! {
                        "id" => diet.id.as_str(),
                        "name" => option.name.as_str(),
                        "description" => diet.description.as_str(),
                        "cost_per_thousand" => diet.cost_per_thousand,
                        "cost" => option.cost,
                        "effects" => &effects_array(&diet.effects),
                        "requirements" => &requirements_dict(diet),
                        "lent_rule" => diet.lent_rule.key(),
                        "winter_rule" => diet.winter_rule.key(),
                        "available" => option.available,
                        "reasons" => &reasons,
                        "current" => option.current,
                        "sources" => &sources,
                    }
                    .to_variant(),
                )
            })
            .collect()
    }

    /// Budget line « Table » of `faction`: `{total, last_turn,
    /// provinces[{province, diet, cost}]}` (only provinces with a paying
    /// diet are listed; `total` is this season's projection).
    #[func]
    fn get_table_budget(&self, faction: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarDictionary::new();
        };
        let Some(f) = state.faction_state(&faction) else {
            return VarDictionary::new();
        };
        let provinces: VarArray = state
            .controlled_provinces(&faction)
            .iter()
            .filter_map(|id| {
                let diet = state.province_diet(id);
                let cost = state.diet_cost(data, id, &diet);
                (cost > 0).then(|| {
                    vdict! {
                        "province" => id.as_str(),
                        "diet" => diet.as_str(),
                        "cost" => cost,
                    }
                    .to_variant()
                })
            })
            .collect();
        vdict! {
            "total" => state.faction_table_upkeep(data, &faction),
            "last_turn" => f.table_upkeep_last_turn,
            "provinces" => &provinces,
        }
    }
}
