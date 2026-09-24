//! `CampaignSim` general's retinue (lot C7): catalogue and per-character
//! retinue, read only (the rules live in `sim_campaign::retinue`; a
//! transfer goes through `submit_order({"type": "transfer_companion"})`).

use data_model::{Companion, GameData};
use godot::prelude::*;
use sim_campaign::CampaignState;

use crate::campaign_sim::{effects_array, CampaignSim};

#[godot_api(secondary)]
impl CampaignSim {
    /// `{max, companions: [companion]}`: the whole catalogue of
    /// `data/retinue.json` (see [`companion_dict`]); empty before a
    /// campaign or without the file.
    #[func]
    fn get_retinue_catalog(&self) -> VarDictionary {
        let Some(retinue) = self.data.as_ref().and_then(|d| d.retinue.as_ref()) else {
            return VarDictionary::new();
        };
        let data = self.data.as_ref().expect("checked above");
        let companions: VarArray = retinue
            .companions
            .iter()
            .map(|c| companion_dict(data, c).to_variant())
            .collect();
        vdict! {
            "max" => i64::from(retinue.max_per_character),
            "companions" => &companions,
        }
    }

    /// `[{id, name}]`: generals `character` may hand a companion to
    /// (`sim_campaign::retinue::transfer_targets`); the transfer itself is
    /// the order `{"type": "transfer_companion", from, to, companion}`.
    #[func]
    fn get_retinue_transfer_targets(&self, character: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(from) = data_model::CharacterId::new(character.to_string()) else {
            return VarArray::new();
        };
        sim_campaign::retinue::transfer_targets(state, data, &from)
            .iter()
            .map(|id| {
                vdict! {
                    "id" => id.as_str(),
                    "name" => state.character_name(data, id).as_str(),
                }
                .to_variant()
            })
            .collect()
    }
}

/// `{id, name, local_name, category, glyph, description, inheritable,
/// effects, acquisition: [{trigger, chance_permille, building,
/// building_name}], factions, min_battles, min_command}`.
pub(crate) fn companion_dict(data: &GameData, companion: &Companion) -> VarDictionary {
    let acquisition: VarArray = companion
        .acquisition
        .iter()
        .map(|rule| {
            let building = rule.building.as_ref().map_or("", |b| b.as_str());
            let building_name = rule
                .building
                .as_ref()
                .and_then(|b| data.buildings.get(b))
                .map_or(String::new(), |b| b.name.display.clone());
            vdict! {
                "trigger" => rule.trigger.key(),
                "chance_permille" => i64::from(rule.chance_permille),
                "building" => building,
                "building_name" => building_name.as_str(),
            }
            .to_variant()
        })
        .collect();
    let factions: PackedStringArray = companion
        .conditions
        .factions
        .iter()
        .map(|f| {
            GString::from(
                data.factions
                    .get(f)
                    .map_or(f.as_str(), |d| d.short_or_display_name()),
            )
        })
        .collect();
    vdict! {
        "id" => companion.id.as_str(),
        "name" => companion.name.display.as_str(),
        "local_name" => companion.name.local.as_deref().unwrap_or(""),
        "category" => companion.category.key(),
        "glyph" => companion.glyph.as_str(),
        "description" => companion.description.as_str(),
        "inheritable" => companion.inheritable,
        "effects" => &effects_array(&companion.effects),
        "acquisition" => &acquisition,
        "factions" => &factions,
        "min_battles" => i64::from(companion.conditions.min_battles.unwrap_or(0)),
        "min_command" => i64::from(companion.conditions.min_command.unwrap_or(0)),
    }
}

/// The retinue of `character`, in order of arrival (companion dicts).
pub(crate) fn retinue_array(
    state: &CampaignState,
    data: &GameData,
    character: &data_model::CharacterId,
) -> VarArray {
    let (Some(retinue), Some(c)) = (&data.retinue, state.characters.get(character)) else {
        return VarArray::new();
    };
    c.retinue
        .iter()
        .filter_map(|id| retinue.companion(id))
        .map(|companion| companion_dict(data, companion).to_variant())
        .collect()
}
