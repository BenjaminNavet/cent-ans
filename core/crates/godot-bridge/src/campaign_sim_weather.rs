//! `CampaignSim` map weather (lot CM2), read only: rain, snow, morning fog
//! and storms per province for the current turn (`sim_campaign::weather`).

use godot::prelude::*;
use sim_campaign::weather::{self, ProvinceWeather};

use crate::campaign_sim::{CampaignSim, Ctx};

fn weather_dict(w: &ProvinceWeather) -> VarDictionary {
    vdict! {
        "kind" => w.kind.as_str(),
        "intensity" => w.intensity,
        "label" => w.kind.label_fr(),
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Weather of every province this turn: `{province_id: {kind, intensity,
    /// label}}`, `kind` in `clear`, `fog`, `rain`, `snow`, `storm`; empty
    /// before a campaign starts. Deterministic (seed, date, province).
    #[func]
    fn get_campaign_weather(&self) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let mut out = VarDictionary::new();
        for (id, w) in weather::weather_map(data, state) {
            out.set(id.as_str(), &weather_dict(&w).to_variant());
        }
        out
    }

    /// Weather of one province this turn (`{kind, intensity, label}`), empty
    /// when unknown.
    #[func]
    fn get_province_weather(&self, province: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let wanted = province.to_string();
        weather::weather_map(data, state)
            .into_iter()
            .find(|(id, _)| id.as_str() == wanted)
            .map_or_else(VarDictionary::new, |(_, w)| weather_dict(&w))
    }
}
