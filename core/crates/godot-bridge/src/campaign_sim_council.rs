//! `CampaignSim` API of the council of the realm (RG13a, ADR 0341), in its
//! own secondary `#[godot_api]` block (ADR 0002). The orders go through
//! `submit_order({"type": "appoint_councillor", "office": …, "character": …})`
//! and `submit_order({"type": "dismiss_councillor", "office": …})`.

use godot::prelude::*;

use crate::campaign_sim::{CampaignSim, Ctx};

#[godot_api(secondary)]
impl CampaignSim {
    /// Council of `faction`, one entry per office in file order:
    /// `[{office_id, office_name, skill, holder_id, holder_name,
    /// holder_skill, effect_text, advice_domains}]`. `skill` is the skill
    /// the office reads (`command`, `governance`, `court`); `holder_id` is ""
    /// for a vacant office (`holder_name` "", `holder_skill` 0, `effect_text`
    /// then gives the effect per skill point); `advice_domains` lists the
    /// alert, hint and journal kinds the office advises on.
    #[func]
    fn get_council(&self, faction_id: GString) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        let (Ok(faction), Some(offices)) = (
            data_model::FactionId::new(faction_id.to_string()),
            data.council.as_ref(),
        ) else {
            return VarArray::new();
        };
        offices
            .offices
            .iter()
            .map(|def| {
                let holder = sim_campaign::council::holder(state, &faction, &def.id);
                let skill = holder.map(|h| sim_campaign::council::skill_of(state, h, def.skill));
                let advice: PackedStringArray =
                    def.advice_kinds.iter().map(GString::from).collect();
                vdict! {
                    "office_id" => def.id.as_str(),
                    "office_name" => def.name.display.as_str(),
                    "skill" => def.skill.key(),
                    "holder_id" => holder.map_or("", |h| h.as_str()),
                    "holder_name" => holder
                        .map(|h| state.character_name(data, h))
                        .unwrap_or_default()
                        .as_str(),
                    "holder_skill" => i64::from(skill.unwrap_or(0)),
                    "effect_text" => sim_campaign::council::effect_text(def, skill).as_str(),
                    "advice_domains" => &advice,
                }
                .to_variant()
            })
            .collect()
    }

    /// Characters who may sit in `office` of `faction`, best skill first:
    /// `[{character_id, name, skill_value, current_office}]`
    /// (`current_office` is "" or the id of the office already held; such a
    /// character cannot be appointed before being dismissed).
    #[func]
    fn get_council_candidates(&self, faction_id: GString, office_id: GString) -> VarArray {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarArray::new();
        };
        let Ok(faction) = data_model::FactionId::new(faction_id.to_string()) else {
            return VarArray::new();
        };
        sim_campaign::council::candidates(state, data, &faction, &office_id.to_string())
            .into_iter()
            .map(|candidate| {
                vdict! {
                    "character_id" => candidate.character.as_str(),
                    "name" => state.character_name(data, &candidate.character).as_str(),
                    "skill_value" => i64::from(candidate.skill),
                    "current_office" => candidate.current_office.as_deref().unwrap_or(""),
                }
                .to_variant()
            })
            .collect()
    }
}
