//! `CampaignSim` campaign difficulty (lot DF1, ADR 0037): the level chosen
//! on the faction screen, set right after `new_campaign` and frozen once the
//! first turn is played.

use godot::prelude::*;
use sim_campaign::difficulty::{effect_summary, Difficulty};

use crate::campaign_sim::CampaignSim;

#[godot_api(secondary)]
impl CampaignSim {
    /// Sets the campaign's difficulty (`easy`, `normal`, `hard`,
    /// `very_hard`); call it right after `new_campaign`. False for an unknown
    /// id, before a campaign exists, or once the first turn has been played.
    #[func]
    fn set_difficulty(&mut self, id: GString) -> bool {
        if self.refuse_while_turn_pending("set_difficulty") {
            return false;
        }
        let Some(level) = Difficulty::from_key(&id.to_string()) else {
            godot_error!("CampaignSim.set_difficulty: unknown level {id}");
            return false;
        };
        self.state
            .as_mut()
            .is_some_and(|state| state.set_difficulty(level))
    }

    /// Id of the campaign's difficulty (`normal` before a campaign exists).
    #[func]
    fn get_difficulty(&self) -> GString {
        let level = self
            .state
            .as_ref()
            .map_or(Difficulty::Normal, |state| state.difficulty());
        GString::from(level.key())
    }

    /// The four levels, easiest first: `[{id, label, description, effects:
    /// PackedStringArray, summary, default: bool}]`. Works before a campaign
    /// exists (built-in mirror of `data/rules/difficulty.json` when no data
    /// has been loaded yet).
    #[func]
    fn get_difficulty_levels(&self) -> VarArray {
        let data = self.data.clone().or_else(crate::campaign_sim::loaded_data);
        let rules = data
            .as_ref()
            .map_or_else(Default::default, |d| d.difficulty.clone());
        let mut out = VarArray::new();
        for level in &rules.levels {
            let effects = effect_summary(&level.modifiers);
            let mut packed = PackedStringArray::new();
            for line in &effects {
                packed.push(line.as_str());
            }
            let dict = vdict! {
                "id" => level.id.as_str(),
                "label" => level.label.as_str(),
                "description" => level.description.as_str(),
                "effects" => &packed,
                "summary" => effects.join("\n").as_str(),
                "default" => level.id == rules.default,
            };
            out.push(&dict.to_variant());
        }
        out
    }
}
