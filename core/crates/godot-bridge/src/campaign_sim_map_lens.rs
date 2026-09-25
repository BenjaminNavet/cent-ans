//! `CampaignSim` map filter API (lot MF1): per-province values of the
//! campaign map filters (wealth, population, unrest, vassal loyalty,
//! supply, claims), seen from the player. Read only.

use godot::prelude::*;
use sim_campaign::map_lens::{map_lens, ClaimStance};

use crate::campaign_sim::CampaignSim;

fn claim_key(claim: ClaimStance) -> &'static str {
    match claim {
        ClaimStance::None => "",
        ClaimStance::Ours => "ours",
        ClaimStance::AgainstUs => "against_us",
        ClaimStance::Contested => "contested",
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// One dictionary per id of `province_ids`, in order: `{income,
    /// population, unrest, vassal_loyalty (-1 if the owner is no vassal),
    /// suzerain, supply_change, claim ("", "ours", "against_us",
    /// "contested")}`; empty for an unknown id.
    #[func]
    fn get_map_lens(&self, province_ids: PackedStringArray) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let lens = map_lens(state, data, state.player_faction());
        province_ids
            .as_slice()
            .iter()
            .map(|id| {
                let Some(values) = data_model::ProvinceId::new(id.to_string())
                    .ok()
                    .and_then(|p| lens.get(&p))
                else {
                    return VarDictionary::new().to_variant();
                };
                vdict! {
                    "income" => values.income,
                    "population" => values.population as i64,
                    "unrest" => values.unrest,
                    "vassal_loyalty" => values.vassal_loyalty.map_or(-1, i64::from),
                    "suzerain" => values.suzerain.as_ref().map_or("", |f| f.as_str()),
                    "supply_change" => i64::from(values.supply_change),
                    "claim" => claim_key(values.claim),
                }
                .to_variant()
            })
            .collect()
    }
}
